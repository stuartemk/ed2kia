//! Confidence Calculator v2 — Calculadora de confianza ponderada para señales de alineación
//!
//! Implementa `ConfidenceCalculator` para:
//! 1. Cálculo de confianza ponderada basado en reputación del anotador
//! 2. Decaimiento temporal de confianza no verificada
//! 3. Detección de anomalías en patrones de feedback
//! 4. Agregación de confianza multi-fuente con consenso
//! 5. Historial de confianza por anotador con tendencias
//!
//! **Feature:** `v1.1-sprint4`

#[cfg(feature = "v1.1-sprint4")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "v1.1-sprint4")]
use std::collections::{HashMap, VecDeque};
#[cfg(feature = "v1.1-sprint4")]
use thiserror::Error;
#[cfg(feature = "v1.1-sprint4")]
use tracing::{debug, warn};

// ============================================================================
// Errors
// ============================================================================

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Error)]
pub enum ConfidenceError {
    #[error("Invalid confidence value: {value} (must be 0.0 - 1.0)")]
    InvalidConfidence { value: f32 },

    #[error("Annotator not found: {annotator_id}")]
    AnnotatorNotFound { annotator_id: String },

    #[error("Insufficient data for confidence calculation: {count} samples < {min}")]
    InsufficientData { count: usize, min: usize },

    #[error("Anomaly detected for annotator: {annotator_id}")]
    AnomalyDetected { annotator_id: String },

    #[error("Trust decay exceeded threshold: {decay:.4} < {threshold:.4}")]
    TrustDecayed { decay: f32, threshold: f32 },
}

// ============================================================================
// Annotator Trust Record
// ============================================================================

/// Registro de confianza por anotador
#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotatorTrustRecord {
    /// ID del anotador
    pub annotator_id: String,
    /// Confianza actual (0.0 - 1.0)
    pub current_confidence: f32,
    /// Confianza base inicial
    pub base_confidence: f32,
    /// Total de feedback proporcionados
    pub total_feedback: u64,
    /// Feedback aceptados (drift reducido post-steering)
    pub accepted_feedback: u64,
    /// Feedback rechazados (drift aumentado post-steering)
    pub rejected_feedback: u64,
    /// Última actualización (epoch ms)
    pub last_updated_ms: u64,
    /// Historial de confianza (últimos N valores)
    pub confidence_history: VecDeque<f32>,
    /// Desviación estándar de confianza
    pub std_dev: f32,
}

#[cfg(feature = "v1.1-sprint4")]
impl AnnotatorTrustRecord {
    pub fn new(annotator_id: String, base_confidence: f32) -> Self {
        Self {
            annotator_id,
            current_confidence: base_confidence,
            base_confidence,
            total_feedback: 0,
            accepted_feedback: 0,
            rejected_feedback: 0,
            last_updated_ms: current_timestamp_ms(),
            confidence_history: VecDeque::with_capacity(50),
            std_dev: 0.0,
        }
    }

    /// Actualiza confianza basado en resultado de feedback
    pub fn update(&mut self, accepted: bool, max_history: usize) {
        self.total_feedback += 1;

        if accepted {
            self.accepted_feedback += 1;
        } else {
            self.rejected_feedback += 1;
        }

        // Recalcular confianza como ratio de aceptación con suavizado
        let acceptance_ratio = if self.total_feedback > 0 {
            self.accepted_feedback as f32 / self.total_feedback as f32
        } else {
            self.base_confidence
        };

        // Suavizado exponencial
        let alpha = 0.3;
        self.current_confidence =
            alpha * acceptance_ratio + (1.0 - alpha) * self.current_confidence;

        // Clamp a [0.0, 1.0]
        self.current_confidence = self.current_confidence.clamp(0.0, 1.0);

        self.confidence_history.push_back(self.current_confidence);
        if self.confidence_history.len() > max_history {
            self.confidence_history.pop_front();
        }

        self.std_dev = self.compute_std_dev();
        self.last_updated_ms = current_timestamp_ms();

        debug!(
            "Trust updated: annotator={}, confidence={:.4}, accepted={}/{}",
            self.annotator_id, self.current_confidence, self.accepted_feedback, self.total_feedback,
        );
    }

    fn compute_std_dev(&self) -> f32 {
        if self.confidence_history.len() < 2 {
            return 0.0;
        }

        let mean: f32 =
            self.confidence_history.iter().sum::<f32>() / self.confidence_history.len() as f32;
        let variance: f32 = self
            .confidence_history
            .iter()
            .map(|&v| (v - mean).powi(2))
            .sum::<f32>()
            / self.confidence_history.len() as f32;

        variance.sqrt()
    }

