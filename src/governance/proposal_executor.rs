//! Proposal Executor — Ejecutor de propuestas con time-lock y ledger
//!
//! Maneja la ejecución automática de propuestas aprobadas con:
//! - Cola de ejecución con time-lock
//! - Ledger inmutable de ejecuciones
//! - Rollback automático en caso de fallo
//! - Priorización por criticidad
//!
//! Feature-gated: `#[cfg(feature = "v1.2-sprint2")]`

use serde::{Deserialize, Serialize};
use std::collections::{BinaryHeap, HashMap};
use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Error del ejecutor de propuestas
#[derive(Debug, Error)]
pub enum ExecutorError {
    #[error("Proposal not found: {0}")]
    ProposalNotFound(String),
    #[error("Proposal not ready for execution: {0}")]
    NotReady(String),
    #[error("Time-lock active: {remaining:?} remaining")]
    TimeLockActive { remaining: Duration },
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Already executed: {0}")]
    AlreadyExecuted(String),
    #[error("Invalid state transition: {from} -> {to}")]
    InvalidTransition { from: String, to: String },
}

/// Estado de ejecución
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionState {
    /// En cola esperando time-lock
    Queued,
    /// Time-lock expirado, listo
    Ready,
    /// Ejecutando
    Executing,
    /// Completado exitosamente
    Completed,
    /// Fallido
    Failed,
    /// Revertido
    Reverted,
    /// Cancelado
    Cancelled,
}

impl fmt::Display for ExecutionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutionState::Queued => write!(f, "Queued"),
            ExecutionState::Ready => write!(f, "Ready"),
            ExecutionState::Executing => write!(f, "Executing"),
            ExecutionState::Completed => write!(f, "Completed"),
            ExecutionState::Failed => write!(f, "Failed"),
            ExecutionState::Reverted => write!(f, "Reverted"),
            ExecutionState::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Prioridad de propuesta
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProposalPriority {
    Critical,
    High,
    Normal,
    Low,
}

impl fmt::Display for ProposalPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProposalPriority::Critical => write!(f, "Critical"),
            ProposalPriority::High => write!(f, "High"),
            ProposalPriority::Normal => write!(f, "Normal"),
            ProposalPriority::Low => write!(f, "Low"),
        }
    }
}

impl ProposalPriority {
    fn order(&self) -> u8 {
        match self {
            ProposalPriority::Critical => 0,
            ProposalPriority::High => 1,
            ProposalPriority::Normal => 2,
            ProposalPriority::Low => 3,
        }
    }
}

/// Entrada del ledger de ejecución
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    /// ID de la propuesta
    pub proposal_id: String,
    /// Estado
    pub state: ExecutionState,
    /// Timestamp
    pub timestamp: u64,
    /// Mensaje
    pub message: String,
}

/// Propuesta ejecutable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutableProposal {
    /// ID único
    pub id: String,
    /// Título
    pub title: String,
    /// Prioridad
    pub priority: ProposalPriority,
    /// Estado actual
    pub state: ExecutionState,
    /// Timestamp de aprobación
    pub approved_at: u64,
    /// Duración del time-lock
    pub timelock_duration: Duration,
    /// Timestamp de ejecución (si aplica)
    pub executed_at: Option<u64>,
    /// Mensaje de error (si falla)
    pub error_message: Option<String>,
}

impl ExecutableProposal {
    pub fn new(
        id: String,
        title: String,
        priority: ProposalPriority,
        timelock_duration: Duration,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        ExecutableProposal {
            id,
            title,
            priority,
            state: ExecutionState::Queued,
            approved_at: timestamp,
            timelock_duration,
            executed_at: None,
            error_message: None,
        }
    }

