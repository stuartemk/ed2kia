//! Technical Staking — Reputation-weighted technical staking for DAO governance.
//!
//! Nodes stake technical reputation to gain voting weight in DAO governance.
//! No financial tokens: staking represents commitment of compute resources
//! and technical reputation. Analogous to Linux's `nice` priority but for
//! federated governance decisions.
//!
//! Zero financial logic: staking = technical reputation + compute credits.

use std::collections::HashMap;

/// Errors for technical staking operations.
#[derive(Debug)]
pub enum StakingError {
    NodeNotFound(String),
    InsufficientReputation { available: f64, required: f64 },
    InsufficientCredits { available: f64, required: f64 },
    AlreadyStaked(String),
    NotStaked(String),
    MaxStakeExceeded(f64),
}

impl std::fmt::Display for StakingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StakingError::NodeNotFound(id) => {
                write!(f, "Node not found: {}", id)
            }
            StakingError::InsufficientReputation {
                available,
                required,
            } => {
                write!(
                    f,
                    "Insufficient reputation: available={}, required={}",
                    available, required
                )
            }
            StakingError::InsufficientCredits {
                available,
                required,
            } => {
                write!(
                    f,
                    "Insufficient credits: available={}, required={}",
                    available, required
                )
            }
            StakingError::AlreadyStaked(id) => {
                write!(f, "Node already staked: {}", id)
            }
            StakingError::NotStaked(id) => {
                write!(f, "Node not staked: {}", id)
            }
            StakingError::MaxStakeExceeded(max) => {
                write!(f, "Max stake exceeded: {}", max)
            }
        }
    }
}

/// Configuration for technical staking.
#[derive(Debug, Clone)]
pub struct StakingConfig {
    /// Minimum reputation required to stake.
    pub min_reputation: f64,
    /// Minimum compute credits required to stake.
    pub min_credits: f64,
    /// Maximum stake weight (prevents dominance).
    pub max_stake_weight: f64,
    /// Decay rate for inactive stakes per epoch.
    pub inactivity_decay: f64,
    /// Epoch duration in milliseconds.
    pub epoch_duration_ms: u64,
}

impl Default for StakingConfig {
    fn default() -> Self {
        Self {
            min_reputation: 0.3,
            min_credits: 10.0,
            max_stake_weight: 0.8,
            inactivity_decay: 0.02,
            epoch_duration_ms: 3_600_000, // 1 hour
        }
    }
}

/// A node's stake record.
#[derive(Debug, Clone)]
pub struct StakeRecord {
    /// Node identifier.
    pub node_id: String,
    /// Staked reputation amount.
    pub staked_reputation: f64,
    /// Staked compute credits.
    pub staked_credits: f64,
    /// Computed voting weight (0.0–1.0).
    pub voting_weight: f64,
    /// Epoch when stake was placed.
    pub stake_epoch: u64,
    /// Last active epoch.
    pub last_active_epoch: u64,
    /// Total epochs active.
    pub epochs_active: u64,
}

impl StakeRecord {
    /// Create a new stake record.
    pub fn new(
        node_id: String,
        staked_reputation: f64,
        staked_credits: f64,
        current_epoch: u64,
    ) -> Self {
        let voting_weight = compute_voting_weight(staked_reputation, staked_credits);
        Self {
            node_id,
            staked_reputation,
            staked_credits,
            voting_weight,
            stake_epoch: current_epoch,
            last_active_epoch: current_epoch,
            epochs_active: 1,
        }
    }

    /// Check if stake is active (not decayed below threshold).
    pub fn is_active(&self, min_weight: f64) -> bool {
        self.voting_weight >= min_weight
    }
}

/// Node profile with available reputation and credits.
#[derive(Debug, Clone)]
pub struct NodeStakeProfile {
    /// Node identifier.
    pub node_id: String,
    /// Total technical reputation (0.0–1.0).
    pub reputation: f64,
    /// Available compute credits.
    pub available_credits: f64,
    /// Currently staked credits.
    pub staked_credits: f64,
    /// Is currently staking.
    pub is_staking: bool,
}

