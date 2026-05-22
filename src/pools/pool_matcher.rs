//! Pool Matcher — Request-to-shard matching engine using weighted scoring.
//!
//! Matches incoming pool requests to the optimal shards based on
//! reputation-weighted scoring, latency, and available capacity.
//! Similar to Linux's `cpuset` affinity but distributed across federation.
//!
//! Zero financial logic: matching is purely technical (compute resources).

use std::collections::HashMap;

/// Errors for pool matching operations.
#[derive(Debug)]
pub enum MatcherError {
    NoSuitableShard(String),
    ShardUnavailable(String),
    InvalidRequest(String),
    ScoringFailed(String),
    PoolEmpty,
}

impl std::fmt::Display for MatcherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatcherError::NoSuitableShard(reason) => {
                write!(f, "No suitable shard found: {}", reason)
            }
            MatcherError::ShardUnavailable(id) => {
                write!(f, "Shard unavailable: {}", id)
            }
            MatcherError::InvalidRequest(msg) => {
                write!(f, "Invalid request: {}", msg)
            }
            MatcherError::ScoringFailed(msg) => {
                write!(f, "Scoring failed: {}", msg)
            }
            MatcherError::PoolEmpty => {
                write!(f, "Pool is empty")
            }
        }
    }
}

/// Configuration for the pool matcher.
#[derive(Debug, Clone)]
pub struct MatcherConfig {
    /// Weight for reputation in scoring (0.0–1.0).
    pub reputation_weight: f64,
    /// Weight for latency in scoring (0.0–1.0).
    pub latency_weight: f64,
    /// Weight for capacity in scoring (0.0–1.0).
    pub capacity_weight: f64,
    /// Minimum score to accept a match (0.0–1.0).
    pub min_match_score: f64,
    /// Maximum candidates to consider per request.
    pub max_candidates: usize,
    /// Enable score caching for repeated requests.
    pub enable_caching: bool,
}

impl Default for MatcherConfig {
    fn default() -> Self {
        Self {
            reputation_weight: 0.4,
            latency_weight: 0.35,
            capacity_weight: 0.25,
            min_match_score: 0.1,
            max_candidates: 16,
            enable_caching: true,
        }
    }
}

/// Resource request type from the pool consumer.
#[derive(Debug, Clone)]
pub enum RequestType {
    Compute {
        /// Required compute credits.
        credits: f64,
        /// Maximum acceptable latency (ms).
        max_latency_ms: f64,
    },
    Storage {
        /// Required storage credits.
        credits: f64,
        /// Durability level (0 = minimal, 1 = maximum).
        durability: f64,
    },
    Inference {
        /// Required inference credits.
        credits: f64,
        /// Model size class (small, medium, large).
        model_size: u32,
    },
    Custom {
        /// Custom resource requirements.
        credits: f64,
        /// Metadata for custom matching.
        metadata: HashMap<String, String>,
    },
}

impl std::fmt::Display for RequestType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestType::Compute {
                credits,
                max_latency_ms,
            } => {
                write!(
                    f,
                    "Compute(credits={}, max_latency={}ms)",
                    credits, max_latency_ms
                )
            }
            RequestType::Storage {
                credits,
                durability,
            } => {
                write!(f, "Storage(credits={}, durability={})", credits, durability)
            }
            RequestType::Inference {
                credits,
                model_size,
            } => {
                write!(
                    f,
                    "Inference(credits={}, model_size={})",
                    credits, model_size
                )
            }
            RequestType::Custom { credits, .. } => {
                write!(f, "Custom(credits={})", credits)
            }
        }
    }
}

/// Incoming pool request.
#[derive(Debug, Clone)]
pub struct PoolRequest {
    /// Unique request identifier.
    pub request_id: String,
    /// Requesting node identifier.
    pub requester_id: String,
    /// Resource request type.
    pub request_type: RequestType,
    /// Priority level (0 = lowest, 10 = highest).
    pub priority: u32,
    /// Timestamp of request creation (ms).
    pub created_at_ms: u64,
}

impl PoolRequest {
    /// Create a new pool request.
    pub fn new(
        request_id: String,
        requester_id: String,
        request_type: RequestType,
        priority: u32,
    ) -> Self {
        Self {
            request_id,
            requester_id,
            request_type,
            priority: priority.clamp(0, 10),
            created_at_ms: current_timestamp_ms(),
        }
    }

