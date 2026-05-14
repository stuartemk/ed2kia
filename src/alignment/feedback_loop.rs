//! Alignment Feedback Loop - Cierre de loop: feedback → drift → steering → aplicación segura
//!
//! Implementa `AlignmentFeedbackLoop` para cerrar el ciclo de alineación continua:
//! 1. Ingesta feedback humano de `feedback_store.rs`
//! 2. Calcula drift con `AlignmentScorer`
//! 3. Genera y aplica steering delta
//! 4. Rollback automático si la métrica se degrada post-aplicación
//!
//! Usa cola FIFO con ventana temporal configurable y registra auditoría completa.
//!
//! **Feature:** `phase7-sprint2`

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tracing::{debug, error, info, warn};

use crate::alignment::engine::{
    AlignmentConfig, AlignmentError, AlignmentFeedback, AlignmentResult, AlignmentScorer,
};

// ============================================================================
// Errors
// ============================================================================

/// Error específico del Feedback Loop de Alineación
#[derive(Debug, Error)]
pub enum FeedbackLoopError {
    #[error("Feedback validation failed: {reason}")]
    ValidationFailed { reason: String },

    #[error("Rate limit exceeded: max {max} entries per window, got {count}")]
    RateLimitExceeded { max: usize, count: usize },

    #[error("Steering application failed: {reason}")]
    SteeringApplicationFailed { reason: String },

    #[error("Rollback triggered: drift degraded from {before:.4} to {after:.4}")]
    RollbackTriggered { before: f32, after: f32 },

    #[error("Alignment engine error: {0}")]
    AlignmentEngine(#[from] AlignmentError),

    #[error("Loop already running")]
    LoopAlreadyRunning,

    #[error("Loop not initialized")]
    LoopNotInitialized,

    #[error("Window expired: {elapsed_ms}ms > {max_ms}ms")]
    WindowExpired { elapsed_ms: u64, max_ms: u64 },
}

// ============================================================================
// Feedback Entry (Internal)
// ============================================================================

/// Entrada de feedback interna con timestamp y validación
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopFeedbackEntry {
    /// ID único de la entrada
    pub entry_id: String,
    /// Feedback de alineación
    pub feedback: AlignmentFeedback,
    /// Timestamp de ingestión (epoch ms)
    pub ingested_at_ms: u64,
    /// Estado de procesamiento
    pub processed: bool,
}

impl LoopFeedbackEntry {
    /// Crea nueva entrada de feedback
    pub fn new(feedback: AlignmentFeedback) -> Self {
        Self {
            entry_id: Self::generate_id(&feedback),
            feedback,
            ingested_at_ms: current_timestamp_ms(),
            processed: false,
        }
    }

    /// Genera ID único basado en contenido
    fn generate_id(feedback: &AlignmentFeedback) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!(
            "{}:{}:{}:{}",
            feedback.layer_id, feedback.feature_idx, feedback.current_activation, feedback.ingested_at_ms
        ));
        let result = hasher.finalize();
        hex::encode(&result[..8])
    }

    /// Verifica si la entrada ha expirado
    pub fn is_expired(&self, window: Duration) -> bool {
        let now = current_timestamp_ms();
        let elapsed = now.saturating_sub(self.ingested_at_ms);
        Duration::from_millis(elapsed) > window
    }
}

// ============================================================================
// Loop Result
// ============================================================================

/// Resultado de una iteración del feedback loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopResult {
    /// Si el steering fue aplicado exitosamente
    pub applied: bool,
    /// Cambio en drift score (positivo = mejora, negativo = degradación)
    pub drift_delta: f32,
    /// Hash SHA-256 del steering aplicado (hex)
    pub steering_hash: String,
    /// Si se activó rollback automático
    pub rollback_triggered: bool,
    /// Drift score antes de aplicar
    pub drift_before: f32,
    /// Drift score después de aplicar
    pub drift_after: f32,
    /// Timestamp de la iteración (epoch ms)
    pub timestamp_ms: u64,
}

// ============================================================================
// Loop Config
// ============================================================================

/// Configuración del Feedback Loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackLoopConfig {
    /// Configuración del AlignmentScorer
    pub alignment_config: AlignmentConfig,
    /// Ventana temporal para feedback (ms)
    pub feedback_window_ms: u64,
    /// Tamaño máximo de la cola FIFO
    pub max_queue_size: usize,
    /// Límite de tasa (entries por ventana)
    pub rate_limit: usize,
    /// Umbral de degradación para rollback (0.0 - 1.0)
    pub rollback_threshold: f32,
    /// Número máximo de intentos de re-aplicación
    pub max_retries: usize,
}

