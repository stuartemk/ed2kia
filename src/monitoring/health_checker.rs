//! Health Checker — System health monitoring with dependency checks and SLA tracking.
//!
//! Performs periodic health checks on system components, tracks SLA compliance,
//! and provides aggregate health status. Feature-gated behind `cfg(feature = "v1.4-sprint1")`.

use std::collections::HashMap;

/// Error types for health checker operations.
#[derive(Debug)]
pub enum HealthError {
    /// Check already registered.
    CheckExists(String),
    /// Check not found.
    CheckNotFound(String),
    /// Invalid threshold.
    InvalidThreshold(String),
}

impl std::fmt::Display for HealthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthError::CheckExists(name) => write!(f, "Check exists: {}", name),
            HealthError::CheckNotFound(name) => write!(f, "Check not found: {}", name),
            HealthError::InvalidThreshold(msg) => write!(f, "Invalid threshold: {}", msg),
        }
    }
}

/// Health status of a component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
            HealthStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// Result of a single health check.
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub check_name: String,
    pub status: HealthStatus,
    pub message: String,
    pub latency_ms: f64,
    pub timestamp_ms: u64,
}

impl CheckResult {
    pub fn new(
        check_name: String,
        status: HealthStatus,
        message: String,
        latency_ms: f64,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            check_name,
            status,
            message,
            latency_ms,
            timestamp_ms,
        }
    }
}

/// Configuration for a health check.
#[derive(Debug, Clone)]
pub struct CheckConfig {
    pub name: String,
    pub description: String,
    /// Maximum allowed latency in milliseconds.
    pub max_latency_ms: f64,
    /// Minimum success rate (0.0 - 1.0) over the window.
    pub min_success_rate: f64,
    /// Check interval in milliseconds.
    pub interval_ms: u64,
    /// Number of consecutive failures before marking unhealthy.
    pub failure_threshold: u32,
    /// Number of consecutive successes before marking healthy.
    pub recovery_threshold: u32,
    /// Enabled.
    pub enabled: bool,
}

impl Default for CheckConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            max_latency_ms: 1000.0,
            min_success_rate: 0.99,
            interval_ms: 30_000,
            failure_threshold: 3,
            recovery_threshold: 2,
            enabled: true,
        }
    }
}

/// State of a registered health check.
#[derive(Debug, Clone)]
pub struct CheckState {
    pub config: CheckConfig,
    pub current_status: HealthStatus,
    pub consecutive_failures: u32,
    pub consecutive_successes: u32,
    pub total_checks: u64,
    pub total_failures: u64,
    pub last_result: Option<CheckResult>,
    pub last_check_ms: u64,
}

impl CheckState {
    pub fn new(config: CheckConfig) -> Self {
        Self {
            config,
            current_status: HealthStatus::Unknown,
            consecutive_failures: 0,
            consecutive_successes: 0,
            total_checks: 0,
            total_failures: 0,
            last_result: None,
            last_check_ms: 0,
        }
    }

    /// Get success rate.
    pub fn success_rate(&self) -> f64 {
        if self.total_checks == 0 {
            return 0.0;
        }
        (self.total_checks - self.total_failures) as f64 / self.total_checks as f64
    }
}

/// SLA target for a component.
#[derive(Debug, Clone)]
pub struct SLATarget {
    pub name: String,
    /// Target availability (0.0 - 1.0).
    pub target_availability: f64,
    /// Measurement window in milliseconds.
    pub window_ms: u64,
    /// Current availability.
    pub current_availability: f64,
    /// SLA compliant.
    pub compliant: bool,
}

impl SLATarget {
    pub fn new(name: String, target_availability: f64, window_ms: u64) -> Self {
        Self {
            name,
            target_availability,
            window_ms,
            current_availability: 1.0,
            compliant: true,
        }
    }

    pub fn update_availability(&mut self, availability: f64) {
        self.current_availability = availability;
        self.compliant = availability >= self.target_availability;
    }
}