    /// Get required credits from request type.
    pub fn required_credits(&self) -> f64 {
        match &self.request_type {
            RequestType::Compute { credits, .. } => *credits,
            RequestType::Storage { credits, .. } => *credits,
            RequestType::Inference { credits, .. } => *credits,
            RequestType::Custom { credits, .. } => *credits,
        }
    }
}

/// Candidate shard for matching.
#[derive(Debug, Clone)]
pub struct ShardCandidate {
    /// Shard identifier.
    pub shard_id: String,
    /// Available compute credits.
    pub available_credits: f64,
    /// Technical reputation score (0.0–1.0).
    pub reputation: f64,
    /// Average latency (ms).
    pub avg_latency_ms: f64,
    /// Current load factor (0.0–1.0).
    pub load_factor: f64,
    /// Shard is healthy and accepting work.
    pub healthy: bool,
}

impl ShardCandidate {
    /// Create a new shard candidate.
    pub fn new(
        shard_id: String,
        available_credits: f64,
        reputation: f64,
        avg_latency_ms: f64,
        load_factor: f64,
    ) -> Self {
        Self {
            shard_id,
            available_credits,
            reputation: reputation.clamp(0.0, 1.0),
            avg_latency_ms: avg_latency_ms.max(0.0),
            load_factor: load_factor.clamp(0.0, 1.0),
            healthy: true,
        }
    }

    /// Check if shard can fulfill the credit requirement.
    pub fn can_fulfill(&self, required_credits: f64) -> bool {
        self.healthy && self.available_credits >= required_credits
    }
}

/// Match result with scoring details.
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// Request ID that was matched.
    pub request_id: String,
    /// Selected shard ID.
    pub matched_shard_id: String,
    /// Final composite score (0.0–1.0).
    pub score: f64,
    /// Reputation component of the score.
    pub reputation_score: f64,
    /// Latency component of the score.
    pub latency_score: f64,
    /// Capacity component of the score.
    pub capacity_score: f64,
    /// Match timestamp (ms).
    pub matched_at_ms: u64,
    /// Number of candidates evaluated.
    pub candidates_evaluated: usize,
}

impl MatchResult {
    /// Create a new match result.
    pub fn new(
        request_id: String,
        matched_shard_id: String,
        score: f64,
        reputation_score: f64,
        latency_score: f64,
        capacity_score: f64,
        candidates_evaluated: usize,
    ) -> Self {
        Self {
            request_id,
            matched_shard_id,
            score: score.clamp(0.0, 1.0),
            reputation_score,
            latency_score,
            capacity_score,
            matched_at_ms: current_timestamp_ms(),
            candidates_evaluated,
        }
    }
}

/// Matching statistics.
#[derive(Debug, Clone)]
pub struct MatcherStats {
    /// Total requests processed.
    pub total_requests: usize,
    /// Total successful matches.
    pub successful_matches: usize,
    /// Total failed matches.
    pub failed_matches: usize,
    /// Average match score.
    pub avg_match_score: f64,
    /// Average match time (ms).
    pub avg_match_time_ms: f64,
    /// Last match timestamp (ms).
    pub last_match_ms: u64,
}

impl Default for MatcherStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_matches: 0,
            failed_matches: 0,
            avg_match_score: 0.0,
            avg_match_time_ms: 0.0,
            last_match_ms: 0,
        }
    }
}

/// Pool Matcher engine — matches requests to optimal shards using weighted scoring.
pub struct PoolMatcher {
    /// Matcher configuration.
    pub config: MatcherConfig,
    /// Registered shard candidates.
    candidates: HashMap<String, ShardCandidate>,
    /// Matching statistics.
    stats: MatcherStats,
    /// Score cache for repeated request patterns.
    score_cache: HashMap<String, Vec<(String, f64)>>,
}

impl PoolMatcher {
    /// Create a new matcher with config.
    pub fn new(config: MatcherConfig) -> Self {
        Self {
            config,
            candidates: HashMap::new(),
            stats: MatcherStats::default(),
            score_cache: HashMap::new(),
        }
    }

    /// Create matcher with default config.
    pub fn with_defaults() -> Self {
        Self::new(MatcherConfig::default())
    }

    /// Register a shard candidate.
    pub fn register_candidate(&mut self, candidate: ShardCandidate) {
        self.candidates
            .insert(candidate.shard_id.clone(), candidate);
    }