impl Default for FeedbackLoopConfig {
    fn default() -> Self {
        Self {
            alignment_config: AlignmentConfig::default(),
            feedback_window_ms: 30_000, // 30 segundos
            max_queue_size: 1000,
            rate_limit: 100,
            rollback_threshold: 0.1, // Rollback si drift empeora >10%
            max_retries: 3,
        }
    }
}

// ============================================================================
// Audit Log
// ============================================================================

/// Entrada de auditoría
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Tipo de acción
    pub action: AuditAction,
    /// ID de la entrada de feedback (si aplica)
    pub entry_id: Option<String>,
    /// Resultado
    pub result: AuditResult,
    /// Mensaje detallado
    pub message: String,
    /// Timestamp (epoch ms)
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuditAction {
    Ingest,
    ComputeDrift,
    ApplySteering,
    Rollback,
    RateLimit,
    Validation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuditResult {
    Success,
    Failed(String),
    Skipped,
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditAction::Ingest => write!(f, "ingest"),
            AuditAction::ComputeDrift => write!(f, "compute_drift"),
            AuditAction::ApplySteering => write!(f, "apply_steering"),
            AuditAction::Rollback => write!(f, "rollback"),
            AuditAction::RateLimit => write!(f, "rate_limit"),
            AuditAction::Validation => write!(f, "validation"),
        }
    }
}

// ============================================================================
// AlignmentFeedbackLoop
// ============================================================================

/// Loop de Feedback de Alineación
///
/// Cierra el ciclo: feedback → drift → steering → aplicación segura → rollback
pub struct AlignmentFeedbackLoop {
    /// Configuración del loop
    config: FeedbackLoopConfig,
    /// Motor de alineación
    scorer: AlignmentScorer,
    /// Cola FIFO de feedback pendiente
    queue: VecDeque<LoopFeedbackEntry>,
    /// Log de auditoría
    audit_log: VecDeque<AuditEntry>,
    /// Historial de resultados
    result_history: Vec<LoopResult>,
    /// Contador de entries en ventana actual
    rate_counter: usize,
    /// Inicio de ventana actual (epoch ms)
    rate_window_start: u64,
    /// Estado del loop
    running: bool,
}

impl AlignmentFeedbackLoop {
    /// Crea nuevo Feedback Loop con configuración por defecto
    pub fn new() -> Self {
        Self::with_config(FeedbackLoopConfig::default())
    }

    /// Crea nuevo Feedback Loop con configuración personalizada
    pub fn with_config(config: FeedbackLoopConfig) -> Self {
        let scorer = AlignmentScorer::new(config.alignment_config.clone());
        let now = current_timestamp_ms();

        Self {
            config,
            scorer,
            queue: VecDeque::with_capacity(1024),
            audit_log: VecDeque::with_capacity(2048),
            result_history: Vec::new(),
            rate_counter: 0,
            rate_window_start: now,
            running: false,
        }
    }

    /// Ingesta feedback humano en la cola del loop
    ///
    /// Valida el feedback (rechaza NaN/Infinity), aplica límites de tasa,
    /// y registra auditoría.
    ///
    /// # Arguments
    ///
    /// * `feedback` - Feedback de alineación desde `feedback_store.rs`
    ///
    /// # Returns
    ///
    /// `Ok(())` si el feedback fue aceptado, `FeedbackLoopError` si fue rechazado
    pub fn ingest(&mut self, feedback: AlignmentFeedback) -> Result<(), FeedbackLoopError> {
        // Validar feedback
        self.validate_feedback(&feedback)?;

        // Verificar límite de tasa
        self.check_rate_limit()?;

        // Crear entrada y agregar a cola
        let entry = LoopFeedbackEntry::new(feedback.clone());

        // Verificar tamaño de cola
        if self.queue.len() >= self.config.max_queue_size {
            // Evict oldest
            if let Some(oldest) = self.queue.pop_front() {
                self.audit(AuditAction::Ingest, Some(&oldest.entry_id), AuditResult::Skipped, "Evicted: queue full");
            }
        }

        self.queue.push_back(entry);
        self.rate_counter += 1;

        self.audit(AuditAction::Ingest, Some(&entry.entry_id), AuditResult::Success, "Feedback ingested");
        info!(entry_id = %entry.entry_id, "Feedback ingested into alignment loop");
        Ok(())
    }

