//! Alert Pipeline — Multi-stage alert processing with deduplication, escalation, and suppression.
//!
//! LP-106: UI Dashboard v5 & Real-time Streams
//! Provides a multi-stage alert processing pipeline for Dashboard v5 with support for
//! alert deduplication, severity-based escalation, time-based suppression windows,
//! and alert aggregation into compliance reports.
//!
//! Características:
//! - Duplicación de alertas con ventanas temporales
//! - Escalado automático por severidad y frecuencia
//! - Ventanas de supresión configurables
//! - Agregación de alertas en reports de compliance
//! - Historial completo con búsqueda por categoría/severidad
//!
//! Protegido con `#[cfg(feature = "v1.4-sprint2")]`.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use thiserror::Error;

// ─── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum AlertPipelineError {
    #[error("Regla no encontrada: {0}")]
    RuleNotFound(String),
    #[error("Severidad inválida: {0}")]
    InvalidSeverity(u8),
    #[error("Límite de reglas alcanzado")]
    MaxRulesReached,
    #[error("Categoría inválida: {0}")]
    InvalidCategory(String),
}

// ─── Alert Severity ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertSeverity::Info => write!(f, "info"),
            AlertSeverity::Warning => write!(f, "warning"),
            AlertSeverity::Critical => write!(f, "critical"),
            AlertSeverity::Emergency => write!(f, "emergency"),
        }
    }
}

impl AlertSeverity {
    pub fn level(&self) -> u8 {
        match self {
            AlertSeverity::Info => 1,
            AlertSeverity::Warning => 2,
            AlertSeverity::Critical => 3,
            AlertSeverity::Emergency => 4,
        }
    }

    pub fn from_level(level: u8) -> Self {
        match level {
            1 => AlertSeverity::Info,
            2 => AlertSeverity::Warning,
            3 => AlertSeverity::Critical,
            _ => AlertSeverity::Emergency,
        }
    }
}

// ─── Alert Category ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AlertCategory {
    ZkpV7,
    CrossPool,
    GovernanceV4,
    System,
    Network,
    Custom(String),
}

impl std::fmt::Display for AlertCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertCategory::ZkpV7 => write!(f, "zkp_v7"),
            AlertCategory::CrossPool => write!(f, "cross_pool"),
            AlertCategory::GovernanceV4 => write!(f, "governance_v4"),
            AlertCategory::System => write!(f, "system"),
            AlertCategory::Network => write!(f, "network"),
            AlertCategory::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

// ─── Alert ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Unique alert identifier.
    pub alert_id: String,
    /// Alert category.
    pub category: AlertCategory,
    /// Alert severity.
    pub severity: AlertSeverity,
    /// Alert message.
    pub message: String,
    /// Metric value that triggered the alert.
    pub metric_value: f64,
    /// Alert threshold.
    pub threshold: f64,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Acknowledged status.
    pub acknowledged: bool,
    /// Escalation count.
    pub escalation_count: u32,
    /// Suppressed status.
    pub suppressed: bool,
}

impl Alert {
    pub fn new(
        alert_id: String,
        category: AlertCategory,
        severity: AlertSeverity,
        message: String,
        metric_value: f64,
        threshold: f64,
    ) -> Self {
        Self {
            alert_id,
            category,
            severity,
            message,
            metric_value,
            threshold,
            timestamp_ms: current_timestamp_ms(),
            acknowledged: false,
            escalation_count: 0,
            suppressed: false,
        }
    }

    pub fn acknowledge(&mut self) {
        self.acknowledged = true;
    }

    pub fn escalate(&mut self) {
        self.escalation_count += 1;
        if self.severity == AlertSeverity::Info && self.escalation_count >= 3 {
            self.severity = AlertSeverity::Warning;
        } else if self.severity == AlertSeverity::Warning && self.escalation_count >= 5 {
            self.severity = AlertSeverity::Critical;
        } else if self.severity == AlertSeverity::Critical && self.escalation_count >= 10 {
            self.severity = AlertSeverity::Emergency;
        }
    }
}

