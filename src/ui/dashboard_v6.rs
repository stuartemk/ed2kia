//! Dashboard v6 — Unified state engine with Federation Scaling v5, Async ZKP v10 & Bridge v4
//!
//! LP-125: UI Dashboard v6 & Real-time Streams
//! Extends Dashboard v5 with metrics from Federation Scaling v5 (reputation-weighted sharding,
//! partition tolerance, cross-model delegation), Async ZKP v10 (ML cost prediction, cross-federation
//! delegation, replay protection) and Federation ZKP Bridge v4 (reputation routing, consensus,
//! Merkle root sync).
//!
//! Features:
//! - Federation Scaling v5 summary: nodes, shards, partition health, delegation stats
//! - Async ZKP v10 summary: proofs submitted/verified, cost prediction accuracy, delegation depth
//! - Bridge v4 summary: routed proofs, consensus rate, cross-federation aggregations
//! - Integrated alerts v6: scaling threshold breaches, ZKP replay attempts, bridge consensus failures
//! - Unified snapshot v6 with all sections
//!
//! Protected with `#[cfg(feature = "v1.5-sprint2")]`.

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ─── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum DashboardV6Error {
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
pub enum MetricV6 {
    // Federation Scaling v5 metrics
    ScalingV5NodesActive,
    ScalingV5ShardsActive,
    ScalingV5PartitionHealth,
    ScalingV5AssignmentsSuccess,
    ScalingV5AssignmentsFailed,
    ScalingV5Rebalances,
    ScalingV5CrossModelSyncs,
    ScalingV5AvgReputation,
    ScalingV5AvgLatencyMs,
    // Async ZKP v10 metrics
    ZkpV10ProofsSubmitted,
    ZkpV10ProofsVerified,
    ZkpV10ProofsFailed,
    ZkpV10Delegations,
    ZkpV10ReplaysDetected,
    ZkpV10AvgVerificationMs,
    ZkpV10AvgPredictedCost,
    ZkpV10CostPredictionAccuracy,
    // Bridge v4 metrics
    BridgeV4ProofsRouted,
    BridgeV4ProofsVerified,
    BridgeV4ConsensusFailures,
    BridgeV4AvgRoutingMs,
    BridgeV4CrossFedAggregations,
    // Network metrics
    NetworkActiveConnections,
    NetworkBandwidthMbits,
    NetworkLatencyP99,
    // System metrics
    SystemCpuPercent,
    SystemMemoryPercent,
}

impl std::fmt::Display for MetricV6 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricV6::ScalingV5NodesActive => write!(f, "scaling_v5.nodes_active"),
            MetricV6::ScalingV5ShardsActive => write!(f, "scaling_v5.shards_active"),
            MetricV6::ScalingV5PartitionHealth => write!(f, "scaling_v5.partition_health"),
            MetricV6::ScalingV5AssignmentsSuccess => write!(f, "scaling_v5.assignments_success"),
            MetricV6::ScalingV5AssignmentsFailed => write!(f, "scaling_v5.assignments_failed"),
            MetricV6::ScalingV5Rebalances => write!(f, "scaling_v5.rebalances"),
            MetricV6::ScalingV5CrossModelSyncs => write!(f, "scaling_v5.cross_model_syncs"),
            MetricV6::ScalingV5AvgReputation => write!(f, "scaling_v5.avg_reputation"),
            MetricV6::ScalingV5AvgLatencyMs => write!(f, "scaling_v5.avg_latency_ms"),
            MetricV6::ZkpV10ProofsSubmitted => write!(f, "zkp_v10.proofs_submitted"),
            MetricV6::ZkpV10ProofsVerified => write!(f, "zkp_v10.proofs_verified"),
            MetricV6::ZkpV10ProofsFailed => write!(f, "zkp_v10.proofs_failed"),
            MetricV6::ZkpV10Delegations => write!(f, "zkp_v10.delegations"),
            MetricV6::ZkpV10ReplaysDetected => write!(f, "zkp_v10.replays_detected"),
            MetricV6::ZkpV10AvgVerificationMs => write!(f, "zkp_v10.avg_verification_ms"),
            MetricV6::ZkpV10AvgPredictedCost => write!(f, "zkp_v10.avg_predicted_cost"),
            MetricV6::ZkpV10CostPredictionAccuracy => write!(f, "zkp_v10.cost_prediction_accuracy"),
            MetricV6::BridgeV4ProofsRouted => write!(f, "bridge_v4.proofs_routed"),
            MetricV6::BridgeV4ProofsVerified => write!(f, "bridge_v4.proofs_verified"),
            MetricV6::BridgeV4ConsensusFailures => write!(f, "bridge_v4.consensus_failures"),
            MetricV6::BridgeV4AvgRoutingMs => write!(f, "bridge_v4.avg_routing_ms"),
            MetricV6::BridgeV4CrossFedAggregations => write!(f, "bridge_v4.cross_fed_aggregations"),
            MetricV6::NetworkActiveConnections => write!(f, "network.active_connections"),
            MetricV6::NetworkBandwidthMbits => write!(f, "network.bandwidth_mbits"),
            MetricV6::NetworkLatencyP99 => write!(f, "network.latency_p99"),
            MetricV6::SystemCpuPercent => write!(f, "system.cpu_percent"),
            MetricV6::SystemMemoryPercent => write!(f, "system.memory_percent"),
        }
    }
}