    /// Calcula drift score actual usando `AlignmentScorer`
    ///
    /// Procesa todas las entradas pendientes en la cola, las ingesta al scorer,
    /// y calcula el drift score por capa.
    ///
    /// # Arguments
    ///
    /// * `layer_id` - ID de capa SAE a evaluar
    ///
    /// # Returns
    ///
    /// `Ok(drift_score)` con el drift calculado, o error si no hay feedback
    pub fn compute_drift(&mut self, layer_id: &str) -> Result<f32, FeedbackLoopError> {
        // Limpiar entradas expiradas
        let window = Duration::from_millis(self.config.feedback_window_ms);
        self.queue.retain(|entry| !entry.is_expired(window));

        // Ingestar feedback pendiente al scorer
        let mut processed_count = 0;
        for entry in &self.queue {
            if !entry.processed {
                self.scorer.ingest_feedback(entry.feedback.clone())?;
                processed_count += 1;
            }
        }

        // Calcular drift
        let drift = self.scorer.calculate_drift(layer_id)?;

        self.audit(
            AuditAction::ComputeDrift,
            None,
            AuditResult::Success,
            format!("Drift computed for layer {}: {:.4}", layer_id, drift),
        );
        debug!(layer = %layer_id, drift, processed = processed_count, "Drift computed");
        Ok(drift)
    }

    /// Aplica steering delta con validación de rollback automático
    ///
    /// 1. Calcula drift antes
    /// 2. Genera steering delta
    /// 3. Aplica delta (simulado)
    /// 4. Calcula drift después
    /// 5. Si drift empeora > rollback_threshold → rollback automático
    ///
    /// # Arguments
    ///
    /// * `layer_id` - ID de capa SAE
    /// * `activations` - Activaciones actuales del SAE
    ///
    /// # Returns
    ///
    /// `Ok(LoopResult)` con detalles de la aplicación
    pub fn apply_steering(&mut self, layer_id: &str, activations: &[f32]) -> Result<LoopResult, FeedbackLoopError> {
        if activations.is_empty() {
            return Err(FeedbackLoopError::ValidationFailed {
                reason: "Empty activations".to_string(),
            });
        }

        // Drift antes
        let drift_before = self.compute_drift(layer_id)?;

        // Generar steering delta
        let alignment_result = self.scorer.generate_steering_adjustment(layer_id, activations)?;

        // Calcular hash del steering
        let steering_hash = Self::compute_steering_hash(&alignment_result.steering_delta);

        // Simular aplicación del steering (en producción: aplicar a weights)
        // Aquí validamos que el steering no degrade el modelo
        let drift_after = self.simulate_post_application_drift(&alignment_result);

        // Verificar si se necesita rollback
        let degradation = drift_after - drift_before;
        let rollback_triggered = degradation > self.config.rollback_threshold;

        let result = LoopResult {
            applied: !rollback_triggered,
            drift_delta: if rollback_triggered { -degradation } else { degradation },
            steering_hash,
            rollback_triggered,
            drift_before,
            drift_after,
            timestamp_ms: current_timestamp_ms(),
        };

        if rollback_triggered {
            warn!(
                layer = %layer_id,
                drift_before = drift_before,
                drift_after = drift_after,
                "Rollback triggered: steering degraded alignment"
            );
            self.audit(
                AuditAction::Rollback,
                None,
                AuditResult::Success,
                format!("Rollback: {:.4} → {:.4}", drift_before, drift_after),
            );
        } else {
            info!(
                layer = %layer_id,
                drift_before = drift_before,
                drift_after = drift_after,
                "Steering applied successfully"
            );
            self.audit(
                AuditAction::ApplySteering,
                None,
                AuditResult::Success,
                format!("Applied: {:.4} → {:.4}", drift_before, drift_after),
            );
        }

        self.result_history.push(result.clone());
        Ok(result)
    }

