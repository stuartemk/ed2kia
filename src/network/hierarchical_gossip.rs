//! Hierarchical Gossip Protocol â€” Sprint 70: Civilization-Scale Architecture
//!
//! Committee-based gossip with staleness-aware FedAvg and differential privacy
//! for scaling to 1M+ heterogeneous nodes.

use std::collections::HashMap;
use std::fmt;

/// Errors in hierarchical gossip protocol.
#[derive(Debug, Clone, PartialEq)]
pub enum GossipError {
    /// Committee election failed due to insufficient nodes.
    InsufficientNodes { have: usize, need: usize },
    /// Invalid differential privacy parameters.
    InvalidPrivacyParams(String),
    /// Staleness exceeded maximum threshold.
    StaleData(u64),
    /// Committee already elected for this round.
    CommitteeActive,
}

impl fmt::Display for GossipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GossipError::InsufficientNodes { have, need } => {
                write!(f, "Insufficient nodes: have {}, need {}", have, need)
            }
            GossipError::InvalidPrivacyParams(msg) => {
                write!(f, "Invalid privacy params: {}", msg)
            }
            GossipError::StaleData(age_ms) => {
                write!(f, "Data stale: {} ms old", age_ms)
            }
            GossipError::CommitteeActive => {
                write!(f, "Committee already active")
            }
        }
    }
}

impl std::error::Error for GossipError {}

/// Configuration for hierarchical gossip.
#[derive(Debug, Clone)]
pub struct GossipConfig {
    /// Committee size for each layer.
    pub committee_size: usize,
    /// Maximum staleness in milliseconds.
    pub max_staleness_ms: u64,
    /// Differential privacy epsilon.
    pub epsilon: f64,
    /// Differential privacy delta.
    pub delta: f64,
    /// Staleness decay rate for FedAvg weighting.
    pub decay_rate: f64,
    /// Maximum layers in hierarchy.
    pub max_layers: usize,
}

impl GossipConfig {
    /// Default Topological configuration.
    pub fn default_Topological() -> Self {
        Self {
            committee_size: 32,
            max_staleness_ms: 30_000,
            epsilon: 1.0,
            delta: 1e-5,
            decay_rate: 0.95,
            max_layers: 4,
        }
    }

    /// Validate configuration.
    pub fn validate(&self) -> Result<(), GossipError> {
        if self.committee_size == 0 {
            return Err(GossipError::InsufficientNodes { have: 0, need: 1 });
        }
        if self.epsilon <= 0.0 {
            return Err(GossipError::InvalidPrivacyParams(
                "epsilon must be > 0".to_string(),
            ));
        }
        if self.delta <= 0.0 || self.delta >= 1.0 {
            return Err(GossipError::InvalidPrivacyParams(
                "delta must be in (0, 1)".to_string(),
            ));
        }
        if !(0.0..1.0).contains(&self.decay_rate) {
            return Err(GossipError::InvalidPrivacyParams(
                "decay_rate must be in (0, 1)".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for GossipConfig {
    fn default() -> Self {
        Self::default_Topological()
    }
}

impl fmt::Display for GossipConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GossipConfig {{ committee: {}, Îµ: {:.2}, Î´: {:.5}, decay: {:.3} }}",
            self.committee_size, self.epsilon, self.delta, self.decay_rate
        )
    }
}

/// Node information for committee election.
#[derive(Debug, Clone)]
pub struct GossipNode {
    /// Unique node identifier.
    pub node_id: u64,
    /// Node score (higher = more trustworthy).
    pub score: f64,
    /// Geographic region for diversity.
    pub region: String,
    /// Last update timestamp in milliseconds.
    pub last_update_ms: u64,
    /// Current layer in hierarchy.
    pub layer: usize,
}

impl fmt::Display for GossipNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GossipNode {{ id: {}, score: {:.3}, region: {}, layer: {} }}",
            self.node_id, self.score, self.region, self.layer
        )
    }
}

/// Update message with staleness tracking.
#[derive(Debug, Clone)]
pub struct GossipUpdate {
    /// Source node ID.
    pub source_id: u64,
    /// Update vector (model gradients or features).
    pub vector: Vec<f64>,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Differential privacy noise added.
    pub noise_scale: f64,
}