/// Configuration for the health checker.
#[derive(Debug, Clone)]
pub struct HealthCheckerConfig {
    /// Enable health checking.
    pub enabled: bool,
    /// Default check interval in milliseconds.
    pub default_interval_ms: u64,
    /// Maximum checks allowed.
    pub max_checks: usize,
    /// Enable SLA tracking.
    pub sla_tracking: bool,
}

impl Default for HealthCheckerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_interval_ms: 30_000,
            max_checks: 256,
            sla_tracking: true,
        }
    }
}

/// Aggregate health report.
#[derive(Debug, Clone)]
pub struct HealthReport {
    pub overall_status: HealthStatus,
    pub total_checks: usize,
    pub healthy_count: usize,
    pub degraded_count: usize,
    pub unhealthy_count: usize,
    pub unknown_count: usize,
    pub results: Vec<CheckResult>,
    pub sla_compliance: Vec<(String, bool)>,
    pub timestamp_ms: u64,
}

/// Health checker engine.
pub struct HealthChecker {
    config: HealthCheckerConfig,
    checks: HashMap<String, CheckState>,
    sla_targets: HashMap<String, SLATarget>,
    current_time_ms: u64,
}

impl HealthChecker {
    pub fn new(config: HealthCheckerConfig) -> Self {
        Self {
            config,
            checks: HashMap::new(),
            sla_targets: HashMap::new(),
            current_time_ms: 0,
        }
    }

    /// Set current time (for testing).
    pub fn set_time(&mut self, now_ms: u64) {
        self.current_time_ms = now_ms;
    }

    /// Register a health check.
    pub fn register_check(&mut self, check: CheckConfig) -> Result<(), HealthError> {
        if self.checks.contains_key(&check.name) {
            return Err(HealthError::CheckExists(check.name));
        }
        if self.checks.len() >= self.config.max_checks {
            return Err(HealthError::InvalidThreshold(format!(
                "Max checks ({}) reached",
                self.config.max_checks
            )));
        }
        self.checks
            .insert(check.name.clone(), CheckState::new(check));
        Ok(())
    }

    /// Register an SLA target.
    pub fn register_sla(&mut self, sla: SLATarget) {
        self.sla_targets.insert(sla.name.clone(), sla);
    }

    /// Execute a health check with the given result.
    pub fn record_check(
        &mut self,
        check_name: &str,
        status: HealthStatus,
        message: String,
        latency_ms: f64,
    ) -> Result<CheckResult, HealthError> {
        let state = self
            .checks
            .get_mut(check_name)
            .ok_or(HealthError::CheckNotFound(check_name.to_string()))?;

        state.total_checks += 1;
        state.last_check_ms = self.current_time_ms;

        let result = if status == HealthStatus::Healthy {
            state.consecutive_successes += 1;
            state.consecutive_failures = 0;
            if state.consecutive_successes >= state.config.recovery_threshold {
                state.current_status = HealthStatus::Healthy;
            }
            CheckResult::new(
                check_name.to_string(),
                state.current_status,
                message,
                latency_ms,
                self.current_time_ms,
            )
        } else {
            state.consecutive_failures += 1;
            state.consecutive_successes = 0;
            state.total_failures += 1;
            if state.consecutive_failures >= state.config.failure_threshold {
                state.current_status = if status == HealthStatus::Degraded {
                    HealthStatus::Degraded
                } else {
                    HealthStatus::Unhealthy
                };
            }
            CheckResult::new(
                check_name.to_string(),
                state.current_status,
                message,
                latency_ms,
                self.current_time_ms,
            )
        };

        state.last_result = Some(result.clone());
        Ok(result)
    }

    /// Get the current status of a check.
    pub fn get_status(&self, check_name: &str) -> Option<HealthStatus> {
        self.checks.get(check_name).map(|s| s.current_status)
    }

