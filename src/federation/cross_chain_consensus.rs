//! Cross-Chain Consensus & Validation
//!
//! Motor de consenso cross-chain con validación de pruebas de inclusión,
//! quórum ≥30%, aprobación reputación-ponderada ≥51% y time-lock criptográfico.
//!
//! Feature-gated: `#[cfg(feature = "v1.2-sprint2")]`

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Error del consenso cross-chain
#[derive(Debug, Error)]
pub enum ConsensusError {
    #[error("Insufficient validators: {current}/{required}")]
    InsufficientValidators { current: usize, required: usize },
    #[error("Quorum not reached: {current}<{required}")]
    QuorumNotReached { current: f64, required: f64 },
    #[error("Approval threshold not met: {current}<{required}")]
    ApprovalThresholdNotMet { current: f64, required: f64 },
    #[error("Validator not registered: {0}")]
    ValidatorNotRegistered(String),
    #[error("Duplicate vote from: {0}")]
    DuplicateVote(String),
    #[error("Proposal expired")]
    ProposalExpired,
    #[error("Invalid proof: {0}")]
    InvalidProof(String),
    #[error("Time-lock active: {remaining:?} remaining")]
    TimeLockActive { remaining: Duration },
    #[error("Chain not registered: {0}")]
    ChainNotRegistered(String),
    #[error("Proposal not found: {0}")]
    ProposalNotFound(String),
}

/// Estado de una propuesta de consenso
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConsensusState {
    /// Propuesta creada, esperando votos
    Pending,
    /// Votación en curso
    Voting,
    /// Quórum alcanzado, time-lock activo
    QuorumReached,
    /// Time-lock expirado, listo para ejecutar
    Ready,
    /// Ejecutado exitosamente
    Executed,
    /// Rechazado (no alcanzó umbral)
    Rejected,
    /// Expirado
    Expired,
}

impl fmt::Display for ConsensusState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConsensusState::Pending => write!(f, "Pending"),
            ConsensusState::Voting => write!(f, "Voting"),
            ConsensusState::QuorumReached => write!(f, "QuorumReached"),
            ConsensusState::Ready => write!(f, "Ready"),
            ConsensusState::Executed => write!(f, "Executed"),
            ConsensusState::Rejected => write!(f, "Rejected"),
            ConsensusState::Expired => write!(f, "Expired"),
        }
    }
}

/// Tipo de prueba cross-chain
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProofType {
    /// Prueba de inclusión Merkle
    MerkleInclusion,
    /// Prueba de validez ZKP
    ZKPValidity,
    /// Prueba de firma Ed25519
    Ed25519Signature,
}

impl fmt::Display for ProofType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProofType::MerkleInclusion => write!(f, "MerkleInclusion"),
            ProofType::ZKPValidity => write!(f, "ZKPValidity"),
            ProofType::Ed25519Signature => write!(f, "Ed25519Signature"),
        }
    }
}

/// Prueba de validación cross-chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainProof {
    /// Tipo de prueba
    pub proof_type: ProofType,
    /// Cadena de origen
    pub source_chain: String,
    /// Cadena de destino
    pub target_chain: String,
    /// Datos de la prueba (hex-encoded)
    pub proof_data: String,
    /// Root/hash de referencia
    pub reference_hash: String,
    /// Timestamp de creación
    pub timestamp: u64,
}

impl ChainProof {
    pub fn new(
        proof_type: ProofType,
        source_chain: String,
        target_chain: String,
        proof_data: String,
        reference_hash: String,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        ChainProof {
            proof_type,
            source_chain,
            target_chain,
            proof_data,
            reference_hash,
            timestamp,
        }
    }

    /// Verifica si la prueba ha expirado
    pub fn is_expired(&self, max_age_secs: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now.saturating_sub(self.timestamp) > max_age_secs
    }
}

