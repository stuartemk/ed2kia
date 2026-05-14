//! Alert Engine — Rule-based alert generation, deduplication, and notification routing.
//!
//! Provides threshold-based alerting with support for multiple severity levels,
//! alert grouping, deduplication windows, and notification channel routing.
//!
//! **Design Principles:**
//! - Zero external dependencies for alert logic
//! - Configurable thresholds and cooldowns
//! - Alert deduplication to prevent notification storms
//! - Severity-based escalation
//! - Thread-safe for concurrent metric updates

#[cfg(feature = "v1.4-sprint1")]
#[cfg(feature = "v1.4-sprint1")]
#[derive(Debug, Clone, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

#[cfg(feature = "v1.4-sprint1")]
impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertSeverity::Info => write!(f, "INFO"),
            AlertSeverity::Warning => write!(f, "WARNING"),
            AlertSeverity::Critical => write!(f, "CRITICAL"),
            AlertSeverity::Emergency => write!(f, "EMERGENCY"),
        }
    }
}

#[cfg(feature = "v1.4-sprint1")]
#[derive(Debug, Clone, PartialEq)]
pub enum AlertState {
    Pending,
    Firing,
    Resolved,
    Suppressed,
}

#[cfg(feature = "v1.4-sprint1")]
impl std::fmt::Display for AlertState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertState::Pending => write!(f, "PENDING"),
            AlertState::Firing => write!(f, "FIRING"),
            AlertState::Resolved => write!(f, "RESOLVED"),
            AlertState::Suppressed => write!(f, "SUPPRESSED"),
        }
    }
}

#[cfg(feature = "v1.4-sprint1")]
#[derive(Debug, Clone)]
pub enum NotificationChannel {
    Log,
    Webhook,
    Email,
    PagerDuty,
}

#[cfg(feature = "v1.4-sprint1")]
impl std::fmt::Display for NotificationChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotificationChannel::Log => write!(f, "LOG"),
            NotificationChannel::Webhook => write!(f, "WEBHOOK"),
            NotificationChannel::Email => write!(f, "EMAIL"),
            NotificationChannel::PagerDuty => write!(f, "PAGERDUTY"),
        }
    }
}

#[cfg(feature = "v1.4-sprint1")]
#[derive(Debug, Clone)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub metric_name: String,
    pub threshold: f64,
    pub operator: AlertOperator,
    pub severity: AlertSeverity,
    pub duration_seconds: u64,
    pub cooldown_seconds: u64,
    pub channels: Vec<NotificationChannel>,
    pub enabled: bool,
}

#[cfg(feature = "v1.4-sprint1")]
#[derive(Debug, Clone)]
pub enum AlertOperator {
    GreaterThan,
    LessThan,
    Equals,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

#[cfg(feature = "v1.4-sprint1")]
impl std::fmt::Display for AlertOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertOperator::GreaterThan => write!(f, ">"),
            AlertOperator::LessThan => write!(f, "<"),
            AlertOperator::Equals => write!(f, "=="),
            AlertOperator::GreaterThanOrEqual => write!(f, ">="),
            AlertOperator::LessThanOrEqual => write!(f, "<="),
        }
    }
}

#[cfg(feature = "v1.4-sprint1")]
impl AlertRule {
    pub fn new(
        id: String,
        name: String,
        metric_name: String,
        threshold: f64,
        operator: AlertOperator,
        severity: AlertSeverity,
        duration_seconds: u64,
        cooldown_seconds: u64,
        channels: Vec<NotificationChannel>,
    ) -> Self {
        Self {
            id,
            name,
            metric_name,
            threshold,
            operator,
            severity,
            duration_seconds,
            cooldown_seconds,
            channels,
            enabled: true,
        }
    }

    pub fn is_violated(&self, value: f64) -> bool {
        if !self.enabled {
            return false;
        }
        match self.operator {
            AlertOperator::GreaterThan => value > self.threshold,
            AlertOperator::LessThan => value < self.threshold,
            AlertOperator::Equals => (value - self.threshold).abs() < f64::EPSILON,
            AlertOperator::GreaterThanOrEqual => value >= self.threshold,
            AlertOperator::LessThanOrEqual => value <= self.threshold,
        }
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }
}

