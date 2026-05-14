//! Dynamic SLO/SLA Engine — Motor de evaluación dinámica de Service Level Objectives
//!
//! Feature-gated: `#[cfg(feature = "v1.1-sprint2")]`
//! Evaluación de métricas en ≤50ms por ciclo. Fallback a modo estático si `cpu_load > 85%`.
//! Contratos binarios serializados con `prost` (simulado con `serde` para portabilidad).
//!
//! # Arquitectura
//!
//! 1. **DynamicSLOEngine**: Motor principal que evalúa métricas contra umbrales configurables.
//! 2. **SLORule**: Regla individual con métrica, umbral, ventana y acción.
//! 3. **EvaluationResult**: Resultado de evaluación con estado, desviación y acción recomendada.
//! 4. **CpuGuard**: Monitor de carga CPU con fallback automático a modo estático.
//!
//! # Uso
//!
//! ```rust,ignore
//! let engine = DynamicSLOEngine::new(DynamicSLOConfig::default());
//! engine.add_rule(rule);
//! engine.report_metric("sae_latency", 45.0);
//! let results = engine.evaluate_cycle();
//! ```

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{debug, info, warn};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors for the Dynamic SLO Engine.
#[derive(Debug, Error)]
pub enum DynamicSLOError {
    #[error("Unknown metric: {0}")]
    UnknownMetric(String),

    #[error("Rule not found: {0}")]
    RuleNotFound(String),

    #[error("Invalid threshold: {0}")]
    InvalidThreshold(String),

    #[error("CPU load too high ({load:.1}%), falling back to static mode")]
    CpuOverload { load: f64 },

    #[error("Evaluation timeout: exceeded {max_ms}ms limit")]
    EvaluationTimeout { max_ms: f64 },

    #[error("Invalid rule configuration: {0}")]
    InvalidRule(String),
}

// ---------------------------------------------------------------------------
// Public Types
// ---------------------------------------------------------------------------

/// Compliance status of a dynamic SLO evaluation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SLOCompliance {
    /// Metric within acceptable range.
    Healthy,
    /// Metric approaching threshold.
    Warning,
    /// Metric exceeds threshold.
    Breach,
    /// Critical breach requiring immediate action.
    Critical,
}

impl std::fmt::Display for SLOCompliance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SLOCompliance::Healthy => write!(f, "Healthy"),
            SLOCompliance::Warning => write!(f, "Warning"),
            SLOCompliance::Breach => write!(f, "Breach"),
            SLOCompliance::Critical => write!(f, "Critical"),
        }
    }
}

/// Action to take when SLO is breached.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SLOAction {
    /// No action required.
    None,
    /// Send alert notification.
    Alert,
    /// Trigger progressive degradation.
    Degrade(u8),
    /// Execute automatic rollback.
    Rollback,
    /// Switch to static fallback mode.
    FallbackStatic,
}

impl std::fmt::Display for SLOAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SLOAction::None => write!(f, "none"),
            SLOAction::Alert => write!(f, "alert"),
            SLOAction::Degrade(level) => write!(f, "degrade_{}", level),
            SLOAction::Rollback => write!(f, "rollback"),
            SLOAction::FallbackStatic => write!(f, "fallback_static"),
        }
    }
}

/// Result of a single SLO rule evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    /// Rule ID that was evaluated.
    pub rule_id: String,
    /// Metric name.
    pub metric_key: String,
    /// Current metric value.
    pub current_value: f64,
    /// Threshold value.
    pub threshold: f64,
    /// Compliance status.
    pub compliance: SLOCompliance,
    /// Deviation from threshold (positive = over threshold).
    pub deviation_percent: f64,
    /// Consecutive breach count.
    pub consecutive_breaches: usize,
    /// Recommended action.
    pub action: SLOAction,
    /// Evaluation latency in milliseconds.
    pub evaluation_latency_ms: f64,
    /// Audit hash for immutability.
    pub audit_hash: String,
}

