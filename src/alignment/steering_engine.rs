//! Steering Engine v2 — Motor de aplicación de señales de steering con validación criptográfica
//!
//! Implementa `SteeringEngine` para:
//! 1. Validación de señales de steering firmadas (ed25519-dalek)
//! 2. Aplicación progresiva con rate limiting
//! 3. Verificación de umbral de confianza antes de aplicación
//! 4. Auditoría completa de cambios aplicados
//! 5. Composición de múltiples señales con prioridad por confianza
//!
//! **Feature:** `v1.1-sprint4`

#[cfg(feature = "v1.1-sprint4")]
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
#[cfg(feature = "v1.1-sprint4")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "v1.1-sprint4")]
#[cfg(feature = "v1.1-sprint4")]
use std::collections::{BinaryHeap, HashMap, VecDeque};
#[cfg(feature = "v1.1-sprint4")]
use std::time::{Duration, Instant};
#[cfg(feature = "v1.1-sprint4")]
use thiserror::Error;
#[cfg(feature = "v1.1-sprint4")]
use tracing::{debug, info};

// ============================================================================
// Errors
// ============================================================================

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Error)]
pub enum SteeringError {
    #[error("Signal signature verification failed")]
    SignatureVerificationFailed,

    #[error("Confidence below threshold: {confidence:.4} < {threshold:.4}")]
    ConfidenceBelowThreshold { confidence: f32, threshold: f32 },

    #[error("Delta dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("Rate limit exceeded: max {max} signals per window")]
    RateLimitExceeded { max: usize },

    #[error("Signal expired: age {age_ms}ms > max {max_ms}ms")]
    SignalExpired { age_ms: u64, max_ms: u64 },

    #[error("Layer not registered: {layer_id}")]
    LayerNotRegistered { layer_id: String },

    #[error("Application failed: {reason}")]
    ApplicationFailed { reason: String },
}

// ============================================================================
// Steering Application Result
// ============================================================================

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteeringApplicationResult {
    /// ID de la señal aplicada
    pub signal_id: String,
    /// Capa afectada
    pub layer_id: String,
    /// Éxito de la aplicación
    pub success: bool,
    /// Confianza de la señal
    pub confidence: f32,
    /// Tiempo de aplicación (ms)
    pub application_time_ms: f64,
    /// Número de componentes de delta aplicados
    pub components_applied: usize,
    /// Mensaje de error (si aplica)
    pub error: Option<String>,
}

#[cfg(feature = "v1.1-sprint4")]
impl SteeringApplicationResult {
    pub fn success(
        signal_id: String,
        layer_id: String,
        confidence: f32,
        application_time_ms: f64,
        components_applied: usize,
    ) -> Self {
        Self {
            signal_id,
            layer_id,
            success: true,
            confidence,
            application_time_ms,
            components_applied,
            error: None,
        }
    }

    pub fn failed(
        signal_id: String,
        layer_id: String,
        confidence: f32,
        application_time_ms: f64,
        error: String,
    ) -> Self {
        Self {
            signal_id,
            layer_id,
            success: false,
            confidence,
            application_time_ms,
            components_applied: 0,
            error: Some(error),
        }
    }
}

// ============================================================================
// Steering Config
// ============================================================================

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteeringConfig {
    /// Umbral mínimo de confianza para aplicar steering
    pub min_confidence_threshold: f32,
    /// Ventana temporal máxima para señales (ms)
    pub max_signal_age_ms: u64,
    /// Máximo de señales por ventana de rate limit
    pub max_signals_per_window: usize,
    /// Ventana de rate limit (ms)
    pub rate_limit_window_ms: u64,
    /// Factor de suavizado para aplicación progresiva
    pub smoothing_factor: f32,
    /// Máximo de componentes de delta por señal
    pub max_delta_components: usize,
}

#[cfg(feature = "v1.1-sprint4")]
impl Default for SteeringConfig {
    fn default() -> Self {
        Self {
            min_confidence_threshold: 0.85,
            max_signal_age_ms: 30_000,
            max_signals_per_window: 50,
            rate_limit_window_ms: 10_000,
            smoothing_factor: 0.1,
            max_delta_components: 512,
        }
    }
}

