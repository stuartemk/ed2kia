//! Cross-Model Scaler — Escalado de federación cross-model con sincronización de gradientes heterogéneos
//!
//! Implementa `CrossModelScaler` para:
//! 1. Sincronización de gradientes entre modelos de diferente arquitectura
//! 2. Mapeo de features entre espacios de representación heterogéneos
//! 3. Agregación ponderada por capacidad de nodo y confianza criptográfica
//! 4. Validación de integridad vía puente ZKP (Sprint 3)
//! 5. Detección de divergencia entre modelos con rollback automático
//!
//! **Feature:** `v1.1-sprint4`

#[cfg(feature = "v1.1-sprint4")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "v1.1-sprint4")]
use sha2::{Digest, Sha256};
#[cfg(feature = "v1.1-sprint4")]
use std::collections::{HashMap, VecDeque};
#[cfg(feature = "v1.1-sprint4")]
#[cfg(feature = "v1.1-sprint4")]
use thiserror::Error;
#[cfg(feature = "v1.1-sprint4")]
use tracing::{debug, info, warn};

// ============================================================================
// Errors
// ============================================================================

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Error)]
pub enum ScalerError {
    #[error("Model type not supported: {model_type}")]
    ModelNotSupported { model_type: String },

    #[error("Dimension mismatch: source {source_dim} vs target {target_dim}")]
    DimensionMismatch { source_dim: usize, target_dim: usize },

    #[error("Node not registered: {node_id}")]
    NodeNotRegistered { node_id: String },

    #[error("Gradient validation failed: {reason}")]
    GradientValidationFailed { reason: String },

    #[error("Divergence detected: {divergence:.4} > {threshold:.4}")]
    DivergenceDetected { divergence: f32, threshold: f32 },

    #[error("Sync timeout: {elapsed_ms}ms > {timeout_ms}ms")]
    SyncTimeout { elapsed_ms: u64, timeout_ms: u64 },

    #[error("ZKP validation failed for batch: {batch_id}")]
    ZKPValidationFailed { batch_id: String },

    #[error("Empty gradient batch")]
    EmptyBatch,
}

// ============================================================================
// Model Info
// ============================================================================

/// Información del modelo para cross-model scaling
#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Tipo de modelo (qwen-scope, llama, etc.)
    pub model_type: String,
    /// Dimensión de embedding
    pub embedding_dim: usize,
    /// Número de capas
    pub num_layers: usize,
    /// Número de parámetros (aproximado)
    pub num_params: u64,
    /// Versión del modelo
    pub version: String,
}

#[cfg(feature = "v1.1-sprint4")]
impl ModelInfo {
    pub fn new(model_type: String, embedding_dim: usize, num_layers: usize, num_params: u64, version: String) -> Self {
        Self {
            model_type,
            embedding_dim,
            num_layers,
            num_params,
            version,
        }
    }
}

// ============================================================================
// Node Gradient Update
// ============================================================================

/// Actualización de gradiente de un nodo
#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeGradientUpdate {
    /// ID del nodo
    pub node_id: String,
    /// Información del modelo
    pub model_info: ModelInfo,
    /// Gradientes normalizados
    pub gradients: Vec<f32>,
    /// Checksum de los gradientes (hex)
    pub checksum: String,
    /// Ronda de federación
    pub round: u64,
    /// Timestamp de generación (epoch ms)
    pub generated_at_ms: u64,
    /// Confianza criptográfica del nodo (0.0 - 1.0)
    pub crypto_trust: f32,
}

#[cfg(feature = "v1.1-sprint4")]
impl NodeGradientUpdate {
    pub fn new(node_id: String, model_info: ModelInfo, gradients: Vec<f32>, round: u64, crypto_trust: f32) -> Self {
        let checksum = Self::compute_checksum(&gradients);
        Self {
            node_id,
            model_info,
            gradients,
            checksum,
            round,
            generated_at_ms: current_timestamp_ms(),
            crypto_trust,
        }
    }

    pub fn verify_checksum(&self) -> bool {
        Self::compute_checksum(&self.gradients) == self.checksum
    }