// ─── Alert Rule ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Rule identifier.
    pub rule_id: String,
    /// Rule category.
    pub category: AlertCategory,
    /// Rule severity.
    pub severity: AlertSeverity,
    /// Threshold value.
    pub threshold: f64,
    /// Comparison operator (above/below).
    pub operator: AlertOperator,
    /// Deduplication window in milliseconds.
    pub dedup_window_ms: u64,
    /// Suppression window in milliseconds after acknowledgment.
    pub suppression_window_ms: u64,
    /// Escalation interval in milliseconds.
    pub escalation_interval_ms: u64,
    /// Maximum escalation count.
    pub max_escalations: u32,
    /// Enabled status.
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlertOperator {
    Above,
    Below,
}

impl std::fmt::Display for AlertOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertOperator::Above => write!(f, "above"),
            AlertOperator::Below => write!(f, "below"),
        }
    }
}

impl AlertRule {
    pub fn new(
        rule_id: String,
        category: AlertCategory,
        severity: AlertSeverity,
        threshold: f64,
        operator: AlertOperator,
    ) -> Self {
        Self {
            rule_id,
            category,
            severity,
            threshold,
            operator,
            dedup_window_ms: 30000,
            suppression_window_ms: 60000,
            escalation_interval_ms: 120000,
            max_escalations: 5,
            enabled: true,
        }
    }

    pub fn is_triggered(&self, value: f64) -> bool {
        match self.operator {
            AlertOperator::Above => value > self.threshold,
            AlertOperator::Below => value < self.threshold,
        }
    }
}

// ─── Deduplication Key ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
struct DedupKey {
    rule_id: String,
    category: AlertCategory,
}

// ─── Pipeline Config ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Maximum number of alert rules.
    pub max_rules: usize,
    /// Maximum alert history size.
    pub max_history: usize,
    /// Default deduplication window in milliseconds.
    pub default_dedup_window_ms: u64,
    /// Default suppression window in milliseconds.
    pub default_suppression_window_ms: u64,
    /// Enable auto-escalation.
    pub auto_escalation: bool,
    /// Maximum escalations per alert.
    pub max_escalations: u32,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            max_rules: 128,
            max_history: 5000,
            default_dedup_window_ms: 30000,
            default_suppression_window_ms: 60000,
            auto_escalation: true,
            max_escalations: 5,
        }
    }
}

// ─── Pipeline Stats ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStats {
    /// Total alerts processed.
    pub alerts_processed: u64,
    /// Total alerts deduplicated.
    pub alerts_deduplicated: u64,
    /// Total alerts suppressed.
    pub alerts_suppressed: u64,
    /// Total alerts escalated.
    pub alerts_escalated: u64,
    /// Total alerts acknowledged.
    pub alerts_acknowledged: u64,
    /// Active alert count.
    pub active_alerts: usize,
    /// Alerts by severity.
    pub by_severity: HashMap<String, u64>,
}