    /// Remove a shard candidate.
    pub fn remove_candidate(&mut self, shard_id: &str) -> Option<ShardCandidate> {
        self.candidates.remove(shard_id)
    }

    /// Get a shard candidate by ID.
    pub fn get_candidate(&self, shard_id: &str) -> Option<&ShardCandidate> {
        self.candidates.get(shard_id)
    }

    /// Update candidate metrics.
    pub fn update_candidate(
        &mut self,
        shard_id: &str,
        available_credits: f64,
        reputation: f64,
        avg_latency_ms: f64,
        load_factor: f64,
    ) -> Result<(), MatcherError> {
        let candidate = self
            .candidates
            .get_mut(shard_id)
            .ok_or(MatcherError::ShardUnavailable(shard_id.to_string()))?;
        candidate.available_credits = available_credits;
        candidate.reputation = reputation.clamp(0.0, 1.0);
        candidate.avg_latency_ms = avg_latency_ms.max(0.0);
        candidate.load_factor = load_factor.clamp(0.0, 1.0);
        Ok(())
    }

    /// Match a request to the best available shard.
    pub fn match_request(&mut self, request: &PoolRequest) -> Result<MatchResult, MatcherError> {
        let start_ms = current_timestamp_ms();

        // Filter eligible candidates
        let required = request.required_credits();
        let eligible: Vec<&ShardCandidate> = self
            .candidates
            .values()
            .filter(|c| c.can_fulfill(required))
            .collect();

        if eligible.is_empty() {
            self.stats.total_requests += 1;
            self.stats.failed_matches += 1;
            return Err(MatcherError::NoSuitableShard(format!(
                "No shard with {} credits",
                required
            )));
        }

        // Limit candidates
        let limited: Vec<&ShardCandidate> = if self.config.max_candidates > 0 {
            let mut sorted: Vec<&ShardCandidate> = eligible;
            sorted.sort_by(|a, b| b.reputation.partial_cmp(&a.reputation).unwrap());
            sorted
                .into_iter()
                .take(self.config.max_candidates)
                .collect()
        } else {
            eligible
        };

        // Score each candidate
        let mut scored: Vec<(String, f64, f64, f64, f64)> = Vec::new();
        for candidate in &limited {
            let rep_score = self.score_reputation(candidate);
            let lat_score = self.score_latency(candidate, request);
            let cap_score = self.score_capacity(candidate, request);
            let composite = rep_score * self.config.reputation_weight
                + lat_score * self.config.latency_weight
                + cap_score * self.config.capacity_weight;
            scored.push((
                candidate.shard_id.clone(),
                composite,
                rep_score,
                lat_score,
                cap_score,
            ));
        }

        // Sort by composite score descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let best = scored.first().ok_or(MatcherError::PoolEmpty)?;

        // Check minimum score threshold
        if best.1 < self.config.min_match_score {
            self.stats.total_requests += 1;
            self.stats.failed_matches += 1;
            return Err(MatcherError::NoSuitableShard(format!(
                "Best score {} below minimum {}",
                best.1, self.config.min_match_score
            )));
        }

        let elapsed_ms = current_timestamp_ms().saturating_sub(start_ms);
        let result = MatchResult::new(
            request.request_id.clone(),
            best.0.clone(),
            best.1,
            best.2,
            best.3,
            best.4,
            scored.len(),
        );

        // Update stats
        self.stats.total_requests += 1;
        self.stats.successful_matches += 1;
        self.stats.avg_match_score =
            (self.stats.avg_match_score * (self.stats.successful_matches - 1) as f64 + best.1)
                / self.stats.successful_matches as f64;
        self.stats.avg_match_time_ms = (self.stats.avg_match_time_ms
            * (self.stats.successful_matches - 1) as f64
            + elapsed_ms as f64)
            / self.stats.successful_matches as f64;
        self.stats.last_match_ms = current_timestamp_ms();

        Ok(result)
    }

    /// Score reputation component (0.0–1.0).
    fn score_reputation(&self, candidate: &ShardCandidate) -> f64 {
        candidate.reputation
    }

    /// Score latency component (0.0–1.0).
    /// Lower latency = higher score.
    fn score_latency(&self, candidate: &ShardCandidate, _request: &PoolRequest) -> f64 {
        // Normalize: 0ms = 1.0, 1000ms = 0.0
        let normalized = 1.0 - (candidate.avg_latency_ms / 1000.0);
        normalized.clamp(0.0, 1.0)
    }

