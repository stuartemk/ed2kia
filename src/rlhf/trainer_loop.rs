//! Trainer Loop - Generación de batches, validación, preparación para SFT/RL
//!
//! Agrupa feedback por ventana temporal, calcula distribución de conceptos,
//! detecta drift semántico y genera `TrainingBatch` listo para exportación.

// FIX: E0599 - writeln!/flush require Write trait in scope
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use super::feedback_store::{FeedbackDecision, FeedbackEntry, FeedbackStore};

/// Batch de entrenamiento listo para exportación
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingBatch {
    /// ID único del batch
    pub batch_id: String,
    /// Timestamp de creación (epoch ms)
    pub created_at_ms: u64,
    /// Ventana temporal del batch (en segundos)
    pub window_seconds: u64,
    /// Entradas de feedback incluidas
    pub entries: Vec<FeedbackEntry>,
    /// Estadísticas del batch
    pub statistics: BatchStatistics,
    /// Indicador de drift semántico detectado
    pub semantic_drift: Option<SemanticDriftReport>,
    /// Estado del batch
    pub state: BatchState,
}

impl TrainingBatch {
    pub fn new(batch_id: String, window_seconds: u64, entries: Vec<FeedbackEntry>) -> Self {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0));

        let statistics = Self::calculate_statistics(&entries);
        let semantic_drift = Self::detect_semantic_drift(&entries);

        Self {
            batch_id,
            created_at_ms: duration.as_millis() as u64,
            window_seconds,
            entries,
            statistics,
            semantic_drift,
            state: BatchState::Ready,
        }
    }

    fn calculate_statistics(entries: &[FeedbackEntry]) -> BatchStatistics {
        let mut approved = 0;
        let mut rejected = 0;
        let mut corrected = 0;
        let mut uncertain = 0;
        let mut total_confidence = 0.0;
        let mut concept_counts: HashMap<String, usize> = HashMap::new();
        let mut layer_counts: HashMap<String, usize> = HashMap::new();

        for entry in entries {
            match &entry.decision {
                FeedbackDecision::Approved => approved += 1,
                FeedbackDecision::Rejected => rejected += 1,
                FeedbackDecision::Corrected => corrected += 1,
                FeedbackDecision::Uncertain => uncertain += 1,
            }
            total_confidence += entry.model_confidence;

            if let Some(concept) = &entry.concept {
                *concept_counts.entry(concept.clone()).or_insert(0) += 1;
            }
            *layer_counts.entry(entry.layer_id.clone()).or_insert(0) += 1;
        }

        let total = entries.len() as f64;
        BatchStatistics {
            total_entries: entries.len(),
            approved,
            rejected,
            corrected,
            uncertain,
            avg_confidence: if total > 0.0 {
                total_confidence / total
            } else {
                0.0
            },
            approval_rate: if total > 0.0 {
                approved as f64 / total
            } else {
                0.0
            },
            rejection_rate: if total > 0.0 {
                rejected as f64 / total
            } else {
                0.0
            },
            unique_concepts: concept_counts.len(),
            unique_layers: layer_counts.len(),
            top_concepts: concept_counts
                .into_iter()
                .filter(|(_, count)| *count >= 3)
                .collect(),
        }
    }

    fn detect_semantic_drift(entries: &[FeedbackEntry]) -> Option<SemanticDriftReport> {
        if entries.len() < 10 {
            return None;
        }

        // Calcula tasa de rechazo por concepto
        let mut concept_approvals: HashMap<String, usize> = HashMap::new();
        let mut concept_rejections: HashMap<String, usize> = HashMap::new();

        for entry in entries {
            if let Some(concept) = &entry.concept {
                match &entry.decision {
                    FeedbackDecision::Approved => {
                        *concept_approvals.entry(concept.clone()).or_insert(0) += 1;
                    }
                    FeedbackDecision::Rejected => {
                        *concept_rejections.entry(concept.clone()).or_insert(0) += 1;
                    }
                    _ => {}
                }
            }
        }

        // Detecta conceptos con alta tasa de rechazo (>40%)
        let drifted_concepts: Vec<(String, f64)> = concept_approvals
            .keys()
            .filter_map(|concept| {
                let approvals = concept_approvals[concept];
                let rejections = concept_rejections.get(concept).copied().unwrap_or(0);
                let total = approvals + rejections;
                if total >= 5 {
                    let rejection_rate = rejections as f64 / total as f64;
                    if rejection_rate > 0.4 {
                        return Some((concept.clone(), rejection_rate));
                    }
                }
                None
            })
            .collect();

        if !drifted_concepts.is_empty() {
            // FIX: borrow/move - Extract length before moving drifted_concepts
            let drift_count = drifted_concepts.len();
            Some(SemanticDriftReport {
                drift_detected: true,
                drifted_concepts,
                severity: if drift_count > 5 {
                    DriftSeverity::High
                } else if drift_count > 2 {
                    DriftSeverity::Medium
                } else {
                    DriftSeverity::Low
                },
                recommendation: format!(
                    "Review {} concepts with high rejection rate. Consider retraining or updating semantic map.",
                    drift_count
                ),
            })
        } else {
            None
        }
    }

    /// Marca batch como exportado
    pub fn mark_exported(&mut self) {
        self.state = BatchState::Exported;
    }

    /// Marca batch como procesado (fine-tuning iniciado)
    pub fn mark_processed(&mut self) {
        self.state = BatchState::Processed;
    }
}

