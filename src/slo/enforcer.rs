//! SLA Enforcer — Automatización SLA: degradación progresiva, rollback seguro, notificaciones
//!
//! Implementa `SLAEnforcer` para automatizar la evaluación de SLOs, ejecución de
//! degradación progresiva (4 niveles), rollback seguro y notificaciones a operaciones.
//!
//! Niveles de degradación:
//! - Level 1: Log warning
//! - Level 2: Reduce peers
//! - Level 3: Fallback core-only
//! - Level 4: Auto-rollback
//!
//! **Feature:** `phase8-sprint2`

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

// ============================================================================
// Errors
// ============================================================================

/// Error específico del enforcer SLA
#[derive(Debug, Error)]
pub enum EnforcerError {
    #[error("SLO not registered: {metric_key}")]
    SloNotRegistered { metric_key: String },

    #[error("Rollback failed: {reason}")]
    RollbackFailed { reason: String },

    #[error("Notification delivery failed: {channel}")]
    NotificationFailed { channel: String },

    #[error("Degradation level exceeded: current={current}, max={max}")]
    DegradationExceeded { current: u8, max: u8 },

    #[error("Invalid SLO value: {reason}")]
    InvalidSloValue { reason: String },
}

// ============================================================================
// Degradation Levels
// ============================================================================

/// Niveles de degradación progresiva
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DegradationLevel {
    /// Normal: sin degradación
    Normal,
    /// Level 1: Log warning
    Level1Warning,
    /// Level 2: Reduce peers
    Level2ReducePeers,
    /// Level 3: Fallback core-only
    Level3CoreOnly,
    /// Level 4: Auto-rollback
    Level4Rollback,
}

impl DegradationLevel {
    /// Retorna el nivel numérico
    pub fn as_u8(self) -> u8 {
        match self {
            DegradationLevel::Normal => 0,
            DegradationLevel::Level1Warning => 1,
            DegradationLevel::Level2ReducePeers => 2,
            DegradationLevel::Level3CoreOnly => 3,
            DegradationLevel::Level4Rollback => 4,
        }
    }

    /// Retorna la descripción de acción
    pub fn action_description(self) -> &'static str {
        match self {
            DegradationLevel::Normal => "No action",
            DegradationLevel::Level1Warning => "Log warning",
            DegradationLevel::Level2ReducePeers => "Reduce peer connections",
            DegradationLevel::Level3CoreOnly => "Fallback to core-only mode",
            DegradationLevel::Level4Rollback => "Execute automatic rollback",
        }
    }

    /// Construye desde un nivel numérico
    pub fn from_u8(level: u8) -> Self {
        match level {
            0 => DegradationLevel::Normal,
            1 => DegradationLevel::Level1Warning,
            2 => DegradationLevel::Level2ReducePeers,
            3 => DegradationLevel::Level3CoreOnly,
            4 => DegradationLevel::Level4Rollback,
            _ => DegradationLevel::Level4Rollback,
        }
    }
}

impl std::fmt::Display for DegradationLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DegradationLevel::Normal => write!(f, "Normal"),
            DegradationLevel::Level1Warning => write!(f, "Level1-Warning"),
            DegradationLevel::Level2ReducePeers => write!(f, "Level2-ReducePeers"),
            DegradationLevel::Level3CoreOnly => write!(f, "Level3-CoreOnly"),
            DegradationLevel::Level4Rollback => write!(f, "Level4-Rollback"),
        }
    }
}

// ============================================================================
// SLO Status Record
// ============================================================================

/// Registro de estado de un SLO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SloStatusRecord {
    /// Clave del SLO
    pub metric_key: String,
    /// Nombre del SLO
    pub slo_name: String,
    /// Valor actual
    pub current_value: f64,
    /// Objetivo
    pub target_value: f64,
    /// Ventanas de breach consecutivas
    pub breach_windows: usize,
    /// Nivel de degradación actual
    pub degradation_level: DegradationLevel,
    /// Timestamp del último update (epoch ms)
    pub last_update_ms: u64,
}

