//! Feature Analyzer - Análisis de activaciones SAE + detección de anomalías
//!
//! Procesa `sparse_features` del SAE y calcula:
//! - Densidad de activación
//! - Desviación estándar por feature
//! - Detección de anomalías (activación > 2σ)
//! - Categorización de patrones (contradicción, repetición, etc.)

use anyhow::Result;
use candle_core::Tensor;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

use crate::p2p::protocol::SparseFeature;

// ============================================================================
// Resultados de Análisis
// ============================================================================

/// Resultado del análisis de features SAE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// Score de anomalía (0.0 = normal, 1.0 = altamente anómalo)
    pub anomaly_score: f32,
    /// Índices de features flaggeadas como anómalas
    pub flagged_features: Vec<usize>,
    /// Confianza del análisis (0.0 - 1.0)
    pub confidence: f32,
    /// Densidad de activación (ratio de features activas vs total)
    pub activation_density: f32,
    /// Desviación estándar de las activaciones
    pub std_deviation: f32,
    /// Media de las activaciones
    pub mean_activation: f32,
    /// Patrones detectados
    pub detected_patterns: Vec<PatternType>,
    /// Timestamp del análisis (Unix epoch ms)
    pub timestamp: u64,
}

/// Tipo de patrón detectado
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PatternType {
    /// Activación contradictoria (features mutuamente excluyentes activas)
    Contradiction,
    /// Repetición de patrones (mismas features activas en múltiples windows)
    Repetition,
    /// Activación anómala (> 2σ de la media)
    Anomaly,
    /// Activación concentrada (pocas features con alta activación)
    Concentrated,
    /// Activación dispersa (muchas features con baja activación)
    Dispersed,
    // TODO: Phase 3 - Más patrones basados en RLHF feedback
}

impl std::fmt::Display for PatternType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatternType::Contradiction => write!(f, "contradiction"),
            PatternType::Repetition => write!(f, "repetition"),
            PatternType::Anomaly => write!(f, "anomaly"),
            PatternType::Concentrated => write!(f, "concentrated"),
            PatternType::Dispersed => write!(f, "dispersed"),
        }
    }
}

// ============================================================================
// Estadísticas Históricas
// ============================================================================

/// Estadísticas acumuladas por feature para detección de anomalías
#[derive(Debug, Clone)]
pub struct FeatureStatistics {
    /// Número de observaciones
    pub count: usize,
    /// Suma de activaciones
    pub sum: f64,
    /// Suma de activaciones al cuadrado
    pub sum_sq: f64,
    /// Activación máxima observada
    pub max_activation: f32,
    /// Activación mínima observada
    pub min_activation: f32,
}

impl Default for FeatureStatistics {
    fn default() -> Self {
        Self::new()
    }
}

impl FeatureStatistics {
    pub fn new() -> Self {
        Self {
            count: 0,
            sum: 0.0,
            sum_sq: 0.0,
            max_activation: f32::NEG_INFINITY,
            min_activation: f32::INFINITY,
        }
    }

    /// Actualizar estadísticas con nueva observación (Welford's algorithm)
    pub fn update(&mut self, value: f32) {
        let v = value as f64;
        self.count += 1;
        self.sum += v;
        self.sum_sq += v * v;
        if value > self.max_activation {
            self.max_activation = value;
        }
        if value < self.min_activation {
            self.min_activation = value;
        }
    }

    /// Calcular media
    pub fn mean(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        self.sum / self.count as f64
    }

    /// Calcular desviación estándar
    pub fn std_dev(&self) -> f64 {
        if self.count < 2 {
            return 0.0;
        }
        let mean = self.mean();
        let variance = (self.sum_sq / self.count as f64) - (mean * mean);
        if variance < 0.0 {
            return 0.0;
        }
        variance.sqrt()
    }

    /// Calcular z-score para un valor
    pub fn z_score(&self, value: f32) -> f64 {
        let std = self.std_dev();
        if std < 1e-9 {
            return 0.0;
        }
        (value as f64 - self.mean()) / std
    }
}

// ============================================================================
// Feature Analyzer
// ============================================================================