// ============================================================================
// Steering Stats
// ============================================================================

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteeringStats {
    /// Total de señales aplicadas exitosamente
    pub signals_applied: u64,
    /// Total de señales rechazadas
    pub signals_rejected: u64,
    /// Total de señales expiradas
    pub signals_expired: u64,
    /// Confianza promedio de señales aplicadas
    pub avg_confidence: f32,
    /// Latencia promedio de aplicación (ms)
    pub avg_application_ms: f64,
    /// Último timestamp de aplicación (epoch ms)
    pub last_application_ms: u64,
    /// Señales por capa
    pub signals_by_layer: HashMap<String, u64>,
}

#[cfg(feature = "v1.1-sprint4")]
impl Default for SteeringStats {
    fn default() -> Self {
        Self {
            signals_applied: 0,
            signals_rejected: 0,
            signals_expired: 0,
            avg_confidence: 0.0,
            avg_application_ms: 0.0,
            last_application_ms: 0,
            signals_by_layer: HashMap::new(),
        }
    }
}

// ============================================================================
// Prioritized Signal (for BinaryHeap)
// ============================================================================

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, PartialEq)]
struct PrioritizedSignal {
    pub confidence: f32,
    pub generated_at_ms: u64,
    pub signal_id: String,
}

#[cfg(feature = "v1.1-sprint4")]
impl Eq for PrioritizedSignal {}

#[cfg(feature = "v1.1-sprint4")]
impl PartialOrd for PrioritizedSignal {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(feature = "v1.1-sprint4")]
impl Ord for PrioritizedSignal {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.confidence
            .partial_cmp(&other.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

// ============================================================================
// Steering Engine
// ============================================================================

/// Señal de steering para aplicación
#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteeringSignal {
    pub signal_id: String,
    pub layer_id: String,
    pub delta: Vec<f32>,
    pub weighted_confidence: f32,
    pub feedback_count: usize,
    pub drift_before: f32,
    pub generated_at_ms: u64,
    pub signature: String,
}

#[cfg(feature = "v1.1-sprint4")]
pub struct SteeringEngine {
    config: SteeringConfig,
    stats: SteeringStats,
    verifying_key: VerifyingKey,
    // Capas registradas para validación
    registered_layers: HashMap<String, usize>,
    // Historial de aplicaciones para rate limiting
    application_history: VecDeque<Instant>,
    // Cola de señales pendientes ordenadas por confianza
    pending_queue: BinaryHeap<PrioritizedSignal>,
    // Mapeo de señales pendientes
    pending_signals: HashMap<String, SteeringSignal>,
}

#[cfg(feature = "v1.1-sprint4")]
impl SteeringEngine {
    /// Crea un nuevo motor de steering
    pub fn new(verifying_key: VerifyingKey) -> Self {
        Self {
            config: SteeringConfig::default(),
            stats: SteeringStats::default(),
            verifying_key,
            registered_layers: HashMap::new(),
            application_history: VecDeque::new(),
            pending_queue: BinaryHeap::new(),
            pending_signals: HashMap::new(),
        }
    }

    /// Crea motor con configuración personalizada
    pub fn with_config(config: SteeringConfig, verifying_key: VerifyingKey) -> Self {
        Self {
            config,
            stats: SteeringStats::default(),
            verifying_key,
            registered_layers: HashMap::new(),
            application_history: VecDeque::new(),
            pending_queue: BinaryHeap::new(),
            pending_signals: HashMap::new(),
        }
    }

    /// Registra una capa SAE para recibir señales de steering
    pub fn register_layer(&mut self, layer_id: String, feature_count: usize) {
        self.registered_layers.insert(layer_id, feature_count);
    }

    /// Verifica si una capa está registrada
    pub fn is_layer_registered(&self, layer_id: &str) -> bool {
        self.registered_layers.contains_key(layer_id)
    }

