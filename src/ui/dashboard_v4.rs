//! Dashboard v4 — Motor de estado unificado para pools técnicos, ZKP v4 y DAO Ledger v2
//!
//! LP-84: UI Dashboard v4 & Real-time Streams
//! Extiende Dashboard v3 con métricas de pools técnicos de recursos cross-chain,
//! Async ZKP v4 (pruebas por pool, lotificación, verificación cross-pool) y
//! DAO Ledger v2 (gobernanza técnica con staking y propuestas).
//!
//! Características:
//! - Resumen de pools: shards registrados, créditos disponibles, asignaciones activas
//! - Resumen ZKP v4: declaraciones en cola, lotes generados, tasa de verificación
//! - Resumen DAO Ledger v2: propuestas activas, participación, staking técnico
//! - Alertas integradas: pool agotamiento, ZKP fallos, DAO quorum bajo
//! - Snapshot unificado v4 con todas las secciones
//!
//! Protegido con `#[cfg(feature = "v1.3-sprint2")]`.

#[cfg(feature = "v1.3-sprint2")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "v1.3-sprint2")]
use thiserror::Error;
#[cfg(feature = "v1.3-sprint2")]
use tracing::{debug, info, warn};

// ─── Errors ───────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum DashboardV4Error {
    #[error("Métrica no disponible: {0}")]
    MetricUnavailable(String),
    #[error("Error de agregación: {0}")]
    AggregationError(String),
    #[error("Límite de tasa excedido")]
    RateLimitExceeded,
    #[error("Sección no registrada: {0}")]
    SectionNotRegistered(String),
}

// ─── Metric Types ─────────────────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MetricV4 {
    // Pool metrics
    PoolActiveShards,
    PoolAvailableCredits,
    PoolAllocationsActive,
    PoolAvgLatencyMs,
    PoolReputationAvg,
    PoolCreditDecayRate,
    // ZKP v4 metrics
    ZkpStatementsQueued,
    ZkpBatchesGenerated,
    ZkpVerificationRate,
    ZkpAvgProofTimeMs,
    ZkpCrossPoolVerifications,
    ZkpFallbackRate,
    // DAO Ledger v2 metrics
    DaoActiveProposals,
    DaoVoterParticipation,
    DaoQuorumRate,
    DaoStakingActive,
    DaoStakingTotalCredits,
    DaoEpochCurrent,
    // Network metrics
    NetworkLatencyP99,
    NetworkThroughput,
    NetworkErrorRate,
}

#[cfg(feature = "v1.3-sprint2")]
impl std::fmt::Display for MetricV4 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricV4::PoolActiveShards => write!(f, "pool.active_shards"),
            MetricV4::PoolAvailableCredits => write!(f, "pool.available_credits"),
            MetricV4::PoolAllocationsActive => write!(f, "pool.allocations_active"),
            MetricV4::PoolAvgLatencyMs => write!(f, "pool.avg_latency_ms"),
            MetricV4::PoolReputationAvg => write!(f, "pool.reputation_avg"),
            MetricV4::PoolCreditDecayRate => write!(f, "pool.credit_decay_rate"),
            MetricV4::ZkpStatementsQueued => write!(f, "zkp.statements_queued"),
            MetricV4::ZkpBatchesGenerated => write!(f, "zkp.batches_generated"),
            MetricV4::ZkpVerificationRate => write!(f, "zkp.verification_rate"),
            MetricV4::ZkpAvgProofTimeMs => write!(f, "zkp.avg_proof_time_ms"),
            MetricV4::ZkpCrossPoolVerifications => {
                write!(f, "zkp.cross_pool_verifications")
            }
            MetricV4::ZkpFallbackRate => write!(f, "zkp.fallback_rate"),
            MetricV4::DaoActiveProposals => write!(f, "dao.active_proposals"),
            MetricV4::DaoVoterParticipation => write!(f, "dao.voter_participation"),
            MetricV4::DaoQuorumRate => write!(f, "dao.quorum_rate"),
            MetricV4::DaoStakingActive => write!(f, "dao.staking_active"),
            MetricV4::DaoStakingTotalCredits => write!(f, "dao.staking_total_credits"),
            MetricV4::DaoEpochCurrent => write!(f, "dao.epoch_current"),
            MetricV4::NetworkLatencyP99 => write!(f, "network.latency_p99"),
            MetricV4::NetworkThroughput => write!(f, "network.throughput"),
            MetricV4::NetworkErrorRate => write!(f, "network.error_rate"),
        }
    }
}

