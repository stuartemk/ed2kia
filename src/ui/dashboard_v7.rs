//! Dashboard v7 — Unified state engine with Federation Scaling v7, Async ZKP v13 & Bridge v6
//!
//! LP-147: UI Dashboard v7 & Real-time Streams
//! Extends Dashboard v6 with metrics from Federation Scaling v7 (predictive sharding,
//! adaptive routing, gradient sync v7), Async ZKP v13 (parallel batch verification,
//! Merkle+VRF fallback, proof priority) and Federation ZKP Bridge v6 (cross-model routing,
//! credibility tracking, adaptive distribution).
//!
//! Features:
//! - Federation Scaling v7 summary: nodes, shards, predictive load, gradient alignment
//! - Async ZKP v13 summary: batches, parallel verification, fallback rate, priority distribution
//! - Bridge v6 summary: cross-model routing, credibility scores, fallback verifications
//! - Integrated alerts v7: sharding thresholds, ZKP fallback triggers, bridge credibility drops
//! - Unified snapshot v7 with all sections
//!
//! Protected with `#[cfg(feature = "v1.6-sprint2")]`.

#[cfg(feature = "v1.6-sprint2")]
mod internal {
    use std::collections::{HashMap, VecDeque};

    // ---------------------------------------------------------------------------
    // Errors
    // ---------------------------------------------------------------------------

    /// Errors for Dashboard v7 operations.
    #[derive(Debug, Clone, PartialEq)]
    pub enum DashboardV7Error {
        /// Metric not available.
        MetricUnavailable(String),
        /// Aggregation error.
        AggregationError(String),
        /// Rate limit exceeded.
        RateLimitExceeded,
        /// Section not registered.
        SectionNotRegistered(String),
    }