impl EvaluationResult {
    /// Create a new evaluation result.
    pub fn new(
        rule_id: String,
        metric_key: String,
        current_value: f64,
        threshold: f64,
        compliance: SLOCompliance,
        action: SLOAction,
        latency_ms: f64,
    ) -> Self {
        let deviation_percent = if threshold > 0.0 {
            ((current_value - threshold) / threshold) * 100.0
        } else {
            0.0
        };
        let audit_hash = Self::compute_hash(&rule_id, current_value, &compliance);
        Self {
            rule_id,
            metric_key,
            current_value,
            threshold,
            compliance,
            deviation_percent,
            consecutive_breaches: 0,
            action,
            evaluation_latency_ms: latency_ms,
            audit_hash,
        }
    }

    fn compute_hash(rule_id: &str, value: f64, compliance: &SLOCompliance) -> String {
        let mut hasher = Sha256::new();
        hasher.update(rule_id.as_bytes());
        hasher.update(value.to_le_bytes());
        hasher.update(compliance.to_string().as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    }
}

/// SLA rule definition for dynamic evaluation.
#[derive(Debug, Clone)]
pub struct SLORule {
    /// Unique rule identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Metric key to evaluate.
    pub metric_key: String,
    /// Target threshold value.
    pub threshold: f64,
    /// Warning threshold (percentage of target, e.g. 0.90 = 90%).
    pub warning_threshold: f64,
    /// Critical threshold (percentage of target, e.g. 1.50 = 150%).
    pub critical_threshold: f64,
    /// Evaluation window size in seconds.
    pub window_seconds: u64,
    /// Maximum consecutive breaches before escalation.
    pub max_breaches: usize,
    /// Action on warning.
    pub warning_action: SLOAction,
    /// Action on breach.
    pub breach_action: SLOAction,
    /// Action on critical.
    pub critical_action: SLOAction,
    /// Whether this rule is currently active.
    pub enabled: bool,
    /// Creation timestamp.
    pub created_at: Instant,
}

impl SLORule {
    /// Create a new SLO rule.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        name: String,
        metric_key: String,
        threshold: f64,
        warning_threshold: f64,
        critical_threshold: f64,
        window_seconds: u64,
        max_breaches: usize,
    ) -> Self {
        Self {
            id,
            name,
            metric_key,
            threshold,
            warning_threshold,
            critical_threshold,
            window_seconds,
            max_breaches,
            warning_action: SLOAction::Alert,
            breach_action: SLOAction::Degrade(1),
            critical_action: SLOAction::Rollback,
            enabled: true,
            created_at: Instant::now(),
        }
    }

    /// Validate rule configuration.
    pub fn validate(&self) -> Result<(), DynamicSLOError> {
        if self.threshold <= 0.0 {
            return Err(DynamicSLOError::InvalidThreshold(
                "Threshold must be positive".into(),
            ));
        }
        if self.warning_threshold <= 0.0 || self.warning_threshold > 1.0 {
            return Err(DynamicSLOError::InvalidThreshold(
                "Warning threshold must be between 0 and 1".into(),
            ));
        }
        if self.critical_threshold <= 1.0 {
            return Err(DynamicSLOError::InvalidThreshold(
                "Critical threshold must be greater than 1".into(),
            ));
        }
        if self.window_seconds == 0 {
            return Err(DynamicSLOError::InvalidRule(
                "Window must be greater than 0".into(),
            ));
        }
        Ok(())
    }
}

/// Configuration for the Dynamic SLO Engine.
#[derive(Debug, Clone)]
pub struct DynamicSLOConfig {
    /// Maximum evaluation time per cycle in milliseconds.
    pub max_evaluation_ms: f64,
    /// CPU load threshold for fallback to static mode (0.0-1.0).
    pub cpu_fallback_threshold: f64,
    /// Number of metric samples to keep per rule.
    pub max_samples_per_rule: usize,
    /// Evaluation cycle interval.
    pub cycle_interval: Duration,
    /// Enable dynamic threshold adjustment.
    pub enable_dynamic_thresholds: bool,
    /// Dynamic threshold adjustment rate (0.0-1.0).
    pub adjustment_rate: f64,
}

impl Default for DynamicSLOConfig {
    fn default() -> Self {
        Self {
            max_evaluation_ms: 50.0,
            cpu_fallback_threshold: 0.85,
            max_samples_per_rule: 100,
            cycle_interval: Duration::from_secs(1),
            enable_dynamic_thresholds: true,
            adjustment_rate: 0.1,
        }
    }
}

