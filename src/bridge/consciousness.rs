//! Consciousness Bridge - Agregación, resolución de conflictos y generación de steering signals
//!
//! Agrega features de múltiples nodos, resuelve conflictos cuando ≥2 nodos
//! reportan activación contradictoria en la misma feature, y genera
//! `ContextInjection` para el LLM downstream.

use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::Instant;
use tracing::{debug, info, warn};

use crate::consensus::validator::ConsensusValidator;
use crate::interpret::feature_analyzer::{AnalysisResult, FeatureAnalyzer};
use crate::interpret::semantic_map::SemanticMap;
use crate::p2p::protocol::{SparseFeature, SteeringSignal};

// ============================================================================
// Context Injection
// ============================================================================

/// Inyección de contexto generada por el Consciousness Bridge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextInjection {
    /// ID único de la inyección
    pub injection_id: String,
    /// Mensaje de contexto para el LLM
    pub context_message: String,
    /// Features involucradas
    pub feature_indices: Vec<u32>,
    /// Capas involucradas
    pub layer_ids: Vec<u32>,
    /// Tipo de acción sugerida
    pub action: InjectionAction,
    /// Confianza (0.0 - 1.0)
    pub confidence: f32,
    /// Timestamp (Unix epoch ms)
    pub timestamp: u64,
}

/// Tipo de acción sugerida
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InjectionAction {
    /// Suprimir activación de features específicas
    Suppress,
    /// Amplificar activación de features específicas
    Amplify,
    /// Re-evaluar (solicitar análisis adicional)
    Reevaluate,
    /// Sin acción (informativo)
    Informative,
}

impl std::fmt::Display for InjectionAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InjectionAction::Suppress => write!(f, "suppress"),
            InjectionAction::Amplify => write!(f, "amplify"),
            InjectionAction::Reevaluate => write!(f, "reevaluate"),
            InjectionAction::Informative => write!(f, "informative"),
        }
    }
}

// ============================================================================
// Conflict Resolution
// ============================================================================

/// Conflicto detectado entre nodos
#[derive(Debug, Clone)]
pub struct FeatureConflict {
    /// Feature index en conflicto
    pub feature_index: u32,
    /// Layer ID
    pub layer_id: u32,
    /// Reportes contradictorios
    pub conflicting_reports: Vec<ConflictReport>,
    /// Estado del conflicto
    pub state: ConflictState,
    /// Resolución (si aplica)
    pub resolution: Option<ConflictResolution>,
    /// MIGRATION: Instant doesn't implement Serialize/Deserialize
    pub detected_at: Instant,
}

/// Reporte individual de un nodo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictReport {
    /// Peer ID del nodo reportante
    pub peer_id: String,
    /// Valor de activación reportado
    pub activation_value: f32,
    /// Confianza del reporte
    pub confidence: f32,
}

/// Estado del conflicto
// MIGRATION: serde Serialize/Deserialize required for ConflictState
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConflictState {
    /// Detectado, pendiente de resolución
    Detected,
    /// En proceso de re-evaluación
    Reevaluating,
    /// Resuelto
    Resolved,
    /// No resuelto (UNRESOLVED)
    Unresolved,
}

/// Resolución de conflicto
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Consenso alcanzado por valor promedio
    ConsensusAverage(f32),
    /// Mayoría votó por un valor específico
    MajorityVote(f32),
    /// Se solicitó re-evaluación a nodos adicionales
    RequestedReevaluation,
}

// ============================================================================
// Feedback Queue
// ============================================================================

/// Cola de feedback asíncrono
pub struct FeedbackQueue {
    /// Cola interna con tamaño máximo 128
    queue: VecDeque<AnalysisResult>,
    /// Tamaño máximo
    max_size: usize,
    /// Procesados total
    processed_count: u64,
}