    /// Score capacity component (0.0–1.0).
    /// Higher available capacity = higher score.
    fn score_capacity(&self, candidate: &ShardCandidate, request: &PoolRequest) -> f64 {
        let required = request.required_credits();
        if candidate.available_credits == 0.0 {
            return 0.0;
        }
        // Ratio of available to required (capped at 1.0)
        let ratio = required / candidate.available_credits;
        (1.0 - ratio).clamp(0.0, 1.0)
    }

    /// Get all candidates.
    pub fn get_all_candidates(&self) -> Vec<&ShardCandidate> {
        self.candidates.values().collect()
    }

    /// Get current statistics.
    pub fn get_stats(&self) -> MatcherStats {
        self.stats.clone()
    }

    /// Reset statistics.
    pub fn reset_stats(&mut self) {
        self.stats = MatcherStats::default();
    }

    /// Clear score cache.
    pub fn clear_cache(&mut self) {
        self.score_cache.clear();
    }
}

impl Default for PoolMatcher {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Helper to get current timestamp in milliseconds.
fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_candidate(
        id: &str,
        credits: f64,
        reputation: f64,
        latency: f64,
        load: f64,
    ) -> ShardCandidate {
        ShardCandidate::new(id.to_string(), credits, reputation, latency, load)
    }

    fn make_request(id: &str, requester: &str, credits: f64, priority: u32) -> PoolRequest {
        PoolRequest::new(
            id.to_string(),
            requester.to_string(),
            RequestType::Compute {
                credits,
                max_latency_ms: 500.0,
            },
            priority,
        )
    }

    #[test]
    fn test_matcher_creation() {
        let matcher = PoolMatcher::with_defaults();
        assert_eq!(matcher.get_stats().total_requests, 0);
        assert_eq!(matcher.get_all_candidates().len(), 0);
    }

    #[test]
    fn test_register_candidate() {
        let mut matcher = PoolMatcher::with_defaults();
        let c = make_candidate("s1", 100.0, 0.9, 50.0, 0.3);
        matcher.register_candidate(c);
        assert_eq!(matcher.get_all_candidates().len(), 1);
        assert!(matcher.get_candidate("s1").is_some());
    }

    #[test]
    fn test_remove_candidate() {
        let mut matcher = PoolMatcher::with_defaults();
        let c = make_candidate("s1", 100.0, 0.9, 50.0, 0.3);
        matcher.register_candidate(c);
        let removed = matcher.remove_candidate("s1").unwrap();
        assert_eq!(removed.shard_id, "s1");
        assert!(matcher.get_candidate("s1").is_none());
    }

    #[test]
    fn test_update_candidate() {
        let mut matcher = PoolMatcher::with_defaults();
        let c = make_candidate("s1", 100.0, 0.9, 50.0, 0.3);
        matcher.register_candidate(c);
        assert!(matcher
            .update_candidate("s1", 200.0, 0.95, 30.0, 0.2)
            .is_ok());
        let updated = matcher.get_candidate("s1").unwrap();
        assert_eq!(updated.available_credits, 200.0);
        assert_eq!(updated.reputation, 0.95);
    }

    #[test]
    fn test_match_request_basic() {
        let mut matcher = PoolMatcher::with_defaults();
        let c = make_candidate("s1", 100.0, 0.9, 50.0, 0.3);
        matcher.register_candidate(c);
        let req = make_request("r1", "node1", 50.0, 5);
        let result = matcher.match_request(&req).unwrap();
        assert_eq!(result.matched_shard_id, "s1");
        assert!(result.score >= matcher.config.min_match_score);
    }

    #[test]
    fn test_match_request_no_candidates() {
        let mut matcher = PoolMatcher::with_defaults();
        let req = make_request("r1", "node1", 50.0, 5);
        match matcher.match_request(&req) {
            Err(MatcherError::NoSuitableShard(_)) => {}
            _ => panic!("Expected NoSuitableShard"),
        }
    }

    #[test]
    fn test_match_request_insufficient_credits() {
        let mut matcher = PoolMatcher::with_defaults();
        let c = make_candidate("s1", 30.0, 0.9, 50.0, 0.3);
        matcher.register_candidate(c);
        let req = make_request("r1", "node1", 50.0, 5);
        match matcher.match_request(&req) {
            Err(MatcherError::NoSuitableShard(_)) => {}
            _ => panic!("Expected NoSuitableShard"),
        }
    }