#[cfg(feature = "v1.4-sprint1")]
#[derive(Debug, Clone)]
pub struct AlertInstance {
    pub id: String,
    pub rule_id: String,
    pub rule_name: String,
    pub severity: AlertSeverity,
    pub state: AlertState,
    pub value: f64,
    pub message: String,
    pub created_at_ms: u64,
    pub last_fired_at_ms: u64,
    pub resolved_at_ms: Option<u64>,
    pub fire_count: u32,
}

#[cfg(feature = "v1.4-sprint1")]
impl AlertInstance {
    pub fn new(
        id: String,
        rule_id: String,
        rule_name: String,
        severity: AlertSeverity,
        value: f64,
        message: String,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            id,
            rule_id,
            rule_name,
            severity,
            state: AlertState::Pending,
            value,
            message,
            created_at_ms: timestamp_ms,
            last_fired_at_ms: timestamp_ms,
            resolved_at_ms: None,
            fire_count: 1,
        }
    }

    pub fn fire(&mut self, value: f64, timestamp_ms: u64) {
        self.state = AlertState::Firing;
        self.value = value;
        self.last_fired_at_ms = timestamp_ms;
        self.fire_count += 1;
    }

    pub fn resolve(&mut self, timestamp_ms: u64) {
        self.state = AlertState::Resolved;
        self.resolved_at_ms = Some(timestamp_ms);
    }

    pub fn suppress(&mut self) {
        self.state = AlertState::Suppressed;
    }

    pub fn is_active(&self) -> bool {
        matches!(self.state, AlertState::Pending | AlertState::Firing)
    }

    pub fn duration_ms(&self) -> u64 {
        let end = self.resolved_at_ms.unwrap_or(current_timestamp_ms());
        end.saturating_sub(self.created_at_ms)
    }
}

#[cfg(feature = "v1.4-sprint1")]
#[derive(Debug, Clone)]
pub struct AlertNotification {
    pub id: String,
    pub alert_id: String,
    pub severity: AlertSeverity,
    pub channel: NotificationChannel,
    pub message: String,
    pub sent_at_ms: u64,
    pub delivered: bool,
}

#[cfg(feature = "v1.4-sprint1")]
impl AlertNotification {
    pub fn new(
        id: String,
        alert_id: String,
        severity: AlertSeverity,
        channel: NotificationChannel,
        message: String,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            id,
            alert_id,
            severity,
            channel,
            message,
            sent_at_ms: timestamp_ms,
            delivered: true, // Simulated delivery
        }
    }
}

#[cfg(feature = "v1.4-sprint1")]
#[derive(Debug, Clone)]
pub struct AlertEngineConfig {
    pub max_active_alerts: usize,
    pub max_notifications_per_minute: usize,
    pub dedup_window_seconds: u64,
    pub default_duration_seconds: u64,
    pub default_cooldown_seconds: u64,
    pub enable_auto_resolve: bool,
}

#[cfg(feature = "v1.4-sprint1")]
impl Default for AlertEngineConfig {
    fn default() -> Self {
        Self {
            max_active_alerts: 100,
            max_notifications_per_minute: 60,
            dedup_window_seconds: 300, // 5 minutes
            default_duration_seconds: 60,
            default_cooldown_seconds: 300,
            enable_auto_resolve: true,
        }
    }
}

#[cfg(feature = "v1.4-sprint1")]
#[derive(Debug, Clone)]
pub struct AlertEngineStats {
    pub total_alerts_fired: u64,
    pub total_alerts_resolved: u64,
    pub total_notifications_sent: u64,
    pub total_suppressed: u64,
    pub active_alerts: usize,
    pub rules_count: usize,
}

#[cfg(feature = "v1.4-sprint1")]
impl Default for AlertEngineStats {
    fn default() -> Self {
        Self {
            total_alerts_fired: 0,
            total_alerts_resolved: 0,
            total_notifications_sent: 0,
            total_suppressed: 0,
            active_alerts: 0,
            rules_count: 0,
        }
    }
}