impl NodeStakeProfile {
    /// Create a new node stake profile.
    pub fn new(node_id: String, reputation: f64, available_credits: f64) -> Self {
        Self {
            node_id,
            reputation: reputation.clamp(0.0, 1.0),
            available_credits: available_credits.max(0.0),
            staked_credits: 0.0,
            is_staking: false,
        }
    }

    /// Unstaked credits available for staking.
    pub fn unstaked_credits(&self) -> f64 {
        self.available_credits - self.staked_credits
    }
}

/// Staking statistics.
#[derive(Debug, Clone, Default)]
pub struct StakingStats {
    /// Total nodes registered.
    pub total_nodes: usize,
    /// Total active stakes.
    pub active_stakes: usize,
    /// Total voting weight distributed.
    pub total_voting_weight: f64,
    /// Current epoch.
    pub current_epoch: u64,
    /// Total stakes placed.
    pub total_stakes_placed: usize,
    /// Total stakes withdrawn.
    pub total_stakes_withdrawn: usize,
}

/// Technical Staking engine for DAO governance.
pub struct TechnicalStaking {
    /// Staking configuration.
    pub config: StakingConfig,
    /// Node profiles.
    profiles: HashMap<String, NodeStakeProfile>,
    /// Active stake records.
    stakes: HashMap<String, StakeRecord>,
    /// Staking statistics.
    stats: StakingStats,
}

impl TechnicalStaking {
    /// Create a new staking engine with config.
    pub fn new(config: StakingConfig) -> Self {
        Self {
            config,
            profiles: HashMap::new(),
            stakes: HashMap::new(),
            stats: StakingStats::default(),
        }
    }

    /// Create staking engine with default config.
    pub fn with_defaults() -> Self {
        Self::new(StakingConfig::default())
    }

    /// Register a node with reputation and credits.
    pub fn register_node(&mut self, node_id: String, reputation: f64, credits: f64) {
        let profile = NodeStakeProfile::new(node_id.clone(), reputation, credits);
        self.profiles.insert(node_id, profile);
        self.stats.total_nodes = self.profiles.len();
    }

    /// Update node reputation.
    pub fn update_reputation(
        &mut self,
        node_id: &str,
        reputation: f64,
    ) -> Result<(), StakingError> {
        let profile = self
            .profiles
            .get_mut(node_id)
            .ok_or(StakingError::NodeNotFound(node_id.to_string()))?;
        profile.reputation = reputation.clamp(0.0, 1.0);
        Ok(())
    }

    /// Update node available credits.
    pub fn update_credits(&mut self, node_id: &str, credits: f64) -> Result<(), StakingError> {
        let profile = self
            .profiles
            .get_mut(node_id)
            .ok_or(StakingError::NodeNotFound(node_id.to_string()))?;
        profile.available_credits = credits.max(0.0);
        Ok(())
    }

    /// Place a stake (commit reputation and credits for voting weight).
    pub fn place_stake(
        &mut self,
        node_id: &str,
        reputation_amount: f64,
        credit_amount: f64,
    ) -> Result<StakeRecord, StakingError> {
        // Check not already staked
        if self.stakes.contains_key(node_id) {
            return Err(StakingError::AlreadyStaked(node_id.to_string()));
        }

        let profile = self
            .profiles
            .get(node_id)
            .ok_or(StakingError::NodeNotFound(node_id.to_string()))?;

        // Check minimum reputation
        if profile.reputation < self.config.min_reputation {
            return Err(StakingError::InsufficientReputation {
                available: profile.reputation,
                required: self.config.min_reputation,
            });
        }

        // Check reputation amount
        if reputation_amount > profile.reputation {
            return Err(StakingError::InsufficientReputation {
                available: profile.reputation,
                required: reputation_amount,
            });
        }

        // Check available credits
        let unstaked = profile.unstaked_credits();
        if credit_amount > unstaked {
            return Err(StakingError::InsufficientCredits {
                available: unstaked,
                required: credit_amount,
            });
        }

        // Compute voting weight
        let weight = compute_voting_weight(reputation_amount, credit_amount);
        if weight > self.config.max_stake_weight {
            return Err(StakingError::MaxStakeExceeded(self.config.max_stake_weight));
        }

        // Create stake record
        let stake = StakeRecord::new(
            node_id.to_string(),
            reputation_amount,
            credit_amount,
            self.stats.current_epoch,
        );

        // Update profile
        let profile = self.profiles.get_mut(node_id).unwrap();
        profile.staked_credits += credit_amount;
        profile.is_staking = true;

        // Store stake
        self.stakes.insert(node_id.to_string(), stake.clone());

        // Update stats
        self.stats.active_stakes = self.stakes.len();
        self.stats.total_voting_weight = self.stakes.values().map(|s| s.voting_weight).sum();
        self.stats.total_stakes_placed += 1;

        Ok(stake)
    }