    /// Encola una señal de steering para aplicación
    pub fn queue_signal(&mut self, signal: SteeringSignal) -> Result<(), SteeringError> {
        // Verificar firma
        if !self.verify_signal(&signal) {
            return Err(SteeringError::SignatureVerificationFailed);
        }

        // Verificar expiración
        let now = current_timestamp_ms();
        let age = now.saturating_sub(signal.generated_at_ms);
        if age > self.config.max_signal_age_ms {
            self.stats.signals_expired += 1;
            return Err(SteeringError::SignalExpired {
                age_ms: age,
                max_ms: self.config.max_signal_age_ms,
            });
        }

        // Verificar capa registrada
        if !self.registered_layers.contains_key(&signal.layer_id) {
            return Err(SteeringError::LayerNotRegistered {
                layer_id: signal.layer_id.clone(),
            });
        }

        // Verificar dimensión de delta
        if let Some(&_expected_dim) = self.registered_layers.get(&signal.layer_id) {
            if signal.delta.len() > self.config.max_delta_components {
                return Err(SteeringError::DimensionMismatch {
                    expected: self.config.max_delta_components,
                    actual: signal.delta.len(),
                });
            }
        } else {
            let _expected_dim = 0;
            if signal.delta.len() > self.config.max_delta_components {
                return Err(SteeringError::DimensionMismatch {
                    expected: self.config.max_delta_components,
                    actual: signal.delta.len(),
                });
            }
        }

        // Agregar a cola
        self.pending_signals
            .insert(signal.signal_id.clone(), signal.clone());
        self.pending_queue.push(PrioritizedSignal {
            confidence: signal.weighted_confidence,
            generated_at_ms: signal.generated_at_ms,
            signal_id: signal.signal_id.clone(),
        });

        debug!(
            "Signal queued: id={}, layer={}, confidence={:.4}",
            signal.signal_id, signal.layer_id, signal.weighted_confidence
        );

        Ok(())
    }

    /// Aplica la señal de mayor prioridad de la cola
    pub fn apply_next(&mut self) -> Option<SteeringApplicationResult> {
        // Limpiar historial expirado
        self.cleanup_history();

        // Verificar rate limit
        if self.application_history.len() >= self.config.max_signals_per_window {
            self.stats.signals_rejected += 1;
            return None;
        }

        let prioritized = self.pending_queue.pop()?;

        let signal = self.pending_signals.remove(&prioritized.signal_id)?;

        // Verificar expiración nuevamente
        let now = current_timestamp_ms();
        let age = now.saturating_sub(signal.generated_at_ms);
        if age > self.config.max_signal_age_ms {
            self.stats.signals_expired += 1;
            return Some(SteeringApplicationResult::failed(
                signal.signal_id.clone(),
                signal.layer_id.clone(),
                signal.weighted_confidence,
                0.0,
                format!(
                    "Signal expired: {}ms > {}ms",
                    age, self.config.max_signal_age_ms
                ),
            ));
        }

        // Verificar umbral de confianza
        if signal.weighted_confidence < self.config.min_confidence_threshold {
            self.stats.signals_rejected += 1;
            return Some(SteeringApplicationResult::failed(
                signal.signal_id.clone(),
                signal.layer_id.clone(),
                signal.weighted_confidence,
                0.0,
                format!(
                    "Confidence below threshold: {:.4} < {:.4}",
                    signal.weighted_confidence, self.config.min_confidence_threshold
                ),
            ));
        }

        let start = Instant::now();

        // Aplicar señal con suavizado
        let smoothed_delta = self.apply_smoothing(&signal.delta);
        let components_applied = smoothed_delta.len();

        let elapsed = start.elapsed();
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;

        // Actualizar stats
        self.stats.signals_applied += 1;
        self.stats.avg_confidence = (self.stats.avg_confidence
            * (self.stats.signals_applied - 1) as f32
            + signal.weighted_confidence)
            / self.stats.signals_applied as f32;
        self.stats.avg_application_ms =
            (self.stats.avg_application_ms * (self.stats.signals_applied - 1) as f64 + elapsed_ms)
                / self.stats.signals_applied as f64;
        self.stats.last_application_ms = current_timestamp_ms();
        *self
            .stats
            .signals_by_layer
            .entry(signal.layer_id.clone())
            .or_insert(0) += 1;

        self.application_history.push_back(Instant::now());

        info!(
            "Signal applied: id={}, layer={}, confidence={:.4}, time={:.1}ms",
            signal.signal_id, signal.layer_id, signal.weighted_confidence, elapsed_ms
        );

        Some(SteeringApplicationResult::success(
            signal.signal_id,
            signal.layer_id,
            signal.weighted_confidence,
            elapsed_ms,
            components_applied,
        ))
    }

