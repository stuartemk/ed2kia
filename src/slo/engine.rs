//! SLO Engine — Service Level Objective tracking, evaluation and enforcement
//!
//! Feature-gated: `#[cfg(feature = "phase8-sprint1")]`
//! Configurable SLOs with automatic degradation when SLO < target for >N windows.
//! Fallback to `core-only` mode, alert routing via `ops/alert_rules_v2.yml`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use thiserror::Error;
use tracing::{debug, info, warn};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum SLOError {
    #[error("Unknown metric: {0}")]
    UnknownMetric(String),
    #[error("SLO not configured: {0}")]
    SloNotConfigured(String),
    #[error("Invalid threshold: {0}")]
    InvalidThreshold(String),
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Compliance status of an SLO evaluation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SLOStatus {
    Compliant,
    Warning,
    Critical,
}

impl std::fmt::Display for SLOStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SLOStatus::Compliant => write!(f, "Compliant"),
            SLOStatus::Warning => write!(f, "Warning"),
            SLOStatus::Critical => write!(f, "Critical"),
        }
    }
}

/// Result of an SLO evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SLOResult {
    pub status: SLOStatus,
    pub breach_duration: u64,
    pub action_taken: String,
    pub audit_log: Vec<String>,
}

impl SLOResult {
    pub fn compliant() -> Self {
        Self {
            status: SLOStatus::Compliant,
            breach_duration: 0,
            action_taken: "none".into(),
            audit_log: vec!["SLO compliant".into()],
        }
    }

    pub fn warning(message: &str) -> Self {
        Self {
            status: SLOStatus::Warning,
            breach_duration: 0,
            action_taken: "alert_sent".into(),
            audit_log: vec![message.into()],
        }
    }

    pub fn critical(message: &str, action: &str, duration: u64) -> Self {
        Self {
            status: SLOStatus::Critical,
            breach_duration: duration,
            action_taken: action.into(),
            audit_log: vec![message.into(), format!("action: {}", action)],
        }
    }
}

/// Configurable SLO definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SLOConfig {
    /// Human-readable name (e.g. "SAE Latency", "Consensus", "Node Uptime")
    pub name: String,
    /// Metric key to track
    pub metric_key: String,
    /// Target value (e.g. 99.9 for 99.9% uptime, 50.0 for 50ms latency)
    pub target: f64,
    /// Warning threshold (percentage of target, e.g. 0.95 = 95% of target)
    pub warning_threshold: f64,
    /// Maximum consecutive breach windows before degradation
    pub max_breach_windows: usize,
    /// Unit label (ms, %, MB, etc.)
    pub unit: String,
}

/// A single metric data point.
#[derive(Debug, Clone)]
struct MetricPoint {
    value: f64,
    timestamp: u64,
}

/// Action taken during degradation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DegradationAction {
    None,
    Alert,
    FallbackCoreOnly,
    Throttle,
    Rollback,
}

impl std::fmt::Display for DegradationAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DegradationAction::None => write!(f, "none"),
            DegradationAction::Alert => write!(f, "alert"),
            DegradationAction::FallbackCoreOnly => write!(f, "fallback_core_only"),
            DegradationAction::Throttle => write!(f, "throttle"),
            DegradationAction::Rollback => write!(f, "rollback"),
        }
    }
}

// ---------------------------------------------------------------------------
// SLOEngine
// ---------------------------------------------------------------------------

pub struct SLOEngine {
    configs: HashMap<String, SLOConfig>,
    windows: HashMap<String, VecDeque<MetricPoint>>,
    breach_counters: HashMap<String, usize>,
    audit_trail: VecDeque<String>,
    max_window_size: usize,
    is_degraded: bool,
}

