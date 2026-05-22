//! Reputation Matcher — Matching ponderado por reputación criptográfica
//!
//! Algoritmo de matching que pondera las ofertas del marketplace
//! usando reputación criptográfica, historial de trades y scores SLO.
//! Integra con cross_chain_settlement para liquidación verificada.
//!
//! Feature-gated: `#[cfg(feature = "v1.2-sprint4")]`

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use thiserror::Error;
use tracing::info;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors for reputation matcher.
#[derive(Debug, Error)]
pub enum ReputationError {
    #[error("Node not found: {0}")]
    NodeNotFound(String),
    #[error("Insufficient reputation: {score:.3} < {threshold:.3}")]
    InsufficientReputation { score: f64, threshold: f64 },
    #[error("No candidates available")]
    NoCandidates,
    #[error("Reputation score invalid: {0}")]
    InvalidScore(String),
    #[error("Sybil detection: {0}")]
    SybilDetected(String),
    #[error("Matching pool empty")]
    PoolEmpty,
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Cryptographic reputation profile for a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationProfile {
    /// Node identifier.
    pub node_id: String,
    /// Cryptographic reputation score (0.0 - 1.0).
    pub reputation_score: f64,
    /// Total completed trades.
    pub total_trades: usize,
    /// Successful trades.
    pub successful_trades: usize,
    /// Failed trades.
    pub failed_trades: usize,
    /// Average SLO compliance rate (0.0 - 1.0).
    pub slo_compliance: f64,
    /// Average response latency (ms).
    pub avg_latency_ms: f64,
    /// Chain diversity score (number of chains participated).
    pub chain_diversity: usize,
    /// Last activity timestamp (ms).
    pub last_activity_ms: u64,
    /// Reputation hash for integrity verification.
    pub reputation_hash: String,
}

impl ReputationProfile {
    /// Creates a new reputation profile.
    pub fn new(node_id: String, reputation_score: f64) -> Self {
        let now = current_timestamp_ms();
        let reputation_hash = compute_reputation_hash(&node_id, reputation_score, now);
        Self {
            node_id,
            reputation_score,
            total_trades: 0,
            successful_trades: 0,
            failed_trades: 0,
            slo_compliance: 1.0,
            avg_latency_ms: 0.0,
            chain_diversity: 1,
            last_activity_ms: now,
            reputation_hash,
        }
    }

    /// Records a successful trade.
    pub fn record_success(&mut self, slo_met: bool, latency_ms: f64) {
        self.total_trades += 1;
        self.successful_trades += 1;
        self.last_activity_ms = current_timestamp_ms();

        // Update SLO compliance (exponential moving average)
        let alpha = 0.3;
        let slo_value = if slo_met { 1.0 } else { 0.0 };
        self.slo_compliance = alpha * slo_value + (1.0 - alpha) * self.slo_compliance;

        // Update average latency (exponential moving average)
        self.avg_latency_ms = alpha * latency_ms + (1.0 - alpha) * self.avg_latency_ms;

        // Adjust reputation
        let adjustment = if slo_met { 0.02 } else { -0.01 };
        self.reputation_score = (self.reputation_score + adjustment).clamp(0.0, 1.0);

        // Update hash
        self.reputation_hash =
            compute_reputation_hash(&self.node_id, self.reputation_score, self.last_activity_ms);
    }

    /// Records a failed trade.
    pub fn record_failure(&mut self) {
        self.total_trades += 1;
        self.failed_trades += 1;
        self.last_activity_ms = current_timestamp_ms();

        // Penalize reputation
        self.reputation_score = (self.reputation_score - 0.05).clamp(0.0, 1.0);

        // Update hash
        self.reputation_hash =
            compute_reputation_hash(&self.node_id, self.reputation_score, self.last_activity_ms);
    }

    /// Adds chain diversity.
    pub fn add_chain(&mut self) {
        self.chain_diversity += 1;
        self.last_activity_ms = current_timestamp_ms();
        self.reputation_hash =
            compute_reputation_hash(&self.node_id, self.reputation_score, self.last_activity_ms);
    }