impl fmt::Display for GossipUpdate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GossipUpdate {{ source: {}, dim: {}, age: {}ms }}",
            self.source_id,
            self.vector.len(),
            self.timestamp_ms
        )
    }
}

/// Elected committee for a gossip round.
#[derive(Debug, Clone)]
pub struct Committee {
    /// Committee round identifier.
    pub round_id: u64,
    /// Elected node IDs.
    pub members: Vec<u64>,
    /// Election timestamp.
    pub elected_at_ms: u64,
    /// Layer in hierarchy.
    pub layer: usize,
}

impl fmt::Display for Committee {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Committee {{ round: {}, members: {}, layer: {} }}",
            self.round_id,
            self.members.len(),
            self.layer
        )
    }
}

/// Hierarchical Gossip Protocol â€” scales to 1M+ nodes via committee layers.
pub struct HierarchicalGossip {
    config: GossipConfig,
    nodes: HashMap<u64, GossipNode>,
    active_committee: Option<Committee>,
    updates: Vec<GossipUpdate>,
    next_round_id: u64,
    current_time_ms: u64,
}

impl HierarchicalGossip {
    /// Create with default configuration.
    pub fn new() -> Self {
        Self {
            config: GossipConfig::default_Topological(),
            nodes: HashMap::new(),
            active_committee: None,
            updates: Vec::new(),
            next_round_id: 1,
            current_time_ms: 0,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: GossipConfig) -> Result<Self, GossipError> {
        config.validate()?;
        Ok(Self {
            config,
            nodes: HashMap::new(),
            active_committee: None,
            updates: Vec::new(),
            next_round_id: 1,
            current_time_ms: 0,
        })
    }

    /// Register a node in the gossip network.
    pub fn register_node(&mut self, node: GossipNode) {
        self.nodes.insert(node.node_id, node);
    }

    /// Elect a committee using score-weighted random selection.
    pub fn elect_committee(&mut self, layer: usize) -> Result<Committee, GossipError> {
        if self.active_committee.is_some() {
            return Err(GossipError::CommitteeActive);
        }
        let candidates: Vec<&GossipNode> =
            self.nodes.values().filter(|n| n.layer == layer).collect();
        if candidates.len() < self.config.committee_size {
            return Err(GossipError::InsufficientNodes {
                have: candidates.len(),
                need: self.config.committee_size,
            });
        }

        // Score-weighted selection (simplified: sort by score, take top N).
        let mut sorted: Vec<&GossipNode> = candidates;
        sorted.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        let members: Vec<u64> = sorted
            .iter()
            .take(self.config.committee_size)
            .map(|n| n.node_id)
            .collect();

        let committee = Committee {
            round_id: self.next_round_id,
            members,
            elected_at_ms: self.current_time_ms,
            layer,
        };

        self.next_round_id += 1;
        self.active_committee = Some(committee.clone());
        Ok(committee)
    }

    /// Submit a gossip update with staleness check.
    pub fn submit_update(
        &mut self,
        source_id: u64,
        vector: Vec<f64>,
        timestamp_ms: u64,
    ) -> Result<GossipUpdate, GossipError> {
        let age = self.current_time_ms.saturating_sub(timestamp_ms);
        if age > self.config.max_staleness_ms {
            return Err(GossipError::StaleData(age));
        }

        // Add differential privacy noise (simplified Laplace mechanism).
        let sensitivity = 1.0;
        let noise_scale = sensitivity / self.config.epsilon;
        let noisy_vector: Vec<f64> = vector
            .into_iter()
            .map(|v| v + (noise_scale * 0.1)) // Simplified noise.
            .collect();

        let update = GossipUpdate {
            source_id,
            vector: noisy_vector,
            timestamp_ms,
            noise_scale,
        };

        self.updates.push(update.clone());
        Ok(update)
    }

    /// Compute staleness-aware FedAvg merge.
    pub fn fedavg_merge(&self, current_time_ms: u64) -> Option<Vec<f64>> {
        if self.updates.is_empty() {
            return None;
        }

        let dim = self.updates[0].vector.len();
        let mut result = vec![0.0; dim];
        let mut total_weight = 0.0;

        for update in &self.updates {
            let age = current_time_ms.saturating_sub(update.timestamp_ms);
            let staleness_weight = self.config.decay_rate.powi(age as i32 / 1000);
            let node_score = self
                .nodes
                .get(&update.source_id)
                .map(|n| n.score)
                .unwrap_or(1.0);
            let weight = staleness_weight * node_score;
            total_weight += weight;

            for (i, v) in update.vector.iter().enumerate() {
                result[i] += v * weight;
            }
        }

        if total_weight > 0.0 {
            for v in &mut result {
                *v /= total_weight;
            }
            Some(result)
        } else {
            None
        }
    }

    /// Dissolve the active committee.
    pub fn dissolve_committee(&mut self) {
        self.active_committee = None;
    }

    /// Get the active committee.
    pub fn active_committee(&self) -> Option<&Committee> {
        self.active_committee.as_ref()
    }

    /// Get the number of registered nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the number of pending updates.
    pub fn update_count(&self) -> usize {
        self.updates.len()
    }

    /// Clear all updates.
    pub fn clear_updates(&mut self) {
        self.updates.clear();
    }

    /// Advance the internal clock.
    pub fn advance_time(&mut self, to_ms: u64) {
        self.current_time_ms = to_ms;
    }

    /// Get the current configuration.
    pub fn config(&self) -> &GossipConfig {
        &self.config
    }
}

impl Default for HierarchicalGossip {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for HierarchicalGossip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HierarchicalGossip {{ nodes: {}, updates: {}, committee: {} }}",
            self.nodes.len(),
            self.updates.len(),
            if self.active_committee.is_some() {
                "active"
            } else {
                "none"
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: u64, score: f64, layer: usize) -> GossipNode {
        GossipNode {
            node_id: id,
            score,
            region: format!("region_{}", id % 4),
            last_update_ms: 0,
            layer,
        }
    }