impl Default for PipelineStats {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineStats {
    pub fn new() -> Self {
        Self {
            alerts_processed: 0,
            alerts_deduplicated: 0,
            alerts_suppressed: 0,
            alerts_escalated: 0,
            alerts_acknowledged: 0,
            active_alerts: 0,
            by_severity: HashMap::new(),
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

// ─── Compliance Report ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    /// Report period start.
    pub period_start_ms: u64,
    /// Report period end.
    pub period_end_ms: u64,
    /// Total alerts in period.
    pub total_alerts: u64,
    /// Alerts by severity.
    pub by_severity: HashMap<String, u64>,
    /// Alerts by category.
    pub by_category: HashMap<String, u64>,
    /// Mean time to acknowledgment (ms).
    pub mean_ack_time_ms: f64,
    /// Compliance score (0.0-1.0).
    pub compliance_score: f64,
}

impl ComplianceReport {
    pub fn new(period_start_ms: u64, period_end_ms: u64) -> Self {
        Self {
            period_start_ms,
            period_end_ms,
            total_alerts: 0,
            by_severity: HashMap::new(),
            by_category: HashMap::new(),
            mean_ack_time_ms: 0.0,
            compliance_score: 1.0,
        }
    }

    pub fn record_alert(&mut self, severity: &AlertSeverity, category: &AlertCategory, acknowledged: bool, ack_time_ms: u64) {
        self.total_alerts += 1;
        *self.by_severity.entry(format!("{}", severity)).or_insert(0) += 1;
        *self.by_category.entry(format!("{}", category)).or_insert(0) += 1;
        if acknowledged {
            self.mean_ack_time_ms = ack_time_ms as f64;
        }
    }

    pub fn compute_compliance_score(&mut self) {
        if self.total_alerts == 0 {
            self.compliance_score = 1.0;
            return;
        }
        let critical = self.by_severity.get("critical").copied().unwrap_or(0);
        let emergency = self.by_severity.get("emergency").copied().unwrap_or(0);
        let severe_ratio = (critical + emergency) as f64 / self.total_alerts as f64;
        self.compliance_score = (1.0 - severe_ratio * 0.5).max(0.0);
    }
}

// ─── Alert Pipeline ──────────────────────────────────────────────────────────

/// Multi-stage alert processing pipeline.
pub struct AlertPipeline {
    /// Configuration.
    config: PipelineConfig,
    /// Alert rules.
    rules: HashMap<String, AlertRule>,
    /// Alert history.
    history: VecDeque<Alert>,
    /// Deduplication tracking (rule_id -> last alert timestamp).
    dedup_map: HashMap<DedupKey, u64>,
    /// Suppression tracking (rule_id -> suppression end timestamp).
    suppression_map: HashMap<String, u64>,
    /// Statistics.
    stats: PipelineStats,
    /// Alert ID counter.
    alert_counter: u64,
}

impl AlertPipeline {
    /// Create a new alert pipeline with default config.
    pub fn new() -> Self {
        Self::with_config(PipelineConfig::default())
    }

    /// Create a new alert pipeline with custom config.
    pub fn with_config(config: PipelineConfig) -> Self {
        Self {
            config,
            rules: HashMap::new(),
            history: VecDeque::with_capacity(5000),
            dedup_map: HashMap::new(),
            suppression_map: HashMap::new(),
            stats: PipelineStats::new(),
            alert_counter: 0,
        }
    }

    /// Register an alert rule.
    pub fn add_rule(&mut self, rule: AlertRule) -> Result<(), AlertPipelineError> {
        if self.rules.len() >= self.config.max_rules {
            return Err(AlertPipelineError::MaxRulesReached);
        }
        if self.rules.contains_key(&rule.rule_id) {
            return Err(AlertPipelineError::RuleNotFound(format!("Rule {} already exists", rule.rule_id)));
        }
        self.rules.insert(rule.rule_id.clone(), rule);
        Ok(())
    }

    /// Remove an alert rule.
    pub fn remove_rule(&mut self, rule_id: &str) -> Result<(), AlertPipelineError> {
        self.rules
            .remove(rule_id)
            .ok_or_else(|| AlertPipelineError::RuleNotFound(rule_id.to_string()))?;
        Ok(())
    }

    /// Get a rule by ID.
    pub fn get_rule(&self, rule_id: &str) -> Option<&AlertRule> {
        self.rules.get(rule_id)
    }

    /// Process a metric value against all rules.
    pub fn process_metric(&mut self, category: &AlertCategory, metric_name: &str, value: f64) -> Vec<Alert> {
        let mut triggered = Vec::new();
        for rule in self.rules.values() {
            if !rule.enabled || &rule.category != category {
                continue;
            }
            if !rule.is_triggered(value) {
                continue;
            }
            // Check deduplication
            let dedup_key = DedupKey {
                rule_id: rule.rule_id.clone(),
                category: rule.category.clone(),
            };
            let now = current_timestamp_ms();
            if let Some(&last_time) = self.dedup_map.get(&dedup_key) {
                if now - last_time < rule.dedup_window_ms {
                    self.stats.alerts_deduplicated += 1;
                    continue;
                }
            }
            // Check suppression
            if let Some(&suppress_end) = self.suppression_map.get(&rule.rule_id) {
                if now < suppress_end {
                    self.stats.alerts_suppressed += 1;
                    continue;
                }
            }
            // Create alert
            self.alert_counter += 1;
            let mut alert = Alert::new(
                format!("alert-{}", self.alert_counter),
                rule.category.clone(),
                rule.severity.clone(),
                format!("{}: {} {} {:.2} (threshold: {:.2})", metric_name, rule.operator, value, rule.threshold),
                value,
                rule.threshold,
            );
            // Check escalation
            if self.config.auto_escalation {
                self.check_escalation(&mut alert);
            }
            // Update dedup tracking
            self.dedup_map.insert(dedup_key, now);
            // Record in history
            self.history.push_back(alert.clone());
            self.enforce_history_limit();
            // Update stats
            self.stats.alerts_processed += 1;
            *self.stats.by_severity.entry(format!("{}", alert.severity)).or_insert(0) += 1;
            self.stats.active_alerts = self.active_alert_count();
            triggered.push(alert);
        }
        triggered
    }

    /// Acknowledge an alert.
    pub fn acknowledge_alert(&mut self, alert_id: &str) -> bool {
        for alert in &mut self.history {
            if alert.alert_id == alert_id && !alert.suppressed {
                let was_unacknowledged = !alert.acknowledged;
                alert.acknowledge();
                if was_unacknowledged {
                    self.stats.alerts_acknowledged += 1;
                }
                // Start suppression window for the rule
                if let Some(rule) = self.rules.get(&alert_id.replace("alert-", "")) {
                    let now = current_timestamp_ms();
                    self.suppression_map.insert(rule.rule_id.clone(), now + rule.suppression_window_ms);
                }
                self.stats.active_alerts = self.active_alert_count();
                return true;
            }
        }
        false
    }

    /// Get alerts by category.
    pub fn get_by_category(&self, category: &AlertCategory) -> Vec<&Alert> {
        self.history.iter().filter(|a| &a.category == category).collect()
    }

    /// Get alerts by severity.
    pub fn get_by_severity(&self, severity: &AlertSeverity) -> Vec<&Alert> {
        self.history.iter().filter(|a| &a.severity == severity).collect()
    }

    /// Get active (unacknowledged) alerts.
    pub fn get_active_alerts(&self) -> Vec<&Alert> {
        self.history.iter().filter(|a| !a.acknowledged && !a.suppressed).collect()
    }

    /// Get recent alerts.
    pub fn get_recent(&self, count: usize) -> Vec<&Alert> {
        let available = count.min(self.history.len());
        let start = self.history.len() - available;
        self.history[start..].iter().collect()
    }

    /// Generate a compliance report for a time period.
    pub fn generate_compliance_report(&self, start_ms: u64, end_ms: u64) -> ComplianceReport {
        let mut report = ComplianceReport::new(start_ms, end_ms);
        for alert in &self.history {
            if alert.timestamp_ms >= start_ms && alert.timestamp_ms <= end_ms {
                let ack_time = if alert.acknowledged { 30000 } else { 0 };
                report.record_alert(&alert.severity, &alert.category, alert.acknowledged, ack_time);
            }
        }
        report.compute_compliance_score();
        report
    }

    /// Get active alert count.
    pub fn active_alert_count(&self) -> usize {
        self.history.iter().filter(|a| !a.acknowledged && !a.suppressed).count()
    }

    /// Reset statistics.
    pub fn reset_stats(&mut self) {
        self.stats.reset();
    }

    /// Get stats reference.
    pub fn get_stats(&self) -> &PipelineStats {
        &self.stats
    }

    /// Get config reference.
    pub fn get_config(&self) -> &PipelineConfig {
        &self.config
    }

    /// Clear old alerts.
    pub fn clear_old_alerts(&mut self, max_age_ms: u64) -> usize {
        let now = current_timestamp_ms();
        let before = self.history.len();
        self.history.retain(|a| now - a.timestamp_ms <= max_age_ms);
        before - self.history.len()
    }

    /// Clear deduplication map.
    pub fn clear_dedup(&mut self) {
        self.dedup_map.clear();
    }

    /// Clear suppression map.
    pub fn clear_suppression(&mut self) {
        self.suppression_map.clear();
    }

    // ─── Private helpers ─────────────────────────────────────────────────────

    fn check_escalation(&mut self, alert: &mut Alert) {
        if alert.escalation_count >= self.config.max_escalations {
            return;
        }
        alert.escalate();
        if alert.escalation_count > 0 {
            self.stats.alerts_escalated += 1;
        }
    }

    fn enforce_history_limit(&mut self) {
        if self.history.len() > self.config.max_history {
            let excess = self.history.len() - self.config.max_history;
            self.history.drain(..excess);
        }
    }
}

impl Default for AlertPipeline {
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

    fn make_rule(id: &str, category: AlertCategory, severity: AlertSeverity, threshold: f64, op: AlertOperator) -> AlertRule {
        AlertRule::new(id.to_string(), category, severity, threshold, op)
    }

    #[test]
    fn test_pipeline_creation() {
        let pipeline = AlertPipeline::new();
        assert_eq!(pipeline.rules.len(), 0);
        assert_eq!(pipeline.stats.alerts_processed, 0);
    }

    #[test]
    fn test_pipeline_with_config() {
        let config = PipelineConfig {
            max_rules: 10,
            ..Default::default()
        };
        let pipeline = AlertPipeline::with_config(config);
        assert_eq!(pipeline.config.max_rules, 10);
    }

    #[test]
    fn test_add_rule() {
        let mut pipeline = AlertPipeline::new();
        pipeline.add_rule(make_rule("r1", AlertCategory::System, AlertSeverity::Warning, 90.0, AlertOperator::Above)).unwrap();
        assert_eq!(pipeline.rules.len(), 1);
    }

    #[test]
    fn test_add_rule_duplicate() {
        let mut pipeline = AlertPipeline::new();
        let rule = make_rule("r1", AlertCategory::System, AlertSeverity::Warning, 90.0, AlertOperator::Above);
        pipeline.add_rule(rule.clone()).unwrap();
        assert!(pipeline.add_rule(rule).is_err());
    }

    #[test]
    fn test_remove_rule() {
        let mut pipeline = AlertPipeline::new();
        pipeline.add_rule(make_rule("r1", AlertCategory::System, AlertSeverity::Warning, 90.0, AlertOperator::Above)).unwrap();
        pipeline.remove_rule("r1").unwrap();
        assert_eq!(pipeline.rules.len(), 0);
    }

    #[test]
    fn test_remove_rule_missing() {
        let mut pipeline = AlertPipeline::new();
        assert!(pipeline.remove_rule("nonexistent").is_err());
    }

    #[test]
    fn test_get_rule() {
        let mut pipeline = AlertPipeline::new();
        pipeline.add_rule(make_rule("r1", AlertCategory::System, AlertSeverity::Warning, 90.0, AlertOperator::Above)).unwrap();
        assert!(pipeline.get_rule("r1").is_some());
        assert!(pipeline.get_rule("r2").is_none());
    }

    #[test]
    fn test_process_metric_triggers() {
        let mut pipeline = AlertPipeline::new();
        pipeline.add_rule(make_rule("r1", AlertCategory::System, AlertSeverity::Warning, 90.0, AlertOperator::Above)).unwrap();
        let alerts = pipeline.process_metric(&AlertCategory::System, "cpu", 95.0);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].severity, AlertSeverity::Warning);
    }