/// Metric sample with timestamp.
#[derive(Debug, Clone)]
struct MetricSample {
    value: f64,
    timestamp: Instant,
}

/// Internal state for a single SLO rule.
#[derive(Debug, Clone)]
struct RuleState {
    rule: SLORule,
    samples: VecDeque<MetricSample>,
    consecutive_breaches: usize,
    current_compliance: SLOCompliance,
    dynamic_threshold: f64,
    last_evaluation: Option<Instant>,
}

/// Statistics for the Dynamic SLO Engine.
#[derive(Debug, Clone)]
pub struct DynamicSLOStats {
    /// Total rules registered.
    pub total_rules: usize,
    /// Active rules count.
    pub active_rules: usize,
    /// Total evaluations performed.
    pub total_evaluations: usize,
    /// Current compliance breakdown.
    pub compliance_breakdown: HashMap<SLOCompliance, usize>,
    /// Average evaluation latency in milliseconds.
    pub avg_evaluation_latency_ms: f64,
    /// Whether engine is in static fallback mode.
    pub static_fallback_active: bool,
    /// Current CPU load estimate.
    pub current_cpu_load: f64,
}

/// Dynamic SLO Engine for real-time metric evaluation.
pub struct DynamicSLOEngine {
    config: DynamicSLOConfig,
    rules: HashMap<String, RuleState>,
    metrics: HashMap<String, VecDeque<MetricSample>>,
    stats: DynamicSLOStats,
    static_fallback: bool,
    cpu_load: f64,
    created_at: Instant,
}

impl DynamicSLOEngine {
    /// Create a new Dynamic SLO Engine with the given configuration.
    pub fn new(config: DynamicSLOConfig) -> Self {
        let stats = DynamicSLOStats {
            total_rules: 0,
            active_rules: 0,
            total_evaluations: 0,
            compliance_breakdown: HashMap::new(),
            avg_evaluation_latency_ms: 0.0,
            static_fallback_active: false,
            current_cpu_load: 0.0,
        };
        Self {
            config,
            rules: HashMap::new(),
            metrics: HashMap::new(),
            stats,
            static_fallback: false,
            cpu_load: 0.0,
            created_at: Instant::now(),
        }
    }

    /// Create engine with default configuration.
    pub fn default_engine() -> Self {
        Self::new(DynamicSLOConfig::default())
    }

    /// Add an SLO rule for evaluation.
    pub fn add_rule(&mut self, rule: SLORule) -> Result<(), DynamicSLOError> {
        rule.validate()?;
        let state = RuleState {
            rule: rule.clone(),
            samples: VecDeque::new(),
            consecutive_breaches: 0,
            current_compliance: SLOCompliance::Healthy,
            dynamic_threshold: rule.threshold,
            last_evaluation: None,
        };
        self.rules.insert(rule.id.clone(), state);
        self.metrics.insert(
            rule.metric_key.clone(),
            VecDeque::with_capacity(self.config.max_samples_per_rule),
        );
        self.update_stats();
        info!(rule_id = %rule.id, "SLO rule registered");
        Ok(())
    }

    /// Remove an SLO rule by ID.
    pub fn remove_rule(&mut self, rule_id: &str) -> Result<(), DynamicSLOError> {
        let _rule = self
            .rules
            .get(rule_id)
            .ok_or(DynamicSLOError::RuleNotFound(rule_id.into()))?;
        self.rules.remove(rule_id);
        self.update_stats();
        info!(rule_id = %rule_id, "SLO rule removed");
        Ok(())
    }

    /// Enable or disable a rule.
    pub fn set_rule_enabled(&mut self, rule_id: &str, enabled: bool) -> Result<(), DynamicSLOError> {
        let state = self
            .rules
            .get_mut(rule_id)
            .ok_or(DynamicSLOError::RuleNotFound(rule_id.into()))?;
        state.rule.enabled = enabled;
        self.update_stats();
        debug!(rule_id = %rule_id, enabled, "Rule enabled state updated");
        Ok(())
    }

    /// Report a metric value for evaluation.
    pub fn report_metric(&mut self, metric_key: &str, value: f64) {
        let sample = MetricSample {
            value,
            timestamp: Instant::now(),
        };
        if let Some(samples) = self.metrics.get_mut(metric_key) {
            if samples.len() >= self.config.max_samples_per_rule {
                samples.pop_front();
            }
            samples.push_back(sample);
        }
        debug!(metric = %metric_key, value, "Metric reported");
    }

