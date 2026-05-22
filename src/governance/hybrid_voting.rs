//! Hybrid Voting Engine — Motor de votación híbrida on-chain/off-chain
//!
//! Combina votación on-chain (con stake) y off-chain (quórum flexible)
//! con validación de umbrales combinados y detección de manipulación.
//!
//! Feature-gated: `#[cfg(feature = "v1.2-sprint2")]`

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Error de votación híbrida
#[derive(Debug, Error)]
pub enum HybridVoteError {
    #[error("Voter not registered: {0}")]
    VoterNotRegistered(String),
    #[error("Duplicate vote: {0}")]
    DuplicateVote(String),
    #[error("On-chain ratio below threshold: {current}<{required}")]
    OnChainRatioLow { current: f64, required: f64 },
    #[error("Manipulation detected: {0}")]
    ManipulationDetected(String),
    #[error("Voting period expired")]
    VotingExpired,
    #[error("Insufficient participation: {current}<{required}")]
    InsufficientParticipation { current: usize, required: usize },
}

/// Canal de votación
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VotingChannel {
    /// On-chain (requiere stake verificado)
    OnChain,
    /// Off-chain (voto sin stake, quórum flexible)
    OffChain,
}

impl fmt::Display for VotingChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VotingChannel::OnChain => write!(f, "OnChain"),
            VotingChannel::OffChain => write!(f, "OffChain"),
        }
    }
}

/// Voto híbrido
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridVote {
    /// ID del votante
    pub voter_id: String,
    /// Canal de votación
    pub channel: VotingChannel,
    /// Voto a favor
    pub for_proposal: bool,
    /// Peso del voto
    pub weight: f64,
    /// Stake asociado (solo on-chain)
    pub stake: u64,
    /// Timestamp
    pub timestamp: u64,
}

/// Configuración de votación híbrida
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridVotingConfig {
    /// Ratio mínimo de votos on-chain
    pub min_onchain_ratio: f64,
    /// Participación mínima (número de votantes)
    pub min_participants: usize,
    /// Umbral de detección de manipulación (votos similares en corto tiempo)
    pub manipulation_window_secs: u64,
    /// Máximo de votos en ventana de manipulación
    pub manipulation_max_votes: usize,
    /// Peso máximo por votante off-chain
    pub max_offchain_weight: f64,
}

impl Default for HybridVotingConfig {
    fn default() -> Self {
        HybridVotingConfig {
            min_onchain_ratio: 0.30,
            min_participants: 4,
            manipulation_window_secs: 60,
            manipulation_max_votes: 50,
            max_offchain_weight: 0.1,
        }
    }
}

/// Resultado de votación híbrida
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridVoteResult {
    /// Votos a favor
    pub for_weight: f64,
    /// Votos en contra
    pub against_weight: f64,
    /// Peso on-chain
    pub onchain_weight: f64,
    /// Peso off-chain
    pub offchain_weight: f64,
    /// Ratio on-chain
    pub onchain_ratio: f64,
    /// Total de participantes
    pub participant_count: usize,
    /// Manipulación detectada
    pub manipulation_detected: bool,
    /// Válido
    pub valid: bool,
}

/// Motor de votación híbrida
pub struct HybridVotingEngine {
    config: HybridVotingConfig,
    votes: Vec<HybridVote>,
    voter_registry: HashMap<String, u64>, // voter_id -> stake
    active: bool,
    start_time: u64,
}

impl HybridVotingEngine {
    pub fn new() -> Self {
        Self::with_config(HybridVotingConfig::default())
    }