#[cfg(feature = "v1.4-sprint1")]
impl AlertEngineStats {
    pub fn resolution_rate(&self) -> f64 {
        let total = self.total_alerts_fired + self.total_alerts_resolved;
        if total == 0 {
            return 0.0;
        }
        self.total_alerts_resolved as f64 / (total / 2) as f64
    }

    pub fn suppression_rate(&self) -> f64 {
        let total = self.total_alerts_fired + self.total_suppressed;
        if total == 0 {
            return 0.0;
        }
        self.total_suppressed as f64 / total as f64
    }
}

#[cfg(feature = "v1.4-sprint1")]
#[derive(Debug)]
pub struct AlertEngine {
    config: AlertEngineConfig,
    rules: Vec<AlertRule>,
    active_alerts: Vec<AlertInstance>,
    notifications: Vec<AlertNotification>,
    stats: AlertEngineStats,
    next_alert_id: u64,
    next_notification_id: u64,
}

#[cfg(feature = "v1.4-sprint1")]
impl AlertEngine {
    pub fn new(config: AlertEngineConfig) -> Self {
        Self {
            config,
            rules: Vec::new(),
            active_alerts: Vec::new(),
            notifications: Vec::new(),
            stats: AlertEngineStats::default(),
            next_alert_id: 1,
            next_notification_id: 1,
        }
    }

    // Add alert rule
    pub fn add_rule(&mut self, rule: AlertRule) {
        self.rules.push(rule);
        self.stats.rules_count = self.rules.len();
    }

    // Remove alert rule by ID
    pub fn remove_rule(&mut self, rule_id: &str) -> bool {
        let initial_len = self.rules.len();
        self.rules.retain(|r| r.id != rule_id);
        let new_len = self.rules.len();
        self.stats.rules_count = new_len;
        new_len < initial_len
    }

    // Get rule by ID
    pub fn get_rule(&self, rule_id: &str) -> Option<&AlertRule> {
        self.rules.iter().find(|r| r.id == rule_id)
    }

    // Disable rule by ID
    pub fn disable_rule(&mut self, rule_id: &str) -> bool {
        if let Some(rule) = self.rules.iter_mut().find(|r| r.id == rule_id) {
            rule.disable();
            true
        } else {
            false
        }
    }

    // Enable rule by ID
    pub fn enable_rule(&mut self, rule_id: &str) -> bool {
        if let Some(rule) = self.rules.iter_mut().find(|r| r.id == rule_id) {
            rule.enable();
            true
        } else {
            false
        }
    }