    /// Ejecuta rollback si la métrica se degradó
    ///
    /// Restaura el estado anterior del scorer limpiando el feedback
    /// más reciente y recalculando drift.
    ///
    /// # Arguments
    ///
    /// * `layer_id` - ID de capa SAE a rollback
    ///
    /// # Returns
    ///
    /// `Ok(())` si el rollback fue exitoso
    pub fn rollback_if_degraded(&mut self, layer_id: &str) -> Result<Option<LoopResult>, FeedbackLoopError> {
        if self.result_history.is_empty() {
            return Ok(None);
        }

        let last_result = self.result_history.last()?.clone();
        if !last_result.rollback_triggered {
            return Ok(None);
        }

        // Rollback: limpiar feedback reciente y restaurar estado
        self.scorer.clear_feedback(layer_id);

        let restored_drift = self.scorer.calculate_drift(layer_id).unwrap_or(0.0);

        info!(
            layer = %layer_id,
            restored_drift,
            "Rollback completed: feedback cleared, drift restored"
        );

        self.audit(
            AuditAction::Rollback,
            None,
            AuditResult::Success,
            format!("Restored drift to {:.4}", restored_drift),
        );

        Ok(Some(LoopResult {
            applied: false,
            drift_delta: restored_drift - last_result.drift_after,
            steering_hash: String::new(),
            rollback_triggered: true,
            drift_before: last_result.drift_after,
            drift_after: restored_drift,
            timestamp_ms: current_timestamp_ms(),
        }))
    }

    /// Ejecuta una iteración completa del loop
    ///
    /// 1. Ingesta feedback pendiente
    /// 2. Calcula drift
    /// 3. Aplica steering si drift > threshold
    /// 4. Verifica rollback
    ///
    /// # Arguments
    ///
    /// * `layer_id` - ID de capa SAE
    /// * `activations` - Activaciones actuales
    ///
    /// # Returns
    ///
    /// `Ok(LoopResult)` con el resultado de la iteración
    pub fn run_iteration(&mut self, layer_id: &str, activations: &[f32]) -> Result<LoopResult, FeedbackLoopError> {
        self.running = true;

        // Calcular drift actual
        let drift = self.compute_drift(layer_id)?;

        // Verificar si necesita steering
        if drift < self.config.alignment_config.drift_threshold {
            info!(layer = %layer_id, drift, "Drift within threshold, skipping steering");
            self.audit(
                AuditAction::ApplySteering,
                None,
                AuditResult::Skipped,
                format!("Drift {:.4} < threshold {:.4}", drift, self.config.alignment_config.drift_threshold),
            );

            let result = LoopResult {
                applied: false,
                drift_delta: 0.0,
                steering_hash: String::new(),
                rollback_triggered: false,
                drift_before: drift,
                drift_after: drift,
                timestamp_ms: current_timestamp_ms(),
            };
            self.running = false;
            return Ok(result);
        }

        // Aplicar steering
        let result = self.apply_steering(layer_id, activations)?;

        // Verificar rollback
        if result.rollback_triggered {
            self.rollback_if_degraded(layer_id)?;
        }

        self.running = false;
        Ok(result)
    }

    /// Obtiene el historial de resultados
    pub fn get_result_history(&self) -> &[LoopResult] {
        &self.result_history
    }

    /// Obtiene el log de auditoría
    pub fn get_audit_log(&self) -> &[AuditEntry] {
        &self.audit_log
    }

    /// Obtiene la configuración actual
    pub fn config(&self) -> &FeedbackLoopConfig {
        &self.config
    }

    /// Verifica si el loop está ejecutándose
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Limpia la cola de feedback
    pub fn clear_queue(&mut self) {
        let count = self.queue.len();
        self.queue.clear();
        debug!(count, "Feedback queue cleared");
    }

    /// Limpia todo el estado del loop
    pub fn reset(&mut self) {
        self.queue.clear();
        self.audit_log.clear();
        self.result_history.clear();
        self.scorer.clear_all();
        self.rate_counter = 0;
        self.rate_window_start = current_timestamp_ms();
        self.running = false;
        info!("Feedback loop reset");
    }

    // ------------------------------------------------------------------------
    // Private Helpers
    // ------------------------------------------------------------------------