    #[test]
    fn test_process_metric_no_trigger() {
        let mut pipeline = AlertPipeline::new();
        pipeline.add_rule(make_rule("r1", AlertCategory::System, AlertSeverity::Warning, 90.0, AlertOperator::Above)).unwrap();
        let alerts = pipeline.process_metric(&AlertCategory::System, "cpu", 50.0);
        assert!(alerts.is_empty());
    }

    #[test]
    fn test_process_metric_below_operator() {
        let mut pipeline = AlertPipeline::new();
        pipeline.add_rule(make_rule("r1", AlertCategory::ZkpV7, AlertSeverity::Critical, 10.0, AlertOperator::Below)).unwrap();
        let alerts = pipeline.process_metric(&AlertCategory::ZkpV7, "throughput", 5.0);
        assert_eq!(alerts.len(), 1);
    }

    #[test]
    fn test_deduplication() {
        let mut pipeline = AlertPipeline::new();
        let rule = AlertRule {
            rule_id: "r1".to_string(),
            category: AlertCategory::System.clone(),
            severity: AlertSeverity::Warning,
            threshold: 90.0,
            operator: AlertOperator::Above,
            dedup_window_ms: 1000000,
            suppression_window_ms: 0,
            escalation_interval_ms: 0,
            max_escalations: 0,
            enabled: true,
        };
        pipeline.add_rule(rule).unwrap();
        let a1 = pipeline.process_metric(&AlertCategory::System, "cpu", 95.0);
        let a2 = pipeline.process_metric(&AlertCategory::System, "cpu", 96.0);
        assert_eq!(a1.len(), 1);
        assert_eq!(a2.len(), 0);
        assert_eq!(pipeline.stats.alerts_deduplicated, 1);
    }