    /// Withdraw a stake (release reputation and credits).
    pub fn withdraw_stake(&mut self, node_id: &str) -> Result<StakeRecord, StakingError> {
        let stake = self
            .stakes
            .remove(node_id)
            .ok_or(StakingError::NotStaked(node_id.to_string()))?;

        // Update profile
        if let Some(profile) = self.profiles.get_mut(node_id) {
            profile.staked_credits = profile.staked_credits.max(0.0) - stake.staked_credits;
            profile.is_staking = false;
        }

        // Update stats
        self.stats.active_stakes = self.stakes.len();
        self.stats.total_voting_weight = self.stakes.values().map(|s| s.voting_weight).sum();
        self.stats.total_stakes_withdrawn += 1;

        Ok(stake)
    }

    /// Get a node's stake record.
    pub fn get_stake(&self, node_id: &str) -> Option<&StakeRecord> {
        self.stakes.get(node_id)
    }

    /// Get a node's profile.
    pub fn get_profile(&self, node_id: &str) -> Option<&NodeStakeProfile> {
        self.profiles.get(node_id)
    }

    /// Get all active stakes.
    pub fn get_active_stakes(&self) -> Vec<&StakeRecord> {
        self.stakes.values().collect()
    }

    /// Get voting weight for a node.
    pub fn get_voting_weight(&self, node_id: &str) -> Option<f64> {
        self.stakes.get(node_id).map(|s| s.voting_weight)
    }

    /// Advance to next epoch and apply decay.
    pub fn advance_epoch(&mut self) {
        self.stats.current_epoch += 1;
        for stake in self.stakes.values_mut() {
            if stake.last_active_epoch < self.stats.current_epoch {
                // Apply inactivity decay
                stake.voting_weight *= 1.0 - self.config.inactivity_decay;
                stake.voting_weight = stake.voting_weight.max(0.0);
            }
        }
        self.stats.total_voting_weight = self.stakes.values().map(|s| s.voting_weight).sum();
    }

    /// Record node activity (resets decay for this epoch).
    pub fn record_activity(&mut self, node_id: &str) -> Result<(), StakingError> {
        let stake = self
            .stakes
            .get_mut(node_id)
            .ok_or(StakingError::NotStaked(node_id.to_string()))?;
        stake.last_active_epoch = self.stats.current_epoch + 1;
        stake.epochs_active += 1;
        Ok(())
    }

    /// Get current statistics.
    pub fn get_stats(&self) -> StakingStats {
        self.stats.clone()
    }

    /// Reset statistics.
    pub fn reset_stats(&mut self) {
        self.stats = StakingStats::default();
        self.stats.total_nodes = self.profiles.len();
        self.stats.active_stakes = self.stakes.len();
    }
}