    /// Verifica si el time-lock ha expirado
    pub fn is_timelock_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let elapsed = now.saturating_sub(self.approved_at);
        Duration::from_secs(elapsed) >= self.timelock_duration
    }

    /// Tiempo restante del time-lock
    pub fn timelock_remaining(&self) -> Option<Duration> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let elapsed = now.saturating_sub(self.approved_at);
        let elapsed_dur = Duration::from_secs(elapsed);
        if elapsed_dur >= self.timelock_duration {
            None
        } else {
            Some(self.timelock_duration - elapsed_dur)
        }
    }
}

// Implementación para BinaryHeap (cola de prioridad)
impl PartialEq for PriorityItem {
    fn eq(&self, other: &Self) -> bool {
        self.priority.order() == other.priority.order()
            && self.timestamp == other.timestamp
    }
}

impl Eq for PriorityItem {}

impl PartialOrd for PriorityItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Prioridad más baja (Critical=0) va primero → reverse ordering
        self.priority
            .order()
            .cmp(&other.priority.order())
            .then(self.timestamp.cmp(&other.timestamp))
            .reverse()
    }
}

struct PriorityItem {
    priority: ProposalPriority,
    timestamp: u64,
    proposal_id: String,
}

/// Configuración del ejecutor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfig {
    /// Time-lock por defecto para propuestas críticas
    pub critical_timelock: Duration,
    /// Time-lock por defecto para propuestas normales
    pub normal_timelock: Duration,
    /// Máximo de ejecuciones en cola
    pub max_queue_size: usize,
    /// Habilitar rollback automático
    pub auto_rollback: bool,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        ExecutorConfig {
            critical_timelock: Duration::from_secs(72 * 3600), // 72h
            normal_timelock: Duration::from_secs(24 * 3600), // 24h
            max_queue_size: 100,
            auto_rollback: true,
        }
    }
}

/// Resultado de ejecución
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOutcome {
    /// ID de la propuesta
    pub proposal_id: String,
    /// Exitosa
    pub success: bool,
    /// Estado final
    pub final_state: ExecutionState,
    /// Mensaje
    pub message: String,
    /// Timestamp
    pub timestamp: u64,
}

/// Ejecutor de propuestas
pub struct ProposalExecutor {
    config: ExecutorConfig,
    proposals: HashMap<String, ExecutableProposal>,
    queue: BinaryHeap<PriorityItem>,
    ledger: Vec<LedgerEntry>,
}

impl ProposalExecutor {
    pub fn new() -> Self {
        Self::with_config(ExecutorConfig::default())
    }

    pub fn with_config(config: ExecutorConfig) -> Self {
        ProposalExecutor {
            config,
            proposals: HashMap::new(),
            queue: BinaryHeap::new(),
            ledger: Vec::new(),
        }
    }

    /// Encola una propuesta para ejecución
    pub fn enqueue(
        &mut self,
        proposal: ExecutableProposal,
    ) -> Result<(), ExecutorError> {
        if self.proposals.len() >= self.config.max_queue_size {
            return Err(ExecutorError::ExecutionFailed(
                "Queue full".to_string(),
            ));
        }

        let id = proposal.id.clone();
        let priority = proposal.priority.clone();
        let approved_at = proposal.approved_at;

        self.proposals.insert(id.clone(), proposal);
        self.queue.push(PriorityItem {
            priority,
            timestamp: approved_at,
            proposal_id: id.clone(),
        });

        self.ledger.push(LedgerEntry {
            proposal_id: id,
            state: ExecutionState::Queued,
            timestamp: approved_at,
            message: "Proposal queued for execution".to_string(),
        });

        Ok(())
    }

    /// Actualiza estados basados en time-lock
    pub fn update_timelocks(&mut self) {
        let ids: Vec<String> = self.proposals.keys().cloned().collect();
        for id in ids {
            if let Some(proposal) = self.proposals.get_mut(&id) {
                if proposal.state == ExecutionState::Queued && proposal.is_timelock_expired() {
                    proposal.state = ExecutionState::Ready;
                    let timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    self.ledger.push(LedgerEntry {
                        proposal_id: id,
                        state: ExecutionState::Ready,
                        timestamp,
                        message: "Time-lock expired, proposal ready".to_string(),
                    });
                }
            }
        }
    }

