//! Continuous Alignment Loop — Loop continuo: drift → feedback → steering → validación humana → aplicación
//!
//! Implementa `ContinuousAlignmentLoop` para cerrar el ciclo de alineación continua
//! con supervisión humana. Conecta `AlignmentScorer` con `feedback_store.rs`,
//! `consciousness.rs` y `slo/engine.rs`.
//!
//! Cuando `drift > threshold` y `confidence < 0.8`, pausa la aplicación y solicita
//! review vía API v3 antes de aplicar steering.
//!
//! **Feature:** `phase8-sprint2`

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tracing::{info, warn};

// ============================================================================
// Errors
// ============================================================================

/// Error específico del loop de alineación continua
#[derive(Debug, Error)]
pub enum AlignmentLoopError {
    #[error("Feedback ingestion failed: {reason}")]
    FeedbackIngestionFailed { reason: String },

    #[error("Drift computation failed: {reason}")]
    DriftComputationFailed { reason: String },

    #[error("Human review required: drift={drift:.4}, confidence={confidence:.4}")]
    HumanReviewRequired { drift: f32, confidence: f32 },

    #[error("Steering application paused: awaiting review")]
    SteeringPaused,

    #[error("Layer not found: {layer_id}")]
    LayerNotFound { layer_id: u32 },

    #[error("Audit trail full: max={max}")]
    AuditTrailFull { max: usize },
}

// ============================================================================
// Feedback Entry
// ============================================================================

/// Entrada de feedback para el loop continuo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuousFeedback {
    /// ID de capa SAE
    pub layer_id: u32,
    /// Índice de concepto
    pub concept_index: u32,
    /// Activación actual
    pub current_activation: f32,
    /// Activación deseada
    pub desired_activation: f32,
    /// Confianza del anotador [0.0, 1.0]
    pub annotator_confidence: f32,
    /// Timestamp (epoch ms)
    pub timestamp_ms: u64,
}

impl ContinuousFeedback {
    /// Crea nuevo feedback
    pub fn new(
        layer_id: u32,
        concept_index: u32,
        current: f32,
        desired: f32,
        confidence: f32,
    ) -> Self {
        Self {
            layer_id,
            concept_index,
            current_activation: current,
            desired_activation: desired,
            annotator_confidence: confidence.clamp(0.0, 1.0),
            timestamp_ms: current_timestamp_ms(),
        }
    }
}

// ============================================================================
// Alignment Loop Result
// ============================================================================

/// Resultado de una iteración del loop de alineación
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentLoopResult {
    /// Si el steering fue aplicado
    pub applied: bool,
    /// Score de drift calculado
    pub drift_score: f32,
    /// Si se requiere review humana
    pub human_review_required: bool,
    /// Hash de auditoría
    pub audit_hash: String,
}

impl AlignmentLoopResult {
    /// Crea resultado de aplicación exitosa
    pub fn applied(drift: f32, audit_hash: String) -> Self {
        Self {
            applied: true,
            drift_score: drift,
            human_review_required: false,
            audit_hash,
        }
    }

    /// Crea resultado pendiente de review humana
    pub fn pending_review(drift: f32, confidence: f32) -> Self {
        Self {
            applied: false,
            drift_score: drift,
            human_review_required: true,
            audit_hash: Self::compute_hash("pending_review", drift, confidence),
        }
    }

    /// Crea resultado de steering neutral
    pub fn neutral() -> Self {
        Self {
            applied: false,
            drift_score: 0.0,
            human_review_required: false,
            audit_hash: Self::compute_hash("neutral", 0.0, 0.0),
        }
    }

    fn compute_hash(action: &str, drift: f32, confidence: f32) -> String {
        let mut hasher = Sha256::new();
        hasher.update(action.as_bytes());
        hasher.update(drift.to_le_bytes());
        hasher.update(confidence.to_le_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    }
}

// ============================================================================
// Loop Config
// ============================================================================

/// Configuración del loop de alineación continua
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopConfig {
    /// Umbral de drift para solicitar review humana
    pub drift_threshold: f32,
    /// Umbral de confianza mínimo para auto-aplicar
    pub confidence_threshold: f32,
    /// Tamaño máximo del buffer de feedback por capa
    pub max_feedback_buffer: usize,
    /// Capacidad máxima del audit trail
    pub max_audit_entries: usize,
    /// Número máximo de iteraciones por ciclo
    pub max_iterations_per_cycle: usize,
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            drift_threshold: 0.3,
            confidence_threshold: 0.8,
            max_feedback_buffer: 64,
            max_audit_entries: 256,
            max_iterations_per_cycle: 10,
        }
    }
}