// ─── Metric Value ─────────────────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValueV4 {
    pub metric: MetricV4,
    pub value: f64,
    pub timestamp_ms: u64,
    pub source: Option<String>,
}

#[cfg(feature = "v1.3-sprint2")]
impl MetricValueV4 {
    pub fn new(metric: MetricV4, value: f64, source: Option<String>) -> Self {
        Self {
            metric,
            value,
            timestamp_ms: current_timestamp_ms(),
            source,
        }
    }
}

// ─── Pool Summary ─────────────────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolSummary {
    pub active_shards: usize,
    pub available_credits: f64,
    pub allocations_active: usize,
    pub avg_latency_ms: f64,
    pub reputation_avg: f64,
    pub credit_decay_rate: f64,
}

#[cfg(feature = "v1.3-sprint2")]
impl Default for PoolSummary {
    fn default() -> Self {
        Self {
            active_shards: 0,
            available_credits: 0.0,
            allocations_active: 0,
            avg_latency_ms: 0.0,
            reputation_avg: 0.0,
            credit_decay_rate: 0.0,
        }
    }
}

#[cfg(feature = "v1.3-sprint2")]
impl PoolSummary {
    pub fn new() -> Self {
        Self::default()
    }
}

// ─── ZKP v4 Summary ──────────────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkpV4Summary {
    pub statements_queued: usize,
    pub batches_generated: usize,
    pub verification_rate: f64,
    pub avg_proof_time_ms: f64,
    pub cross_pool_verifications: usize,
    pub fallback_rate: f64,
}

#[cfg(feature = "v1.3-sprint2")]
impl Default for ZkpV4Summary {
    fn default() -> Self {
        Self {
            statements_queued: 0,
            batches_generated: 0,
            verification_rate: 0.0,
            avg_proof_time_ms: 0.0,
            cross_pool_verifications: 0,
            fallback_rate: 0.0,
        }
    }
}

#[cfg(feature = "v1.3-sprint2")]
impl ZkpV4Summary {
    pub fn new() -> Self {
        Self::default()
    }
}

// ─── DAO Ledger v2 Summary ───────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaoLedgerSummary {
    pub active_proposals: usize,
    pub voter_participation: f64,
    pub quorum_rate: f64,
    pub staking_active: usize,
    pub staking_total_credits: f64,
    pub epoch_current: u64,
}

#[cfg(feature = "v1.3-sprint2")]
impl Default for DaoLedgerSummary {
    fn default() -> Self {
        Self {
            active_proposals: 0,
            voter_participation: 0.0,
            quorum_rate: 0.0,
            staking_active: 0,
            staking_total_credits: 0.0,
            epoch_current: 0,
        }
    }
}

#[cfg(feature = "v1.3-sprint2")]
impl DaoLedgerSummary {
    pub fn new() -> Self {
        Self::default()
    }
}

// ─── Dashboard V4 Snapshot ───────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardV4Snapshot {
    pub timestamp_ms: u64,
    pub pool: PoolSummary,
    pub zkp: ZkpV4Summary,
    pub dao: DaoLedgerSummary,
    pub network_latency_p99_ms: f64,
    pub network_throughput: f64,
    pub network_error_rate: f64,
    pub total_metrics: usize,
    pub alert_count: usize,
}

// ─── Alert ────────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardAlertV4 {
    pub alert_id: String,
    pub metric: MetricV4,
    pub value: f64,
    pub threshold: f64,
    pub message: String,
    pub timestamp_ms: u64,
    pub acknowledged: bool,
}

