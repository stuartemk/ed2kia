//! Dashboard v2 — Motor de estado y agregación de métricas para el dashboard en tiempo real
//!
//! LP-36: Dashboard UI v2
//! Proporciona el backend de agregación de métricas para visualización en tiempo real,
//! combinando datos de alignment, federación, gobernanza y marketplace.
//!
//! Características:
//! - Agregación de métricas multi-módulo (alignment, federation, governance, marketplace)
//! - Ventanas deslizantes para métricas de rendimiento (latencia, throughput)
//! - Estados de nodo con detección de anomalías
//! - Serialización optimizada para streaming WebSocket/SSE
//! - Rate limiting integrado para consultas de dashboard
//!
//! Protegido con `#[cfg(feature = "v1.1-sprint5")]`.

#[cfg(feature = "v1.1-sprint5")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "v1.1-sprint5")]
use std::collections::{HashMap, VecDeque};
#[cfg(feature = "v1.1-sprint5")]
#[cfg(feature = "v1.1-sprint5")]
use thiserror::Error;

// ─── Errors ───────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint5")]
#[derive(Error, Debug)]
pub enum DashboardError {
    #[error("Métrica no disponible: {0}")]
    MetricUnavailable(String),

    #[error("Rate limit excedido: {current}/{max} consultas/s")]
    RateLimitExceeded { current: usize, max: usize },

    #[error("Error de serialización: {0}")]
    SerializationError(String),
}

// ─── Metric Types ─────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint5")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DashboardMetric {
    // Alignment
    AlignmentDrift,
    AlignmentConfidence,
    AlignmentFeedbackCount,
    SteeringSignalCount,

    // Federation
    FederationNodeCount,
    FederationTrustScore,
    FederationGradientNorm,
    FederationSyncRound,

    // Governance
    GovernanceActiveProposals,
    GovernanceParticipationRate,
    GovernanceReputationAvg,

    // Marketplace
    MarketplaceActiveListings,
    MarketplaceMatchRate,
    MarketplaceEscrowVolume,

    // System
    SystemCpuUsage,
    SystemMemoryUsage,
    SystemNetworkLatency,
    SystemThroughput,
}

#[cfg(feature = "v1.1-sprint5")]
impl std::fmt::Display for DashboardMetric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DashboardMetric::AlignmentDrift => write!(f, "alignment_drift"),
            DashboardMetric::AlignmentConfidence => write!(f, "alignment_confidence"),
            DashboardMetric::AlignmentFeedbackCount => write!(f, "alignment_feedback_count"),
            DashboardMetric::SteeringSignalCount => write!(f, "steering_signal_count"),
            DashboardMetric::FederationNodeCount => write!(f, "federation_node_count"),
            DashboardMetric::FederationTrustScore => write!(f, "federation_trust_score"),
            DashboardMetric::FederationGradientNorm => write!(f, "federation_gradient_norm"),
            DashboardMetric::FederationSyncRound => write!(f, "federation_sync_round"),
            DashboardMetric::GovernanceActiveProposals => write!(f, "governance_active_proposals"),
            DashboardMetric::GovernanceParticipationRate => {
                write!(f, "governance_participation_rate")
            }
            DashboardMetric::GovernanceReputationAvg => write!(f, "governance_reputation_avg"),
            DashboardMetric::MarketplaceActiveListings => {
                write!(f, "marketplace_active_listings")
            }
            DashboardMetric::MarketplaceMatchRate => write!(f, "marketplace_match_rate"),
            DashboardMetric::MarketplaceEscrowVolume => write!(f, "marketplace_escrow_volume"),
            DashboardMetric::SystemCpuUsage => write!(f, "system_cpu_usage"),
            DashboardMetric::SystemMemoryUsage => write!(f, "system_memory_usage"),
            DashboardMetric::SystemNetworkLatency => write!(f, "system_network_latency"),
            DashboardMetric::SystemThroughput => write!(f, "system_throughput"),
        }
    }
}

