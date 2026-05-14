//! Alignment Loop v2 — Motor de alineación continua con feedback loop, steering signals y confianza ponderada
//!
//! Implementa `AlignmentLoopV2` para cerrar el ciclo de alineación con:
//! 1. Ingesta de feedback con validación criptográfica (ed25519-dalek)
//! 2. Cálculo de drift ponderado por confianza del anotador
//! 3. Generación de steering deltas con umbral de confianza mínimo
//! 4. Rollback automático si la métrica se degrada post-aplicación
//! 5. Auditoría completa de cada ciclo de alineación
//!
//! **Feature:** `v1.1-sprint4`

#[cfg(feature = "v1.1-sprint4")]
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
#[cfg(feature = "v1.1-sprint4")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "v1.1-sprint4")]
use sha2::{Digest, Sha256};
#[cfg(feature = "v1.1-sprint4")]
use std::collections::{HashMap, VecDeque};
#[cfg(feature = "v1.1-sprint4")]
use std::time::{Duration, Instant};
#[cfg(feature = "v1.1-sprint4")]
use thiserror::Error;
#[cfg(feature = "v1.1-sprint4")]
use tracing::{debug, info, warn};

// ============================================================================
// Errors
// ============================================================================

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Error)]
pub enum LoopV2Error {
    #[error("Feedback validation failed: {reason}")]
    ValidationFailed { reason: String },

    #[error("Signature verification failed for annotator: {annotator_id}")]
    SignatureFailed { annotator_id: String },

    #[error("Steering threshold not met: confidence {confidence:.4} < {threshold:.4}")]
    SteeringThresholdNotMet { confidence: f32, threshold: f32 },

    #[error("Rollback triggered: drift degraded from {before:.4} to {after:.4}")]
    RollbackTriggered { before: f32, after: f32 },

    #[error("Rate limit exceeded: {current}/{max} entries per window")]
    RateLimitExceeded { current: usize, max: usize },

    #[error("Loop already running")]
    LoopAlreadyRunning,

    #[error("No feedback available for processing")]
    NoFeedbackAvailable,

    #[error("Window expired: {elapsed_ms}ms > {max_ms}ms")]
    WindowExpired { elapsed_ms: u64, max_ms: u64 },
}

// ============================================================================
// Feedback Entry
// ============================================================================

/// Entrada de feedback con firma criptográfica
#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackEntryV2 {
    /// ID único del feedback
    pub entry_id: String,
    /// ID del anotador
    pub annotator_id: String,
    /// Capa SAE afectada
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
    /// Timestamp de ingestión (epoch ms)
    pub ingested_at_ms: u64,
    /// Firma ed25519 del anotador (hex)
    pub signature: String,
    /// Estado de procesamiento
    pub processed: bool,
}

#[cfg(feature = "v1.1-sprint4")]
impl FeedbackEntryV2 {
    /// Crea nueva entrada de feedback
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        annotator_id: String,
        layer_id: String,
        feature_idx: u32,
        current_activation: f32,
        desired_value: f32,
        annotator_confidence: f32,
        concept: Option<String>,
        signing_key: &SigningKey,
    ) -> Self {
        let ingested_at_ms = current_timestamp_ms();
        let message = format!(
            "{}:{}:{}:{}:{}:{}",
            annotator_id, layer_id, feature_idx, current_activation, desired_value, ingested_at_ms
        );
        let signature = signing_key.sign(message.as_bytes());
        let entry_id = Self::generate_id(&message);

        Self {
            entry_id,
            annotator_id,
            layer_id,
            feature_idx,
            current_activation,
            desired_value,
            annotator_confidence,
            concept,
            ingested_at_ms,
            signature: hex::encode(signature.to_bytes()),
            processed: false,
        }
    }

    fn generate_id(message: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(message);
        let result = hasher.finalize();
        hex::encode(&result[..8])
    }

    /// Verifica la firma del anotador
    pub fn verify_signature(&self, verifying_key: &VerifyingKey) -> bool {
        let sig_bytes = match hex::decode(&self.signature) {
            Ok(b) => b,
            Err(_) => return false,
        };
        let signature = match Signature::from_slice(&sig_bytes) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let message = format!(
            "{}:{}:{}:{}:{}:{}",
            self.annotator_id,
            self.layer_id,
            self.feature_idx,
            self.current_activation,
            self.desired_value,
            self.ingested_at_ms,
        );
        verifying_key.verify(message.as_bytes(), &signature).is_ok()
    }

    /// Verifica si la entrada ha expirado
    pub fn is_expired(&self, window: Duration) -> bool {
        let now = current_timestamp_ms();
        let elapsed = now.saturating_sub(self.ingested_at_ms);
        Duration::from_millis(elapsed) > window
    }

    /// Calcula el drift individual de esta entrada
    pub fn drift(&self) -> f32 {
        (self.current_activation - self.desired_value).abs().min(1.0)
    }
}