// ─── Config ───────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardV4Config {
    pub max_metrics_history: usize,
    pub rate_limit_per_second: usize,
    pub alert_threshold_pool_credits: f64,
    pub alert_threshold_zkp_fallback: f64,
    pub alert_threshold_dao_quorum: f64,
    pub alert_threshold_latency_ms: f64,
    pub alert_threshold_error_rate: f64,
}

#[cfg(feature = "v1.3-sprint2")]
impl Default for DashboardV4Config {
    fn default() -> Self {
        Self {
            max_metrics_history: 2000,
            rate_limit_per_second: 50,
            alert_threshold_pool_credits: 10.0,
            alert_threshold_zkp_fallback: 0.3,
            alert_threshold_dao_quorum: 0.5,
            alert_threshold_latency_ms: 500.0,
            alert_threshold_error_rate: 0.05,
        }
    }
}

// ─── Stats ────────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DashboardV4Stats {
    pub total_snapshots: usize,
    pub total_alerts: usize,
    pub total_metrics_recorded: usize,
    pub rate_limited_count: usize,
}

// ─── Dashboard V4 State ──────────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
pub struct DashboardV4State {
    config: DashboardV4Config,
    metrics: Vec<MetricValueV4>,
    alerts: Vec<DashboardAlertV4>,
    alert_counter: u64,
    last_rate_check_ms: u64,
    requests_this_second: usize,
    pub stats: DashboardV4Stats,
}

#[cfg(feature = "v1.3-sprint2")]
impl DashboardV4State {
    pub fn new() -> Self {
        Self::with_config(DashboardV4Config::default())
    }

    pub fn with_config(config: DashboardV4Config) -> Self {
        Self {
            config,
            metrics: Vec::new(),
            alerts: Vec::new(),
            alert_counter: 0,
            last_rate_check_ms: current_timestamp_ms(),
            requests_this_second: 0,
            stats: DashboardV4Stats::default(),
        }
    }

    /// Registra una métrica
    pub fn record_metric(&mut self, metric: MetricV4, value: f64, source: Option<String>) {
        self.check_rate_limit();
        let entry = MetricValueV4::new(metric.clone(), value, source);
        self.metrics.push(entry);
        self.enforce_history_limit();
        self.check_alerts(&metric, value);
        self.stats.total_metrics_recorded += 1;
        debug!("Métrica v4 registrada: {} = {}", metric, value);
    }

    /// Genera un snapshot unificado v4
    pub fn get_snapshot(&mut self) -> Result<DashboardV4Snapshot, DashboardV4Error> {
        self.check_rate_limit();
        self.stats.total_snapshots += 1;

        let pool = self.aggregate_pool();
        let zkp = self.aggregate_zkp();
        let dao = self.aggregate_dao();

        let latency = self.get_metric_value(&MetricV4::NetworkLatencyP99, 0.0);
        let throughput = self.get_metric_value(&MetricV4::NetworkThroughput, 0.0);
        let error_rate = self.get_metric_value(&MetricV4::NetworkErrorRate, 0.0);

        let unacknowledged = self
            .alerts
            .iter()
            .filter(|a| !a.acknowledged)
            .count();

        Ok(DashboardV4Snapshot {
            timestamp_ms: current_timestamp_ms(),
            pool,
            zkp,
            dao,
            network_latency_p99_ms: latency,
            network_throughput: throughput,
            network_error_rate: error_rate,
            total_metrics: self.metrics.len(),
            alert_count: unacknowledged,
        })
    }

    /// Acknowledge una alerta
    pub fn acknowledge_alert(&mut self, alert_id: &str) -> bool {
        if let Some(alert) = self.alerts.iter_mut().find(|a| a.alert_id == alert_id) {
            alert.acknowledged = true;
            info!("Alerta v4 reconocida: {}", alert_id);
            true
        } else {
            false
        }
    }

    /// Obtiene alertas no reconocidas
    pub fn get_active_alerts(&self) -> Vec<&DashboardAlertV4> {
        self.alerts.iter().filter(|a| !a.acknowledged).collect()
    }