// ─── Metric Value ─────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint5")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValue {
    pub metric: DashboardMetric,
    pub value: f64,
    pub timestamp_ms: u64,
    pub source_node: Option<String>,
}

#[cfg(feature = "v1.1-sprint5")]
impl MetricValue {
    pub fn new(metric: DashboardMetric, value: f64, source_node: Option<String>) -> Self {
        Self {
            metric,
            value,
            timestamp_ms: current_timestamp_ms(),
            source_node,
        }
    }
}

// ─── Node Status ──────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint5")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Offline,
}

#[cfg(feature = "v1.1-sprint5")]
impl std::fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeStatus::Healthy => write!(f, "healthy"),
            NodeStatus::Degraded => write!(f, "degraded"),
            NodeStatus::Unhealthy => write!(f, "unhealthy"),
            NodeStatus::Offline => write!(f, "offline"),
        }
    }
}

#[cfg(feature = "v1.1-sprint5")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDashboardInfo {
    pub node_id: String,
    pub status: NodeStatus,
    pub last_heartbeat_ms: u64,
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub network_latency_ms: f32,
    pub alignment_confidence: f32,
    pub trust_score: f32,
    pub active_connections: usize,
}

#[cfg(feature = "v1.1-sprint5")]
impl NodeDashboardInfo {
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            status: NodeStatus::Healthy,
            last_heartbeat_ms: current_timestamp_ms(),
            cpu_usage: 0.0,
            memory_usage: 0.0,
            network_latency_ms: 0.0,
            alignment_confidence: 1.0,
            trust_score: 1.0,
            active_connections: 0,
        }
    }

    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat_ms = current_timestamp_ms();
        self.update_status();
    }

    fn update_status(&mut self) {
        let now = current_timestamp_ms();
        let elapsed = now.saturating_sub(self.last_heartbeat_ms);

        if elapsed > 30_000 {
            self.status = NodeStatus::Offline;
        } else if elapsed > 15_000 || self.cpu_usage > 0.9 || self.memory_usage > 0.9 {
            self.status = NodeStatus::Unhealthy;
        } else if elapsed > 5_000 || self.cpu_usage > 0.7 || self.memory_usage > 0.7 {
            self.status = NodeStatus::Degraded;
        } else {
            self.status = NodeStatus::Healthy;
        }
    }
}

// ─── Dashboard Snapshot ───────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint5")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSnapshot {
    pub timestamp_ms: u64,
    pub metrics: HashMap<DashboardMetric, f64>,
    pub nodes: Vec<NodeDashboardInfo>,
    pub alerts: Vec<DashboardAlert>,
    pub summary: DashboardSummary,
}

#[cfg(feature = "v1.1-sprint5")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSummary {
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub degraded_nodes: usize,
    pub unhealthy_nodes: usize,
    pub offline_nodes: usize,
    pub avg_alignment_confidence: f32,
    pub avg_trust_score: f32,
    pub active_alerts: usize,
}

#[cfg(feature = "v1.1-sprint5")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardAlert {
    pub alert_id: String,
    pub severity: AlertSeverity,
    pub metric: DashboardMetric,
    pub message: String,
    pub timestamp_ms: u64,
    pub acknowledged: bool,
}

#[cfg(feature = "v1.1-sprint5")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[cfg(feature = "v1.1-sprint5")]
impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertSeverity::Info => write!(f, "info"),
            AlertSeverity::Warning => write!(f, "warning"),
            AlertSeverity::Critical => write!(f, "critical"),
        }
    }
}

// ─── Sliding Window ───────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint5")]
struct SlidingWindow {
    values: VecDeque<(u64, f64)>,
    window_ms: u64,
    max_size: usize,
}

#[cfg(feature = "v1.1-sprint5")]
impl SlidingWindow {
    fn new(window_ms: u64, max_size: usize) -> Self {
        Self {
            values: VecDeque::with_capacity(max_size),
            window_ms,
            max_size,
        }
    }