impl FeedbackQueue {
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: VecDeque::with_capacity(max_size),
            max_size,
            processed_count: 0,
        }
    }

    /// Agregar resultado al final de la cola
    pub fn push(&mut self, result: AnalysisResult) {
        if self.queue.len() >= self.max_size {
            // Eliminar más antiguo
            self.queue.pop_front();
            debug!("FeedbackQueue llena, eliminando entrada más antigua");
        }
        self.queue.push_back(result);
    }

    /// Extraer resultados para procesamiento por batch
    pub fn drain_batch(&mut self, batch_size: usize) -> Vec<AnalysisResult> {
        let batch: Vec<AnalysisResult> = self
            .queue
            .drain(..batch_size.min(self.queue.len()))
            .collect();
        self.processed_count += batch.len() as u64;
        batch
    }

    /// Tamaño actual
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Total procesados
    pub fn processed_count(&self) -> u64 {
        self.processed_count
    }
}

impl Default for FeedbackQueue {
    fn default() -> Self {
        Self::new(128)
    }
}

// ============================================================================
// Consciousness Bridge
// ============================================================================

/// Consciousness Bridge - Núcleo de agregación y decisión
pub struct ConsciousnessBridge {
    /// Features agregadas por (layer_id, feature_index)
    aggregated_features: RwLock<HashMap<(u32, u32), FeatureAggregation>>,
    /// Conflictos activos
    conflicts: RwLock<Vec<FeatureConflict>>,
    /// Cola de feedback asíncrono
    feedback_queue: RwLock<FeedbackQueue>,
    /// Inyecciones de contexto generadas
    context_injections: RwLock<Vec<ContextInjection>>,
    /// Analizador de features
    analyzer: RwLock<FeatureAnalyzer>,
    /// Mapa semántico
    semantic_map: RwLock<SemanticMap>,
    /// Validador de consenso
    consensus: RwLock<ConsensusValidator>,
    /// Umbral de confianza para generar steering signals
    steering_confidence_threshold: f32,
    /// Umbral de anomalía para generar steering signals
    anomaly_threshold: f32,
    /// Total de steering signals generados
    steering_signals_generated: u64,
}

/// Agregación de feature por múltiples nodos
#[derive(Debug, Clone)]
pub struct FeatureAggregation {
    /// Valores reportados por cada nodo
    pub reports: Vec<(String, f32, f32)>, // (peer_id, activation, confidence)
    /// Valor promedio
    pub average_activation: f32,
    /// Desviación estándar
    pub std_deviation: f32,
    /// Última actualización
    pub last_updated: Instant,
}

impl Default for FeatureAggregation {
    fn default() -> Self {
        Self::new()
    }
}

impl FeatureAggregation {
    pub fn new() -> Self {
        Self {
            reports: Vec::new(),
            average_activation: 0.0,
            std_deviation: 0.0,
            last_updated: Instant::now(),
        }
    }

    /// Agregar reporte de un nodo
    pub fn add_report(&mut self, peer_id: String, activation: f32, confidence: f32) {
        self.reports.push((peer_id, activation, confidence));
        self.recalculate_stats();
        self.last_updated = Instant::now();
    }

    /// Recalcular estadísticas
    fn recalculate_stats(&mut self) {
        if self.reports.is_empty() {
            self.average_activation = 0.0;
            self.std_deviation = 0.0;
            return;
        }

        let activations: Vec<f64> = self.reports.iter().map(|(_, a, _)| *a as f64).collect();
        let sum: f64 = activations.iter().sum();
        let mean = sum / activations.len() as f64;

        let variance: f64 =
            activations.iter().map(|a| (a - mean).powi(2)).sum::<f64>() / activations.len() as f64;

        self.average_activation = mean as f32;
        self.std_deviation = variance.sqrt() as f32;
    }

    /// Detectar si hay conflicto (alta varianza entre reportes)
    pub fn has_conflict(&self, threshold: f32) -> bool {
        self.reports.len() >= 2 && self.std_deviation > threshold
    }
}