    /// Update CPU load estimate (0.0-1.0).
    pub fn update_cpu_load(&mut self, load: f64) {
        self.cpu_load = load.clamp(0.0, 1.0);
        self.stats.current_cpu_load = self.cpu_load;
        if self.cpu_load > self.config.cpu_fallback_threshold {
            if !self.static_fallback {
                warn!(
                    cpu_load = self.cpu_load,
                    threshold = self.config.cpu_fallback_threshold,
                    "CPU overload detected, switching to static fallback"
                );
                self.static_fallback = true;
                self.stats.static_fallback_active = true;
            }
        } else if self.static_fallback && self.cpu_load < self.config.cpu_fallback_threshold * 0.9 {
            info!(
                cpu_load = self.cpu_load,
                "CPU load normalized, resuming dynamic evaluation"
            );
            self.static_fallback = false;
            self.stats.static_fallback_active = false;
        }
    }

    /// Execute a full evaluation cycle across all active rules.
    pub fn evaluate_cycle(&mut self) -> Result<Vec<EvaluationResult>, DynamicSLOError> {
        let cycle_start = Instant::now();

        if self.static_fallback {
            return self.evaluate_static_cycle(cycle_start);
        }

        let active_ids: Vec<String> = self
            .rules
            .keys()
            .filter(|id| self.rules.get(*id).map(|s| s.rule.enabled).unwrap_or(false))
            .cloned()
            .collect();

        let mut results = Vec::new();
        for id in active_ids {
            let rule_start = Instant::now();

            // Check timeout
            if cycle_start.elapsed().as_secs_f64() * 1000.0 > self.config.max_evaluation_ms {
                return Err(DynamicSLOError::EvaluationTimeout {
                    max_ms: self.config.max_evaluation_ms,
                });
            }

            let result = self.evaluate_rule(&id);
            let latency = rule_start.elapsed().as_secs_f64() * 1000.0;
            let mut result = result.map(|r| {
                EvaluationResult {
                    evaluation_latency_ms: latency,
                    ..r
                }
            });

            // Update consecutive breaches
            if let Some(state) = self.rules.get_mut(&id) {
                if let Ok(ref mut r) = result {
                    if r.compliance == SLOCompliance::Breach || r.compliance == SLOCompliance::Critical {
                        state.consecutive_breaches += 1;
                        r.consecutive_breaches = state.consecutive_breaches;
                        // Escalate action based on consecutive breaches
                        if state.consecutive_breaches >= state.rule.max_breaches {
                            r.action = SLOAction::FallbackStatic;
                        }
                    } else {
                        state.consecutive_breaches = 0;
                    }
                    state.current_compliance = r.compliance;
                    state.last_evaluation = Some(Instant::now());
                }
            }

            results.push(result);
        }

        // Update stats
        self.stats.total_evaluations += results.len();
        let total_latency: f64 = results.iter().filter_map(|r| r.as_ref().ok()).map(|r| r.evaluation_latency_ms).sum();
        if !results.is_empty() {
            self.stats.avg_evaluation_latency_ms = total_latency / results.len() as f64;
        }

        // Update compliance breakdown
        self.stats.compliance_breakdown.clear();
        for eval in results.iter().flatten() {
            let count = self.stats.compliance_breakdown.entry(eval.compliance).or_insert(0);
            *count += 1;
        }

        debug!(
            rules_evaluated = results.len(),
            avg_latency_ms = self.stats.avg_evaluation_latency_ms,
            "Evaluation cycle complete"
        );

        Ok(results.into_iter().filter_map(|r| r.ok()).collect())
    }