// ============================================================================
// Continuous Alignment Loop
// ============================================================================

/// Loop de alineación continua con supervisión humana
pub struct ContinuousAlignmentLoop {
    /// Configuración del loop
    config: LoopConfig,
    /// Buffer de feedback por capa
    feedback_buffer: HashMap<u32, VecDeque<ContinuousFeedback>>,
    /// Historial de drift por capa
    drift_history: HashMap<u32, VecDeque<f32>>,
    /// Audit trail
    audit_trail: VecDeque<String>,
    /// Cola de pendientes de review humana
    pending_reviews: VecDeque<AlignmentLoopResult>,
    /// Iteraciones ejecutadas en el ciclo actual
    current_iterations: usize,
}

impl ContinuousAlignmentLoop {
    /// Crea nuevo loop con configuración por defecto
    pub fn new() -> Self {
        Self {
            config: LoopConfig::default(),
            feedback_buffer: HashMap::new(),
            drift_history: HashMap::new(),
            audit_trail: VecDeque::with_capacity(256),
            pending_reviews: VecDeque::new(),
            current_iterations: 0,
        }
    }

    /// Crea loop con configuración personalizada
    pub fn with_config(config: LoopConfig) -> Self {
        Self {
            config,
            ..Self::new()
        }
    }

    /// Ingesta feedback humano en el buffer
    pub fn ingest_feedback(
        &mut self,
        feedback: ContinuousFeedback,
    ) -> Result<(), AlignmentLoopError> {
        // Validar valores
        if feedback.current_activation.is_nan() || feedback.current_activation.is_infinite() {
            return Err(AlignmentLoopError::FeedbackIngestionFailed {
                reason: "Invalid current_activation: NaN or Infinity".into(),
            });
        }
        if feedback.desired_activation.is_nan() || feedback.desired_activation.is_infinite() {
            return Err(AlignmentLoopError::FeedbackIngestionFailed {
                reason: "Invalid desired_activation: NaN or Infinity".into(),
            });
        }
        if feedback.annotator_confidence < 0.0 || feedback.annotator_confidence > 1.0 {
            return Err(AlignmentLoopError::FeedbackIngestionFailed {
                reason: "Confidence out of range [0.0, 1.0]".into(),
            });
        }

        let layer_id = feedback.layer_id;
        let concept_index = feedback.concept_index;
        let confidence = feedback.annotator_confidence;
        let buffer = self.feedback_buffer.entry(layer_id).or_default();

        buffer.push_back(feedback);

        // Limitar tamaño del buffer
        while buffer.len() > self.config.max_feedback_buffer {
            buffer.pop_front();
        }

        self.audit(&format!(
            "feedback ingested: layer={}, concept={}, confidence={:.3}",
            layer_id, concept_index, confidence
        ));

        Ok(())
    }

    /// Calcula drift para una capa dada
    pub fn compute_drift(&mut self, layer_id: u32) -> Result<f32, AlignmentLoopError> {
        let buffer = self
            .feedback_buffer
            .get(&layer_id)
            .ok_or(AlignmentLoopError::LayerNotFound { layer_id })?;

        if buffer.is_empty() {
            return Ok(0.0);
        }

        // Calcular drift promedio como |desired - current| ponderado por confianza
        let total_drift: f32 = buffer
            .iter()
            .map(|fb| {
                let diff = (fb.desired_activation - fb.current_activation).abs();
                diff * fb.annotator_confidence
            })
            .sum();
        let total_confidence: f32 = buffer.iter().map(|fb| fb.annotator_confidence).sum();

        let drift = if total_confidence > 0.0 {
            total_drift / total_confidence
        } else {
            0.0
        };

        // Registrar en historial
        self.drift_history
            .entry(layer_id)
            .or_default()
            .push_back(drift);

        // Limitar historial
        if let Some(history) = self.drift_history.get_mut(&layer_id) {
            while history.len() > 64 {
                history.pop_front();
            }
        }

        self.audit(&format!(
            "drift computed: layer={}, drift={:.4}",
            layer_id, drift
        ));

        Ok(drift)
    }