    /// Computes the weighted matching score.
    /// Higher is better.
    pub fn matching_score(&self, weights: &MatchingWeights) -> f64 {
        let trade_success_rate = if self.total_trades > 0 {
            self.successful_trades as f64 / self.total_trades as f64
        } else {
            0.5 // Neutral for new nodes
        };

        let latency_factor = if self.avg_latency_ms > 0.0 {
            (1.0 / (1.0 + self.avg_latency_ms / 1000.0)).min(1.0)
        } else {
            1.0
        };

        let diversity_bonus = (self.chain_diversity as f64 / 10.0).min(0.1);

        weights.reputation * self.reputation_score
            + weights.trade_success * trade_success_rate
            + weights.slo_compliance * self.slo_compliance
            + weights.latency * latency_factor
            + weights.diversity * diversity_bonus
    }

    /// Verifies the reputation hash integrity.
    pub fn verify_hash(&self) -> bool {
        let expected =
            compute_reputation_hash(&self.node_id, self.reputation_score, self.last_activity_ms);
        self.reputation_hash == expected
    }

    /// Checks if the profile is stale.
    pub fn is_stale(&self, max_stale_ms: u64) -> bool {
        let now = current_timestamp_ms();
        now.saturating_sub(self.last_activity_ms) > max_stale_ms
    }
}

/// Weights for the matching score calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingWeights {
    /// Weight for reputation score.
    pub reputation: f64,
    /// Weight for trade success rate.
    pub trade_success: f64,
    /// Weight for SLO compliance.
    pub slo_compliance: f64,
    /// Weight for latency factor.
    pub latency: f64,
    /// Weight for chain diversity.
    pub diversity: f64,
}

impl Default for MatchingWeights {
    fn default() -> Self {
        Self {
            reputation: 0.35,
            trade_success: 0.25,
            slo_compliance: 0.20,
            latency: 0.15,
            diversity: 0.05,
        }
    }
}

/// Result of a reputation-based match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchCandidate {
    /// Selected node ID.
    pub node_id: String,
    /// Matching score.
    pub score: f64,
    /// Reputation score.
    pub reputation: f64,
    /// SLO compliance rate.
    pub slo_compliance: f64,
    /// Estimated latency (ms).
    pub estimated_latency_ms: f64,
    /// Rank among candidates.
    pub rank: usize,
}

/// Result of a matching operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingResult {
    /// Selected candidates in order.
    pub candidates: Vec<MatchCandidate>,
    /// Best candidate (if any).
    pub best_candidate: Option<MatchCandidate>,
    /// Total candidates evaluated.
    pub total_evaluated: usize,
    /// Matching timestamp (ms).
    pub matched_at_ms: u64,
}

/// Statistics for the reputation matcher.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatcherStats {
    /// Total profiles registered.
    pub total_profiles: usize,
    /// Total matches performed.
    pub total_matches: usize,
    /// Average matching time (ms).
    pub avg_match_time_ms: f64,
    /// Sybil detections.
    pub sybil_detections: usize,
    /// Average reputation score.
    pub avg_reputation: f64,
}

impl Default for MatcherStats {
    fn default() -> Self {
        Self {
            total_profiles: 0,
            total_matches: 0,
            avg_match_time_ms: 0.0,
            sybil_detections: 0,
            avg_reputation: 0.5,
        }
    }
}

/// Configuration for reputation matcher.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationConfig {
    /// Minimum reputation to participate.
    pub min_reputation: f64,
    /// Maximum stale time (ms).
    pub max_stale_ms: u64,
    /// Sybil detection: max profiles per IP hash.
    pub max_profiles_per_ip: usize,
    /// Matching weights.
    pub weights: MatchingWeights,
    /// Top N candidates to return.
    pub top_n_candidates: usize,
}

impl Default for ReputationConfig {
    fn default() -> Self {
        Self {
            min_reputation: 0.2,
            max_stale_ms: 86_400_000, // 24 hours
            max_profiles_per_ip: 3,
            weights: MatchingWeights::default(),
            top_n_candidates: 5,
        }
    }
}

// ---------------------------------------------------------------------------
// ReputationMatcher Engine
// ---------------------------------------------------------------------------