impl SLOEngine {
    /// Create a new SLO engine with default window size (10 samples).
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
            windows: HashMap::new(),
            breach_counters: HashMap::new(),
            audit_trail: VecDeque::with_capacity(256),
            max_window_size: 10,
            is_degraded: false,
        }
    }

    /// Create with custom window size.
    pub fn with_window_size(max_window: usize) -> Self {
        Self {
            max_window_size: max_window,
            ..Self::new()
        }
    }

    /// Register an SLO configuration.
    pub fn register_slo(&mut self, config: SLOConfig) {
        let key = config.metric_key.clone();
        debug!(name = %config.name, key = %key, "SLO registered");
        self.configs.insert(key, config);
    }

    /// Remove an SLO configuration.
    pub fn unregister_slo(&mut self, metric_key: &str) -> bool {
        let removed = self.configs.remove(metric_key).is_some();
        if removed {
            self.windows.remove(metric_key);
            self.breach_counters.remove(metric_key);
        }
        removed
    }

    // ---- Tracking ---------------------------------------------------------

    /// Track a new metric data point.
    pub fn track_metric(
        &mut self,
        metric_key: &str,
        value: f64,
        timestamp: u64,
    ) -> Result<(), SLOError> {
        if !self.configs.contains_key(metric_key) {
            return Err(SLOError::SloNotConfigured(metric_key.into()));
        }

        self.windows
            .entry(metric_key.into())
            .or_default()
            .push_back(MetricPoint { value, timestamp });

        // Enforce window size
        if let Some(window) = self.windows.get_mut(metric_key) {
            while window.len() > self.max_window_size {
                window.pop_front();
            }
        }

        self.audit(&format!("metric tracked: {} = {:.4}", metric_key, value));

        Ok(())
    }

    // ---- Evaluation -------------------------------------------------------

    /// Evaluate all registered SLOs and return results.
    pub fn evaluate_slo(&mut self, metric_key: &str) -> Result<SLOResult, SLOError> {
        let config = self
            .configs
            .get(metric_key)
            .ok_or_else(|| SLOError::SloNotConfigured(metric_key.into()))?;

        let window = self.windows.get(metric_key);
        let has_data = window.is_some_and(|w| !w.is_empty()); // CLEANUP: map_or(false, |w| ...) -> is_some_and
        if !has_data {
            let name = config.name.clone();
            return Ok(SLOResult::warning(&format!("No data for {}", name)));
        }

        let window = window.unwrap();

        // Calculate average over window
        let avg: f64 = window.iter().map(|p| p.value).sum::<f64>() / window.len() as f64;

        // Determine compliance
        let warning_bound = config.target * config.warning_threshold;

        // For uptime/accuracy: higher is better
        // For latency/error_rate: lower is better
        let is_lower_better = config.metric_key.contains("latency")
            || config.metric_key.contains("error")
            || config.metric_key.contains("memory");

        let (compliant, warning) = if is_lower_better {
            (avg <= config.target, avg <= warning_bound)
        } else {
            (avg >= config.target, avg >= warning_bound)
        };

        // Clone config fields before mutable borrows
        let name = config.name.clone();
        let target = config.target;

        if compliant {
            // Reset breach counter
            *self.breach_counters.entry(metric_key.into()).or_insert(0) = 0;
            self.audit(&format!(
                "SLO compliant: {} avg={:.4} target={}",
                name, avg, target
            ));
            Ok(SLOResult::compliant())
        } else if warning {
            let counter = self.breach_counters.entry(metric_key.into()).or_insert(0);
            *counter += 1;
            let count = *counter;
            self.audit(&format!(
                "SLO warning: {} avg={:.4} (breach count: {})",
                name, avg, count
            ));
            Ok(SLOResult::warning(&format!(
                "{} approaching threshold (avg={:.4})",
                name, avg
            )))
        } else {
            let counter = self.breach_counters.entry(metric_key.into()).or_insert(0);
            *counter += 1;
            let count = *counter;
            self.audit(&format!(
                "SLO breach: {} avg={:.4} < target={} (breach count: {})",
                name, avg, target, count
            ));
            Ok(SLOResult::critical(
                &format!("{} below threshold", name),
                "alert_sent",
                count as u64,
            ))
        }
    }

    // ---- Enforcement ------------------------------------------------------

    /// Enforce SLA: if breach count exceeds max_windows, trigger degradation.
    pub fn enforce_sla(&mut self, metric_key: &str) -> Result<SLOResult, SLOError> {
        let config = self
            .configs
            .get(metric_key)
            .ok_or_else(|| SLOError::SloNotConfigured(metric_key.into()))?;

        // Clone config fields before mutable borrows
        let name = config.name.clone();
        let max_windows = config.max_breach_windows;

        let breach_count = self.breach_counters.get(metric_key).copied().unwrap_or(0);

        if breach_count >= max_windows {
            let action = self.trigger_degradation(&name);
            self.audit(&format!(
                "SLA enforced: {} after {} breaches → {}",
                name, breach_count, action
            ));
            Ok(SLOResult::critical(
                &format!("{} breached SLA after {} windows", name, breach_count),
                &action.to_string(),
                breach_count as u64,
            ))
        } else {
            Ok(self.evaluate_slo(metric_key)?)
        }
    }

    // ---- Degradation ------------------------------------------------------

    /// Trigger automatic degradation for a breached SLO.
    pub fn trigger_degradation(&mut self, slo_name: &str) -> DegradationAction {
        self.is_degraded = true;

        // Determine action based on SLO type
        let action = if slo_name.contains("Latency") || slo_name.contains("latency") {
            DegradationAction::Throttle
        } else if slo_name.contains("Uptime") || slo_name.contains("uptime") {
            DegradationAction::FallbackCoreOnly
        } else if slo_name.contains("Error") || slo_name.contains("error") {
            DegradationAction::Rollback
        } else {
            DegradationAction::Alert
        };

        warn!(
            slo = %slo_name,
            action = %action,
            degraded = true,
            "degradation triggered"
        );

        self.audit(&format!("degradation: {} → {}", slo_name, action));

        action
    }

    /// Reset degradation state (manual recovery).
    pub fn recover(&mut self) {
        self.is_degraded = false;
        self.breach_counters.clear();
        self.audit("degradation recovered: all breach counters reset");
        info!("SLO engine recovered from degraded state");
    }

    /// Check if engine is currently in degraded state.
    pub fn is_degraded(&self) -> bool {
        self.is_degraded
    }

    // ---- Audit ------------------------------------------------------------

    fn audit(&mut self, message: &str) {
        let hash = format!("{:x}", Sha256::digest(message.as_bytes()));
        self.audit_trail
            .push_back(format!("[{}] {}", &hash[..8], message));
        while self.audit_trail.len() > 256 {
            self.audit_trail.pop_front();
        }
    }

    /// Get the full audit trail.
    pub fn get_audit_trail(&self) -> Vec<String> {
        self.audit_trail.iter().cloned().collect()
    }

    /// Get all registered SLO names.
    pub fn slo_names(&self) -> Vec<String> {
        self.configs.values().map(|c| c.name.clone()).collect()
    }

    /// Get breach count for a metric.
    pub fn breach_count(&self, metric_key: &str) -> usize {
        self.breach_counters.get(metric_key).copied().unwrap_or(0)
    }
}

impl Default for SLOEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Unit tests in tests.rs
// ---------------------------------------------------------------------------