    /// Ejecuta la siguiente propuesta lista (mayor prioridad)
    pub fn execute_next(&mut self) -> Option<ExecutionOutcome> {
        self.update_timelocks();

        // Encontrar la próxima propuesta lista con mayor prioridad
        let mut ready_proposals: Vec<&ExecutableProposal> = self
            .proposals
            .values()
            .filter(|p| p.state == ExecutionState::Ready)
            .collect();

        ready_proposals.sort_by(|a, b| a.priority.order().cmp(&b.priority.order()));

        let proposal = ready_proposals.first()?;
        let id = proposal.id.clone();

        // Marcar como ejecutando
        if let Some(p) = self.proposals.get_mut(&id) {
            p.state = ExecutionState::Executing;
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Simular ejecución exitosa
        if let Some(p) = self.proposals.get_mut(&id) {
            p.state = ExecutionState::Completed;
            p.executed_at = Some(timestamp);
        }

        self.ledger.push(LedgerEntry {
            proposal_id: id.clone(),
            state: ExecutionState::Completed,
            timestamp,
            message: "Proposal executed successfully".to_string(),
        });

        Some(ExecutionOutcome {
            proposal_id: id,
            success: true,
            final_state: ExecutionState::Completed,
            message: "Executed successfully".to_string(),
            timestamp,
        })
    }

    /// Ejecuta una propuesta específica
    pub fn execute_proposal(
        &mut self,
        proposal_id: &str,
    ) -> Result<ExecutionOutcome, ExecutorError> {
        let proposal = self.proposals.get(proposal_id).ok_or(
            ExecutorError::ProposalNotFound(proposal_id.to_string()),
        )?;

        if proposal.state == ExecutionState::Completed
            || proposal.state == ExecutionState::Reverted
        {
            return Err(ExecutorError::AlreadyExecuted(proposal_id.to_string()));
        }

        if proposal.state == ExecutionState::Queued {
            if let Some(remaining) = proposal.timelock_remaining() {
                return Err(ExecutorError::TimeLockActive { remaining });
            }
        }

        if proposal.state != ExecutionState::Ready
            && proposal.state != ExecutionState::Queued
        {
            return Err(ExecutorError::NotReady(proposal_id.to_string()));
        }

        // Marcar como ejecutando
        if let Some(p) = self.proposals.get_mut(proposal_id) {
            p.state = ExecutionState::Executing;
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Simular ejecución exitosa
        if let Some(p) = self.proposals.get_mut(proposal_id) {
            p.state = ExecutionState::Completed;
            p.executed_at = Some(timestamp);
        }

        self.ledger.push(LedgerEntry {
            proposal_id: proposal_id.to_string(),
            state: ExecutionState::Completed,
            timestamp,
            message: "Proposal executed successfully".to_string(),
        });

        Ok(ExecutionOutcome {
            proposal_id: proposal_id.to_string(),
            success: true,
            final_state: ExecutionState::Completed,
            message: "Executed successfully".to_string(),
            timestamp,
        })
    }

    /// Revierte una propuesta fallida
    pub fn rollback(
        &mut self,
        proposal_id: &str,
        error_message: String,
    ) -> Result<(), ExecutorError> {
        if !self.config.auto_rollback {
            return Err(ExecutorError::ExecutionFailed(
                "Auto-rollback disabled".to_string(),
            ));
        }

        let proposal = self.proposals.get(proposal_id).ok_or(
            ExecutorError::ProposalNotFound(proposal_id.to_string()),
        )?;

        if proposal.state != ExecutionState::Executing
            && proposal.state != ExecutionState::Failed
        {
            return Err(ExecutorError::InvalidTransition {
                from: format!("{:?}", proposal.state),
                to: "Reverted".to_string(),
            });
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if let Some(p) = self.proposals.get_mut(proposal_id) {
            p.state = ExecutionState::Reverted;
            p.error_message = Some(error_message.clone());
        }

        self.ledger.push(LedgerEntry {
            proposal_id: proposal_id.to_string(),
            state: ExecutionState::Reverted,
            timestamp,
            message: format!("Rolled back: {}", error_message),
        });

        Ok(())
    }

    /// Cancela una propuesta en cola
    pub fn cancel(&mut self, proposal_id: &str) -> Result<(), ExecutorError> {
        let proposal = self.proposals.get(proposal_id).ok_or(
            ExecutorError::ProposalNotFound(proposal_id.to_string()),
        )?;

        if proposal.state != ExecutionState::Queued
            && proposal.state != ExecutionState::Ready
        {
            return Err(ExecutorError::InvalidTransition {
                from: format!("{:?}", proposal.state),
                to: "Cancelled".to_string(),
            });
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if let Some(p) = self.proposals.get_mut(proposal_id) {
            p.state = ExecutionState::Cancelled;
        }

        self.ledger.push(LedgerEntry {
            proposal_id: proposal_id.to_string(),
            state: ExecutionState::Cancelled,
            timestamp,
            message: "Proposal cancelled".to_string(),
        });

        Ok(())
    }

    /// Obtiene propuesta
    pub fn get_proposal(&self, proposal_id: &str) -> Option<&ExecutableProposal> {
        self.proposals.get(proposal_id)
    }

    /// Obtiene ledger
    pub fn ledger(&self) -> &[LedgerEntry] {
        &self.ledger
    }

    /// Tamaño de cola
    pub fn queue_size(&self) -> usize {
        self.proposals
            .values()
            .filter(|p| {
                p.state == ExecutionState::Queued || p.state == ExecutionState::Ready
            })
            .count()
    }

    /// Propuestas listas
    pub fn ready_count(&self) -> usize {
        self.proposals
            .values()
            .filter(|p| p.state == ExecutionState::Ready)
            .count()
    }

    /// Config
    pub fn config(&self) -> &ExecutorConfig {
        &self.config
    }
}

impl Default for ProposalExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_proposal(id: &str, priority: ProposalPriority) -> ExecutableProposal {
        ExecutableProposal::new(
            id.to_string(),
            "Test Proposal".to_string(),
            priority,
            Duration::from_secs(0), // Sin time-lock para tests
        )
    }

    #[test]
    fn test_executor_creation() {
        let executor = ProposalExecutor::new();
        assert_eq!(executor.queue_size(), 0);
    }

    #[test]
    fn test_enqueue_proposal() {
        let mut executor = ProposalExecutor::new();
        let proposal = make_proposal("p1", ProposalPriority::Normal);
        executor.enqueue(proposal).unwrap();
        assert_eq!(executor.queue_size(), 1);
    }

    #[test]
    fn test_execute_proposal() {
        let mut executor = ProposalExecutor::new();
        let proposal = make_proposal("p1", ProposalPriority::Normal);
        executor.enqueue(proposal).unwrap();
        let outcome = executor.execute_proposal("p1");
        assert!(outcome.is_ok());
        assert!(outcome.unwrap().success);
    }

    #[test]
    fn test_execute_next_priority() {
        let mut executor = ProposalExecutor::new();
        executor.enqueue(make_proposal("low", ProposalPriority::Low)).unwrap();
        executor.enqueue(make_proposal("critical", ProposalPriority::Critical))
            .unwrap();
        executor.enqueue(make_proposal("high", ProposalPriority::High)).unwrap();
        let outcome = executor.execute_next().unwrap();
        assert_eq!(outcome.proposal_id, "critical");
    }

    #[test]
    fn test_rollback() {
        let mut executor = ProposalExecutor::new();
        let mut proposal = make_proposal("p1", ProposalPriority::Normal);
        proposal.state = ExecutionState::Executing;
        executor.proposals.insert("p1".to_string(), proposal);
        let rollback = executor.rollback("p1", "Test error".to_string());
        assert!(rollback.is_ok());
        assert_eq!(
            executor.get_proposal("p1").unwrap().state,
            ExecutionState::Reverted
        );
    }

    #[test]
    fn test_cancel() {
        let mut executor = ProposalExecutor::new();
        let proposal = make_proposal("p1", ProposalPriority::Normal);
        executor.enqueue(proposal).unwrap();
        executor.cancel("p1").unwrap();
        assert_eq!(
            executor.get_proposal("p1").unwrap().state,
            ExecutionState::Cancelled
        );
    }

    #[test]
    fn test_already_executed() {
        let mut executor = ProposalExecutor::new();
        let proposal = make_proposal("p1", ProposalPriority::Normal);
        executor.enqueue(proposal).unwrap();
        executor.execute_proposal("p1").unwrap();
        assert!(executor.execute_proposal("p1").is_err());
    }

    #[test]
    fn test_proposal_not_found() {
        let mut executor = ProposalExecutor::new();
        assert!(executor.execute_proposal("nonexistent").is_err());
    }

    #[test]
    fn test_ledger_entries() {
        let mut executor = ProposalExecutor::new();
        let proposal = make_proposal("p1", ProposalPriority::Normal);
        executor.enqueue(proposal).unwrap();
        executor.execute_proposal("p1").unwrap();
        assert_eq!(executor.ledger().len(), 2);
    }

    #[test]
    fn test_timelock_expired() {
        let proposal = ExecutableProposal::new(
            "p1".to_string(),
            "Test".to_string(),
            ProposalPriority::Normal,
            Duration::from_secs(1),
        );
        // Esperar un poco para que expire
        std::thread::sleep(Duration::from_secs(2));
        assert!(proposal.is_timelock_expired());
    }

    #[test]
    fn test_timelock_remaining() {
        let proposal = ExecutableProposal::new(
            "p1".to_string(),
            "Test".to_string(),
            ProposalPriority::Normal,
            Duration::from_secs(3600),
        );
        assert!(proposal.timelock_remaining().is_some());
    }

    #[test]
    fn test_state_display() {
        assert_eq!(format!("{}", ExecutionState::Queued), "Queued");
        assert_eq!(format!("{}", ExecutionState::Completed), "Completed");
    }

    #[test]
    fn test_priority_display() {
        assert_eq!(
            format!("{}", ProposalPriority::Critical),
            "Critical"
        );
        assert_eq!(format!("{}", ProposalPriority::Low), "Low");
    }

    #[test]
    fn test_config_default() {
        let config = ExecutorConfig::default();
        assert_eq!(config.critical_timelock, Duration::from_secs(72 * 3600));
        assert!(config.auto_rollback);
    }

    #[test]
    fn test_executor_default() {
        let executor = ProposalExecutor::default();
        assert_eq!(executor.queue_size(), 0);
    }

    #[test]
    fn test_ready_count() {
        let mut executor = ProposalExecutor::new();
        executor.enqueue(make_proposal("p1", ProposalPriority::Normal)).unwrap();
        executor.enqueue(make_proposal("p2", ProposalPriority::High)).unwrap();
        executor.update_timelocks();
        assert_eq!(executor.ready_count(), 2);
    }

    #[test]
    fn test_queue_full() {
        let mut executor = ProposalExecutor::with_config(ExecutorConfig {
            max_queue_size: 2,
            ..ExecutorConfig::default()
        });
        executor.enqueue(make_proposal("p1", ProposalPriority::Normal)).unwrap();
        executor.enqueue(make_proposal("p2", ProposalPriority::Normal)).unwrap();
        assert!(executor
            .enqueue(make_proposal("p3", ProposalPriority::Normal))
            .is_err());
    }

    #[test]
    fn test_rollback_disabled() {
        let mut executor = ProposalExecutor::with_config(ExecutorConfig {
            auto_rollback: false,
            ..ExecutorConfig::default()
        });
        assert!(executor.rollback("p1", "error".to_string()).is_err());
    }
}