/// Estadísticas del batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchStatistics {
    pub total_entries: usize,
    pub approved: usize,
    pub rejected: usize,
    pub corrected: usize,
    pub uncertain: usize,
    pub avg_confidence: f64,
    pub approval_rate: f64,
    pub rejection_rate: f64,
    pub unique_concepts: usize,
    pub unique_layers: usize,
    pub top_concepts: Vec<(String, usize)>,
}

/// Reporte de drift semántico
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticDriftReport {
    pub drift_detected: bool,
    /// (concept, rejection_rate)
    pub drifted_concepts: Vec<(String, f64)>,
    pub severity: DriftSeverity,
    pub recommendation: String,
}

/// Severidad del drift
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DriftSeverity {
    Low,
    Medium,
    High,
}

impl std::fmt::Display for DriftSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DriftSeverity::Low => write!(f, "low"),
            DriftSeverity::Medium => write!(f, "medium"),
            DriftSeverity::High => write!(f, "high"),
        }
    }
}

/// Estado del batch
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatchState {
    Pending,
    Ready,
    Exported,
    Processed,
}

/// Configuración del TrainerLoop
#[derive(Debug, Clone)]
pub struct TrainerLoopConfig {
    /// Ventana temporal para batches (en segundos)
    pub batch_window_seconds: u64,
    /// Mínimo de entradas para crear batch
    pub min_entries_per_batch: usize,
    /// Umbral de drift para alertas (tasa de rechazo)
    pub drift_threshold: f64,
    /// Directorio de exportación
    pub export_directory: PathBuf,
    /// Habilitar detección automática de drift
    pub auto_drift_detection: bool,
}

impl Default for TrainerLoopConfig {
    fn default() -> Self {
        Self {
            batch_window_seconds: 3600, // 1 hora
            min_entries_per_batch: 50,
            drift_threshold: 0.4,
            export_directory: PathBuf::from("./data/training_batches"),
            auto_drift_detection: true,
        }
    }
}

/// Loop principal de entrenamiento RLHF
pub struct TrainerLoop {
    /// Store de feedback
    feedback_store: Arc<FeedbackStore>,
    /// Configuración
    config: TrainerLoopConfig,
    /// Batches generados
    batches: RwLock<Vec<TrainingBatch>>,
    /// Flag para ejecutar loop
    running: AtomicBool,
    /// Contador de batches generados
    batch_counter: AtomicU64,
    /// Último timestamp procesado
    last_processed_ms: RwLock<u64>,
}