/// Analizador de features SAE
pub struct FeatureAnalyzer {
    /// Estadísticas históricas por feature index
    statistics: HashMap<usize, FeatureStatistics>,
    /// Umbral de z-score para detección de anomalías
    anomaly_z_threshold: f64,
    /// Umbral de densidad para patrón concentrado
    concentrated_threshold: f32,
    /// Umbral de densidad para patrón disperso
    dispersed_threshold: f32,
    /// Total de features posibles (latent_dim del SAE)
    total_features: usize,
    /// Historial de activaciones para detección de repetición
    activation_history: Vec<Vec<usize>>,
    /// Tamaño máximo del historial
    max_history_size: usize,
}

impl FeatureAnalyzer {
    /// Crear nuevo analizador
    pub fn new(total_features: usize) -> Self {
        Self {
            statistics: HashMap::new(),
            anomaly_z_threshold: 2.0, // 2σ default
            concentrated_threshold: 0.05, // <5% features activas = concentrado
            dispersed_threshold: 0.5,     // >50% features activas = disperso
            total_features,
            activation_history: Vec::new(),
            max_history_size: 64,
        }
    }

    /// Configurar umbral de anomalía (z-score)
    pub fn with_anomaly_threshold(mut self, threshold: f64) -> Self {
        self.anomaly_z_threshold = threshold;
        self
    }

    /// Analizar batch de sparse features
    ///
    /// # Arguments
    /// * `features` - Features sparse del SAE
    /// * `layer_id` - ID de la capa SAE de origen
    ///
    /// # Returns
    /// `AnalysisResult` con scores y patrones detectados
    pub fn analyze(&mut self, features: &[SparseFeature], layer_id: u32) -> AnalysisResult {
        info!(
            "Analizando {} features de layer {}",
            features.len(),
            layer_id
        );

        if features.is_empty() {
            return AnalysisResult {
                anomaly_score: 0.0,
                flagged_features: vec![],
                confidence: 0.0,
                activation_density: 0.0,
                std_deviation: 0.0,
                mean_activation: 0.0,
                detected_patterns: vec![],
                timestamp: current_timestamp_ms(),
            };
        }

        // Extraer valores de activación
        let activations: Vec<f32> = features.iter().map(|f| f.activation_value).collect();

        // Calcular estadísticas del batch actual
        let (mean, std_dev) = self.compute_batch_stats(&activations);

        // Calcular densidad de activación
        let activation_density = features.len() as f32 / self.total_features.max(1) as f32;

        // Detectar anomalías por z-score
        let mut flagged_features = Vec::new();
        for feature in features {
            let idx = feature.neuron_index as usize;

            // Obtener o crear estadísticas
            let stats = self
                .statistics
                .entry(idx)
                .or_default(); // CLEANUP: or_insert_with -> or_default (FeatureStatistics implements Default)

            // Calcular z-score antes de actualizar
            let z_score = stats.z_score(feature.activation_value);

            if z_score.abs() > self.anomaly_z_threshold && stats.count >= 3 {
                flagged_features.push(idx);
                warn!(
                    "Anomalía detectada: feature={}, z_score={:.2}, activation={:.4}",
                    idx, z_score, feature.activation_value
                );
            }

            // Actualizar estadísticas
            stats.update(feature.activation_value);
        }

        // Detectar patrones
        let mut detected_patterns = Vec::new();

        // Patrón: Concentrado
        if activation_density < self.concentrated_threshold && !features.is_empty() {
            detected_patterns.push(PatternType::Concentrated);
        }

        // Patrón: Disperso
        if activation_density > self.dispersed_threshold {
            detected_patterns.push(PatternType::Dispersed);
        }

        // Patrón: Anomalía
        if !flagged_features.is_empty() {
            detected_patterns.push(PatternType::Anomaly);
        }

        // Patrón: Repetición (comparar con historial)
        let current_active: Vec<usize> =
            features.iter().map(|f| f.neuron_index as usize).collect();
        self.activation_history.push(current_active.clone());
        if self.activation_history.len() > self.max_history_size {
            self.activation_history.remove(0);
        }
        if self.detect_repetition(&self.activation_history) {
            detected_patterns.push(PatternType::Repetition);
        }

        // Patrón: Contradicción (features mutuamente excluyentes)
        if self.detect_contradiction(features) {
            detected_patterns.push(PatternType::Contradiction);
        }

        // Calcular score de anomalía
        let anomaly_score = self.compute_anomaly_score(
            &flagged_features,
            &detected_patterns,
            activation_density,
        );

        // Calcular confianza
        let confidence = self.compute_confidence(features.len(), &detected_patterns);

        let result = AnalysisResult {
            anomaly_score,
            flagged_features,
            confidence,
            activation_density,
            std_deviation: std_dev,
            mean_activation: mean,
            detected_patterns,
            timestamp: current_timestamp_ms(),
        };

        debug!("Resultado análisis: {:?}", result);
        result
    }