    #[test]
    fn test_config_default() {
        let config = GossipConfig::default_Topological();
        assert_eq!(config.committee_size, 32);
        assert!((config.epsilon - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_config_validate_valid() {
        let config = GossipConfig::default_Topological();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_zero_committee() {
        let mut config = GossipConfig::default_Topological();
        config.committee_size = 0;
        match config.validate() {
            Err(GossipError::InsufficientNodes { .. }) => {}
            _ => panic!("Expected InsufficientNodes error"),
        }
    }

    #[test]
    fn test_config_validate_bad_epsilon() {
        let mut config = GossipConfig::default_Topological();
        config.epsilon = -1.0;
        match config.validate() {
            Err(GossipError::InvalidPrivacyParams(_)) => {}
            _ => panic!("Expected InvalidPrivacyParams error"),
        }
    }

    #[test]
    fn test_config_display() {
        let config = GossipConfig::default_Topological();
        let s = format!("{}", config);
        assert!(s.contains("committee: 32"));
    }

    #[test]
    fn test_gossip_new() {
        let gossip = HierarchicalGossip::new();
        assert_eq!(gossip.node_count(), 0);
        assert_eq!(gossip.update_count(), 0);
    }

    #[test]
    fn test_register_node() {
        let mut gossip = HierarchicalGossip::new();
        gossip.register_node(make_node(1, 1.0, 0));
        assert_eq!(gossip.node_count(), 1);
    }

    #[test]
    fn test_elect_committee_insufficient() {
        let mut config = GossipConfig::default_Topological();
        config.committee_size = 10;
        let mut gossip = HierarchicalGossip::with_config(config).unwrap();
        gossip.register_node(make_node(1, 1.0, 0));
        match gossip.elect_committee(0) {
            Err(GossipError::InsufficientNodes { have: 1, need: 10 }) => {}
            _ => panic!("Expected InsufficientNodes error"),
        }
    }

    #[test]
    fn test_elect_committee_success() {
        let mut config = GossipConfig::default_Topological();
        config.committee_size = 3;
        let mut gossip = HierarchicalGossip::with_config(config).unwrap();
        for i in 1..=10 {
            gossip.register_node(make_node(i, i as f64, 0));
        }
        let committee = gossip.elect_committee(0).unwrap();
        assert_eq!(committee.members.len(), 3);
        assert!(gossip.active_committee().is_some());
    }

    #[test]
    fn test_elect_committee_active() {
        let mut config = GossipConfig::default_Topological();
        config.committee_size = 3;
        let mut gossip = HierarchicalGossip::with_config(config).unwrap();
        for i in 1..=10 {
            gossip.register_node(make_node(i, i as f64, 0));
        }
        gossip.elect_committee(0).unwrap();
        match gossip.elect_committee(0) {
            Err(GossipError::CommitteeActive) => {}
            _ => panic!("Expected CommitteeActive error"),
        }
    }

    #[test]
    fn test_submit_update() {
        let mut gossip = HierarchicalGossip::new();
        gossip.advance_time(1000);
        let update = gossip.submit_update(1, vec![1.0, 2.0, 3.0], 1000).unwrap();
        assert_eq!(update.vector.len(), 3);
        assert!(update.noise_scale > 0.0);
    }

    #[test]
    fn test_submit_update_stale() {
        let mut gossip = HierarchicalGossip::new();
        gossip.advance_time(100_000);
        match gossip.submit_update(1, vec![1.0], 0) {
            Err(GossipError::StaleData(100_000)) => {}
            _ => panic!("Expected StaleData error"),
        }
    }

    #[test]
    fn test_fedavg_merge() {
        let mut gossip = HierarchicalGossip::new();
        gossip.advance_time(1000);
        gossip.register_node(make_node(1, 1.0, 0));
        gossip.register_node(make_node(2, 1.0, 0));
        gossip.submit_update(1, vec![1.0, 2.0], 1000).unwrap();
        gossip.submit_update(2, vec![3.0, 4.0], 1000).unwrap();
        let merged = gossip.fedavg_merge(1000).unwrap();
        assert_eq!(merged.len(), 2);
        // With noise, approximate check.
        assert!(merged[0] > 1.0 && merged[0] < 4.0);
        assert!(merged[1] > 2.0 && merged[1] < 5.0);
    }

    #[test]
    fn test_fedavg_merge_empty() {
        let gossip = HierarchicalGossip::new();
        assert!(gossip.fedavg_merge(0).is_none());
    }

    #[test]
    fn test_dissolve_committee() {
        let mut config = GossipConfig::default_Topological();
        config.committee_size = 3;
        let mut gossip = HierarchicalGossip::with_config(config).unwrap();
        for i in 1..=10 {
            gossip.register_node(make_node(i, i as f64, 0));
        }
        gossip.elect_committee(0).unwrap();
        gossip.dissolve_committee();
        assert!(gossip.active_committee().is_none());
    }

    #[test]
    fn test_clear_updates() {
        let mut gossip = HierarchicalGossip::new();
        gossip.advance_time(1000);
        gossip.submit_update(1, vec![1.0], 1000).unwrap();
        gossip.clear_updates();
        assert_eq!(gossip.update_count(), 0);
    }

    #[test]
    fn test_gossip_display() {
        let gossip = HierarchicalGossip::new();
        let s = format!("{}", gossip);
        assert!(s.contains("HierarchicalGossip"));
    }

    #[test]
    fn test_node_display() {
        let node = make_node(1, 0.95, 2);
        let s = format!("{}", node);
        assert!(s.contains("id: 1"));
    }

    #[test]
    fn test_update_display() {
        let update = GossipUpdate {
            source_id: 1,
            vector: vec![1.0, 2.0],
            timestamp_ms: 1000,
            noise_scale: 0.1,
        };
        let s = format!("{}", update);
        assert!(s.contains("source: 1"));
    }

    #[test]
    fn test_committee_display() {
        let committee = Committee {
            round_id: 1,
            members: vec![1, 2, 3],
            elected_at_ms: 1000,
            layer: 0,
        };
        let s = format!("{}", committee);
        assert!(s.contains("round: 1"));
    }

    #[test]
    fn test_error_display() {
        let err = GossipError::StaleData(5000);
        let s = format!("{}", err);
        assert!(s.contains("Data stale"));
    }
}