    /// Limpia alertas antiguas
    pub fn clear_old_alerts(&mut self, max_age_ms: u64) -> usize {
        let now = current_timestamp_ms();
        let before = self.alerts.len();
        self.alerts.retain(|a| now.saturating_sub(a.timestamp_ms) < max_age_ms);
        before - self.alerts.len()
    }

    /// Obtiene métricas por tipo
    pub fn get_metric_values(&self, metric: &MetricV4) -> Vec<&MetricValueV4> {
        self.metrics.iter().filter(|m| &m.metric == metric).collect()
    }

    /// Obtiene la última métrica por tipo
    pub fn get_latest_metric(&self, metric: &MetricV4) -> Option<f64> {
        self.metrics
            .iter()
            .rev()
            .find(|m| &m.metric == metric)
            .map(|m| m.value)
    }

    /// Resetea estadísticas
    pub fn reset_stats(&mut self) {
        self.stats = DashboardV4Stats::default();
    }

    // ─── Private helpers ──────────────────────────────────────────────────────

    fn check_rate_limit(&mut self) {
        let now = current_timestamp_ms();
        if now.saturating_sub(self.last_rate_check_ms) > 1000 {
            self.requests_this_second = 0;
            self.last_rate_check_ms = now;
        }
        self.requests_this_second += 1;
        if self.requests_this_second > self.config.rate_limit_per_second {
            self.stats.rate_limited_count += 1;
        }
    }

    fn enforce_history_limit(&mut self) {
        if self.metrics.len() > self.config.max_metrics_history {
            let remove_count = self.metrics.len() - self.config.max_metrics_history;
            self.metrics.drain(0..remove_count);
        }
    }

    fn get_metric_value(&self, metric: &MetricV4, default: f64) -> f64 {
        self.metrics
            .iter()
            .rev()
            .find(|m| &m.metric == metric)
            .map(|m| m.value)
            .unwrap_or(default)
    }

    fn aggregate_pool(&self) -> PoolSummary {
        let mut summary = PoolSummary::new();
        summary.active_shards = self.get_metric_value(&MetricV4::PoolActiveShards, 0.0) as usize;
        summary.available_credits = self.get_metric_value(&MetricV4::PoolAvailableCredits, 0.0);
        summary.allocations_active =
            self.get_metric_value(&MetricV4::PoolAllocationsActive, 0.0) as usize;
        summary.avg_latency_ms = self.get_metric_value(&MetricV4::PoolAvgLatencyMs, 0.0);
        summary.reputation_avg = self.get_metric_value(&MetricV4::PoolReputationAvg, 0.0);
        summary.credit_decay_rate =
            self.get_metric_value(&MetricV4::PoolCreditDecayRate, 0.0);
        summary
    }

    fn aggregate_zkp(&self) -> ZkpV4Summary {
        let mut summary = ZkpV4Summary::new();
        summary.statements_queued =
            self.get_metric_value(&MetricV4::ZkpStatementsQueued, 0.0) as usize;
        summary.batches_generated =
            self.get_metric_value(&MetricV4::ZkpBatchesGenerated, 0.0) as usize;
        summary.verification_rate =
            self.get_metric_value(&MetricV4::ZkpVerificationRate, 0.0);
        summary.avg_proof_time_ms =
            self.get_metric_value(&MetricV4::ZkpAvgProofTimeMs, 0.0);
        summary.cross_pool_verifications =
            self.get_metric_value(&MetricV4::ZkpCrossPoolVerifications, 0.0) as usize;
        summary.fallback_rate = self.get_metric_value(&MetricV4::ZkpFallbackRate, 0.0);
        summary
    }