/// Reputation-based matcher for marketplace nodes.
pub struct ReputationMatcher {
    config: ReputationConfig,
    profiles: BTreeMap<String, ReputationProfile>,
    stats: MatcherStats,
    /// IP hash -> node IDs for sybil detection
    ip_registry: HashMap<String, Vec<String>>,
    /// Match history for analytics
    match_history: Vec<MatchingResult>,
}

impl ReputationMatcher {
    /// Creates a new matcher with default config.
    pub fn new() -> Self {
        Self::with_config(ReputationConfig::default())
    }

    /// Creates a matcher with custom config.
    pub fn with_config(config: ReputationConfig) -> Self {
        Self {
            config,
            profiles: BTreeMap::new(),
            stats: MatcherStats::default(),
            ip_registry: HashMap::new(),
            match_history: Vec::new(),
        }
    }

    /// Registers a node profile.
    pub fn register_profile(&mut self, profile: ReputationProfile) -> Result<(), ReputationError> {
        if profile.reputation_score < 0.0 || profile.reputation_score > 1.0 {
            return Err(ReputationError::InvalidScore(format!(
                "Score {} out of range [0.0, 1.0]",
                profile.reputation_score
            )));
        }

        self.profiles.insert(profile.node_id.clone(), profile);
        self.stats.total_profiles = self.profiles.len();
        self.update_avg_reputation();
        info!(
            "ReputationMatcher: registered {}",
            self.stats.total_profiles
        );
        Ok(())
    }

    /// Registers a node with IP hash for sybil detection.
    pub fn register_with_ip(
        &mut self,
        profile: ReputationProfile,
        ip_hash: String,
    ) -> Result<(), ReputationError> {
        // Check sybil
        if let Some(existing) = self.ip_registry.get(&ip_hash) {
            if existing.len() >= self.config.max_profiles_per_ip {
                self.stats.sybil_detections += 1;
                return Err(ReputationError::SybilDetected(format!(
                    "IP {} has {} profiles (max {})",
                    ip_hash,
                    existing.len(),
                    self.config.max_profiles_per_ip
                )));
            }
        }

        self.ip_registry
            .entry(ip_hash)
            .or_default()
            .push(profile.node_id.clone());
        self.register_profile(profile)
    }

    /// Updates a profile's reputation score.
    pub fn update_reputation(&mut self, node_id: &str, score: f64) -> Result<(), ReputationError> {
        let profile = self
            .profiles
            .get_mut(node_id)
            .ok_or(ReputationError::NodeNotFound(node_id.to_string()))?;

        profile.reputation_score = score.clamp(0.0, 1.0);
        profile.last_activity_ms = current_timestamp_ms();
        profile.reputation_hash =
            compute_reputation_hash(node_id, profile.reputation_score, profile.last_activity_ms);
        self.update_avg_reputation();
        Ok(())
    }

    /// Records a successful trade for a node.
    pub fn record_trade_success(
        &mut self,
        node_id: &str,
        slo_met: bool,
        latency_ms: f64,
    ) -> Result<(), ReputationError> {
        let profile = self
            .profiles
            .get_mut(node_id)
            .ok_or(ReputationError::NodeNotFound(node_id.to_string()))?;
        profile.record_success(slo_met, latency_ms);
        self.update_avg_reputation();
        Ok(())
    }

    /// Records a failed trade for a node.
    pub fn record_trade_failure(&mut self, node_id: &str) -> Result<(), ReputationError> {
        let profile = self
            .profiles
            .get_mut(node_id)
            .ok_or(ReputationError::NodeNotFound(node_id.to_string()))?;
        profile.record_failure();
        self.update_avg_reputation();
        Ok(())
    }