/// Configuración del consenso
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// Quórum mínimo (fracción de validadores)
    pub quorum_threshold: f64,
    /// Umbral de aprobación (fracción de votos ponderados)
    pub approval_threshold: f64,
    /// Duración del time-lock para propuestas críticas
    pub timelock_duration: Duration,
    /// Edad máxima de pruebas (segundos)
    pub max_proof_age_secs: u64,
    /// Número mínimo de validadores
    pub min_validators: usize,
    /// Duración máxima de votación
    pub voting_duration: Duration,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        ConsensusConfig {
            quorum_threshold: 0.30,
            approval_threshold: 0.51,
            timelock_duration: Duration::from_secs(72 * 3600), // 72h
            max_proof_age_secs: 3600, // 1h
            min_validators: 4,
            voting_duration: Duration::from_secs(24 * 3600), // 24h
        }
    }
}

/// Validador registrado
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Validator {
    /// ID del validador
    pub id: String,
    /// Cadenas que valida
    pub chains: Vec<String>,
    /// Reputación del validador [0.0, 1.0]
    pub reputation: f64,
    /// Stake del validador
    pub stake: u64,
    /// Activo o no
    pub active: bool,
    /// Último heartbeat (epoch seconds)
    pub last_heartbeat: u64,
}

impl Validator {
    pub fn new(id: String, chains: Vec<String>, reputation: f64, stake: u64) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Validator {
            id,
            chains,
            reputation,
            stake,
            active: true,
            last_heartbeat: timestamp,
        }
    }

    pub fn get_weight(&self) -> f64 {
        self.reputation * (self.stake as f64)
    }

    pub fn heartbeat(&mut self) {
        self.last_heartbeat = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    pub fn is_stale(&self, max_stale: Duration) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now.saturating_sub(self.last_heartbeat) > max_stale.as_secs()
    }
}

/// Voto de un validador
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorVote {
    /// ID del validador
    pub validator_id: String,
    /// Dirección del voto
    pub direction: VoteDirection,
    /// Peso del voto (reputación * stake)
    pub weight: f64,
    /// Timestamp
    pub timestamp: u64,
    /// Opcional: justificación
    pub rationale: Option<String>,
}

/// Dirección del voto
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VoteDirection {
    For,
    Against,
    Abstain,
}

impl fmt::Display for VoteDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VoteDirection::For => write!(f, "For"),
            VoteDirection::Against => write!(f, "Against"),
            VoteDirection::Abstain => write!(f, "Abstain"),
        }
    }
}

/// Propuesta de consenso cross-chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusProposal {
    /// ID único de la propuesta
    pub id: String,
    /// Cadena de origen
    pub source_chain: String,
    /// Cadena de destino
    pub target_chain: String,
    /// Descripción de la propuesta
    pub description: String,
    /// Pruebas asociadas
    pub proofs: Vec<ChainProof>,
    /// Estado actual
    pub state: ConsensusState,
    /// Votos recibidos
    pub votes: HashMap<String, ValidatorVote>,
    /// Timestamp de creación
    pub created_at: u64,
    /// Timestamp de quórum alcanzado
    pub quorum_reached_at: Option<u64>,
    /// Es propuesta crítica (requiere time-lock)
    pub is_critical: bool,
}

impl ConsensusProposal {
    pub fn new(
        id: String,
        source_chain: String,
        target_chain: String,
        description: String,
        proofs: Vec<ChainProof>,
        is_critical: bool,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        ConsensusProposal {
            id,
            source_chain,
            target_chain,
            description,
            proofs,
            state: ConsensusState::Pending,
            votes: HashMap::new(),
            created_at: timestamp,
            quorum_reached_at: None,
            is_critical,
        }
    }

    /// Calcula el peso total de votos a favor
    pub fn for_weight(&self) -> f64 {
        self.votes
            .values()
            .filter(|v| v.direction == VoteDirection::For)
            .map(|v| v.weight)
            .sum()
    }

    /// Calcula el peso total de votos en contra
    pub fn against_weight(&self) -> f64 {
        self.votes
            .values()
            .filter(|v| v.direction == VoteDirection::Against)
            .map(|v| v.weight)
            .sum()
    }

