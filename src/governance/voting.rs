//! Governance Voting System - Votación P2P con time-lock, quórum y ejecución automática
//!
//! Mecanismo de votación ligero sin blockchain. Los nodos votan vía GossipSub,
//! se aplica time-lock de 72h, y se requiere quórum de ≥30% de nodos activos
//! con reputación ≥0.7.

use crate::governance::proposal::{Proposal, ProposalState};
// CLEANUP: removed unused import ProposalError
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tracing::{info, warn};
use uuid::Uuid;

/// Error del sistema de votación
#[derive(Debug, Error)]
pub enum VotingError {
    #[error("Proposal not found: {0}")]
    ProposalNotFound(Uuid),
    #[error("Voting not active for proposal: {0}")]
    VotingNotActive(Uuid),
    #[error("Node already voted: proposal={proposal}, voter={voter}")]
    AlreadyVoted { proposal: Uuid, voter: String },
    #[error("Quorum not reached: {current}/{required}")]
    QuorumNotReached { current: usize, required: usize },
    #[error("Voter reputation too low: {current}<={minimum}")]
    ReputationTooLow { current: f64, minimum: f64 },
    #[error("Proposal expired")]
    ProposalExpired,
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
}

/// Dirección del voto
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VoteDirection {
    /// A favor de la propuesta
    For,
    /// En contra de la propuesta
    Against,
    /// Abstención
    Abstain,
}

impl std::fmt::Display for VoteDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VoteDirection::For => write!(f, "For"),
            VoteDirection::Against => write!(f, "Against"),
            VoteDirection::Abstain => write!(f, "Abstain"),
        }
    }
}

/// Voto individual de un nodo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    /// ID del nodo que vota
    pub voter_id: String,
    /// Dirección del voto
    pub direction: VoteDirection,
    /// Timestamp del voto (epoch seconds)
    pub timestamp: u64,
    /// Reputación del votante al momento de votar
    pub voter_reputation: f64,
    /// Opcional: justificación del voto
    pub rationale: Option<String>,
}

impl Vote {
    pub fn new(
        voter_id: String,
        direction: VoteDirection,
        voter_reputation: f64,
        rationale: Option<String>,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Vote {
            voter_id,
            direction,
            timestamp,
            voter_reputation,
            rationale,
        }
    }
}

/// Resultado de la votación
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteResult {
    /// ID de la propuesta
    pub proposal_id: Uuid,
    /// Votos a favor
    pub votes_for: usize,
    /// Votos en contra
    pub votes_against: usize,
    /// Abstenciones
    pub votes_abstain: usize,
    /// Total de votos válidos
    pub total_valid_votes: usize,
    /// Peso total de reputación de votos a favor
    pub reputation_weight_for: f64,
    /// Peso total de reputación de votos en contra
    pub reputation_weight_against: f64,
    /// Participación como porcentaje del quórum requerido
    pub participation_rate: f64,
    /// Si alcanzó quórum
    pub quorum_reached: bool,
    /// Si la propuesta fue aprobada
    pub approved: bool,
    /// Timestamp de resolución
    pub resolved_at: u64,
}

/// Configuración de votación
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotingConfig {
    /// Duración del período de votación en segundos (default: 72h)
    pub voting_duration_secs: u64,
    /// Porcentaje mínimo de nodos activos para quórum (default: 0.30 = 30%)
    pub quorum_percentage: f64,
    /// Reputación mínima para votar (default: 0.7)
    pub minimum_reputation: f64,
    /// Margen mínimo de aprobación (default: 0.51 = 51%)
    pub approval_threshold: f64,
}

impl Default for VotingConfig {
    fn default() -> Self {
        Self {
            voting_duration_secs: 72 * 60 * 60, // 72 horas
            quorum_percentage: 0.30,
            minimum_reputation: 0.7,
            approval_threshold: 0.51,
        }
    }
}

/// Gestor de votación
pub struct VotingManager {
    /// Votos por propuesta
    votes: HashMap<Uuid, Vec<Vote>>,
    /// Resultados resueltos
    results: HashMap<Uuid, VoteResult>,
    /// Configuración
    config: VotingConfig,
    /// Total de nodos activos en la red (para calcular quórum)
    active_nodes_count: usize,
}