impl ConsciousnessBridge {
    /// Crear nuevo Consciousness Bridge
    pub fn new(total_sae_features: usize) -> Self {
        Self {
            aggregated_features: RwLock::new(HashMap::new()),
            conflicts: RwLock::new(Vec::new()),
            feedback_queue: RwLock::new(FeedbackQueue::new(128)),
            context_injections: RwLock::new(Vec::new()),
            analyzer: RwLock::new(FeatureAnalyzer::new(total_sae_features)),
            semantic_map: RwLock::new(SemanticMap::new()),
            consensus: RwLock::new(ConsensusValidator::new()),
            steering_confidence_threshold: 0.85,
            anomaly_threshold: 0.7,
            steering_signals_generated: 0,
        }
    }

    /// Configurar umbrales
    pub fn with_thresholds(mut self, steering_confidence: f32, anomaly: f32) -> Self {
        self.steering_confidence_threshold = steering_confidence;
        self.anomaly_threshold = anomaly;
        self
    }

    /// Agregar features de un nodo
    ///
    /// # Arguments
    /// * `peer_id` - ID del nodo reportante
    /// * `layer_id` - Capa SAE de origen
    /// * `features` - Features sparse del nodo
    pub fn add_node_features(
        &self,
        peer_id: String,
        layer_id: u32,
        features: &[SparseFeature],
    ) -> Result<()> {
        info!(
            "Agregando {} features de peer={}, layer={}",
            features.len(),
            peer_id,
            layer_id
        );

        let mut aggregated = self.aggregated_features.write();

        for feature in features {
            let key = (layer_id, feature.neuron_index);
            let agg = aggregated.entry(key).or_default(); // CLEANUP: or_insert_with -> or_default (FeatureAggregation implements Default)

            agg.add_report(
                peer_id.clone(),
                feature.activation_value,
                feature.importance,
            );

            // Detectar conflicto
            if agg.has_conflict(0.3) {
                self.detect_conflict(layer_id, feature.neuron_index, agg);
            }
        }

        Ok(())
    }

    /// Detectar conflicto entre reportes
    fn detect_conflict(&self, layer_id: u32, feature_index: u32, aggregation: &FeatureAggregation) {
        let conflict = FeatureConflict {
            feature_index,
            layer_id,
            conflicting_reports: aggregation
                .reports
                .iter()
                .map(|(peer_id, activation, confidence)| ConflictReport {
                    peer_id: peer_id.clone(),
                    activation_value: *activation,
                    confidence: *confidence,
                })
                .collect(),
            state: ConflictState::Detected,
            resolution: None,
            detected_at: Instant::now(),
        };

        warn!(
            "Conflicto detectado: layer={}, feature={}, std_dev={:.3}",
            layer_id, feature_index, aggregation.std_deviation
        );

        self.conflicts.write().push(conflict);

        // Si ≥2 nodos reportan activación contradictoria, marcar como UNRESOLVED
        // y solicitar re-evaluación
        if aggregation.reports.len() >= 2 {
            self.request_reevaluation(layer_id, feature_index);
        }
    }

    /// Solicitar re-evaluación de feature en conflicto
    fn request_reevaluation(&self, layer_id: u32, feature_index: u32) {
        let mut conflicts = self.conflicts.write();
        for conflict in conflicts.iter_mut() {
            if conflict.feature_index == feature_index && conflict.layer_id == layer_id {
                conflict.state = ConflictState::Reevaluating;
                conflict.resolution = Some(ConflictResolution::RequestedReevaluation);
                info!(
                    "Re-evaluación solicitada: layer={}, feature={}",
                    layer_id, feature_index
                );
            }
        }
    }

    /// Procesar cola de feedback (llamar cada 5 segundos)
    pub fn process_feedback_queue(&mut self) -> Vec<SteeringSignal> {
        let mut signals = Vec::new();
        let batch = self.feedback_queue.write().drain_batch(16);

        for result in &batch {
            // Si confidence ≥ 0.85 y anomaly_score ≥ 0.7, generar steering signal
            if result.confidence >= self.steering_confidence_threshold
                && result.anomaly_score >= self.anomaly_threshold
            {
                let signal = self.generate_steering_signal(result);
                signals.push(signal);
                self.steering_signals_generated += 1;
            }
        }

        if !signals.is_empty() {
            info!(
                "Feedback procesado: {} steering signals generados",
                signals.len()
            );
        }

        signals
    }

