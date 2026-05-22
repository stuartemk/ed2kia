//! Human-in-the-Loop Feedback CLI
//!
//! Interfaz para etiquetado humano de features y conceptos:
//! - Modo interactivo (TTY) con crossterm
//! - Modo batch (JSON stdin/stdout) para automatización
//! - Etiquetado de features: aprobar, rechazar, corregir concepto
//! - Exportación de feedback para RLHF
//! - Validación de consistencia semántica

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{debug, info, warn};

/// Entrada de feedback humano
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanFeedback {
    /// ID único del feedback
    pub id: String,
    /// ID del batch de features evaluado
    pub batch_id: String,
    /// Índice de la feature evaluada
    pub feature_index: usize,
    /// Activación de la feature
    pub activation_value: f64,
    /// Concepto reportado por el sistema
    pub reported_concept: String,
    /// Decisión del humano
    pub decision: FeedbackDecision,
    /// Concepto corregido (si aplica)
    pub corrected_concept: Option<String>,
    /// Comentarios adicionales
    pub comments: Option<String>,
    /// Timestamp (ns desde epoch)
    pub timestamp: u128,
    /// ID del humano/etiquetador
    pub annotator_id: String,
}

/// Decisión de feedback
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackDecision {
    /// Concepto aprobado por el humano
    Approved,
    /// Concepto rechazado (incorrecto)
    Rejected,
    /// Concepto corregido por el humano
    Corrected,
    /// Humano no está seguro
    Uncertain,
}

impl std::fmt::Display for FeedbackDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeedbackDecision::Approved => write!(f, "APPROVED"),
            FeedbackDecision::Rejected => write!(f, "REJECTED"),
            FeedbackDecision::Corrected => write!(f, "CORRECTED"),
            FeedbackDecision::Uncertain => write!(f, "UNCERTAIN"),
        }
    }
}

/// Solicitud de etiquetado
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelingRequest {
    pub batch_id: String,
    pub feature_index: usize,
    pub activation_value: f64,
    pub reported_concept: String,
    pub confidence: f64,
    pub context_hint: Option<String>,
}

/// Estadísticas de feedback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackStats {
    pub total_feedback: u64,
    pub approved: u64,
    pub rejected: u64,
    pub corrected: u64,
    pub uncertain: u64,
    pub approval_rate: f64,
    pub annotators: usize,
}

/// Gestor de feedback humano
pub struct FeedbackManager {
    /// Historial de feedback
    feedback_history: Vec<HumanFeedback>,
    /// Feedback pendiente de procesamiento
    pending_queue: Vec<HumanFeedback>,
    /// Contadores por decisión
    approved_count: AtomicU64,
    rejected_count: AtomicU64,
    corrected_count: AtomicU64,
    uncertain_count: AtomicU64,
    /// Annotadores únicos
    annotators: HashMap<String, u64>,
    /// Path para exportación de feedback
    export_path: Option<PathBuf>,
}

impl FeedbackManager {
    /// Crea un nuevo gestor de feedback
    pub fn new(export_path: Option<PathBuf>) -> Self {
        Self {
            feedback_history: Vec::new(),
            pending_queue: Vec::new(),
            approved_count: AtomicU64::new(0),
            rejected_count: AtomicU64::new(0),
            corrected_count: AtomicU64::new(0),
            uncertain_count: AtomicU64::new(0),
            annotators: HashMap::new(),
            export_path,
        }
    }

    /// Inicia modo interactivo (TTY)
    pub fn run_interactive(
        &mut self,
        requests: Vec<LabelingRequest>,
        annotator_id: &str,
    ) -> Result<()> {
        if requests.is_empty() {
            info!("No labeling requests available");
            return Ok(());
        }

        info!(
            "Starting interactive labeling session: {} requests, annotator={}",
            requests.len(),
            annotator_id
        );

        println!("\n{}", "=".repeat(60));
        println!("  ed2kIA - Human-in-the-Loop Feedback");
        println!("  Requests: {}", requests.len());
        println!("  Annotator: {}", annotator_id);
        println!("{}", "=".repeat(60));
        println!("\nCommands:");
        println!("  [a] Approve - Concept is correct");
        println!("  [r] Reject  - Concept is incorrect");
        println!("  [c] Correct - Provide corrected concept");
        println!("  [u] Uncertain - Not sure");
        println!("  [q] Quit    - Save and exit\n");

        for (i, request) in requests.iter().enumerate() {
            self.label_single(request, annotator_id, i + 1, requests.len())?;
            println!();
        }

        // Procesa feedback pendiente
        self.process_pending()?;

        info!(
            "Interactive session completed: {} labels collected",
            self.feedback_history.len()
        );
        Ok(())
    }