    fn add(&mut self, timestamp_ms: u64, value: f64) {
        self.values.push_back((timestamp_ms, value));
        self.cleanup(timestamp_ms);
        if self.values.len() > self.max_size {
            self.values.pop_front();
        }
    }

    fn cleanup(&mut self, now_ms: u64) {
        let cutoff = now_ms.saturating_sub(self.window_ms);
        while let Some(&(ts, _)) = self.values.front() {
            if ts < cutoff {
                self.values.pop_front();
            } else {
                break;
            }
        }
    }

    fn avg(&self) -> Option<f64> {
        if self.values.is_empty() {
            return None;
        }
        let sum: f64 = self.values.iter().map(|(_, v)| v).sum();
        Some(sum / self.values.len() as f64)
    }

    fn latest(&self) -> Option<f64> {
        self.values.back().map(|(_, v)| *v)
    }

    fn count(&self) -> usize {
        self.values.len()
    }
}

// ─── Dashboard Config ─────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint5")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    pub window_ms: u64,
    pub max_metric_history: usize,
    pub rate_limit_per_sec: usize,
    pub alert_threshold_cpu: f32,
    pub alert_threshold_memory: f32,
    pub alert_threshold_latency_ms: f32,
    pub alert_threshold_alignment: f32,
    pub alert_threshold_trust: f32,
}

#[cfg(feature = "v1.1-sprint5")]
impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            window_ms: 60_000,
            max_metric_history: 1000,
            rate_limit_per_sec: 50,
            alert_threshold_cpu: 0.85,
            alert_threshold_memory: 0.9,
            alert_threshold_latency_ms: 200.0,
            alert_threshold_alignment: 0.5,
            alert_threshold_trust: 0.3,
        }
    }
}

// ─── Dashboard State ──────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint5")]
pub struct DashboardState {
    config: DashboardConfig,
    windows: HashMap<DashboardMetric, SlidingWindow>,
    nodes: HashMap<String, NodeDashboardInfo>,
    alerts: VecDeque<DashboardAlert>,
    rate_limit_counter: usize,
    rate_limit_window_start: u64,
    alert_counter: u64,
}

#[cfg(feature = "v1.1-sprint5")]
impl DashboardState {
    pub fn new() -> Self {
        Self::with_config(DashboardConfig::default())
    }

    pub fn with_config(config: DashboardConfig) -> Self {
        Self {
            config,
            windows: HashMap::new(),
            nodes: HashMap::new(),
            alerts: VecDeque::new(),
            rate_limit_counter: 0,
            rate_limit_window_start: current_timestamp_ms(),
            alert_counter: 0,
        }
    }

    // ─── Rate Limiting ────────────────────────────────────────────────────────

    #[cfg(feature = "v1.1-sprint5")]
    pub fn check_rate_limit(&mut self) -> Result<(), DashboardError> {
        let now = current_timestamp_ms();
        if now.saturating_sub(self.rate_limit_window_start) > 1000 {
            self.rate_limit_counter = 0;
            self.rate_limit_window_start = now;
        }
        self.rate_limit_counter += 1;
        if self.rate_limit_counter > self.config.rate_limit_per_sec {
            return Err(DashboardError::RateLimitExceeded {
                current: self.rate_limit_counter,
                max: self.config.rate_limit_per_sec,
            });
        }
        Ok(())
    }

    // ─── Metric Recording ─────────────────────────────────────────────────────

    #[cfg(feature = "v1.1-sprint5")]
    pub fn record_metric(&mut self, metric: DashboardMetric, value: f64, _source_node: Option<String>) {
        let now = current_timestamp_ms();
        let window = self
            .windows
            .entry(metric.clone())
            .or_insert_with(|| SlidingWindow::new(self.config.window_ms, self.config.max_metric_history));
        window.add(now, value);

        // Check alert thresholds
        self.check_metric_alerts(&metric, value);
    }