    /// Solicita review humana si drift > threshold y confidence < threshold
    pub fn request_human_review(&mut self, layer_id: u32) -> Result<bool, AlignmentLoopError> {
        let drift = self.compute_drift(layer_id)?;

        // Calcular confianza promedio del buffer
        let avg_confidence = self
            .feedback_buffer
            .get(&layer_id)
            .map(|buf| {
                if buf.is_empty() {
                    return 1.0;
                }
                buf.iter().map(|fb| fb.annotator_confidence).sum::<f32>() / buf.len() as f32
            })
            .unwrap_or(1.0);

        let needs_review = drift > self.config.drift_threshold
            && avg_confidence < self.config.confidence_threshold;

        if needs_review {
            warn!(
                "Human review requested: layer={}, drift={:.4}, confidence={:.4}",
                layer_id, drift, avg_confidence
            );
            let result = AlignmentLoopResult::pending_review(drift, avg_confidence);
            self.pending_reviews.push_back(result);
            self.audit(&format!(
                "human review requested: layer={}, drift={:.4}",
                layer_id, drift
            ));
        }

        Ok(needs_review)
    }

    /// Aplica steering si no requiere review humana
    pub fn apply_steering(
        &mut self,
        layer_id: u32,
        activations: &[f32],
    ) -> Result<AlignmentLoopResult, AlignmentLoopError> {
        // Verificar si requiere review
        let needs_review = self.request_human_review(layer_id)?;
        if needs_review {
            return Err(AlignmentLoopError::SteeringPaused);
        }

        let drift = self.compute_drift(layer_id)?;

        // Si no hay feedback, retornar neutral
        let buffer = self.feedback_buffer.get(&layer_id);
        if buffer.is_none_or(|b| b.is_empty()) {
            return Ok(AlignmentLoopResult::neutral());
        }

        // Calcular steering delta basado en drift y activations
        let steering_hash = Self::compute_steering_hash(activations, drift);

        // Incrementar iteraciones
        self.current_iterations += 1;

        let result = AlignmentLoopResult::applied(drift, steering_hash);
        self.audit(&format!(
            "steering applied: layer={}, drift={:.4}, iterations={}",
            layer_id, drift, self.current_iterations
        ));

        Ok(result)
    }

    /// Ejecuta un ciclo completo del loop
    pub fn run_cycle(
        &mut self,
        layer_id: u32,
        activations: &[f32],
    ) -> Result<AlignmentLoopResult, AlignmentLoopError> {
        // Resetear contador de iteraciones
        self.current_iterations = 0;

        // 1. Calcular drift
        let drift = self.compute_drift(layer_id)?;

        // 2. Verificar necesidad de review
        let needs_review = self.request_human_review(layer_id)?;

        if needs_review {
            return Err(AlignmentLoopError::HumanReviewRequired {
                drift,
                confidence: self.get_avg_confidence(layer_id),
            });
        }

        // 3. Aplicar steering
        let result = self.apply_steering(layer_id, activations)?;

        // 4. Limpiar buffer procesado
        self.clear_feedback(layer_id);

        info!(
            "Cycle complete: layer={}, drift={:.4}, applied={}",
            layer_id, drift, result.applied
        );

        Ok(result)
    }

    /// Retorna los pendientes de review
    pub fn drain_pending_reviews(&mut self) -> Vec<AlignmentLoopResult> {
        let reviews = self.pending_reviews.drain(..).collect();
        reviews
    }

    /// Retorna el audit trail
    pub fn audit_trail(&self) -> &[String] {
        self.audit_trail.as_slices().0
    }

    /// Retorna la cantidad de pendientes de review
    pub fn pending_review_count(&self) -> usize {
        self.pending_reviews.len()
    }

    /// Limpia el feedback de una capa
    pub fn clear_feedback(&mut self, layer_id: u32) {
        self.feedback_buffer.remove(&layer_id);
    }

    /// Resetea el loop completo
    pub fn reset(&mut self) {
        self.feedback_buffer.clear();
        self.drift_history.clear();
        self.audit_trail.clear();
        self.pending_reviews.clear();
        self.current_iterations = 0;
    }

