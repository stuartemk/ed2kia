//! Gradient Normalizer — Normalización de gradientes heterogéneos para federación cross-model
//!
//! Implementa `GradientNormalizer` para:
//! 1. Normalización de gradientes por capacidad de nodo (FLOPS, memoria, etc.)
//! 2. Escalado de gradientes entre modelos de diferente dimensión
//! 3. Validación de integridad de gradientes con checksums
//! 4. Detección de gradientes maliciosos (outlier detection)
//! 5. Agregación ponderada por confianza criptográfica
//!
//! **Feature:** `v1.1-sprint4`

#[cfg(feature = "v1.1-sprint4")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "v1.1-sprint4")]
use sha2::{Digest, Sha256};
#[cfg(feature = "v1.1-sprint4")]
use std::collections::HashMap;
#[cfg(feature = "v1.1-sprint4")]
use thiserror::Error;
#[cfg(feature = "v1.1-sprint4")]
use tracing::{debug, warn};

// ============================================================================
// Errors
// ============================================================================

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Error)]
pub enum NormalizerError {
    #[error("Invalid gradient dimensions: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("Gradient checksum verification failed for node: {node_id}")]
    ChecksumFailed { node_id: String },

    #[error("Malicious gradient detected from node: {node_id} (outlier score: {score:.4})")]
    MaliciousGradient { node_id: String, score: f32 },

    #[error("Node capacity invalid: {node_id} (capacity must be > 0)")]
    InvalidCapacity { node_id: String },

    #[error("Empty gradient batch provided")]
    EmptyBatch,

    #[error("Normalization factor overflow: {value}")]
    Overflow { value: f64 },
}

// ============================================================================
// Node Capacity Info
// ============================================================================

/// Información de capacidad del nodo para normalización
#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapacity {
    /// ID del nodo
    pub node_id: String,
    /// Capacidad de cómputo en FLOPS (normalizado 0.0 - 1.0)
    pub compute_capacity: f32,
    /// Memoria disponible en GB (normalizado 0.0 - 1.0)
    pub memory_capacity: f32,
    /// Ancho de banda en Mbps (normalizado 0.0 - 1.0)
    pub bandwidth_capacity: f32,
    /// Dimensión del modelo local
    pub model_dim: usize,
    /// Tipo de modelo (para cross-model scaling)
    pub model_type: String,
}

#[cfg(feature = "v1.1-sprint4")]
impl NodeCapacity {
    /// Calcula capacidad ponderada promedio
    pub fn weighted_capacity(&self) -> f32 {
        let compute_weight = 0.4;
        let memory_weight = 0.35;
        let bandwidth_weight = 0.25;

        compute_weight * self.compute_capacity
            + memory_weight * self.memory_capacity
            + bandwidth_weight * self.bandwidth_capacity
    }

    /// Verifica que la capacidad sea válida
    pub fn is_valid(&self) -> bool {
        self.compute_capacity > 0.0
            && self.memory_capacity > 0.0
            && self.bandwidth_capacity > 0.0
            && self.model_dim > 0
    }
}

// ============================================================================
// Gradient Batch
// ============================================================================

/// Batch de gradientes de un nodo
#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientBatch {
    /// ID del nodo que envió los gradientes
    pub node_id: String,
    /// Datos de gradiente
    pub data: Vec<f32>,
    /// Checksum SHA-256 de los datos (hex)
    pub checksum: String,
    /// Timestamp de generación (epoch ms)
    pub generated_at_ms: u64,
    /// Ronda de federación
    pub round: u64,
    /// Dimensión original del modelo
    pub original_dim: usize,
}

#[cfg(feature = "v1.1-sprint4")]
impl GradientBatch {
    /// Crea un nuevo batch de gradientes
    pub fn new(node_id: String, data: Vec<f32>, round: u64, original_dim: usize) -> Self {
        let checksum = Self::compute_checksum(&data);
        Self {
            node_id,
            data,
            checksum,
            generated_at_ms: current_timestamp_ms(),
            round,
            original_dim,
        }
    }

    /// Verifica el checksum del batch
    pub fn verify_checksum(&self) -> bool {
        Self::compute_checksum(&self.data) == self.checksum
    }

    fn compute_checksum(data: &[f32]) -> String {
        let mut hasher = Sha256::new();
        // Serializar f32 como bytes para hash consistente
        let bytes: &[u8] =
            unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4) };
        hasher.update(bytes);
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Calcula la norma L2 del gradiente
    pub fn l2_norm(&self) -> f32 {
        self.data.iter().map(|v| v * v).sum::<f32>().sqrt()
    }

    /// Calcula la norma L1 del gradiente
    pub fn l1_norm(&self) -> f32 {
        self.data.iter().map(|v| v.abs()).sum()
    }
}