    #[cfg(feature = "v1.1-sprint5")]
    fn check_metric_alerts(&mut self, metric: &DashboardMetric, value: f64) {
        let (severity, threshold) = match metric {
            DashboardMetric::SystemCpuUsage => {
                if value > self.config.alert_threshold_cpu as f64 {
                    (AlertSeverity::Critical, self.config.alert_threshold_cpu as f64)
                } else if value > (self.config.alert_threshold_cpu * 0.9) as f64 {
                    (AlertSeverity::Warning, self.config.alert_threshold_cpu as f64)
                } else {
                    return;
                }
            }
            DashboardMetric::SystemMemoryUsage => {
                if value > self.config.alert_threshold_memory as f64 {
                    (AlertSeverity::Critical, self.config.alert_threshold_memory as f64)
                } else if value > (self.config.alert_threshold_memory * 0.9) as f64 {
                    (AlertSeverity::Warning, self.config.alert_threshold_memory as f64)
                } else {
                    return;
                }
            }
            DashboardMetric::SystemNetworkLatency => {
                if value > self.config.alert_threshold_latency_ms as f64 {
                    (AlertSeverity::Warning, self.config.alert_threshold_latency_ms as f64)
                } else {
                    return;
                }
            }
            DashboardMetric::AlignmentConfidence => {
                if value < self.config.alert_threshold_alignment as f64 {
                    (AlertSeverity::Critical, self.config.alert_threshold_alignment as f64)
                } else if value < (self.config.alert_threshold_alignment * 1.2) as f64 {
                    (AlertSeverity::Warning, self.config.alert_threshold_alignment as f64)
                } else {
                    return;
                }
            }
            DashboardMetric::FederationTrustScore => {
                if value < self.config.alert_threshold_trust as f64 {
                    (AlertSeverity::Critical, self.config.alert_threshold_trust as f64)
                } else if value < (self.config.alert_threshold_trust * 1.2) as f64 {
                    (AlertSeverity::Warning, self.config.alert_threshold_trust as f64)
                } else {
                    return;
                }
            }
            _ => return,
        };

        self.alert_counter += 1;
        let alert = DashboardAlert {
            alert_id: format!("alert-{}", self.alert_counter),
            severity,
            metric: metric.clone(),
            message: format!(
                "{} value {:.4} exceeded threshold {:.4}",
                metric, value, threshold
            ),
            timestamp_ms: current_timestamp_ms(),
            acknowledged: false,
        };
        self.alerts.push_back(alert);

        // Keep max 100 alerts
        if self.alerts.len() > 100 {
            self.alerts.pop_front();
        }
    }

    // ─── Node Management ──────────────────────────────────────────────────────

    #[cfg(feature = "v1.1-sprint5")]
    pub fn register_node(&mut self, node_id: String) {
        let info = NodeDashboardInfo::new(node_id.clone());
        self.nodes.insert(node_id, info);
    }

    #[cfg(feature = "v1.1-sprint5")]
    pub fn update_node(&mut self, node_id: String, info: NodeDashboardInfo) {
        self.nodes.insert(node_id, info);
    }

    #[cfg(feature = "v1.1-sprint5")]
    pub fn remove_node(&mut self, node_id: &str) {
        self.nodes.remove(node_id);
    }

    #[cfg(feature = "v1.1-sprint5")]
    pub fn heartbeat_node(&mut self, node_id: &str) {
        if let Some(info) = self.nodes.get_mut(node_id) {
            info.update_heartbeat();
        }
    }

    // ─── Snapshot Generation ──────────────────────────────────────────────────