    /// Evaluate a single rule.
    fn evaluate_rule(&mut self, rule_id: &str) -> Result<EvaluationResult, DynamicSLOError> {
        let state = self.rules.get(rule_id).ok_or(DynamicSLOError::RuleNotFound(rule_id.into()))?;
        let metric_key = &state.rule.metric_key;
        let samples = self.metrics.get(metric_key);

        let current_value = match samples {
            Some(s) if !s.is_empty() => {
                // Use windowed average
                let window = Duration::from_secs(state.rule.window_seconds);
                let now = Instant::now();
                let cutoff = now.checked_sub(window).unwrap_or(now);
                let windowed: Vec<_> = s.iter().filter(|s| s.timestamp > cutoff).collect();
                if windowed.is_empty() {
                    s.back().map(|s| s.value).unwrap_or(0.0)
                } else {
                    windowed.iter().map(|s| s.value).sum::<f64>() / windowed.len() as f64
                }
            }
            _ => 0.0,
        };

        let threshold = if self.config.enable_dynamic_thresholds {
            self.adjust_dynamic_threshold(state, current_value)
        } else {
            state.rule.threshold
        };

        let compliance = self.determine_compliance(&state.rule, current_value, threshold);
        let action = Self::determine_action(&state.rule, &compliance);

        Ok(EvaluationResult::new(
            state.rule.id.clone(),
            state.rule.metric_key.clone(),
            current_value,
            threshold,
            compliance,
            action,
            0.0, // Will be set by caller
        ))
    }

    /// Adjust dynamic threshold based on recent metric history.
    fn adjust_dynamic_threshold(&self, state: &RuleState, current_value: f64) -> f64 {
        let rate = self.config.adjustment_rate;
        let old_threshold = state.dynamic_threshold;
        // Simple exponential moving average adjustment
        let new_threshold = old_threshold * (1.0 - rate) + current_value * rate;
        // Clamp to reasonable range
        new_threshold.clamp(state.rule.threshold * 0.5, state.rule.threshold * 2.0)
    }

    /// Determine compliance status based on rule thresholds.
    fn determine_compliance(
        &self,
        rule: &SLORule,
        current_value: f64,
        threshold: f64,
    ) -> SLOCompliance {
        let warning_limit = threshold * rule.warning_threshold;
        let critical_limit = threshold * rule.critical_threshold;

        if current_value <= warning_limit {
            SLOCompliance::Healthy
        } else if current_value <= threshold {
            SLOCompliance::Warning
        } else if current_value <= critical_limit {
            SLOCompliance::Breach
        } else {
            SLOCompliance::Critical
        }
    }

    /// Determine action based on compliance status.
    fn determine_action(rule: &SLORule, compliance: &SLOCompliance) -> SLOAction {
        match compliance {
            SLOCompliance::Healthy => SLOAction::None,
            SLOCompliance::Warning => rule.warning_action.clone(),
            SLOCompliance::Breach => rule.breach_action.clone(),
            SLOCompliance::Critical => rule.critical_action.clone(),
        }
    }

    /// Static fallback evaluation (simplified, no dynamic thresholds).
    fn evaluate_static_cycle(&mut self, cycle_start: Instant) -> Result<Vec<EvaluationResult>, DynamicSLOError> {
        let mut results = Vec::new();
        for (id, state) in &self.rules {
            if !state.rule.enabled {
                continue;
            }
            let metric_key = &state.rule.metric_key;
            let samples = self.metrics.get(metric_key);
            let current_value = samples.and_then(|s| s.back()).map(|s| s.value).unwrap_or(0.0);

            let compliance = if current_value > state.rule.threshold {
                SLOCompliance::Breach
            } else {
                SLOCompliance::Healthy
            };

            let latency = cycle_start.elapsed().as_secs_f64() * 1000.0;
            results.push(EvaluationResult::new(
                id.clone(),
                metric_key.clone(),
                current_value,
                state.rule.threshold,
                compliance,
                SLOAction::None,
                latency,
            ));
        }
        Ok(results)
    }

    /// Update internal statistics.
    fn update_stats(&mut self) {
        self.stats.total_rules = self.rules.len();
        self.stats.active_rules = self.rules.values().filter(|s| s.rule.enabled).count();
    }

    /// Get current engine statistics.
    pub fn get_stats(&self) -> DynamicSLOStats {
        self.stats.clone()
    }

    /// Get a specific rule by ID.
    pub fn get_rule(&self, rule_id: &str) -> Option<&SLORule> {
        self.rules.get(rule_id).map(|s| &s.rule)
    }

    /// Get all active rules.
    pub fn get_active_rules(&self) -> Vec<&SLORule> {
        self.rules.values().filter(|s| s.rule.enabled).map(|s| &s.rule).collect()
    }