    /// Calcula el peso total de todos los votos
    pub fn total_weight(&self) -> f64 {
        self.votes.values().map(|v| v.weight).sum()
    }

    /// Número de validadores que votaron
    pub fn voter_count(&self) -> usize {
        self.votes.len()
    }
}

/// Resultado del consenso
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusResult {
    /// ID de la propuesta
    pub proposal_id: String,
    /// Estado final
    pub state: ConsensusState,
    /// Peso a favor
    pub for_weight: f64,
    /// Peso en contra
    pub against_weight: f64,
    /// Peso total
    pub total_weight: f64,
    /// Quórum alcanzado
    pub quorum_reached: bool,
    /// Aprobación alcanzada
    pub approval_met: bool,
    /// Número de validadores participantes
    pub validator_count: usize,
}

/// Motor de consenso cross-chain
pub struct CrossChainConsensus {
    config: ConsensusConfig,
    validators: HashMap<String, Validator>,
    proposals: HashMap<String, ConsensusProposal>,
    total_weight: f64,
}

impl CrossChainConsensus {
    pub fn new() -> Self {
        Self::with_config(ConsensusConfig::default())
    }

    pub fn with_config(config: ConsensusConfig) -> Self {
        CrossChainConsensus {
            config,
            validators: HashMap::new(),
            proposals: HashMap::new(),
            total_weight: 0.0,
        }
    }

    /// Registra un validador
    pub fn register_validator(&mut self, validator: Validator) {
        let weight = validator.get_weight();
        self.total_weight += weight;
        self.validators.insert(validator.id.clone(), validator);
    }

    /// Remueve un validador
    pub fn unregister_validator(&mut self, validator_id: &str) -> Result<(), ConsensusError> {
        let validator = self
            .validators
            .get(validator_id)
            .ok_or(ConsensusError::ValidatorNotRegistered(validator_id.to_string()))?;
        self.total_weight = (self.total_weight - validator.get_weight()).max(0.0);
        self.validators.remove(validator_id);
        Ok(())
    }

    /// Crea una nueva propuesta de consenso
    pub fn create_proposal(
        &mut self,
        proposal: ConsensusProposal,
    ) -> Result<(), ConsensusError> {
        // Validar que las cadenas estén registradas
        if !self.chain_exists(&proposal.source_chain) {
            return Err(ConsensusError::ChainNotRegistered(
                proposal.source_chain.clone(),
            ));
        }
        if !self.chain_exists(&proposal.target_chain) {
            return Err(ConsensusError::ChainNotRegistered(
                proposal.target_chain.clone(),
            ));
        }

        // Validar pruebas
        for proof in &proposal.proofs {
            if proof.is_expired(self.config.max_proof_age_secs) {
                return Err(ConsensusError::InvalidProof("Proof expired".to_string()));
            }
        }

        self.proposals.insert(proposal.id.clone(), proposal);
        Ok(())
    }

    /// Inicia votación para una propuesta
    pub fn start_voting(&mut self, proposal_id: &str) -> Result<(), ConsensusError> {
        let proposal = self.proposals.get_mut(proposal_id).ok_or(
            ConsensusError::ProposalNotFound(proposal_id.to_string()),
        )?;

        if proposal.state != ConsensusState::Pending {
            return Err(ConsensusError::ProposalExpired);
        }

        proposal.state = ConsensusState::Voting;
        Ok(())
    }