    #[cfg(feature = "v1.1-sprint5")]
    pub fn get_snapshot(&mut self) -> Result<DashboardSnapshot, DashboardError> {
        self.check_rate_limit()?;

        let now = current_timestamp_ms();
        let mut metrics = HashMap::new();

        for (metric, window) in &self.windows {
            if let Some(avg) = window.avg() {
                metrics.insert(metric.clone(), avg);
            }
        }

        let nodes: Vec<NodeDashboardInfo> = self.nodes.values().cloned().collect();
        let alerts: Vec<DashboardAlert> = self.alerts.iter().filter(|a| !a.acknowledged).cloned().collect();

        let healthy = nodes.iter().filter(|n| n.status == NodeStatus::Healthy).count();
        let degraded = nodes.iter().filter(|n| n.status == NodeStatus::Degraded).count();
        let unhealthy = nodes.iter().filter(|n| n.status == NodeStatus::Unhealthy).count();
        let offline = nodes.iter().filter(|n| n.status == NodeStatus::Offline).count();

        let avg_alignment = if nodes.is_empty() {
            0.0
        } else {
            nodes.iter().map(|n| n.alignment_confidence).sum::<f32>() / nodes.len() as f32
        };

        let avg_trust = if nodes.is_empty() {
            0.0
        } else {
            nodes.iter().map(|n| n.trust_score).sum::<f32>() / nodes.len() as f32
        };

        let summary = DashboardSummary {
            total_nodes: nodes.len(),
            healthy_nodes: healthy,
            degraded_nodes: degraded,
            unhealthy_nodes: unhealthy,
            offline_nodes: offline,
            avg_alignment_confidence: avg_alignment,
            avg_trust_score: avg_trust,
            active_alerts: alerts.len(),
        };

        Ok(DashboardSnapshot {
            timestamp_ms: now,
            metrics,
            nodes,
            alerts,
            summary,
        })
    }

    // ─── Alert Management ─────────────────────────────────────────────────────

    #[cfg(feature = "v1.1-sprint5")]
    pub fn acknowledge_alert(&mut self, alert_id: &str) -> bool {
        for alert in &mut self.alerts {
            if alert.alert_id == alert_id {
                alert.acknowledged = true;
                return true;
            }
        }
        false
    }

    #[cfg(feature = "v1.1-sprint5")]
    pub fn clear_old_alerts(&mut self, max_age_ms: u64) -> usize {
        let now = current_timestamp_ms();
        let cutoff = now.saturating_sub(max_age_ms);
        let before = self.alerts.len();
        self.alerts.retain(|a| a.timestamp_ms >= cutoff);
        before - self.alerts.len()
    }

    // ─── Metric Queries ───────────────────────────────────────────────────────

    #[cfg(feature = "v1.1-sprint5")]
    pub fn get_metric_avg(&self, metric: &DashboardMetric) -> Option<f64> {
        self.windows.get(metric).and_then(|w| w.avg())
    }

    #[cfg(feature = "v1.1-sprint5")]
    pub fn get_metric_latest(&self, metric: &DashboardMetric) -> Option<f64> {
        self.windows.get(metric).and_then(|w| w.latest())
    }

    #[cfg(feature = "v1.1-sprint5")]
    pub fn get_metric_history(&self, metric: &DashboardMetric) -> Vec<(u64, f64)> {
        self.windows
            .get(metric)
            .map(|w| w.values.iter().cloned().collect())
            .unwrap_or_default()
    }
}