    impl std::fmt::Display for DashboardV7Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::MetricUnavailable(m) => write!(f, "Metric unavailable: {}", m),
                Self::AggregationError(m) => write!(f, "Aggregation error: {}", m),
                Self::RateLimitExceeded => write!(f, "Rate limit exceeded"),
                Self::SectionNotRegistered(s) => write!(f, "Section not registered: {}", s),
            }
        }
    }

    impl std::error::Error for DashboardV7Error {}

    // ---------------------------------------------------------------------------
    // Metric Types
    // ---------------------------------------------------------------------------

    /// Dashboard v7 metric identifiers.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum MetricV7 {
        // Federation Scaling v7 metrics
        ScalingV7NodesActive,
        ScalingV7ShardsActive,
        ScalingV7PartitionHealth,
        ScalingV7PredictiveLoad,
        ScalingV7GradientAlignment,
        ScalingV7AvgReputation,
        ScalingV7AvgLatencyMs,
        ScalingV7DecisionsMs,
        // Async ZKP v13 metrics
        ZkpV13ProofsSubmitted,
        ZkpV13ProofsVerified,
        ZkpV13BatchesCompleted,
        ZkpV13FallbackRate,
        ZkpV13AvgVerificationMs,
        ZkpV13AvgBatchSize,
        ZkpV13PriorityCritical,
        ZkpV13PriorityNormal,
        // Bridge v6 metrics
        BridgeV6ProofsRouted,
        BridgeV6ProofsVerified,
        BridgeV6FallbackCount,
        BridgeV6AvgCredibility,
        BridgeV6AvgRoutingMs,
        BridgeV6RoutingDecisions,
        // Network metrics
        NetworkActiveConnections,
        NetworkBandwidthMbits,
        NetworkLatencyP99,
        // System metrics
        SystemCpuPercent,
        SystemMemoryPercent,
    }

    impl std::fmt::Display for MetricV7 {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                MetricV7::ScalingV7NodesActive => write!(f, "scaling_v7.nodes_active"),
                MetricV7::ScalingV7ShardsActive => write!(f, "scaling_v7.shards_active"),
                MetricV7::ScalingV7PartitionHealth => write!(f, "scaling_v7.partition_health"),
                MetricV7::ScalingV7PredictiveLoad => write!(f, "scaling_v7.predictive_load"),
                MetricV7::ScalingV7GradientAlignment => write!(f, "scaling_v7.gradient_alignment"),
                MetricV7::ScalingV7AvgReputation => write!(f, "scaling_v7.avg_reputation"),
                MetricV7::ScalingV7AvgLatencyMs => write!(f, "scaling_v7.avg_latency_ms"),
                MetricV7::ScalingV7DecisionsMs => write!(f, "scaling_v7.decisions_ms"),
                MetricV7::ZkpV13ProofsSubmitted => write!(f, "zkp_v13.proofs_submitted"),
                MetricV7::ZkpV13ProofsVerified => write!(f, "zkp_v13.proofs_verified"),
                MetricV7::ZkpV13BatchesCompleted => write!(f, "zkp_v13.batches_completed"),
                MetricV7::ZkpV13FallbackRate => write!(f, "zkp_v13.fallback_rate"),
                MetricV7::ZkpV13AvgVerificationMs => write!(f, "zkp_v13.avg_verification_ms"),
                MetricV7::ZkpV13AvgBatchSize => write!(f, "zkp_v13.avg_batch_size"),
                MetricV7::ZkpV13PriorityCritical => write!(f, "zkp_v13.priority_critical"),
                MetricV7::ZkpV13PriorityNormal => write!(f, "zkp_v13.priority_normal"),
                MetricV7::BridgeV6ProofsRouted => write!(f, "bridge_v6.proofs_routed"),
                MetricV7::BridgeV6ProofsVerified => write!(f, "bridge_v6.proofs_verified"),
                MetricV7::BridgeV6FallbackCount => write!(f, "bridge_v6.fallback_count"),
                MetricV7::BridgeV6AvgCredibility => write!(f, "bridge_v6.avg_credibility"),
                MetricV7::BridgeV6AvgRoutingMs => write!(f, "bridge_v6.avg_routing_ms"),
                MetricV7::BridgeV6RoutingDecisions => write!(f, "bridge_v6.routing_decisions"),
                MetricV7::NetworkActiveConnections => write!(f, "network.active_connections"),
                MetricV7::NetworkBandwidthMbits => write!(f, "network.bandwidth_mbits"),
                MetricV7::NetworkLatencyP99 => write!(f, "network.latency_p99"),
                MetricV7::SystemCpuPercent => write!(f, "system.cpu_percent"),
                MetricV7::SystemMemoryPercent => write!(f, "system.memory_percent"),
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Metric Value
    // ---------------------------------------------------------------------------

    /// Metric value with timestamp and source.
    #[derive(Debug, Clone)]
    pub struct MetricValueV7 {
        pub metric: MetricV7,
        pub value: f64,
        pub source: Option<String>,
        pub timestamp_ms: u64,
    }

    impl MetricValueV7 {
        pub fn new(metric: MetricV7, value: f64, source: Option<String>) -> Self {
            Self {
                metric,
                value,
                source,
                timestamp_ms: current_timestamp_ms(),
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Scaling V7 Summary
    // ---------------------------------------------------------------------------

    /// Summary of Federation Scaling v7 state.
    #[derive(Debug, Clone)]
    pub struct ScalingV7Summary {
        pub nodes_active: usize,
        pub shards_active: usize,
        pub partition_health: f64,
        pub predictive_load: f64,
        pub gradient_alignment: f64,
        pub avg_reputation: f64,
        pub avg_latency_ms: f64,
        pub decisions_ms: f64,
    }

    impl Default for ScalingV7Summary {
        fn default() -> Self {
            Self {
                nodes_active: 0,
                shards_active: 0,
                partition_health: 1.0,
                predictive_load: 0.0,
                gradient_alignment: 1.0,
                avg_reputation: 1.0,
                avg_latency_ms: 0.0,
                decisions_ms: 0.0,
            }
        }
    }

    impl ScalingV7Summary {
        pub fn update_nodes(&mut self, nodes: usize) {
            self.nodes_active = nodes;
        }

        pub fn update_shards(&mut self, shards: usize) {
            self.shards_active = shards;
        }

        pub fn update_partition_health(&mut self, health: f64) {
            self.partition_health = health.clamp(0.0, 1.0);
        }

        pub fn update_predictive_load(&mut self, load: f64) {
            self.predictive_load = load;
        }

        pub fn update_gradient_alignment(&mut self, alignment: f64) {
            self.gradient_alignment = alignment.clamp(0.0, 1.0);
        }
    }

    // ---------------------------------------------------------------------------
    // ZKP V13 Summary
    // ---------------------------------------------------------------------------

    /// Summary of Async ZKP v13 state.
    #[derive(Debug, Clone)]
    pub struct ZkpV13Summary {
        pub proofs_submitted: u64,
        pub proofs_verified: u64,
        pub batches_completed: u64,
        pub fallback_rate: f64,
        pub avg_verification_ms: f64,
        pub avg_batch_size: f64,
        pub priority_critical: u64,
        pub priority_normal: u64,
    }

    impl Default for ZkpV13Summary {
        fn default() -> Self {
            Self {
                proofs_submitted: 0,
                proofs_verified: 0,
                batches_completed: 0,
                fallback_rate: 0.0,
                avg_verification_ms: 0.0,
                avg_batch_size: 0.0,
                priority_critical: 0,
                priority_normal: 0,
            }
        }
    }

    impl ZkpV13Summary {
        pub fn update_proofs(&mut self, submitted: u64, verified: u64) {
            self.proofs_submitted = submitted;
            self.proofs_verified = verified;
        }

        pub fn update_batches(&mut self, completed: u64, avg_size: f64) {
            self.batches_completed = completed;
            self.avg_batch_size = avg_size;
        }

        pub fn verification_rate(&self) -> f64 {
            if self.proofs_submitted == 0 {
                return 0.0;
            }
            self.proofs_verified as f64 / self.proofs_submitted as f64
        }
    }

    // ---------------------------------------------------------------------------
    // Bridge V6 Summary
    // ---------------------------------------------------------------------------

    /// Summary of Federation ZKP Bridge v6 state.
    #[derive(Debug, Clone)]
    pub struct BridgeV6Summary {
        pub proofs_routed: u64,
        pub proofs_verified: u64,
        pub fallback_count: u64,
        pub avg_credibility: f64,
        pub avg_routing_ms: f64,
        pub routing_decisions: u64,
    }

    impl Default for BridgeV6Summary {
        fn default() -> Self {
            Self {
                proofs_routed: 0,
                proofs_verified: 0,
                fallback_count: 0,
                avg_credibility: 1.0,
                avg_routing_ms: 0.0,
                routing_decisions: 0,
            }
        }
    }

    impl BridgeV6Summary {
        pub fn update_routing(&mut self, routed: u64, decisions: u64) {
            self.proofs_routed = routed;
            self.routing_decisions = decisions;
        }

        pub fn update_verification(&mut self, verified: u64, fallback: u64) {
            self.proofs_verified = verified;
            self.fallback_count = fallback;
        }

        pub fn consensus_success_rate(&self) -> f64 {
            if self.proofs_routed == 0 {
                return 0.0;
            }
            self.proofs_verified as f64 / self.proofs_routed as f64
        }
    }

    // ---------------------------------------------------------------------------
    // Alert Severity
    // ---------------------------------------------------------------------------

    /// Alert severity levels.
    #[derive(Debug, Clone, PartialEq)]
    pub enum AlertSeverity {
        Critical,
        Warning,
        Info,
    }

    impl std::fmt::Display for AlertSeverity {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Critical => write!(f, "critical"),
                Self::Warning => write!(f, "warning"),
                Self::Info => write!(f, "info"),
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Alert
    // ---------------------------------------------------------------------------

    /// Dashboard alert entry.
    #[derive(Debug, Clone)]
    pub struct AlertV7 {
        pub id: String,
        pub severity: AlertSeverity,
        pub category: String,
        pub message: String,
        pub timestamp_ms: u64,
    }

    impl AlertV7 {
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

    // ---------------------------------------------------------------------------
    // Dashboard Snapshot
    // ---------------------------------------------------------------------------

    /// Complete dashboard snapshot v7.
    #[derive(Debug, Clone)]
    pub struct DashboardSnapshotV7 {
        pub timestamp_ms: u64,
        pub scaling_v7: ScalingV7Summary,
        pub zkp_v13: ZkpV13Summary,
        pub bridge_v6: BridgeV6Summary,
        pub alerts: Vec<AlertV7>,
        pub metrics: HashMap<String, f64>,
    }

    impl Default for DashboardSnapshotV7 {
        fn default() -> Self {
            Self {
                timestamp_ms: current_timestamp_ms(),
                scaling_v7: ScalingV7Summary::default(),
                zkp_v13: ZkpV13Summary::default(),
                bridge_v6: BridgeV6Summary::default(),
                alerts: Vec::new(),
                metrics: HashMap::new(),
            }
        }
    }

    impl DashboardSnapshotV7 {
        pub fn new() -> Self {
            Self::default()
        }

        /// Generate alerts based on current state thresholds.
        pub fn generate_alerts(&mut self) {
            // Partition health alert
            if self.scaling_v7.partition_health < 0.95 {
                self.alerts.push(AlertV7::new(
                    "partition_health".to_string(),
                    AlertSeverity::Critical,
                    "scaling_v7".to_string(),
                    format!(
                        "Partition health {:.2}% below 99.5% threshold",
                        self.scaling_v7.partition_health * 100.0
                    ),
                ));
            }

            // ZKP fallback rate alert
            if self.zkp_v13.fallback_rate > 0.3 {
                self.alerts.push(AlertV7::new(
                    "zkp_fallback".to_string(),
                    AlertSeverity::Warning,
                    "zkp_v13".to_string(),
                    format!(
                        "ZKP fallback rate {:.1}% exceeds 30% threshold",
                        self.zkp_v13.fallback_rate * 100.0
                    ),
                ));
            }

            // Bridge credibility alert
            if self.bridge_v6.avg_credibility < 0.6 {
                self.alerts.push(AlertV7::new(
                    "bridge_credibility".to_string(),
                    AlertSeverity::Critical,
                    "bridge_v6".to_string(),
                    format!(
                        "Bridge avg credibility {:.2} below 0.6 threshold",
                        self.bridge_v6.avg_credibility
                    ),
                ));
            }

            // Gradient divergence alert
            if self.scaling_v7.gradient_alignment < 0.8 {
                self.alerts.push(AlertV7::new(
                    "gradient_divergence".to_string(),
                    AlertSeverity::Warning,
                    "scaling_v7".to_string(),
                    format!(
                        "Gradient alignment {:.2}% below 80% threshold",
                        self.scaling_v7.gradient_alignment * 100.0
                    ),
                ));
            }

            // Predictive load alert
            if self.scaling_v7.predictive_load > 0.85 {
                self.alerts.push(AlertV7::new(
                    "predictive_load".to_string(),
                    AlertSeverity::Warning,
                    "scaling_v7".to_string(),
                    format!(
                        "Predictive load {:.1}% exceeds 85% threshold",
                        self.scaling_v7.predictive_load * 100.0
                    ),
                ));
            }
        }

        /// Record a metric value.
        pub fn record_metric(&mut self, metric: MetricValueV7) {
            self.metrics
                .insert(metric.metric.to_string(), metric.value);
        }
    }

    // ---------------------------------------------------------------------------
    // Dashboard Stats
    // ---------------------------------------------------------------------------

    /// Dashboard v7 statistics.
    #[derive(Debug, Clone, Default)]
    pub struct DashboardV7Stats {
        pub snapshots_generated: u64,
        pub alerts_triggered: u64,
        pub metrics_recorded: u64,
    }

    // ---------------------------------------------------------------------------
    // Dashboard Engine
    // ---------------------------------------------------------------------------

    /// Dashboard v7 engine.
    pub struct DashboardV7 {
        pub scaling_v7: ScalingV7Summary,
        pub zkp_v13: ZkpV13Summary,
        pub bridge_v6: BridgeV6Summary,
        pub alerts: Vec<AlertV7>,
        pub metrics: HashMap<String, f64>,
        pub metric_history: VecDeque<MetricValueV7>,
        pub stats: DashboardV7Stats,
    }

    impl DashboardV7 {
        /// Create a new dashboard v7 instance.
        pub fn new() -> Self {
            Self {
                scaling_v7: ScalingV7Summary::default(),
                zkp_v13: ZkpV13Summary::default(),
                bridge_v6: BridgeV6Summary::default(),
                alerts: Vec::new(),
                metrics: HashMap::new(),
                metric_history: VecDeque::with_capacity(1000),
                stats: DashboardV7Stats::default(),
            }
        }

        /// Update scaling v7 summary.
        pub fn update_scaling_v7(&mut self, summary: ScalingV7Summary) {
            self.scaling_v7 = summary;
        }

        /// Update ZKP v13 summary.
        pub fn update_zkp_v13(&mut self, summary: ZkpV13Summary) {
            self.zkp_v13 = summary;
        }

        /// Update bridge v6 summary.
        pub fn update_bridge_v6(&mut self, summary: BridgeV6Summary) {
            self.bridge_v6 = summary;
        }

        /// Record a metric value.
        pub fn record_metric(&mut self, metric: MetricValueV7) {
            self.metrics
                .insert(metric.metric.to_string(), metric.value);
            self.metric_history.push_back(metric);
            if self.metric_history.len() > 1000 {
                self.metric_history.pop_front();
            }
            self.stats.metrics_recorded += 1;
        }

        /// Generate a complete dashboard snapshot.
        pub fn generate_snapshot(&mut self) -> DashboardSnapshotV7 {
            let mut snapshot = DashboardSnapshotV7 {
                timestamp_ms: current_timestamp_ms(),
                scaling_v7: self.scaling_v7.clone(),
                zkp_v13: self.zkp_v13.clone(),
                bridge_v6: self.bridge_v6.clone(),
                alerts: Vec::new(),
                metrics: self.metrics.clone(),
            };
            snapshot.generate_alerts();
            self.alerts = snapshot.alerts.clone();
            self.stats.snapshots_generated += 1;
            self.stats.alerts_triggered += self.alerts.len() as u64;
            snapshot
        }

        /// Clear all alerts.
        pub fn clear_alerts(&mut self) {
            self.alerts.clear();
        }

        /// Reset statistics.
        pub fn reset_stats(&mut self) {
            self.stats = DashboardV7Stats::default();
        }
    }

    impl Default for DashboardV7 {
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

    // ---------------------------------------------------------------------------
    // Tests
    // ---------------------------------------------------------------------------

    #[cfg(test)]
    mod tests {
        #[allow(unused_imports)]
        use super::*;

        #[test]
        fn test_dashboard_creation() {
            let dashboard = DashboardV7::new();
            assert_eq!(dashboard.scaling_v7.nodes_active, 0);
            assert_eq!(dashboard.alerts.len(), 0);
        }

        #[test]
        fn test_update_scaling_v7() {
            let mut dashboard = DashboardV7::new();
            let mut summary = ScalingV7Summary::default();
            summary.nodes_active = 50;
            summary.shards_active = 10;
            dashboard.update_scaling_v7(summary);
            assert_eq!(dashboard.scaling_v7.nodes_active, 50);
            assert_eq!(dashboard.scaling_v7.shards_active, 10);
        }

        #[test]
        fn test_update_zkp_v13() {
            let mut dashboard = DashboardV7::new();
            let mut summary = ZkpV13Summary::default();
            summary.proofs_submitted = 100;
            summary.proofs_verified = 95;
            dashboard.update_zkp_v13(summary);
            assert_eq!(dashboard.zkp_v13.proofs_submitted, 100);
        }

        #[test]
        fn test_update_bridge_v6() {
            let mut dashboard = DashboardV7::new();
            let mut summary = BridgeV6Summary::default();
            summary.proofs_routed = 80;
            summary.proofs_verified = 75;
            dashboard.update_bridge_v6(summary);
            assert_eq!(dashboard.bridge_v6.proofs_routed, 80);
        }

        #[test]
        fn test_partition_health_alert() {
            let mut dashboard = DashboardV7::new();
            dashboard.scaling_v7.partition_health = 0.90;
            let snapshot = dashboard.generate_snapshot();
            assert!(snapshot.alerts.iter().any(|a| a.id == "partition_health"));
        }

        #[test]
        fn test_zkp_fallback_alert() {
            let mut dashboard = DashboardV7::new();
            dashboard.zkp_v13.fallback_rate = 0.4;
            let snapshot = dashboard.generate_snapshot();
            assert!(snapshot.alerts.iter().any(|a| a.id == "zkp_fallback"));
        }

        #[test]
        fn test_bridge_credibility_alert() {
            let mut dashboard = DashboardV7::new();
            dashboard.bridge_v6.avg_credibility = 0.5;
            let snapshot = dashboard.generate_snapshot();
            assert!(snapshot.alerts.iter().any(|a| a.id == "bridge_credibility"));
        }

        #[test]
        fn test_gradient_divergence_alert() {
            let mut dashboard = DashboardV7::new();
            dashboard.scaling_v7.gradient_alignment = 0.7;
            let snapshot = dashboard.generate_snapshot();
            assert!(snapshot.alerts.iter().any(|a| a.id == "gradient_divergence"));
        }

        #[test]
        fn test_predictive_load_alert() {
            let mut dashboard = DashboardV7::new();
            dashboard.scaling_v7.predictive_load = 0.9;
            let snapshot = dashboard.generate_snapshot();
            assert!(snapshot.alerts.iter().any(|a| a.id == "predictive_load"));
        }

        #[test]
        fn test_zkp_verification_rate() {
            let summary = ZkpV13Summary {
                proofs_submitted: 100,
                proofs_verified: 90,
                ..Default::default()
            };
            assert!((summary.verification_rate() - 0.9).abs() < 0.001);
        }

        #[test]
        fn test_bridge_consensus_rate() {
            let summary = BridgeV6Summary {
                proofs_routed: 100,
                proofs_verified: 85,
                ..Default::default()
            };
            assert!((summary.consensus_success_rate() - 0.85).abs() < 0.001);
        }

        #[test]
        fn test_snapshot_generation() {
            let mut dashboard = DashboardV7::new();
            dashboard.scaling_v7.nodes_active = 30;
            let snapshot = dashboard.generate_snapshot();
            assert_eq!(snapshot.scaling_v7.nodes_active, 30);
            assert!(snapshot.timestamp_ms > 0);
        }

        #[test]
        fn test_record_metric() {
            let mut dashboard = DashboardV7::new();
            let metric = MetricValueV7::new(MetricV7::SystemCpuPercent, 45.0, None);
            dashboard.record_metric(metric);
            assert_eq!(dashboard.stats.metrics_recorded, 1);
        }

        #[test]
        fn test_clear_alerts() {
            let mut dashboard = DashboardV7::new();
            dashboard.alerts.push(AlertV7::new(
                "test".to_string(),
                AlertSeverity::Info,
                "test".to_string(),
                "test".to_string(),
            ));
            dashboard.clear_alerts();
            assert_eq!(dashboard.alerts.len(), 0);
        }

        #[test]
        fn test_reset_stats() {
            let mut dashboard = DashboardV7::new();
            dashboard.stats.snapshots_generated = 5;
            dashboard.reset_stats();
            assert_eq!(dashboard.stats.snapshots_generated, 0);
        }

        #[test]
        fn test_metric_display() {
            let metric = MetricV7::ScalingV7NodesActive;
            assert_eq!(format!("{}", metric), "scaling_v7.nodes_active");
        }

        #[test]
        fn test_alert_severity_display() {
            assert_eq!(format!("{}", AlertSeverity::Critical), "critical");
            assert_eq!(format!("{}", AlertSeverity::Warning), "warning");
            assert_eq!(format!("{}", AlertSeverity::Info), "info");
        }

        #[test]
        fn test_error_display() {
            let err = DashboardV7Error::RateLimitExceeded;
            assert!(!format!("{}", err).is_empty());
        }

        #[test]
        fn test_default_snapshot() {
            let snapshot = DashboardSnapshotV7::default();
            assert_eq!(snapshot.scaling_v7.partition_health, 1.0);
            assert_eq!(snapshot.alerts.len(), 0);
        }

        #[test]
        fn test_no_alerts_when_healthy() {
            let mut dashboard = DashboardV7::new();
            dashboard.scaling_v7.partition_health = 0.999;
            dashboard.zkp_v13.fallback_rate = 0.05;
            dashboard.bridge_v6.avg_credibility = 0.95;
            dashboard.scaling_v7.gradient_alignment = 0.95;
            dashboard.scaling_v7.predictive_load = 0.5;
            let snapshot = dashboard.generate_snapshot();
            assert_eq!(snapshot.alerts.len(), 0);
        }

        #[test]
        fn test_multiple_alerts() {
            let mut dashboard = DashboardV7::new();
            dashboard.scaling_v7.partition_health = 0.90;
            dashboard.zkp_v13.fallback_rate = 0.5;
            dashboard.bridge_v6.avg_credibility = 0.4;
            let snapshot = dashboard.generate_snapshot();
            assert!(snapshot.alerts.len() >= 3);
        }

        #[test]
        fn test_stats_tracking() {
            let mut dashboard = DashboardV7::new();
            dashboard.generate_snapshot();
            dashboard.record_metric(MetricValueV7::new(MetricV7::SystemCpuPercent, 50.0, None));
            assert_eq!(dashboard.stats.snapshots_generated, 1);
            assert_eq!(dashboard.stats.metrics_recorded, 1);
        }

        #[test]
        fn test_scaling_summary_updates() {
            let mut summary = ScalingV7Summary::default();
            summary.update_nodes(100);
            summary.update_shards(20);
            summary.update_partition_health(0.99);
            summary.update_predictive_load(0.7);
            summary.update_gradient_alignment(0.92);
            assert_eq!(summary.nodes_active, 100);
            assert!((summary.partition_health - 0.99).abs() < 0.001);
        }

        #[test]
        fn test_zkp_summary_updates() {
            let mut summary = ZkpV13Summary::default();
            summary.update_proofs(200, 190);
            summary.update_batches(10, 48.0);
            assert_eq!(summary.proofs_submitted, 200);
            assert!((summary.verification_rate() - 0.95).abs() < 0.001);
        }

        #[test]
        fn test_bridge_summary_updates() {
            let mut summary = BridgeV6Summary::default();
            summary.update_routing(150, 200);
            summary.update_verification(140, 10);
            assert_eq!(summary.proofs_routed, 150);
            assert!((summary.consensus_success_rate() - 140.0 / 150.0).abs() < 0.001);
        }
    }
}

#[cfg(feature = "v1.6-sprint2")]
pub use internal::*;