    pub fn with_config(config: HybridVotingConfig) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        HybridVotingEngine {
            config,
            votes: Vec::new(),
            voter_registry: HashMap::new(),
            active: true,
            start_time: timestamp,
        }
    }

    /// Registra un votante con stake
    pub fn register_voter(&mut self, voter_id: String, stake: u64) {
        self.voter_registry.insert(voter_id, stake);
    }

    /// Registra un voto
    pub fn cast_vote(
        &mut self,
        voter_id: String,
        channel: VotingChannel,
        for_proposal: bool,
    ) -> Result<HybridVote, HybridVoteError> {
        if !self.active {
            return Err(HybridVoteError::VotingExpired);
        }

        // Verificar votante
        let stake = *self
            .voter_registry
            .get(&voter_id)
            .ok_or(HybridVoteError::VoterNotRegistered(voter_id.clone()))?;

        // Verificar voto duplicado
        if self.votes.iter().any(|v| v.voter_id == voter_id) {
            return Err(HybridVoteError::DuplicateVote(voter_id.clone()));
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Calcular peso
        let weight = match channel {
            VotingChannel::OnChain => (stake as f64).min(1.0),
            VotingChannel::OffChain => self.config.max_offchain_weight,
        };

        let vote = HybridVote {
            voter_id: voter_id.clone(),
            channel,
            for_proposal,
            weight,
            stake,
            timestamp,
        };

        // Verificar manipulación
        self.check_manipulation(&vote)?;

        self.votes.push(vote.clone());
        Ok(vote)
    }

    /// Verifica manipulación (votos masivos en corto tiempo)
    fn check_manipulation(&self, new_vote: &HybridVote) -> Result<(), HybridVoteError> {
        let window = self.config.manipulation_window_secs;
        let max_votes = self.config.manipulation_max_votes;

        let recent_votes: usize = self
            .votes
            .iter()
            .filter(|v| v.timestamp.abs_diff(new_vote.timestamp) <= window)
            .count();

        if recent_votes >= max_votes {
            return Err(HybridVoteError::ManipulationDetected(format!(
                "{} votes in {}s window (max {})",
                recent_votes + 1,
                window,
                max_votes
            )));
        }

        Ok(())
    }

    /// Calcula resultado de votación
    pub fn calculate_result(&self) -> HybridVoteResult {
        let for_weight: f64 = self
            .votes
            .iter()
            .filter(|v| v.for_proposal)
            .map(|v| v.weight)
            .sum();

        let against_weight: f64 = self
            .votes
            .iter()
            .filter(|v| !v.for_proposal)
            .map(|v| v.weight)
            .sum();

        let onchain_weight: f64 = self
            .votes
            .iter()
            .filter(|v| v.channel == VotingChannel::OnChain)
            .map(|v| v.weight)
            .sum();

        let offchain_weight: f64 = self
            .votes
            .iter()
            .filter(|v| v.channel == VotingChannel::OffChain)
            .map(|v| v.weight)
            .sum();

        let total_weight = onchain_weight + offchain_weight;
        let onchain_ratio = if total_weight > 0.0 {
            onchain_weight / total_weight
        } else {
            0.0
        };

        let participant_count = self.votes.len();

        // Verificar manipulación
        let manipulation_detected = self.detect_manipulation_pattern();

        // Verificar validez
        let valid = participant_count >= self.config.min_participants
            && onchain_ratio >= self.config.min_onchain_ratio
            && !manipulation_detected;

        HybridVoteResult {
            for_weight,
            against_weight,
            onchain_weight,
            offchain_weight,
            onchain_ratio,
            participant_count,
            manipulation_detected,
            valid,
        }
    }

    /// Detecta patrones de manipulación
    fn detect_manipulation_pattern(&self) -> bool {
        if self.votes.is_empty() {
            return false;
        }

        let window = self.config.manipulation_window_secs;
        let max_votes = self.config.manipulation_max_votes;

        // Verificar cada ventana de tiempo
        for (i, vote) in self.votes.iter().enumerate() {
            let window_votes: usize = self.votes[i..]
                .iter()
                .take_while(|v| v.timestamp - vote.timestamp <= window)
                .count();

            if window_votes >= max_votes {
                return true;
            }
        }

        false
    }

    /// Finaliza votación
    pub fn finalize(&mut self) {
        self.active = false;
    }

    /// Verifica si la votación está activa
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Número de votos
    pub fn vote_count(&self) -> usize {
        self.votes.len()
    }

    /// Obtiene votos
    pub fn get_votes(&self) -> &[HybridVote] {
        &self.votes
    }

    /// Config
    pub fn config(&self) -> &HybridVotingConfig {
        &self.config
    }
}

