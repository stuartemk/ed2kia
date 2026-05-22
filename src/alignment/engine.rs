//! Alignment Engine - Motor de Alineación Continua para Fase 7 Sprint 1
//!
//! Implementa `AlignmentScorer` para detección de deriva semántica,
//! integración de feedback humano y generación de ajustes de steering.
//!
//! Este módulo calcula la divergencia entre activaciones SAE y etiquetas
//! humanas proporcionadas por `feedback_store.rs`, generando deltas de
//! steering que mantienen la red alineada con la intención humana.

use candle_core::{Device, Tensor};
// CLEANUP: removed unused import Result as CResult
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tracing::{debug, info, warn};

/// Error específico del motor de alineación
#[derive(Debug, Error)]
pub enum AlignmentError {
    #[error(
        "Drift threshold exceeded: {reason} (threshold={drift_threshold:.4}, current={current:.4})"
    )]
    DriftThresholdExceeded {
        reason: String,
        drift_threshold: f32,
        current: f32,
    },

    // FIX: E0599 - Vec<usize> doesn't implement Display, format as joined string
    #[error("Tensor shape mismatch: expected [{expected}], got [{actual}]")]
    TensorShapeMismatch { expected: String, actual: String },

    #[error("Invalid feedback data: {reason}")]
    InvalidFeedback { reason: String },

    #[error("Device error: {0}")]
    Device(#[from] candle_core::Error),

    #[error("Empty activations provided for drift calculation")]
    EmptyActivations,

    #[error("Feedback store returned no entries for layer {layer_id}")]
    NoFeedbackForLayer { layer_id: String },
}

/// Resultado de evaluación de alineación
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentResult {
    /// Puntuación de deriva: 0.0 = perfectamente alineado, 1.0 = máxima deriva
    pub drift_score: f32,
    /// Conceptos que excedieron el umbral de deriva
    pub flagged_concepts: Vec<String>,
    /// Delta de steering para corrección (tensor plano serializado)
    pub steering_delta: Vec<f32>,
    /// Confianza del resultado (0.0 - 1.0)
    pub confidence: f32,
    /// Número de features analizadas
    pub features_analyzed: usize,
    /// Timestamp de la evaluación (epoch ms)
    pub timestamp_ms: u64,
}

/// Entrada de feedback para el motor de alineación
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentFeedback {
    /// ID de capa SAE
    pub layer_id: String,
    /// Índice de feature
    pub feature_idx: u32,
    /// Activación actual del modelo
    pub current_activation: f32,
    /// Valor deseado (etiqueta humana)
    pub desired_value: f32,
    /// Confianza del anotador (0.0 - 1.0)
    pub annotator_confidence: f32,
    /// Concepto asociado (opcional)
    pub concept: Option<String>,
}

/// Configuración del AlignmentScorer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentConfig {
    /// Umbral de deriva para alertas (0.0 - 1.0)
    pub drift_threshold: f32,
    /// Umbral crítico para acción inmediata
    pub critical_threshold: f32,
    /// Peso del feedback en el cálculo de steering (0.0 - 1.0)
    pub feedback_weight: f32,
    /// Tasa de aprendizaje para ajustes de steering
    pub learning_rate: f32,
    /// Número máximo de features por evaluación
    pub max_features: usize,
    /// Ventana de feedback (entries más recientes por capa)
    pub feedback_window: usize,
}

impl Default for AlignmentConfig {
    fn default() -> Self {
        Self {
            drift_threshold: 0.3,
            critical_threshold: 0.7,
            feedback_weight: 0.6,
            learning_rate: 0.001,
            max_features: 4096,
            feedback_window: 100,
        }
    }
}

/// Motor de evaluación de alineación continua
pub struct AlignmentScorer {
    config: AlignmentConfig,
    /// Buffer de feedback por capa: layer_id -> Vec<AlignmentFeedback>
    feedback_buffer: HashMap<String, Vec<AlignmentFeedback>>,
    /// Historial de scores de deriva por capa
    drift_history: HashMap<String, Vec<f32>>,
    device: Device,
}

impl AlignmentScorer {
    /// Crea una nueva instancia de AlignmentScorer
    pub fn new(config: AlignmentConfig) -> Self {
        Self {
            config,
            feedback_buffer: HashMap::new(),
            drift_history: HashMap::new(),
            device: Device::Cpu,
        }
    }