// ─── Metric Value ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValueV6 {
    pub metric: MetricV6,
    pub value: f64,
    pub source: Option<String>,
    pub timestamp_ms: u64,
}

impl MetricValueV6 {
    pub fn new(metric: MetricV6, value: f64, source: Option<String>) -> Self {
        Self {
            metric,
            value,
            source,
            timestamp_ms: current_timestamp_ms(),
        }
    }
}

// ─── Federation Scaling v5 Summary ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingV5Summary {
    /// Total active nodes.
    pub nodes_active: usize,
    /// Total active shards.
    pub shards_active: usize,
    /// Partition health ratio (0.0-1.0).
    pub partition_health: f64,
    /// Successful assignments.
    pub assignments_success: u64,
    /// Failed assignments.
    pub assignments_failed: u64,
    /// Total rebalances performed.
    pub rebalances: u64,
    /// Cross-model synchronization count.
    pub cross_model_syncs: u64,
    /// Average node reputation.
    pub avg_reputation: f64,
    /// Average node latency in ms.
    pub avg_latency_ms: f64,
}

impl Default for ScalingV5Summary {
    fn default() -> Self {
        Self {
            nodes_active: 0,
            shards_active: 0,
            partition_health: 0.0,
            assignments_success: 0,
            assignments_failed: 0,
            rebalances: 0,
            cross_model_syncs: 0,
            avg_reputation: 0.0,
            avg_latency_ms: 0.0,
        }
    }
}

impl ScalingV5Summary {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        nodes_active: usize,
        shards_active: usize,
        partition_health: f64,
        assignments_success: u64,
        assignments_failed: u64,
        rebalances: u64,
        cross_model_syncs: u64,
        avg_reputation: f64,
        avg_latency_ms: f64,
    ) -> Self {
        Self {
            nodes_active,
            shards_active,
            partition_health,
            assignments_success,
            assignments_failed,
            rebalances,
            cross_model_syncs,
            avg_reputation,
            avg_latency_ms,
        }
    }

    pub fn is_partition_healthy(&self) -> bool {
        self.partition_health >= 0.995
    }

    pub fn assignment_success_rate(&self) -> f64 {
        let total = self.assignments_success + self.assignments_failed;
        if total == 0 {
            return 1.0;
        }
        self.assignments_success as f64 / total as f64
    }
}

// ─── Async ZKP v10 Summary ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkpV10Summary {
    /// Total proofs submitted.
    pub proofs_submitted: u64,
    /// Total proofs verified.
    pub proofs_verified: u64,
    /// Total proofs failed.
    pub proofs_failed: u64,
    /// Total cross-federation delegations.
    pub delegations: u64,
    /// Replay attempts detected.
    pub replays_detected: u64,
    /// Average verification time in ms.
    pub avg_verification_ms: f64,
    /// Average predicted proof cost.
    pub avg_predicted_cost: f64,
    /// Cost prediction accuracy (0.0-1.0).
    pub cost_prediction_accuracy: f64,
}

