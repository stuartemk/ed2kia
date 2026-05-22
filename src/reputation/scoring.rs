//! Reputation Scoring - Cálculo de créditos por cómputo verificado
//!
//! Sistema de reputación con decay exponencial, bonificación por detección
//! de anomalías, multiplicador ZKP y protección anti-Sybil.

use crate::reputation::ledger::{Contribution, ContributionType, ReputationLedger};
use chrono::Utc;
// CLEANUP: removed unused import Duration
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
// CLEANUP: removed unused import std::path::Path
use thiserror::Error;
use tracing::{info, warn};

/// Error del sistema de scoring
#[derive(Debug, Error)]
pub enum ScoringError {
    #[error("Node not found: {0}")]
    NodeNotFound(String),
    #[error("Invalid credit calculation: {0}")]
    InvalidCalculation(String),
    #[error("Anti-Sybil limit exceeded for IP/ASN: {identifier}")]
    AntiSybilLimitExceeded { identifier: String },
    #[error("Ledger error: {0}")]
    Ledger(#[from] crate::reputation::ledger::LedgerError),
}

/// Configuración de scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringConfig {
    /// Créditos base por forward pass SAE
    pub base_sae_forward_credits: f64,
    /// Créditos base por batch de consenso
    pub base_consensus_credits: f64,
    /// Créditos base por feedback humano
    pub base_feedback_credits: f64,
    /// Créditos base por concepto aprendido
    pub base_concept_credits: f64,
    /// Créditos base por propuesta de gobernanza
    pub base_governance_proposal_credits: f64,
    /// Créditos base por voto de gobernanza
    pub base_governance_vote_credits: f64,
    /// Créditos base por sync de modelo
    pub base_model_sync_credits: f64,
    /// Multiplicador por verificación ZKP
    pub zkp_multiplier: f64,
    /// Bonificación máxima por detección de anomalías (0.0 - 1.0)
    pub max_anomaly_bonus: f64,
    /// Período de decay en días (50% cada período)
    pub decay_period_days: u64,
    /// Límite de créditos por IP/ASN por período (anti-Sybil)
    pub antisybil_limit_per_period: f64,
    /// Período anti-Sybil en horas
    pub antisybil_period_hours: u64,
    /// Reputación mínima para participar en gobernanza
    pub governance_minimum_reputation: f64,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            base_sae_forward_credits: 10.0,
            base_consensus_credits: 15.0,
            base_feedback_credits: 5.0,
            base_concept_credits: 8.0,
            base_governance_proposal_credits: 20.0,
            base_governance_vote_credits: 2.0,
            base_model_sync_credits: 12.0,
            zkp_multiplier: 1.5,
            max_anomaly_bonus: 0.5,
            decay_period_days: 30,
            antisybil_limit_per_period: 1000.0,
            antisybil_period_hours: 24,
            governance_minimum_reputation: 0.7,
        }
    }
}

/// Registro de reputación por nodo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeReputation {
    /// ID del nodo
    pub node_id: String,
    /// Créditos totales acumulados (sin decay)
    pub total_earned_credits: f64,
    /// Créditos actuales (con decay aplicado)
    pub current_credits: f64,
    /// Score de reputación normalizado (0.0 - 1.0)
    pub reputation_score: f64,
    /// Timestamp del último cálculo de decay
    pub last_decay_timestamp: u64,
    /// Timestamp de la última contribución
    pub last_contribution_timestamp: u64,
    /// Total de contribuciones
    pub total_contributions: usize,
    /// Contribuciones verificadas con ZKP
    pub zkp_verified_contributions: usize,
    /// IP/ASN del nodo (para anti-Sybil)
    pub network_identifier: Option<String>,
}

impl NodeReputation {
    pub fn new(node_id: String) -> Self {
        let now = Utc::now().timestamp() as u64;
        Self {
            node_id,
            total_earned_credits: 0.0,
            current_credits: 0.0,
            reputation_score: 0.0,
            last_decay_timestamp: now,
            last_contribution_timestamp: now,
            total_contributions: 0,
            zkp_verified_contributions: 0,
            network_identifier: None,
        }
    }
}