impl SloStatusRecord {
    /// Crea nuevo registro
    pub fn new(metric_key: String, slo_name: String, target_value: f64) -> Self {
        Self {
            metric_key,
            slo_name,
            current_value: 0.0,
            target_value,
            breach_windows: 0,
            degradation_level: DegradationLevel::Normal,
            last_update_ms: current_timestamp_ms(),
        }
    }
}

// ============================================================================
// Enforcement Result
// ============================================================================

/// Resultado de una ejecución de enforcement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnforcementResult {
    /// Nivel de degradación aplicado
    pub level: u8,
    /// Acción tomada
    pub action: String,
    /// Si se ejecutó rollback
    pub rollback_executed: bool,
    /// Si se envió notificación
    pub notification_sent: bool,
    /// Timestamp (epoch ms)
    pub timestamp_ms: u64,
}

impl EnforcementResult {
    /// Crea resultado sin acción
    pub fn no_action() -> Self {
        Self {
            level: 0,
            action: "No action required".into(),
            rollback_executed: false,
            notification_sent: false,
            timestamp_ms: current_timestamp_ms(),
        }
    }

    /// Crea resultado con degradación
    pub fn degraded(level: DegradationLevel, action: String, notification_sent: bool) -> Self {
        Self {
            level: level.as_u8(),
            action,
            rollback_executed: level == DegradationLevel::Level4Rollback,
            notification_sent,
            timestamp_ms: current_timestamp_ms(),
        }
    }

    /// Crea resultado de rollback
    pub fn rollback_executed(reason: String) -> Self {
        Self {
            level: 4,
            action: reason,
            rollback_executed: true,
            notification_sent: true,
            timestamp_ms: current_timestamp_ms(),
        }
    }
}

// ============================================================================
// Notification
// ============================================================================

/// Notificación de operaciones
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpsNotification {
    /// Canal de notificación (e.g., "webhook", "log", "email")
    pub channel: String,
    /// Título
    pub title: String,
    /// Mensaje
    pub message: String,
    /// Severidad (info, warning, critical)
    pub severity: String,
    /// Timestamp (epoch ms)
    pub timestamp_ms: u64,
}

impl OpsNotification {
    /// Crea nueva notificación
    pub fn new(channel: String, title: String, message: String, severity: String) -> Self {
        Self {
            channel,
            title,
            message,
            severity,
            timestamp_ms: current_timestamp_ms(),
        }
    }
}

// ============================================================================
// Enforcer Config
// ============================================================================

/// Configuración del enforcer SLA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnforcerConfig {
    /// Ventanas de breach para Level 1
    pub level1_threshold: usize,
    /// Ventanas de breach para Level 2
    pub level2_threshold: usize,
    /// Ventanas de breach para Level 3
    pub level3_threshold: usize,
    /// Ventanas de breach para Level 4 (rollback)
    pub level4_threshold: usize,
    /// Canales de notificación habilitados
    pub notification_channels: Vec<String>,
    /// Capacidad máxima del audit trail
    pub max_audit_entries: usize,
    /// Auto-rollback habilitado
    pub auto_rollback_enabled: bool,
}

impl Default for EnforcerConfig {
    fn default() -> Self {
        Self {
            level1_threshold: 1,
            level2_threshold: 3,
            level3_threshold: 5,
            level4_threshold: 10,
            notification_channels: vec!["log".into(), "webhook".into()],
            max_audit_entries: 512,
            auto_rollback_enabled: true,
        }
    }
}

// ============================================================================
// SLA Enforcer
// ============================================================================

/// Enforcer SLA con degradación progresiva y rollback automático
pub struct SLAEnforcer {
    /// Configuración
    config: EnforcerConfig,
    /// Registros de estado por SLO
    slo_records: HashMap<String, SloStatusRecord>,
    /// Audit trail
    audit_trail: VecDeque<String>,
    /// Notificaciones pendientes
    pending_notifications: VecDeque<OpsNotification>,
    /// Historial de enforcement
    enforcement_history: VecDeque<EnforcementResult>,
}

impl SLAEnforcer {
    /// Crea nuevo enforcer con configuración por defecto
    pub fn new() -> Self {
        Self {
            config: EnforcerConfig::default(),
            slo_records: HashMap::new(),
            audit_trail: VecDeque::with_capacity(512),
            pending_notifications: VecDeque::new(),
            enforcement_history: VecDeque::with_capacity(256),
        }
    }

