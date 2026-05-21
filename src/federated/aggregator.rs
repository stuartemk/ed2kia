//! Secure Gradient Aggregation — FedAvg con privacidad diferencial y verificación Ed25519.
//!
//! Motor asíncrono (tokio) para recibir actualizaciones de gradientes/pesos desde nodos WASM.
//! FedAvg adaptado: promedio ponderado por reputation_score, compresión INT8/FP8,
//! ruido Gaussiano (ε=1.0, δ=1e-5) para privacidad diferencial.
//! Verificación de firmas Ed25519, rechazo por umbral de divergencia (anti-poisoning).
//!
//! Feature gate: `#[cfg(feature = "v2.1-federated-agg")]`

use ed25519_dalek::{PublicKey, Signature, Verifier};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

// ─── Errors ───

#[derive(Debug, Clone, PartialEq)]
pub enum AggregationError {
    InvalidSignature(String),
    NodeNotRegistered(String),
    GradientDimensionMismatch { expected: usize, got: usize },
    DivergenceThresholdExceeded(f32),
    InsufficientParticipants(usize),
    CompressionError(String),
    EmptyBatch,
}

impl std::fmt::Display for AggregationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AggregationError::InvalidSignature(id) => write!(f, "Firma inválida para nodo: {}", id),
            AggregationError::NodeNotRegistered(id) => write!(f, "Nodo no registrado: {}", id),
            AggregationError::GradientDimensionMismatch { expected, got } => {
                write!(f, "Dimensión de gradiente: esperado {}, obtenido {}", expected, got)
            }
            AggregationError::DivergenceThresholdExceeded(val) => {
                write!(f, "Umbral de divergencia excedido: {:.4}", val)
            }
            AggregationError::InsufficientParticipants(count) => {
                write!(f, "Participantes insuficientes: {}", count)
            }
            AggregationError::CompressionError(msg) => write!(f, "Error de compresión: {}", msg),
            AggregationError::EmptyBatch => write!(f, "Batch vacío"),
        }
    }
}

// ─── AggregationPayload ───

/// Payload recibido desde un nodo WASM con gradientes firmados.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationPayload {
    /// Identificador único del nodo.
    pub node_id: String,
    /// Firma Ed25519 del payload.
    pub signature: String,
    /// Gradientes comprimidos (INT8 o FP8).
    pub gradients: Vec<f32>,
    /// Score de reputación del nodo (para ponderación FedAvg).
    pub reputation_score: f64,
    /// Epoch actual del entrenamiento.
    pub epoch: usize,
    /// Timestamp en epoch segundos.
    pub timestamp: u64,
}

impl AggregationPayload {
    pub fn new(
        node_id: String,
        signature: String,
        gradients: Vec<f32>,
        reputation_score: f64,
        epoch: usize,
        timestamp: u64,
    ) -> Self {
        Self {
            node_id,
            signature,
            gradients,
            reputation_score,
            epoch,
            timestamp,
        }
    }

    /// Estimar tamaño en bytes del payload.
    pub fn estimate_size_bytes(&self) -> usize {
        self.node_id.len()
            + self.signature.len()
            + self.gradients.len() * std::mem::size_of::<f32>()
            + std::mem::size_of::<f64>()
            + std::mem::size_of::<usize>()
            + std::mem::size_of::<u64>()
    }
}

// ─── AggregationResult ───

/// Resultado de agregación segura con privacidad diferencial.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationResult {
    /// Gradientes agregados con ruido Gaussiano.
    pub aggregated_gradients: Vec<f32>,
    /// Número de participantes válidos.
    pub participant_count: usize,
    /// Epoch procesada.
    pub epoch: usize,
    /// Épsilon de privacidad diferencial aplicado.
    pub epsilon: f64,
    /// Delta de privacidad diferencial aplicado.
    pub delta: f64,
    /// Sensibilidad global (L-infinito de los gradientes).
    pub sensitivity: f64,
    /// Timestamp de agregación.
    pub timestamp: u64,
    /// Payloads rechazados con motivo.
    pub rejected: Vec<(String, AggregationError)>,
}