// ============================================================================
// Steering Signal
// ============================================================================

/// Señal de steering generada por el loop
#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteeringSignal {
    /// ID único de la señal
    pub signal_id: String,
    /// Capa SAE afectada
    pub layer_id: String,
    /// Delta de steering (vector de ajustes)
    pub delta: Vec<f32>,
    /// Confianza ponderada de la señal
    pub weighted_confidence: f32,
    /// Número de entradas de feedback que contribuyeron
    pub feedback_count: usize,
    /// Drift promedio antes de aplicar
    pub drift_before: f32,
    /// Timestamp de generación (epoch ms)
    pub generated_at_ms: u64,
    /// Firma del loop manager (hex)
    pub signature: String,
}

#[cfg(feature = "v1.1-sprint4")]
impl SteeringSignal {
    pub fn new(layer_id: String, delta: Vec<f32>, weighted_confidence: f32, feedback_count: usize, drift_before: f32, signing_key: &SigningKey) -> Self {
        let generated_at_ms = current_timestamp_ms();
        let message = format!(
            "{}:{}:{}:{}:{}",
            layer_id,
            weighted_confidence,
            feedback_count,
            drift_before,
            generated_at_ms
        );
        let signature = signing_key.sign(message.as_bytes());
        let signal_id = {
            let mut hasher = Sha256::new();
            hasher.update(&message);
            let result = hasher.finalize();
            hex::encode(&result[..8])
        };

        Self {
            signal_id,
            layer_id,
            delta,
            weighted_confidence,
            feedback_count,
            drift_before,
            generated_at_ms,
            signature: hex::encode(signature.to_bytes()),
        }
    }

    /// Verifica la firma de la señal
    pub fn verify_signature(&self, verifying_key: &VerifyingKey) -> bool {
        let sig_bytes = match hex::decode(&self.signature) {
            Ok(b) => b,
            Err(_) => return false,
        };
        let signature = match Signature::from_slice(&sig_bytes) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let message = format!(
            "{}:{}:{}:{}:{}",
            self.layer_id,
            self.weighted_confidence,
            self.feedback_count,
            self.drift_before,
            self.generated_at_ms,
        );
        verifying_key.verify(message.as_bytes(), &signature).is_ok()
    }
}

// ============================================================================
// Loop Stats
// ============================================================================

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopV2Stats {
    /// Total de feedback procesados
    pub feedback_processed: u64,
    /// Total de señales de steering aplicadas
    pub signals_applied: u64,
    /// Total de rollbacks ejecutados
    pub rollbacks_executed: u64,
    /// Drift promedio actual
    pub avg_drift: f32,
    /// Confianza promedio de señales
    pub avg_signal_confidence: f32,
    /// Último ciclo completado (epoch ms)
    pub last_cycle_ms: u64,
    /// Latencia promedio de procesamiento (ms)
    pub avg_processing_ms: f64,
}

#[cfg(feature = "v1.1-sprint4")]
impl Default for LoopV2Stats {
    fn default() -> Self {
        Self {
            feedback_processed: 0,
            signals_applied: 0,
            rollbacks_executed: 0,
            avg_drift: 1.0,
            avg_signal_confidence: 0.0,
            last_cycle_ms: 0,
            avg_processing_ms: 0.0,
        }
    }
}

// ============================================================================
// Loop Config
// ============================================================================

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopV2Config {
    /// Umbral mínimo de confianza para aplicar steering
    pub min_confidence_threshold: f32,
    /// Ventana temporal de feedback (ms)
    pub feedback_window_ms: u64,
    /// Máximo de entradas por ventana
    pub max_entries_per_window: usize,
    /// Tasa de aprendizaje para ajustes de steering
    pub learning_rate: f32,
    /// Umbral de rollback (drift degradado > este %)
    pub rollback_threshold: f32,
    /// Número máximo de features por señal
    pub max_features_per_signal: usize,
}