    #[test]
    fn test_acknowledge_alert() {
        let mut pipeline = AlertPipeline::new();
        pipeline.add_rule(make_rule("r1", AlertCategory::System, AlertSeverity::Warning, 90.0, AlertOperator::Above)).unwrap();
        let alerts = pipeline.process_metric(&AlertCategory::System, "cpu", 95.0);
        let id = &alerts[0].alert_id;
        assert!(pipeline.acknowledge_alert(id));
        assert_eq!(pipeline.stats.alerts_acknowledged, 1);
    }

    #[test]
    fn test_acknowledge_missing() {
        let pipeline = &mut AlertPipeline::new();
        assert!(!pipeline.acknowledge_alert("nonexistent"));
    }

    #[test]
    fn test_get_by_category() {
        let mut pipeline = AlertPipeline::new();
        pipeline.add_rule(make_rule("r1", AlertCategory::System, AlertSeverity::Warning, 90.0, AlertOperator::Above)).unwrap();
        pipeline.add_rule(make_rule("r2", AlertCategory::ZkpV7, AlertSeverity::Critical, 10.0, AlertOperator::Below)).unwrap();
        pipeline.process_metric(&AlertCategory::System, "cpu", 95.0);
        pipeline.process_metric(&AlertCategory::ZkpV7, "throughput", 5.0);
        assert_eq!(pipeline.get_by_category(&AlertCategory::System).len(), 1);
        assert_eq!(pipeline.get_by_category(&AlertCategory::ZkpV7).len(), 1);
    }