    /// Crea enforcer con configuración personalizada
    pub fn with_config(config: EnforcerConfig) -> Self {
        Self {
            config,
            ..Self::new()
        }
    }

    /// Registra un SLO para monitoreo
    pub fn register_slo(&mut self, metric_key: String, slo_name: String, target_value: f64) {
        let record = SloStatusRecord::new(metric_key.clone(), slo_name, target_value);
        self.slo_records.insert(metric_key.clone(), record);
        self.audit(&format!("SLO registered: {} (target={})", metric_key, target_value));
    }

    /// Evalúa todos los SLOs registrados y aplica degradación si es necesario
    pub fn evaluate_slos(&mut self) -> Vec<EnforcementResult> {
        let mut results = Vec::new();
        let keys: Vec<String> = self.slo_records.keys().cloned().collect();

        for key in keys {
            if let Some(result) = self.evaluate_single_slo(&key) {
                results.push(result);
            }
        }

        results
    }

    /// Evalúa un SLO individual y reporta su valor actual
    pub fn report_slo_value(
        &mut self,
        metric_key: &str,
        value: f64,
    ) -> Result<(), EnforcerError> {
        // Extraer datos necesarios antes del borrow mutable
        let maybe_record = self.slo_records.get(metric_key).cloned().ok_or_else(|| {
            EnforcerError::SloNotRegistered {
                metric_key: metric_key.into(),
            }
        })?;

        let previous_breach_windows = maybe_record.breach_windows;
        let target_value = maybe_record.target_value;
        let metric_key_clone = metric_key.to_string();

        // Determinar breach usando misma lógica que is_breach
        let is_lower_better = metric_key.contains("latency") || metric_key.contains("error");
        let is_breach = if is_lower_better {
            value > target_value
        } else {
            value < target_value
        };

        // Ahora aplicar cambios con borrow mutable
        let record = self.slo_records.get_mut(&metric_key_clone).unwrap();
        record.current_value = value;
        record.last_update_ms = current_timestamp_ms();

        if is_breach {
            record.breach_windows += 1;
        } else {
            // Reset breach counter si vuelve a compliant
            record.breach_windows = 0;
            record.degradation_level = DegradationLevel::Normal;
        }

        // Ejecutar audit fuera del scope del borrow mutable
        if !is_breach && previous_breach_windows > 0 {
            self.audit(&format!(
                "SLO recovered: {} (breach_windows reset from {})",
                metric_key, previous_breach_windows
            ));
        }

        // Asegurar que los campos se resetean si no se hizo arriba
        if !is_breach {
            let rec = self.slo_records.get_mut(metric_key).unwrap();
            rec.breach_windows = 0;
            rec.degradation_level = DegradationLevel::Normal;
        }

        Ok(())
    }

    /// Dispara degradación basada en breach windows
    pub fn trigger_degradation(
        &mut self,
        metric_key: &str,
    ) -> Result<EnforcementResult, EnforcerError> {
        let record = self.slo_records.get(metric_key).cloned().ok_or_else(|| {
            EnforcerError::SloNotRegistered {
                metric_key: metric_key.into(),
            }
        })?;

        let level = self.determine_degradation_level(record.breach_windows);
        let action = level.action_description().to_string();

        // Actualizar nivel en el registro
        if let Some(rec) = self.slo_records.get_mut(metric_key) {
            rec.degradation_level = level;
        }

        // Enviar notificación
        let notification_sent = self.notify_ops(&record, &level, &action);

        let result = EnforcementResult::degraded(level, action, notification_sent);
        self.enforcement_history.push_back(result.clone());

        self.audit(&format!(
            "Degradation triggered: {} -> {} (breach_windows={})",
            metric_key,
            level,
            record.breach_windows
        ));

        Ok(result)
    }