    /// Crea con dispositivo específico
    pub fn with_device(config: AlignmentConfig, device: Device) -> Self {
        Self {
            device,
            ..Self::new(config)
        }
    }

    /// Ingesta feedback de alineación
    ///
    /// Valida los datos de feedback y los almacena en el buffer
    /// para la capa SAE especificada.
    pub fn ingest_feedback(&mut self, feedback: AlignmentFeedback) -> Result<(), AlignmentError> {
        // Validar rangos
        if feedback.current_activation.is_nan() || feedback.current_activation.is_infinite() {
            return Err(AlignmentError::InvalidFeedback {
                reason: format!(
                    "Invalid current_activation for feature {}: {}",
                    feedback.feature_idx, feedback.current_activation
                ),
            });
        }

        if feedback.desired_value.is_nan() || feedback.desired_value.is_infinite() {
            return Err(AlignmentError::InvalidFeedback {
                reason: format!(
                    "Invalid desired_value for feature {}: {}",
                    feedback.feature_idx, feedback.desired_value
                ),
            });
        }

        if feedback.annotator_confidence < 0.0 || feedback.annotator_confidence > 1.0 {
            return Err(AlignmentError::InvalidFeedback {
                reason: format!(
                    "annotator_confidence out of range [0,1]: {}",
                    feedback.annotator_confidence
                ),
            });
        }

        let layer_id = feedback.layer_id.clone();

        // Almacenar en buffer con límite
        let buffer = self.feedback_buffer.entry(layer_id.clone()).or_default(); // CLEANUP: or_insert_with -> or_default

        if buffer.len() >= self.config.feedback_window {
            buffer.remove(0); // FIFO eviction
        }

        // FIX: borrow/move - Extract feature_idx before pushing feedback to avoid borrow-after-move | borrow/move
        let feature_idx = feedback.feature_idx;
        buffer.push(feedback);

        debug!(
            layer = %layer_id,
            feature_idx = feature_idx,
            buffer_size = buffer.len(),
            "Feedback ingested"
        );

        Ok(())
    }

    /// Calcula la puntuación de deriva semántica para una capa dada
    ///
    /// Compara las activaciones SAE actuales con los valores deseados
    /// del feedback humano, calculando la divergencia media ponderada.
    ///
    /// # Argumentos
    /// * `layer_id` - ID de la capa SAE a evaluar
    /// * `activations` - Tensor de activaciones actuales (batch_size x feature_dim)
    ///
    /// # Retorna
    /// Puntuación de deriva en el rango [0.0, 1.0]
    pub fn calculate_drift(
        &self,
        layer_id: &str,
        activations: &Tensor,
    ) -> Result<f32, AlignmentError> {
        let feedback_list = match self.feedback_buffer.get(layer_id) {
            Some(fb) if !fb.is_empty() => fb,
            Some(_) => {
                return Err(AlignmentError::NoFeedbackForLayer {
                    layer_id: layer_id.to_string(),
                })
            }
            None => {
                // Sin feedback: deriva no calculable, retornar 0.0
                warn!(layer = %layer_id, "No feedback available for drift calculation");
                return Ok(0.0);
            }
        };

        // Validar que activations no esté vacía
        let shape = activations.shape().dims().to_vec();
        if shape.is_empty() || shape.iter().product::<usize>() == 0 {
            return Err(AlignmentError::EmptyActivations);
        }

        // Convertir activations a vector para procesamiento
        let activations_vec = activations
            .to_vec2::<f32>()
            .map_err(AlignmentError::Device)?; // CLEANUP: redundant closure

        if activations_vec.is_empty() {
            return Err(AlignmentError::EmptyActivations);
        }

        // Calcular divergencia ponderada por feature
        let mut total_divergence: f64 = 0.0;
        let mut total_weight: f64 = 0.0;

        for fb in feedback_list {
            if fb.feature_idx as usize >= activations_vec[0].len() {
                continue; // Skip features fuera de rango
            }

            let current = fb.current_activation as f64;
            let desired = fb.desired_value as f64;
            let weight = fb.annotator_confidence as f64;

            // Divergencia normalizada: |current - desired| / (1.0 + |desired|)
            let divergence = (current - desired).abs() / (1.0 + desired.abs());
            total_divergence += divergence * weight;
            total_weight += weight;
        }

        let drift_score = if total_weight > 0.0 {
            (total_divergence / total_weight) as f32
        } else {
            0.0
        };

        // Clamp a [0.0, 1.0]
        let drift_score = drift_score.clamp(0.0, 1.0);

        // Actualizar historial
        if let Some(history) = self.drift_history.get(layer_id) {
            let latest_score = history.last().copied().unwrap_or(drift_score);
            // Smooth con exponential moving average
            let alpha = 0.3;
            let smoothed = alpha * drift_score + (1.0 - alpha) * latest_score;
            info!(
                layer = %layer_id,
                drift = drift_score,
                smoothed = smoothed,
                feedback_count = feedback_list.len(),
                "Drift calculated"
            );
            Ok(smoothed)
        } else {
            info!(
                layer = %layer_id,
                drift = drift_score,
                feedback_count = feedback_list.len(),
                "Drift calculated (first measurement)"
            );
            Ok(drift_score)
        }
    }