    /// Generar steering signal desde resultado de análisis
    fn generate_steering_signal(&self, result: &AnalysisResult) -> SteeringSignal {
        // Crear máscara de supresión/amplificación
        let payload = self.build_steering_payload(result);

        SteeringSignal {
            signal_type: if result.anomaly_score > 0.8 {
                "suppress".to_string()
            } else {
                "amplify".to_string()
            },
            payload,
            priority: (result.anomaly_score * 100.0) as u8,
            timestamp: result.timestamp,
        }
    }

    /// Construir payload de steering
    fn build_steering_payload(&self, result: &AnalysisResult) -> String {
        let flagged = &result.flagged_features;
        let patterns: Vec<String> = result
            .detected_patterns
            .iter()
            .map(|p| p.to_string())
            .collect();

        serde_json::json!({
            "flagged_features": flagged,
            "anomaly_score": result.anomaly_score,
            "confidence": result.confidence,
            "patterns": patterns,
            "action": if result.anomaly_score > 0.8 { "suppress" } else { "amplify" },
            "mask": self.build_feature_mask(flagged),
        })
        .to_string()
    }

    /// Construir máscara de features (para supresión/amplificación)
    fn build_feature_mask(&self, flagged: &[usize]) -> Vec<usize> {
        flagged.to_vec()
    }

    /// Generar ContextInjection para el LLM
    pub fn generate_context_injection(&self, layer_id: u32) -> Option<ContextInjection> {
        let aggregated = self.aggregated_features.read();

        // Obtener features de esta capa
        let layer_features: Vec<_> = aggregated
            .iter()
            .filter(|((l, _), _)| *l == layer_id)
            .collect();

        if layer_features.is_empty() {
            return None;
        }

        // Identificar features más activas
        let mut active: Vec<_> = layer_features
            .iter()
            .map(|((_, f), agg)| (*f, agg.average_activation))
            .filter(|(_, activation)| *activation > 0.5)
            .collect();

        active.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        active.truncate(10); // Top 10

        // Generar mensaje de contexto
        let context_message = self.build_context_message(&active, layer_id);

        // Determinar acción
        let action = self.determine_action(&active);

        let injection = ContextInjection {
            injection_id: uuid::Uuid::new_v4().to_string(),
            context_message,
            feature_indices: active.iter().map(|(f, _)| *f).collect(),
            layer_ids: vec![layer_id],
            action,
            confidence: 0.8, // TODO: Phase 3 - Calcular confianza real
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        };

        self.context_injections.write().push(injection.clone());
        Some(injection)
    }

    /// Construir mensaje de contexto
    fn build_context_message(&self, active_features: &[(u32, f32)], layer_id: u32) -> String {
        let semantic = self.semantic_map.read();
        let descriptions: Vec<String> = active_features
            .iter()
            .take(5)
            .map(|(idx, activation)| {
                let concepts = semantic.lookup_by_feature(*idx);
                if let Some(concept) = concepts.first() {
                    format!(
                        "Feature [{}] '{}' activado en capa [{}] con confianza [{:.2}]",
                        idx, concept.name, layer_id, activation
                    )
                } else {
                    format!(
                        "Feature [{}] activado en capa [{}] con confianza [{:.2}]",
                        idx, layer_id, activation
                    )
                }
            })
            .collect();

        let suggestion = if active_features.iter().any(|(_, a)| *a > 0.9) {
            "Sugerencia: suprimir activaciones altas."
        } else {
            "Sugerencia: amplificar activaciones moderadas."
        };

        format!("{} | {}", descriptions.join("; "), suggestion)
    }