#[cfg(feature = "v1.1-sprint5")]
impl Default for DashboardState {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

#[cfg(feature = "v1.1-sprint5")]
fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_dashboard_creation() {
        let dashboard = DashboardState::new();
        assert_eq!(dashboard.nodes.len(), 0);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_dashboard_with_config() {
        let config = DashboardConfig {
            rate_limit_per_sec: 100,
            ..DashboardConfig::default()
        };
        let dashboard = DashboardState::with_config(config);
        assert_eq!(dashboard.config.rate_limit_per_sec, 100);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_record_metric() {
        let mut dashboard = DashboardState::new();
        dashboard.record_metric(DashboardMetric::SystemCpuUsage, 0.5, None);
        assert_eq!(dashboard.get_metric_latest(&DashboardMetric::SystemCpuUsage), Some(0.5));
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_record_multiple_metrics() {
        let mut dashboard = DashboardState::new();
        dashboard.record_metric(DashboardMetric::SystemCpuUsage, 0.4, None);
        dashboard.record_metric(DashboardMetric::SystemCpuUsage, 0.6, None);
        let avg = dashboard.get_metric_avg(&DashboardMetric::SystemCpuUsage);
        assert!(avg.is_some());
        assert!((avg.unwrap() - 0.5).abs() < 0.01);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_register_node() {
        let mut dashboard = DashboardState::new();
        dashboard.register_node("node-1".to_string());
        assert_eq!(dashboard.nodes.len(), 1);
        assert!(dashboard.nodes.contains_key("node-1"));
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_node_heartbeat() {
        let mut dashboard = DashboardState::new();
        dashboard.register_node("node-1".to_string());
        dashboard.heartbeat_node("node-1");
        let node = dashboard.nodes.get("node-1").unwrap();
        assert_eq!(node.status, NodeStatus::Healthy);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_snapshot_generation() {
        let mut dashboard = DashboardState::new();
        dashboard.register_node("node-1".to_string());
        dashboard.record_metric(DashboardMetric::SystemCpuUsage, 0.5, None);
        let snapshot = dashboard.get_snapshot().unwrap();
        assert_eq!(snapshot.summary.total_nodes, 1);
        assert!(snapshot.metrics.contains_key(&DashboardMetric::SystemCpuUsage));
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_alert_generation() {
        let mut dashboard = DashboardState::new();
        dashboard.record_metric(DashboardMetric::SystemCpuUsage, 0.95, None);
        let snapshot = dashboard.get_snapshot().unwrap();
        assert!(snapshot.alerts.len() > 0);
        assert_eq!(snapshot.alerts[0].severity, AlertSeverity::Critical);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_alert_acknowledgment() {
        let mut dashboard = DashboardState::new();
        dashboard.record_metric(DashboardMetric::SystemCpuUsage, 0.95, None);
        assert!(dashboard.acknowledge_alert("alert-1"));
        let snapshot = dashboard.get_snapshot().unwrap();
        assert_eq!(snapshot.summary.active_alerts, 0);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_rate_limiting() {
        let config = DashboardConfig {
            rate_limit_per_sec: 5,
            ..DashboardConfig::default()
        };
        let mut dashboard = DashboardState::with_config(config);
        for _ in 0..5 {
            assert!(dashboard.check_rate_limit().is_ok());
        }
        assert!(dashboard.check_rate_limit().is_err());
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_remove_node() {
        let mut dashboard = DashboardState::new();
        dashboard.register_node("node-1".to_string());
        dashboard.remove_node("node-1");
        assert_eq!(dashboard.nodes.len(), 0);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_clear_old_alerts() {
        let mut dashboard = DashboardState::new();
        dashboard.record_metric(DashboardMetric::SystemCpuUsage, 0.95, None);
        assert_eq!(dashboard.alerts.len(), 1);
        // Set alert timestamp to the past so it's considered old
        if let Some(alert) = dashboard.alerts.back_mut() {
            alert.timestamp_ms = 1000;
        }
        // Clear alerts older than now (1 second ago from current time)
        let cleared = dashboard.clear_old_alerts(1);
        assert_eq!(cleared, 1);
        assert_eq!(dashboard.alerts.len(), 0);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_metric_history() {
        let mut dashboard = DashboardState::new();
        dashboard.record_metric(DashboardMetric::SystemCpuUsage, 0.3, None);
        dashboard.record_metric(DashboardMetric::SystemCpuUsage, 0.5, None);
        dashboard.record_metric(DashboardMetric::SystemCpuUsage, 0.7, None);
        let history = dashboard.get_metric_history(&DashboardMetric::SystemCpuUsage);
        assert_eq!(history.len(), 3);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_node_status_display() {
        assert_eq!(NodeStatus::Healthy.to_string(), "healthy");
        assert_eq!(NodeStatus::Degraded.to_string(), "degraded");
        assert_eq!(NodeStatus::Unhealthy.to_string(), "unhealthy");
        assert_eq!(NodeStatus::Offline.to_string(), "offline");
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_alert_severity_display() {
        assert_eq!(AlertSeverity::Info.to_string(), "info");
        assert_eq!(AlertSeverity::Warning.to_string(), "warning");
        assert_eq!(AlertSeverity::Critical.to_string(), "critical");
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_metric_display() {
        assert_eq!(DashboardMetric::SystemCpuUsage.to_string(), "system_cpu_usage");
        assert_eq!(
            DashboardMetric::AlignmentConfidence.to_string(),
            "alignment_confidence"
        );
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_config_default() {
        let config = DashboardConfig::default();
        assert_eq!(config.window_ms, 60_000);
        assert_eq!(config.rate_limit_per_sec, 50);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_dashboard_default() {
        let dashboard = DashboardState::default();
        assert_eq!(dashboard.nodes.len(), 0);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_node_info_creation() {
        let info = NodeDashboardInfo::new("test-node".to_string());
        assert_eq!(info.node_id, "test-node");
        assert_eq!(info.status, NodeStatus::Healthy);
        assert_eq!(info.alignment_confidence, 1.0);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_metric_value_creation() {
        let val = MetricValue::new(DashboardMetric::SystemCpuUsage, 0.5, Some("node-1".into()));
        assert_eq!(val.value, 0.5);
        assert_eq!(val.source_node, Some("node-1".into()));
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_multiple_nodes_snapshot() {
        let mut dashboard = DashboardState::new();
        dashboard.register_node("node-1".to_string());
        dashboard.register_node("node-2".to_string());
        dashboard.register_node("node-3".to_string());
        let snapshot = dashboard.get_snapshot().unwrap();
        assert_eq!(snapshot.summary.total_nodes, 3);
        assert_eq!(snapshot.summary.healthy_nodes, 3);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_alignment_alert() {
        let mut dashboard = DashboardState::new();
        dashboard.record_metric(DashboardMetric::AlignmentConfidence, 0.3, None);
        let snapshot = dashboard.get_snapshot().unwrap();
        assert!(snapshot.alerts.iter().any(|a| a.metric == DashboardMetric::AlignmentConfidence));
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_trust_alert() {
        let mut dashboard = DashboardState::new();
        dashboard.record_metric(DashboardMetric::FederationTrustScore, 0.2, None);
        let snapshot = dashboard.get_snapshot().unwrap();
        assert!(snapshot.alerts.iter().any(|a| a.metric == DashboardMetric::FederationTrustScore));
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_no_alert_for_normal_values() {
        let mut dashboard = DashboardState::new();
        dashboard.record_metric(DashboardMetric::SystemCpuUsage, 0.5, None);
        dashboard.record_metric(DashboardMetric::AlignmentConfidence, 0.9, None);
        let snapshot = dashboard.get_snapshot().unwrap();
        assert_eq!(snapshot.summary.active_alerts, 0);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_update_node() {
        let mut dashboard = DashboardState::new();
        dashboard.register_node("node-1".to_string());
        let mut info = NodeDashboardInfo::new("node-1".to_string());
        info.cpu_usage = 0.75;
        dashboard.update_node("node-1".to_string(), info);
        let node = dashboard.nodes.get("node-1").unwrap();
        assert_eq!(node.cpu_usage, 0.75);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_sliding_window_avg() {
        let mut window = SlidingWindow::new(60_000, 100);
        let now = current_timestamp_ms();
        window.add(now, 10.0);
        window.add(now, 20.0);
        window.add(now, 30.0);
        assert!((window.avg().unwrap() - 20.0).abs() < 0.01);
    }

    #[cfg(feature = "v1.1-sprint5")]
    #[test]
    fn test_sliding_window_cleanup() {
        let mut window = SlidingWindow::new(1000, 100);
        let now = current_timestamp_ms();
        window.add(now - 2000, 10.0);
        window.add(now, 20.0);
        assert_eq!(window.count(), 1);
        assert_eq!(window.avg(), Some(20.0));
    }
}