impl Default for ZkpV10Summary {
    fn default() -> Self {
        Self {
            proofs_submitted: 0,
            proofs_verified: 0,
            proofs_failed: 0,
            delegations: 0,
            replays_detected: 0,
            avg_verification_ms: 0.0,
            avg_predicted_cost: 0.0,
            cost_prediction_accuracy: 0.0,
        }
    }
}

impl ZkpV10Summary {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        proofs_submitted: u64,
        proofs_verified: u64,
        proofs_failed: u64,
        delegations: u64,
        replays_detected: u64,
        avg_verification_ms: f64,
        avg_predicted_cost: f64,
        cost_prediction_accuracy: f64,
    ) -> Self {
        Self {
            proofs_submitted,
            proofs_verified,
            proofs_failed,
            delegations,
            replays_detected,
            avg_verification_ms,
            avg_predicted_cost,
            cost_prediction_accuracy,
        }
    }

    pub fn verification_rate(&self) -> f64 {
        let total = self.proofs_verified + self.proofs_failed;
        if total == 0 {
            return 1.0;
        }
        self.proofs_verified as f64 / total as f64
    }
}

// ─── Bridge v4 Summary ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeV4Summary {
    /// Total proofs routed.
    pub proofs_routed: u64,
    /// Total proofs verified via bridge.
    pub proofs_verified: u64,
    /// Consensus failure count.
    pub consensus_failures: u64,
    /// Average routing time in ms.
    pub avg_routing_ms: f64,
    /// Cross-federation aggregation count.
    pub cross_fed_aggregations: u64,
}

impl Default for BridgeV4Summary {
    fn default() -> Self {
        Self {
            proofs_routed: 0,
            proofs_verified: 0,
            consensus_failures: 0,
            avg_routing_ms: 0.0,
            cross_fed_aggregations: 0,
        }
    }
}

impl BridgeV4Summary {
    pub fn new(
        proofs_routed: u64,
        proofs_verified: u64,
        consensus_failures: u64,
        avg_routing_ms: f64,
        cross_fed_aggregations: u64,
    ) -> Self {
        Self {
            proofs_routed,
            proofs_verified,
            consensus_failures,
            avg_routing_ms,
            cross_fed_aggregations,
        }
    }

    pub fn consensus_success_rate(&self) -> f64 {
        let total = self.proofs_verified + self.consensus_failures;
        if total == 0 {
            return 1.0;
        }
        self.proofs_verified as f64 / total as f64
    }
}

// ─── Alert System ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertSeverity::Info => write!(f, "info"),
            AlertSeverity::Warning => write!(f, "warning"),
            AlertSeverity::Critical => write!(f, "critical"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertV6 {
    pub id: String,
    pub severity: AlertSeverity,
    pub category: String,
    pub message: String,
    pub timestamp_ms: u64,
}

impl AlertV6 {
    pub fn new(id: String, severity: AlertSeverity, category: String, message: String) -> Self {
        Self {
            id,
            severity,
            category,
            message,
            timestamp_ms: current_timestamp_ms(),
        }
    }
}

// ─── Dashboard Snapshot ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSnapshotV6 {
    pub timestamp_ms: u64,
    pub scaling_v5: ScalingV5Summary,
    pub zkp_v10: ZkpV10Summary,
    pub bridge_v4: BridgeV4Summary,
    pub alerts: Vec<AlertV6>,
    pub metrics: Vec<MetricValueV6>,
}

impl Default for DashboardSnapshotV6 {
    fn default() -> Self {
        Self {
            timestamp_ms: current_timestamp_ms(),
            scaling_v5: ScalingV5Summary::default(),
            zkp_v10: ZkpV10Summary::default(),
            bridge_v4: BridgeV4Summary::default(),
            alerts: Vec::new(),
            metrics: Vec::new(),
        }
    }
}