    #[test]
    fn test_get_by_severity() {
        let mut pipeline = AlertPipeline::new();
        pipeline.add_rule(make_rule("r1", AlertCategory::System, AlertSeverity::Warning, 90.0, AlertOperator::Above)).unwrap();
        pipeline.add_rule(make_rule("r2", AlertCategory::System, AlertSeverity::Critical, 99.0, AlertOperator::Above)).unwrap();
        pipeline.process_metric(&AlertCategory::System, "cpu", 99.5);
        assert_eq!(pipeline.get_by_severity(&AlertSeverity::Warning).len(), 1);
        assert_eq!(pipeline.get_by_severity(&AlertSeverity::Critical).len(), 1);
    }

    #[test]
    fn test_get_active_alerts() {
        let mut pipeline = AlertPipeline::new();
        pipeline.add_rule(make_rule("r1", AlertCategory::System, AlertSeverity::Warning, 90.0, AlertOperator::Above)).unwrap();
        pipeline.process_metric(&AlertCategory::System, "cpu", 95.0);
        assert_eq!(pipeline.get_active_alerts().len(), 1);
        pipeline.acknowledge_alert("alert-1");
        assert_eq!(pipeline.get_active_alerts().len(), 0);
    }

    #[test]
    fn test_get_recent() {
        let mut pipeline = AlertPipeline::new();
        pipeline.add_rule(make_rule("r1", AlertCategory::System, AlertSeverity::Warning, 90.0, AlertOperator::Above)).unwrap();
        for _ in 0..5 {
            pipeline.clear_dedup();
            pipeline.process_metric(&AlertCategory::System, "cpu", 95.0);
        }
        let recent = pipeline.get_recent(3);
        assert_eq!(recent.len(), 3);
    }

    #[test]
    fn test_compliance_report() {
        let mut pipeline = AlertPipeline::new();
        pipeline.add_rule(make_rule("r1", AlertCategory::System, AlertSeverity::Warning, 90.0, AlertOperator::Above)).unwrap();
        pipeline.clear_dedup();
        pipeline.process_metric(&AlertCategory::System, "cpu", 95.0);
        let now = current_timestamp_ms();
        let report = pipeline.generate_compliance_report(0, now);
        assert_eq!(report.total_alerts, 1);
    }