    /// Aplica decaimiento temporal a la confianza
    pub fn apply_decay(&mut self, decay_rate: f32, max_age_ms: u64) {
        let now = current_timestamp_ms();
        let age_ms = now.saturating_sub(self.last_updated_ms);

        if age_ms > max_age_ms {
            let excess_ms = age_ms - max_age_ms;
            let decay_factor = 1.0 - (decay_rate * excess_ms as f32 / 1_000_000.0);
            let clamped_factor = decay_factor.clamp(0.1, 1.0);

            self.current_confidence *= clamped_factor;
            self.current_confidence = self.current_confidence.clamp(0.0, 1.0);

            if clamped_factor < 0.5 {
                warn!(
                    "Significant trust decay for annotator {}: {:.4} (age: {}ms)",
                    self.annotator_id, self.current_confidence, age_ms
                );
            }
        }
    }

    /// Verifica si hay anomalía en el patrón de confianza
    pub fn has_anomaly(&self, std_dev_threshold: f32) -> bool {
        // Anomalía si desviación estándar es muy alta (inconsistencia)
        if self.std_dev > std_dev_threshold {
            return true;
        }

        // Anomalía si confianza cae abruptamente
        if self.confidence_history.len() >= 3 {
            let recent: Vec<f32> = self
                .confidence_history
                .iter()
                .rev()
                .take(3)
                .cloned()
                .collect();
            if recent[0] + 0.3 < recent[2] {
                // Caída de más de 0.3 en 3 muestras
                return true;
            }
        }

        false
    }
}

// ============================================================================
// Confidence Stats
// ============================================================================

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceStats {
    /// Total de anotadores registrados
    pub total_annotators: usize,
    /// Confianza promedio global
    pub avg_confidence: f32,
    /// Confianza máxima
    pub max_confidence: f32,
    /// Confianza mínima
    pub min_confidence: f32,
    /// Anotadores con anomalía detectada
    pub anomaly_count: usize,
    /// Total de feedback procesados
    pub total_feedback_processed: u64,
}

#[cfg(feature = "v1.1-sprint4")]
impl Default for ConfidenceStats {
    fn default() -> Self {
        Self {
            total_annotators: 0,
            avg_confidence: 0.0,
            max_confidence: 0.0,
            min_confidence: 1.0,
            anomaly_count: 0,
            total_feedback_processed: 0,
        }
    }
}

// ============================================================================
// Confidence Config
// ============================================================================

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceConfig {
    /// Confianza base para nuevos anotadores
    pub base_confidence: f32,
    /// Tasa de decaimiento temporal (por millón de ms)
    pub decay_rate: f32,
    /// Edad máxima antes de decaimiento (ms)
    pub max_age_before_decay_ms: u64,
    /// Umbral de desviación estándar para anomalías
    pub std_dev_anomaly_threshold: f32,
    /// Máximo de muestras en historial
    pub max_history_size: usize,
    /// Mínimo de feedback para calcular confianza
    pub min_feedback_for_confidence: usize,
    /// Umbral mínimo de confianza para aceptar feedback
    pub min_acceptable_confidence: f32,
}

#[cfg(feature = "v1.1-sprint4")]
impl Default for ConfidenceConfig {
    fn default() -> Self {
        Self {
            base_confidence: 0.7,
            decay_rate: 0.05,
            max_age_before_decay_ms: 3_600_000, // 1 hora
            std_dev_anomaly_threshold: 0.2,
            max_history_size: 50,
            min_feedback_for_confidence: 5,
            min_acceptable_confidence: 0.3,
        }
    }
}

// ============================================================================
// Confidence Calculator
// ============================================================================

#[cfg(feature = "v1.1-sprint4")]
pub struct ConfidenceCalculator {
    config: ConfidenceConfig,
    records: HashMap<String, AnnotatorTrustRecord>,
    stats: ConfidenceStats,
}

#[cfg(feature = "v1.1-sprint4")]
impl Default for ConfidenceCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "v1.1-sprint4")]
impl ConfidenceCalculator {
    /// Crea calculadora con configuración default
    pub fn new() -> Self {
        Self {
            config: ConfidenceConfig::default(),
            records: HashMap::new(),
            stats: ConfidenceStats::default(),
        }
    }

    /// Crea calculadora con configuración personalizada
    pub fn with_config(config: ConfidenceConfig) -> Self {
        Self {
            config,
            records: HashMap::new(),
            stats: ConfidenceStats::default(),
        }
    }

    /// Registra un nuevo anotador
    pub fn register_annotator(&mut self, annotator_id: String) {
        let record = AnnotatorTrustRecord::new(annotator_id.clone(), self.config.base_confidence);
        self.records.insert(annotator_id, record);
        self.recalc_stats();
    }