    /// Etiqueta una solicitud individual
    fn label_single(
        &mut self,
        request: &LabelingRequest,
        annotator_id: &str,
        current: usize,
        total: usize,
    ) -> Result<()> {
        println!("--- Request {}/{} ---", current, total);
        println!("  Batch: {}", request.batch_id);
        println!("  Feature: #{}", request.feature_index);
        println!("  Activation: {:.4}", request.activation_value);
        println!("  Reported Concept: {}", request.reported_concept);
        println!("  Confidence: {:.2}%", request.confidence * 100.0);

        if let Some(hint) = &request.context_hint {
            println!("  Context: {}", hint);
        }

        print!("\n  Decision [a/r/c/u/q]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().lock().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        let decision = match input.as_str() {
            "a" | "approve" => FeedbackDecision::Approved,
            "r" | "reject" => FeedbackDecision::Rejected,
            "c" | "correct" => {
                print!("  Corrected concept: ");
                io::stdout().flush()?;

                let mut correction = String::new();
                io::stdin().lock().read_line(&mut correction)?;
                let correction = correction.trim().to_string();

                if correction.is_empty() {
                    warn!("Empty correction, treating as reject");
                    FeedbackDecision::Rejected
                } else {
                    // Guarda corrección para procesar después
                    FeedbackDecision::Corrected
                }
            }
            "u" | "uncertain" => FeedbackDecision::Uncertain,
            "q" | "quit" => {
                info!("User quit labeling session");
                return Ok(());
            }
            _ => {
                warn!("Invalid input: '{}', skipping", input);
                return Ok(());
            }
        };

        // Pide comentarios opcionales
        let mut comments = None;
        if decision == FeedbackDecision::Corrected || decision == FeedbackDecision::Rejected {
            print!("  Comments (optional, press Enter to skip): ");
            io::stdout().flush()?;

            let mut comment_input = String::new();
            io::stdin().lock().read_line(&mut comment_input)?;
            let comment = comment_input.trim().to_string();
            if !comment.is_empty() {
                comments = Some(comment);
            }
        }

        // Para corrected, obtiene el concepto corregido
        let corrected_concept = if decision == FeedbackDecision::Corrected {
            print!("  Enter corrected concept: ");
            io::stdout().flush()?;

            let mut correction = String::new();
            io::stdin().lock().read_line(&mut correction)?;
            Some(correction.trim().to_string())
        } else {
            None
        };

        let feedback = HumanFeedback {
            id: format!("fb-{}-{}", annotator_id, self.feedback_history.len()),
            batch_id: request.batch_id.clone(),
            feature_index: request.feature_index,
            activation_value: request.activation_value,
            reported_concept: request.reported_concept.clone(),
            decision: decision.clone(),
            corrected_concept,
            comments,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            annotator_id: annotator_id.to_string(),
        };

        self.submit_feedback(feedback)?;
        Ok(())
    }

    /// Procesa feedback desde JSON (modo batch)
    pub fn process_json_feedback(&mut self, json_input: &str) -> Result<Vec<HumanFeedback>> {
        let feedbacks: Vec<HumanFeedback> = serde_json::from_str(json_input)
            .map_err(|e| anyhow!("Failed to parse JSON feedback: {}", e))?;

        let mut processed = Vec::new();
        for feedback in feedbacks {
            self.validate_feedback(&feedback)?;
            self.submit_feedback(feedback.clone())?;
            processed.push(feedback);
        }

        info!("Processed {} JSON feedback entries", processed.len());
        Ok(processed)
    }

    /// Genera solicitudes de etiquetado desde features
    pub fn generate_labeling_requests(
        &self,
        batch_id: &str,
        features: &[(usize, f64, String)],
        _annotator_id: &str,
    ) -> Vec<LabelingRequest> {
        features
            .iter()
            .map(|(feature_index, activation, concept)| LabelingRequest {
                batch_id: batch_id.to_string(),
                feature_index: *feature_index,
                activation_value: *activation,
                reported_concept: concept.clone(),
                confidence: 0.7, // Placeholder
                context_hint: None,
            })
            .collect()
    }

    /// Valida un feedback
    fn validate_feedback(&self, feedback: &HumanFeedback) -> Result<()> {
        if feedback.batch_id.is_empty() {
            return Err(anyhow!("Batch ID cannot be empty"));
        }

        if feedback.reported_concept.is_empty() {
            return Err(anyhow!("Reported concept cannot be empty"));
        }

        if feedback.decision == FeedbackDecision::Corrected
            && feedback
                .corrected_concept
                .as_ref()
                .is_none_or(|s| s.is_empty())
        {
            return Err(anyhow!("Corrected decision requires a corrected concept"));
        }

        Ok(())
    }