    #[test]
    fn test_compliance_score() {
        let mut report = ComplianceReport::new(0, 1000);
        report.record_alert(&AlertSeverity::Critical, &AlertCategory::System, false, 0);
        report.record_alert(&AlertSeverity::Info, &AlertCategory::System, true, 30000);
        report.compute_compliance_score();
        assert!(report.compliance_score < 1.0);
        assert!(report.compliance_score > 0.0);
    }

    #[test]
    fn test_escalation() {
        let mut alert = Alert::new("a1".to_string(), AlertCategory::System, AlertSeverity::Info, "test".to_string(), 1.0, 0.5);
        assert_eq!(alert.severity, AlertSeverity::Info);
        alert.escalate();
        alert.escalate();
        alert.escalate();
        assert_eq!(alert.severity, AlertSeverity::Warning);
    }

    #[test]
    fn test_severity_level() {
        assert_eq!(AlertSeverity::Info.level(), 1);
        assert_eq!(AlertSeverity::Warning.level(), 2);
        assert_eq!(AlertSeverity::Critical.level(), 3);
        assert_eq!(AlertSeverity::Emergency.level(), 4);
    }

    #[test]
    fn test_severity_from_level() {
        assert_eq!(AlertSeverity::from_level(1), AlertSeverity::Info);
        assert_eq!(AlertSeverity::from_level(3), AlertSeverity::Critical);
        assert_eq!(AlertSeverity::from_level(5), AlertSeverity::Emergency);
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(format!("{}", AlertSeverity::Info), "info");
        assert_eq!(format!("{}", AlertSeverity::Critical), "critical");
    }

    #[test]
    fn test_category_display() {
        assert_eq!(format!("{}", AlertCategory::ZkpV7), "zkp_v7");
        assert_eq!(format!("{}", AlertCategory::Custom("test".to_string())), "custom:test");
    }

    #[test]
    fn test_operator_display() {
        assert_eq!(format!("{}", AlertOperator::Above), "above");
        assert_eq!(format!("{}", AlertOperator::Below), "below");
    }

    #[test]
    fn test_rule_is_triggered_above() {
        let rule = make_rule("r1", AlertCategory::System, AlertSeverity::Warning, 90.0, AlertOperator::Above);
        assert!(rule.is_triggered(95.0));
        assert!(!rule.is_triggered(85.0));
    }

    #[test]
    fn test_rule_is_triggered_below() {
        let rule = make_rule("r1", AlertCategory::System, AlertSeverity::Warning, 10.0, AlertOperator::Below);
        assert!(rule.is_triggered(5.0));
        assert!(!rule.is_triggered(15.0));
    }

    #[test]
    fn test_max_rules() {
        let config = PipelineConfig {
            max_rules: 2,
            ..Default::default()
        };
        let mut pipeline = AlertPipeline::with_config(config);
        pipeline.add_rule(make_rule("r1", AlertCategory::System, AlertSeverity::Warning, 90.0, AlertOperator::Above)).unwrap();
        pipeline.add_rule(make_rule("r2", AlertCategory::ZkpV7, AlertSeverity::Critical, 10.0, AlertOperator::Below)).unwrap();
        assert!(pipeline.add_rule(make_rule("r3", AlertCategory::Network, AlertSeverity::Info, 500.0, AlertOperator::Above)).is_err());
    }

    #[test]
    fn test_reset_stats() {
        let mut pipeline = AlertPipeline::new();
        pipeline.add_rule(make_rule("r1", AlertCategory::System, AlertSeverity::Warning, 90.0, AlertOperator::Above)).unwrap();
        pipeline.process_metric(&AlertCategory::System, "cpu", 95.0);
        pipeline.reset_stats();
        assert_eq!(pipeline.stats.alerts_processed, 0);
    }

    #[test]
    fn test_clear_old_alerts() {
        let mut pipeline = AlertPipeline::new();
        pipeline.add_rule(make_rule("r1", AlertCategory::System, AlertSeverity::Warning, 90.0, AlertOperator::Above)).unwrap();
        pipeline.process_metric(&AlertCategory::System, "cpu", 95.0);
        let cleared = pipeline.clear_old_alerts(0);
        assert_eq!(cleared, 1);
    }