#[cfg(feature = "v1.1-sprint4")]
impl Default for LoopV2Config {
    fn default() -> Self {
        Self {
            min_confidence_threshold: 0.85,
            feedback_window_ms: 5_000,
            max_entries_per_window: 1000,
            learning_rate: 0.01,
            rollback_threshold: 0.1,
            max_features_per_signal: 256,
        }
    }
}

// ============================================================================
// Alignment Loop V2
// ============================================================================

#[cfg(feature = "v1.1-sprint4")]
pub struct AlignmentLoopV2 {
    config: LoopV2Config,
    feedback_queue: VecDeque<FeedbackEntryV2>,
    applied_signals: VecDeque<SteeringSignal>,
    stats: LoopV2Stats,
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
    // Mapeo de anotadores a claves de verificación
    annotator_keys: HashMap<String, VerifyingKey>,
    // Historial de drift por capa para detección de rollback
    layer_drift_history: HashMap<String, VecDeque<f32>>,
    running: bool,
    // Latencias de procesamiento para stats
    processing_latencies: VecDeque<f64>,
}

#[cfg(feature = "v1.1-sprint4")]
impl AlignmentLoopV2 {
    /// Crea un nuevo loop con clave de firma
    pub fn new(signing_key: SigningKey) -> Self {
        let verifying_key = signing_key.verifying_key();
        Self {
            config: LoopV2Config::default(),
            feedback_queue: VecDeque::new(),
            applied_signals: VecDeque::new(),
            stats: LoopV2Stats::default(),
            signing_key,
            verifying_key,
            annotator_keys: HashMap::new(),
            layer_drift_history: HashMap::new(),
            running: false,
            processing_latencies: VecDeque::new(),
        }
    }

    /// Crea loop con configuración personalizada
    pub fn with_config(config: LoopV2Config, signing_key: SigningKey) -> Self {
        let verifying_key = signing_key.verifying_key();
        Self {
            config,
            feedback_queue: VecDeque::new(),
            applied_signals: VecDeque::new(),
            stats: LoopV2Stats::default(),
            signing_key,
            verifying_key,
            annotator_keys: HashMap::new(),
            layer_drift_history: HashMap::new(),
            running: false,
            processing_latencies: VecDeque::new(),
        }
    }

    /// Registra la clave de verificación de un anotador
    pub fn register_annotator(&mut self, annotator_id: String, verifying_key: VerifyingKey) {
        self.annotator_keys.insert(annotator_id, verifying_key);
    }

    /// Obtiene la clave de verificación del loop
    pub fn verifying_key(&self) -> &VerifyingKey {
        &self.verifying_key
    }

    /// Ingesta feedback con validación de firma
    pub fn ingest_feedback(&mut self, entry: FeedbackEntryV2) -> Result<(), LoopV2Error> {
        // Verificar firma del anotador
        if let Some(verifying_key) = self.annotator_keys.get(&entry.annotator_id) {
            if !entry.verify_signature(verifying_key) {
                return Err(LoopV2Error::SignatureFailed {
                    annotator_id: entry.annotator_id.clone(),
                });
            }
        }

        // Validar rangos
        if entry.annotator_confidence < 0.0 || entry.annotator_confidence > 1.0 {
            return Err(LoopV2Error::ValidationFailed {
                reason: "Confianza del anotador fuera de rango [0.0, 1.0]".to_string(),
            });
        }

        // Verificar rate limit
        if self.feedback_queue.len() >= self.config.max_entries_per_window {
            return Err(LoopV2Error::RateLimitExceeded {
                current: self.feedback_queue.len(),
                max: self.config.max_entries_per_window,
            });
        }

        // Limpiar entradas expiradas
        self.cleanup_expired();

        self.feedback_queue.push_back(entry);
        debug!("Feedback ingested: entry_id={}", self.feedback_queue.back().unwrap().entry_id);
        Ok(())
    }