// ============================================================================
// Normalization Stats
// ============================================================================

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizerStats {
    /// Total de batches normalizados
    pub batches_normalized: u64,
    /// Total de gradientes rechazados (maliciosos)
    pub gradients_rejected: u64,
    /// Total de nodos registrados
    pub nodes_registered: usize,
    /// Dimensión objetivo actual
    pub target_dim: usize,
    /// Factor de normalización promedio
    pub avg_normalization_factor: f32,
    /// Última ronda procesada
    pub last_round: u64,
    /// Tiempo promedio de normalización (ms)
    pub avg_normalization_ms: f64,
}

#[cfg(feature = "v1.1-sprint4")]
impl Default for NormalizerStats {
    fn default() -> Self {
        Self {
            batches_normalized: 0,
            gradients_rejected: 0,
            nodes_registered: 0,
            target_dim: 0,
            avg_normalization_factor: 0.0,
            last_round: 0,
            avg_normalization_ms: 0.0,
        }
    }
}

// ============================================================================
// Normalizer Config
// ============================================================================

#[cfg(feature = "v1.1-sprint4")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizerConfig {
    /// Dimensión objetivo para todos los gradientes
    pub target_dim: usize,
    /// Umbral de outlier para detección de gradientes maliciosos
    pub outlier_threshold: f32,
    /// Factor de decaimiento para gradientes antiguos (ms)
    pub gradient_age_decay_ms: u64,
    /// Máximo de gradientes por ronda
    pub max_gradients_per_round: usize,
    /// Usar normalización L2 (true) o L1 (false)
    pub use_l2_normalization: bool,
    /// Factor de escalado por capacidad (0.0 = ignorar, 1.0 = completo)
    pub capacity_scaling_factor: f32,
}

#[cfg(feature = "v1.1-sprint4")]
impl Default for NormalizerConfig {
    fn default() -> Self {
        Self {
            target_dim: 1024,
            outlier_threshold: 3.0, // 3 desviaciones estándar
            gradient_age_decay_ms: 60_000, // 1 minuto
            max_gradients_per_round: 500,
            use_l2_normalization: true,
            capacity_scaling_factor: 0.8,
        }
    }
}

// ============================================================================
// Gradient Normalizer
// ============================================================================

#[cfg(feature = "v1.1-sprint4")]
pub struct GradientNormalizer {
    target_dim_initial: usize,
    config: NormalizerConfig,
    stats: NormalizerStats,
    // Capacidades de nodos registrados
    node_capacities: HashMap<String, NodeCapacity>,
    // Estadísticas globales para outlier detection
    global_mean: Vec<f32>,
    global_std: Vec<f32>,
    total_samples: u64,
    // Historial de factores de normalización
    normalization_factors: Vec<f32>,
}