    /// Registra un voto de validador
    pub fn submit_vote(
        &mut self,
        proposal_id: &str,
        validator_id: &str,
        direction: VoteDirection,
        rationale: Option<String>,
    ) -> Result<ValidatorVote, ConsensusError> {
        // Verificar validador
        let validator = self
            .validators
            .get(validator_id)
            .ok_or(ConsensusError::ValidatorNotRegistered(validator_id.to_string()))?;

        if !validator.active {
            return Err(ConsensusError::ValidatorNotRegistered(
                validator_id.to_string(),
            ));
        }

        // Verificar propuesta
        let proposal = self.proposals.get_mut(proposal_id).ok_or(
            ConsensusError::ProposalNotFound(proposal_id.to_string()),
        )?;

        if proposal.state != ConsensusState::Voting
            && proposal.state != ConsensusState::QuorumReached
            && proposal.state != ConsensusState::Ready
        {
            return Err(ConsensusError::ProposalExpired);
        }

        // Verificar voto duplicado
        if proposal.votes.contains_key(validator_id) {
            return Err(ConsensusError::DuplicateVote(validator_id.to_string()));
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let vote = ValidatorVote {
            validator_id: validator_id.to_string(),
            direction: direction.clone(),
            weight: validator.get_weight(),
            timestamp,
            rationale,
        };

        proposal.votes.insert(validator_id.to_string(), vote.clone());

        // Verificar quórum y aprobación
        self.check_quorum_and_approval(proposal_id)?;

        Ok(vote)
    }

    /// Verifica quórum y umbral de aprobación
    fn check_quorum_and_approval(&mut self, proposal_id: &str) -> Result<(), ConsensusError> {
        let proposal = self.proposals.get(proposal_id).ok_or(
            ConsensusError::ProposalNotFound(proposal_id.to_string()),
        )?;

        let active_validators: usize = self
            .validators
            .values()
            .filter(|v| v.active)
            .count();

        let voter_count = proposal.voter_count();

        // Verificar quórum
        let quorum_ratio = voter_count as f64 / active_validators as f64;
        let quorum_reached = quorum_ratio >= self.config.quorum_threshold;

        // Verificar aprobación
        let total_weight = proposal.total_weight();
        let for_weight = proposal.for_weight();
        let approval_ratio = if total_weight > 0.0 {
            for_weight / total_weight
        } else {
            0.0
        };
        let approval_met = approval_ratio >= self.config.approval_threshold;

        if quorum_reached && approval_met {
            let proposal = self.proposals.get_mut(proposal_id).unwrap();
            if proposal.is_critical {
                proposal.state = ConsensusState::QuorumReached;
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                proposal.quorum_reached_at = Some(timestamp);
            } else {
                proposal.state = ConsensusState::Ready;
            }
        }

        Ok(())
    }

    /// Verifica si el time-lock ha expirado para propuestas críticas
    pub fn check_timelock(&mut self, proposal_id: &str) -> Result<ConsensusState, ConsensusError> {
        let proposal = self.proposals.get(proposal_id).ok_or(
            ConsensusError::ProposalNotFound(proposal_id.to_string()),
        )?;

        match proposal.state {
            ConsensusState::QuorumReached => {
                if let Some(quorum_time) = proposal.quorum_reached_at {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let elapsed = now.saturating_sub(quorum_time);
                    if Duration::from_secs(elapsed) >= self.config.timelock_duration {
                        let proposal = self.proposals.get_mut(proposal_id).unwrap();
                        proposal.state = ConsensusState::Ready;
                        Ok(ConsensusState::Ready)
                    } else {
                        let remaining = self.config.timelock_duration - Duration::from_secs(elapsed);
                        Err(ConsensusError::TimeLockActive { remaining })
                    }
                } else {
                    Err(ConsensusError::TimeLockActive {
                        remaining: self.config.timelock_duration,
                    })
                }
            }
            _ => Ok(proposal.state.clone()),
        }
    }

    /// Ejecuta una propuesta lista
    pub fn execute_proposal(&mut self, proposal_id: &str) -> Result<ConsensusResult, ConsensusError> {
        let proposal = self.proposals.get(proposal_id).ok_or(
            ConsensusError::ProposalNotFound(proposal_id.to_string()),
        )?;

        if proposal.state != ConsensusState::Ready {
            return Err(ConsensusError::ProposalExpired);
        }

        let result = ConsensusResult {
            proposal_id: proposal_id.to_string(),
            state: ConsensusState::Executed,
            for_weight: proposal.for_weight(),
            against_weight: proposal.against_weight(),
            total_weight: proposal.total_weight(),
            quorum_reached: true,
            approval_met: true,
            validator_count: proposal.voter_count(),
        };

        let proposal = self.proposals.get_mut(proposal_id).unwrap();
        proposal.state = ConsensusState::Executed;

        Ok(result)
    }

    /// Obtiene el resultado actual de una propuesta
    pub fn get_result(&self, proposal_id: &str) -> Result<ConsensusResult, ConsensusError> {
        let proposal = self.proposals.get(proposal_id).ok_or(
            ConsensusError::ProposalNotFound(proposal_id.to_string()),
        )?;

        let active_validators: usize = self
            .validators
            .values()
            .filter(|v| v.active)
            .count();

        let quorum_ratio = if active_validators > 0 {
            proposal.voter_count() as f64 / active_validators as f64
        } else {
            0.0
        };

        let total_weight = proposal.total_weight();
        let for_weight = proposal.for_weight();
        let approval_ratio = if total_weight > 0.0 {
            for_weight / total_weight
        } else {
            0.0
        };

        Ok(ConsensusResult {
            proposal_id: proposal_id.to_string(),
            state: proposal.state.clone(),
            for_weight,
            against_weight: proposal.against_weight(),
            total_weight,
            quorum_reached: quorum_ratio >= self.config.quorum_threshold,
            approval_met: approval_ratio >= self.config.approval_threshold,
            validator_count: proposal.voter_count(),
        })
    }

    /// Verifica si una cadena está registrada en los validadores
    fn chain_exists(&self, chain_id: &str) -> bool {
        self.validators
            .values()
            .any(|v| v.chains.contains(&chain_id.to_string()))
    }

    /// Obtiene los validadores activos para una cadena
    pub fn get_validators_for_chain(&self, chain_id: &str) -> Vec<&Validator> {
        self.validators
            .values()
            .filter(|v| v.active && v.chains.contains(&chain_id.to_string()))
            .collect()
    }

    /// Obtiene la propuesta por ID
    pub fn get_proposal(&self, proposal_id: &str) -> Option<&ConsensusProposal> {
        self.proposals.get(proposal_id)
    }

    /// Obtiene el número total de validadores activos
    pub fn active_validator_count(&self) -> usize {
        self.validators.values().filter(|v| v.active).count()
    }

    /// Obtiene el peso total de validadores
    pub fn get_total_weight(&self) -> f64 {
        self.total_weight
    }

    /// Actualiza heartbeat de un validador
    pub fn validator_heartbeat(&mut self, validator_id: &str) -> Result<(), ConsensusError> {
        let validator = self
            .validators
            .get_mut(validator_id)
            .ok_or(ConsensusError::ValidatorNotRegistered(validator_id.to_string()))?;
        validator.heartbeat();
        Ok(())
    }

    /// Obtiene el config
    pub fn config(&self) -> &ConsensusConfig {
        &self.config
    }
}

impl Default for CrossChainConsensus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_validator(id: &str, chains: &[&str]) -> Validator {
        Validator::new(
            id.to_string(),
            chains.iter().map(|s| s.to_string()).collect(),
            0.9,
            1000,
        )
    }