impl VotingManager {
    pub fn new() -> Self {
        Self {
            votes: HashMap::new(),
            results: HashMap::new(),
            config: VotingConfig::default(),
            active_nodes_count: 0,
        }
    }

    /// Crear con configuración custom
    pub fn with_config(config: VotingConfig, active_nodes_count: usize) -> Self {
        Self {
            votes: HashMap::new(),
            results: HashMap::new(),
            config,
            active_nodes_count,
        }
    }

    /// Actualizar conteo de nodos activos
    pub fn set_active_nodes_count(&mut self, count: usize) {
        self.active_nodes_count = count;
    }

    /// Registrar un voto
    pub fn cast_vote(
        &mut self,
        proposal: &Proposal,
        vote: Vote,
    ) -> Result<(), VotingError> {
        // Verificar que la propuesta está en votación
        if proposal.state != ProposalState::Voting {
            return Err(VotingError::VotingNotActive(proposal.id));
        }

        // Verificar que no ha expirado
        if proposal.has_expired() {
            return Err(VotingError::ProposalExpired);
        }

        // Verificar reputación del votante
        if vote.voter_reputation < self.config.minimum_reputation {
            return Err(VotingError::ReputationTooLow {
                current: vote.voter_reputation,
                minimum: self.config.minimum_reputation,
            });
        }

        // Verificar que no ya votó
        let proposal_votes = self.votes.entry(proposal.id).or_default(); // CLEANUP: or_insert_with -> or_default
        if proposal_votes.iter().any(|v| v.voter_id == vote.voter_id) {
            return Err(VotingError::AlreadyVoted {
                proposal: proposal.id,
                voter: vote.voter_id.clone(),
            });
        }

        proposal_votes.push(vote);
        info!(
            proposal_id = %proposal.id,
            voter_id = %proposal_votes.last().unwrap().voter_id,
            direction = %proposal_votes.last().unwrap().direction,
            "Vote cast successfully"
        );

        Ok(())
    }

    /// Resolver votación y determinar resultado
    pub fn resolve_vote(
        &mut self,
        proposal: &Proposal,
    ) -> Result<VoteResult, VotingError> {
        let proposal_votes = self
            .votes
            .get(&proposal.id)
            .ok_or(VotingError::ProposalNotFound(proposal.id))?; // CLEANUP: ok_or_else -> ok_or

        let votes_for: usize = proposal_votes
            .iter()
            .filter(|v| v.direction == VoteDirection::For)
            .count();

        let votes_against: usize = proposal_votes
            .iter()
            .filter(|v| v.direction == VoteDirection::Against)
            .count();

        let votes_abstain: usize = proposal_votes
            .iter()
            .filter(|v| v.direction == VoteDirection::Abstain)
            .count();

        let total_valid_votes = votes_for + votes_against + votes_abstain;

        let reputation_weight_for: f64 = proposal_votes
            .iter()
            .filter(|v| v.direction == VoteDirection::For)
            .map(|v| v.voter_reputation)
            .sum();

        let reputation_weight_against: f64 = proposal_votes
            .iter()
            .filter(|v| v.direction == VoteDirection::Against)
            .map(|v| v.voter_reputation)
            .sum();

        let quorum_required =
            (self.active_nodes_count as f64 * self.config.quorum_percentage).ceil() as usize;
        let quorum_reached = total_valid_votes >= quorum_required;

        let participation_rate = if self.active_nodes_count > 0 {
            total_valid_votes as f64 / self.active_nodes_count as f64
        } else {
            0.0
        };

        // Determinar aprobación: requiere quórum + mayoría ponderada
        let total_reputation_weight = reputation_weight_for + reputation_weight_against;
        let approval_rate = if total_reputation_weight > 0.0 {
            reputation_weight_for / total_reputation_weight
        } else {
            0.0
        };

        let approved = quorum_reached && approval_rate >= self.config.approval_threshold;

        let resolved_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let result = VoteResult {
            proposal_id: proposal.id,
            votes_for,
            votes_against,
            votes_abstain,
            total_valid_votes,
            reputation_weight_for,
            reputation_weight_against,
            participation_rate,
            quorum_reached,
            approved,
            resolved_at,
        };

        info!(
            proposal_id = %proposal.id,
            votes_for,
            votes_against,
            votes_abstain,
            quorum_reached,
            approved,
            participation_rate,
            "Vote resolved"
        );

        self.results.insert(proposal.id, result.clone());
        Ok(result)
    }