    fn aggregate_dao(&self) -> DaoLedgerSummary {
        let mut summary = DaoLedgerSummary::new();
        summary.active_proposals =
            self.get_metric_value(&MetricV4::DaoActiveProposals, 0.0) as usize;
        summary.voter_participation =
            self.get_metric_value(&MetricV4::DaoVoterParticipation, 0.0);
        summary.quorum_rate = self.get_metric_value(&MetricV4::DaoQuorumRate, 0.0);
        summary.staking_active =
            self.get_metric_value(&MetricV4::DaoStakingActive, 0.0) as usize;
        summary.staking_total_credits =
            self.get_metric_value(&MetricV4::DaoStakingTotalCredits, 0.0);
        summary.epoch_current =
            self.get_metric_value(&MetricV4::DaoEpochCurrent, 0.0) as u64;
        summary
    }

    fn check_alerts(&mut self, metric: &MetricV4, value: f64) {
        let threshold = match metric {
            MetricV4::PoolAvailableCredits => Some(self.config.alert_threshold_pool_credits),
            MetricV4::ZkpFallbackRate => Some(self.config.alert_threshold_zkp_fallback),
            MetricV4::DaoQuorumRate => Some(self.config.alert_threshold_dao_quorum),
            MetricV4::NetworkLatencyP99 => Some(self.config.alert_threshold_latency_ms),
            MetricV4::NetworkErrorRate => Some(self.config.alert_threshold_error_rate),
            _ => None,
        };

        if let Some(threshold) = threshold {
            let triggered = match metric {
                MetricV4::PoolAvailableCredits => value < threshold,
                MetricV4::DaoQuorumRate => value < threshold,
                _ => value > threshold,
            };

            if triggered {
                self.alert_counter += 1;
                let alert = DashboardAlertV4 {
                    alert_id: format!("alert-v4-{}", self.alert_counter),
                    metric: metric.clone(),
                    value,
                    threshold,
                    message: format!(
                        "Alerta {}: valor {} excede umbral {}",
                        metric, value, threshold
                    ),
                    timestamp_ms: current_timestamp_ms(),
                    acknowledged: false,
                };
                warn!("Alerta v4: {}", alert.message);
                self.alerts.push(alert);
                self.stats.total_alerts += 1;
            }
        }
    }
}