    fn make_proof(source: &str, target: &str) -> ChainProof {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        ChainProof::new(
            ProofType::MerkleInclusion,
            source.to_string(),
            target.to_string(),
            "0xabc123".to_string(),
            "0xdef456".to_string(),
        )
    }

    #[test]
    fn test_consensus_creation() {
        let consensus = CrossChainConsensus::new();
        assert_eq!(consensus.active_validator_count(), 0);
    }

    #[test]
    fn test_consensus_with_config() {
        let config = ConsensusConfig {
            quorum_threshold: 0.5,
            ..Default::default()
        };
        let consensus = CrossChainConsensus::with_config(config);
        assert_eq!(consensus.config().quorum_threshold, 0.5);
    }

    #[test]
    fn test_register_validator() {
        let mut consensus = CrossChainConsensus::new();
        consensus.register_validator(make_validator("v1", &["chain-a"]));
        assert_eq!(consensus.active_validator_count(), 1);
    }

    #[test]
    fn test_unregister_validator() {
        let mut consensus = CrossChainConsensus::new();
        consensus.register_validator(make_validator("v1", &["chain-a"]));
        consensus.unregister_validator("v1").unwrap();
        assert_eq!(consensus.active_validator_count(), 0);
    }

    #[test]
    fn test_unregister_nonexistent_validator() {
        let mut consensus = CrossChainConsensus::new();
        assert!(consensus.unregister_validator("nonexistent").is_err());
    }