#[cfg(feature = "v1.1-sprint4")]
impl GradientNormalizer {
    /// Crea normalizador con configuración default
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let config = NormalizerConfig::default();
        let target_dim = config.target_dim;
        Self {
            target_dim_initial: target_dim,
            config,
            stats: NormalizerStats::default(),
            node_capacities: HashMap::new(),
            global_mean: vec![0.0; target_dim],
            global_std: vec![1.0; target_dim],
            total_samples: 0,
            normalization_factors: Vec::new(),
        }
    }

    /// Crea normalizador con configuración personalizada
    pub fn with_config(config: NormalizerConfig) -> Self {
        let target_dim = config.target_dim;
        Self {
            target_dim_initial: target_dim,
            config,
            stats: NormalizerStats::default(),
            node_capacities: HashMap::new(),
            global_mean: vec![0.0; target_dim],
            global_std: vec![1.0; target_dim],
            total_samples: 0,
            normalization_factors: Vec::new(),
        }
    }

    /// Registra la capacidad de un nodo
    pub fn register_node(&mut self, capacity: NodeCapacity) -> Result<(), NormalizerError> {
        if !capacity.is_valid() {
            return Err(NormalizerError::InvalidCapacity {
                node_id: capacity.node_id.clone(),
            });
        }

        let node_id = capacity.node_id.clone();
        self.node_capacities.insert(node_id.clone(), capacity);
        self.stats.nodes_registered = self.node_capacities.len();
        debug!("Node registered: {} (capacity: {:.4})", self.stats.nodes_registered, self.node_capacities.get(&node_id).unwrap().weighted_capacity());

        Ok(())
    }

    /// Normaliza un batch de gradientes
    pub fn normalize(&mut self, batch: GradientBatch) -> Result<Vec<f32>, NormalizerError> {
        use std::time::Instant;
        let start = Instant::now();

        // Verificar checksum
        if !batch.verify_checksum() {
            return Err(NormalizerError::ChecksumFailed {
                node_id: batch.node_id.clone(),
            });
        }

        // Obtener capacidad del nodo
        let capacity = self.node_capacities.get(&batch.node_id);
        let capacity_factor = capacity
            .map(|c| c.weighted_capacity())
            .unwrap_or(1.0);

        // Escalar a dimensión objetivo
        let scaled = self.scale_to_target_dim(&batch.data, batch.original_dim);

        // Normalizar por norma
        let normalized = if self.config.use_l2_normalization {
            self.normalize_l2(&scaled)
        } else {
            self.normalize_l1(&scaled)
        };

        // Aplicar escalado por capacidad
        let capacity_scaled: Vec<f32> = normalized
            .iter()
            .map(|v| {
                v * (1.0 - self.config.capacity_scaling_factor
                    + self.config.capacity_scaling_factor * capacity_factor)
            })
            .collect();

        // Detección de outliers
        self.detect_outliers(&batch.node_id, &capacity_scaled)?;

        // Actualizar estadísticas globales
        self.update_global_stats(&capacity_scaled);

        // Calcular factor de normalización
        let factor = if batch.data.is_empty() {
            1.0
        } else {
            let original_norm = batch.l2_norm();
            let new_norm = capacity_scaled.iter().map(|v| v * v).sum::<f32>().sqrt();
            if original_norm > 0.0 {
                new_norm / original_norm
            } else {
                1.0
            }
        };

        self.normalization_factors.push(factor);
        if self.normalization_factors.len() > 100 {
            self.normalization_factors.remove(0);
        }

        let elapsed = start.elapsed();
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;

        // Actualizar stats
        self.stats.batches_normalized += 1;
        self.stats.last_round = batch.round;
        self.stats.avg_normalization_factor =
            self.normalization_factors.iter().sum::<f32>() / self.normalization_factors.len() as f32;
        self.stats.avg_normalization_ms =
            (self.stats.avg_normalization_ms * (self.stats.batches_normalized - 1) as f64 + elapsed_ms)
                / self.stats.batches_normalized as f64;

        debug!(
            "Gradient normalized: node={}, dim={}→{}, factor={:.4}, time={:.1}ms",
            batch.node_id,
            batch.original_dim,
            self.config.target_dim,
            factor,
            elapsed_ms
        );

        Ok(capacity_scaled)
    }

    /// Normaliza un batch completo de gradientes
    pub fn normalize_batch(
        &mut self,
        batches: Vec<GradientBatch>,
    ) -> Result<Vec<(String, Vec<f32>)>, NormalizerError> {
        if batches.is_empty() {
            return Err(NormalizerError::EmptyBatch);
        }

        let mut results = Vec::with_capacity(batches.len());

        for batch in batches {
            let normalized = self.normalize(batch.clone())?;
            results.push((batch.node_id, normalized));
        }

        Ok(results)
    }

    fn scale_to_target_dim(&self, data: &[f32], original_dim: usize) -> Vec<f32> {
        let target = self.config.target_dim;

        if original_dim == target {
            return data.to_vec();
        }

        if original_dim > target {
            // Downsample: promediar bloques
            let block_size = original_dim as f32 / target as f32;
            let mut result = Vec::with_capacity(target);

            for i in 0..target {
                let start = (i as f32 * block_size) as usize;
                let end = ((i as f32 + 1.0) * block_size) as usize;
                let block: &[_] = &data[start..end.min(original_dim)];
                let avg = block.iter().sum::<f32>() / block.len() as f32;
                result.push(avg);
            }

            result
        } else {
            // Upsample: interpolación lineal
            let mut result = Vec::with_capacity(target);
            let ratio = original_dim as f32 / target as f32;

            for i in 0..target {
                let pos = i as f32 * ratio;
                let idx = pos.floor() as usize;
                let next_idx = (idx + 1).min(original_dim - 1);
                let frac = pos - idx as f32;

                let interpolated = data[idx] * (1.0 - frac) + data[next_idx] * frac;
                result.push(interpolated);
            }

            result
        }
    }

    fn normalize_l2(&self, data: &[f32]) -> Vec<f32> {
        let norm = data.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            data.iter().map(|v| v / norm).collect()
        } else {
            data.to_vec()
        }
    }

    fn normalize_l1(&self, data: &[f32]) -> Vec<f32> {
        let norm = data.iter().map(|v| v.abs()).sum::<f32>();
        if norm > 0.0 {
            data.iter().map(|v| v / norm).collect()
        } else {
            data.to_vec()
        }
    }

    fn detect_outliers(&mut self, node_id: &str, data: &[f32]) -> Result<(), NormalizerError> {
        if self.total_samples < 3 {
            return Ok(());
        }

        let dim = data.len().min(self.global_mean.len());
        let mut max_z_score = 0.0f32;

        for (i, &val) in data.iter().enumerate().take(dim) {
            let std = self.global_std[i].max(1e-8);
            let z_score = (val - self.global_mean[i]).abs() / std;
            if z_score > max_z_score {
                max_z_score = z_score;
            }
        }

        if max_z_score > self.config.outlier_threshold {
            warn!(
                "Outlier detected: node={}, z_score={:.4}",
                node_id, max_z_score
            );
            self.stats.gradients_rejected += 1;
            return Err(NormalizerError::MaliciousGradient {
                node_id: node_id.to_string(),
                score: max_z_score,
            });
        }

        Ok(())
    }

    fn update_global_stats(&mut self, data: &[f32]) {
        self.total_samples += 1;
        let n = self.total_samples as f32;
        let dim = data.len().min(self.global_mean.len());

        for (i, &val) in data.iter().enumerate().take(dim) {
            let old_mean = self.global_mean[i];
            self.global_mean[i] = old_mean + (val - old_mean) / n;

            // Welford's algorithm para varianza online
            let diff = val - old_mean;
            let diff2 = val - self.global_mean[i];
            let variance = self.global_std[i] * self.global_std[i];
            let new_variance = variance + diff * diff2 / n;
            self.global_std[i] = new_variance.sqrt().max(1e-8);
        }
    }

    /// Obtiene estadísticas del normalizador
    pub fn get_stats(&self) -> NormalizerStats {
        self.stats.clone()
    }

    /// Obtiene configuración actual
    pub fn config(&self) -> &NormalizerConfig {
        &self.config
    }

    /// Obtiene número de nodos registrados
    pub fn node_count(&self) -> usize {
        self.node_capacities.len()
    }

    /// Reinicia estadísticas
    pub fn reset_stats(&mut self) {
        self.stats = NormalizerStats::default();
        self.normalization_factors.clear();
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
    fn test_normalizer_creation() {
        let normalizer = GradientNormalizer::new();
        assert_eq!(normalizer.node_count(), 0);
        let stats = normalizer.get_stats();
        assert_eq!(stats.batches_normalized, 0);
    }

    #[test]
    fn test_normalizer_with_config() {
        let config = NormalizerConfig {
            target_dim: 512,
            outlier_threshold: 5.0,
            ..Default::default()
        };
        let normalizer = GradientNormalizer::with_config(config);
        assert_eq!(normalizer.config().target_dim, 512);
    }

    #[test]
    fn test_node_capacity_weighted() {
        let capacity = NodeCapacity {
            node_id: "node-1".to_string(),
            compute_capacity: 0.8,
            memory_capacity: 0.6,
            bandwidth_capacity: 0.7,
            model_dim: 1024,
            model_type: "qwen-scope".to_string(),
        };
        let weighted = capacity.weighted_capacity();
        assert!((weighted - 0.71).abs() < 0.01);
        assert!(capacity.is_valid());
    }

    #[test]
    fn test_gradient_batch_checksum() {
        let batch = GradientBatch::new(
            "node-1".to_string(),
            vec![0.1, -0.2, 0.3, -0.4],
            1,
            4,
        );
        assert!(batch.verify_checksum());
    }

    #[test]
    fn test_gradient_batch_norms() {
        let batch = GradientBatch::new(
            "node-1".to_string(),
            vec![3.0, 4.0],
            1,
            2,
        );
        assert!((batch.l2_norm() - 5.0).abs() < 0.01);
        assert!((batch.l1_norm() - 7.0).abs() < 0.01);
    }

    #[test]
    fn test_config_default() {
        let config = NormalizerConfig::default();
        assert_eq!(config.target_dim, 1024);
        assert_eq!(config.outlier_threshold, 3.0);
        assert!(config.use_l2_normalization);
    }

    #[test]
    fn test_stats_default() {
        let stats = NormalizerStats::default();
        assert_eq!(stats.batches_normalized, 0);
        assert_eq!(stats.gradients_rejected, 0);
    }

    #[test]
    fn test_invalid_capacity() {
        let capacity = NodeCapacity {
            node_id: "node-1".to_string(),
            compute_capacity: 0.0,
            memory_capacity: 0.6,
            bandwidth_capacity: 0.7,
            model_dim: 1024,
            model_type: "test".to_string(),
        };
        assert!(!capacity.is_valid());
    }
}