#[cfg(feature = "v1.3-sprint2")]
impl Default for DashboardV4State {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.3-sprint2")]
fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "v1.3-sprint2")]
    fn make_state() -> DashboardV4State {
        DashboardV4State::new()
    }

    #[cfg(feature = "v1.3-sprint2")]
    fn make_state_with_config(config: DashboardV4Config) -> DashboardV4State {
        DashboardV4State::with_config(config)
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_dashboard_creation() {
        let state = make_state();
        assert_eq!(state.stats.total_snapshots, 0);
        assert_eq!(state.stats.total_metrics_recorded, 0);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_dashboard_with_config() {
        let config = DashboardV4Config {
            max_metrics_history: 500,
            rate_limit_per_second: 200,
            ..Default::default()
        };
        let state = make_state_with_config(config);
        assert_eq!(state.config.max_metrics_history, 500);
        assert_eq!(state.config.rate_limit_per_second, 200);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_record_metric() {
        let mut state = make_state();
        state.record_metric(MetricV4::PoolActiveShards, 5.0, None);
        assert_eq!(state.metrics.len(), 1);
        assert_eq!(state.metrics[0].metric, MetricV4::PoolActiveShards);
        assert_eq!(state.metrics[0].value, 5.0);
        assert_eq!(state.stats.total_metrics_recorded, 1);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_record_metric_with_source() {
        let mut state = make_state();
        state.record_metric(
            MetricV4::ZkpVerificationRate,
            0.95,
            Some("pool-1".to_string()),
        );
        assert_eq!(state.metrics[0].source, Some("pool-1".to_string()));
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_snapshot_empty() {
        let mut state = make_state();
        let snapshot = state.get_snapshot().unwrap();
        assert_eq!(snapshot.pool.active_shards, 0);
        assert_eq!(snapshot.zkp.statements_queued, 0);
        assert_eq!(snapshot.dao.active_proposals, 0);
        assert_eq!(snapshot.total_metrics, 0);
        assert_eq!(snapshot.alert_count, 0);
        assert_eq!(state.stats.total_snapshots, 1);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_snapshot_with_pool_metrics() {
        let mut state = make_state();
        state.record_metric(MetricV4::PoolActiveShards, 10.0, None);
        state.record_metric(MetricV4::PoolAvailableCredits, 500.0, None);
        state.record_metric(MetricV4::PoolAllocationsActive, 3.0, None);
        state.record_metric(MetricV4::PoolAvgLatencyMs, 25.0, None);
        state.record_metric(MetricV4::PoolReputationAvg, 0.85, None);

        let snapshot = state.get_snapshot().unwrap();
        assert_eq!(snapshot.pool.active_shards, 10);
        assert_eq!(snapshot.pool.available_credits, 500.0);
        assert_eq!(snapshot.pool.allocations_active, 3);
        assert_eq!(snapshot.pool.avg_latency_ms, 25.0);
        assert_eq!(snapshot.pool.reputation_avg, 0.85);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_snapshot_with_zkp_metrics() {
        let mut state = make_state();
        state.record_metric(MetricV4::ZkpStatementsQueued, 15.0, None);
        state.record_metric(MetricV4::ZkpBatchesGenerated, 8.0, None);
        state.record_metric(MetricV4::ZkpVerificationRate, 0.92, None);
        state.record_metric(MetricV4::ZkpAvgProofTimeMs, 120.0, None);
        state.record_metric(MetricV4::ZkpCrossPoolVerifications, 5.0, None);
        state.record_metric(MetricV4::ZkpFallbackRate, 0.05, None);

        let snapshot = state.get_snapshot().unwrap();
        assert_eq!(snapshot.zkp.statements_queued, 15);
        assert_eq!(snapshot.zkp.batches_generated, 8);
        assert_eq!(snapshot.zkp.verification_rate, 0.92);
        assert_eq!(snapshot.zkp.avg_proof_time_ms, 120.0);
        assert_eq!(snapshot.zkp.cross_pool_verifications, 5);
        assert_eq!(snapshot.zkp.fallback_rate, 0.05);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_snapshot_with_dao_metrics() {
        let mut state = make_state();
        state.record_metric(MetricV4::DaoActiveProposals, 4.0, None);
        state.record_metric(MetricV4::DaoVoterParticipation, 0.75, None);
        state.record_metric(MetricV4::DaoQuorumRate, 0.82, None);
        state.record_metric(MetricV4::DaoStakingActive, 12.0, None);
        state.record_metric(MetricV4::DaoStakingTotalCredits, 1000.0, None);
        state.record_metric(MetricV4::DaoEpochCurrent, 42.0, None);

        let snapshot = state.get_snapshot().unwrap();
        assert_eq!(snapshot.dao.active_proposals, 4);
        assert_eq!(snapshot.dao.voter_participation, 0.75);
        assert_eq!(snapshot.dao.quorum_rate, 0.82);
        assert_eq!(snapshot.dao.staking_active, 12);
        assert_eq!(snapshot.dao.staking_total_credits, 1000.0);
        assert_eq!(snapshot.dao.epoch_current, 42);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_snapshot_with_network_metrics() {
        let mut state = make_state();
        state.record_metric(MetricV4::NetworkLatencyP99, 150.0, None);
        state.record_metric(MetricV4::NetworkThroughput, 5000.0, None);
        state.record_metric(MetricV4::NetworkErrorRate, 0.01, None);

        let snapshot = state.get_snapshot().unwrap();
        assert_eq!(snapshot.network_latency_p99_ms, 150.0);
        assert_eq!(snapshot.network_throughput, 5000.0);
        assert_eq!(snapshot.network_error_rate, 0.01);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_alert_pool_credits_low() {
        let mut state = make_state();
        state.config.alert_threshold_pool_credits = 100.0;
        state.record_metric(MetricV4::PoolAvailableCredits, 50.0, None);

        let alerts = state.get_active_alerts();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].metric, MetricV4::PoolAvailableCredits);
        assert_eq!(alerts[0].value, 50.0);
        assert!(!alerts[0].acknowledged);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_alert_zkp_fallback_high() {
        let mut state = make_state();
        state.config.alert_threshold_zkp_fallback = 0.2;
        state.record_metric(MetricV4::ZkpFallbackRate, 0.35, None);

        let alerts = state.get_active_alerts();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].metric, MetricV4::ZkpFallbackRate);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_alert_dao_quorum_low() {
        let mut state = make_state();
        state.config.alert_threshold_dao_quorum = 0.6;
        state.record_metric(MetricV4::DaoQuorumRate, 0.4, None);

        let alerts = state.get_active_alerts();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].metric, MetricV4::DaoQuorumRate);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_alert_latency_high() {
        let mut state = make_state();
        state.config.alert_threshold_latency_ms = 200.0;
        state.record_metric(MetricV4::NetworkLatencyP99, 350.0, None);

        let alerts = state.get_active_alerts();
        assert_eq!(alerts.len(), 1);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_alert_acknowledge() {
        let mut state = make_state();
        state.config.alert_threshold_latency_ms = 200.0;
        state.record_metric(MetricV4::NetworkLatencyP99, 350.0, None);

        assert_eq!(state.get_active_alerts().len(), 1);
        let alert_id = state.alerts[0].alert_id.clone();
        assert!(state.acknowledge_alert(&alert_id));
        assert_eq!(state.get_active_alerts().len(), 0);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_alert_acknowledge_missing() {
        let mut state = make_state();
        assert!(!state.acknowledge_alert("nonexistent"));
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_clear_old_alerts() {
        let mut state = make_state();
        state.config.alert_threshold_latency_ms = 200.0;
        state.record_metric(MetricV4::NetworkLatencyP99, 350.0, None);
        state.record_metric(MetricV4::NetworkErrorRate, 0.1, None);

        assert_eq!(state.alerts.len(), 2);
        let cleared = state.clear_old_alerts(0);
        assert_eq!(cleared, 2);
        assert_eq!(state.alerts.len(), 0);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_get_metric_values() {
        let mut state = make_state();
        state.record_metric(MetricV4::PoolActiveShards, 5.0, None);
        state.record_metric(MetricV4::PoolActiveShards, 8.0, None);
        state.record_metric(MetricV4::ZkpStatementsQueued, 3.0, None);

        let values = state.get_metric_values(&MetricV4::PoolActiveShards);
        assert_eq!(values.len(), 2);
        assert_eq!(values[0].value, 5.0);
        assert_eq!(values[1].value, 8.0);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_get_latest_metric() {
        let mut state = make_state();
        state.record_metric(MetricV4::PoolActiveShards, 5.0, None);
        state.record_metric(MetricV4::PoolActiveShards, 8.0, None);

        assert_eq!(state.get_latest_metric(&MetricV4::PoolActiveShards), Some(8.0));
        assert_eq!(
            state.get_latest_metric(&MetricV4::ZkpStatementsQueued),
            None
        );
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_history_limit() {
        let config = DashboardV4Config {
            max_metrics_history: 3,
            ..Default::default()
        };
        let mut state = make_state_with_config(config);
        for i in 0..10 {
            state.record_metric(MetricV4::PoolActiveShards, f64::from(i), None);
        }
        assert_eq!(state.metrics.len(), 3);
        assert_eq!(state.metrics[0].value, 7.0);
        assert_eq!(state.metrics[1].value, 8.0);
        assert_eq!(state.metrics[2].value, 9.0);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_reset_stats() {
        let mut state = make_state();
        state.record_metric(MetricV4::PoolActiveShards, 5.0, None);
        let _ = state.get_snapshot();
        assert_eq!(state.stats.total_metrics_recorded, 1);
        assert_eq!(state.stats.total_snapshots, 1);

        state.reset_stats();
        assert_eq!(state.stats.total_metrics_recorded, 0);
        assert_eq!(state.stats.total_snapshots, 0);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_metric_display() {
        assert_eq!(
            format!("{}", MetricV4::PoolActiveShards),
            "pool.active_shards"
        );
        assert_eq!(
            format!("{}", MetricV4::ZkpVerificationRate),
            "zkp.verification_rate"
        );
        assert_eq!(
            format!("{}", MetricV4::DaoStakingActive),
            "dao.staking_active"
        );
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_config_default() {
        let config = DashboardV4Config::default();
        assert_eq!(config.max_metrics_history, 2000);
        assert_eq!(config.rate_limit_per_second, 50);
        assert_eq!(config.alert_threshold_pool_credits, 10.0);
        assert_eq!(config.alert_threshold_zkp_fallback, 0.3);
        assert_eq!(config.alert_threshold_dao_quorum, 0.5);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_stats_default() {
        let stats = DashboardV4Stats::default();
        assert_eq!(stats.total_snapshots, 0);
        assert_eq!(stats.total_alerts, 0);
        assert_eq!(stats.total_metrics_recorded, 0);
        assert_eq!(stats.rate_limited_count, 0);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_pool_summary_new() {
        let summary = PoolSummary::new();
        assert_eq!(summary.active_shards, 0);
        assert_eq!(summary.available_credits, 0.0);
        assert_eq!(summary.allocations_active, 0);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_zkp_summary_new() {
        let summary = ZkpV4Summary::new();
        assert_eq!(summary.statements_queued, 0);
        assert_eq!(summary.batches_generated, 0);
        assert_eq!(summary.verification_rate, 0.0);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_dao_summary_new() {
        let summary = DaoLedgerSummary::new();
        assert_eq!(summary.active_proposals, 0);
        assert_eq!(summary.voter_participation, 0.0);
        assert_eq!(summary.epoch_current, 0);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_full_snapshot() {
        let mut state = make_state();
        // Pool
        state.record_metric(MetricV4::PoolActiveShards, 8.0, None);
        state.record_metric(MetricV4::PoolAvailableCredits, 1200.0, None);
        // ZKP
        state.record_metric(MetricV4::ZkpStatementsQueued, 20.0, None);
        state.record_metric(MetricV4::ZkpVerificationRate, 0.95, None);
        // DAO
        state.record_metric(MetricV4::DaoActiveProposals, 3.0, None);
        state.record_metric(MetricV4::DaoVoterParticipation, 0.8, None);
        // Network
        state.record_metric(MetricV4::NetworkLatencyP99, 100.0, None);

        let snapshot = state.get_snapshot().unwrap();
        assert_eq!(snapshot.pool.active_shards, 8);
        assert_eq!(snapshot.pool.available_credits, 1200.0);
        assert_eq!(snapshot.zkp.statements_queued, 20);
        assert_eq!(snapshot.zkp.verification_rate, 0.95);
        assert_eq!(snapshot.dao.active_proposals, 3);
        assert_eq!(snapshot.dao.voter_participation, 0.8);
        assert_eq!(snapshot.network_latency_p99_ms, 100.0);
        assert_eq!(snapshot.total_metrics, 7);
        assert_eq!(snapshot.alert_count, 0);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_multiple_alerts() {
        let mut state = make_state();
        state.config.alert_threshold_pool_credits = 100.0;
        state.config.alert_threshold_latency_ms = 200.0;

        state.record_metric(MetricV4::PoolAvailableCredits, 50.0, None);
        state.record_metric(MetricV4::NetworkLatencyP99, 350.0, None);

        let alerts = state.get_active_alerts();
        assert_eq!(alerts.len(), 2);
        assert_eq!(state.stats.total_alerts, 2);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_no_alert_when_safe() {
        let mut state = make_state();
        state.config.alert_threshold_pool_credits = 10.0;
        state.record_metric(MetricV4::PoolAvailableCredits, 500.0, None);

        assert_eq!(state.get_active_alerts().len(), 0);
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_error_display() {
        let err = DashboardV4Error::MetricUnavailable("test".to_string());
        assert!(format!("{}", err).contains("test"));
    }

    #[test]
    #[cfg(feature = "v1.3-sprint2")]
    fn test_dashboard_default() {
        let state = DashboardV4State::default();
        assert_eq!(state.stats.total_snapshots, 0);
        assert_eq!(state.metrics.len(), 0);
    }
}