    /// Analizar batch usando operaciones vectorizadas con Candle
    ///
    /// Usa `candle_core::Tensor` para cálculos eficientes en CPU/GPU.
    pub fn analyze_with_tensor(&mut self, activations: &Tensor) -> Result<AnalysisResult> {
        // Obtener dimensiones
        let shape = activations.dims();
        debug!("Tensor shape: {:?}", shape);

        // Calcular media con Candle
        let mean_tensor = activations.mean_all()?;
        let mean: f64 = mean_tensor.to_scalar::<f64>()?;

        // Calcular varianza: E[X^2] - E[X]^2
        let squared = activations.sqr()?;
        let mean_sq: f64 = squared.mean_all()?.to_scalar::<f64>()?;
        let variance = (mean_sq - mean * mean).max(0.0);
        let std_dev = variance.sqrt() as f32;

        // Para detección de anomalías, necesitamos los valores individuales
        let flat = activations.flatten_all()?;
        let values: Vec<f32> = flat.to_vec1()?;

        // Crear features placeholder para análisis de patrones
        let features: Vec<SparseFeature> = values
            .iter()
            .enumerate()
            .filter(|(_, &v)| v > 0.0) // Solo activaciones no-cero
            .map(|(i, &v)| SparseFeature {
                neuron_index: i as u32,
                activation_value: v,
                importance: v,
            })
            .collect();

        // Usar análisis estándar con las features extraídas
        let mut result = self.analyze(&features, 0);
        result.mean_activation = mean as f32;
        result.std_deviation = std_dev;

        Ok(result)
    }

    /// Calcular estadísticas del batch
    fn compute_batch_stats(&self, activations: &[f32]) -> (f32, f32) {
        if activations.is_empty() {
            return (0.0, 0.0);
        }

        let sum: f64 = activations.iter().map(|&v| v as f64).sum();
        let mean = sum / activations.len() as f64;

        let sum_sq: f64 = activations
            .iter()
            .map(|&v| {
                let diff = v as f64 - mean;
                diff * diff
            })
            .sum();
        let variance = sum_sq / activations.len() as f64;
        let std_dev = variance.sqrt() as f32;

        (mean as f32, std_dev)
    }

    /// Detectar patrón de repetición
    fn detect_repetition(&self, history: &[Vec<usize>]) -> bool {
        if history.len() < 3 {
            return false;
        }

        // Comparar último batch con los anteriores
        let last = &history[history.len() - 1];
        let window = history.len().min(10);
        let start = history.len() - window;

        let mut similar_count = 0;
        // CLEANUP: Fix clippy needless_range_loop
        for prev in history.iter().take(history.len() - 1).skip(start) {
            let overlap = self.jaccard_similarity(prev, last);
            if overlap > 0.7 {
                similar_count += 1;
            }
        }

        // Si ≥50% de los batches recientes son similares
        similar_count >= window / 2
    }

    /// Calcular coeficiente Jaccard entre dos sets
    fn jaccard_similarity(&self, a: &[usize], b: &[usize]) -> f64 {
        if a.is_empty() && b.is_empty() {
            return 1.0;
        }

        let set_a: std::collections::HashSet<_> = a.iter().collect();
        let set_b: std::collections::HashSet<_> = b.iter().collect();

        let intersection = set_a.intersection(&set_b).count() as f64;
        let union = set_a.union(&set_b).count() as f64;

        if union == 0.0 {
            return 0.0;
        }
        intersection / union
    }