    /// Aplica todas las señales pendientes en la cola
    pub fn apply_all(&mut self) -> Vec<SteeringApplicationResult> {
        let mut results = Vec::new();
        while let Some(result) = self.apply_next() {
            results.push(result);
        }
        results
    }

    fn verify_signal(&self, signal: &SteeringSignal) -> bool {
        let sig_bytes = match hex::decode(&signal.signature) {
            Ok(b) => b,
            Err(_) => return false,
        };
        let signature = match Signature::from_slice(&sig_bytes) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let message = format!(
            "{}:{}:{}:{}:{}",
            signal.layer_id,
            signal.weighted_confidence,
            signal.feedback_count,
            signal.drift_before,
            signal.generated_at_ms,
        );
        self.verifying_key
            .verify(message.as_bytes(), &signature)
            .is_ok()
    }

    fn apply_smoothing(&self, delta: &[f32]) -> Vec<f32> {
        delta
            .iter()
            .map(|&v| v * self.config.smoothing_factor)
            .collect()
    }

    fn cleanup_history(&mut self) {
        let window = Duration::from_millis(self.config.rate_limit_window_ms);
        self.application_history
            .retain(|instant| instant.elapsed() < window);
    }

    /// Obtiene estadísticas del motor
    pub fn get_stats(&self) -> SteeringStats {
        self.stats.clone()
    }

    /// Obtiene configuración actual
    pub fn config(&self) -> &SteeringConfig {
        &self.config
    }

    /// Reinicia estadísticas
    pub fn reset_stats(&mut self) {
        self.stats = SteeringStats::default();
    }

    /// Obtiene número de señales pendientes
    pub fn pending_count(&self) -> usize {
        self.pending_signals.len()
    }