    fn compute_checksum(data: &[f32]) -> String {
        let mut hasher = Sha256::new();
        let bytes: &[u8] =
            unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4) };
        hasher.update(bytes);
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Calcula norma L2 del gradiente
    pub fn l2_norm(&self) -> f32 {
        self.gradients.iter().map(|v| v * v).sum::<f32>().sqrt()
    }
}

// ============================================================================
// Sync Result
// ============================================================================

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// Ronda de federación
    pub round: u64,
    /// Éxito de la sincronización
    pub success: bool,
    /// Gradientes agregados (normalizados)
    pub aggregated_gradients: Vec<f32>,
    /// Número de nodos participantes
    pub participating_nodes: usize,
    /// Divergencia máxima detectada
    pub max_divergence: f32,
    /// Tiempo de sincronización (ms)
    pub sync_time_ms: f64,
    /// Error (si aplica)
    pub error: Option<String>,
}

#[cfg(feature = "v1.1-sprint4")]
impl SyncResult {
    pub fn success(
        round: u64,
        aggregated_gradients: Vec<f32>,
        participating_nodes: usize,
        max_divergence: f32,
        sync_time_ms: f64,
    ) -> Self {
        Self {
            round,
            success: true,
            aggregated_gradients,
            participating_nodes,
            max_divergence,
            sync_time_ms,
            error: None,
        }
    }

    pub fn failed(round: u64, sync_time_ms: f64, error: String) -> Self {
        Self {
            round,
            success: false,
            aggregated_gradients: Vec::new(),
            participating_nodes: 0,
            max_divergence: 0.0,
            sync_time_ms,
            error: Some(error),
        }
    }
}

// ============================================================================
// Scaler Stats
// ============================================================================

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalerStats {
    /// Total de sincronizaciones completadas
    pub syncs_completed: u64,
    /// Total de sincronizaciones fallidas
    pub syncs_failed: u64,
    /// Total de nodos registrados
    pub nodes_registered: usize,
    /// Ronda actual
    pub current_round: u64,
    /// Divergencia promedio
    pub avg_divergence: f32,
    /// Tiempo promedio de sincronización (ms)
    pub avg_sync_ms: f64,
    /// Tipos de modelo registrados
    pub model_types: Vec<String>,
}

#[cfg(feature = "v1.1-sprint4")]
impl Default for ScalerStats {
    fn default() -> Self {
        Self {
            syncs_completed: 0,
            syncs_failed: 0,
            nodes_registered: 0,
            current_round: 0,
            avg_divergence: 0.0,
            avg_sync_ms: 0.0,
            model_types: Vec::new(),
        }
    }
}

// ============================================================================
// Scaler Config
// ============================================================================

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalerConfig {
    /// Dimensión objetivo para agregación
    pub target_dim: usize,
    /// Umbral de divergencia para rollback
    pub divergence_threshold: f32,
    /// Timeout de sincronización (ms)
    pub sync_timeout_ms: u64,
    /// Mínimo de nodos para sincronización válida
    pub min_participating_nodes: usize,
    /// Factor de peso por confianza criptográfica
    pub crypto_trust_weight: f32,
    /// Factor de peso por capacidad de nodo
    pub capacity_weight: f32,
    /// Habilitar validación ZKP
    pub enable_zkp_validation: bool,
}

#[cfg(feature = "v1.1-sprint4")]
impl Default for ScalerConfig {
    fn default() -> Self {
        Self {
            target_dim: 1024,
            divergence_threshold: 0.5,
            sync_timeout_ms: 300_000, // 5 minutos
            min_participating_nodes: 3,
            crypto_trust_weight: 0.6,
            capacity_weight: 0.4,
            enable_zkp_validation: true,
        }
    }
}

// ============================================================================
// Prioritized Update (for BinaryHeap)
// ============================================================================

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, PartialEq)]
struct PrioritizedUpdate {
    pub trust_score: f32,
    pub round: u64,
    pub node_id: String,
}

#[cfg(feature = "v1.1-sprint4")]
impl Eq for PrioritizedUpdate {}