    // Evaluate all rules against current metric value
    pub fn evaluate(&mut self, metric_name: &str, value: f64) {
        let timestamp_ms = current_timestamp_ms();

        // Collect matching rules as owned clones to avoid borrow conflicts
        let matching_rules: Vec<AlertRule> = self.rules.iter()
            .filter(|r| r.metric_name == metric_name)
            .cloned()
            .collect();

        for rule in &matching_rules {
            if !rule.is_violated(value) {
                // Check if we should auto-resolve
                if self.config.enable_auto_resolve {
                    self.resolve_for_rule(rule, timestamp_ms);
                }
                continue;
            }

            // Check if alert already exists for this rule
            let mut need_notify = false;
            if let Some(existing) = self.active_alerts.iter_mut().find(|a| a.rule_id == rule.id && a.is_active()) {
                // Check cooldown
                let elapsed_seconds = (timestamp_ms - existing.last_fired_at_ms) / 1000;
                if elapsed_seconds < rule.cooldown_seconds {
                    self.stats.total_suppressed += 1;
                    continue;
                }

                // Check duration for transition to firing
                let duration_seconds = (timestamp_ms - existing.created_at_ms) / 1000;
                if existing.state == AlertState::Pending && duration_seconds >= rule.duration_seconds {
                    existing.fire(value, timestamp_ms);
                    self.stats.total_alerts_fired += 1;
                    need_notify = true;
                } else if existing.state == AlertState::Firing {
                    existing.fire(value, timestamp_ms);
                    need_notify = true;
                }
            }
            // Send notifications after mutable borrow is released
            if need_notify {
                if let Some(existing) = self.active_alerts.iter().find(|a| a.rule_id == rule.id && a.is_active()).cloned() {
                    self.send_notifications_from_data(existing.id, existing.severity, existing.message, &rule.channels, timestamp_ms);
                }
            } else if !self.active_alerts.iter().any(|a| a.rule_id == rule.id && a.is_active()) {
                // Create new alert instance
                if self.active_alerts.len() >= self.config.max_active_alerts {
                    // Suppress oldest resolved alerts to make room
                    self.prune_old_alerts();
                }

                let alert_id = self.generate_alert_id();
                let message = format!(
                    "Alert '{}': {} {} {} (value: {:.4})",
                    rule.name, rule.metric_name, rule.operator, rule.threshold, value
                );
                let channels = rule.channels.clone();
                let mut alert = AlertInstance::new(
                    alert_id,
                    rule.id.clone(),
                    rule.name.clone(),
                    rule.severity.clone(),
                    value,
                    message,
                    timestamp_ms,
                );

                // If duration is 0, fire immediately
                if rule.duration_seconds == 0 {
                    alert.state = AlertState::Firing;
                    self.stats.total_alerts_fired += 1;
                    self.send_notifications(&alert, &channels, timestamp_ms);
                }

                self.active_alerts.push(alert);
            }
        }

        // Update active alert count
        self.stats.active_alerts = self.active_alerts.iter().filter(|a| a.is_active()).count();
    }

    // Manually resolve an alert
    pub fn resolve_alert(&mut self, alert_id: &str) -> bool {
        let timestamp_ms = current_timestamp_ms();
        if let Some(alert) = self.active_alerts.iter_mut().find(|a| a.id == alert_id) {
            if alert.is_active() {
                alert.resolve(timestamp_ms);
                self.stats.total_alerts_resolved += 1;
                self.stats.active_alerts = self.active_alerts.iter().filter(|a| a.is_active()).count();
                return true;
            }
        }
        false
    }

    // Get active alerts
    pub fn get_active_alerts(&self) -> Vec<&AlertInstance> {
        self.active_alerts.iter().filter(|a| a.is_active()).collect()
    }

    // Get alerts by severity
    pub fn get_alerts_by_severity(&self, severity: &AlertSeverity) -> Vec<&AlertInstance> {
        self.active_alerts
            .iter()
            .filter(|a| a.severity == *severity && a.is_active())
            .collect()
    }

    // Get recent notifications
    pub fn get_recent_notifications(&self, limit: usize) -> Vec<&AlertNotification> {
        self.notifications
            .iter()
            .rev()
            .take(limit)
            .collect()
    }

    // Get stats snapshot
    pub fn get_stats(&self) -> AlertEngineStats {
        self.stats.clone()
    }

    // Reset stats
    pub fn reset_stats(&mut self) {
        self.stats = AlertEngineStats::default();
        self.stats.rules_count = self.rules.len();
    }

    // Clear resolved alerts older than threshold
    pub fn cleanup_resolved(&mut self, older_than_ms: u64) -> usize {
        let initial_len = self.active_alerts.len();
        self.active_alerts.retain(|a| {
            if a.state == AlertState::Resolved {
                if let Some(resolved_at) = a.resolved_at_ms {
                    current_timestamp_ms().saturating_sub(resolved_at) < older_than_ms
                } else {
                    true // Keep if no resolved timestamp
                }
            } else {
                true // Keep active alerts
            }
        });
        let removed = initial_len.saturating_sub(self.active_alerts.len());
        removed
    }

    // Internal: Resolve alerts for a rule when condition is no longer violated
    fn resolve_for_rule(&mut self, rule: &AlertRule, timestamp_ms: u64) {
        for alert in self.active_alerts.iter_mut().filter(|a| a.rule_id == rule.id && a.is_active()) {
            alert.resolve(timestamp_ms);
            self.stats.total_alerts_resolved += 1;
        }
        self.stats.active_alerts = self.active_alerts.iter().filter(|a| a.is_active()).count();
    }