    /// Ejecuta un ciclo completo del loop: feedback → drift → steering → aplicación
    pub fn run_cycle(&mut self) -> Result<Vec<SteeringSignal>, LoopV2Error> {
        if self.running {
            return Err(LoopV2Error::LoopAlreadyRunning);
        }

        if self.feedback_queue.iter().filter(|e| !e.processed).count() == 0 {
            return Err(LoopV2Error::NoFeedbackAvailable);
        }

        let start = Instant::now();
        self.running = true;

        // Agrupar feedback no procesado por capa (clonar para evitar borrow conflicts)
        let mut layer_feedback: HashMap<String, Vec<FeedbackEntryV2>> = HashMap::new();
        for entry in self.feedback_queue.iter() {
            if !entry.processed {
                layer_feedback
                    .entry(entry.layer_id.clone())
                    .or_default()
                    .push(entry.clone());
            }
        }

        let mut signals = Vec::new();

        // Generar señal de steering por capa
        for (layer_id, entries) in layer_feedback {
            let signal = self.generate_steering_signal_owned(&layer_id, entries)?;
            if let Some(signal) = signal {
                // Verificar confianza mínima
                if signal.weighted_confidence < self.config.min_confidence_threshold {
                    warn!(
                        "Steering signal confidence below threshold: {:.4} < {:.4}",
                        signal.weighted_confidence, self.config.min_confidence_threshold
                    );
                    continue;
                }

                // Aplicar señal con rollback check
                let drift_before = signal.drift_before;
                self.apply_signal(&signal)?;

                // Verificar rollback
                let drift_after = self.compute_layer_drift(&layer_id);
                if drift_after - drift_before > self.config.rollback_threshold {
                    warn!(
                        "Rollback triggered for layer {}: drift {:.4} → {:.4}",
                        layer_id, drift_before, drift_after
                    );
                    self.rollback(&layer_id)?;
                    self.stats.rollbacks_executed += 1;
                } else {
                    signals.push(signal);
                }
            }
        }

        // Marcar feedback como procesado
        for entry in self.feedback_queue.iter_mut() {
            entry.processed = true;
        }

        let elapsed = start.elapsed();
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
        self.processing_latencies.push_back(elapsed_ms);
        if self.processing_latencies.len() > 100 {
            self.processing_latencies.pop_front();
        }

        // Actualizar stats
        self.stats.feedback_processed += self.feedback_queue.iter().filter(|e| e.processed).count() as u64;
        self.stats.signals_applied += signals.len() as u64;
        self.stats.last_cycle_ms = current_timestamp_ms();

        if !self.processing_latencies.is_empty() {
            self.stats.avg_processing_ms = self.processing_latencies.iter().sum::<f64>() / self.processing_latencies.len() as f64;
        }

        self.running = false;
        info!(
            "Cycle completed: {} signals, {:.1}ms",
            signals.len(),
            elapsed_ms
        );

        Ok(signals)
    }

    fn generate_steering_signal_owned(
        &self,
        layer_id: &str,
        entries: Vec<FeedbackEntryV2>,
    ) -> Result<Option<SteeringSignal>, LoopV2Error> {
        if entries.is_empty() {
            return Ok(None);
        }

        // Calcular drift ponderado por confianza
        let total_confidence: f32 = entries.iter().map(|e| e.annotator_confidence).sum();
        if total_confidence == 0.0 {
            return Ok(None);
        }

        // Agrupar por feature_idx para calcular deltas
        let mut feature_deltas: HashMap<u32, (f32, f32)> = HashMap::new();
        for entry in &entries {
            let drift = entry.drift();
            let weighted_drift = drift * entry.annotator_confidence;
            let existing = feature_deltas.entry(entry.feature_idx).or_insert((0.0, 0.0));
            existing.0 += weighted_drift;
            existing.1 += entry.annotator_confidence;
        }

        // Calcular deltas normalizados
        let mut delta: Vec<f32> = feature_deltas
            .into_iter()
            .map(|(_idx, (weighted_drift, total_conf))| {
                -weighted_drift / total_conf * self.config.learning_rate
            })
            .collect();
        delta.truncate(self.config.max_features_per_signal);

        let weighted_confidence = total_confidence / entries.len() as f32;
        let drift_before = entries.iter().map(|e| e.drift()).sum::<f32>() / entries.len() as f32;

        let signal = SteeringSignal::new(
            layer_id.to_string(),
            delta,
            weighted_confidence,
            entries.len(),
            drift_before,
            &self.signing_key,
        );

        Ok(Some(signal))
    }