/// Gestor de scoring de reputación
pub struct ReputationScorer {
    config: ScoringConfig,
    reputations: HashMap<String, NodeReputation>,
    /// Tracking anti-Sybil: IP/ASN -> (credits_earned, period_start)
    antisybil_tracker: HashMap<String, (f64, u64)>,
}

impl ReputationScorer {
    pub fn new() -> Self {
        Self {
            config: ScoringConfig::default(),
            reputations: HashMap::new(),
            antisybil_tracker: HashMap::new(),
        }
    }

    /// Crear con configuración custom
    pub fn with_config(config: ScoringConfig) -> Self {
        Self {
            config,
            reputations: HashMap::new(),
            antisybil_tracker: HashMap::new(),
        }
    }

    /// Cargar reputaciones desde ledger
    pub fn load_from_ledger(&mut self, ledger: &ReputationLedger) -> Result<(), ScoringError> {
        let contributions = ledger.get_all()?;

        for contribution in contributions {
            self.process_contribution(&contribution)?;
        }

        // Aplicar decay a todos los nodos
        self.apply_decay_to_all()?;

        info!(
            nodes_loaded = self.reputations.len(),
            "Reputations loaded from ledger"
        );

        Ok(())
    }

    /// Procesar contribución y calcular créditos
    pub fn process_contribution(
        &mut self,
        contribution: &Contribution,
    ) -> Result<f64, ScoringError> {
        // FIX: E0308 - node_id is String, not Option<String>
        self.check_antisybil(&contribution.node_id)?;

        // Obtener o crear reputación del nodo
        let reputation = self
            .reputations
            .entry(contribution.node_id.clone())
            .or_insert_with(|| NodeReputation::new(contribution.node_id.clone()));

        // Calcular créditos base según tipo
        let base_credits = match &contribution.contribution_type {
            ContributionType::SaeForward => self.config.base_sae_forward_credits,
            ContributionType::ConsensusBatch => self.config.base_consensus_credits,
            ContributionType::HumanFeedback => self.config.base_feedback_credits,
            ContributionType::ConceptLearned => self.config.base_concept_credits,
            ContributionType::GovernanceProposal => self.config.base_governance_proposal_credits,
            ContributionType::GovernanceVote => self.config.base_governance_vote_credits,
            ContributionType::ModelSync => self.config.base_model_sync_credits,
        };

        // Aplicar multiplicador ZKP
        let zkp_mult = if contribution.zkp_verified {
            self.config.zkp_multiplier
        } else {
            1.0
        };

        // Bonificación por detección de anomalías (simulada: 0.0 - max_anomaly_bonus)
        // TODO: Phase 6 - Integrar con feature_analyzer para bonificación real
        let anomaly_bonus = 0.0;

        // Calcular créditos finales
        let credits = base_credits * (1.0 + anomaly_bonus) * zkp_mult;

        // Actualizar reputación
        reputation.total_earned_credits += credits;
        reputation.current_credits += credits;
        reputation.total_contributions += 1;
        reputation.last_contribution_timestamp = contribution.timestamp;

        if contribution.zkp_verified {
            reputation.zkp_verified_contributions += 1;
        }

        // FIX: E0502 - extract fields before calling self method to release mutable borrow
        let credits_for_score = reputation.current_credits;
        let total_for_score = reputation.total_contributions;
        let zkp_for_score = reputation.zkp_verified_contributions;
        let new_score = Self::calculate_reputation_score_static(
            credits_for_score,
            total_for_score,
            zkp_for_score,
        );
        reputation.reputation_score = new_score;

        info!(
            node_id = %contribution.node_id,
            credits,
            contribution_type = %contribution.contribution_type,
            zkp_verified = contribution.zkp_verified,
            new_score = reputation.reputation_score,
            "Contribution processed"
        );

        Ok(credits)
    }