impl DashboardSnapshotV6 {
    pub fn new(
        scaling_v5: ScalingV5Summary,
        zkp_v10: ZkpV10Summary,
        bridge_v4: BridgeV4Summary,
    ) -> Self {
        Self {
            timestamp_ms: current_timestamp_ms(),
            scaling_v5,
            zkp_v10,
            bridge_v4,
            alerts: Vec::new(),
            metrics: Vec::new(),
        }
    }

    pub fn add_alert(&mut self, alert: AlertV6) {
        self.alerts.push(alert);
    }

    pub fn add_metric(&mut self, metric: MetricValueV6) {
        self.metrics.push(metric);
    }

    /// Generate alerts based on current state thresholds.
    pub fn generate_alerts(&mut self) {
        // Partition tolerance alert
        if !self.scaling_v5.is_partition_healthy() {
            self.add_alert(AlertV6::new(
                "partition_health".into(),
                AlertSeverity::Critical,
                "scaling_v5".into(),
                format!(
                    "Partition health {:.2}% below 99.5% threshold",
                    self.scaling_v5.partition_health * 100.0
                ),
            ));
        }

        // Assignment failure alert
        let rate = self.scaling_v5.assignment_success_rate();
        if rate < 0.95 {
            self.add_alert(AlertV6::new(
                "assignment_failures".into(),
                AlertSeverity::Warning,
                "scaling_v5".into(),
                format!("Assignment success rate {:.2}% below 95%", rate * 100.0),
            ));
        }

        // ZKP replay detection alert
        if self.zkp_v10.replays_detected > 0 {
            self.add_alert(AlertV6::new(
                "replay_detected".into(),
                AlertSeverity::Critical,
                "zkp_v10".into(),
                format!("{} replay attempts detected", self.zkp_v10.replays_detected),
            ));
        }

        // ZKP verification rate alert
        let v_rate = self.zkp_v10.verification_rate();
        if v_rate < 0.90 {
            self.add_alert(AlertV6::new(
                "zkp_verification_rate".into(),
                AlertSeverity::Warning,
                "zkp_v10".into(),
                format!("ZKP verification rate {:.2}% below 90%", v_rate * 100.0),
            ));
        }

        // Bridge consensus failure alert
        let c_rate = self.bridge_v4.consensus_success_rate();
        if c_rate < 0.95 {
            self.add_alert(AlertV6::new(
                "bridge_consensus".into(),
                AlertSeverity::Warning,
                "bridge_v4".into(),
                format!("Bridge consensus rate {:.2}% below 95%", c_rate * 100.0),
            ));
        }
    }
}

// ─── Dashboard Engine ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DashboardV6Stats {
    pub snapshots_generated: u64,
    pub alerts_triggered: u64,
    pub metrics_recorded: u64,
}

pub struct DashboardV6 {
    pub scaling_v5: ScalingV5Summary,
    pub zkp_v10: ZkpV10Summary,
    pub bridge_v4: BridgeV4Summary,
    pub alerts: Vec<AlertV6>,
    pub metrics: Vec<MetricValueV6>,
    pub stats: DashboardV6Stats,
}

impl DashboardV6 {
    pub fn new() -> Self {
        Self {
            scaling_v5: ScalingV5Summary::default(),
            zkp_v10: ZkpV10Summary::default(),
            bridge_v4: BridgeV4Summary::default(),
            alerts: Vec::new(),
            metrics: Vec::new(),
            stats: DashboardV6Stats::default(),
        }
    }

    pub fn update_scaling_v5(&mut self, summary: ScalingV5Summary) {
        self.scaling_v5 = summary;
    }

    pub fn update_zkp_v10(&mut self, summary: ZkpV10Summary) {
        self.zkp_v10 = summary;
    }

    pub fn update_bridge_v4(&mut self, summary: BridgeV4Summary) {
        self.bridge_v4 = summary;
    }