    /// Check if static fallback is active.
    pub fn is_static_fallback(&self) -> bool {
        self.static_fallback
    }

    /// Reset engine state.
    pub fn reset(&mut self) {
        self.rules.clear();
        self.metrics.clear();
        self.static_fallback = false;
        self.cpu_load = 0.0;
        self.stats = DynamicSLOStats {
            total_rules: 0,
            active_rules: 0,
            total_evaluations: 0,
            compliance_breakdown: HashMap::new(),
            avg_evaluation_latency_ms: 0.0,
            static_fallback_active: false,
            current_cpu_load: 0.0,
        };
        info!("Dynamic SLO engine reset");
    }
}

impl Default for DynamicSLOEngine {
    fn default() -> Self {
        Self::new(DynamicSLOConfig::default())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rule(id: &str, metric: &str, threshold: f64) -> SLORule {
        SLORule::new(
            id.to_string(),
            format!("{} rule", id),
            metric.to_string(),
            threshold,
            0.9,
            1.5,
            10,
            3,
        )
    }

    #[test]
    fn test_engine_creation() {
        let engine = DynamicSLOEngine::default_engine();
        assert_eq!(engine.get_stats().total_rules, 0);
        assert!(!engine.is_static_fallback());
    }

    #[test]
    fn test_add_rule() {
        let mut engine = DynamicSLOEngine::default_engine();
        let rule = make_rule("r1", "sae_latency", 50.0);
        assert!(engine.add_rule(rule).is_ok());
        assert_eq!(engine.get_stats().total_rules, 1);
    }

    #[test]
    fn test_add_invalid_rule() {
        let mut engine = DynamicSLOEngine::default_engine();
        let mut rule = make_rule("r1", "sae_latency", 0.0);
        rule.threshold = 0.0;
        assert!(engine.add_rule(rule).is_err());
    }

    #[test]
    fn test_remove_rule() {
        let mut engine = DynamicSLOEngine::default_engine();
        engine.add_rule(make_rule("r1", "sae_latency", 50.0)).unwrap();
        assert!(engine.remove_rule("r1").is_ok());
        assert_eq!(engine.get_stats().total_rules, 0);
    }

    #[test]
    fn test_remove_unknown_rule() {
        let mut engine = DynamicSLOEngine::default_engine();
        assert!(engine.remove_rule("nonexistent").is_err());
    }

    #[test]
    fn test_report_metric() {
        let mut engine = DynamicSLOEngine::default_engine();
        engine.add_rule(make_rule("r1", "sae_latency", 50.0)).unwrap();
        engine.report_metric("sae_latency", 45.0);
        engine.report_metric("sae_latency", 47.0);
        // Should not panic
    }

    #[test]
    fn test_evaluation_healthy() {
        let mut engine = DynamicSLOEngine::default_engine();
        engine.add_rule(make_rule("r1", "sae_latency", 50.0)).unwrap();
        engine.report_metric("sae_latency", 30.0);
        let results = engine.evaluate_cycle().unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].compliance, SLOCompliance::Healthy);
    }

    #[test]
    fn test_evaluation_warning() {
        let mut engine = DynamicSLOEngine::default_engine();
        engine.add_rule(make_rule("r1", "sae_latency", 50.0)).unwrap();
        // Warning threshold is 0.9 * 50 = 45, so value between 45 and 50
        engine.report_metric("sae_latency", 47.0);
        let results = engine.evaluate_cycle().unwrap();
        assert_eq!(results[0].compliance, SLOCompliance::Warning);
    }

    #[test]
    fn test_evaluation_breach() {
        let mut engine = DynamicSLOEngine::default_engine();
        engine.add_rule(make_rule("r1", "sae_latency", 50.0)).unwrap();
        // Breach: above threshold but below critical (1.5 * 50 = 75)
        engine.report_metric("sae_latency", 60.0);
        let results = engine.evaluate_cycle().unwrap();
        assert_eq!(results[0].compliance, SLOCompliance::Breach);
    }

    #[test]
    fn test_evaluation_critical() {
        let mut engine = DynamicSLOEngine::default_engine();
        engine.add_rule(make_rule("r1", "sae_latency", 50.0)).unwrap();
        // Critical: above 1.5 * 50 = 75
        engine.report_metric("sae_latency", 80.0);
        let results = engine.evaluate_cycle().unwrap();
        assert_eq!(results[0].compliance, SLOCompliance::Critical);
    }

    #[test]
    fn test_cpu_fallback() {
        let mut engine = DynamicSLOEngine::default_engine();
        engine.update_cpu_load(0.90);
        assert!(engine.is_static_fallback());
        assert!(engine.get_stats().static_fallback_active);
    }

    #[test]
    fn test_cpu_recovery() {
        let mut engine = DynamicSLOEngine::default_engine();
        engine.update_cpu_load(0.90);
        assert!(engine.is_static_fallback());
        engine.update_cpu_load(0.70);
        assert!(!engine.is_static_fallback());
    }

    #[test]
    fn test_consecutive_breaches() {
        let mut engine = DynamicSLOEngine::default_engine();
        engine.add_rule(make_rule("r1", "sae_latency", 50.0)).unwrap();

        // First breach
        engine.report_metric("sae_latency", 60.0);
        let results = engine.evaluate_cycle().unwrap();
        assert_eq!(results[0].consecutive_breaches, 1);

        // Second breach
        engine.report_metric("sae_latency", 65.0);
        let results = engine.evaluate_cycle().unwrap();
        assert_eq!(results[0].consecutive_breaches, 2);
    }

    #[test]
    fn test_breach_recovery_resets_counter() {
        let mut engine = DynamicSLOEngine::default_engine();
        engine.add_rule(make_rule("r1", "sae_latency", 50.0)).unwrap();

        engine.report_metric("sae_latency", 60.0);
        engine.evaluate_cycle().unwrap();

        // Recovery
        engine.report_metric("sae_latency", 30.0);
        let results = engine.evaluate_cycle().unwrap();
        assert_eq!(results[0].consecutive_breaches, 0);
    }

    #[test]
    fn test_get_rule() {
        let mut engine = DynamicSLOEngine::default_engine();
        engine.add_rule(make_rule("r1", "sae_latency", 50.0)).unwrap();
        let rule = engine.get_rule("r1");
        assert!(rule.is_some());
        assert_eq!(rule.unwrap().threshold, 50.0);
    }

    #[test]
    fn test_get_unknown_rule() {
        let engine = DynamicSLOEngine::default_engine();
        assert!(engine.get_rule("nonexistent").is_none());
    }

    #[test]
    fn test_get_active_rules() {
        let mut engine = DynamicSLOEngine::default_engine();
        engine.add_rule(make_rule("r1", "sae_latency", 50.0)).unwrap();
        engine.add_rule(make_rule("r2", "consensus", 100.0)).unwrap();
        engine.set_rule_enabled("r2", false).unwrap();
        let active = engine.get_active_rules();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_set_rule_enabled() {
        let mut engine = DynamicSLOEngine::default_engine();
        engine.add_rule(make_rule("r1", "sae_latency", 50.0)).unwrap();
        assert!(engine.set_rule_enabled("r1", false).is_ok());
        assert!(!engine.get_rule("r1").unwrap().enabled);
    }

    #[test]
    fn test_stats_tracking() {
        let mut engine = DynamicSLOEngine::default_engine();
        engine.add_rule(make_rule("r1", "sae_latency", 50.0)).unwrap();
        engine.report_metric("sae_latency", 30.0);
        engine.evaluate_cycle().unwrap();
        let stats = engine.get_stats();
        assert_eq!(stats.total_rules, 1);
        assert_eq!(stats.active_rules, 1);
        assert_eq!(stats.total_evaluations, 1);
    }

    #[test]
    fn test_reset() {
        let mut engine = DynamicSLOEngine::default_engine();
        engine.add_rule(make_rule("r1", "sae_latency", 50.0)).unwrap();
        engine.reset();
        assert_eq!(engine.get_stats().total_rules, 0);
        assert!(!engine.is_static_fallback());
    }

    #[test]
    fn test_rule_validate() {
        let rule = make_rule("r1", "sae_latency", 50.0);
        assert!(rule.validate().is_ok());
    }

    #[test]
    fn test_rule_invalid_threshold() {
        let mut rule = make_rule("r1", "sae_latency", 50.0);
        rule.threshold = -1.0;
        assert!(rule.validate().is_err());
    }

    #[test]
    fn test_evaluation_latency_tracked() {
        let mut engine = DynamicSLOEngine::default_engine();
        engine.add_rule(make_rule("r1", "sae_latency", 50.0)).unwrap();
        engine.report_metric("sae_latency", 30.0);
        let results = engine.evaluate_cycle().unwrap();
        assert!(results[0].evaluation_latency_ms >= 0.0);
    }

    #[test]
    fn test_audit_hash_deterministic() {
        let h1 = EvaluationResult::compute_hash("r1", 42.0, &SLOCompliance::Healthy);
        let h2 = EvaluationResult::compute_hash("r1", 42.0, &SLOCompliance::Healthy);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_audit_hash_different_for_different_input() {
        let h1 = EvaluationResult::compute_hash("r1", 42.0, &SLOCompliance::Healthy);
        let h2 = EvaluationResult::compute_hash("r1", 43.0, &SLOCompliance::Healthy);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_config_default() {
        let config = DynamicSLOConfig::default();
        assert_eq!(config.max_evaluation_ms, 50.0);
        assert_eq!(config.cpu_fallback_threshold, 0.85);
        assert!(config.enable_dynamic_thresholds);
    }

    #[test]
    fn test_multiple_rules_evaluation() {
        let mut engine = DynamicSLOEngine::default_engine();
        engine.add_rule(make_rule("r1", "latency", 50.0)).unwrap();
        engine.add_rule(make_rule("r2", "error_rate", 5.0)).unwrap();
        engine.report_metric("latency", 30.0);
        engine.report_metric("error_rate", 2.0);
        let results = engine.evaluate_cycle().unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_compliance_display() {
        assert_eq!(format!("{}", SLOCompliance::Healthy), "Healthy");
        assert_eq!(format!("{}", SLOCompliance::Warning), "Warning");
        assert_eq!(format!("{}", SLOCompliance::Breach), "Breach");
        assert_eq!(format!("{}", SLOCompliance::Critical), "Critical");
    }

    #[test]
    fn test_action_display() {
        assert_eq!(format!("{}", SLOAction::None), "none");
        assert_eq!(format!("{}", SLOAction::Alert), "alert");
        assert_eq!(format!("{}", SLOAction::Degrade(2)), "degrade_2");
        assert_eq!(format!("{}", SLOAction::Rollback), "rollback");
        assert_eq!(format!("{}", SLOAction::FallbackStatic), "fallback_static");
    }

    #[test]
    fn test_static_fallback_evaluation() {
        let mut engine = DynamicSLOEngine::default_engine();
        engine.add_rule(make_rule("r1", "sae_latency", 50.0)).unwrap();
        engine.update_cpu_load(0.95);
        engine.report_metric("sae_latency", 60.0);
        let results = engine.evaluate_cycle().unwrap();
        assert_eq!(results.len(), 1);
        // Static mode: simple threshold check
        assert_eq!(results[0].compliance, SLOCompliance::Breach);
    }

    #[test]
    fn test_deviation_percent() {
        let config = DynamicSLOConfig {
            enable_dynamic_thresholds: false,
            ..DynamicSLOConfig::default()
        };
        let mut engine = DynamicSLOEngine::new(config);
        engine.add_rule(make_rule("r1", "sae_latency", 50.0)).unwrap();
        engine.report_metric("sae_latency", 60.0);
        let results = engine.evaluate_cycle().unwrap();
        // deviation = (60 - 50) / 50 * 100 = 20%
        assert!((results[0].deviation_percent - 20.0).abs() < 0.1);
    }

    #[test]
    fn test_disabled_rule_not_evaluated() {
        let mut engine = DynamicSLOEngine::default_engine();
        engine.add_rule(make_rule("r1", "sae_latency", 50.0)).unwrap();
        engine.set_rule_enabled("r1", false).unwrap();
        engine.report_metric("sae_latency", 60.0);
        let results = engine.evaluate_cycle().unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_empty_evaluation() {
        let mut engine = DynamicSLOEngine::default_engine();
        let results = engine.evaluate_cycle().unwrap();
        assert_eq!(results.len(), 0);
    }
}