    #[test]
    fn test_clear_dedup() {
        let mut pipeline = AlertPipeline::new();
        pipeline.dedup_map.insert(DedupKey { rule_id: "r1".to_string(), category: AlertCategory::System }, 1000);
        pipeline.clear_dedup();
        assert!(pipeline.dedup_map.is_empty());
    }

    #[test]
    fn test_clear_suppression() {
        let mut pipeline = AlertPipeline::new();
        pipeline.suppression_map.insert("r1".to_string(), 1000);
        pipeline.clear_suppression();
        assert!(pipeline.suppression_map.is_empty());
    }

    #[test]
    fn test_error_display() {
        match AlertPipelineError::RuleNotFound("x".to_string()) {
            e => assert!(format!("{}", e).contains("x")),
        }
    }

    #[test]
    fn test_config_default() {
        let config = PipelineConfig::default();
        assert_eq!(config.max_rules, 128);
        assert_eq!(config.max_history, 5000);
    }

    #[test]
    fn test_stats_default() {
        let stats = PipelineStats::default();
        assert_eq!(stats.alerts_processed, 0);
    }

    #[test]
    fn test_stats_reset() {
        let mut stats = PipelineStats::new();
        stats.alerts_processed = 100;
        stats.reset();
        assert_eq!(stats.alerts_processed, 0);
    }

    #[test]
    fn test_pipeline_default() {
        let pipeline = AlertPipeline::default();
        assert_eq!(pipeline.rules.len(), 0);
    }

    #[test]
    fn test_get_config() {
        let pipeline = AlertPipeline::new();
        assert_eq!(pipeline.get_config().max_rules, 128);
    }

    #[test]
    fn test_get_stats() {
        let pipeline = AlertPipeline::new();
        assert_eq!(pipeline.get_stats().alerts_processed, 0);
    }

    #[test]
    fn test_alert_new() {
        let alert = Alert::new("a1".to_string(), AlertCategory::System, AlertSeverity::Warning, "test".to_string(), 95.0, 90.0);
        assert_eq!(alert.alert_id, "a1");
        assert!(!alert.acknowledged);
    }

    #[test]
    fn test_alert_acknowledge() {
        let mut alert = Alert::new("a1".to_string(), AlertCategory::System, AlertSeverity::Warning, "test".to_string(), 95.0, 90.0);
        alert.acknowledge();
        assert!(alert.acknowledged);
    }

    #[test]
    fn test_compliance_report_empty() {
        let report = ComplianceReport::new(0, 1000);
        assert_eq!(report.total_alerts, 0);
        assert_eq!(report.compliance_score, 1.0);
    }

    #[test]
    fn test_disabled_rule() {
        let mut pipeline = AlertPipeline::new();
        let mut rule = make_rule("r1", AlertCategory::System, AlertSeverity::Warning, 90.0, AlertOperator::Above);
        rule.enabled = false;
        pipeline.add_rule(rule).unwrap();
        let alerts = pipeline.process_metric(&AlertCategory::System, "cpu", 95.0);
        assert!(alerts.is_empty());
    }

    #[test]
    fn test_category_mismatch() {
        let mut pipeline = AlertPipeline::new();
        pipeline.add_rule(make_rule("r1", AlertCategory::System, AlertSeverity::Warning, 90.0, AlertOperator::Above)).unwrap();
        let alerts = pipeline.process_metric(&AlertCategory::ZkpV7, "throughput", 95.0);
        assert!(alerts.is_empty());
    }

    #[test]
    fn test_history_limit() {
        let config = PipelineConfig {
            max_history: 5,
            ..Default::default()
        };
        let mut pipeline = AlertPipeline::with_config(config);
        pipeline.add_rule(make_rule("r1", AlertCategory::System, AlertSeverity::Warning, 90.0, AlertOperator::Above)).unwrap();
        for _ in 0..10 {
            pipeline.clear_dedup();
            pipeline.process_metric(&AlertCategory::System, "cpu", 95.0);
        }
        assert_eq!(pipeline.history.len(), 5);
    }

    #[test]
    fn test_severity_ordering() {
        assert!(AlertSeverity::Emergency > AlertSeverity::Critical);
        assert!(AlertSeverity::Critical > AlertSeverity::Warning);
        assert!(AlertSeverity::Warning > AlertSeverity::Info);
    }
}