    /// Ejecuta rollback seguro
    pub fn execute_rollback(
        &mut self,
        metric_key: &str,
        reason: String,
    ) -> Result<EnforcementResult, EnforcerError> {
        if !self.config.auto_rollback_enabled {
            return Err(EnforcerError::RollbackFailed {
                reason: "Auto-rollback disabled in config".into(),
            });
        }

        // Resetear estado del SLO
        if let Some(record) = self.slo_records.get_mut(metric_key) {
            record.breach_windows = 0;
            record.degradation_level = DegradationLevel::Normal;
        }

        let result = EnforcementResult::rollback_executed(reason.clone());
        self.enforcement_history.push_back(result.clone());

        // Notificar
        self.pending_notifications.push_back(OpsNotification::new(
            "webhook".into(),
            "SLA Rollback Executed".into(),
            format!("Metric: {}, Reason: {}", metric_key, reason),
            "critical".into(),
        ));

        self.audit(&format!("Rollback executed: {} - {}", metric_key, reason));

        Ok(result)
    }

    /// Envía notificación a operaciones
    pub fn notify_ops(
        &mut self,
        record: &SloStatusRecord,
        level: &DegradationLevel,
        action: &str,
    ) -> bool {
        let severity = match level {
            DegradationLevel::Normal => "info",
            DegradationLevel::Level1Warning => "warning",
            DegradationLevel::Level2ReducePeers => "warning",
            DegradationLevel::Level3CoreOnly => "critical",
            DegradationLevel::Level4Rollback => "critical",
        };

        let notification = OpsNotification::new(
            "log".into(),
            format!("SLA Degradation: {}", level),
            format!(
                "SLO: {} ({}), Value: {:.4}, Target: {:.4}, Action: {}",
                record.slo_name, record.metric_key, record.current_value, record.target_value, action
            ),
            severity.into(),
        );

        self.pending_notifications.push_back(notification);
        self.audit(&format!(
            "Notification sent: {} -> {}",
            record.metric_key, level
        ));

        true
    }

    /// Retorna las notificaciones pendientes
    pub fn drain_notifications(&mut self) -> Vec<OpsNotification> {
        self.pending_notifications.drain(..).collect()
    }

    /// Retorna el historial de enforcement
    pub fn enforcement_history(&self) -> &[EnforcementResult] {
        self.enforcement_history.as_slices().0
    }

    /// Retorna el audit trail
    pub fn audit_trail(&self) -> &[String] {
        self.audit_trail.as_slices().0
    }

    /// Retorna los registros de SLO
    pub fn slo_records(&self) -> &HashMap<String, SloStatusRecord> {
        &self.slo_records
    }

    /// Retorna la cantidad de SLOs registrados
    pub fn slo_count(&self) -> usize {
        self.slo_records.len()
    }

    /// Retorna la cantidad de notificaciones pendientes
    pub fn pending_notification_count(&self) -> usize {
        self.pending_notifications.len()
    }

    /// Resetea el enforcer
    pub fn reset(&mut self) {
        self.slo_records.clear();
        self.audit_trail.clear();
        self.pending_notifications.clear();
        self.enforcement_history.clear();
    }

    // ---- Internal helpers ----

    fn evaluate_single_slo(&mut self, metric_key: &str) -> Option<EnforcementResult> {
        let record_clone = self.slo_records.get(metric_key).cloned()?;
        let breach_windows = record_clone.breach_windows;

        if breach_windows == 0 {
            return Some(EnforcementResult::no_action());
        }

        let level = self.determine_degradation_level(breach_windows);
        if level == DegradationLevel::Normal {
            return Some(EnforcementResult::no_action());
        }

        let action = level.action_description().to_string();
        let notification_sent = self.notify_ops(&record_clone, &level, &action);

        // Actualizar nivel
        if let Some(rec) = self.slo_records.get_mut(metric_key) {
            rec.degradation_level = level;
        }

        let result = EnforcementResult::degraded(level, action, notification_sent);
        self.enforcement_history.push_back(result.clone());

        Some(result)
    }

    fn determine_degradation_level(&self, breach_windows: usize) -> DegradationLevel {
        if breach_windows >= self.config.level4_threshold {
            DegradationLevel::Level4Rollback
        } else if breach_windows >= self.config.level3_threshold {
            DegradationLevel::Level3CoreOnly
        } else if breach_windows >= self.config.level2_threshold {
            DegradationLevel::Level2ReducePeers
        } else if breach_windows >= self.config.level1_threshold {
            DegradationLevel::Level1Warning
        } else {
            DegradationLevel::Normal
        }
    }