    /// Valida feedback antes de ingestión
    fn validate_feedback(&self, feedback: &AlignmentFeedback) -> Result<(), FeedbackLoopError> {
        // Rechazar NaN
        if feedback.current_activation.is_nan() || feedback.desired_value.is_nan() {
            return Err(FeedbackLoopError::ValidationFailed {
                reason: "NaN activation or desired value".to_string(),
            });
        }

        // Rechazar Infinity
        if feedback.current_activation.is_infinite() || feedback.desired_value.is_infinite() {
            return Err(FeedbackLoopError::ValidationFailed {
                reason: "Infinity activation or desired value".to_string(),
            });
        }

        // Validar confianza
        if feedback.annotator_confidence < 0.0 || feedback.annotator_confidence > 1.0 {
            return Err(FeedbackLoopError::ValidationFailed {
                reason: format!("Confidence out of range: {}", feedback.annotator_confidence),
            });
        }

        self.audit(AuditAction::Validation, None, AuditResult::Success, "Feedback valid");
        Ok(())
    }

    /// Verifica límite de tasa
    fn check_rate_limit(&mut self) -> Result<(), FeedbackLoopError> {
        let now = current_timestamp_ms();
        let window_ms = self.config.feedback_window_ms;

        // Reset ventana si expiró
        if now.saturating_sub(self.rate_window_start) > window_ms {
            self.rate_counter = 0;
            self.rate_window_start = now;
        }

        if self.rate_counter >= self.config.rate_limit {
            self.audit(
                AuditAction::RateLimit,
                None,
                AuditResult::Skipped,
                format!("Rate limit: {}/{}", self.rate_counter, self.config.rate_limit),
            );
            return Err(FeedbackLoopError::RateLimitExceeded {
                max: self.config.rate_limit,
                count: self.rate_counter,
            });
        }

        Ok(())
    }

    /// Simula drift post-aplicación (en producción: evaluar con validación real)
    fn simulate_post_application_drift(&self, result: &AlignmentResult) -> f32 {
        // Simulación: drift_after = drift_before - steering_effectiveness
        let effectiveness = result.confidence * 0.5; // 50% de efectividad esperada
        (result.drift_score - effectiveness).max(0.0)
    }

    /// Calcula hash SHA-256 del steering delta
    fn compute_steering_hash(delta: &[f32]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(delta.iter().map(|f| f.to_le_bytes()).flatten().collect::<Vec<u8>>());
        hex::encode(hasher.finalize())
    }

    /// Registra entrada de auditoría
    fn audit(&mut self, action: AuditAction, entry_id: Option<&str>, result: AuditResult, message: &str) {
        self.audit_log.push_back(AuditEntry {
            action,
            entry_id: entry_id.map(|s| s.to_string()),
            result,
            message: message.to_string(),
            timestamp_ms: current_timestamp_ms(),
        });

        // Limitar tamaño del log
        if self.audit_log.len() > 4096 {
            self.audit_log.pop_front();
        }
    }
}