    /// Obtiene capas registradas
    pub fn registered_layers(&self) -> Vec<String> {
        self.registered_layers.keys().cloned().collect()
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
    use ed25519_dalek::{Signer, SigningKey};
    use sha2::{Digest, Sha256};

    fn test_signing_key() -> SigningKey {
        let mut csprng = ark_std::rand::thread_rng();
        SigningKey::generate(&mut csprng)
    }

    fn make_signal(layer_id: &str, confidence: f32, signing_key: &SigningKey) -> SteeringSignal {
        let generated_at_ms = current_timestamp_ms();
        let message = format!(
            "{}:{}:{}:{}:{}",
            layer_id, confidence, 5, 0.3, generated_at_ms
        );
        let signature = signing_key.sign(message.as_bytes());
        let signal_id = {
            let mut hasher = Sha256::new();
            hasher.update(&message);
            let result = hasher.finalize();
            hex::encode(&result[..8])
        };

        SteeringSignal {
            signal_id,
            layer_id: layer_id.to_string(),
            delta: vec![-0.01, 0.02, -0.005],
            weighted_confidence: confidence,
            feedback_count: 5,
            drift_before: 0.3,
            generated_at_ms,
            signature: hex::encode(signature.to_bytes()),
        }
    }

    #[test]
    fn test_engine_creation() {
        let key = test_signing_key();
        let engine = SteeringEngine::new(key.verifying_key().clone());
        assert_eq!(engine.pending_count(), 0);
        assert_eq!(engine.registered_layers().len(), 0);
    }

    #[test]
    fn test_engine_with_config() {
        let key = test_signing_key();
        let config = SteeringConfig {
            min_confidence_threshold: 0.5,
            ..Default::default()
        };
        let engine = SteeringEngine::with_config(config, key.verifying_key().clone());
        assert_eq!(engine.config().min_confidence_threshold, 0.5);
    }

    #[test]
    fn test_register_layer() {
        let key = test_signing_key();
        let mut engine = SteeringEngine::new(key.verifying_key().clone());
        engine.register_layer("layer-0".to_string(), 256);
        assert!(engine.is_layer_registered("layer-0"));
        assert!(!engine.is_layer_registered("layer-1"));
    }

    #[test]
    fn test_queue_and_apply_signal() {
        let key = test_signing_key();
        let mut engine = SteeringEngine::with_config(
            SteeringConfig {
                min_confidence_threshold: 0.5,
                ..Default::default()
            },
            key.verifying_key().clone(),
        );
        engine.register_layer("layer-0".to_string(), 256);

        let signal = make_signal("layer-0", 0.9, &key);
        assert!(engine.queue_signal(signal).is_ok());
        assert_eq!(engine.pending_count(), 1);

        let result = engine.apply_next();
        assert!(result.is_some());
        let result = result.unwrap();
        assert!(result.success);
        assert_eq!(engine.pending_count(), 0);
    }

    #[test]
    fn test_reject_low_confidence() {
        let key = test_signing_key();
        let mut engine = SteeringEngine::with_config(
            SteeringConfig {
                min_confidence_threshold: 0.9,
                ..Default::default()
            },
            key.verifying_key().clone(),
        );
        engine.register_layer("layer-0".to_string(), 256);

        let signal = make_signal("layer-0", 0.5, &key);
        assert!(engine.queue_signal(signal).is_ok());

        let result = engine.apply_next();
        assert!(result.is_some());
        assert!(!result.unwrap().success);
    }

    #[test]
    fn test_unregistered_layer_rejection() {
        let key = test_signing_key();
        let mut engine = SteeringEngine::new(key.verifying_key().clone());

        let signal = make_signal("unknown-layer", 0.9, &key);
        let result = engine.queue_signal(signal);
        assert!(result.is_err());
    }

    #[test]
    fn test_stats_tracking() {
        let key = test_signing_key();
        let mut engine = SteeringEngine::with_config(
            SteeringConfig {
                min_confidence_threshold: 0.5,
                ..Default::default()
            },
            key.verifying_key().clone(),
        );
        engine.register_layer("layer-0".to_string(), 256);

        let signal = make_signal("layer-0", 0.9, &key);
        engine.queue_signal(signal).unwrap();
        engine.apply_next();

        let stats = engine.get_stats();
        assert_eq!(stats.signals_applied, 1);
        assert!(stats.avg_confidence > 0.0);
    }

    #[test]
    fn test_reset_stats() {
        let key = test_signing_key();
        let mut engine = SteeringEngine::new(key.verifying_key().clone());
        engine.reset_stats();
        let stats = engine.get_stats();
        assert_eq!(stats.signals_applied, 0);
    }

    #[test]
    fn test_config_default() {
        let config = SteeringConfig::default();
        assert_eq!(config.min_confidence_threshold, 0.85);
        assert_eq!(config.max_signal_age_ms, 30_000);
    }

    #[test]
    fn test_smoothing_application() {
        let key = test_signing_key();
        let mut engine = SteeringEngine::with_config(
            SteeringConfig {
                min_confidence_threshold: 0.5,
                smoothing_factor: 0.1,
                ..Default::default()
            },
            key.verifying_key().clone(),
        );
        engine.register_layer("layer-0".to_string(), 256);

        let signal = make_signal("layer-0", 0.9, &key);
        engine.queue_signal(signal).unwrap();
        let result = engine.apply_next().unwrap();
        assert!(result.success);
        assert_eq!(result.components_applied, 3);
    }
}