    /// Actualiza confianza del anotador basado en resultado
    pub fn update_confidence(
        &mut self,
        annotator_id: &str,
        accepted: bool,
    ) -> Result<f32, ConfidenceError> {
        // Verificar que el anotador existe
        if !self.records.contains_key(annotator_id) {
            return Err(ConfidenceError::AnnotatorNotFound {
                annotator_id: annotator_id.to_string(),
            });
        }

        // Obtener referencia mutable y actualizar
        if let Some(record) = self.records.get_mut(annotator_id) {
            let max_history = self.config.max_history_size;
            record.update(accepted, max_history);
        }
        self.recalc_stats();

        // Devolver confianza actual
        Ok(self
            .records
            .get(annotator_id)
            .map(|r| r.current_confidence)
            .unwrap_or(0.0))
    }

    /// Obtiene confianza actual de un anotador
    pub fn get_confidence(&self, annotator_id: &str) -> Result<f32, ConfidenceError> {
        let record =
            self.records
                .get(annotator_id)
                .ok_or_else(|| ConfidenceError::AnnotatorNotFound {
                    annotator_id: annotator_id.to_string(),
                })?;

        Ok(record.current_confidence)
    }

    /// Calcula confianza ponderada para un conjunto de feedback
    pub fn compute_weighted_confidence(
        &self,
        annotator_ids: &[String],
    ) -> Result<f32, ConfidenceError> {
        if annotator_ids.is_empty() {
            return Err(ConfidenceError::InsufficientData {
                count: 0,
                min: self.config.min_feedback_for_confidence,
            });
        }

        let mut total_weight: f32 = 0.0;
        let mut total_confidence: f32 = 0.0;

        for id in annotator_ids {
            let record =
                self.records
                    .get(id)
                    .ok_or_else(|| ConfidenceError::AnnotatorNotFound {
                        annotator_id: id.clone(),
                    })?;

            // Verificar mínimo de feedback
            if record.total_feedback < self.config.min_feedback_for_confidence as u64 {
                continue;
            }

            // Verificar anomalía
            if record.has_anomaly(self.config.std_dev_anomaly_threshold) {
                warn!("Anomaly detected for annotator: {}", id);
                // Reducir peso a la mitad para anotadores con anomalía
                total_weight += 0.5;
                total_confidence += record.current_confidence * 0.5;
            } else {
                total_weight += 1.0;
                total_confidence += record.current_confidence;
            }
        }

        if total_weight == 0.0 {
            return Err(ConfidenceError::InsufficientData {
                count: annotator_ids.len(),
                min: self.config.min_feedback_for_confidence,
            });
        }

        Ok(total_confidence / total_weight)
    }

    /// Aplica decaimiento temporal a todos los anotadores
    pub fn apply_decay_to_all(&mut self) {
        for record in self.records.values_mut() {
            record.apply_decay(self.config.decay_rate, self.config.max_age_before_decay_ms);
        }
        self.recalc_stats();
    }

    /// Detecta anotadores con anomalías
    pub fn detect_anomalies(&self) -> Vec<String> {
        self.records
            .values()
            .filter(|r| r.has_anomaly(self.config.std_dev_anomaly_threshold))
            .map(|r| r.annotator_id.clone())
            .collect()
    }

    /// Verifica si un anotador pasa el umbral mínimo
    pub fn passes_threshold(&self, annotator_id: &str) -> Result<bool, ConfidenceError> {
        let confidence = self.get_confidence(annotator_id)?;
        Ok(confidence >= self.config.min_acceptable_confidence)
    }

    /// Obtiene registro completo de un anotador
    pub fn get_record(&self, annotator_id: &str) -> Result<AnnotatorTrustRecord, ConfidenceError> {
        self.records
            .get(annotator_id)
            .cloned()
            .ok_or_else(|| ConfidenceError::AnnotatorNotFound {
                annotator_id: annotator_id.to_string(),
            })
    }

    fn recalc_stats(&mut self) {
        if self.records.is_empty() {
            self.stats = ConfidenceStats::default();
            return;
        }

        let confidences: Vec<f32> = self
            .records
            .values()
            .map(|r| r.current_confidence)
            .collect();
        let total: f32 = confidences.iter().sum();

        self.stats.total_annotators = self.records.len();
        self.stats.avg_confidence = total / self.records.len() as f32;
        self.stats.max_confidence = *confidences
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(&0.0);
        self.stats.min_confidence = *confidences
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(&1.0);
        self.stats.anomaly_count = self.detect_anomalies().len();
        self.stats.total_feedback_processed = self.records.values().map(|r| r.total_feedback).sum();
    }