impl Default for HybridVotingEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = HybridVotingEngine::new();
        assert!(engine.is_active());
        assert_eq!(engine.vote_count(), 0);
    }

    #[test]
    fn test_register_voter() {
        let mut engine = HybridVotingEngine::new();
        engine.register_voter("v1".to_string(), 10000);
        assert_eq!(engine.voter_registry.get("v1"), Some(&10000));
    }

    #[test]
    fn test_cast_onchain_vote() {
        let mut engine = HybridVotingEngine::new();
        engine.register_voter("v1".to_string(), 10000);
        let vote = engine.cast_vote("v1".to_string(), VotingChannel::OnChain, true);
        assert!(vote.is_ok());
        assert_eq!(engine.vote_count(), 1);
    }

    #[test]
    fn test_cast_offchain_vote() {
        let mut engine = HybridVotingEngine::new();
        engine.register_voter("v1".to_string(), 0);
        let vote = engine.cast_vote("v1".to_string(), VotingChannel::OffChain, true);
        assert!(vote.is_ok());
    }

    #[test]
    fn test_duplicate_vote() {
        let mut engine = HybridVotingEngine::new();
        engine.register_voter("v1".to_string(), 10000);
        engine
            .cast_vote("v1".to_string(), VotingChannel::OnChain, true)
            .unwrap();
        assert!(engine
            .cast_vote("v1".to_string(), VotingChannel::OnChain, false)
            .is_err());
    }

    #[test]
    fn test_unregistered_voter() {
        let mut engine = HybridVotingEngine::new();
        assert!(engine
            .cast_vote("unknown".to_string(), VotingChannel::OnChain, true)
            .is_err());
    }

    #[test]
    fn test_voting_expired() {
        let mut engine = HybridVotingEngine::new();
        engine.finalize();
        engine.register_voter("v1".to_string(), 10000);
        assert!(engine
            .cast_vote("v1".to_string(), VotingChannel::OnChain, true)
            .is_err());
    }

    #[test]
    fn test_result_valid_with_onchain() {
        let mut engine = HybridVotingEngine::new();
        for i in 1..=5 {
            engine.register_voter(format!("v{}", i), 10000);
        }
        for i in 1..=3 {
            engine
                .cast_vote(format!("v{}", i), VotingChannel::OnChain, true)
                .unwrap();
        }
        for i in 4..=5 {
            engine
                .cast_vote(format!("v{}", i), VotingChannel::OffChain, true)
                .unwrap();
        }
        let result = engine.calculate_result();
        assert!(result.valid);
        assert!(result.onchain_ratio >= engine.config.min_onchain_ratio);
    }

    #[test]
    fn test_result_invalid_low_onchain() {
        let mut engine = HybridVotingEngine::new();
        for i in 1..=5 {
            engine.register_voter(format!("v{}", i), 0);
        }
        for i in 1..=5 {
            engine
                .cast_vote(format!("v{}", i), VotingChannel::OffChain, true)
                .unwrap();
        }
        let result = engine.calculate_result();
        assert!(!result.valid);
        assert!(result.onchain_ratio < engine.config.min_onchain_ratio);
    }

    #[test]
    fn test_result_insufficient_participants() {
        let mut engine = HybridVotingEngine::new();
        engine.register_voter("v1".to_string(), 10000);
        engine
            .cast_vote("v1".to_string(), VotingChannel::OnChain, true)
            .unwrap();
        let result = engine.calculate_result();
        assert!(!result.valid);
    }

    #[test]
    fn test_weight_calculation() {
        let mut engine = HybridVotingEngine::new();
        engine.register_voter("v1".to_string(), 10000);
        engine.register_voter("v2".to_string(), 0);
        engine
            .cast_vote("v1".to_string(), VotingChannel::OnChain, true)
            .unwrap();
        engine
            .cast_vote("v2".to_string(), VotingChannel::OffChain, false)
            .unwrap();
        let result = engine.calculate_result();
        assert!(result.for_weight > 0.0);
        assert!(result.against_weight > 0.0);
    }

    #[test]
    fn test_config_default() {
        let config = HybridVotingConfig::default();
        assert_eq!(config.min_onchain_ratio, 0.30);
        assert_eq!(config.min_participants, 4);
    }

    #[test]
    fn test_engine_default() {
        let engine = HybridVotingEngine::default();
        assert!(engine.is_active());
    }

    #[test]
    fn test_channel_display() {
        assert_eq!(format!("{}", VotingChannel::OnChain), "OnChain");
        assert_eq!(format!("{}", VotingChannel::OffChain), "OffChain");
    }

    #[test]
    fn test_get_votes() {
        let mut engine = HybridVotingEngine::new();
        engine.register_voter("v1".to_string(), 10000);
        engine
            .cast_vote("v1".to_string(), VotingChannel::OnChain, true)
            .unwrap();
        assert_eq!(engine.get_votes().len(), 1);
    }
}