    /// Genera ajuste de steering basado en feedback y deriva
    ///
    /// Calcula un delta de steering que corrige las activaciones
    /// hacia los valores deseados por los anotadores humanos.
    ///
    /// # Argumentos
    /// * `layer_id` - ID de la capa SAE
    /// * `current_activations` - Tensor de activaciones actuales
    ///
    /// # Retorna
    /// AlignmentResult con drift_score, flagged_concepts, steering_delta y confidence
    pub fn generate_steering_adjustment(
        &mut self,
        layer_id: &str,
        current_activations: &Tensor,
    ) -> Result<AlignmentResult, AlignmentError> {
        // Calcular drift primero
        let drift_score = self.calculate_drift(layer_id, current_activations)?;

        let feedback_list = match self.feedback_buffer.get(layer_id) {
            Some(fb) => fb,
            None => {
                // Sin feedback: resultado neutro
                return self.create_neutral_result(current_activations); // CLEANUP: Removed unnecessary Ok() wrapping with ?
            }
        };

        // Identificar conceptos con deriva superior al umbral
        let mut flagged_concepts = Vec::new();
        let mut concept_drift: HashMap<String, (f64, f64)> = HashMap::new(); // concept -> (total_div, total_weight)

        let activations_vec = current_activations
            .to_vec2::<f32>()
            .map_err(AlignmentError::Device)?; // CLEANUP: redundant closure

        for fb in feedback_list {
            let concept = match &fb.concept {
                Some(c) => c.clone(),
                None => format!("feature_{}", fb.feature_idx),
            };

            if fb.feature_idx as usize >= activations_vec[0].len() {
                continue;
            }

            let current = fb.current_activation as f64;
            let desired = fb.desired_value as f64;
            let weight = fb.annotator_confidence as f64;
            let divergence = (current - desired).abs() / (1.0 + desired.abs());

            let entry = concept_drift.entry(concept.clone()).or_insert((0.0, 0.0));
            entry.0 += divergence * weight;
            entry.1 += weight;
        }

        // Flag concepts exceeding threshold
        for (concept, (total_div, total_weight)) in &concept_drift {
            if *total_weight > 0.0 {
                let avg_divergence = total_div / total_weight;
                if avg_divergence > self.config.drift_threshold as f64 {
                    flagged_concepts.push(concept.clone());
                    warn!(
                        concept = %concept,
                        divergence = avg_divergence,
                        threshold = self.config.drift_threshold,
                        "Concept flagged for high drift"
                    );
                }
            }
        }

        // Calcular steering delta como tensor
        let steering_delta =
            self.compute_steering_delta(layer_id, &activations_vec, feedback_list)?;

        // Calcular confianza basado en cantidad y calidad de feedback
        let confidence = self.compute_confidence(feedback_list);

        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Ok(AlignmentResult {
            drift_score,
            flagged_concepts,
            steering_delta,
            confidence,
            features_analyzed: feedback_list.len(),
            timestamp_ms,
        })
    }