impl TrainerLoop {
    pub fn new(feedback_store: Arc<FeedbackStore>, config: TrainerLoopConfig) -> Self {
        // Crear directorio de exportación
        if let Err(e) = std::fs::create_dir_all(&config.export_directory) {
            warn!(
                dir = %config.export_directory.display(),
                error = %e,
                "Failed to create export directory"
            );
        }

        Self {
            feedback_store,
            config,
            batches: RwLock::new(Vec::new()),
            running: AtomicBool::new(false),
            batch_counter: AtomicU64::new(0),
            last_processed_ms: RwLock::new(0),
        }
    }

    /// Inicia el loop de procesamiento
    pub fn start(&self) {
        self.running.store(true, Ordering::SeqCst);
        info!("Trainer loop started");
    }

    /// Detiene el loop de procesamiento
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        info!("Trainer loop stopped");
    }

    /// Verifica si el loop está corriendo
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Ejecuta un ciclo de procesamiento de batches
    pub fn process_cycle(&self) -> Result<Option<TrainingBatch>, TrainerLoopError> {
        if !self.is_running() {
            return Ok(None);
        }

        let now_ms = Self::current_timestamp_ms();
        let last_processed = *self.last_processed_ms.read();

        // Obtener feedback desde último procesamiento
        let new_entries = self
            .feedback_store
            .get_feedback_in_range(last_processed, now_ms)?;

        if new_entries.is_empty() {
            return Ok(None);
        }

        // Verificar si hay suficientes entradas para un batch
        if new_entries.len() < self.config.min_entries_per_batch {
            debug!(
                entries = new_entries.len(),
                min_required = self.config.min_entries_per_batch,
                "Not enough entries for batch"
            );
            // Actualizar timestamp pero no crear batch
            *self.last_processed_ms.write() = now_ms;
            return Ok(None);
        }

        // Crear batch
        let batch_num = self.batch_counter.fetch_add(1, Ordering::SeqCst);
        let batch_id = format!("batch_{}_{}", batch_num, now_ms);

        let batch = TrainingBatch::new(batch_id, self.config.batch_window_seconds, new_entries);

        // Guardar batch
        self.batches.write().push(batch.clone());
        *self.last_processed_ms.write() = now_ms;

        info!(
            batch_id = &batch.batch_id,
            entries = batch.statistics.total_entries,
            approval_rate = batch.statistics.approval_rate,
            drift_detected = batch.semantic_drift.is_some(),
            "Training batch created"
        );

        // Alertar si hay drift
        if let Some(drift) = &batch.semantic_drift {
            warn!(
                severity = %drift.severity,
                drifted_concepts = drift.drifted_concepts.len(),
                "Semantic drift detected in batch"
            );
        }

        Ok(Some(batch))
    }

    /// Exporta batch a JSONL
    pub fn export_batch(&self, batch_id: &str) -> Result<PathBuf, TrainerLoopError> {
        let batches = self.batches.read();
        let batch = batches
            .iter()
            .find(|b| b.batch_id == batch_id)
            .ok_or_else(|| TrainerLoopError::BatchNotFound(batch_id.to_string()))?;

        let path = self
            .config
            .export_directory
            .join(format!("{}.jsonl", batch_id));

        let file = std::fs::File::create(&path)?;
        let mut writer = std::io::BufWriter::new(file);

        for entry in &batch.entries {
            let line = entry.to_jsonl_training_format();
            writeln!(writer, "{}", line)?;
        }

        writer.flush()?;

        // Marcar batch como exportado
        drop(batches); // Liberar lock de lectura
        if let Some(batch) = self
            .batches
            .write()
            .iter_mut()
            .find(|b| b.batch_id == batch_id)
        {
            batch.mark_exported();
        }

        info!(
            batch_id,
            path = %path.display(),
            "Batch exported to JSONL"
        );

        Ok(path)
    }

    /// Exporta todos los batches listos
    pub fn export_all_ready_batches(&self) -> Result<Vec<PathBuf>, TrainerLoopError> {
        let batch_ids: Vec<String> = self
            .batches
            .read()
            .iter()
            .filter(|b| b.state == BatchState::Ready)
            .map(|b| b.batch_id.clone())
            .collect();

        let mut paths = Vec::new();
        for batch_id in batch_ids {
            let path = self.export_batch(&batch_id)?;
            paths.push(path);
        }

        Ok(paths)
    }

    /// Obtiene estadísticas del loop
    pub fn stats(&self) -> TrainerLoopStats {
        let batches = self.batches.read();
        let total = batches.len();
        let ready = batches
            .iter()
            .filter(|b| b.state == BatchState::Ready)
            .count();
        let exported = batches
            .iter()
            .filter(|b| b.state == BatchState::Exported)
            .count();
        let processed = batches
            .iter()
            .filter(|b| b.state == BatchState::Processed)
            .count();

        let total_entries = batches.iter().map(|b| b.statistics.total_entries).sum();
        let drift_batches = batches
            .iter()
            .filter(|b| b.semantic_drift.is_some())
            .count();

        TrainerLoopStats {
            running: self.is_running(),
            total_batches: total,
            ready_batches: ready,
            exported_batches: exported,
            processed_batches: processed,
            total_entries_processed: total_entries,
            drift_detected_batches: drift_batches,
            batch_window_seconds: self.config.batch_window_seconds,
            min_entries_per_batch: self.config.min_entries_per_batch,
        }
    }

    /// Obtiene todos los batches
    pub fn get_batches(&self) -> Vec<TrainingBatch> {
        self.batches.read().clone()
    }

    /// Obtiene batch por ID
    pub fn get_batch(&self, batch_id: &str) -> Option<TrainingBatch> {
        self.batches
            .read()
            .iter()
            .find(|b| b.batch_id == batch_id)
            .cloned()
    }

    fn current_timestamp_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0))
            .as_millis() as u64
    }
}