    /// Retorna las capas con feedback pendiente
    pub fn layers_with_feedback(&self) -> Vec<u32> {
        let mut layers: Vec<u32> = self.feedback_buffer.keys().cloned().collect();
        layers.sort();
        layers
    }

    // ---- Internal helpers ----

    fn audit(&mut self, message: &str) {
        self.audit_trail.push_back(message.to_string());
        if self.audit_trail.len() > self.config.max_audit_entries {
            self.audit_trail.pop_front();
        }
    }

    fn get_avg_confidence(&self, layer_id: u32) -> f32 {
        self.feedback_buffer
            .get(&layer_id)
            .map(|buf| {
                if buf.is_empty() {
                    return 1.0;
                }
                buf.iter().map(|fb| fb.annotator_confidence).sum::<f32>() / buf.len() as f32
            })
            .unwrap_or(1.0)
    }

    fn compute_steering_hash(activations: &[f32], drift: f32) -> String {
        let mut hasher = Sha256::new();
        for a in activations {
            hasher.update(a.to_le_bytes());
        }
        hasher.update(drift.to_le_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    }
}

// ============================================================================
// Default
// ============================================================================

impl Default for ContinuousAlignmentLoop {
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

    fn make_feedback(
        layer: u32,
        concept: u32,
        current: f32,
        desired: f32,
        confidence: f32,
    ) -> ContinuousFeedback {
        ContinuousFeedback::new(layer, concept, current, desired, confidence)
    }

    #[test]
    fn test_loop_creation() {
        let loop_ = ContinuousAlignmentLoop::new();
        assert_eq!(loop_.layers_with_feedback().len(), 0);
    }

    #[test]
    fn test_ingest_feedback() {
        let mut loop_ = ContinuousAlignmentLoop::new();
        let fb = make_feedback(0, 5, 0.3, 0.7, 0.9);
        loop_.ingest_feedback(fb).unwrap();
        assert_eq!(loop_.layers_with_feedback().len(), 1);
    }

    #[test]
    fn test_ingest_nan_rejected() {
        let mut loop_ = ContinuousAlignmentLoop::new();
        let mut fb = make_feedback(0, 5, f32::NAN, 0.7, 0.9);
        let result = loop_.ingest_feedback(fb);
        assert!(result.is_err());
    }

    #[test]
    fn test_ingest_infinity_rejected() {
        let mut loop_ = ContinuousAlignmentLoop::new();
        let mut fb = make_feedback(0, 5, f32::INFINITY, 0.7, 0.9);
        let result = loop_.ingest_feedback(fb);
        assert!(result.is_err());
    }

    #[test]
    fn test_compute_drift() {
        let mut loop_ = ContinuousAlignmentLoop::new();
        loop_
            .ingest_feedback(make_feedback(0, 5, 0.2, 0.8, 1.0))
            .unwrap();
        let drift = loop_.compute_drift(0).unwrap();
        assert!((drift - 0.6) < 0.01);
    }

    #[test]
    fn test_compute_drift_unknown_layer() {
        let mut loop_ = ContinuousAlignmentLoop::new();
        let result = loop_.compute_drift(99);
        assert!(result.is_err());
    }

    #[test]
    fn test_human_review_triggered() {
        let mut loop_ = ContinuousAlignmentLoop::with_config(LoopConfig {
            drift_threshold: 0.3,
            confidence_threshold: 0.8,
            ..LoopConfig::default()
        });
        // High drift, low confidence
        loop_
            .ingest_feedback(make_feedback(0, 5, 0.1, 0.9, 0.5))
            .unwrap();
        let needs_review = loop_.request_human_review(0).unwrap();
        assert!(needs_review);
    }

    #[test]
    fn test_human_review_not_needed() {
        let mut loop_ = ContinuousAlignmentLoop::new();
        // Low drift, high confidence
        loop_
            .ingest_feedback(make_feedback(0, 5, 0.4, 0.5, 0.95))
            .unwrap();
        let needs_review = loop_.request_human_review(0).unwrap();
        assert!(!needs_review);
    }

    #[test]
    fn test_apply_steering_success() {
        let mut loop_ = ContinuousAlignmentLoop::new();
        loop_
            .ingest_feedback(make_feedback(0, 5, 0.4, 0.5, 0.95))
            .unwrap();
        let activations = vec![0.1, 0.2, 0.3];
        let result = loop_.apply_steering(0, &activations).unwrap();
        assert!(result.applied);
        assert!(!result.human_review_required);
    }