    #[test]
    fn test_match_selects_best_score() {
        let mut matcher = PoolMatcher::with_defaults();
        // High reputation, low latency
        let c1 = make_candidate("best", 100.0, 0.95, 20.0, 0.2);
        // Low reputation, high latency
        let c2 = make_candidate("worst", 100.0, 0.5, 200.0, 0.8);
        matcher.register_candidate(c1);
        matcher.register_candidate(c2);
        let req = make_request("r1", "node1", 50.0, 5);
        let result = matcher.match_request(&req).unwrap();
        assert_eq!(result.matched_shard_id, "best");
    }

    #[test]
    fn test_match_reputation_weight() {
        let mut matcher = PoolMatcher::with_defaults();
        // High reputation, slightly worse latency
        let c1 = make_candidate("rep", 100.0, 0.99, 100.0, 0.3);
        // Lower reputation, better latency
        let c2 = make_candidate("lat", 100.0, 0.7, 10.0, 0.3);
        matcher.register_candidate(c1);
        matcher.register_candidate(c2);
        let req = make_request("r1", "node1", 50.0, 5);
        let result = matcher.match_request(&req).unwrap();
        // With reputation_weight=0.4, reputation should dominate
        assert_eq!(result.matched_shard_id, "rep");
    }

    #[test]
    fn test_score_reputation() {
        let matcher = PoolMatcher::with_defaults();
        let c = make_candidate("s1", 100.0, 0.8, 50.0, 0.3);
        assert_eq!(matcher.score_reputation(&c), 0.8);
    }

    #[test]
    fn test_score_latency() {
        let matcher = PoolMatcher::with_defaults();
        let req = make_request("r1", "node1", 50.0, 5);
        // 0ms latency = 1.0
        let c0 = make_candidate("s0", 100.0, 0.9, 0.0, 0.3);
        assert_eq!(matcher.score_latency(&c0, &req), 1.0);
        // 1000ms latency = 0.0
        let c1000 = make_candidate("s1000", 100.0, 0.9, 1000.0, 0.3);
        assert_eq!(matcher.score_latency(&c1000, &req), 0.0);
        // 500ms latency = 0.5
        let c500 = make_candidate("s500", 100.0, 0.9, 500.0, 0.3);
        assert_eq!(matcher.score_latency(&c500, &req), 0.5);
    }

    #[test]
    fn test_score_capacity() {
        let matcher = PoolMatcher::with_defaults();
        let req = make_request("r1", "node1", 50.0, 5);
        // 100 credits available, 50 required = 0.5
        let c = make_candidate("s1", 100.0, 0.9, 50.0, 0.3);
        assert_eq!(matcher.score_capacity(&c, &req), 0.5);
    }