    fn is_breach(&self, record: &SloStatusRecord) -> bool {
        // Para métricas donde mayor es mejor (uptime, accuracy)
        // Para métricas donde menor es mejor (latency, error_rate)
        let is_lower_better = record.metric_key.contains("latency")
            || record.metric_key.contains("error");

        if is_lower_better {
            record.current_value > record.target_value
        } else {
            record.current_value < record.target_value
        }
    }

    fn audit(&mut self, message: &str) {
        self.audit_trail.push_back(message.to_string());
        if self.audit_trail.len() > self.config.max_audit_entries {
            self.audit_trail.pop_front();
        }
    }
}

// ============================================================================
// Default
// ============================================================================

impl Default for SLAEnforcer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enforcer_creation() {
        let enforcer = SLAEnforcer::new();
        assert_eq!(enforcer.slo_count(), 0);
    }

    #[test]
    fn test_register_slo() {
        let mut enforcer = SLAEnforcer::new();
        enforcer.register_slo("node_uptime".into(), "Node Uptime".into(), 99.9);
        assert_eq!(enforcer.slo_count(), 1);
    }

    #[test]
    fn test_report_slo_value_no_breach() {
        let mut enforcer = SLAEnforcer::new();
        enforcer.register_slo("node_uptime".into(), "Node Uptime".into(), 99.9);
        enforcer.report_slo_value("node_uptime", 99.95).unwrap();

        let record = enforcer.slo_records().get("node_uptime").unwrap();
        assert_eq!(record.breach_windows, 0);
        assert_eq!(record.degradation_level, DegradationLevel::Normal);
    }

    #[test]
    fn test_report_slo_value_breach() {
        let mut enforcer = SLAEnforcer::new();
        enforcer.register_slo("node_uptime".into(), "Node Uptime".into(), 99.9);
        enforcer.report_slo_value("node_uptime", 99.0).unwrap();

        let record = enforcer.slo_records().get("node_uptime").unwrap();
        assert_eq!(record.breach_windows, 1);
    }

    #[test]
    fn test_report_unknown_slo() {
        let mut enforcer = SLAEnforcer::new();
        let result = enforcer.report_slo_value("unknown_metric", 50.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_trigger_degradation_level1() {
        let mut enforcer = SLAEnforcer::new();
        enforcer.register_slo("api_error_rate".into(), "API Error Rate".into(), 0.05);
        enforcer.report_slo_value("api_error_rate", 0.1).unwrap();

        let result = enforcer.trigger_degradation("api_error_rate").unwrap();
        assert_eq!(result.level, 1);
        assert!(result.notification_sent);
    }

    #[test]
    fn test_degradation_level_progression() {
        let mut enforcer = SLAEnforcer::with_config(EnforcerConfig {
            level1_threshold: 1,
            level2_threshold: 3,
            level3_threshold: 5,
            level4_threshold: 10,
            ..EnforcerConfig::default()
        });

        assert_eq!(enforcer.determine_degradation_level(0), DegradationLevel::Normal);
        assert_eq!(enforcer.determine_degradation_level(1), DegradationLevel::Level1Warning);
        assert_eq!(enforcer.determine_degradation_level(3), DegradationLevel::Level2ReducePeers);
        assert_eq!(enforcer.determine_degradation_level(5), DegradationLevel::Level3CoreOnly);
        assert_eq!(enforcer.determine_degradation_level(10), DegradationLevel::Level4Rollback);
    }

    #[test]
    fn test_execute_rollback() {
        let mut enforcer = SLAEnforcer::new();
        enforcer.register_slo("sae_latency".into(), "SAE Latency".into(), 100.0);

        let result = enforcer
            .execute_rollback("sae_latency", "Latency SLA breached".into())
            .unwrap();

        assert!(result.rollback_executed);
        assert_eq!(result.level, 4);
    }

    #[test]
    fn test_rollback_disabled() {
        let mut enforcer = SLAEnforcer::with_config(EnforcerConfig {
            auto_rollback_enabled: false,
            ..EnforcerConfig::default()
        });
        enforcer.register_slo("test".into(), "Test".into(), 50.0);

        let result = enforcer.execute_rollback("test", "Test rollback".into());
        assert!(result.is_err());
    }

    #[test]
    fn test_evaluate_slos() {
        let mut enforcer = SLAEnforcer::new();
        enforcer.register_slo("node_uptime".into(), "Uptime".into(), 99.9);
        enforcer.register_slo("api_error_rate".into(), "Error Rate".into(), 0.05);

        // Uptime OK
        enforcer.report_slo_value("node_uptime", 99.95).unwrap();
        // Error rate breach
        enforcer.report_slo_value("api_error_rate", 0.1).unwrap();

        let results = enforcer.evaluate_slos();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_notification_drain() {
        let mut enforcer = SLAEnforcer::new();
        enforcer.register_slo("test".into(), "Test".into(), 50.0);
        enforcer.report_slo_value("test", 40.0).unwrap();
        let _ = enforcer.trigger_degradation("test");

        let notifications = enforcer.drain_notifications();
        assert!(!notifications.is_empty());
        assert_eq!(enforcer.pending_notification_count(), 0);
    }

    #[test]
    fn test_audit_trail() {
        let mut enforcer = SLAEnforcer::new();
        enforcer.register_slo("test".into(), "Test".into(), 50.0);
        let trail = enforcer.audit_trail();
        assert!(!trail.is_empty());
        assert!(trail[0].contains("SLO registered"));
    }

    #[test]
    fn test_enforcement_history() {
        let mut enforcer = SLAEnforcer::new();
        enforcer.register_slo("test".into(), "Test".into(), 50.0);
        enforcer.report_slo_value("test", 40.0).unwrap();
        let _ = enforcer.trigger_degradation("test");

        let history = enforcer.enforcement_history();
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn test_reset() {
        let mut enforcer = SLAEnforcer::new();
        enforcer.register_slo("test".into(), "Test".into(), 50.0);
        enforcer.reset();
        assert_eq!(enforcer.slo_count(), 0);
        assert!(enforcer.audit_trail().is_empty());
    }

    #[test]
    fn test_degradation_level_display() {
        assert_eq!(format!("{}", DegradationLevel::Normal), "Normal");
        assert_eq!(
            format!("{}", DegradationLevel::Level1Warning),
            "Level1-Warning"
        );
        assert_eq!(
            format!("{}", DegradationLevel::Level4Rollback),
            "Level4-Rollback"
        );
    }

    #[test]
    fn test_degradation_level_from_u8() {
        assert_eq!(DegradationLevel::from_u8(0), DegradationLevel::Normal);
        assert_eq!(DegradationLevel::from_u8(3), DegradationLevel::Level3CoreOnly);
        assert_eq!(DegradationLevel::from_u8(9), DegradationLevel::Level4Rollback);
    }

    #[test]
    fn test_enforcement_result_no_action() {
        let result = EnforcementResult::no_action();
        assert_eq!(result.level, 0);
        assert!(!result.rollback_executed);
    }

    #[test]
    fn test_enforcement_result_rollback() {
        let result = EnforcementResult::rollback_executed("Test reason".into());
        assert!(result.rollback_executed);
        assert_eq!(result.level, 4);
    }

    #[test]
    fn test_slo_recovery_resets_breach() {
        let mut enforcer = SLAEnforcer::new();
        enforcer.register_slo("node_uptime".into(), "Uptime".into(), 99.9);

        // Breach
        enforcer.report_slo_value("node_uptime", 99.0).unwrap();
        // Recover
        enforcer.report_slo_value("node_uptime", 99.95).unwrap();

        let record = enforcer.slo_records().get("node_uptime").unwrap();
        assert_eq!(record.breach_windows, 0);
    }

    #[test]
    fn test_config_default() {
        let config = EnforcerConfig::default();
        assert_eq!(config.level1_threshold, 1);
        assert_eq!(config.level4_threshold, 10);
        assert!(config.auto_rollback_enabled);
    }

    #[test]
    fn test_default() {
        let enforcer = SLAEnforcer::default();
        assert_eq!(enforcer.slo_count(), 0);
    }
}