impl Default for TechnicalStaking {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Compute voting weight from reputation and credits.
fn compute_voting_weight(reputation: f64, credits: f64) -> f64 {
    // Weighted combination: 60% reputation, 40% normalized credits
    let normalized_credits = credits / (credits + 100.0); // Soft cap at ~100 credits
    let weight = reputation * 0.6 + normalized_credits * 0.4;
    weight.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_staking_creation() {
        let staking = TechnicalStaking::with_defaults();
        assert_eq!(staking.get_stats().total_nodes, 0);
        assert_eq!(staking.get_stats().active_stakes, 0);
    }

    #[test]
    fn test_register_node() {
        let mut staking = TechnicalStaking::with_defaults();
        staking.register_node("node1".to_string(), 0.8, 50.0);
        assert_eq!(staking.get_stats().total_nodes, 1);
        let profile = staking.get_profile("node1").unwrap();
        assert_eq!(profile.reputation, 0.8);
        assert_eq!(profile.available_credits, 50.0);
    }

    #[test]
    fn test_place_stake() {
        let mut staking = TechnicalStaking::with_defaults();
        staking.register_node("node1".to_string(), 0.8, 50.0);
        let stake = staking.place_stake("node1", 0.5, 20.0).unwrap();
        assert_eq!(stake.node_id, "node1");
        assert_eq!(stake.staked_reputation, 0.5);
        assert_eq!(stake.staked_credits, 20.0);
        assert!(stake.voting_weight > 0.0);
    }

    #[test]
    fn test_place_stake_insufficient_reputation() {
        let mut staking = TechnicalStaking::with_defaults();
        staking.register_node("node1".to_string(), 0.1, 50.0);
        match staking.place_stake("node1", 0.5, 20.0) {
            Err(StakingError::InsufficientReputation { .. }) => {}
            _ => panic!("Expected InsufficientReputation"),
        }
    }

    #[test]
    fn test_place_stake_insufficient_credits() {
        let mut staking = TechnicalStaking::with_defaults();
        staking.register_node("node1".to_string(), 0.8, 10.0);
        match staking.place_stake("node1", 0.5, 50.0) {
            Err(StakingError::InsufficientCredits { .. }) => {}
            _ => panic!("Expected InsufficientCredits"),
        }
    }

    #[test]
    fn test_place_stake_already_staked() {
        let mut staking = TechnicalStaking::with_defaults();
        staking.register_node("node1".to_string(), 0.8, 50.0);
        staking.place_stake("node1", 0.5, 20.0).unwrap();
        match staking.place_stake("node1", 0.3, 10.0) {
            Err(StakingError::AlreadyStaked(id)) => assert_eq!(id, "node1"),
            _ => panic!("Expected AlreadyStaked"),
        }
    }

    #[test]
    fn test_withdraw_stake() {
        let mut staking = TechnicalStaking::with_defaults();
        staking.register_node("node1".to_string(), 0.8, 50.0);
        staking.place_stake("node1", 0.5, 20.0).unwrap();
        let withdrawn = staking.withdraw_stake("node1").unwrap();
        assert_eq!(withdrawn.node_id, "node1");
        assert!(staking.get_stake("node1").is_none());
    }

    #[test]
    fn test_withdraw_not_staked() {
        let mut staking = TechnicalStaking::with_defaults();
        staking.register_node("node1".to_string(), 0.8, 50.0);
        match staking.withdraw_stake("node1") {
            Err(StakingError::NotStaked(id)) => assert_eq!(id, "node1"),
            _ => panic!("Expected NotStaked"),
        }
    }

    #[test]
    fn test_voting_weight_calculation() {
        let mut staking = TechnicalStaking::with_defaults();
        staking.register_node("node1".to_string(), 0.9, 100.0);
        let stake = staking.place_stake("node1", 0.9, 100.0).unwrap();
        // High reputation + high credits = high weight
        assert!(stake.voting_weight > 0.5);
    }

    #[test]
    fn test_advance_epoch_decay() {
        let mut staking = TechnicalStaking::with_defaults();
        staking.register_node("node1".to_string(), 0.8, 50.0);
        staking.place_stake("node1", 0.5, 20.0).unwrap();
        let initial_weight = staking.get_stake("node1").unwrap().voting_weight;
        staking.advance_epoch();
        let decayed_weight = staking.get_stake("node1").unwrap().voting_weight;
        assert!(decayed_weight < initial_weight);
    }

    #[test]
    fn test_record_activity_prevents_decay() {
        let mut staking = TechnicalStaking::with_defaults();
        staking.register_node("node1".to_string(), 0.8, 50.0);
        staking.place_stake("node1", 0.5, 20.0).unwrap();
        staking.record_activity("node1").unwrap();
        let weight_before = staking.get_stake("node1").unwrap().voting_weight;
        staking.advance_epoch();
        let weight_after = staking.get_stake("node1").unwrap().voting_weight;
        // Activity recorded, so no decay
        assert_eq!(weight_before, weight_after);
    }

    #[test]
    fn test_update_reputation() {
        let mut staking = TechnicalStaking::with_defaults();
        staking.register_node("node1".to_string(), 0.5, 50.0);
        staking.update_reputation("node1", 0.9).unwrap();
        assert_eq!(staking.get_profile("node1").unwrap().reputation, 0.9);
    }

    #[test]
    fn test_update_credits() {
        let mut staking = TechnicalStaking::with_defaults();
        staking.register_node("node1".to_string(), 0.8, 50.0);
        staking.update_credits("node1", 100.0).unwrap();
        assert_eq!(
            staking.get_profile("node1").unwrap().available_credits,
            100.0
        );
    }

    #[test]
    fn test_stats_tracking() {
        let mut staking = TechnicalStaking::with_defaults();
        staking.register_node("node1".to_string(), 0.8, 50.0);
        staking.place_stake("node1", 0.5, 20.0).unwrap();
        let stats = staking.get_stats();
        assert_eq!(stats.total_nodes, 1);
        assert_eq!(stats.active_stakes, 1);
        assert_eq!(stats.total_stakes_placed, 1);
    }

    #[test]
    fn test_reset_stats() {
        let mut staking = TechnicalStaking::with_defaults();
        staking.register_node("node1".to_string(), 0.8, 50.0);
        staking.place_stake("node1", 0.5, 20.0).unwrap();
        staking.reset_stats();
        let stats = staking.get_stats();
        assert_eq!(stats.total_stakes_placed, 0);
        assert_eq!(stats.total_stakes_withdrawn, 0);
    }

    #[test]
    fn test_node_profile_unstaked_credits() {
        let profile = NodeStakeProfile::new("n1".to_string(), 0.8, 50.0);
        assert_eq!(profile.unstaked_credits(), 50.0);
        let profile_with_stake = NodeStakeProfile {
            staked_credits: 20.0,
            ..profile
        };
        assert_eq!(profile_with_stake.unstaked_credits(), 30.0);
    }

    #[test]
    fn test_stake_is_active() {
        let stake = StakeRecord::new("n1".to_string(), 0.5, 20.0, 1);
        assert!(stake.is_active(0.1));
        assert!(!stake.is_active(0.99));
    }

    #[test]
    fn test_config_default() {
        let config = StakingConfig::default();
        assert_eq!(config.min_reputation, 0.3);
        assert_eq!(config.min_credits, 10.0);
        assert_eq!(config.max_stake_weight, 0.8);
        assert_eq!(config.inactivity_decay, 0.02);
    }

    #[test]
    fn test_stats_default() {
        let stats = StakingStats::default();
        assert_eq!(stats.total_nodes, 0);
        assert_eq!(stats.active_stakes, 0);
        assert_eq!(stats.current_epoch, 0);
    }

    #[test]
    fn test_staking_default() {
        let staking = TechnicalStaking::default();
        assert_eq!(staking.get_stats().total_nodes, 0);
    }

    #[test]
    fn test_reputation_clamping() {
        let mut staking = TechnicalStaking::with_defaults();
        staking.register_node("n1".to_string(), 1.5, 50.0);
        assert_eq!(staking.get_profile("n1").unwrap().reputation, 1.0);
    }

    #[test]
    fn test_error_display() {
        match StakingError::NodeNotFound("x".to_string()) {
            e => assert!(!e.to_string().is_empty()),
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_get_voting_weight() {
        let mut staking = TechnicalStaking::with_defaults();
        staking.register_node("n1".to_string(), 0.8, 50.0);
        staking.place_stake("n1", 0.5, 20.0).unwrap();
        let weight = staking.get_voting_weight("n1").unwrap();
        assert!(weight > 0.0);
        assert!(staking.get_voting_weight("n2").is_none());
    }

    #[test]
    fn test_multiple_stakes() {
        let mut staking = TechnicalStaking::with_defaults();
        staking.register_node("n1".to_string(), 0.8, 50.0);
        staking.register_node("n2".to_string(), 0.9, 100.0);
        staking.place_stake("n1", 0.5, 20.0).unwrap();
        staking.place_stake("n2", 0.6, 30.0).unwrap();
        assert_eq!(staking.get_active_stakes().len(), 2);
        assert_eq!(staking.get_stats().active_stakes, 2);
    }
}