    /// Performs reputation-based matching.
    /// Returns the best candidates sorted by score.
    pub fn match_nodes(
        &mut self,
        min_reputation: Option<f64>,
    ) -> Result<MatchingResult, ReputationError> {
        let start = std::time::Instant::now();
        let threshold = min_reputation.unwrap_or(self.config.min_reputation);

        // Filter eligible candidates
        let mut eligible: Vec<&ReputationProfile> = self
            .profiles
            .values()
            .filter(|p| p.reputation_score >= threshold)
            .filter(|p| !p.is_stale(self.config.max_stale_ms))
            .filter(|p| p.verify_hash())
            .collect();

        if eligible.is_empty() {
            return Err(ReputationError::NoCandidates);
        }

        // Score and sort
        eligible.sort_by(|a, b| {
            let score_a = a.matching_score(&self.config.weights);
            let score_b = b.matching_score(&self.config.weights);
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Build candidates
        let top_n = self.config.top_n_candidates.min(eligible.len());
        let mut candidates = Vec::with_capacity(top_n);

        for (i, profile) in eligible.iter().take(top_n).enumerate() {
            let score = profile.matching_score(&self.config.weights);
            candidates.push(MatchCandidate {
                node_id: profile.node_id.clone(),
                score,
                reputation: profile.reputation_score,
                slo_compliance: profile.slo_compliance,
                estimated_latency_ms: profile.avg_latency_ms,
                rank: i + 1,
            });
        }

        // Extract data before dropping borrow
        let total_evaluated = eligible.len();
        drop(eligible);

        let elapsed_ms = start.elapsed().as_micros() as f64 / 1000.0;
        self.update_match_time(elapsed_ms);

        let result = MatchingResult {
            best_candidate: candidates.first().cloned(),
            candidates,
            total_evaluated,
            matched_at_ms: current_timestamp_ms(),
        };

        self.stats.total_matches += 1;
        self.match_history.push(result.clone());

        if let Some(best) = &result.best_candidate {
            info!(
                "ReputationMatcher: best match {} (score={:.4})",
                best.node_id, best.score
            );
        }

        Ok(result)
    }

    /// Gets a profile.
    pub fn get_profile(&self, node_id: &str) -> Option<&ReputationProfile> {
        self.profiles.get(node_id)
    }

    /// Gets matcher statistics.
    pub fn get_stats(&self) -> MatcherStats {
        self.stats.clone()
    }

    /// Gets all profiles.
    pub fn get_all_profiles(&self) -> Vec<&ReputationProfile> {
        self.profiles.values().collect()
    }

    /// Removes stale profiles.
    pub fn remove_stale(&mut self) -> usize {
        let before = self.profiles.len();
        self.profiles
            .retain(|_, p| !p.is_stale(self.config.max_stale_ms));
        let removed = before - self.profiles.len();
        self.stats.total_profiles = self.profiles.len();
        if removed > 0 {
            info!("ReputationMatcher: removed {} stale profiles", removed);
        }
        removed
    }

    fn update_avg_reputation(&mut self) {
        if !self.profiles.is_empty() {
            let sum: f64 = self.profiles.values().map(|p| p.reputation_score).sum();
            self.stats.avg_reputation = sum / self.profiles.len() as f64;
        }
    }

    fn update_match_time(&mut self, elapsed_ms: f64) {
        let alpha = 0.1;
        self.stats.avg_match_time_ms =
            alpha * elapsed_ms + (1.0 - alpha) * self.stats.avg_match_time_ms;
    }
}

impl Default for ReputationMatcher {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/// Computes reputation hash for integrity verification.
pub fn compute_reputation_hash(node_id: &str, score: f64, timestamp_ms: u64) -> String {
    let mut hasher = Sha256::new();
    hasher.update(node_id.as_bytes());
    hasher.update(score.to_le_bytes());
    hasher.update(timestamp_ms.to_le_bytes());
    let result = hasher.finalize();
    format!("0x{}", hex::encode(result))
}

/// Returns current timestamp in milliseconds.
fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_profile(id: &str, score: f64) -> ReputationProfile {
        ReputationProfile::new(id.to_string(), score)
    }

    #[test]
    fn test_matcher_creation() {
        let matcher = ReputationMatcher::new();
        assert_eq!(matcher.get_stats().total_profiles, 0);
    }

    #[test]
    fn test_register_profile() {
        let mut matcher = ReputationMatcher::new();
        let profile = make_profile("node-1", 0.8);
        matcher.register_profile(profile).unwrap();
        assert_eq!(matcher.get_stats().total_profiles, 1);
    }

    #[test]
    fn test_register_invalid_score() {
        let mut matcher = ReputationMatcher::new();
        let mut profile = make_profile("node-1", 0.5);
        profile.reputation_score = 1.5;
        let result = matcher.register_profile(profile);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_reputation() {
        let mut matcher = ReputationMatcher::new();
        matcher
            .register_profile(make_profile("node-1", 0.5))
            .unwrap();
        matcher.update_reputation("node-1", 0.9).unwrap();
        assert_eq!(matcher.get_profile("node-1").unwrap().reputation_score, 0.9);
    }

    #[test]
    fn test_record_trade_success() {
        let mut matcher = ReputationMatcher::new();
        matcher
            .register_profile(make_profile("node-1", 0.5))
            .unwrap();
        matcher.record_trade_success("node-1", true, 50.0).unwrap();

        let profile = matcher.get_profile("node-1").unwrap();
        assert_eq!(profile.total_trades, 1);
        assert_eq!(profile.successful_trades, 1);
        assert!(profile.reputation_score > 0.5);
    }

    #[test]
    fn test_record_trade_failure() {
        let mut matcher = ReputationMatcher::new();
        matcher
            .register_profile(make_profile("node-1", 0.5))
            .unwrap();
        matcher.record_trade_failure("node-1").unwrap();

        let profile = matcher.get_profile("node-1").unwrap();
        assert_eq!(profile.total_trades, 1);
        assert_eq!(profile.failed_trades, 1);
        assert!(profile.reputation_score < 0.5);
    }

    #[test]
    fn test_match_nodes() {
        let mut matcher = ReputationMatcher::new();
        matcher
            .register_profile(make_profile("high", 0.95))
            .unwrap();
        matcher.register_profile(make_profile("mid", 0.7)).unwrap();
        matcher.register_profile(make_profile("low", 0.3)).unwrap();

        let result = matcher.match_nodes(None).unwrap();
        assert_eq!(result.best_candidate.as_ref().unwrap().node_id, "high");
        assert!(result.candidates.len() >= 2);
    }

    #[test]
    fn test_match_with_min_reputation() {
        let mut matcher = ReputationMatcher::new();
        matcher
            .register_profile(make_profile("high", 0.95))
            .unwrap();
        matcher.register_profile(make_profile("low", 0.3)).unwrap();

        let result = matcher.match_nodes(Some(0.5)).unwrap();
        assert_eq!(result.candidates.len(), 1);
        assert_eq!(result.candidates[0].node_id, "high");
    }

    #[test]
    fn test_match_no_candidates() {
        let mut matcher = ReputationMatcher::new();
        let result = matcher.match_nodes(None);
        assert!(result.is_err());
    }

    #[test]
    fn test_sybil_detection() {
        let config = ReputationConfig {
            max_profiles_per_ip: 2,
            ..Default::default()
        };
        let mut matcher = ReputationMatcher::with_config(config);

        matcher
            .register_with_ip(make_profile("n1", 0.8), "ip-hash".to_string())
            .unwrap();
        matcher
            .register_with_ip(make_profile("n2", 0.8), "ip-hash".to_string())
            .unwrap();

        let result = matcher.register_with_ip(make_profile("n3", 0.8), "ip-hash".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_reputation_hash_verification() {
        let profile = make_profile("node-1", 0.8);
        assert!(profile.verify_hash());
    }

    #[test]
    fn test_reputation_hash_tampering() {
        let mut profile = make_profile("node-1", 0.8);
        profile.reputation_hash = "0xtampered".to_string();
        assert!(!profile.verify_hash());
    }

    #[test]
    fn test_profile_stale_detection() {
        let mut profile = make_profile("node-1", 0.8);
        profile.last_activity_ms = 0; // Very old
        assert!(profile.is_stale(1_000_000));
    }

    #[test]
    fn test_profile_not_stale() {
        let profile = make_profile("node-1", 0.8);
        assert!(!profile.is_stale(86400_000));
    }

    #[test]
    fn test_matching_score_calculation() {
        let profile = make_profile("node-1", 0.9);
        let weights = MatchingWeights::default();
        let score = profile.matching_score(&weights);
        assert!(score > 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_stats_tracking() {
        let mut matcher = ReputationMatcher::new();
        matcher.register_profile(make_profile("n1", 0.8)).unwrap();
        matcher.register_profile(make_profile("n2", 0.9)).unwrap();
        matcher.match_nodes(None).unwrap();

        let stats = matcher.get_stats();
        assert_eq!(stats.total_profiles, 2);
        assert_eq!(stats.total_matches, 1);
    }

    #[test]
    fn test_remove_stale() {
        let mut matcher = ReputationMatcher::new();
        matcher
            .register_profile(make_profile("fresh", 0.8))
            .unwrap();

        let mut stale = make_profile("stale", 0.5);
        stale.last_activity_ms = 0;
        matcher.register_profile(stale).unwrap();

        let removed = matcher.remove_stale();
        assert_eq!(removed, 1);
        assert_eq!(matcher.get_stats().total_profiles, 1);
    }

    #[test]
    fn test_add_chain_diversity() {
        let mut profile = make_profile("node-1", 0.8);
        assert_eq!(profile.chain_diversity, 1);
        profile.add_chain();
        assert_eq!(profile.chain_diversity, 2);
    }

    #[test]
    fn test_reputation_clamping() {
        let mut matcher = ReputationMatcher::new();
        matcher.register_profile(make_profile("n1", 0.5)).unwrap();
        matcher.update_reputation("n1", 1.5).unwrap();
        assert_eq!(matcher.get_profile("n1").unwrap().reputation_score, 1.0);

        matcher.update_reputation("n1", -0.5).unwrap();
        assert_eq!(matcher.get_profile("n1").unwrap().reputation_score, 0.0);
    }

    #[test]
    fn test_config_default() {
        let config = ReputationConfig::default();
        assert_eq!(config.min_reputation, 0.2);
        assert_eq!(config.top_n_candidates, 5);
    }

    #[test]
    fn test_weights_default() {
        let weights = MatchingWeights::default();
        let total = weights.reputation
            + weights.trade_success
            + weights.slo_compliance
            + weights.latency
            + weights.diversity;
        assert!((total - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_matcher_default() {
        let matcher = ReputationMatcher::default();
        assert_eq!(matcher.get_stats().total_profiles, 0);
    }

    #[test]
    fn test_get_nonexistent_profile() {
        let matcher = ReputationMatcher::new();
        assert!(matcher.get_profile("ghost").is_none());
    }

    #[test]
    fn test_error_display() {
        let err = ReputationError::NodeNotFound("x".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("x"));
    }

    #[test]
    fn test_candidate_ranking() {
        let mut matcher = ReputationMatcher::new();
        matcher.register_profile(make_profile("a", 0.99)).unwrap();
        matcher.register_profile(make_profile("b", 0.88)).unwrap();
        matcher.register_profile(make_profile("c", 0.77)).unwrap();

        let result = matcher.match_nodes(None).unwrap();
        assert_eq!(result.candidates[0].rank, 1);
        assert_eq!(result.candidates[1].rank, 2);
        assert_eq!(result.candidates[2].rank, 3);
    }

    #[test]
    fn test_multiple_trades_improve_reputation() {
        let mut matcher = ReputationMatcher::new();
        matcher.register_profile(make_profile("n1", 0.5)).unwrap();

        for _ in 0..10 {
            matcher.record_trade_success("n1", true, 30.0).unwrap();
        }

        let profile = matcher.get_profile("n1").unwrap();
        assert!(profile.reputation_score > 0.5);
        assert_eq!(profile.total_trades, 10);
    }

    #[test]
    fn test_hash_consistency() {
        let h1 = compute_reputation_hash("n1", 0.8, 1000);
        let h2 = compute_reputation_hash("n1", 0.8, 1000);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_uniqueness() {
        let h1 = compute_reputation_hash("n1", 0.8, 1000);
        let h2 = compute_reputation_hash("n2", 0.8, 1000);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_all_profiles() {
        let mut matcher = ReputationMatcher::new();
        matcher.register_profile(make_profile("n1", 0.8)).unwrap();
        matcher.register_profile(make_profile("n2", 0.9)).unwrap();
        assert_eq!(matcher.get_all_profiles().len(), 2);
    }
}