    /// Get the last result of a check.
    pub fn get_last_result(&self, check_name: &str) -> Option<&CheckResult> {
        self.checks
            .get(check_name)
            .and_then(|s| s.last_result.as_ref())
    }

    /// Generate a full health report.
    pub fn generate_report(&self) -> HealthReport {
        let mut healthy = 0;
        let mut degraded = 0;
        let mut unhealthy = 0;
        let mut unknown = 0;
        let mut results = Vec::new();

        for state in self.checks.values() {
            match state.current_status {
                HealthStatus::Healthy => healthy += 1,
                HealthStatus::Degraded => degraded += 1,
                HealthStatus::Unhealthy => unhealthy += 1,
                HealthStatus::Unknown => unknown += 1,
            }
            if let Some(ref result) = state.last_result {
                results.push(result.clone());
            }
        }

        let overall = if unhealthy > 0 {
            HealthStatus::Unhealthy
        } else if degraded > 0 {
            HealthStatus::Degraded
        } else if unknown > 0 {
            HealthStatus::Unknown
        } else {
            HealthStatus::Healthy
        };

        let sla_compliance: Vec<(String, bool)> = self
            .sla_targets
            .iter()
            .map(|(name, sla)| (name.clone(), sla.compliant))
            .collect();

        HealthReport {
            overall_status: overall,
            total_checks: self.checks.len(),
            healthy_count: healthy,
            degraded_count: degraded,
            unhealthy_count: unhealthy,
            unknown_count: unknown,
            results,
            sla_compliance,
            timestamp_ms: self.current_time_ms,
        }
    }

    /// Update SLA availability based on check success rates.
    pub fn update_sla_compliance(&mut self) {
        for (name, sla) in self.sla_targets.iter_mut() {
            if let Some(state) = self.checks.get(name) {
                let availability = state.success_rate();
                sla.update_availability(availability);
            }
        }
    }

    /// Get SLA target.
    pub fn get_sla(&self, name: &str) -> Option<&SLATarget> {
        self.sla_targets.get(name)
    }

    /// Get configuration.
    pub fn config(&self) -> &HealthCheckerConfig {
        &self.config
    }

    /// Get total check count.
    pub fn check_count(&self) -> usize {
        self.checks.len()
    }

