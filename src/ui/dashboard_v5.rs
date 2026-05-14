//! Dashboard v5 — Motor de estado unificado con ZKP v7, Cross-Pool Verification y Governance v4
//!
//! LP-106: UI Dashboard v5 & Real-time Streams
//! Extiende Dashboard v4 con métricas de Async ZKP v7 (agregación recursiva, profiling de
//! throughput, delegación por shard), Cross-Pool Verification (sesiones, consenso, reputación)
//! y Governance v4 (DAO ledger, hybrid executor, audit trail).
//!
//! Características:
//! - Resumen ZKP v7: shards registrados, lotes activos, tasa de agregación, throughput adaptativo
//! - Resumen Cross-Pool: sesiones activas, consenso alcanzado, reputación promedio
//! - Resumen Governance v4: entradas de auditoría, acciones por severidad, compliance reports
//! - Alertas integradas v5: ZKP lifecycle failures, cross-pool consensus failures, audit anomalies
//! - Snapshot unificado v5 con todas las secciones
//!
//! Protegido con `#[cfg(feature = "v1.4-sprint2")]`.

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ─── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum DashboardV5Error {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MetricV5 {
    // ZKP v7 metrics
    ZkpV7StatementsQueued,
    ZkpV7BatchesActive,
    ZkpV7ProofsGenerated,
    ZkpV7AggregationDepth,
    ZkpV7ThroughputProofsPerSec,
    ZkpV7ShardsRegistered,
    ZkpV7AvgGenerationMs,
    ZkpV7LifecycleFailures,
    // Cross-Pool metrics
    CrossPoolSessionsActive,
    CrossPoolConsensusReached,
    CrossPoolAvgReputation,
    CrossPoolChallengeRate,
    CrossPoolQuorumFailures,
    CrossPoolVoteLatencyMs,
    // Governance v4 metrics
    GovernanceAuditEntries,
    GovernanceActionsBySeverity,
    GovernanceComplianceScore,
    GovernanceHybridExecutions,
    GovernanceQuorumLevel,
    GovernanceProposalLatencyMs,
    // Network metrics
    NetworkActiveConnections,
    NetworkBandwidthMbits,
    NetworkLatencyP99,
    NetworkPacketLoss,
    // System metrics
    SystemCpuPercent,
    SystemMemoryPercent,
    SystemDiskIoMbits,
    SystemGoroutines,
}

impl std::fmt::Display for MetricV5 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricV5::ZkpV7StatementsQueued => write!(f, "zkp_v7.statements_queued"),
            MetricV5::ZkpV7BatchesActive => write!(f, "zkp_v7.batches_active"),
            MetricV5::ZkpV7ProofsGenerated => write!(f, "zkp_v7.proofs_generated"),
            MetricV5::ZkpV7AggregationDepth => write!(f, "zkp_v7.aggregation_depth"),
            MetricV5::ZkpV7ThroughputProofsPerSec => write!(f, "zkp_v7.throughput"),
            MetricV5::ZkpV7ShardsRegistered => write!(f, "zkp_v7.shards_registered"),
            MetricV5::ZkpV7AvgGenerationMs => write!(f, "zkp_v7.avg_generation_ms"),
            MetricV5::ZkpV7LifecycleFailures => write!(f, "zkp_v7.lifecycle_failures"),
            MetricV5::CrossPoolSessionsActive => write!(f, "cross_pool.sessions_active"),
            MetricV5::CrossPoolConsensusReached => write!(f, "cross_pool.consensus_reached"),
            MetricV5::CrossPoolAvgReputation => write!(f, "cross_pool.avg_reputation"),
            MetricV5::CrossPoolChallengeRate => write!(f, "cross_pool.challenge_rate"),
            MetricV5::CrossPoolQuorumFailures => write!(f, "cross_pool.quorum_failures"),
            MetricV5::CrossPoolVoteLatencyMs => write!(f, "cross_pool.vote_latency_ms"),
            MetricV5::GovernanceAuditEntries => write!(f, "governance.audit_entries"),
            MetricV5::GovernanceActionsBySeverity => write!(f, "governance.actions_by_severity"),
            MetricV5::GovernanceComplianceScore => write!(f, "governance.compliance_score"),
            MetricV5::GovernanceHybridExecutions => write!(f, "governance.hybrid_executions"),
            MetricV5::GovernanceQuorumLevel => write!(f, "governance.quorum_level"),
            MetricV5::GovernanceProposalLatencyMs => write!(f, "governance.proposal_latency_ms"),
            MetricV5::NetworkActiveConnections => write!(f, "network.active_connections"),
            MetricV5::NetworkBandwidthMbits => write!(f, "network.bandwidth_mbits"),
            MetricV5::NetworkLatencyP99 => write!(f, "network.latency_p99"),
            MetricV5::NetworkPacketLoss => write!(f, "network.packet_loss"),
            MetricV5::SystemCpuPercent => write!(f, "system.cpu_percent"),
            MetricV5::SystemMemoryPercent => write!(f, "system.memory_percent"),
            MetricV5::SystemDiskIoMbits => write!(f, "system.disk_io_mbits"),
            MetricV5::SystemGoroutines => write!(f, "system.goroutines"),
        }
    }
}