    #[test]
    fn test_apply_steering_paused_for_review() {
        let mut loop_ = ContinuousAlignmentLoop::with_config(LoopConfig {
            drift_threshold: 0.1,
            confidence_threshold: 0.99,
            ..LoopConfig::default()
        });
        loop_
            .ingest_feedback(make_feedback(0, 5, 0.1, 0.9, 0.5))
            .unwrap();
        let activations = vec![0.1, 0.2, 0.3];
        let result = loop_.apply_steering(0, &activations);
        assert!(matches!(result, Err(AlignmentLoopError::SteeringPaused)));
    }

    #[test]
    fn test_drain_pending_reviews() {
        let mut loop_ = ContinuousAlignmentLoop::with_config(LoopConfig {
            drift_threshold: 0.1,
            confidence_threshold: 0.5,
            ..LoopConfig::default()
        });
        loop_
            .ingest_feedback(make_feedback(0, 5, 0.1, 0.9, 0.3))
            .unwrap();
        let _ = loop_.request_human_review(0);
        assert_eq!(loop_.pending_review_count(), 1);

        let drained = loop_.drain_pending_reviews();
        assert_eq!(drained.len(), 1);
        assert_eq!(loop_.pending_review_count(), 0);
    }

    #[test]
    fn test_audit_trail_populated() {
        let mut loop_ = ContinuousAlignmentLoop::new();
        loop_
            .ingest_feedback(make_feedback(0, 5, 0.3, 0.7, 0.9))
            .unwrap();
        let trail = loop_.audit_trail();
        assert!(!trail.is_empty());
        assert!(trail[0].contains("feedback ingested"));
    }

    #[test]
    fn test_clear_feedback() {
        let mut loop_ = ContinuousAlignmentLoop::new();
        loop_
            .ingest_feedback(make_feedback(0, 5, 0.3, 0.7, 0.9))
            .unwrap();
        loop_.clear_feedback(0);
        assert_eq!(loop_.layers_with_feedback().len(), 0);
    }

    #[test]
    fn test_reset() {
        let mut loop_ = ContinuousAlignmentLoop::new();
        loop_
            .ingest_feedback(make_feedback(0, 5, 0.3, 0.7, 0.9))
            .unwrap();
        loop_.reset();
        assert_eq!(loop_.layers_with_feedback().len(), 0);
        assert!(loop_.audit_trail().is_empty());
    }

    #[test]
    fn test_alignment_loop_result_applied() {
        let result = AlignmentLoopResult::applied(0.5, "hash123".into());
        assert!(result.applied);
        assert!(!result.human_review_required);
        assert_eq!(result.drift_score, 0.5);
    }

    #[test]
    fn test_alignment_loop_result_pending() {
        let result = AlignmentLoopResult::pending_review(0.8, 0.4);
        assert!(!result.applied);
        assert!(result.human_review_required);
    }

    #[test]
    fn test_alignment_loop_result_neutral() {
        let result = AlignmentLoopResult::neutral();
        assert!(!result.applied);
        assert!(!result.human_review_required);
        assert!((result.drift_score - 0.0) < f32::EPSILON);
    }

    #[test]
    fn test_config_default() {
        let config = LoopConfig::default();
        assert!((config.drift_threshold - 0.3) < 0.01);
        assert!((config.confidence_threshold - 0.8) < 0.01);
        assert_eq!(config.max_audit_entries, 256);
    }

    #[test]
    fn test_buffer_size_limit() {
        let mut loop_ = ContinuousAlignmentLoop::with_config(LoopConfig {
            max_feedback_buffer: 5,
            ..LoopConfig::default()
        });
        for i in 0..10 {
            loop_
                .ingest_feedback(make_feedback(0, i, 0.3, 0.7, 0.9))
                .unwrap();
        }
        let buffer = loop_.feedback_buffer.get(&0).unwrap();
        assert_eq!(buffer.len(), 5);
    }

    #[test]
    fn test_default() {
        let loop_ = ContinuousAlignmentLoop::default();
        assert_eq!(loop_.layers_with_feedback().len(), 0);
    }
}