    #[test]
    fn test_stats_tracking() {
        let mut matcher = PoolMatcher::with_defaults();
        let c = make_candidate("s1", 100.0, 0.9, 50.0, 0.3);
        matcher.register_candidate(c);
        let req = make_request("r1", "node1", 50.0, 5);
        matcher.match_request(&req).unwrap();
        let stats = matcher.get_stats();
        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.successful_matches, 1);
        assert_eq!(stats.failed_matches, 0);
    }

    #[test]
    fn test_reset_stats() {
        let mut matcher = PoolMatcher::with_defaults();
        let c = make_candidate("s1", 100.0, 0.9, 50.0, 0.3);
        matcher.register_candidate(c);
        let req = make_request("r1", "node1", 50.0, 5);
        matcher.match_request(&req).unwrap();
        matcher.reset_stats();
        let stats = matcher.get_stats();
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.successful_matches, 0);
    }

    #[test]
    fn test_candidate_can_fulfill() {
        let c = make_candidate("s1", 100.0, 0.9, 50.0, 0.3);
        assert!(c.can_fulfill(50.0));
        assert!(!c.can_fulfill(150.0));
    }

    #[test]
    fn test_unhealthy_cannot_fulfill() {
        let mut c = make_candidate("s1", 100.0, 0.9, 50.0, 0.3);
        c.healthy = false;
        assert!(!c.can_fulfill(50.0));
    }

    #[test]
    fn test_request_required_credits() {
        let req = make_request("r1", "node1", 75.0, 5);
        assert_eq!(req.required_credits(), 75.0);
    }

    #[test]
    fn test_priority_clamping() {
        let req = PoolRequest::new(
            "r1".to_string(),
            "node1".to_string(),
            RequestType::Compute {
                credits: 50.0,
                max_latency_ms: 500.0,
            },
            15, // Above max
        );
        assert_eq!(req.priority, 10);
    }

    #[test]
    fn test_config_default() {
        let config = MatcherConfig::default();
        assert_eq!(config.reputation_weight, 0.4);
        assert_eq!(config.latency_weight, 0.35);
        assert_eq!(config.capacity_weight, 0.25);
        assert_eq!(config.min_match_score, 0.1);
        assert_eq!(config.max_candidates, 16);
        assert!(config.enable_caching);
    }

    #[test]
    fn test_stats_default() {
        let stats = MatcherStats::default();
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.successful_matches, 0);
        assert_eq!(stats.failed_matches, 0);
    }

    #[test]
    fn test_matcher_default() {
        let matcher = PoolMatcher::default();
        assert_eq!(matcher.get_stats().total_requests, 0);
    }

    #[test]
    fn test_request_type_display() {
        let req = make_request("r1", "node1", 50.0, 5);
        let display = req.request_type.to_string();
        assert!(display.contains("Compute"));
    }

    #[test]
    fn test_error_display() {
        match MatcherError::PoolEmpty {
            e => assert!(!e.to_string().is_empty()),
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_match_result_score_clamping() {
        let result = MatchResult::new(
            "r1".to_string(),
            "s1".to_string(),
            1.5, // Above max
            0.9,
            0.8,
            0.7,
            5,
        );
        assert_eq!(result.score, 1.0);
    }

    #[test]
    fn test_candidate_reputation_clamping() {
        let c = ShardCandidate::new("s1".to_string(), 100.0, 1.5, 50.0, 0.3);
        assert_eq!(c.reputation, 1.0);
    }

    #[test]
    fn test_candidate_load_clamping() {
        let c = ShardCandidate::new("s1".to_string(), 100.0, 0.9, 50.0, 1.5);
        assert_eq!(c.load_factor, 1.0);
    }

    #[test]
    fn test_max_candidates_limit() {
        let mut matcher = PoolMatcher::with_defaults();
        matcher.config.max_candidates = 3;
        for i in 0..10 {
            let c = make_candidate(&format!("s{}", i), 100.0, 0.9, 50.0, 0.3);
            matcher.register_candidate(c);
        }
        let req = make_request("r1", "node1", 50.0, 5);
        let result = matcher.match_request(&req).unwrap();
        assert!(result.candidates_evaluated <= 3);
    }

    #[test]
    fn test_clear_cache() {
        let mut matcher = PoolMatcher::with_defaults();
        matcher
            .score_cache
            .insert("key".to_string(), vec![("s1".to_string(), 0.9)]);
        matcher.clear_cache();
        assert!(matcher.score_cache.is_empty());
    }

    #[test]
    fn test_storage_request() {
        let mut matcher = PoolMatcher::with_defaults();
        let c = make_candidate("s1", 100.0, 0.9, 50.0, 0.3);
        matcher.register_candidate(c);
        let req = PoolRequest::new(
            "r1".to_string(),
            "node1".to_string(),
            RequestType::Storage {
                credits: 50.0,
                durability: 0.8,
            },
            5,
        );
        let result = matcher.match_request(&req).unwrap();
        assert_eq!(result.matched_shard_id, "s1");
    }

    #[test]
    fn test_inference_request() {
        let mut matcher = PoolMatcher::with_defaults();
        let c = make_candidate("s1", 100.0, 0.9, 50.0, 0.3);
        matcher.register_candidate(c);
        let req = PoolRequest::new(
            "r1".to_string(),
            "node1".to_string(),
            RequestType::Inference {
                credits: 50.0,
                model_size: 2,
            },
            5,
        );
        let result = matcher.match_request(&req).unwrap();
        assert_eq!(result.matched_shard_id, "s1");
    }

    #[test]
    fn test_custom_request() {
        let mut matcher = PoolMatcher::with_defaults();
        let c = make_candidate("s1", 100.0, 0.9, 50.0, 0.3);
        matcher.register_candidate(c);
        let mut metadata = HashMap::new();
        metadata.insert("key".to_string(), "value".to_string());
        let req = PoolRequest::new(
            "r1".to_string(),
            "node1".to_string(),
            RequestType::Custom {
                credits: 50.0,
                metadata,
            },
            5,
        );
        let result = matcher.match_request(&req).unwrap();
        assert_eq!(result.matched_shard_id, "s1");
    }
}