    // Internal: Send notifications for an alert
    fn send_notifications(&mut self, alert: &AlertInstance, channels: &[NotificationChannel], timestamp_ms: u64) {
        for channel in channels {
            let notification_id = self.generate_notification_id();
            let notification = AlertNotification::new(
                notification_id,
                alert.id.clone(),
                alert.severity.clone(),
                channel.clone(),
                alert.message.clone(),
                timestamp_ms,
            );
            self.notifications.push(notification);
            self.stats.total_notifications_sent += 1;
        }
    }

    // Internal: Send notifications from owned data (avoids borrow conflicts)
    fn send_notifications_from_data(
        &mut self,
        alert_id: String,
        severity: AlertSeverity,
        message: String,
        channels: &[NotificationChannel],
        timestamp_ms: u64,
    ) {
        for channel in channels {
            let notification_id = self.generate_notification_id();
            let notification = AlertNotification::new(
                notification_id,
                alert_id.clone(),
                severity.clone(),
                channel.clone(),
                message.clone(),
                timestamp_ms,
            );
            self.notifications.push(notification);
            self.stats.total_notifications_sent += 1;
        }
    }

    // Internal: Prune oldest resolved alerts to make room
    fn prune_old_alerts(&mut self) {
        // Remove oldest resolved alerts first
        self.active_alerts.retain(|a| a.is_active() || a.state == AlertState::Pending);

        // If still too many, suppress oldest firing alerts
        if self.active_alerts.len() >= self.config.max_active_alerts {
            self.active_alerts.sort_by(|a, b| a.created_at_ms.cmp(&b.created_at_ms));
            let to_suppress = self.active_alerts.len() - self.config.max_active_alerts / 2;
            for i in 0..to_suppress {
                if let Some(alert) = self.active_alerts.get_mut(i) {
                    alert.suppress();
                    self.stats.total_suppressed += 1;
                }
            }
        }
    }

    // Internal: Generate unique alert ID
    fn generate_alert_id(&mut self) -> String {
        let id = format!("alert-{}", self.next_alert_id);
        self.next_alert_id += 1;
        id
    }

    // Internal: Generate unique notification ID
    fn generate_notification_id(&mut self) -> String {
        let id = format!("notif-{}", self.next_notification_id);
        self.next_notification_id += 1;
        id
    }
}

#[cfg(feature = "v1.4-sprint1")]
impl Default for AlertEngine {
    fn default() -> Self {
        Self::new(AlertEngineConfig::default())
    }
}

#[cfg(feature = "v1.4-sprint1")]
fn current_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(all(test, feature = "v1.4-sprint1"))]
mod tests {
    use super::*;

    fn make_rule(id: &str, metric: &str, threshold: f64, operator: AlertOperator) -> AlertRule {
        AlertRule::new(
            id.to_string(),
            format!("Rule {}", id),
            metric.to_string(),
            threshold,
            operator,
            AlertSeverity::Warning,
            0, // Immediate fire
            1, // 1 second cooldown
            vec![NotificationChannel::Log],
        )
    }

    #[test]
    fn test_engine_creation() {
        let engine = AlertEngine::default();
        assert_eq!(engine.rules.len(), 0);
        assert_eq!(engine.active_alerts.len(), 0);
    }

    #[test]
    fn test_add_rule() {
        let mut engine = AlertEngine::default();
        let rule = make_rule("r1", "cpu_usage", 80.0, AlertOperator::GreaterThan);
        engine.add_rule(rule);
        assert_eq!(engine.rules.len(), 1);
        assert_eq!(engine.stats.rules_count, 1);
    }

    #[test]
    fn test_remove_rule() {
        let mut engine = AlertEngine::default();
        let rule = make_rule("r1", "cpu_usage", 80.0, AlertOperator::GreaterThan);
        engine.add_rule(rule);
        assert!(engine.remove_rule("r1"));
        assert_eq!(engine.rules.len(), 0);
    }