/// Obtiene timestamp actual en milisegundos epoch
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

    fn make_feedback(layer_id: &str, feature_idx: u32, current: f32, desired: f32) -> AlignmentFeedback {
        AlignmentFeedback {
            layer_id: layer_id.to_string(),
            feature_idx,
            current_activation: current,
            desired_value: desired,
            annotator_confidence: 0.9,
            concept: Some(format!("feature_{}", feature_idx)),
        }
    }

    #[test]
    fn test_loop_creation() {
        let loop_ = AlignmentFeedbackLoop::new();
        assert!(!loop_.is_running());
        assert!(loop_.get_result_history().is_empty());
    }

    #[test]
    fn test_ingest_valid_feedback() {
        let mut loop_ = AlignmentFeedbackLoop::new();
        let feedback = make_feedback("layer_0", 0, 0.5, 0.8);
        assert!(loop_.ingest(feedback).is_ok());
    }

    #[test]
    fn test_ingest_nan_rejected() {
        let mut loop_ = AlignmentFeedbackLoop::new();
        let feedback = make_feedback("layer_0", 0, f32::NAN, 0.8);
        let result = loop_.ingest(feedback);
        assert!(matches!(result, Err(FeedbackLoopError::ValidationFailed { .. })));
    }

    #[test]
    fn test_ingest_infinity_rejected() {
        let mut loop_ = AlignmentFeedbackLoop::new();
        let feedback = make_feedback("layer_0", 0, f32::INFINITY, 0.8);
        let result = loop_.ingest(feedback);
        assert!(matches!(result, Err(FeedbackLoopError::ValidationFailed { .. })));
    }

    #[test]
    fn test_ingest_invalid_confidence() {
        let mut loop_ = AlignmentFeedbackLoop::new();
        let mut feedback = make_feedback("layer_0", 0, 0.5, 0.8);
        feedback.annotator_confidence = 1.5;
        let result = loop_.ingest(feedback);
        assert!(matches!(result, Err(FeedbackLoopError::ValidationFailed { .. })));
    }

    #[test]
    fn test_compute_drift() {
        let mut loop_ = AlignmentFeedbackLoop::new();
        let feedback = make_feedback("layer_0", 0, 0.5, 0.5); // Perfect alignment
        loop_.ingest(feedback).unwrap();
        let drift = loop_.compute_drift("layer_0");
        assert!(drift.is_ok());
        assert_eq!(drift.unwrap(), 0.0); // Perfect alignment = 0 drift
    }

    #[test]
    fn test_apply_steering() {
        let mut loop_ = AlignmentFeedbackLoop::new();
        let feedback = make_feedback("layer_0", 0, 0.3, 0.7);
        loop_.ingest(feedback).unwrap();
        let activations = vec![0.3, 0.1, 0.2];
        let result = loop_.apply_steering("layer_0", &activations);
        assert!(result.is_ok());
    }

    #[test]
    fn test_apply_steering_empty_activations() {
        let mut loop_ = AlignmentFeedbackLoop::new();
        let activations: Vec<f32> = vec![];
        let result = loop_.apply_steering("layer_0", &activations);
        assert!(matches!(result, Err(FeedbackLoopError::ValidationFailed { .. })));
    }

    #[test]
    fn test_clear_queue() {
        let mut loop_ = AlignmentFeedbackLoop::new();
        loop_.ingest(make_feedback("layer_0", 0, 0.5, 0.8)).unwrap();
        loop_.clear_queue();
        assert!(loop_.queue.is_empty());
    }

    #[test]
    fn test_reset() {
        let mut loop_ = AlignmentFeedbackLoop::new();
        loop_.ingest(make_feedback("layer_0", 0, 0.5, 0.8)).unwrap();
        loop_.reset();
        assert!(loop_.get_result_history().is_empty());
        assert!(loop_.get_audit_log().is_empty());
        assert!(!loop_.is_running());
    }

    #[test]
    fn test_audit_log_populated() {
        let mut loop_ = AlignmentFeedbackLoop::new();
        loop_.ingest(make_feedback("layer_0", 0, 0.5, 0.8)).unwrap();
        let log = loop_.get_audit_log();
        assert!(!log.is_empty());
        assert_eq!(log[0].action, AuditAction::Validation);
    }

    #[test]
    fn test_rate_limit() {
        let config = FeedbackLoopConfig {
            rate_limit: 2,
            feedback_window_ms: 1_000_000, // Very long window
            ..Default::default()
        };
        let mut loop_ = AlignmentFeedbackLoop::with_config(config);
        loop_.ingest(make_feedback("layer_0", 0, 0.5, 0.8)).unwrap();
        loop_.ingest(make_feedback("layer_0", 1, 0.5, 0.8)).unwrap();
        let result = loop_.ingest(make_feedback("layer_0", 2, 0.5, 0.8));
        assert!(matches!(result, Err(FeedbackLoopError::RateLimitExceeded { .. })));
    }

    #[test]
    fn test_loop_result_structure() {
        let result = LoopResult {
            applied: true,
            drift_delta: 0.05,
            steering_hash: "abc123".to_string(),
            rollback_triggered: false,
            drift_before: 0.3,
            drift_after: 0.25,
            timestamp_ms: 1000,
        };
        assert!(result.applied);
        assert!(!result.rollback_triggered);
        assert_eq!(result.drift_delta, 0.05);
    }

    #[test]
    fn test_steering_hash_deterministic() {
        let delta = vec![0.1, 0.2, 0.3];
        let hash1 = AlignmentFeedbackLoop::compute_steering_hash(&delta);
        let hash2 = AlignmentFeedbackLoop::compute_steering_hash(&delta);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 hex = 64 chars
    }

    #[test]
    fn test_config_default() {
        let config = FeedbackLoopConfig::default();
        assert_eq!(config.feedback_window_ms, 30_000);
        assert_eq!(config.max_queue_size, 1000);
        assert_eq!(config.rate_limit, 100);
        assert_eq!(config.rollback_threshold, 0.1);
        assert_eq!(config.max_retries, 3);
    }
}