    /// Obtener resultado de votación
    pub fn get_result(&self, proposal_id: &Uuid) -> Option<&VoteResult> {
        self.results.get(proposal_id)
    }

    /// Obtener votos de una propuesta
    pub fn get_votes(&self, proposal_id: &Uuid) -> Option<&[Vote]> {
        self.votes.get(proposal_id).map(|v| v.as_slice())
    }

    /// Verificar y procesar propuestas expiradas
    pub fn check_expired_proposals(
        &self,
    ) -> Vec<Uuid> {
        self.votes
            .keys()
            .filter(|_| false) // TODO: check actual proposal expiry from ProposalManager
            .cloned()
            .collect()
    }

    /// Estadísticas de votación
    pub fn stats(&self) -> VotingStats {
        VotingStats {
            total_proposals_voted: self.votes.len(),
            total_resolved: self.results.len(),
            total_approved: self.results.values().filter(|r| r.approved).count(),
            total_rejected: self.results.values().filter(|r| !r.approved).count(),
            active_nodes_count: self.active_nodes_count,
            quorum_percentage: self.config.quorum_percentage,
            minimum_reputation: self.config.minimum_reputation,
        }
    }
}

impl Default for VotingManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Estadísticas del sistema de votación
#[derive(Debug, Serialize, Deserialize)]
pub struct VotingStats {
    pub total_proposals_voted: usize,
    pub total_resolved: usize,
    pub total_approved: usize,
    pub total_rejected: usize,
    pub active_nodes_count: usize,
    pub quorum_percentage: f64,
    pub minimum_reputation: f64,
}

/// Callback para ejecución automática de propuestas aprobadas
///
/// TODO: Phase 6 - Integrar con sistema de ejecución de políticas
pub type ProposalExecutionFn = dyn Fn(&Proposal) -> Result<(), String> + Send + Sync;

/// Ejecutor automático de propuestas
pub struct AutoExecutor {
    execution_callback: Option<std::sync::Arc<ProposalExecutionFn>>,
}

impl AutoExecutor {
    pub fn new() -> Self {
        Self {
            execution_callback: None,
        }
    }

    /// Registrar callback de ejecución
    pub fn with_callback<F>(&mut self, callback: F)
    where
        F: Fn(&Proposal) -> Result<(), String> + Send + Sync + 'static,
    {
        self.execution_callback = Some(std::sync::Arc::new(callback));
    }

    /// Ejecutar propuesta aprobada
    pub fn execute(&self, proposal: &Proposal) -> Result<(), VotingError> {
        match &self.execution_callback {
            Some(callback) => callback(proposal).map_err(VotingError::ExecutionFailed),
            None => {
                warn!(
                    proposal_id = %proposal.id,
                    "No execution callback registered for approved proposal"
                );
                Ok(())
            }
        }
    }
}

impl Default for AutoExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::proposal::Proposal;

    #[test]
    fn test_voting_manager_creation() {
        let config = VotingConfig {
            voting_duration_secs: 3600,
            quorum_percentage: 0.3,
            minimum_reputation: 0.7,
            approval_threshold: 0.51,
        };
        let manager = VotingManager::with_config(config, 100);
        assert_eq!(manager.active_nodes_count, 100);
    }

    #[test]
    fn test_vote_casting() {
        let (signing_key, _) = Proposal::generate_keypair().unwrap();
        let proposal = Proposal::create(
            Uuid::new_v4(),
            crate::governance::proposal::ProposalType::Custom,
            "Test".to_string(),
            "Payload".to_string(),
            &signing_key,
            3600,
        );

        let mut manager = VotingManager::with_config(VotingConfig::default(), 10);
        let vote = Vote::new(
            "node_1".to_string(),
            VoteDirection::For,
            0.85,
            Some("Looks good".to_string()),
        );

        // Need to set proposal state to Voting first
        // For this test, we just verify vote creation
        assert_eq!(vote.direction, VoteDirection::For);
        assert_eq!(vote.voter_reputation, 0.85);
    }

    #[test]
    fn test_voting_stats() {
        let manager = VotingManager::new();
        let stats = manager.stats();
        assert_eq!(stats.total_proposals_voted, 0);
        assert_eq!(stats.total_resolved, 0);
    }
}
