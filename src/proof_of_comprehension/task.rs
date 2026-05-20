//! Comprehension Task — Tareas de validación de comprensión vía SAE activations.
//!
//! **Stuartian Law 2 (Reconocimiento del Error):** Los nodos deben demostrar
//! comprensión real procesando batches de activaciones SAE y validando gradientes.

use std::fmt;

/// Error al crear o procesar una tarea de comprensión.
#[derive(Debug)]
pub enum ComprehensionTaskError {
    /// Batch de activaciones vacío.
    EmptyActivationBatch,
    /// Dimensiones incompatibles.
    IncompatibleDimensions(String),
    /// Tarea expirada.
    TaskExpired,
    /// Error de serialización.
    Serialization(String),
}

impl fmt::Display for ComprehensionTaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComprehensionTaskError::EmptyActivationBatch => {
                write!(f, "Empty activation batch")
            }
            ComprehensionTaskError::IncompatibleDimensions(msg) => {
                write!(f, "Incompatible dimensions: {}", msg)
            }
            ComprehensionTaskError::TaskExpired => {
                write!(f, "Task expired")
            }
            ComprehensionTaskError::Serialization(msg) => {
                write!(f, "Serialization error: {}", msg)
            }
        }
    }
}

impl std::error::Error for ComprehensionTaskError {}

/// Estado de una tarea de comprensión.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskState {
    /// Tarea pendiente de asignación.
    Pending,
    /// Tarea asignada a un nodo.
    Assigned,
    /// Tarea en progreso.
    InProgress,
    /// Tarea completada exitosamente.
    Completed,
    /// Tarea fallida.
    Failed,
}

impl fmt::Display for TaskState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskState::Pending => write!(f, "Pending"),
            TaskState::Assigned => write!(f, "Assigned"),
            TaskState::InProgress => write!(f, "InProgress"),
            TaskState::Completed => write!(f, "Completed"),
            TaskState::Failed => write!(f, "Failed"),
        }
    }
}

/// Tarea de comprensión para validar trabajo útil.
///
/// **Stuartian Law 2:** En lugar de PoW (hash vacío), los nodos
/// procesan activaciones SAE reales y demuestran comprensión.
#[derive(Debug, Clone)]
pub struct ComprehensionTask {
    /// Identificador único de la tarea.
    pub task_id: String,
    /// Estado actual.
    pub state: TaskState,
    /// Dimensión de las activaciones SAE.
    pub activation_dim: usize,
    /// Número de muestras en el batch.
    pub batch_size: usize,
    /// Timeout en segundos.
    pub timeout_secs: u64,
}

impl ComprehensionTask {
    /// Crea una nueva tarea de comprensión.
    pub fn new(
        task_id: String,
        activation_dim: usize,
        batch_size: usize,
        timeout_secs: u64,
    ) -> Result<Self, ComprehensionTaskError> {
        if batch_size == 0 {
            return Err(ComprehensionTaskError::EmptyActivationBatch);
        }
        if activation_dim == 0 {
            return Err(ComprehensionTaskError::IncompatibleDimensions(
                "activation_dim must be > 0".into(),
            ));
        }
        Ok(Self {
            task_id,
            state: TaskState::Pending,
            activation_dim,
            batch_size,
            timeout_secs,
        })
    }

    /// Transiciona el estado de la tarea.
    pub fn transition(&mut self, new_state: TaskState) {
        self.state = new_state;
    }

    /// Valida que la tarea es procesable.
    pub fn validate(&self) -> Result<(), ComprehensionTaskError> {
        if self.batch_size == 0 {
            return Err(ComprehensionTaskError::EmptyActivationBatch);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = ComprehensionTask::new("task-1".into(), 2048, 32, 60).unwrap();
        assert_eq!(task.state, TaskState::Pending);
        assert_eq!(task.activation_dim, 2048);
    }

    #[test]
    fn test_task_empty_batch() {
        match ComprehensionTask::new("task-2".into(), 2048, 0, 60) {
            Err(ComprehensionTaskError::EmptyActivationBatch) => {}
            other => panic!("Expected EmptyActivationBatch, got {:?}", other),
        }
    }

    #[test]
    fn test_task_zero_dim() {
        match ComprehensionTask::new("task-3".into(), 0, 32, 60) {
            Err(ComprehensionTaskError::IncompatibleDimensions(_)) => {}
            other => panic!("Expected IncompatibleDimensions, got {:?}", other),
        }
    }

    #[test]
    fn test_task_transition() {
        let mut task = ComprehensionTask::new("task-4".into(), 2048, 32, 60).unwrap();
        task.transition(TaskState::Assigned);
        assert_eq!(task.state, TaskState::Assigned);
    }

    #[test]
    fn test_task_state_display() {
        assert_eq!(format!("{}", TaskState::Pending), "Pending");
        assert_eq!(format!("{}", TaskState::Completed), "Completed");
    }

    #[test]
    fn test_error_display() {
        let err = ComprehensionTaskError::TaskExpired;
        assert!(!format!("{}", err).is_empty());
    }
}