// ─── Metric Value ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValueV5 {
    /// The metric identifier.
    pub metric: MetricV5,
    /// The metric value.
    pub value: f64,
    /// Optional source identifier.
    pub source: Option<String>,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
}

impl MetricValueV5 {
    pub fn new(metric: MetricV5, value: f64, source: Option<String>) -> Self {
        Self {
            metric,
            value,
            source,
            timestamp_ms: current_timestamp_ms(),
        }
    }
}

// ─── ZKP v7 Summary ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkpV7Summary {
    /// Number of registered shards.
    pub shards_registered: usize,
    /// Number of active batches.
    pub batches_active: usize,
    /// Total proofs generated.
    pub proofs_generated: u64,
    /// Average aggregation depth.
    pub avg_aggregation_depth: f64,
    /// Current throughput (proofs/sec).
    pub throughput: f64,
    /// Average generation time in ms.
    pub avg_generation_ms: f64,
    /// Lifecycle failure count.
    pub lifecycle_failures: u64,
}

impl Default for ZkpV7Summary {
    fn default() -> Self {
        Self {
            shards_registered: 0,
            batches_active: 0,
            proofs_generated: 0,
            avg_aggregation_depth: 0.0,
            throughput: 0.0,
            avg_generation_ms: 0.0,
            lifecycle_failures: 0,
        }
    }
}

impl ZkpV7Summary {
    pub fn new(
        shards_registered: usize,
        batches_active: usize,
        proofs_generated: u64,
        avg_aggregation_depth: f64,
        throughput: f64,
        avg_generation_ms: f64,
        lifecycle_failures: u64,
    ) -> Self {
        Self {
            shards_registered,
            batches_active,
            proofs_generated,
            avg_aggregation_depth,
            throughput,
            avg_generation_ms,
            lifecycle_failures,
        }
    }
}

// ─── Cross-Pool Summary ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossPoolSummary {
    /// Number of active verification sessions.
    pub sessions_active: usize,
    /// Total consensus reached count.
    pub consensus_reached: u64,
    /// Average pool reputation.
    pub avg_reputation: f64,
    /// Challenge response rate.
    pub challenge_rate: f64,
    /// Quorum failure count.
    pub quorum_failures: u64,
    /// Average vote latency in ms.
    pub avg_vote_latency_ms: f64,
}

impl Default for CrossPoolSummary {
    fn default() -> Self {
        Self {
            sessions_active: 0,
            consensus_reached: 0,
            avg_reputation: 0.0,
            challenge_rate: 0.0,
            quorum_failures: 0,
            avg_vote_latency_ms: 0.0,
        }
    }
}

impl CrossPoolSummary {
    pub fn new(
        sessions_active: usize,
        consensus_reached: u64,
        avg_reputation: f64,
        challenge_rate: f64,
        quorum_failures: u64,
        avg_vote_latency_ms: f64,
    ) -> Self {
        Self {
            sessions_active,
            consensus_reached,
            avg_reputation,
            challenge_rate,
            quorum_failures,
            avg_vote_latency_ms,
        }
    }
}

// ─── Governance v4 Summary ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceV4Summary {
    /// Total audit entries.
    pub audit_entries: u64,
    /// Actions by severity level (critical count).
    pub critical_actions: u64,
    /// Compliance score (0.0-1.0).
    pub compliance_score: f64,
    /// Hybrid execution count.
    pub hybrid_executions: u64,
    /// Current quorum level.
    pub quorum_level: f64,
    /// Average proposal latency in ms.
    pub avg_proposal_latency_ms: f64,
}

impl Default for GovernanceV4Summary {
    fn default() -> Self {
        Self {
            audit_entries: 0,
            critical_actions: 0,
            compliance_score: 0.0,
            hybrid_executions: 0,
            quorum_level: 0.0,
            avg_proposal_latency_ms: 0.0,
        }
    }
}

impl GovernanceV4Summary {
    pub fn new(
        audit_entries: u64,
        critical_actions: u64,
        compliance_score: f64,
        hybrid_executions: u64,
        quorum_level: f64,
        avg_proposal_latency_ms: f64,
    ) -> Self {
        Self {
            audit_entries,
            critical_actions,
            compliance_score,
            hybrid_executions,
            quorum_level,
            avg_proposal_latency_ms,
        }
    }
}