    /// Calcular score de reputación normalizado (0.0 - 1.0)
    fn calculate_reputation_score(&self, reputation: &NodeReputation) -> f64 {
        Self::calculate_reputation_score_static(
            reputation.current_credits,
            reputation.total_contributions,
            reputation.zkp_verified_contributions,
        )
    }

    /// FIX: E0502 - static version to avoid borrow conflicts
    fn calculate_reputation_score_static(
        current_credits: f64,
        total_contributions: usize,
        zkp_verified_contributions: usize,
    ) -> f64 {
        // Usar función sigmoide para normalizar
        // Score = 1 / (1 + e^(-k * (credits - midpoint)))
        let k = 0.01; // Factor de escala
        let midpoint = 100.0; // Créditos donde score = 0.5

        let score = 1.0 / (1.0 + (-k * (current_credits - midpoint)).exp());

        // Bonus por contribuciones ZKP (máximo +0.1)
        let zkp_bonus = if total_contributions > 0 {
            let zkp_ratio = zkp_verified_contributions as f64 / total_contributions as f64;
            zkp_ratio * 0.1
        } else {
            0.0
        };

        (score + zkp_bonus).min(1.0)
    }

    /// Aplicar decay exponencial a un nodo
    pub fn apply_decay(&mut self, node_id: &str) -> Result<f64, ScoringError> {
        let reputation = self
            .reputations
            .get_mut(node_id)
            .ok_or_else(|| ScoringError::NodeNotFound(node_id.to_string()))?;

        let now = Utc::now().timestamp() as u64;
        let elapsed_days = (now - reputation.last_decay_timestamp) as f64 / 86400.0;

        // Decay exponencial: 50% cada decay_period_days
        let decay_factor = 2.0_f64.powf(-elapsed_days / self.config.decay_period_days as f64);

        let old_credits = reputation.current_credits;
        reputation.current_credits *= decay_factor;
        reputation.last_decay_timestamp = now;

        // FIX: borrow conflict - Inline score calculation to avoid borrowing self while reputation is mutably borrowed
        {
            let k = 0.01;
            let midpoint = 100.0;
            let score = 1.0 / (1.0 + (-k * (reputation.current_credits - midpoint)).exp());
            let zkp_bonus = if reputation.total_contributions > 0 {
                let zkp_ratio = reputation.zkp_verified_contributions as f64
                    / reputation.total_contributions as f64;
                zkp_ratio * 0.1
            } else {
                0.0
            };
            reputation.reputation_score = (score + zkp_bonus).min(1.0);
        }

        let decayed_amount = old_credits - reputation.current_credits;

        if decayed_amount > 1.0 {
            info!(
                node_id,
                old_credits,
                new_credits = reputation.current_credits,
                decayed_amount,
                decay_factor,
                "Decay applied"
            );
        }

        Ok(decayed_amount)
    }

    /// Aplicar decay a todos los nodos
    pub fn apply_decay_to_all(&mut self) -> Result<usize, ScoringError> {
        let node_ids: Vec<String> = self.reputations.keys().cloned().collect();

        let mut count = 0;
        for node_id in node_ids {
            self.apply_decay(&node_id)?;
            count += 1;
        }

        Ok(count)
    }

    /// Verificar límites anti-Sybil
    fn check_antisybil(&mut self, identifier: &str) -> Result<(), ScoringError> {
        let now = Utc::now().timestamp() as u64;
        let period_seconds = self.config.antisybil_period_hours * 3600;

        let entry = self
            .antisybil_tracker
            .entry(identifier.to_string())
            .or_insert((0.0, now));

        // Resetear período si expiró
        if now - entry.1 > period_seconds {
            entry.0 = 0.0;
            entry.1 = now;
        }

        // Verificar límite
        if entry.0 >= self.config.antisybil_limit_per_period {
            warn!(
                identifier,
                current_credits = entry.0,
                limit = self.config.antisybil_limit_per_period,
                "Anti-Sybil limit exceeded"
            );
            return Err(ScoringError::AntiSybilLimitExceeded {
                identifier: identifier.to_string(),
            });
        }

        Ok(())
    }