    fn apply_signal(&mut self, signal: &SteeringSignal) -> Result<(), LoopV2Error> {
        // Guardar drift antes para rollback
        self.layer_drift_history
            .entry(signal.layer_id.clone())
            .or_insert_with(|| VecDeque::with_capacity(10))
            .push_back(signal.drift_before);

        self.applied_signals.push_back(signal.clone());
        if self.applied_signals.len() > 100 {
            self.applied_signals.pop_front();
        }

        debug!(
            "Signal applied: layer={}, confidence={:.4}, delta_len={}",
            signal.layer_id,
            signal.weighted_confidence,
            signal.delta.len()
        );

        Ok(())
    }

    fn rollback(&mut self, layer_id: &str) -> Result<(), LoopV2Error> {
        // Remover última señal aplicada para esta capa
        if let Some(pos) = self
            .applied_signals
            .iter()
            .rposition(|s| s.layer_id == layer_id)
        {
            self.applied_signals.remove(pos);
        }

        warn!("Rollback completed for layer: {}", layer_id);
        Ok(())
    }

    fn compute_layer_drift(&self, layer_id: &str) -> f32 {
        let _now = current_timestamp_ms();
        let window = self.config.feedback_window_ms;

        self.feedback_queue
            .iter()
            .filter(|e| {
                e.layer_id == layer_id && !e.is_expired(Duration::from_millis(window))
            })
            .map(|e| e.drift())
            .collect::<Vec<f32>>()
            .into_iter()
            .sum::<f32>()
            .max(0.0)
    }

    fn cleanup_expired(&mut self) {
        let window = Duration::from_millis(self.config.feedback_window_ms);
        let before = self.feedback_queue.len();
        self.feedback_queue
            .retain(|e| !e.is_expired(window) || !e.processed);
        let removed = before.saturating_sub(self.feedback_queue.len());
        if removed > 0 {
            debug!("Cleaned up {} expired feedback entries", removed);
        }
    }

    /// Obtiene estadísticas del loop
    pub fn get_stats(&self) -> LoopV2Stats {
        self.stats.clone()
    }

    /// Obtiene señales aplicadas recientemente
    pub fn get_applied_signals(&self) -> Vec<SteeringSignal> {
        self.applied_signals.iter().cloned().collect()
    }

    /// Obtiene configuración actual
    pub fn config(&self) -> &LoopV2Config {
        &self.config
    }

    /// Reinicia estadísticas
    pub fn reset_stats(&mut self) {
        self.stats = LoopV2Stats::default();
        self.processing_latencies.clear();
    }