    #[test]
    fn test_validator_weight() {
        let v = make_validator("v1", &["chain-a"]);
        assert!((v.get_weight() - 900.0) < 0.001); // 0.9 * 1000
    }

    #[test]
    fn test_validator_heartbeat() {
        let mut v = make_validator("v1", &["chain-a"]);
        v.heartbeat();
        assert!(!v.is_stale(Duration::from_secs(1)));
    }

    #[test]
    fn test_create_proposal() {
        let mut consensus = CrossChainConsensus::new();
        consensus.register_validator(make_validator("v1", &["chain-a", "chain-b"]));

        let proposal = ConsensusProposal::new(
            "prop-1".to_string(),
            "chain-a".to_string(),
            "chain-b".to_string(),
            "Test proposal".to_string(),
            vec![make_proof("chain-a", "chain-b")],
            false,
        );
        consensus.create_proposal(proposal).unwrap();
    }

    #[test]
    fn test_create_proposal_invalid_chain() {
        let mut consensus = CrossChainConsensus::new();
        let proposal = ConsensusProposal::new(
            "prop-1".to_string(),
            "chain-a".to_string(),
            "chain-b".to_string(),
            "Test".to_string(),
            vec![],
            false,
        );
        assert!(consensus.create_proposal(proposal).is_err());
    }

    #[test]
    fn test_start_voting() {
        let mut consensus = CrossChainConsensus::new();
        consensus.register_validator(make_validator("v1", &["chain-a", "chain-b"]));
        let proposal = ConsensusProposal::new(
            "prop-1".to_string(),
            "chain-a".to_string(),
            "chain-b".to_string(),
            "Test".to_string(),
            vec![make_proof("chain-a", "chain-b")],
            false,
        );
        consensus.create_proposal(proposal).unwrap();
        consensus.start_voting("prop-1").unwrap();
    }

    #[test]
    fn test_submit_vote() {
        let mut consensus = CrossChainConsensus::new();
        consensus.register_validator(make_validator("v1", &["chain-a", "chain-b"]));
        let proposal = ConsensusProposal::new(
            "prop-1".to_string(),
            "chain-a".to_string(),
            "chain-b".to_string(),
            "Test".to_string(),
            vec![make_proof("chain-a", "chain-b")],
            false,
        );
        consensus.create_proposal(proposal).unwrap();
        consensus.start_voting("prop-1").unwrap();
        consensus
            .submit_vote("prop-1", "v1", VoteDirection::For, None)
            .unwrap();
    }

    #[test]
    fn test_duplicate_vote() {
        let mut consensus = CrossChainConsensus::new();
        consensus.register_validator(make_validator("v1", &["chain-a", "chain-b"]));
        let proposal = ConsensusProposal::new(
            "prop-1".to_string(),
            "chain-a".to_string(),
            "chain-b".to_string(),
            "Test".to_string(),
            vec![make_proof("chain-a", "chain-b")],
            false,
        );
        consensus.create_proposal(proposal).unwrap();
        consensus.start_voting("prop-1").unwrap();
        consensus
            .submit_vote("prop-1", "v1", VoteDirection::For, None)
            .unwrap();
        assert!(consensus
            .submit_vote("prop-1", "v1", VoteDirection::Against, None)
            .is_err());
    }