    /// Obtiene estadísticas globales
    pub fn get_stats(&self) -> ConfidenceStats {
        self.stats.clone()
    }

    /// Obtiene configuración actual
    pub fn config(&self) -> &ConfidenceConfig {
        &self.config
    }

    /// Obtiene número de anotadores registrados
    pub fn annotator_count(&self) -> usize {
        self.records.len()
    }

    /// Reinicia estadísticas
    pub fn reset_stats(&mut self) {
        self.stats = ConfidenceStats::default();
    }
}

#[cfg(feature = "v1.1-sprint4")]
fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(all(test, feature = "v1.1-sprint4"))]
mod tests {
    use super::*;

    #[test]
    fn test_calculator_creation() {
        let calc = ConfidenceCalculator::new();
        assert_eq!(calc.annotator_count(), 0);
        let stats = calc.get_stats();
        assert_eq!(stats.total_annotators, 0);
    }

    #[test]
    fn test_calculator_with_config() {
        let config = ConfidenceConfig {
            base_confidence: 0.8,
            ..Default::default()
        };
        let calc = ConfidenceCalculator::with_config(config);
        assert_eq!(calc.config().base_confidence, 0.8);
    }

    #[test]
    fn test_register_annotator() {
        let mut calc = ConfidenceCalculator::new();
        calc.register_annotator("annotator-1".to_string());
        assert_eq!(calc.annotator_count(), 1);
        assert_eq!(calc.get_confidence("annotator-1").unwrap(), 0.7);
    }

    #[test]
    fn test_update_confidence_accepted() {
        let mut calc = ConfidenceCalculator::new();
        calc.register_annotator("annotator-1".to_string());

        for _ in 0..10 {
            calc.update_confidence("annotator-1", true).unwrap();
        }

        let confidence = calc.get_confidence("annotator-1").unwrap();
        assert!(confidence > 0.7);
    }

    #[test]
    fn test_update_confidence_rejected() {
        let mut calc = ConfidenceCalculator::new();
        calc.register_annotator("annotator-1".to_string());

        for _ in 0..10 {
            calc.update_confidence("annotator-1", false).unwrap();
        }

        let confidence = calc.get_confidence("annotator-1").unwrap();
        assert!(confidence < 0.7);
    }

    #[test]
    fn test_compute_weighted_confidence() {
        let mut calc = ConfidenceCalculator::new();
        calc.register_annotator("a1".to_string());
        calc.register_annotator("a2".to_string());

        // Dar feedback a ambos
        for _ in 0..10 {
            calc.update_confidence("a1", true).unwrap();
            calc.update_confidence("a2", true).unwrap();
        }

        let weighted = calc
            .compute_weighted_confidence(&["a1".to_string(), "a2".to_string()])
            .unwrap();
        assert!(weighted > 0.7);
    }

    #[test]
    fn test_insufficient_data_error() {
        let calc = ConfidenceCalculator::new();
        let result = calc.compute_weighted_confidence(&["unknown".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_passes_threshold() {
        let mut calc = ConfidenceCalculator::new();
        calc.register_annotator("annotator-1".to_string());

        // Confianza base es 0.7, umbral mínimo es 0.3
        assert!(calc.passes_threshold("annotator-1").unwrap());
    }

    #[test]
    fn test_get_record() {
        let mut calc = ConfidenceCalculator::new();
        calc.register_annotator("annotator-1".to_string());

        let record = calc.get_record("annotator-1").unwrap();
        assert_eq!(record.annotator_id, "annotator-1");
        assert_eq!(record.total_feedback, 0);
    }

    #[test]
    fn test_stats_tracking() {
        let mut calc = ConfidenceCalculator::new();
        calc.register_annotator("a1".to_string());
        calc.register_annotator("a2".to_string());

        let stats = calc.get_stats();
        assert_eq!(stats.total_annotators, 2);
    }

    #[test]
    fn test_reset_stats() {
        let mut calc = ConfidenceCalculator::new();
        calc.register_annotator("a1".to_string());
        calc.reset_stats();
        let stats = calc.get_stats();
        assert_eq!(stats.total_annotators, 0);
    }

    #[test]
    fn test_config_default() {
        let config = ConfidenceConfig::default();
        assert_eq!(config.base_confidence, 0.7);
        assert_eq!(config.min_acceptable_confidence, 0.3);
    }

    #[test]
    fn test_annotator_not_found() {
        let calc = ConfidenceCalculator::new();
        assert!(calc.get_confidence("nonexistent").is_err());
    }

    #[test]
    fn test_anomaly_detection() {
        let calc = ConfidenceCalculator::new();
        let anomalies = calc.detect_anomalies();
        assert!(anomalies.is_empty());
    }
}