    /// Envía feedback al sistema
    fn submit_feedback(&mut self, feedback: HumanFeedback) -> Result<()> {
        self.validate_feedback(&feedback)?;

        // Actualiza contadores
        match &feedback.decision {
            FeedbackDecision::Approved => {
                self.approved_count.fetch_add(1, Ordering::Relaxed);
            }
            FeedbackDecision::Rejected => {
                self.rejected_count.fetch_add(1, Ordering::Relaxed);
            }
            FeedbackDecision::Corrected => {
                self.corrected_count.fetch_add(1, Ordering::Relaxed);
            }
            FeedbackDecision::Uncertain => {
                self.uncertain_count.fetch_add(1, Ordering::Relaxed);
            }
        }

        // Registra annotador
        *self
            .annotators
            .entry(feedback.annotator_id.clone())
            .or_insert(0) += 1;

        // Añade a historial y cola pendiente
        let feedback_id = feedback.id.clone();
        self.feedback_history.push(feedback.clone());
        self.pending_queue.push(feedback);

        debug!("Feedback submitted: {:?}", feedback_id);
        Ok(())
    }

    /// Procesa feedback pendiente y genera actualizaciones
    pub fn process_pending(&mut self) -> Result<Vec<ConceptUpdate>> {
        let pending: Vec<HumanFeedback> = self.pending_queue.drain(..).collect();
        let mut updates = Vec::new();

        for feedback in pending {
            match feedback.decision {
                FeedbackDecision::Corrected => {
                    if let Some(corrected) = &feedback.corrected_concept {
                        updates.push(ConceptUpdate {
                            feature_index: feedback.feature_index,
                            old_concept: feedback.reported_concept,
                            new_concept: corrected.clone(),
                            confidence: 1.0,
                            annotator_id: feedback.annotator_id,
                        });
                    }
                }
                FeedbackDecision::Rejected => {
                    updates.push(ConceptUpdate {
                        feature_index: feedback.feature_index,
                        old_concept: feedback.reported_concept,
                        new_concept: "UNKNOWN".to_string(),
                        confidence: 0.0,
                        annotator_id: feedback.annotator_id,
                    });
                }
                _ => {}
            }
        }

        if !updates.is_empty() {
            info!("Generated {} concept updates from feedback", updates.len());
        }

        // Exporta si hay path configurado
        if let Some(ref path) = self.export_path {
            self.export_to_json(path)?;
        }

        Ok(updates)
    }

    /// Exporta feedback a JSON
    pub fn export_to_json(&self, path: &PathBuf) -> Result<()> {
        let parent = path
            .parent()
            .ok_or_else(|| anyhow!("Invalid export path"))?;
        std::fs::create_dir_all(parent)?;

        let json = serde_json::to_string_pretty(&self.feedback_history)
            .map_err(|e| anyhow!("Failed to serialize feedback: {}", e))?;

        std::fs::write(path, json).map_err(|e| anyhow!("Failed to write feedback file: {}", e))?;

        info!(
            "Exported {} feedback entries to {}",
            self.feedback_history.len(),
            path.display()
        );
        Ok(())
    }

    /// Obtiene estadísticas de feedback
    pub fn get_stats(&self) -> FeedbackStats {
        let total = self.feedback_history.len() as u64;
        let approved = self.approved_count.load(Ordering::Relaxed);
        let rejected = self.rejected_count.load(Ordering::Relaxed);
        let corrected = self.corrected_count.load(Ordering::Relaxed);
        let uncertain = self.uncertain_count.load(Ordering::Relaxed);

        FeedbackStats {
            total_feedback: total,
            approved,
            rejected,
            corrected,
            uncertain,
            approval_rate: if total > 0 {
                approved as f64 / total as f64
            } else {
                0.0
            },
            annotators: self.annotators.len(),
        }
    }

    /// Obtiene historial de feedback
    pub fn get_history(&self, limit: Option<usize>) -> Vec<HumanFeedback> {
        let limit = limit.unwrap_or(100);
        self.feedback_history
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Obtiene feedback por annotador
    pub fn get_feedback_by_annotator(&self, annotator_id: &str) -> Vec<HumanFeedback> {
        self.feedback_history
            .iter()
            .filter(|f| f.annotator_id == annotator_id)
            .cloned()
            .collect()
    }

    /// Obtiene correcciones de conceptos
    pub fn get_corrections(&self) -> Vec<HumanFeedback> {
        self.feedback_history
            .iter()
            .filter(|f| f.decision == FeedbackDecision::Corrected)
            .cloned()
            .collect()
    }
}

/// Actualización de concepto basada en feedback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptUpdate {
    pub feature_index: usize,
    pub old_concept: String,
    pub new_concept: String,
    pub confidence: f64,
    pub annotator_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feedback_manager_creation() {
        let manager = FeedbackManager::new(None);
        let stats = manager.get_stats();
        assert_eq!(stats.total_feedback, 0);
        assert_eq!(stats.annotators, 0);
    }