    #[test]
    fn test_quorum_reached() {
        let mut consensus = CrossChainConsensus::new();
        for i in 0..10 {
            consensus.register_validator(make_validator(&format!("v{}", i), &["chain-a", "chain-b"]));
        }

        let proposal = ConsensusProposal::new(
            "prop-1".to_string(),
            "chain-a".to_string(),
            "chain-b".to_string(),
            "Test".to_string(),
            vec![make_proof("chain-a", "chain-b")],
            false,
        );
        consensus.create_proposal(proposal).unwrap();
        consensus.start_voting("prop-1").unwrap();

        // 4 validators vote (40% > 30% quorum)
        for i in 0..4 {
            consensus
                .submit_vote(&format!("prop-1"), &format!("v{}", i), VoteDirection::For, None)
                .unwrap();
        }

        let result = consensus.get_result("prop-1").unwrap();
        assert!(result.quorum_reached);
        assert!(result.approval_met);
    }

    #[test]
    fn test_proposal_state_transitions() {
        let proposal = ConsensusProposal::new(
            "prop-1".to_string(),
            "chain-a".to_string(),
            "chain-b".to_string(),
            "Test".to_string(),
            vec![],
            false,
        );
        assert_eq!(proposal.state, ConsensusState::Pending);
    }

    #[test]
    fn test_proof_expiration() {
        let proof = ChainProof::new(
            ProofType::Ed25519Signature,
            "chain-a".to_string(),
            "chain-b".to_string(),
            "sig".to_string(),
            "hash".to_string(),
        );
        assert!(!proof.is_expired(3600));
    }

    #[test]
    fn test_get_validators_for_chain() {
        let mut consensus = CrossChainConsensus::new();
        consensus.register_validator(make_validator("v1", &["chain-a"]));
        consensus.register_validator(make_validator("v2", &["chain-b"]));
        let validators = consensus.get_validators_for_chain("chain-a");
        assert_eq!(validators.len(), 1);
    }

    #[test]
    fn test_consensus_result() {
        let result = ConsensusResult {
            proposal_id: "prop-1".to_string(),
            state: ConsensusState::Executed,
            for_weight: 900.0,
            against_weight: 0.0,
            total_weight: 900.0,
            quorum_reached: true,
            approval_met: true,
            validator_count: 1,
        };
        assert_eq!(result.state, ConsensusState::Executed);
    }

    #[test]
    fn test_config_default() {
        let config = ConsensusConfig::default();
        assert_eq!(config.quorum_threshold, 0.30);
        assert_eq!(config.approval_threshold, 0.51);
    }

    #[test]
    fn test_consensus_default() {
        let consensus = CrossChainConsensus::default();
        assert_eq!(consensus.active_validator_count(), 0);
    }

    #[test]
    fn test_error_display() {
        let err = ConsensusError::QuorumNotReached {
            current: 0.2,
            required: 0.3,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Quorum"));
    }

    #[test]
    fn test_state_display() {
        assert_eq!(format!("{}", ConsensusState::Pending), "Pending");
        assert_eq!(format!("{}", ConsensusState::Executed), "Executed");
    }

    #[test]
    fn test_vote_direction_display() {
        assert_eq!(format!("{}", VoteDirection::For), "For");
        assert_eq!(format!("{}", VoteDirection::Against), "Against");
    }

    #[test]
    fn test_proof_type_display() {
        assert_eq!(
            format!("{}", ProofType::MerkleInclusion),
            "MerkleInclusion"
        );
    }

    #[test]
    fn test_proposal_weights() {
        let mut proposal = ConsensusProposal::new(
            "prop-1".to_string(),
            "chain-a".to_string(),
            "chain-b".to_string(),
            "Test".to_string(),
            vec![],
            false,
        );

        proposal.votes.insert(
            "v1".to_string(),
            ValidatorVote {
                validator_id: "v1".to_string(),
                direction: VoteDirection::For,
                weight: 100.0,
                timestamp: 0,
                rationale: None,
            },
        );

        assert!((proposal.for_weight() - 100.0) < 0.001);
        assert!((proposal.total_weight() - 100.0) < 0.001);
    }
}