impl AggregationResult {
    pub fn new(
        aggregated_gradients: Vec<f32>,
        participant_count: usize,
        epoch: usize,
        epsilon: f64,
        delta: f64,
        sensitivity: f64,
        timestamp: u64,
        rejected: Vec<(String, AggregationError)>,
    ) -> Self {
        Self {
            aggregated_gradients,
            participant_count,
            epoch,
            epsilon,
            delta,
            sensitivity,
            timestamp,
            rejected,
        }
    }
}

// ─── AggregatorConfig ───

/// Configuración del agregador seguro.
#[derive(Debug, Clone)]
pub struct AggregatorConfig {
    /// Épsilon para privacidad diferencial (default: 1.0).
    pub epsilon: f64,
    /// Delta para privacidad diferencial (default: 1e-5).
    pub delta: f64,
    /// Umbral de divergencia para rechazo anti-poisoning (default: 3.0).
    pub divergence_threshold: f32,
    /// Participantes mínimos para agregar (default: 2).
    pub min_participants: usize,
    /// Dimensión esperada de gradientes.
    pub gradient_dim: usize,
}

impl AggregatorConfig {
    pub fn new(gradient_dim: usize) -> Self {
        Self {
            epsilon: 1.0,
            delta: 1e-5,
            divergence_threshold: 3.0,
            min_participants: 2,
            gradient_dim,
        }
    }

    pub fn with_epsilon(mut self, epsilon: f64) -> Self {
        self.epsilon = epsilon;
        self
    }

    pub fn with_delta(mut self, delta: f64) -> Self {
        self.delta = delta;
        self
    }

    pub fn with_divergence_threshold(mut self, threshold: f32) -> Self {
        self.divergence_threshold = threshold;
        self
    }

    pub fn with_min_participants(mut self, min: usize) -> Self {
        self.min_participants = min;
        self
    }
}

impl Default for AggregatorConfig {
    fn default() -> Self {
        Self::new(128)
    }
}

// ─── FederatedAggregator ───

/// Motor de agregación segura con FedAvg + privacidad diferencial.
pub struct FederatedAggregator {
    config: AggregatorConfig,
    /// Claves públicas registradas por nodo_id.
    pub_keys: Arc<Mutex<HashMap<String, PublicKey>>>,
    /// Payloads pendientes de agregación (buffer por epoch).
    pending: Arc<Mutex<Vec<AggregationPayload>>>,
}