    /// Remove a check.
    pub fn remove_check(&mut self, name: &str) -> Result<(), HealthError> {
        if self.checks.remove(name).is_some() {
            Ok(())
        } else {
            Err(HealthError::CheckNotFound(name.to_string()))
        }
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new(HealthCheckerConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checker_creation() {
        let checker = HealthChecker::default();
        assert_eq!(checker.check_count(), 0);
    }

    #[test]
    fn test_register_check() {
        let mut checker = HealthChecker::default();
        let check = CheckConfig {
            name: "db".to_string(),
            description: "Database health".to_string(),
            ..Default::default()
        };
        checker.register_check(check).unwrap();
        assert_eq!(checker.check_count(), 1);
    }

    #[test]
    fn test_register_duplicate() {
        let mut checker = HealthChecker::default();
        let check = CheckConfig {
            name: "db".to_string(),
            ..Default::default()
        };
        checker.register_check(check).unwrap();
        let result = checker.register_check(CheckConfig {
            name: "db".to_string(),
            ..Default::default()
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_record_healthy() {
        let mut checker = HealthChecker::default();
        checker.set_time(1000);
        let check = CheckConfig {
            name: "api".to_string(),
            recovery_threshold: 1,
            ..Default::default()
        };
        checker.register_check(check).unwrap();
        checker
            .record_check("api", HealthStatus::Healthy, "OK".to_string(), 5.0)
            .unwrap();
        assert_eq!(checker.get_status("api"), Some(HealthStatus::Healthy));
    }

    #[test]
    fn test_record_failure_threshold() {
        let mut checker = HealthChecker::default();
        let check = CheckConfig {
            name: "api".to_string(),
            failure_threshold: 3,
            ..Default::default()
        };
        checker.register_check(check).unwrap();
        checker
            .record_check("api", HealthStatus::Unhealthy, "err".to_string(), 500.0)
            .unwrap();
        assert_eq!(checker.get_status("api"), Some(HealthStatus::Unknown));
        checker
            .record_check("api", HealthStatus::Unhealthy, "err".to_string(), 500.0)
            .unwrap();
        assert_eq!(checker.get_status("api"), Some(HealthStatus::Unknown));
        checker
            .record_check("api", HealthStatus::Unhealthy, "err".to_string(), 500.0)
            .unwrap();
        assert_eq!(checker.get_status("api"), Some(HealthStatus::Unhealthy));
    }

    #[test]
    fn test_recovery() {
        let mut checker = HealthChecker::default();
        let check = CheckConfig {
            name: "api".to_string(),
            failure_threshold: 1,
            recovery_threshold: 2,
            ..Default::default()
        };
        checker.register_check(check).unwrap();
        checker
            .record_check("api", HealthStatus::Unhealthy, "err".to_string(), 500.0)
            .unwrap();
        assert_eq!(checker.get_status("api"), Some(HealthStatus::Unhealthy));
        checker
            .record_check("api", HealthStatus::Healthy, "ok".to_string(), 5.0)
            .unwrap();
        assert_eq!(checker.get_status("api"), Some(HealthStatus::Unhealthy));
        checker
            .record_check("api", HealthStatus::Healthy, "ok".to_string(), 5.0)
            .unwrap();
        assert_eq!(checker.get_status("api"), Some(HealthStatus::Healthy));
    }

    #[test]
    fn test_generate_report() {
        let mut checker = HealthChecker::default();
        checker.set_time(1000);
        let check1 = CheckConfig {
            name: "a".to_string(),
            recovery_threshold: 1,
            ..Default::default()
        };
        let check2 = CheckConfig {
            name: "b".to_string(),
            failure_threshold: 1,
            ..Default::default()
        };
        checker.register_check(check1).unwrap();
        checker.register_check(check2).unwrap();
        checker
            .record_check("a", HealthStatus::Healthy, "ok".to_string(), 5.0)
            .unwrap();
        checker
            .record_check("b", HealthStatus::Unhealthy, "err".to_string(), 500.0)
            .unwrap();
        let report = checker.generate_report();
        assert_eq!(report.overall_status, HealthStatus::Unhealthy);
        assert_eq!(report.healthy_count, 1);
        assert_eq!(report.unhealthy_count, 1);
    }

    #[test]
    fn test_sla_registration() {
        let mut checker = HealthChecker::default();
        let sla = SLATarget::new("api".to_string(), 0.99, 3600_000);
        checker.register_sla(sla);
        assert!(checker.get_sla("api").is_some());
    }

    #[test]
    fn test_sla_compliance() {
        let mut checker = HealthChecker::default();
        checker.set_time(1000);
        let check = CheckConfig {
            name: "api".to_string(),
            ..Default::default()
        };
        checker.register_check(check).unwrap();
        let sla = SLATarget::new("api".to_string(), 0.99, 3600_000);
        checker.register_sla(sla);
        checker
            .record_check("api", HealthStatus::Healthy, "ok".to_string(), 5.0)
            .unwrap();
        checker
            .record_check("api", HealthStatus::Healthy, "ok".to_string(), 5.0)
            .unwrap();
        checker.update_sla_compliance();
        assert!(checker.get_sla("api").unwrap().compliant);
    }

    #[test]
    fn test_sla_non_compliant() {
        let mut checker = HealthChecker::default();
        let check = CheckConfig {
            name: "api".to_string(),
            ..Default::default()
        };
        checker.register_check(check).unwrap();
        let sla = SLATarget::new("api".to_string(), 0.99, 3600_000);
        checker.register_sla(sla);
        checker
            .record_check("api", HealthStatus::Unhealthy, "err".to_string(), 500.0)
            .unwrap();
        checker.update_sla_compliance();
        assert!(!checker.get_sla("api").unwrap().compliant);
    }

    #[test]
    fn test_remove_check() {
        let mut checker = HealthChecker::default();
        let check = CheckConfig {
            name: "x".to_string(),
            ..Default::default()
        };
        checker.register_check(check).unwrap();
        checker.remove_check("x").unwrap();
        assert_eq!(checker.check_count(), 0);
    }

    #[test]
    fn test_remove_missing() {
        let mut checker = HealthChecker::default();
        assert!(checker.remove_check("missing").is_err());
    }

    #[test]
    fn test_check_state_success_rate() {
        let mut state = CheckState::new(CheckConfig::default());
        state.total_checks = 100;
        state.total_failures = 5;
        assert_eq!(state.success_rate(), 0.95);
    }

    #[test]
    fn test_health_status_display() {
        assert_eq!(HealthStatus::Healthy.to_string(), "healthy");
        assert_eq!(HealthStatus::Degraded.to_string(), "degraded");
        assert_eq!(HealthStatus::Unhealthy.to_string(), "unhealthy");
        assert_eq!(HealthStatus::Unknown.to_string(), "unknown");
    }

    #[test]
    fn test_config_default() {
        let config = HealthCheckerConfig::default();
        assert!(config.enabled);
        assert!(config.sla_tracking);
    }

    #[test]
    fn test_max_checks_limit() {
        let config = HealthCheckerConfig {
            max_checks: 1,
            ..Default::default()
        };
        let mut checker = HealthChecker::new(config);
        checker
            .register_check(CheckConfig {
                name: "a".to_string(),
                ..Default::default()
            })
            .unwrap();
        let result = checker.register_check(CheckConfig {
            name: "b".to_string(),
            ..Default::default()
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_error_display() {
        let e = HealthError::CheckExists("x".to_string());
        assert!(format!("{}", e).contains("x"));
    }

    #[test]
    fn test_get_last_result() {
        let mut checker = HealthChecker::default();
        checker.set_time(1000);
        let check = CheckConfig {
            name: "api".to_string(),
            recovery_threshold: 1,
            ..Default::default()
        };
        checker.register_check(check).unwrap();
        checker
            .record_check("api", HealthStatus::Healthy, "OK".to_string(), 5.0)
            .unwrap();
        let result = checker.get_last_result("api").unwrap();
        assert_eq!(result.message, "OK");
    }

    #[test]
    fn test_report_with_sla() {
        let mut checker = HealthChecker::default();
        checker.set_time(1000);
        let sla = SLATarget::new("api".to_string(), 0.99, 3600_000);
        checker.register_sla(sla);
        let report = checker.generate_report();
        assert_eq!(report.sla_compliance.len(), 1);
    }

    #[test]
    fn test_sla_update_availability() {
        let mut sla = SLATarget::new("api".to_string(), 0.99, 3600_000);
        sla.update_availability(0.995);
        assert!(sla.compliant);
        sla.update_availability(0.98);
        assert!(!sla.compliant);
    }

    #[test]
    fn test_degraded_status() {
        let mut checker = HealthChecker::default();
        let check = CheckConfig {
            name: "api".to_string(),
            failure_threshold: 1,
            ..Default::default()
        };
        checker.register_check(check).unwrap();
        checker
            .record_check("api", HealthStatus::Degraded, "slow".to_string(), 500.0)
            .unwrap();
        assert_eq!(checker.get_status("api"), Some(HealthStatus::Degraded));
    }

    #[test]
    fn test_check_not_found_record() {
        let mut checker = HealthChecker::default();
        let result = checker.record_check("missing", HealthStatus::Healthy, "ok".to_string(), 0.0);
        assert!(result.is_err());
    }
}