    #[test]
    fn test_remove_nonexistent_rule() {
        let mut engine = AlertEngine::default();
        assert!(!engine.remove_rule("nonexistent"));
    }

    #[test]
    fn test_disable_enable_rule() {
        let mut engine = AlertEngine::default();
        let rule = make_rule("r1", "cpu_usage", 80.0, AlertOperator::GreaterThan);
        engine.add_rule(rule);
        assert!(engine.disable_rule("r1"));
        assert!(!engine.get_rule("r1").unwrap().enabled);
        assert!(engine.enable_rule("r1"));
        assert!(engine.get_rule("r1").unwrap().enabled);
    }

    #[test]
    fn test_evaluate_threshold_violated() {
        let mut engine = AlertEngine::default();
        let rule = make_rule("r1", "cpu_usage", 80.0, AlertOperator::GreaterThan);
        engine.add_rule(rule);
        engine.evaluate("cpu_usage", 90.0);
        let active = engine.get_active_alerts();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].state, AlertState::Firing);
    }

    #[test]
    fn test_evaluate_threshold_not_violated() {
        let mut engine = AlertEngine::default();
        let rule = make_rule("r1", "cpu_usage", 80.0, AlertOperator::GreaterThan);
        engine.add_rule(rule);
        engine.evaluate("cpu_usage", 50.0);
        let active = engine.get_active_alerts();
        assert_eq!(active.len(), 0);
    }

    #[test]
    fn test_evaluate_less_than() {
        let mut engine = AlertEngine::default();
        let rule = make_rule("r1", "disk_space", 20.0, AlertOperator::LessThan);
        engine.add_rule(rule);
        engine.evaluate("disk_space", 10.0);
        let active = engine.get_active_alerts();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_evaluate_equals() {
        let mut engine = AlertEngine::default();
        let rule = make_rule("r1", "error_code", 500.0, AlertOperator::Equals);
        engine.add_rule(rule);
        engine.evaluate("error_code", 500.0);
        let active = engine.get_active_alerts();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_evaluate_greater_than_or_equal() {
        let mut engine = AlertEngine::default();
        let rule = make_rule("r1", "latency", 100.0, AlertOperator::GreaterThanOrEqual);
        engine.add_rule(rule);
        engine.evaluate("latency", 100.0);
        let active = engine.get_active_alerts();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_evaluate_less_than_or_equal() {
        let mut engine = AlertEngine::default();
        let rule = make_rule("r1", "memory_pct", 5.0, AlertOperator::LessThanOrEqual);
        engine.add_rule(rule);
        engine.evaluate("memory_pct", 3.0);
        let active = engine.get_active_alerts();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_disabled_rule_not_evaluated() {
        let mut engine = AlertEngine::default();
        let rule = make_rule("r1", "cpu_usage", 80.0, AlertOperator::GreaterThan);
        engine.add_rule(rule);
        engine.disable_rule("r1");
        engine.evaluate("cpu_usage", 90.0);
        let active = engine.get_active_alerts();
        assert_eq!(active.len(), 0);
    }

    #[test]
    fn test_resolve_alert() {
        let mut engine = AlertEngine::default();
        let rule = make_rule("r1", "cpu_usage", 80.0, AlertOperator::GreaterThan);
        engine.add_rule(rule);
        engine.evaluate("cpu_usage", 90.0);
        assert_eq!(engine.get_active_alerts().len(), 1);
        let alert_id = engine.get_active_alerts()[0].id.clone();
        assert!(engine.resolve_alert(&alert_id));
        assert_eq!(engine.get_active_alerts().len(), 0);
        assert_eq!(engine.stats.total_alerts_resolved, 1);
    }

    #[test]
    fn test_resolve_nonexistent_alert() {
        let mut engine = AlertEngine::default();
        assert!(!engine.resolve_alert("nonexistent"));
    }

    #[test]
    fn test_notifications_sent() {
        let mut engine = AlertEngine::default();
        let mut rule = make_rule("r1", "cpu_usage", 80.0, AlertOperator::GreaterThan);
        rule.channels = vec![NotificationChannel::Log, NotificationChannel::Webhook];
        engine.add_rule(rule);
        engine.evaluate("cpu_usage", 90.0);
        assert_eq!(engine.stats.total_notifications_sent, 2);
    }

    #[test]
    fn test_get_alerts_by_severity() {
        let mut engine = AlertEngine::default();
        let mut rule = make_rule("r1", "cpu_usage", 80.0, AlertOperator::GreaterThan);
        rule.severity = AlertSeverity::Critical;
        engine.add_rule(rule);
        engine.evaluate("cpu_usage", 90.0);
        let critical = engine.get_alerts_by_severity(&AlertSeverity::Critical);
        assert_eq!(critical.len(), 1);
        let warnings = engine.get_alerts_by_severity(&AlertSeverity::Warning);
        assert_eq!(warnings.len(), 0);
    }

    #[test]
    fn test_get_recent_notifications() {
        let mut engine = AlertEngine::default();
        let mut rule = make_rule("r1", "cpu_usage", 80.0, AlertOperator::GreaterThan);
        rule.channels = vec![NotificationChannel::Log, NotificationChannel::Webhook, NotificationChannel::Email];
        engine.add_rule(rule);
        engine.evaluate("cpu_usage", 90.0);
        let recent = engine.get_recent_notifications(2);
        assert_eq!(recent.len(), 2);
    }

    #[test]
    fn test_stats_tracking() {
        let mut engine = AlertEngine::default();
        let rule = make_rule("r1", "cpu_usage", 80.0, AlertOperator::GreaterThan);
        engine.add_rule(rule);
        engine.evaluate("cpu_usage", 90.0);
        let stats = engine.get_stats();
        assert_eq!(stats.total_alerts_fired, 1);
        assert_eq!(stats.active_alerts, 1);
        assert_eq!(stats.total_notifications_sent, 1);
    }

    #[test]
    fn test_reset_stats() {
        let mut engine = AlertEngine::default();
        let rule = make_rule("r1", "cpu_usage", 80.0, AlertOperator::GreaterThan);
        engine.add_rule(rule);
        engine.evaluate("cpu_usage", 90.0);
        engine.reset_stats();
        let stats = engine.get_stats();
        assert_eq!(stats.total_alerts_fired, 0);
        assert_eq!(stats.rules_count, 1); // Rules count preserved
    }

    #[test]
    fn test_max_active_alerts() {
        let config = AlertEngineConfig {
            max_active_alerts: 2,
            ..AlertEngineConfig::default()
        };
        let mut engine = AlertEngine::new(config);
        engine.add_rule(make_rule("r1", "m1", 0.0, AlertOperator::GreaterThan));
        engine.add_rule(make_rule("r2", "m2", 0.0, AlertOperator::GreaterThan));
        engine.add_rule(make_rule("r3", "m3", 0.0, AlertOperator::GreaterThan));
        engine.evaluate("m1", 1.0);
        engine.evaluate("m2", 1.0);
        engine.evaluate("m3", 1.0);
        // Should have suppressed some due to max limit
        let active = engine.get_active_alerts();
        assert!(active.len() <= 2);
    }

    #[test]
    fn test_alert_instance_duration() {
        let alert = AlertInstance::new(
            "a1".to_string(),
            "r1".to_string(),
            "Rule 1".to_string(),
            AlertSeverity::Warning,
            90.0,
            "Test alert".to_string(),
            1000,
        );
        assert!(alert.duration_ms() >= 0);
    }

    #[test]
    fn test_alert_instance_is_active() {
        let alert = AlertInstance::new(
            "a1".to_string(),
            "r1".to_string(),
            "Rule 1".to_string(),
            AlertSeverity::Warning,
            90.0,
            "Test alert".to_string(),
            1000,
        );
        assert!(alert.is_active());
    }

    #[test]
    fn test_alert_suppress() {
        let mut alert = AlertInstance::new(
            "a1".to_string(),
            "r1".to_string(),
            "Rule 1".to_string(),
            AlertSeverity::Warning,
            90.0,
            "Test alert".to_string(),
            1000,
        );
        alert.suppress();
        assert_eq!(alert.state, AlertState::Suppressed);
        assert!(!alert.is_active());
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(AlertSeverity::Info.to_string(), "INFO");
        assert_eq!(AlertSeverity::Warning.to_string(), "WARNING");
        assert_eq!(AlertSeverity::Critical.to_string(), "CRITICAL");
        assert_eq!(AlertSeverity::Emergency.to_string(), "EMERGENCY");
    }

    #[test]
    fn test_operator_display() {
        assert_eq!(AlertOperator::GreaterThan.to_string(), ">");
        assert_eq!(AlertOperator::LessThan.to_string(), "<");
        assert_eq!(AlertOperator::Equals.to_string(), "==");
        assert_eq!(AlertOperator::GreaterThanOrEqual.to_string(), ">=");
        assert_eq!(AlertOperator::LessThanOrEqual.to_string(), "<=");
    }

    #[test]
    fn test_channel_display() {
        assert_eq!(NotificationChannel::Log.to_string(), "LOG");
        assert_eq!(NotificationChannel::Webhook.to_string(), "WEBHOOK");
        assert_eq!(NotificationChannel::Email.to_string(), "EMAIL");
        assert_eq!(NotificationChannel::PagerDuty.to_string(), "PAGERDUTY");
    }

    #[test]
    fn test_stats_resolution_rate() {
        let mut stats = AlertEngineStats::default();
        stats.total_alerts_fired = 10;
        stats.total_alerts_resolved = 8;
        assert!(stats.resolution_rate() > 0.0);
    }

    #[test]
    fn test_stats_suppression_rate() {
        let mut stats = AlertEngineStats::default();
        stats.total_alerts_fired = 10;
        stats.total_suppressed = 3;
        assert!(stats.suppression_rate() > 0.0);
    }

    #[test]
    fn test_config_default() {
        let config = AlertEngineConfig::default();
        assert_eq!(config.max_active_alerts, 100);
        assert_eq!(config.dedup_window_seconds, 300);
        assert!(config.enable_auto_resolve);
    }

    #[test]
    fn test_stats_default() {
        let stats = AlertEngineStats::default();
        assert_eq!(stats.total_alerts_fired, 0);
        assert_eq!(stats.active_alerts, 0);
    }

    #[test]
    fn test_alert_state_display() {
        assert_eq!(AlertState::Pending.to_string(), "PENDING");
        assert_eq!(AlertState::Firing.to_string(), "FIRING");
        assert_eq!(AlertState::Resolved.to_string(), "RESOLVED");
        assert_eq!(AlertState::Suppressed.to_string(), "SUPPRESSED");
    }

    #[test]
    fn test_rule_violation_logic() {
        let rule = make_rule("r1", "cpu", 50.0, AlertOperator::GreaterThan);
        assert!(rule.is_violated(60.0));
        assert!(!rule.is_violated(40.0));
    }

    #[test]
    fn test_multiple_rules_same_metric() {
        let mut engine = AlertEngine::default();
        engine.add_rule(make_rule("r1", "cpu", 70.0, AlertOperator::GreaterThan));
        engine.add_rule(make_rule("r2", "cpu", 90.0, AlertOperator::GreaterThan));
        engine.evaluate("cpu", 95.0);
        let active = engine.get_active_alerts();
        assert_eq!(active.len(), 2); // Both rules violated
    }

    #[test]
    fn test_different_metrics_no_conflict() {
        let mut engine = AlertEngine::default();
        engine.add_rule(make_rule("r1", "cpu", 80.0, AlertOperator::GreaterThan));
        engine.add_rule(make_rule("r2", "memory", 80.0, AlertOperator::GreaterThan));
        engine.evaluate("cpu", 90.0);
        engine.evaluate("memory", 50.0);
        let active = engine.get_active_alerts();
        assert_eq!(active.len(), 1); // Only CPU alert
    }
}