    pub fn record_metric(&mut self, metric: MetricValueV6) {
        self.metrics.push(metric);
        self.stats.metrics_recorded += 1;
    }

    pub fn generate_snapshot(&mut self) -> DashboardSnapshotV6 {
        let mut snapshot = DashboardSnapshotV6::new(
            self.scaling_v5.clone(),
            self.zkp_v10.clone(),
            self.bridge_v4.clone(),
        );
        snapshot.alerts = self.alerts.clone();
        snapshot.metrics = self.metrics.clone();
        snapshot.generate_alerts();
        self.stats.snapshots_generated += 1;
        self.stats.alerts_triggered += snapshot.alerts.len() as u64;
        snapshot
    }

    pub fn clear_alerts(&mut self) {
        self.alerts.clear();
    }

    pub fn clear_metrics(&mut self) {
        self.metrics.clear();
    }

    pub fn reset_stats(&mut self) {
        self.stats = DashboardV6Stats::default();
    }
}

impl Default for DashboardV6 {
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

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_creation() {
        let dashboard = DashboardV6::new();
        assert_eq!(dashboard.scaling_v5.nodes_active, 0);
        assert_eq!(dashboard.zkp_v10.proofs_submitted, 0);
        assert_eq!(dashboard.bridge_v4.proofs_routed, 0);
    }

    #[test]
    fn test_update_scaling_v5() {
        let mut dashboard = DashboardV6::new();
        dashboard.update_scaling_v5(ScalingV5Summary::new(
            10, 5, 0.998, 100, 2, 5, 10, 0.85, 45.0,
        ));
        assert_eq!(dashboard.scaling_v5.nodes_active, 10);
        assert_eq!(dashboard.scaling_v5.shards_active, 5);
    }

    #[test]
    fn test_update_zkp_v10() {
        let mut dashboard = DashboardV6::new();
        dashboard.update_zkp_v10(ZkpV10Summary::new(200, 180, 15, 20, 3, 250.0, 45.0, 0.92));
        assert_eq!(dashboard.zkp_v10.proofs_submitted, 200);
        assert_eq!(dashboard.zkp_v10.replays_detected, 3);
    }

    #[test]
    fn test_update_bridge_v4() {
        let mut dashboard = DashboardV6::new();
        dashboard.update_bridge_v4(BridgeV4Summary::new(150, 140, 5, 30.0, 10));
        assert_eq!(dashboard.bridge_v4.proofs_routed, 150);
        assert_eq!(dashboard.bridge_v4.consensus_failures, 5);
    }

    #[test]
    fn test_partition_health_alert() {
        let mut dashboard = DashboardV6::new();
        dashboard.update_scaling_v5(ScalingV5Summary::new(
            10, 5, 0.990, 100, 2, 5, 10, 0.85, 45.0,
        ));
        let snapshot = dashboard.generate_snapshot();
        assert!(snapshot.alerts.iter().any(|a| a.id == "partition_health"));
    }

    #[test]
    fn test_replay_detection_alert() {
        let mut dashboard = DashboardV6::new();
        dashboard.update_zkp_v10(ZkpV10Summary::new(200, 180, 15, 20, 5, 250.0, 45.0, 0.92));
        let snapshot = dashboard.generate_snapshot();
        assert!(snapshot.alerts.iter().any(|a| a.id == "replay_detected"));
    }

    #[test]
    fn test_assignment_success_rate() {
        let summary = ScalingV5Summary::new(10, 5, 0.998, 95, 5, 5, 10, 0.85, 45.0);
        assert!((summary.assignment_success_rate() - 0.95).abs() < 0.001);
    }