    #[test]
    fn test_feedback_validation() {
        let manager = FeedbackManager::new(None);

        let feedback = HumanFeedback {
            id: "test-1".to_string(),
            batch_id: "batch-1".to_string(),
            feature_index: 0,
            activation_value: 0.5,
            reported_concept: "test_concept".to_string(),
            decision: FeedbackDecision::Approved,
            corrected_concept: None,
            comments: None,
            timestamp: 0,
            annotator_id: "annotator-1".to_string(),
        };

        assert!(manager.validate_feedback(&feedback).is_ok());
    }

    #[test]
    fn test_empty_batch_id_rejected() {
        let manager = FeedbackManager::new(None);

        let feedback = HumanFeedback {
            id: "test-2".to_string(),
            batch_id: "".to_string(),
            feature_index: 0,
            activation_value: 0.5,
            reported_concept: "test".to_string(),
            decision: FeedbackDecision::Approved,
            corrected_concept: None,
            comments: None,
            timestamp: 0,
            annotator_id: "a1".to_string(),
        };

        assert!(manager.validate_feedback(&feedback).is_err());
    }

    #[test]
    fn test_corrected_without_concept_rejected() {
        let manager = FeedbackManager::new(None);

        let feedback = HumanFeedback {
            id: "test-3".to_string(),
            batch_id: "b1".to_string(),
            feature_index: 0,
            activation_value: 0.5,
            reported_concept: "old".to_string(),
            decision: FeedbackDecision::Corrected,
            corrected_concept: None,
            comments: None,
            timestamp: 0,
            annotator_id: "a1".to_string(),
        };

        assert!(manager.validate_feedback(&feedback).is_err());
    }

    #[test]
    fn test_json_feedback_processing() {
        let mut manager = FeedbackManager::new(None);

        let json = r#"[
            {
                "id": "fb-1",
                "batch_id": "batch-1",
                "feature_index": 0,
                "activation_value": 0.8,
                "reported_concept": "sentiment",
                "decision": "approved",
                "corrected_concept": null,
                "comments": null,
                "timestamp": 0,
                "annotator_id": "human-1"
            }
        ]"#;

        let result = manager.process_json_feedback(json).unwrap();
        assert_eq!(result.len(), 1);

        let stats = manager.get_stats();
        assert_eq!(stats.total_feedback, 1);
        assert_eq!(stats.approved, 1);
    }

    #[test]
    fn test_feedback_stats() {
        let mut manager = FeedbackManager::new(None);

        let feedback1 = HumanFeedback {
            id: "fb-1".to_string(),
            batch_id: "b1".to_string(),
            feature_index: 0,
            activation_value: 0.5,
            reported_concept: "c1".to_string(),
            decision: FeedbackDecision::Approved,
            corrected_concept: None,
            comments: None,
            timestamp: 0,
            annotator_id: "a1".to_string(),
        };

        let feedback2 = HumanFeedback {
            id: "fb-2".to_string(),
            batch_id: "b1".to_string(),
            feature_index: 1,
            activation_value: 0.3,
            reported_concept: "c2".to_string(),
            decision: FeedbackDecision::Rejected,
            corrected_concept: None,
            comments: None,
            timestamp: 0,
            annotator_id: "a2".to_string(),
        };

        manager.submit_feedback(feedback1).unwrap();
        manager.submit_feedback(feedback2).unwrap();

        let stats = manager.get_stats();
        assert_eq!(stats.total_feedback, 2);
        assert_eq!(stats.approved, 1);
        assert_eq!(stats.rejected, 1);
        assert!((stats.approval_rate - 0.5).abs() < 0.01);
        assert_eq!(stats.annotators, 2);
    }

    #[test]
    fn test_generate_labeling_requests() {
        let manager = FeedbackManager::new(None);
        let features = vec![
            (0, 0.8, "positive_sentiment".to_string()),
            (1, 0.3, "factual_claim".to_string()),
        ];

        let requests = manager.generate_labeling_requests("batch-1", &features, "annotator-1");
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].feature_index, 0);
        assert_eq!(requests[1].feature_index, 1);
    }

    #[test]
    fn test_feedback_decision_display() {
        assert_eq!(format!("{}", FeedbackDecision::Approved), "APPROVED");
        assert_eq!(format!("{}", FeedbackDecision::Rejected), "REJECTED");
        assert_eq!(format!("{}", FeedbackDecision::Corrected), "CORRECTED");
        assert_eq!(format!("{}", FeedbackDecision::Uncertain), "UNCERTAIN");
    }
}