    /// Verifica si el loop está ejecutándose
    pub fn is_running(&self) -> bool {
        self.running
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
    use ed25519_dalek::SigningKey;

    fn test_signing_key() -> SigningKey {
        let mut csprng = ark_std::rand::thread_rng();
        SigningKey::generate(&mut csprng)
    }

    #[test]
    fn test_loop_creation() {
        let key = test_signing_key();
        let loop_v2 = AlignmentLoopV2::new(key);
        assert!(!loop_v2.is_running());
        let stats = loop_v2.get_stats();
        assert_eq!(stats.feedback_processed, 0);
    }

    #[test]
    fn test_loop_with_config() {
        let key = test_signing_key();
        let config = LoopV2Config {
            min_confidence_threshold: 0.5,
            feedback_window_ms: 10_000,
            ..Default::default()
        };
        let loop_v2 = AlignmentLoopV2::with_config(config, key);
        assert_eq!(loop_v2.config().min_confidence_threshold, 0.5);
    }

    #[test]
    fn test_ingest_feedback() {
        let key = test_signing_key();
        let annotator_key = test_signing_key();
        let mut loop_v2 = AlignmentLoopV2::new(key);
        loop_v2.register_annotator("annotator-1".to_string(), annotator_key.verifying_key().clone());

        let entry = FeedbackEntryV2::new(
            "annotator-1".to_string(),
            "layer-0".to_string(),
            0,
            0.8,
            0.2,
            0.9,
            Some("test-concept".to_string()),
            &annotator_key,
        );

        assert!(loop_v2.ingest_feedback(entry).is_ok());
    }

    #[test]
    fn test_feedback_signature_verification() {
        let key = test_signing_key();
        let annotator_key = test_signing_key();
        let wrong_key = test_signing_key();
        let mut loop_v2 = AlignmentLoopV2::new(key);
        // Registrar clave incorrecta
        loop_v2.register_annotator("annotator-1".to_string(), wrong_key.verifying_key().clone());

        let entry = FeedbackEntryV2::new(
            "annotator-1".to_string(),
            "layer-0".to_string(),
            0,
            0.8,
            0.2,
            0.9,
            None,
            &annotator_key,
        );

        assert!(loop_v2.ingest_feedback(entry).is_err());
    }

    #[test]
    fn test_feedback_drift_calculation() {
        let key = test_signing_key();
        let entry = FeedbackEntryV2::new(
            "annotator-1".to_string(),
            "layer-0".to_string(),
            0,
            0.8,
            0.2,
            0.9,
            None,
            &key,
        );
        assert_eq!(entry.drift(), 0.6);
    }

    #[test]
    fn test_rate_limit() {
        let key = test_signing_key();
        let annotator_key = test_signing_key();
        let config = LoopV2Config {
            max_entries_per_window: 2,
            ..Default::default()
        };
        let mut loop_v2 = AlignmentLoopV2::with_config(config, key);
        loop_v2.register_annotator("annotator-1".to_string(), annotator_key.verifying_key().clone());

        let entry1 = FeedbackEntryV2::new(
            "annotator-1".to_string(),
            "layer-0".to_string(),
            0,
            0.8,
            0.2,
            0.9,
            None,
            &annotator_key,
        );
        let entry2 = FeedbackEntryV2::new(
            "annotator-1".to_string(),
            "layer-0".to_string(),
            1,
            0.7,
            0.3,
            0.8,
            None,
            &annotator_key,
        );
        let entry3 = FeedbackEntryV2::new(
            "annotator-1".to_string(),
            "layer-0".to_string(),
            2,
            0.6,
            0.4,
            0.7,
            None,
            &annotator_key,
        );

        assert!(loop_v2.ingest_feedback(entry1).is_ok());
        assert!(loop_v2.ingest_feedback(entry2).is_ok());
        assert!(loop_v2.ingest_feedback(entry3).is_err());
    }

    #[test]
    fn test_run_cycle_no_feedback() {
        let key = test_signing_key();
        let mut loop_v2 = AlignmentLoopV2::new(key);
        let result = loop_v2.run_cycle();
        assert!(result.is_err());
    }

    #[test]
    fn test_steering_signal_signature() {
        let key = test_signing_key();
        let signal = SteeringSignal::new(
            "layer-0".to_string(),
            vec![-0.01, 0.02, -0.005],
            0.9,
            5,
            0.3,
            &key,
        );
        assert!(signal.verify_signature(&key.verifying_key()));
    }

    #[test]
    fn test_stats_tracking() {
        let key = test_signing_key();
        let mut loop_v2 = AlignmentLoopV2::new(key);
        loop_v2.reset_stats();
        let stats = loop_v2.get_stats();
        assert_eq!(stats.feedback_processed, 0);
        assert_eq!(stats.signals_applied, 0);
    }

    #[test]
    fn test_config_default() {
        let config = LoopV2Config::default();
        assert_eq!(config.min_confidence_threshold, 0.85);
        assert_eq!(config.feedback_window_ms, 5_000);
    }

    #[test]
    fn test_stats_default() {
        let stats = LoopV2Stats::default();
        assert_eq!(stats.feedback_processed, 0);
        assert_eq!(stats.avg_drift, 1.0);
    }

    #[test]
    fn test_invalid_confidence_range() {
        let key = test_signing_key();
        let annotator_key = test_signing_key();
        let mut loop_v2 = AlignmentLoopV2::new(key);
        loop_v2.register_annotator("annotator-1".to_string(), annotator_key.verifying_key().clone());

        let entry = FeedbackEntryV2::new(
            "annotator-1".to_string(),
            "layer-0".to_string(),
            0,
            0.8,
            0.2,
            1.5, // Invalid confidence
            None,
            &annotator_key,
        );

        assert!(loop_v2.ingest_feedback(entry).is_err());
    }
}