// ─── Snapshot ────────────────────────────────────────────────────────────────

/// Unified snapshot of all dashboard v5 sections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardV5Snapshot {
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// ZKP v7 summary.
    pub zkp_v7: ZkpV7Summary,
    /// Cross-pool verification summary.
    pub cross_pool: CrossPoolSummary,
    /// Governance v4 summary.
    pub governance_v4: GovernanceV4Summary,
    /// Network metrics.
    pub network_connections: usize,
    pub network_bandwidth_mbits: f64,
    pub network_latency_p99: f64,
    /// System metrics.
    pub system_cpu_percent: f64,
    pub system_memory_percent: f64,
    /// Active alerts.
    pub alerts: Vec<DashboardAlertV5>,
}

// ─── Alert ───────────────────────────────────────────────────────────────────

/// Alert generated by dashboard v5 monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardAlertV5 {
    /// Unique alert identifier.
    pub alert_id: String,
    /// Alert metric source.
    pub metric: MetricV5,
    /// Alert value.
    pub value: f64,
    /// Alert threshold.
    pub threshold: f64,
    /// Alert severity (1-5).
    pub severity: u8,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Acknowledged status.
    pub acknowledged: bool,
}

// ─── Config ──────────────────────────────────────────────────────────────────

/// Dashboard v5 configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardV5Config {
    /// Maximum metric history entries.
    pub max_history: usize,
    /// Snapshot rate limit (ms between snapshots).
    pub snapshot_rate_limit_ms: u64,
    /// ZKP v7 throughput alert threshold (proofs/sec).
    pub zkp_throughput_threshold: f64,
    /// Cross-pool reputation alert threshold.
    pub cross_pool_reputation_threshold: f64,
    /// Governance compliance alert threshold.
    pub governance_compliance_threshold: f64,
    /// System CPU alert threshold (percent).
    pub system_cpu_threshold: f64,
    /// System memory alert threshold (percent).
    pub system_memory_threshold: f64,
    /// Network latency alert threshold (ms).
    pub network_latency_threshold: f64,
}

impl Default for DashboardV5Config {
    fn default() -> Self {
        Self {
            max_history: 5000,
            snapshot_rate_limit_ms: 1000,
            zkp_throughput_threshold: 10.0,
            cross_pool_reputation_threshold: 0.5,
            governance_compliance_threshold: 0.8,
            system_cpu_threshold: 90.0,
            system_memory_threshold: 85.0,
            network_latency_threshold: 500.0,
        }
    }
}

// ─── Stats ───────────────────────────────────────────────────────────────────

/// Dashboard v5 statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardV5Stats {
    /// Total metrics recorded.
    pub metrics_recorded: u64,
    /// Total snapshots generated.
    pub snapshots_generated: u64,
    /// Total alerts triggered.
    pub alerts_triggered: u64,
    /// Total rate limit rejections.
    pub rate_limit_rejections: u64,
}

impl Default for DashboardV5Stats {
    fn default() -> Self {
        Self::new()
    }
}