    #[test]
    fn test_zkp_verification_rate() {
        let summary = ZkpV10Summary::new(200, 180, 20, 20, 0, 250.0, 45.0, 0.92);
        assert!((summary.verification_rate() - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_bridge_consensus_rate() {
        let summary = BridgeV4Summary::new(150, 142, 8, 30.0, 10);
        assert!((summary.consensus_success_rate() - 0.947).abs() < 0.001);
    }

    #[test]
    fn test_snapshot_generation() {
        let mut dashboard = DashboardV6::new();
        dashboard.update_scaling_v5(ScalingV5Summary::new(
            10, 5, 0.998, 100, 2, 5, 10, 0.85, 45.0,
        ));
        let snapshot = dashboard.generate_snapshot();
        assert_eq!(snapshot.scaling_v5.nodes_active, 10);
        assert!(snapshot.timestamp_ms > 0);
    }

    #[test]
    fn test_record_metric() {
        let mut dashboard = DashboardV6::new();
        dashboard.record_metric(MetricValueV6::new(
            MetricV6::ScalingV5NodesActive,
            10.0,
            Some("test".into()),
        ));
        assert_eq!(dashboard.stats.metrics_recorded, 1);
    }

    #[test]
    fn test_clear_alerts() {
        let mut dashboard = DashboardV6::new();
        dashboard.alerts.push(AlertV6::new(
            "test".into(),
            AlertSeverity::Info,
            "test".into(),
            "test alert".into(),
        ));
        dashboard.clear_alerts();
        assert!(dashboard.alerts.is_empty());
    }

    #[test]
    fn test_reset_stats() {
        let mut dashboard = DashboardV6::new();
        dashboard.generate_snapshot();
        dashboard.reset_stats();
        assert_eq!(dashboard.stats.snapshots_generated, 0);
    }

    #[test]
    fn test_metric_display() {
        let metric = MetricV6::ScalingV5NodesActive;
        assert_eq!(metric.to_string(), "scaling_v5.nodes_active");
    }

    #[test]
    fn test_alert_severity_display() {
        assert_eq!(AlertSeverity::Critical.to_string(), "critical");
        assert_eq!(AlertSeverity::Warning.to_string(), "warning");
        assert_eq!(AlertSeverity::Info.to_string(), "info");
    }

    #[test]
    fn test_error_display() {
        let err = DashboardV6Error::MetricUnavailable("test".into());
        assert_eq!(err.to_string(), "Métrica no disponible: test");
    }

    #[test]
    fn test_default_snapshot() {
        let snapshot = DashboardSnapshotV6::default();
        assert!(snapshot.alerts.is_empty());
        assert!(snapshot.metrics.is_empty());
        assert!(snapshot.timestamp_ms > 0);
    }

    #[test]
    fn test_no_alerts_when_healthy() {
        let mut dashboard = DashboardV6::new();
        dashboard.update_scaling_v5(ScalingV5Summary::new(
            10, 5, 0.998, 100, 1, 5, 10, 0.85, 45.0,
        ));
        dashboard.update_zkp_v10(ZkpV10Summary::new(200, 190, 10, 20, 0, 250.0, 45.0, 0.92));
        dashboard.update_bridge_v4(BridgeV4Summary::new(150, 148, 2, 30.0, 10));
        let snapshot = dashboard.generate_snapshot();
        assert!(snapshot.alerts.is_empty());
    }

    #[test]
    fn test_multiple_alerts() {
        let mut dashboard = DashboardV6::new();
        dashboard.update_scaling_v5(ScalingV5Summary::new(
            10, 5, 0.990, 50, 50, 5, 10, 0.85, 45.0,
        ));
        dashboard.update_zkp_v10(ZkpV10Summary::new(200, 100, 100, 20, 5, 250.0, 45.0, 0.92));
        let snapshot = dashboard.generate_snapshot();
        assert!(snapshot.alerts.len() >= 2);
    }

    #[test]
    fn test_stats_tracking() {
        let mut dashboard = DashboardV6::new();
        dashboard.record_metric(MetricValueV6::new(
            MetricV6::ScalingV5NodesActive,
            10.0,
            None,
        ));
        dashboard.record_metric(MetricValueV6::new(
            MetricV6::ZkpV10ProofsSubmitted,
            200.0,
            None,
        ));
        dashboard.generate_snapshot();
        assert_eq!(dashboard.stats.metrics_recorded, 2);
        assert_eq!(dashboard.stats.snapshots_generated, 1);
    }
}