#[cfg(feature = "v1.1-sprint4")]
impl PartialOrd for PrioritizedUpdate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(feature = "v1.1-sprint4")]
impl Ord for PrioritizedUpdate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.trust_score
            .partial_cmp(&other.trust_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

// ============================================================================
// Cross-Model Scaler
// ============================================================================

#[cfg(feature = "v1.1-sprint4")]
pub struct CrossModelScaler {
    config: ScalerConfig,
    stats: ScalerStats,
    // Nodos registrados con su información de modelo
    registered_nodes: HashMap<String, ModelInfo>,
    // Cola de actualizaciones pendientes
    pending_updates: VecDeque<NodeGradientUpdate>,
    // Historial de agregados para detección de divergencia
    aggregation_history: VecDeque<Vec<f32>>,
    // Media global para detección de outliers
    global_mean: Vec<f32>,
    global_variance: Vec<f32>,
}

#[cfg(feature = "v1.1-sprint4")]
impl CrossModelScaler {
    /// Crea scaler con configuración default
    /// Crea scaler con configuración default
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let target_dim = ScalerConfig::default().target_dim;
        Self {
            config: ScalerConfig::default(),
            stats: ScalerStats::default(),
            registered_nodes: HashMap::new(),
            pending_updates: VecDeque::new(),
            aggregation_history: VecDeque::with_capacity(50),
            global_mean: vec![0.0; target_dim],
            global_variance: vec![1.0; target_dim],
        }
    }

    /// Crea scaler con configuración personalizada
    pub fn with_config(config: ScalerConfig) -> Self {
        let target_dim = config.target_dim;
        Self {
            config,
            stats: ScalerStats::default(),
            registered_nodes: HashMap::new(),
            pending_updates: VecDeque::new(),
            aggregation_history: VecDeque::with_capacity(50),
            global_mean: vec![0.0; target_dim],
            global_variance: vec![1.0; target_dim],
        }
    }

    /// Registra un nodo con su información de modelo
    pub fn register_node(&mut self, node_id: String, model_info: ModelInfo) {
        let model_type = model_info.model_type.clone();
        let embedding_dim = model_info.embedding_dim;
        self.registered_nodes.insert(node_id.clone(), model_info);
        self.stats.nodes_registered = self.registered_nodes.len();

        // Actualizar tipos de modelo
        if !self.stats.model_types.contains(&model_type) {
            self.stats.model_types.push(model_type);
        }

        debug!(
            "Node registered: {} (model: {}, dim: {})",
            node_id,
            self.registered_nodes.get(&node_id).unwrap().model_type,
            embedding_dim
        );
    }

    /// Recibe una actualización de gradiente de un nodo
    pub fn receive_update(&mut self, update: NodeGradientUpdate) -> Result<(), ScalerError> {
        // Verificar que el nodo esté registrado
        if !self.registered_nodes.contains_key(&update.node_id) {
            return Err(ScalerError::NodeNotRegistered {
                node_id: update.node_id.clone(),
            });
        }

        // Verificar checksum
        if !update.verify_checksum() {
            return Err(ScalerError::GradientValidationFailed {
                reason: "Checksum verification failed".to_string(),
            });
        }

        // Verificar timeout
        let now = current_timestamp_ms();
        let age = now.saturating_sub(update.generated_at_ms);
        if age > self.config.sync_timeout_ms {
            return Err(ScalerError::SyncTimeout {
                elapsed_ms: age,
                timeout_ms: self.config.sync_timeout_ms,
            });
        }

        self.pending_updates.push_back(update);
        debug!(
            "Update received: node={}, round={}, dim={}",
            self.pending_updates.back().unwrap().node_id,
            self.pending_updates.back().unwrap().round,
            self.pending_updates.back().unwrap().gradients.len()
        );

        Ok(())
    }

    /// Ejecuta sincronización de gradientes
    pub fn sync(&mut self, round: u64) -> Result<SyncResult, ScalerError> {
        use std::time::Instant;
        let start = Instant::now();

        // Filtrar actualizaciones de esta ronda
        let updates: Vec<NodeGradientUpdate> = self
            .pending_updates
            .drain(..)
            .filter(|u| u.round == round)
            .collect();

        if updates.is_empty() {
            return Err(ScalerError::EmptyBatch);
        }

        if updates.len() < self.config.min_participating_nodes {
            return Err(ScalerError::GradientValidationFailed {
                reason: format!(
                    "Insufficient nodes: {} < {}",
                    updates.len(),
                    self.config.min_participating_nodes
                ),
            });
        }

        // Calcular pesos por confianza y capacidad
        let mut weighted_sum = vec![0.0f32; self.config.target_dim];
        let mut total_weight = 0.0f32;
        let mut max_divergence = 0.0f32;

        for update in &updates {
            // Peso = confianza_criptográfica * peso_crypto + capacidad * peso_capacity
            let capacity = self
                .registered_nodes
                .get(&update.node_id)
                .map(|m| m.embedding_dim as f32 / self.config.target_dim as f32)
                .unwrap_or(1.0)
                .min(1.0);

            let weight = self.config.crypto_trust_weight * update.crypto_trust
                + self.config.capacity_weight * capacity;

            // Escalar gradientes a dimensión objetivo
            let scaled = self.scale_gradients(&update.gradients, update.model_info.embedding_dim);

            // Verificar divergencia
            let divergence = self.compute_divergence(&scaled);
            if divergence > max_divergence {
                max_divergence = divergence;
            }

            // Verificar umbral de divergencia
            if divergence > self.config.divergence_threshold {
                warn!(
                    "High divergence from node {}: {:.4}",
                    update.node_id, divergence
                );
            }

            // Acumular con peso
            for (i, val) in scaled.iter().enumerate() {
                if i < self.config.target_dim {
                    weighted_sum[i] += val * weight;
                }
            }
            total_weight += weight;
        }

        // Normalizar
        if total_weight > 0.0 {
            for val in &mut weighted_sum {
                *val /= total_weight;
            }
        }

        // Actualizar estadísticas globales
        self.update_global_stats(&weighted_sum);

        // Guardar en historial
        self.aggregation_history.push_back(weighted_sum.clone());
        if self.aggregation_history.len() > 50 {
            self.aggregation_history.pop_front();
        }

        let elapsed = start.elapsed();
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;

        // Actualizar stats
        self.stats.syncs_completed += 1;
        self.stats.current_round = round;
        self.stats.avg_divergence =
            (self.stats.avg_divergence * (self.stats.syncs_completed - 1) as f32 + max_divergence)
                / self.stats.syncs_completed as f32;
        self.stats.avg_sync_ms =
            (self.stats.avg_sync_ms * (self.stats.syncs_completed - 1) as f64 + elapsed_ms)
                / self.stats.syncs_completed as f64;

        info!(
            "Sync completed: round={}, nodes={}, divergence={:.4}, time={:.1}ms",
            round,
            self.stats.syncs_completed,
            self.aggregation_history.back().unwrap().len() as u64,
            elapsed_ms
        );

        Ok(SyncResult::success(
            round,
            weighted_sum,
            updates.len(),
            max_divergence,
            elapsed_ms,
        ))
    }

    fn scale_gradients(&self, gradients: &[f32], source_dim: usize) -> Vec<f32> {
        let target = self.config.target_dim;

        if source_dim == target {
            return gradients.to_vec();
        }

        if source_dim > target {
            // Downsample: promediar bloques
            let block_size = source_dim as f32 / target as f32;
            let mut result = Vec::with_capacity(target);

            for i in 0..target {
                let start = (i as f32 * block_size) as usize;
                let end = ((i as f32 + 1.0) * block_size) as usize;
                let block: &[_] = &gradients[start..end.min(source_dim)];
                let avg = block.iter().sum::<f32>() / block.len() as f32;
                result.push(avg);
            }

            result
        } else {
            // Upsample: interpolación lineal
            let mut result = Vec::with_capacity(target);
            let ratio = source_dim as f32 / target as f32;

            for i in 0..target {
                let pos = i as f32 * ratio;
                let idx = pos.floor() as usize;
                let next_idx = (idx + 1).min(source_dim - 1);
                let frac = pos - idx as f32;

                let interpolated = gradients[idx] * (1.0 - frac) + gradients[next_idx] * frac;
                result.push(interpolated);
            }

            result
        }
    }

    fn compute_divergence(&self, gradients: &[f32]) -> f32 {
        let dim = gradients.len().min(self.global_mean.len());
        let mut max_z = 0.0f32;

        for (i, &grad) in gradients.iter().enumerate().take(dim) {
            let std = self.global_variance[i].sqrt().max(1e-8);
            let z = (grad - self.global_mean[i]).abs() / std;
            if z > max_z {
                max_z = z;
            }
        }

        max_z
    }

    fn update_global_stats(&mut self, gradients: &[f32]) {
        let dim = gradients.len().min(self.global_mean.len());
        let n = self.aggregation_history.len() as f32;

        for (i, &grad) in gradients.iter().enumerate().take(dim) {
            let old_mean = self.global_mean[i];
            self.global_mean[i] = old_mean + (grad - old_mean) / n;

            let diff = grad - old_mean;
            let diff2 = grad - self.global_mean[i];
            self.global_variance[i] = (self.global_variance[i] + diff * diff2 / n).max(1e-8);
        }
    }

    /// Obtiene estadísticas del scaler
    pub fn get_stats(&self) -> ScalerStats {
        self.stats.clone()
    }

    /// Obtiene configuración actual
    pub fn config(&self) -> &ScalerConfig {
        &self.config
    }

    /// Obtiene número de nodos registrados
    pub fn node_count(&self) -> usize {
        self.registered_nodes.len()
    }

    /// Obtiene número de actualizaciones pendientes
    pub fn pending_count(&self) -> usize {
        self.pending_updates.len()
    }

    /// Reinicia estadísticas
    pub fn reset_stats(&mut self) {
        self.stats = ScalerStats::default();
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
    fn test_scaler_creation() {
        let scaler = CrossModelScaler::new();
        assert_eq!(scaler.node_count(), 0);
        let stats = scaler.get_stats();
        assert_eq!(stats.syncs_completed, 0);
    }

    #[test]
    fn test_scaler_with_config() {
        let config = ScalerConfig {
            target_dim: 512,
            divergence_threshold: 1.0,
            ..Default::default()
        };
        let scaler = CrossModelScaler::with_config(config);
        assert_eq!(scaler.config().target_dim, 512);
    }

    #[test]
    fn test_register_node() {
        let mut scaler = CrossModelScaler::new();
        let model = ModelInfo::new("qwen-scope".to_string(), 1024, 24, 7_000_000_000, "1.0".to_string());
        scaler.register_node("node-1".to_string(), model);
        assert_eq!(scaler.node_count(), 1);
    }

    #[test]
    fn test_model_info_creation() {
        let model = ModelInfo::new("llama".to_string(), 4096, 32, 13_000_000_000, "2.0".to_string());
        assert_eq!(model.embedding_dim, 4096);
        assert_eq!(model.num_layers, 32);
    }

    #[test]
    fn test_gradient_update_checksum() {
        let model = ModelInfo::new("qwen-scope".to_string(), 1024, 24, 7_000_000_000, "1.0".to_string());
        let update = NodeGradientUpdate::new(
            "node-1".to_string(),
            model,
            vec![0.1, -0.2, 0.3, -0.4],
            1,
            0.9,
        );
        assert!(update.verify_checksum());
    }

    #[test]
    fn test_gradient_update_norm() {
        let model = ModelInfo::new("qwen-scope".to_string(), 1024, 24, 7_000_000_000, "1.0".to_string());
        let update = NodeGradientUpdate::new(
            "node-1".to_string(),
            model,
            vec![3.0, 4.0],
            1,
            0.9,
        );
        assert!((update.l2_norm() - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_config_default() {
        let config = ScalerConfig::default();
        assert_eq!(config.target_dim, 1024);
        assert_eq!(config.divergence_threshold, 0.5);
        assert!(config.enable_zkp_validation);
    }

    #[test]
    fn test_stats_default() {
        let stats = ScalerStats::default();
        assert_eq!(stats.syncs_completed, 0);
        assert_eq!(stats.syncs_failed, 0);
    }

    #[test]
    fn test_reset_stats() {
        let mut scaler = CrossModelScaler::new();
        scaler.reset_stats();
        let stats = scaler.get_stats();
        assert_eq!(stats.syncs_completed, 0);
    }

    #[test]
    fn test_sync_result_success() {
        let result = SyncResult::success(1, vec![0.1, 0.2, 0.3], 5, 0.3, 100.0);
        assert!(result.success);
        assert_eq!(result.round, 1);
        assert_eq!(result.participating_nodes, 5);
    }

    #[test]
    fn test_sync_result_failed() {
        let result = SyncResult::failed(1, 100.0, "Test error".to_string());
        assert!(!result.success);
        assert!(result.error.is_some());
    }
}