impl DashboardV5Stats {
    pub fn new() -> Self {
        Self {
            metrics_recorded: 0,
            snapshots_generated: 0,
            alerts_triggered: 0,
            rate_limit_rejections: 0,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

// ─── Dashboard V5 State ──────────────────────────────────────────────────────

/// Unified dashboard v5 state engine.
pub struct DashboardV5State {
    /// Configuration.
    config: DashboardV5Config,
    /// Metric history.
    history: Vec<MetricValueV5>,
    /// Last snapshot timestamp.
    last_snapshot_ms: u64,
    /// Active alerts.
    alerts: Vec<DashboardAlertV5>,
    /// Statistics.
    stats: DashboardV5Stats,
    /// Alert ID counter.
    alert_counter: u64,
}

impl DashboardV5State {
    /// Create a new dashboard v5 state with default config.
    pub fn new() -> Self {
        Self::with_config(DashboardV5Config::default())
    }

    /// Create a new dashboard v5 state with custom config.
    pub fn with_config(config: DashboardV5Config) -> Self {
        Self {
            config,
            history: Vec::new(),
            last_snapshot_ms: 0,
            alerts: Vec::new(),
            stats: DashboardV5Stats::new(),
            alert_counter: 0,
        }
    }

    /// Record a metric value.
    pub fn record_metric(&mut self, metric: MetricV5, value: f64, source: Option<String>) {
        self.history.push(MetricValueV5::new(metric, value, source));
        self.stats.metrics_recorded += 1;
        self.check_alerts(&metric, value);
        self.enforce_history_limit();
    }

    /// Get a unified snapshot.
    pub fn get_snapshot(&mut self) -> Result<DashboardV5Snapshot, DashboardV5Error> {
        if self.last_snapshot_ms > 0 {
            return Err(DashboardV5Error::RateLimitExceeded);
        }
        self.last_snapshot_ms = current_timestamp_ms();
        self.stats.snapshots_generated += 1;

        let zkp_v7 = self.aggregate_zkp_v7();
        let cross_pool = self.aggregate_cross_pool();
        let governance_v4 = self.aggregate_governance_v4();

        Ok(DashboardV5Snapshot {
            timestamp_ms: self.last_snapshot_ms,
            zkp_v7,
            cross_pool,
            governance_v4,
            network_connections: self.get_metric_value(&MetricV5::NetworkActiveConnections, 0.0) as usize,
            network_bandwidth_mbits: self.get_metric_value(&MetricV5::NetworkBandwidthMbits, 0.0),
            network_latency_p99: self.get_metric_value(&MetricV5::NetworkLatencyP99, 0.0),
            system_cpu_percent: self.get_metric_value(&MetricV5::SystemCpuPercent, 0.0),
            system_memory_percent: self.get_metric_value(&MetricV5::SystemMemoryPercent, 0.0),
            alerts: self.alerts.clone(),
        })
    }

    /// Acknowledge an alert.
    pub fn acknowledge_alert(&mut self, alert_id: &str) -> bool {
        for alert in &mut self.alerts {
            if alert.alert_id == alert_id {
                alert.acknowledged = true;
                return true;
            }
        }
        false
    }

    /// Clear old alerts.
    pub fn clear_old_alerts(&mut self, max_age_ms: u64) -> usize {
        let now = current_timestamp_ms();
        let before = self.alerts.len();
        self.alerts.retain(|a| now - a.timestamp_ms <= max_age_ms || !a.acknowledged);
        before - self.alerts.len()
    }

    /// Get latest value for a metric.
    pub fn get_latest_metric(&self, metric: &MetricV5) -> Option<f64> {
        self.history.iter().rev().find(|m| &m.metric == metric).map(|m| m.value)
    }

    /// Get all values for a metric.
    pub fn get_metric_values(&self, metric: &MetricV5) -> Vec<f64> {
        self.history.iter().filter(|m| &m.metric == metric).map(|m| m.value).collect()
    }

    /// Reset rate limit.
    pub fn reset_rate_limit(&mut self) {
        self.last_snapshot_ms = 0;
    }

    /// Reset statistics.
    pub fn reset_stats(&mut self) {
        self.stats.reset();
    }

    /// Get config reference.
    pub fn get_config(&self) -> &DashboardV5Config {
        &self.config
    }

    /// Get stats reference.
    pub fn get_stats(&self) -> &DashboardV5Stats {
        &self.stats
    }

    /// Get active alerts count.
    pub fn active_alert_count(&self) -> usize {
        self.alerts.iter().filter(|a| !a.acknowledged).count()
    }

    // ─── Private helpers ─────────────────────────────────────────────────────

    fn get_metric_value(&self, metric: &MetricV5, default: f64) -> f64 {
        self.history.iter().rev().find(|m| &m.metric == metric).map(|m| m.value).unwrap_or(default)
    }

    fn aggregate_zkp_v7(&self) -> ZkpV7Summary {
        ZkpV7Summary::new(
            self.get_metric_value(&MetricV5::ZkpV7ShardsRegistered, 0.0) as usize,
            self.get_metric_value(&MetricV5::ZkpV7BatchesActive, 0.0) as usize,
            self.get_metric_value(&MetricV5::ZkpV7ProofsGenerated, 0.0) as u64,
            self.get_metric_value(&MetricV5::ZkpV7AggregationDepth, 0.0),
            self.get_metric_value(&MetricV5::ZkpV7ThroughputProofsPerSec, 0.0),
            self.get_metric_value(&MetricV5::ZkpV7AvgGenerationMs, 0.0),
            self.get_metric_value(&MetricV5::ZkpV7LifecycleFailures, 0.0) as u64,
        )
    }

    fn aggregate_cross_pool(&self) -> CrossPoolSummary {
        CrossPoolSummary::new(
            self.get_metric_value(&MetricV5::CrossPoolSessionsActive, 0.0) as usize,
            self.get_metric_value(&MetricV5::CrossPoolConsensusReached, 0.0) as u64,
            self.get_metric_value(&MetricV5::CrossPoolAvgReputation, 0.0),
            self.get_metric_value(&MetricV5::CrossPoolChallengeRate, 0.0),
            self.get_metric_value(&MetricV5::CrossPoolQuorumFailures, 0.0) as u64,
            self.get_metric_value(&MetricV5::CrossPoolVoteLatencyMs, 0.0),
        )
    }

    fn aggregate_governance_v4(&self) -> GovernanceV4Summary {
        GovernanceV4Summary::new(
            self.get_metric_value(&MetricV5::GovernanceAuditEntries, 0.0) as u64,
            self.get_metric_value(&MetricV5::GovernanceActionsBySeverity, 0.0) as u64,
            self.get_metric_value(&MetricV5::GovernanceComplianceScore, 0.0),
            self.get_metric_value(&MetricV5::GovernanceHybridExecutions, 0.0) as u64,
            self.get_metric_value(&MetricV5::GovernanceQuorumLevel, 0.0),
            self.get_metric_value(&MetricV5::GovernanceProposalLatencyMs, 0.0),
        )
    }

    fn check_alerts(&mut self, metric: &MetricV5, value: f64) {
        let threshold = match metric {
            MetricV5::ZkpV7ThroughputProofsPerSec => Some(self.config.zkp_throughput_threshold),
            MetricV5::CrossPoolAvgReputation => Some(self.config.cross_pool_reputation_threshold),
            MetricV5::GovernanceComplianceScore => Some(self.config.governance_compliance_threshold),
            MetricV5::SystemCpuPercent => Some(self.config.system_cpu_threshold),
            MetricV5::SystemMemoryPercent => Some(self.config.system_memory_threshold),
            MetricV5::NetworkLatencyP99 => Some(self.config.network_latency_threshold),
            _ => None,
        };
        if let Some(threshold) = threshold {
            let triggered = match metric {
                MetricV5::ZkpV7ThroughputProofsPerSec
                | MetricV5::CrossPoolAvgReputation
                | MetricV5::GovernanceComplianceScore => value < threshold,
                _ => value > threshold,
            };
            if triggered {
                self.alert_counter += 1;
                let severity = match metric {
                    MetricV5::SystemCpuPercent | MetricV5::SystemMemoryPercent => 5,
                    MetricV5::ZkpV7ThroughputProofsPerSec => 4,
                    MetricV5::CrossPoolAvgReputation => 4,
                    MetricV5::GovernanceComplianceScore => 3,
                    MetricV5::NetworkLatencyP99 => 3,
                    _ => 2,
                };
                self.alerts.push(DashboardAlertV5 {
                    alert_id: format!("alert-v5-{}", self.alert_counter),
                    metric: metric.clone(),
                    value,
                    threshold,
                    severity,
                    timestamp_ms: current_timestamp_ms(),
                    acknowledged: false,
                });
                self.stats.alerts_triggered += 1;
            }
        }
    }

    fn enforce_history_limit(&mut self) {
        if self.history.len() > self.config.max_history {
            let excess = self.history.len() - self.config.max_history;
            self.history.drain(..excess);
        }
    }
}

impl Default for DashboardV5State {
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
        let dashboard = DashboardV5State::new();
        assert_eq!(dashboard.history.len(), 0);
        assert_eq!(dashboard.stats.metrics_recorded, 0);
    }

    #[test]
    fn test_dashboard_with_config() {
        let config = DashboardV5Config {
            max_history: 100,
            ..Default::default()
        };
        let dashboard = DashboardV5State::with_config(config);
        assert_eq!(dashboard.config.max_history, 100);
    }

    #[test]
    fn test_record_metric() {
        let mut dashboard = DashboardV5State::new();
        dashboard.record_metric(MetricV5::ZkpV7ProofsGenerated, 42.0, None);
        assert_eq!(dashboard.history.len(), 1);
        assert_eq!(dashboard.stats.metrics_recorded, 1);
    }

    #[test]
    fn test_record_metric_with_source() {
        let mut dashboard = DashboardV5State::new();
        dashboard.record_metric(MetricV5::ZkpV7ThroughputProofsPerSec, 15.0, Some("shard-1".to_string()));
        let entry = &dashboard.history[0];
        assert_eq!(entry.source, Some("shard-1".to_string()));
    }

    #[test]
    fn test_snapshot_empty() {
        let mut dashboard = DashboardV5State::new();
        let snapshot = dashboard.get_snapshot().unwrap();
        assert_eq!(snapshot.zkp_v7.proofs_generated, 0);
        assert_eq!(snapshot.cross_pool.sessions_active, 0);
    }

    #[test]
    fn test_snapshot_with_zkp_metrics() {
        let mut dashboard = DashboardV5State::new();
        dashboard.record_metric(MetricV5::ZkpV7ShardsRegistered, 5.0, None);
        dashboard.record_metric(MetricV5::ZkpV7ProofsGenerated, 100.0, None);
        dashboard.record_metric(MetricV5::ZkpV7ThroughputProofsPerSec, 25.0, None);
        let snapshot = dashboard.get_snapshot().unwrap();
        assert_eq!(snapshot.zkp_v7.shards_registered, 5);
        assert_eq!(snapshot.zkp_v7.proofs_generated, 100);
        assert_eq!(snapshot.zkp_v7.throughput, 25.0);
    }

    #[test]
    fn test_snapshot_with_cross_pool_metrics() {
        let mut dashboard = DashboardV5State::new();
        dashboard.record_metric(MetricV5::CrossPoolSessionsActive, 3.0, None);
        dashboard.record_metric(MetricV5::CrossPoolConsensusReached, 10.0, None);
        dashboard.record_metric(MetricV5::CrossPoolAvgReputation, 0.85, None);
        let snapshot = dashboard.get_snapshot().unwrap();
        assert_eq!(snapshot.cross_pool.sessions_active, 3);
        assert_eq!(snapshot.cross_pool.consensus_reached, 10);
        assert_eq!(snapshot.cross_pool.avg_reputation, 0.85);
    }

    #[test]
    fn test_snapshot_with_governance_metrics() {
        let mut dashboard = DashboardV5State::new();
        dashboard.record_metric(MetricV5::GovernanceAuditEntries, 500.0, None);
        dashboard.record_metric(MetricV5::GovernanceComplianceScore, 0.95, None);
        dashboard.record_metric(MetricV5::GovernanceHybridExecutions, 25.0, None);
        let snapshot = dashboard.get_snapshot().unwrap();
        assert_eq!(snapshot.governance_v4.audit_entries, 500);
        assert_eq!(snapshot.governance_v4.compliance_score, 0.95);
        assert_eq!(snapshot.governance_v4.hybrid_executions, 25);
    }

    #[test]
    fn test_snapshot_with_system_metrics() {
        let mut dashboard = DashboardV5State::new();
        dashboard.record_metric(MetricV5::SystemCpuPercent, 75.0, None);
        dashboard.record_metric(MetricV5::SystemMemoryPercent, 60.0, None);
        let snapshot = dashboard.get_snapshot().unwrap();
        assert_eq!(snapshot.system_cpu_percent, 75.0);
        assert_eq!(snapshot.system_memory_percent, 60.0);
    }

    #[test]
    fn test_alert_zkp_throughput_low() {
        let mut dashboard = DashboardV5State::new();
        dashboard.record_metric(MetricV5::ZkpV7ThroughputProofsPerSec, 5.0, None);
        assert_eq!(dashboard.active_alert_count(), 1);
        let alert = &dashboard.alerts[0];
        assert_eq!(alert.metric, MetricV5::ZkpV7ThroughputProofsPerSec);
        assert_eq!(alert.value, 5.0);
    }

    #[test]
    fn test_alert_cross_pool_reputation_low() {
        let mut dashboard = DashboardV5State::new();
        dashboard.record_metric(MetricV5::CrossPoolAvgReputation, 0.3, None);
        assert_eq!(dashboard.active_alert_count(), 1);
    }

    #[test]
    fn test_alert_governance_compliance_low() {
        let mut dashboard = DashboardV5State::new();
        dashboard.record_metric(MetricV5::GovernanceComplianceScore, 0.5, None);
        assert_eq!(dashboard.active_alert_count(), 1);
    }

    #[test]
    fn test_alert_cpu_high() {
        let mut dashboard = DashboardV5State::new();
        dashboard.record_metric(MetricV5::SystemCpuPercent, 95.0, None);
        assert_eq!(dashboard.active_alert_count(), 1);
        assert_eq!(dashboard.alerts[0].severity, 5);
    }

    #[test]
    fn test_alert_memory_high() {
        let mut dashboard = DashboardV5State::new();
        dashboard.record_metric(MetricV5::SystemMemoryPercent, 90.0, None);
        assert_eq!(dashboard.active_alert_count(), 1);
        assert_eq!(dashboard.alerts[0].severity, 5);
    }

    #[test]
    fn test_alert_latency_high() {
        let mut dashboard = DashboardV5State::new();
        dashboard.record_metric(MetricV5::NetworkLatencyP99, 600.0, None);
        assert_eq!(dashboard.active_alert_count(), 1);
    }

    #[test]
    fn test_alert_acknowledge() {
        let mut dashboard = DashboardV5State::new();
        dashboard.record_metric(MetricV5::SystemCpuPercent, 95.0, None);
        assert!(dashboard.acknowledge_alert("alert-v5-1"));
        assert_eq!(dashboard.active_alert_count(), 0);
    }

    #[test]
    fn test_alert_acknowledge_missing() {
        let mut dashboard = DashboardV5State::new();
        assert!(!dashboard.acknowledge_alert("nonexistent"));
    }

    #[test]
    fn test_clear_old_alerts() {
        let mut dashboard = DashboardV5State::new();
        dashboard.record_metric(MetricV5::SystemCpuPercent, 95.0, None);
        dashboard.acknowledge_alert("alert-v5-1");
        let cleared = dashboard.clear_old_alerts(0);
        assert_eq!(cleared, 1);
    }

    #[test]
    fn test_get_metric_values() {
        let mut dashboard = DashboardV5State::new();
        dashboard.record_metric(MetricV5::ZkpV7ProofsGenerated, 10.0, None);
        dashboard.record_metric(MetricV5::ZkpV7ProofsGenerated, 20.0, None);
        dashboard.record_metric(MetricV5::ZkpV7ProofsGenerated, 30.0, None);
        let values = dashboard.get_metric_values(&MetricV5::ZkpV7ProofsGenerated);
        assert_eq!(values, vec![10.0, 20.0, 30.0]);
    }

    #[test]
    fn test_get_latest_metric() {
        let mut dashboard = DashboardV5State::new();
        dashboard.record_metric(MetricV5::ZkpV7ProofsGenerated, 10.0, None);
        dashboard.record_metric(MetricV5::ZkpV7ProofsGenerated, 20.0, None);
        assert_eq!(dashboard.get_latest_metric(&MetricV5::ZkpV7ProofsGenerated), Some(20.0));
        assert_eq!(dashboard.get_latest_metric(&MetricV5::SystemCpuPercent), None);
    }

    #[test]
    fn test_history_limit() {
        let config = DashboardV5Config {
            max_history: 5,
            ..Default::default()
        };
        let mut dashboard = DashboardV5State::with_config(config);
        for i in 0..10 {
            dashboard.record_metric(MetricV5::ZkpV7ProofsGenerated, i as f64, None);
        }
        assert_eq!(dashboard.history.len(), 5);
        assert_eq!(dashboard.history[0].value, 5.0);
    }

    #[test]
    fn test_rate_limit() {
        let mut dashboard = DashboardV5State::new();
        dashboard.get_snapshot().unwrap();
        assert!(dashboard.get_snapshot().is_err());
        dashboard.reset_rate_limit();
        dashboard.get_snapshot().unwrap();
    }

    #[test]
    fn test_reset_stats() {
        let mut dashboard = DashboardV5State::new();
        dashboard.record_metric(MetricV5::ZkpV7ProofsGenerated, 10.0, None);
        dashboard.reset_stats();
        assert_eq!(dashboard.stats.metrics_recorded, 0);
    }

    #[test]
    fn test_metric_display() {
        assert_eq!(format!("{}", MetricV5::ZkpV7ProofsGenerated), "zkp_v7.proofs_generated");
        assert_eq!(format!("{}", MetricV5::CrossPoolSessionsActive), "cross_pool.sessions_active");
        assert_eq!(format!("{}", MetricV5::GovernanceAuditEntries), "governance.audit_entries");
        assert_eq!(format!("{}", MetricV5::SystemCpuPercent), "system.cpu_percent");
    }

    #[test]
    fn test_config_default() {
        let config = DashboardV5Config::default();
        assert_eq!(config.max_history, 5000);
        assert_eq!(config.snapshot_rate_limit_ms, 1000);
        assert_eq!(config.zkp_throughput_threshold, 10.0);
    }

    #[test]
    fn test_stats_default() {
        let stats = DashboardV5Stats::default();
        assert_eq!(stats.metrics_recorded, 0);
        assert_eq!(stats.snapshots_generated, 0);
    }

    #[test]
    fn test_stats_reset() {
        let mut stats = DashboardV5Stats::new();
        stats.metrics_recorded = 100;
        stats.reset();
        assert_eq!(stats.metrics_recorded, 0);
    }

    #[test]
    fn test_zkp_summary_default() {
        let summary = ZkpV7Summary::default();
        assert_eq!(summary.shards_registered, 0);
        assert_eq!(summary.proofs_generated, 0);
    }

    #[test]
    fn test_cross_pool_summary_default() {
        let summary = CrossPoolSummary::default();
        assert_eq!(summary.sessions_active, 0);
        assert_eq!(summary.consensus_reached, 0);
    }

    #[test]
    fn test_governance_summary_default() {
        let summary = GovernanceV4Summary::default();
        assert_eq!(summary.audit_entries, 0);
        assert_eq!(summary.compliance_score, 0.0);
    }

    #[test]
    fn test_full_snapshot() {
        let mut dashboard = DashboardV5State::new();
        dashboard.record_metric(MetricV5::ZkpV7ShardsRegistered, 3.0, None);
        dashboard.record_metric(MetricV5::CrossPoolSessionsActive, 2.0, None);
        dashboard.record_metric(MetricV5::GovernanceAuditEntries, 100.0, None);
        dashboard.record_metric(MetricV5::NetworkActiveConnections, 50.0, None);
        dashboard.record_metric(MetricV5::SystemCpuPercent, 45.0, None);
        let snapshot = dashboard.get_snapshot().unwrap();
        assert_eq!(snapshot.zkp_v7.shards_registered, 3);
        assert_eq!(snapshot.cross_pool.sessions_active, 2);
        assert_eq!(snapshot.governance_v4.audit_entries, 100);
        assert_eq!(snapshot.network_connections, 50);
        assert_eq!(snapshot.system_cpu_percent, 45.0);
        assert!(snapshot.alerts.is_empty());
    }

    #[test]
    fn test_multiple_alerts() {
        let mut dashboard = DashboardV5State::new();
        dashboard.record_metric(MetricV5::SystemCpuPercent, 95.0, None);
        dashboard.record_metric(MetricV5::SystemMemoryPercent, 90.0, None);
        assert_eq!(dashboard.active_alert_count(), 2);
    }

    #[test]
    fn test_no_alert_when_safe() {
        let mut dashboard = DashboardV5State::new();
        dashboard.record_metric(MetricV5::SystemCpuPercent, 50.0, None);
        dashboard.record_metric(MetricV5::ZkpV7ThroughputProofsPerSec, 20.0, None);
        assert_eq!(dashboard.active_alert_count(), 0);
    }

    #[test]
    fn test_error_display() {
        match DashboardV5Error::MetricUnavailable("test".to_string()) {
            e => assert!(format!("{}", e).contains("test")),
        }
    }

    #[test]
    fn test_dashboard_default() {
        let dashboard = DashboardV5State::default();
        assert_eq!(dashboard.history.len(), 0);
    }

    #[test]
    fn test_get_config() {
        let dashboard = DashboardV5State::new();
        let config = dashboard.get_config();
        assert_eq!(config.max_history, 5000);
    }

    #[test]
    fn test_get_stats() {
        let mut dashboard = DashboardV5State::new();
        dashboard.record_metric(MetricV5::ZkpV7ProofsGenerated, 10.0, None);
        let stats = dashboard.get_stats();
        assert_eq!(stats.metrics_recorded, 1);
    }

    #[test]
    fn test_metric_value_new() {
        let mv = MetricValueV5::new(MetricV5::ZkpV7ProofsGenerated, 42.0, Some("src".to_string()));
        assert_eq!(mv.value, 42.0);
        assert_eq!(mv.source, Some("src".to_string()));
    }

    #[test]
    fn test_zkp_summary_new() {
        let s = ZkpV7Summary::new(5, 3, 100, 2.5, 20.0, 150.0, 2);
        assert_eq!(s.shards_registered, 5);
        assert_eq!(s.avg_aggregation_depth, 2.5);
    }

    #[test]
    fn test_cross_pool_summary_new() {
        let s = CrossPoolSummary::new(3, 10, 0.85, 0.9, 1, 50.0);
        assert_eq!(s.sessions_active, 3);
        assert_eq!(s.avg_reputation, 0.85);
    }

    #[test]
    fn test_governance_summary_new() {
        let s = GovernanceV4Summary::new(500, 5, 0.95, 25, 0.8, 200.0);
        assert_eq!(s.audit_entries, 500);
        assert_eq!(s.compliance_score, 0.95);
    }

    #[test]
    fn test_alert_severity_levels() {
        let mut dashboard = DashboardV5State::new();
        dashboard.record_metric(MetricV5::SystemCpuPercent, 95.0, None);
        assert_eq!(dashboard.alerts[0].severity, 5);
        dashboard.clear_old_alerts(0);
        dashboard.reset_rate_limit();
        dashboard.record_metric(MetricV5::ZkpV7ThroughputProofsPerSec, 5.0, None);
        assert_eq!(dashboard.alerts[0].severity, 4);
    }

    #[test]
    fn test_snapshot_includes_alerts() {
        let mut dashboard = DashboardV5State::new();
        dashboard.record_metric(MetricV5::SystemCpuPercent, 95.0, None);
        let snapshot = dashboard.get_snapshot().unwrap();
        assert_eq!(snapshot.alerts.len(), 1);
    }
}