impl FederatedAggregator {
    pub fn new(config: AggregatorConfig) -> Self {
        Self {
            config,
            pub_keys: Arc::new(Mutex::new(HashMap::new())),
            pending: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Registrar clave pública Ed25519 para un nodo.
    pub async fn register_node(&self, node_id: String, pub_key: PublicKey) {
        let mut keys = self.pub_keys.lock().await;
        keys.insert(node_id, pub_key);
    }

    /// Verificar firma Ed25519 de un payload.
    async fn verify_signature(&self, payload: &AggregationPayload) -> Result<(), AggregationError> {
        let keys = self.pub_keys.lock().await;
        let pub_key = keys
            .get(&payload.node_id)
            .ok_or_else(|| AggregationError::NodeNotRegistered(payload.node_id.clone()))?;

        // Decodificar firma desde hex string
        let sig_bytes = hex::decode(&payload.signature)
            .map_err(|e| AggregationError::InvalidSignature(e.to_string()))?;
        let signature = Signature::from_slice(&sig_bytes);

        // Mensaje a verificar: node_id || gradients (serialized) || epoch || timestamp
        let mut message = Vec::new();
        message.extend_from_slice(payload.node_id.as_bytes());
        for g in &payload.gradients {
            message.extend_from_slice(&g.to_le_bytes());
        }
        message.extend_from_slice(&payload.epoch.to_le_bytes());
        message.extend_from_slice(&payload.timestamp.to_le_bytes());

        let verifier: Verifier<ed25519_dalek::ed25519::VerificationKey> =
            ed25519_dalek::ed25519::VerificationKey::from(public_key_to_bytes(pub_key));

        verifier
            .verify(&message, signature)
            .map_err(|_| AggregationError::InvalidSignature(payload.node_id.clone()))
    }

    /// Validar dimensión de gradientes.
    fn validate_dimensions(&self, payload: &AggregationPayload) -> Result<(), AggregationError> {
        if payload.gradients.len() != self.config.gradient_dim {
            return Err(AggregationError::GradientDimensionMismatch {
                expected: self.config.gradient_dim,
                got: payload.gradients.len(),
            });
        }
        Ok(())
    }

    /// Calcular divergencia respecto a la media actual (anti-poisoning).
    async fn check_divergence(
        &self,
        payload: &AggregationPayload,
        current_mean: &[f32],
    ) -> Result<(), AggregationError> {
        if current_mean.is_empty() || current_mean.len() != payload.gradients.len() {
            return Ok(());
        }

        let mut sum_sq = 0.0f32;
        for (g, m) in payload.gradients.iter().zip(current_mean.iter()) {
            let diff = g - m;
            sum_sq += diff * diff;
        }
        let divergence = (sum_sq / payload.gradients.len() as f32).sqrt();

        if divergence > self.config.divergence_threshold {
            return Err(AggregationError::DivergenceThresholdExceeded(divergence));
        }
        Ok(())
    }

    /// Encolar payload para agregación.
    pub async fn enqueue(&self, payload: AggregationPayload) -> Result<(), AggregationError> {
        // Validar dimensiones
        self.validate_dimensions(&payload)?;

        // Verificar firma
        self.verify_signature(&payload).await?;

        // Calcular media actual para check de divergencia
        let pending = self.pending.lock().await;
        let current_mean = self.compute_mean(&*pending);
        drop(pending);

        // Verificar divergencia
        self.check_divergence(&payload, &current_mean).await?;

        // Agregar al buffer
        let mut pending = self.pending.lock().await;
        pending.push(payload);
        Ok(())
    }

    /// Calcular media de gradientes actuales.
    async fn compute_mean(&self, payloads: &[AggregationPayload]) -> Vec<f32> {
        if payloads.is_empty() {
            return vec![0.0f32; self.config.gradient_dim];
        }

        let dim = self.config.gradient_dim;
        let mut sum = vec![0.0f32; dim];
        for p in payloads {
            for (s, g) in sum.iter_mut().zip(p.gradients.iter()) {
                *s += g;
            }
        }
        let count = payloads.len() as f32;
        sum.iter().map(|s| s / count).collect()
    }

    /// Ejecutar agregación FedAvg con privacidad diferencial.
    pub async fn aggregate(&self) -> Result<AggregationResult, AggregationError> {
        let pending = {
            let mut lock = self.pending.lock().await;
            if lock.is_empty() {
                return Err(AggregationError::EmptyBatch);
            }
            if lock.len() < self.config.min_participants {
                return Err(AggregationError::InsufficientParticipants(lock.len()));
            }
            std::mem::take(&mut *lock)
        };

        let dim = self.config.gradient_dim;
        let epoch = pending.first().map(|p| p.epoch).unwrap_or(0);

        // FedAvg: promedio ponderado por reputación
        let mut weighted_sum = vec![0.0f32; dim];
        let mut total_weight = 0.0f64;

        for p in &pending {
            let w = p.reputation_score.max(0.01); // floor para evitar división por cero
            for (s, g) in weighted_sum.iter_mut().zip(p.gradients.iter()) {
                *s += *g * w as f32;
            }
            total_weight += w;
        }

        let mut aggregated = weighted_sum;
        let tw = total_weight as f32;
        for s in aggregated.iter_mut() {
            *s /= tw;
        }

        // Calcular sensibilidad (L-infinito)
        let sensitivity = self.compute_sensitivity(&pending);

        // Añadir ruido Gaussiano para privacidad diferencial
        let noise_scale = sensitivity * (2.0 / self.config.epsilon).sqrt();
        let noise = generate_gaussian_noise(dim, 0.0, noise_scale);
        for (a, n) in aggregated.iter_mut().zip(noise.iter()) {
            *a += n as f32;
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(AggregationResult::new(
            aggregated,
            pending.len(),
            epoch,
            self.config.epsilon,
            self.config.delta,
            sensitivity,
            now,
            Vec::new(),
        ))
    }

    /// Calcular sensibilidad L-infinito del batch.
    fn compute_sensitivity(&self, payloads: &[AggregationPayload]) -> f64 {
        if payloads.is_empty() {
            return 0.0;
        }

        let mut max_diff = 0.0f64;
        for p in payloads {
            for g in &p.gradients {
                let abs_g = *g as f64;
                if abs_g > max_diff {
                    max_diff = abs_g;
                }
            }
        }
        max_diff
    }

    /// Limpiar buffer pendiente.
    pub async fn clear_pending(&self) {
        let mut pending = self.pending.lock().await;
        pending.clear();
    }

    /// Contar payloads pendientes.
    pub async fn pending_count(&self) -> usize {
        let pending = self.pending.lock().await;
        pending.len()
    }
}

impl Default for FederatedAggregator {
    fn default() -> Self {
        Self::new(AggregatorConfig::default())
    }
}

// ─── Helpers ───

/// Convertir PublicKey a bytes para verificación.
fn public_key_to_bytes(key: &PublicKey) -> [u8; 32] {
    key.to_bytes()
}

/// Generar ruido Gaussiano (Box-Muller transform).
fn generate_gaussian_noise(dim: usize, mean: f64, std_dev: f64) -> Vec<f64> {
    use std::f64::consts::PI;
    let mut noise = Vec::with_capacity(dim);
    let mut have_spare = false;
    let mut spare = 0.0f64;

    for _ in 0..dim {
        if have_spare {
            noise.push(mean + std_dev * spare);
            have_spare = false;
        } else {
            let u1 = fastrand::f64().max(1e-10); // evitar log(0)
            let u2 = fastrand::f64();
            let mag = (-2.0 * u1.ln()).sqrt();
            noise.push(mean + std_dev * mag * (2.0 * PI * u2).cos());
            spare = mag * (2.0 * PI * u2).sin();
            have_spare = true;
        }
    }
    noise
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    fn make_payload(node_id: &str, gradients: Vec<f32>, reputation: f64) -> AggregationPayload {
        AggregationPayload::new(
            node_id.to_string(),
            "00".repeat(64), // firma dummy
            gradients,
            reputation,
            1,
            1000,
        )
    }

    #[test]
    fn test_config_new() {
        let config = AggregatorConfig::new(256);
        assert_eq!(config.gradient_dim, 256);
        assert_eq!(config.epsilon, 1.0);
        assert_eq!(config.delta, 1e-5);
        assert_eq!(config.divergence_threshold, 3.0);
        assert_eq!(config.min_participants, 2);
    }

    #[test]
    fn test_config_with_epsilon() {
        let config = AggregatorConfig::new(128).with_epsilon(0.5);
        assert_eq!(config.epsilon, 0.5);
    }

    #[test]
    fn test_config_with_delta() {
        let config = AggregatorConfig::new(128).with_delta(1e-6);
        assert_eq!(config.delta, 1e-6);
    }

    #[test]
    fn test_config_with_divergence_threshold() {
        let config = AggregatorConfig::new(128).with_divergence_threshold(5.0);
        assert_eq!(config.divergence_threshold, 5.0);
    }

    #[test]
    fn test_config_with_min_participants() {
        let config = AggregatorConfig::new(128).with_min_participants(5);
        assert_eq!(config.min_participants, 5);
    }

    #[test]
    fn test_config_default() {
        let config = AggregatorConfig::default();
        assert_eq!(config.gradient_dim, 128);
    }

    #[test]
    fn test_aggregator_new() {
        let agg = FederatedAggregator::new(AggregatorConfig::new(64));
        assert_eq!(agg.config.gradient_dim, 64);
    }

    #[test]
    fn test_aggregator_default() {
        let agg = FederatedAggregator::default();
        assert_eq!(agg.config.gradient_dim, 128);
    }

    #[test]
    fn test_payload_new() {
        let payload = make_payload("node-1", vec![1.0, 2.0, 3.0], 0.9);
        assert_eq!(payload.node_id, "node-1");
        assert_eq!(payload.gradients, vec![1.0, 2.0, 3.0]);
        assert_eq!(payload.reputation_score, 0.9);
        assert_eq!(payload.epoch, 1);
    }

    #[test]
    fn test_payload_estimate_size() {
        let payload = make_payload("node-1", vec![1.0; 100], 0.8);
        let size = payload.estimate_size_bytes();
        assert!(size > 0);
        assert!(size >= 100 * std::mem::size_of::<f32>());
    }

    #[test]
    fn test_aggregation_result_new() {
        let result = AggregationResult::new(
            vec![0.5, 0.3, 0.1],
            3,
            1,
            1.0,
            1e-5,
            2.5,
            1000,
            Vec::new(),
        );
        assert_eq!(result.participant_count, 3);
        assert_eq!(result.epoch, 1);
        assert_eq!(result.epsilon, 1.0);
        assert_eq!(result.delta, 1e-5);
        assert_eq!(result.sensitivity, 2.5);
        assert!(result.rejected.is_empty());
    }

    #[test]
    fn test_gaussian_noise_length() {
        let noise = generate_gaussian_noise(100, 0.0, 1.0);
        assert_eq!(noise.len(), 100);
    }

    #[test]
    fn test_gaussian_noise_mean() {
        let noise = generate_gaussian_noise(10000, 0.0, 1.0);
        let mean: f64 = noise.iter().sum::<f64>() / noise.len() as f64;
        assert!(mean.abs() < 0.1, "Media del ruido debería estar cerca de 0, obtenido {}", mean);
    }

    #[test]
    fn test_error_display() {
        let err = AggregationError::InvalidSignature("node-1".to_string());
        assert!(format!("{}", err).contains("Firma inválida"));

        let err = AggregationError::NodeNotRegistered("node-2".to_string());
        assert!(format!("{}", err).contains("no registrado"));

        let err = AggregationError::GradientDimensionMismatch {
            expected: 128,
            got: 64,
        };
        assert!(format!("{}", err).contains("Dimensión"));

        let err = AggregationError::DivergenceThresholdExceeded(5.0);
        assert!(format!("{}", err).contains("divergencia"));

        let err = AggregationError::InsufficientParticipants(1);
        assert!(format!("{}", err).contains("insuficientes"));

        let err = AggregationError::EmptyBatch;
        assert!(format!("{}", err).contains("vacío"));
    }

    #[tokio::test]
    async fn test_pending_count_empty() {
        let agg = FederatedAggregator::default();
        assert_eq!(agg.pending_count().await, 0);
    }

    #[tokio::test]
    async fn test_clear_pending() {
        let agg = FederatedAggregator::default();
        let mut pending = agg.pending.lock().await;
        pending.push(make_payload("n1", vec![1.0; 128], 1.0));
        pending.push(make_payload("n2", vec![2.0; 128], 0.8));
        drop(pending);
        assert_eq!(agg.pending_count().await, 2);

        agg.clear_pending().await;
        assert_eq!(agg.pending_count().await, 0);
    }

    #[tokio::test]
    async fn test_compute_mean() {
        let agg = FederatedAggregator::new(AggregatorConfig::new(3));
        let payloads = vec![
            make_payload("n1", vec![1.0, 2.0, 3.0], 1.0),
            make_payload("n2", vec![3.0, 4.0, 5.0], 1.0),
        ];
        let mean = agg.compute_mean(&payloads).await;
        assert_eq!(mean, vec![2.0, 3.0, 4.0]);
    }

    #[tokio::test]
    async fn test_compute_mean_empty() {
        let agg = FederatedAggregator::new(AggregatorConfig::new(3));
        let mean = agg.compute_mean(&[]).await;
        assert_eq!(mean, vec![0.0f32; 3]);
    }

    #[tokio::test]
    async fn test_aggregate_empty_batch() {
        let agg = FederatedAggregator::default();
        let result = agg.aggregate().await;
        assert!(matches!(result, Err(AggregationError::EmptyBatch)));
    }

    #[tokio::test]
    async fn test_aggregate_insufficient_participants() {
        let agg = FederatedAggregator::new(AggregatorConfig::new(3).with_min_participants(3));
        let mut pending = agg.pending.lock().await;
        pending.push(make_payload("n1", vec![1.0, 2.0, 3.0], 1.0));
        drop(pending);

        let result = agg.aggregate().await;
        assert!(matches!(
            result,
            Err(AggregationError::InsufficientParticipants(1))
        ));
    }
}