    /// Obtener reputación de un nodo
    pub fn get_reputation(&self, node_id: &str) -> Option<&NodeReputation> {
        self.reputations.get(node_id)
    }

    /// Verificar si un nodo pasa el umbral de gobernanza
    pub fn can_participate_in_governance(&self, node_id: &str) -> bool {
        match self.reputations.get(node_id) {
            Some(rep) => rep.reputation_score >= self.config.governance_minimum_reputation,
            None => false,
        }
    }

    /// Obtener ranking de nodos por reputación
    pub fn get_ranking(&self) -> Vec<(String, f64)> {
        let mut ranking: Vec<(String, f64)> = self
            .reputations
            .iter()
            .map(|(id, rep)| (id.clone(), rep.reputation_score))
            .collect();

        ranking.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranking
    }

    /// Estadísticas globales
    pub fn global_stats(&self) -> GlobalReputationStats {
        let total_nodes = self.reputations.len();
        let total_credits: f64 = self.reputations.values().map(|r| r.current_credits).sum();

        let avg_score = if total_nodes > 0 {
            self.reputations
                .values()
                .map(|r| r.reputation_score)
                .sum::<f64>()
                / total_nodes as f64
        } else {
            0.0
        };

        let governance_eligible = self
            .reputations
            .values()
            .filter(|r| r.reputation_score >= self.config.governance_minimum_reputation)
            .count();

        GlobalReputationStats {
            total_nodes,
            total_credits,
            average_reputation_score: avg_score,
            governance_eligible,
            decay_period_days: self.config.decay_period_days,
            zkp_multiplier: self.config.zkp_multiplier,
        }
    }

    /// Obtener configuración
    pub fn config(&self) -> &ScoringConfig {
        &self.config
    }
}

impl Default for ReputationScorer {
    fn default() -> Self {
        Self::new()
    }
}

/// Estadísticas globales de reputación
#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalReputationStats {
    pub total_nodes: usize,
    pub total_credits: f64,
    pub average_reputation_score: f64,
    pub governance_eligible: usize,
    pub decay_period_days: u64,
    pub zkp_multiplier: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scorer_creation() {
        let scorer = ReputationScorer::new();
        assert_eq!(scorer.reputations.len(), 0);
    }

    #[test]
    fn test_contribution_scoring() {
        let mut scorer = ReputationScorer::new();

        let contribution = Contribution::new(
            "node_1".to_string(),
            Some("layer_0".to_string()),
            "batch_abc".to_string(),
            true, // ZKP verified
            ContributionType::SaeForward,
            10.0,
            None,
        );

        let credits = scorer.process_contribution(&contribution).unwrap();
        // base * zkp_multiplier = 10.0 * 1.5 = 15.0
        assert!((credits - 15.0).abs() < 0.01);

        let rep = scorer.get_reputation("node_1").unwrap();
        assert!(rep.reputation_score > 0.0);
    }

    #[test]
    fn test_reputation_ranking() {
        let mut scorer = ReputationScorer::new();

        // Add two nodes
        let c1 = Contribution::new(
            "node_a".to_string(),
            None,
            "batch_1".to_string(),
            true,
            ContributionType::SaeForward,
            10.0,
            None,
        );
        let c2 = Contribution::new(
            "node_b".to_string(),
            None,
            "batch_2".to_string(),
            false,
            ContributionType::HumanFeedback,
            5.0,
            None,
        );

        scorer.process_contribution(&c1).unwrap();
        scorer.process_contribution(&c2).unwrap();

        let ranking = scorer.get_ranking();
        assert_eq!(ranking.len(), 2);
        // node_a should be first (higher credits with ZKP)
        assert_eq!(ranking[0].0, "node_a");
    }

    #[test]
    fn test_global_stats() {
        let scorer = ReputationScorer::new();
        let stats = scorer.global_stats();
        assert_eq!(stats.total_nodes, 0);
        assert_eq!(stats.total_credits, 0.0);
    }
}