/// Estadísticas del TrainerLoop
#[derive(Debug)]
pub struct TrainerLoopStats {
    pub running: bool,
    pub total_batches: usize,
    pub ready_batches: usize,
    pub exported_batches: usize,
    pub processed_batches: usize,
    pub total_entries_processed: usize,
    pub drift_detected_batches: usize,
    pub batch_window_seconds: u64,
    pub min_entries_per_batch: usize,
}

/// Errores del TrainerLoop
#[derive(Debug, thiserror::Error)]
pub enum TrainerLoopError {
    #[error("Feedback store error: {0}")]
    FeedbackStore(#[from] super::feedback_store::FeedbackStoreError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Batch not found: {0}")]
    BatchNotFound(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rlhf::feedback_store::FeedbackStoreConfig;
    use tempfile::TempDir;

    fn create_test_store() -> (Arc<FeedbackStore>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = FeedbackStoreConfig {
            db_path: temp_dir.path().join("test_feedback.redb"),
            ..FeedbackStoreConfig::default()
        };
        let store = FeedbackStore::new(config).unwrap();
        (Arc::new(store), temp_dir)
    }

    #[test]
    fn test_trainer_loop_creation() {
        let (store, _temp) = create_test_store();
        let config = TrainerLoopConfig::default();
        let loop_ = TrainerLoop::new(store, config);
        assert!(!loop_.is_running());
    }

    #[test]
    fn test_batch_statistics() {
        let entries = vec![
            FeedbackEntry::new(
                "1".to_string(),
                "layer_0".to_string(),
                0,
                0.9,
                FeedbackDecision::Approved,
                "annotator_1".to_string(),
            ),
            FeedbackEntry::new(
                "2".to_string(),
                "layer_0".to_string(),
                1,
                0.3,
                FeedbackDecision::Rejected,
                "annotator_1".to_string(),
            ),
        ];

        let stats = TrainingBatch::calculate_statistics(&entries);
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.approved, 1);
        assert_eq!(stats.rejected, 1);
        assert!((stats.approval_rate - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_batch_creation() {
        let entries = vec![FeedbackEntry::new(
            "1".to_string(),
            "layer_0".to_string(),
            0,
            0.9,
            FeedbackDecision::Approved,
            "annotator_1".to_string(),
        )];

        let batch = TrainingBatch::new("test_batch".to_string(), 3600, entries);
        assert_eq!(batch.batch_id, "test_batch");
        assert_eq!(batch.state, BatchState::Ready);
        assert_eq!(batch.statistics.total_entries, 1);
    }
}