    /// Determinar acción basada en features activas
    fn determine_action(&self, active_features: &[(u32, f32)]) -> InjectionAction {
        let max_activation = active_features
            .iter()
            .map(|(_, a)| *a)
            .reduce(f32::max)
            .unwrap_or(0.0);

        if max_activation > 0.9 {
            InjectionAction::Suppress
        } else if max_activation < 0.3 {
            InjectionAction::Amplify
        } else {
            InjectionAction::Informative
        }
    }

    /// Analizar features con el FeatureAnalyzer
    pub fn analyze_features(&self, features: &[SparseFeature], layer_id: u32) -> AnalysisResult {
        let mut analyzer = self.analyzer.write();
        analyzer.analyze(features, layer_id)
    }

    /// Obtener conflictos activos
    pub fn get_active_conflicts(&self) -> Vec<FeatureConflict> {
        self.conflicts
            .read()
            .iter()
            .filter(|c| c.state != ConflictState::Resolved)
            .cloned()
            .collect()
    }

    /// Obtener inyecciones de contexto recientes
    pub fn get_recent_injections(&self, limit: usize) -> Vec<ContextInjection> {
        let injections = self.context_injections.read();
        injections.iter().rev().take(limit).cloned().collect()
    }

    /// Obtener estadísticas
    pub fn stats(&self) -> BridgeStats {
        BridgeStats {
            total_aggregated_features: self.aggregated_features.read().len(),
            active_conflicts: self.get_active_conflicts().len(),
            feedback_queue_size: self.feedback_queue.read().len(),
            context_injections: self.context_injections.read().len(),
            steering_signals_generated: self.steering_signals_generated,
            total_features_analyzed: self.analyzer.read().total_features_analyzed(),
            semantic_concepts: self.semantic_map.read().total_concepts(),
        }
    }
}

/// Estadísticas del Consciousness Bridge
#[derive(Debug, Clone)]
pub struct BridgeStats {
    pub total_aggregated_features: usize,
    pub active_conflicts: usize,
    pub feedback_queue_size: usize,
    pub context_injections: usize,
    pub steering_signals_generated: u64,
    pub total_features_analyzed: usize,
    pub semantic_concepts: usize,
}

// ============================================================================
// Placeholder para Fase 3 - RLHF Loop
// ============================================================================

/// Placeholder para integración RLHF en Fase 3
///
/// En Fase 3, este módulo implementará:
/// - Feedback loop con humanos para refinar conceptos semánticos
/// - Ajuste de pesos SAE basado en feedback humano
/// - Aprendizaje por refuerzo para mejorar detección de anomalías
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RLHFPlaceholder {
    pub feedback_id: String,
    pub human_rating: Option<f32>,
    pub model_prediction: f32,
    pub reward: f32,
}

impl RLHFPlaceholder {
    /// Generar placeholder de RLHF
    pub fn generate(_feature_index: u32, _prediction: f32) -> Self {
        Self {
            feedback_id: uuid::Uuid::new_v4().to_string(),
            human_rating: None, // TODO: Phase 3 - Integrar feedback humano
            model_prediction: _prediction,
            reward: 0.0, // TODO: Phase 3 - Calcular reward
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feedback_queue() {
        let mut queue = FeedbackQueue::new(4);
        for i in 0..6 {
            queue.push(AnalysisResult {
                anomaly_score: i as f32 / 10.0,
                flagged_features: vec![],
                confidence: 0.5,
                activation_density: 0.1,
                std_deviation: 0.1,
                mean_activation: 0.5,
                detected_patterns: vec![],
                timestamp: 0,
            });
        }
        assert_eq!(queue.len(), 4); // Max size
    }

    #[test]
    fn test_feature_aggregation() {
        let mut agg = FeatureAggregation::new();
        agg.add_report("node_a".to_string(), 0.9, 0.8);
        agg.add_report("node_b".to_string(), 0.1, 0.7);
        assert!(agg.has_conflict(0.3));
    }

    #[test]
    fn test_bridge_creation() {
        let bridge = ConsciousnessBridge::new(16384);
        let stats = bridge.stats();
        assert_eq!(stats.total_aggregated_features, 0);
    }
}