    /// Valida que los umbrales de alineación se cumplan
    ///
    /// Retorna Ok si drift < critical_threshold, Err si se excede.
    pub fn validate_thresholds(
        &self,
        layer_id: &str,
        activations: &Tensor,
    ) -> Result<(), AlignmentError> {
        let drift = self.calculate_drift(layer_id, activations)?;

        if drift >= self.config.critical_threshold {
            return Err(AlignmentError::DriftThresholdExceeded {
                reason: format!("Critical drift on layer {}", layer_id),
                drift_threshold: self.config.critical_threshold,
                current: drift,
            });
        }

        if drift >= self.config.drift_threshold {
            warn!(
                layer = %layer_id,
                drift = drift,
                threshold = self.config.drift_threshold,
                "Drift exceeds warning threshold"
            );
        }

        Ok(())
    }

    /// Obtiene la configuración actual
    pub fn config(&self) -> &AlignmentConfig {
        &self.config
    }

    /// Actualiza la configuración
    pub fn set_config(&mut self, config: AlignmentConfig) {
        self.config = config;
    }

    /// Limpia el buffer de feedback para una capa
    pub fn clear_feedback(&mut self, layer_id: &str) {
        self.feedback_buffer.remove(layer_id);
        self.drift_history.remove(layer_id);
    }

    /// Limpia todos los buffers
    pub fn clear_all(&mut self) {
        self.feedback_buffer.clear();
        self.drift_history.clear();
    }

    /// Obtiene el historial de drift para una capa
    pub fn get_drift_history(&self, layer_id: &str) -> Vec<f32> {
        self.drift_history
            .get(layer_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Número de entries de feedback en buffer para una capa
    pub fn feedback_count(&self, layer_id: &str) -> usize {
        self.feedback_buffer.get(layer_id).map_or(0, Vec::len)
    }

    // =====================================================================
    // Métodos privados
    // =====================================================================

    /// Calcula el delta de steering como vector plano
    fn compute_steering_delta(
        &self,
        layer_id: &str,
        activations: &[Vec<f32>], // CLEANUP: &Vec -> &[...] (clippy::ptr_arg)
        feedback_list: &[AlignmentFeedback],
    ) -> Result<Vec<f32>, AlignmentError> {
        if activations.is_empty() {
            return Ok(Vec::new());
        }

        let feature_dim = activations[0].len();
        let mut delta = vec![0.0f32; feature_dim];

        for fb in feedback_list {
            let idx = fb.feature_idx as usize;
            if idx >= feature_dim {
                continue;
            }

            // Delta = learning_rate * feedback_weight * confidence * (desired - current)
            let correction = fb.desired_value - fb.current_activation;
            let weighted_delta = self.config.learning_rate
                * self.config.feedback_weight
                * fb.annotator_confidence
                * correction;

            delta[idx] += weighted_delta;
        }

        debug!(
            layer = %layer_id,
            non_zero_deltas = delta.iter().filter(|&&x| x != 0.0).count(),
            "Steering delta computed"
        );

        Ok(delta)
    }

    /// Calcula confianza basado en cantidad y calidad de feedback
    fn compute_confidence(&self, feedback_list: &[AlignmentFeedback]) -> f32 {
        if feedback_list.is_empty() {
            return 0.0;
        }

        let avg_confidence: f32 = feedback_list
            .iter()
            .map(|fb| fb.annotator_confidence)
            .sum::<f32>()
            / feedback_list.len() as f32;

        // Bonus por volumen de feedback (saturating a 1.0)
        let volume_bonus =
            (feedback_list.len() as f32 / self.config.feedback_window as f32).min(0.3);

        (avg_confidence + volume_bonus).min(1.0)
    }

    /// Crea resultado neutro cuando no hay feedback
    fn create_neutral_result(
        &self,
        activations: &Tensor,
    ) -> Result<AlignmentResult, AlignmentError> {
        let shape = activations.shape().dims().to_vec();
        let features_analyzed = shape.iter().product::<usize>();

        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Ok(AlignmentResult {
            drift_score: 0.0,
            flagged_concepts: Vec::new(),
            steering_delta: Vec::new(),
            confidence: 0.0,
            features_analyzed,
            timestamp_ms,
        })
    }
}

impl Default for AlignmentScorer {
    fn default() -> Self {
        Self::new(AlignmentConfig::default())
    }
}