    /// Detectar patrón de contradicción
    ///
    /// Heurística: features con activaciones altas pero importancia baja
    /// pueden indicar conflicto interpretativo.
    fn detect_contradiction(&self, features: &[SparseFeature]) -> bool {
        let high_activation_low_importance = features
            .iter()
            .filter(|f| f.activation_value > 0.8 && f.importance < 0.3)
            .count();

        // Si ≥20% de las features tienen este patrón
        high_activation_low_importance > features.len() / 5
    }

    /// Calcular score de anomalía
    fn compute_anomaly_score(
        &self,
        flagged: &[usize],
        patterns: &[PatternType],
        density: f32,
    ) -> f32 {
        let mut score = 0.0f32;

        // Contribución de features flaggeadas
        if !flagged.is_empty() {
            score += (flagged.len() as f32 / self.total_features.max(1) as f32).min(0.4);
        }

        // Contribución de patrones
        for pattern in patterns {
            match pattern {
                PatternType::Contradiction => score += 0.3,
                PatternType::Anomaly => score += 0.2,
                PatternType::Repetition => score += 0.1,
                PatternType::Concentrated => score += 0.05,
                PatternType::Dispersed => score += 0.05,
            }
        }

        // Penalización por densidad extrema
        if !(0.01..=0.8).contains(&density) {
            score += 0.1;
        }

        score.min(1.0)
    }

    /// Calcular confianza del análisis
    fn compute_confidence(&self, feature_count: usize, patterns: &[PatternType]) -> f32 {
        let mut confidence = 0.5f32; // Base

        // Más features = más confianza
        if feature_count > 10 {
            confidence += 0.1;
        }
        if feature_count > 50 {
            confidence += 0.1;
        }

        // Patrones claros = más confianza
        if patterns.len() >= 2 {
            confidence += 0.1;
        }

        // Patrones contradictorios = menos confianza
        if patterns.contains(&PatternType::Contradiction)
            && patterns.contains(&PatternType::Concentrated)
        {
            confidence -= 0.1;
        }

        confidence.clamp(0.0, 1.0)
    }

    /// Obtener estadísticas de una feature específica
    pub fn get_feature_stats(&self, feature_index: usize) -> Option<&FeatureStatistics> {
        self.statistics.get(&feature_index)
    }

    /// Obtener número total de features analizadas
    pub fn total_features_analyzed(&self) -> usize {
        self.statistics.len()
    }

    /// Resetear estadísticas
    pub fn reset(&mut self) {
        self.statistics.clear();
        self.activation_history.clear();
        info!("Estadísticas del analizador reseteadas");
    }
}

/// Timestamp actual en milisegundos Unix epoch
fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

impl Default for FeatureAnalyzer {
    fn default() -> Self {
        Self::new(16384) // Default latent_dim
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_statistics() {
        let mut stats = FeatureStatistics::new();
        for &v in &[1.0, 2.0, 3.0, 4.0, 5.0] {
            stats.update(v);
        }
        assert!((stats.mean() - 3.0).abs() < 1e-9);
        assert!(stats.std_dev() > 0.0);
    }

    #[test]
    fn test_analyzer_empty_features() {
        let mut analyzer = FeatureAnalyzer::new(1000);
        let result = analyzer.analyze(&[], 0);
        assert_eq!(result.anomaly_score, 0.0);
        assert!(result.flagged_features.is_empty());
    }

    #[test]
    fn test_analyzer_concentrated_pattern() {
        let mut analyzer = FeatureAnalyzer::new(10000);
        let features = vec![
            SparseFeature {
                neuron_index: 0,
                activation_value: 0.95,
                importance: 0.9,
            },
            SparseFeature {
                neuron_index: 1,
                activation_value: 0.85,
                importance: 0.8,
            },
        ];
        let result = analyzer.analyze(&features, 0);
        assert!(result.detected_patterns.contains(&PatternType::Concentrated));
    }

    #[test]
    fn test_jaccard_similarity() {
        let analyzer = FeatureAnalyzer::new(100);
        let a = vec![1, 2, 3];
        let b = vec![2, 3, 4];
        let similarity = analyzer.jaccard_similarity(&a, &b);
        assert!((similarity - 0.5).abs() < 1e-9); // {2,3} / {1,2,3,4}
    }
}
