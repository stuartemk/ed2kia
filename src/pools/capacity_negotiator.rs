//! Capacity Negotiator — Negotiates compute credit allocation between pools.
//!
//! Implements capacity negotiation protocols for cross-chain pool resource sharing,
//! using offer/request cycles with reputation-weighted priority and expiration windows.
//!
//! **Design:** Linux `cgroups`-inspired resource negotiation for distributed pools.
//!
//! **Key features:**
//! - Offer/request negotiation cycles
//! - Reputation-weighted priority queuing
//! - Expiration-based cleanup
//! - SAE shard-aware capacity tracking
//!
//! **References:**
//! - `cross_chain_pools_v3.rs` — Pool data structures and credit allocation
//! - `pool_matcher.rs` — Priority scoring patterns
//!
//! Apache License 2.0 + Ethical Use Clause

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, VecDeque};

// ─── Error ───────────────────────────────────────────────────────────────────

/// Errors for capacity negotiation.
#[derive(Debug, Clone, PartialEq)]
pub enum NegotiatorError {
    /// Pool not found.
    PoolNotFound(String),
    /// Negotiation ID not found.
    NegotiationNotFound(String),
    /// Insufficient capacity to fulfill request.
    InsufficientCapacity,
    /// Negotiation already completed.
    AlreadyCompleted(String),
    /// Invalid configuration.
    InvalidConfig(String),
    /// Negotiation queue is full.
    QueueFull,
    /// Offer expired.
    OfferExpired,
}

impl std::fmt::Display for NegotiatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NegotiatorError::PoolNotFound(id) => write!(f, "Pool not found: {}", id),
            NegotiatorError::NegotiationNotFound(id) => write!(f, "Negotiation not found: {}", id),
            NegotiatorError::InsufficientCapacity => write!(f, "Insufficient capacity"),
            NegotiatorError::AlreadyCompleted(id) => {
                write!(f, "Negotiation already completed: {}", id)
            }
            NegotiatorError::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
            NegotiatorError::QueueFull => write!(f, "Negotiation queue full"),
            NegotiatorError::OfferExpired => write!(f, "Offer expired"),
        }
    }
}

// ─── Negotiation Status ──────────────────────────────────────────────────────

/// Status of a negotiation.
#[derive(Debug, Clone, PartialEq)]
pub enum NegotiationStatus {
    /// Negotiation is pending acceptance.
    Pending,
    /// Negotiation has been accepted.
    Accepted,
    /// Negotiation has been completed.
    Completed,
    /// Negotiation has been rejected.
    Rejected,
    /// Negotiation has expired.
    Expired,
}

impl std::fmt::Display for NegotiationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NegotiationStatus::Pending => write!(f, "Pending"),
            NegotiationStatus::Accepted => write!(f, "Accepted"),
            NegotiationStatus::Completed => write!(f, "Completed"),
            NegotiationStatus::Rejected => write!(f, "Rejected"),
            NegotiationStatus::Expired => write!(f, "Expired"),
        }
    }
}

// ─── Config ──────────────────────────────────────────────────────────────────

/// Configuration for the capacity negotiator.
#[derive(Debug, Clone)]
pub struct CapacityNegotiatorConfig {
    /// Maximum pending negotiations.
    pub max_pending_negotiations: usize,
    /// Default negotiation window in milliseconds.
    pub default_negotiation_window_ms: u64,
    /// Minimum capacity per negotiation.
    pub min_capacity: f64,
    /// Maximum capacity per negotiation.
    pub max_capacity: f64,
    /// Reputation threshold for negotiation eligibility.
    pub min_reputation: f64,
    /// Priority weight for reputation.
    pub reputation_priority_weight: f64,
    /// Priority weight for urgency (time remaining).
    pub urgency_priority_weight: f64,
    /// Enable automatic negotiation cleanup.
    pub auto_cleanup: bool,
}

impl Default for CapacityNegotiatorConfig {
    fn default() -> Self {
        Self {
            max_pending_negotiations: 1024,
            default_negotiation_window_ms: 30000,
            min_capacity: 1.0,
            max_capacity: 10000.0,
            min_reputation: 0.3,
            reputation_priority_weight: 0.6,
            urgency_priority_weight: 0.4,
            auto_cleanup: true,
        }
    }
}

// ─── Pool Capacity ───────────────────────────────────────────────────────────

/// Pool capacity information for negotiation.
#[derive(Debug, Clone)]
pub struct PoolCapacity {
    /// Pool identifier.
    pub pool_id: String,
    /// Total available capacity.
    pub total_capacity: f64,
    /// Currently allocated capacity.
    pub allocated_capacity: f64,
    /// Reputation score.
    pub reputation: f64,
    /// Average latency.
    pub avg_latency_ms: f64,
    /// Last update timestamp.
    pub last_update_ms: u64,
}

impl PoolCapacity {
    pub fn new(pool_id: String, total_capacity: f64, reputation: f64) -> Self {
        Self {
            pool_id,
            total_capacity,
            allocated_capacity: 0.0,
            reputation,
            avg_latency_ms: 0.0,
            last_update_ms: current_timestamp_ms(),
        }
    }

    /// Available unallocated capacity.
    pub fn available(&self) -> f64 {
        (self.total_capacity - self.allocated_capacity).max(0.0)
    }

    /// Check if pool can offer the requested capacity.
    pub fn can_offer(&self, requested: f64) -> bool {
        self.available() >= requested
    }
}

// ─── Negotiation Request ─────────────────────────────────────────────────────

/// A capacity negotiation request.
#[derive(Debug, Clone)]
pub struct NegotiationRequest {
    /// Unique negotiation ID.
    pub negotiation_id: String,
    /// Requesting pool ID.
    pub requester_pool: String,
    /// Target pool ID.
    pub target_pool: String,
    /// Requested capacity credits.
    pub requested_credits: f64,
    /// Negotiation window in milliseconds.
    pub window_ms: u64,
    /// Creation timestamp.
    pub created_ms: u64,
    /// Expiration timestamp.
    pub expires_ms: u64,
    /// Current status.
    pub status: NegotiationStatus,
    /// Priority score (higher = more urgent).
    pub priority: f64,
    /// Accepted offer (if any).
    pub accepted_offer: Option<f64>,
}

impl NegotiationRequest {
    pub fn new(
        negotiation_id: String,
        requester_pool: String,
        target_pool: String,
        requested_credits: f64,
        window_ms: u64,
        created_ms: u64,
    ) -> Self {
        let expires_ms = created_ms + window_ms;
        Self {
            negotiation_id,
            requester_pool,
            target_pool,
            requested_credits,
            window_ms,
            created_ms,
            expires_ms,
            status: NegotiationStatus::Pending,
            priority: 0.0,
            accepted_offer: None,
        }
    }

    /// Check if the negotiation has expired.
    pub fn is_expired(&self, now_ms: u64) -> bool {
        now_ms >= self.expires_ms
    }

    /// Remaining time in milliseconds.
    pub fn time_remaining(&self, now_ms: u64) -> u64 {
        self.expires_ms.saturating_sub(now_ms)
    }

    /// Urgency ratio (1.0 = just created, 0.0 = about to expire).
    pub fn urgency(&self, now_ms: u64) -> f64 {
        let remaining = self.time_remaining(now_ms) as f64;
        let total = self.window_ms as f64;
        if total == 0.0 {
            return 0.0;
        }
        (remaining / total).clamp(0.0, 1.0)
    }
}

// ─── Negotiation Offer ───────────────────────────────────────────────────────

/// An offer from a target pool.
#[derive(Debug, Clone)]
pub struct NegotiationOffer {
    /// Offer ID.
    pub offer_id: String,
    /// Negotiation ID this offer belongs to.
    pub negotiation_id: String,
    /// Offering pool ID.
    pub offering_pool: String,
    /// Offered capacity credits.
    pub offered_credits: f64,
    /// Offer timestamp.
    pub created_ms: u64,
    /// Offer expiration.
    pub expires_ms: u64,
}

impl NegotiationOffer {
    pub fn new(
        offer_id: String,
        negotiation_id: String,
        offering_pool: String,
        offered_credits: f64,
        window_ms: u64,
        created_ms: u64,
    ) -> Self {
        Self {
            offer_id,
            negotiation_id,
            offering_pool,
            offered_credits,
            created_ms,
            expires_ms: created_ms + window_ms,
        }
    }

    pub fn is_expired(&self, now_ms: u64) -> bool {
        now_ms >= self.expires_ms
    }
}

// ─── Priority Queue Item ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct NegotiationPriorityItem {
    negotiation_id: String,
    priority: f64,
}

impl PartialEq for NegotiationPriorityItem {
    fn eq(&self, other: &Self) -> bool {
        self.negotiation_id == other.negotiation_id && self.priority == other.priority
    }
}

impl Eq for NegotiationPriorityItem {}

impl Ord for NegotiationPriorityItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .partial_cmp(&other.priority)
            .unwrap_or(Ordering::Equal)
            .then_with(|| other.negotiation_id.cmp(&self.negotiation_id))
    }
}

impl PartialOrd for NegotiationPriorityItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// ─── Stats ───────────────────────────────────────────────────────────────────

/// Statistics for capacity negotiation.
#[derive(Debug, Clone)]
pub struct NegotiatorStats {
    /// Total negotiations created.
    pub total_negotiations: usize,
    /// Total negotiations accepted.
    pub total_accepted: usize,
    /// Total negotiations completed.
    pub total_completed: usize,
    /// Total negotiations rejected.
    pub total_rejected: usize,
    /// Total negotiations expired.
    pub total_expired: usize,
    /// Total capacity negotiated.
    pub total_capacity_negotiated: f64,
    /// Current pending negotiations.
    pub pending_negotiations: usize,
    /// Average negotiation time in ms.
    pub avg_negotiation_time_ms: f64,
}

impl Default for NegotiatorStats {
    fn default() -> Self {
        Self {
            total_negotiations: 0,
            total_accepted: 0,
            total_completed: 0,
            total_rejected: 0,
            total_expired: 0,
            total_capacity_negotiated: 0.0,
            pending_negotiations: 0,
            avg_negotiation_time_ms: 0.0,
        }
    }
}

// ─── Main Negotiator ─────────────────────────────────────────────────────────

/// Capacity negotiation engine for cross-chain pools.
pub struct CapacityNegotiator {
    config: CapacityNegotiatorConfig,
    pools: HashMap<String, PoolCapacity>,
    negotiations: HashMap<String, NegotiationRequest>,
    offers: HashMap<String, Vec<NegotiationOffer>>,
    priority_queue: BinaryHeap<NegotiationPriorityItem>,
    completed_history: VecDeque<NegotiationRequest>,
    stats: NegotiatorStats,
    current_time_ms: u64,
    negotiation_counter: u64,
    offer_counter: u64,
    total_negotiation_time_ms: f64,
}

impl CapacityNegotiator {
    // ─── Construction ──────────────────────────────────────────────────────

    /// Create a new capacity negotiator.
    pub fn new(config: CapacityNegotiatorConfig) -> Self {
        Self {
            config,
            pools: HashMap::new(),
            negotiations: HashMap::new(),
            offers: HashMap::new(),
            priority_queue: BinaryHeap::new(),
            completed_history: VecDeque::with_capacity(100),
            stats: NegotiatorStats::default(),
            current_time_ms: current_timestamp_ms(),
            negotiation_counter: 0,
            offer_counter: 0,
            total_negotiation_time_ms: 0.0,
        }
    }

    /// Create with default configuration.
    pub fn default_config() -> Self {
        Self::new(CapacityNegotiatorConfig::default())
    }

    // ─── Pool Management ───────────────────────────────────────────────────

    /// Register a pool with capacity information.
    pub fn register_pool(&mut self, pool: PoolCapacity) -> Result<(), NegotiatorError> {
        if pool.reputation < self.config.min_reputation {
            return Err(NegotiatorError::InvalidConfig(format!(
                "Pool {} reputation {:.3} below minimum {:.3}",
                pool.pool_id, pool.reputation, self.config.min_reputation
            )));
        }

        self.pools.insert(pool.pool_id.clone(), pool);
        Ok(())
    }

    /// Update pool capacity.
    pub fn update_pool_capacity(
        &mut self,
        pool_id: &str,
        total: f64,
        allocated: f64,
    ) -> Result<(), NegotiatorError> {
        let pool = self
            .pools
            .get_mut(pool_id)
            .ok_or_else(|| NegotiatorError::PoolNotFound(pool_id.to_string()))?;

        pool.total_capacity = total;
        pool.allocated_capacity = allocated;
        pool.last_update_ms = self.current_time_ms;
        Ok(())
    }

    /// Update pool reputation.
    pub fn update_pool_reputation(
        &mut self,
        pool_id: &str,
        reputation: f64,
    ) -> Result<(), NegotiatorError> {
        let pool = self
            .pools
            .get_mut(pool_id)
            .ok_or_else(|| NegotiatorError::PoolNotFound(pool_id.to_string()))?;

        pool.reputation = reputation.clamp(0.0, 1.0);
        pool.last_update_ms = self.current_time_ms;
        Ok(())
    }

    /// Get pool capacity info.
    pub fn get_pool(&self, pool_id: &str) -> Option<&PoolCapacity> {
        self.pools.get(pool_id)
    }

    // ─── Negotiation Creation ──────────────────────────────────────────────

    /// Create a new capacity negotiation request.
    pub fn create_negotiation(
        &mut self,
        requester_pool: String,
        target_pool: String,
        requested_credits: f64,
    ) -> Result<String, NegotiatorError> {
        // Validate pools exist
        if !self.pools.contains_key(&requester_pool) {
            return Err(NegotiatorError::PoolNotFound(requester_pool.clone()));
        }
        if !self.pools.contains_key(&target_pool) {
            return Err(NegotiatorError::PoolNotFound(target_pool.clone()));
        }

        // Validate capacity range
        if requested_credits < self.config.min_capacity {
            return Err(NegotiatorError::InvalidConfig(format!(
                "Requested credits {} below minimum {}",
                requested_credits, self.config.min_capacity
            )));
        }
        if requested_credits > self.config.max_capacity {
            return Err(NegotiatorError::InvalidConfig(format!(
                "Requested credits {} above maximum {}",
                requested_credits, self.config.max_capacity
            )));
        }

        // Check queue limit
        let pending = self
            .negotiations
            .values()
            .filter(|n| n.status == NegotiationStatus::Pending)
            .count();
        if pending >= self.config.max_pending_negotiations {
            return Err(NegotiatorError::QueueFull);
        }

        // Create negotiation
        self.negotiation_counter += 1;
        let negotiation_id = format!("neg-{}", self.negotiation_counter);

        let negotiation = NegotiationRequest::new(
            negotiation_id.clone(),
            requester_pool,
            target_pool,
            requested_credits,
            self.config.default_negotiation_window_ms,
            self.current_time_ms,
        );

        // Calculate priority
        let priority = self.calculate_priority(&negotiation);
        let mut negotiation = negotiation;
        negotiation.priority = priority;

        self.negotiations
            .insert(negotiation_id.clone(), negotiation);
        self.priority_queue.push(NegotiationPriorityItem {
            negotiation_id: negotiation_id.clone(),
            priority,
        });

        self.stats.total_negotiations += 1;
        self.stats.pending_negotiations += 1;

        Ok(negotiation_id)
    }

    // ─── Offers ────────────────────────────────────────────────────────────

    /// Submit an offer for a negotiation.
    pub fn submit_offer(
        &mut self,
        negotiation_id: String,
        offering_pool: String,
        offered_credits: f64,
    ) -> Result<String, NegotiatorError> {
        // Validate negotiation exists and is pending
        let negotiation = self
            .negotiations
            .get(&negotiation_id)
            .ok_or_else(|| NegotiatorError::NegotiationNotFound(negotiation_id.clone()))?;

        if negotiation.status != NegotiationStatus::Pending {
            return Err(NegotiatorError::AlreadyCompleted(negotiation_id.clone()));
        }

        if negotiation.is_expired(self.current_time_ms) {
            return Err(NegotiatorError::OfferExpired);
        }

        // Validate pool can offer
        let pool = self
            .pools
            .get(&offering_pool)
            .ok_or_else(|| NegotiatorError::PoolNotFound(offering_pool.clone()))?;

        if !pool.can_offer(offered_credits) {
            return Err(NegotiatorError::InsufficientCapacity);
        }

        // Create offer
        self.offer_counter += 1;
        let offer_id = format!("offer-{}", self.offer_counter);

        let offer = NegotiationOffer::new(
            offer_id.clone(),
            negotiation_id.clone(),
            offering_pool,
            offered_credits,
            self.config.default_negotiation_window_ms,
            self.current_time_ms,
        );

        self.offers
            .entry(negotiation_id)
            .or_insert_with(Vec::new)
            .push(offer);

        Ok(offer_id)
    }

    /// Get offers for a negotiation.
    pub fn get_offers(&self, negotiation_id: &str) -> Option<&Vec<NegotiationOffer>> {
        self.offers.get(negotiation_id)
    }

    // ─── Negotiation Actions ───────────────────────────────────────────────

    /// Accept an offer and complete the negotiation.
    pub fn accept_offer(
        &mut self,
        negotiation_id: &str,
        offered_credits: f64,
    ) -> Result<(), NegotiatorError> {
        let negotiation = self
            .negotiations
            .get(negotiation_id)
            .ok_or_else(|| NegotiatorError::NegotiationNotFound(negotiation_id.to_string()))?;

        if negotiation.status != NegotiationStatus::Pending {
            return Err(NegotiatorError::AlreadyCompleted(
                negotiation_id.to_string(),
            ));
        }

        let negotiation = self.negotiations.get_mut(negotiation_id).unwrap();
        negotiation.status = NegotiationStatus::Accepted;
        negotiation.accepted_offer = Some(offered_credits);

        self.stats.pending_negotiations -= 1;
        self.stats.total_accepted += 1;

        Ok(())
    }

    /// Complete a negotiation (finalize capacity transfer).
    pub fn complete_negotiation(&mut self, negotiation_id: &str) -> Result<(), NegotiatorError> {
        let negotiation = self
            .negotiations
            .get(negotiation_id)
            .ok_or_else(|| NegotiatorError::NegotiationNotFound(negotiation_id.to_string()))?;

        if negotiation.status != NegotiationStatus::Accepted {
            return Err(NegotiatorError::AlreadyCompleted(
                negotiation_id.to_string(),
            ));
        }

        let negotiation = self.negotiations.get_mut(negotiation_id).unwrap();
        negotiation.status = NegotiationStatus::Completed;

        // Update stats
        if let Some(credits) = negotiation.accepted_offer {
            self.stats.total_capacity_negotiated += credits;
        }

        let negotiation_time = self.current_time_ms - negotiation.created_ms;
        self.total_negotiation_time_ms += negotiation_time as f64;
        self.stats.avg_negotiation_time_ms =
            self.total_negotiation_time_ms / self.stats.total_completed.max(1) as f64;

        self.stats.total_completed += 1;

        // Move to history
        self.completed_history.push_back(negotiation.clone());
        if self.completed_history.len() > 100 {
            self.completed_history.pop_front();
        }

        Ok(())
    }

    /// Reject a negotiation.
    pub fn reject_negotiation(&mut self, negotiation_id: &str) -> Result<(), NegotiatorError> {
        let negotiation = self
            .negotiations
            .get(negotiation_id)
            .ok_or_else(|| NegotiatorError::NegotiationNotFound(negotiation_id.to_string()))?;

        if negotiation.status != NegotiationStatus::Pending {
            return Err(NegotiatorError::AlreadyCompleted(
                negotiation_id.to_string(),
            ));
        }

        let negotiation = self.negotiations.get_mut(negotiation_id).unwrap();
        negotiation.status = NegotiationStatus::Rejected;

        self.stats.pending_negotiations -= 1;
        self.stats.total_rejected += 1;

        Ok(())
    }

    // ─── Queries ───────────────────────────────────────────────────────────

    /// Get a negotiation by ID.
    pub fn get_negotiation(&self, negotiation_id: &str) -> Option<&NegotiationRequest> {
        self.negotiations.get(negotiation_id)
    }

    /// Get pending negotiations sorted by priority.
    pub fn pending_negotiations(&self) -> Vec<&NegotiationRequest> {
        let mut pending: Vec<&NegotiationRequest> = self
            .negotiations
            .values()
            .filter(|n| n.status == NegotiationStatus::Pending)
            .collect();
        pending.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(Ordering::Equal)
        });
        pending
    }

    /// Get the highest priority pending negotiation.
    pub fn next_negotiation(&self) -> Option<&NegotiationRequest> {
        self.pending_negotiations().first().copied()
    }

    /// Get completed negotiation history.
    pub fn completed_history(&self) -> &VecDeque<NegotiationRequest> {
        &self.completed_history
    }

    /// Find best pool for a capacity request.
    pub fn find_best_pool(&self, requested_credits: f64) -> Option<&PoolCapacity> {
        self.pools
            .values()
            .filter(|p| p.can_offer(requested_credits))
            .min_by_key(|p| (p.avg_latency_ms * 1000.0) as u64)
    }

    // ─── Cleanup ───────────────────────────────────────────────────────────

    /// Cleanup expired negotiations.
    pub fn cleanup_expired(&mut self) -> usize {
        let mut expired = 0;

        self.negotiations.retain(|_id, neg| {
            if neg.status == NegotiationStatus::Pending && neg.is_expired(self.current_time_ms) {
                neg.status = NegotiationStatus::Expired;
                self.stats.pending_negotiations -= 1;
                self.stats.total_expired += 1;
                expired += 1;
                false
            } else {
                true
            }
        });

        // Clean expired offers
        for offers in self.offers.values_mut() {
            offers.retain(|o| !o.is_expired(self.current_time_ms));
        }

        expired
    }

    /// Run automatic cleanup if enabled.
    pub fn auto_cleanup(&mut self) -> usize {
        if self.config.auto_cleanup {
            self.cleanup_expired()
        } else {
            0
        }
    }

    // ─── Time ──────────────────────────────────────────────────────────────

    /// Advance internal time.
    pub fn advance_time(&mut self, ms: u64) {
        self.current_time_ms += ms;
    }

    // ─── Stats ─────────────────────────────────────────────────────────────

    /// Get current statistics.
    pub fn stats(&self) -> NegotiatorStats {
        NegotiatorStats {
            total_negotiations: self.stats.total_negotiations,
            total_accepted: self.stats.total_accepted,
            total_completed: self.stats.total_completed,
            total_rejected: self.stats.total_rejected,
            total_expired: self.stats.total_expired,
            total_capacity_negotiated: self.stats.total_capacity_negotiated,
            pending_negotiations: self
                .negotiations
                .values()
                .filter(|n| n.status == NegotiationStatus::Pending)
                .count(),
            avg_negotiation_time_ms: self.stats.avg_negotiation_time_ms,
        }
    }

    /// Reset statistics.
    pub fn reset_stats(&mut self) {
        self.stats = NegotiatorStats::default();
        self.total_negotiation_time_ms = 0.0;
    }

    // ─── Internal ──────────────────────────────────────────────────────────

    fn calculate_priority(&self, negotiation: &NegotiationRequest) -> f64 {
        // Get requester reputation
        let requester_rep = self
            .pools
            .get(&negotiation.requester_pool)
            .map(|p| p.reputation)
            .unwrap_or(0.0);

        // Urgency based on window size (smaller window = more urgent)
        let urgency = 1.0 - ((negotiation.window_ms as f64) / 60000.0).clamp(0.0, 1.0);

        self.config.reputation_priority_weight * requester_rep
            + self.config.urgency_priority_weight * urgency
    }
}

impl Default for CapacityNegotiator {
    fn default() -> Self {
        Self::new(CapacityNegotiatorConfig::default())
    }
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pool(id: &str, capacity: f64, reputation: f64) -> PoolCapacity {
        PoolCapacity::new(id.to_string(), capacity, reputation)
    }

    #[test]
    fn test_negotiator_creation() {
        let neg = CapacityNegotiator::new(CapacityNegotiatorConfig::default());
        assert_eq!(neg.stats().total_negotiations, 0);
    }

    #[test]
    fn test_register_pool() {
        let mut neg = CapacityNegotiator::default_config();
        let pool = make_pool("pool-1", 1000.0, 0.8);
        assert!(neg.register_pool(pool).is_ok());
        assert!(neg.get_pool("pool-1").is_some());
    }

    #[test]
    fn test_register_pool_low_reputation() {
        let mut neg = CapacityNegotiator::default_config();
        let pool = make_pool("pool-1", 1000.0, 0.1);
        assert!(neg.register_pool(pool).is_err());
    }

    #[test]
    fn test_update_pool_capacity() {
        let mut neg = CapacityNegotiator::default_config();
        neg.register_pool(make_pool("pool-1", 1000.0, 0.8)).unwrap();
        assert!(neg.update_pool_capacity("pool-1", 2000.0, 500.0).is_ok());
        let pool = neg.get_pool("pool-1").unwrap();
        assert_eq!(pool.total_capacity, 2000.0);
        assert_eq!(pool.allocated_capacity, 500.0);
    }

    #[test]
    fn test_update_pool_reputation() {
        let mut neg = CapacityNegotiator::default_config();
        neg.register_pool(make_pool("pool-1", 1000.0, 0.8)).unwrap();
        assert!(neg.update_pool_reputation("pool-1", 0.95).is_ok());
        assert_eq!(neg.get_pool("pool-1").unwrap().reputation, 0.95);
    }

    #[test]
    fn test_pool_available() {
        let mut pool = make_pool("pool-1", 1000.0, 0.8);
        pool.allocated_capacity = 300.0;
        assert_eq!(pool.available(), 700.0);
    }

    #[test]
    fn test_pool_can_offer() {
        let pool = make_pool("pool-1", 1000.0, 0.8);
        assert!(pool.can_offer(500.0));
        assert!(!pool.can_offer(1500.0));
    }

    #[test]
    fn test_create_negotiation() {
        let mut neg = CapacityNegotiator::default_config();
        neg.register_pool(make_pool("pool-1", 1000.0, 0.8)).unwrap();
        neg.register_pool(make_pool("pool-2", 2000.0, 0.9)).unwrap();
        let id = neg.create_negotiation("pool-1".to_string(), "pool-2".to_string(), 100.0);
        assert!(id.is_ok());
        assert_eq!(neg.stats().total_negotiations, 1);
    }

    #[test]
    fn test_create_negotiation_pool_not_found() {
        let mut neg = CapacityNegotiator::default_config();
        let result = neg.create_negotiation("unknown".to_string(), "pool-2".to_string(), 100.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_negotiation_below_min() {
        let mut neg = CapacityNegotiator::default_config();
        neg.register_pool(make_pool("pool-1", 1000.0, 0.8)).unwrap();
        neg.register_pool(make_pool("pool-2", 2000.0, 0.9)).unwrap();
        let result = neg.create_negotiation("pool-1".to_string(), "pool-2".to_string(), 0.5);
        assert!(result.is_err());
    }

    #[test]
    fn test_submit_offer() {
        let mut neg = CapacityNegotiator::default_config();
        neg.register_pool(make_pool("pool-1", 1000.0, 0.8)).unwrap();
        neg.register_pool(make_pool("pool-2", 2000.0, 0.9)).unwrap();
        let neg_id = neg
            .create_negotiation("pool-1".to_string(), "pool-2".to_string(), 100.0)
            .unwrap();
        let offer_id = neg.submit_offer(neg_id.clone(), "pool-2".to_string(), 100.0);
        assert!(offer_id.is_ok());
    }

    #[test]
    fn test_submit_offer_insufficient_capacity() {
        let mut neg = CapacityNegotiator::default_config();
        neg.register_pool(make_pool("pool-1", 1000.0, 0.8)).unwrap();
        let mut pool2 = make_pool("pool-2", 50.0, 0.9);
        pool2.allocated_capacity = 40.0;
        neg.register_pool(pool2).unwrap();
        let neg_id = neg
            .create_negotiation("pool-1".to_string(), "pool-2".to_string(), 100.0)
            .unwrap();
        let result = neg.submit_offer(neg_id, "pool-2".to_string(), 100.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_accept_offer() {
        let mut neg = CapacityNegotiator::default_config();
        neg.register_pool(make_pool("pool-1", 1000.0, 0.8)).unwrap();
        neg.register_pool(make_pool("pool-2", 2000.0, 0.9)).unwrap();
        let neg_id = neg
            .create_negotiation("pool-1".to_string(), "pool-2".to_string(), 100.0)
            .unwrap();
        assert!(neg.accept_offer(&neg_id, 100.0).is_ok());
        assert_eq!(
            neg.get_negotiation(&neg_id).unwrap().status,
            NegotiationStatus::Accepted
        );
    }

    #[test]
    fn test_complete_negotiation() {
        let mut neg = CapacityNegotiator::default_config();
        neg.register_pool(make_pool("pool-1", 1000.0, 0.8)).unwrap();
        neg.register_pool(make_pool("pool-2", 2000.0, 0.9)).unwrap();
        let neg_id = neg
            .create_negotiation("pool-1".to_string(), "pool-2".to_string(), 100.0)
            .unwrap();
        neg.accept_offer(&neg_id, 100.0).unwrap();
        assert!(neg.complete_negotiation(&neg_id).is_ok());
        assert_eq!(neg.stats().total_completed, 1);
    }

    #[test]
    fn test_reject_negotiation() {
        let mut neg = CapacityNegotiator::default_config();
        neg.register_pool(make_pool("pool-1", 1000.0, 0.8)).unwrap();
        neg.register_pool(make_pool("pool-2", 2000.0, 0.9)).unwrap();
        let neg_id = neg
            .create_negotiation("pool-1".to_string(), "pool-2".to_string(), 100.0)
            .unwrap();
        assert!(neg.reject_negotiation(&neg_id).is_ok());
        assert_eq!(neg.stats().total_rejected, 1);
    }

    #[test]
    fn test_negotiation_expiration() {
        let mut neg = CapacityNegotiator::default_config();
        neg.register_pool(make_pool("pool-1", 1000.0, 0.8)).unwrap();
        neg.register_pool(make_pool("pool-2", 2000.0, 0.9)).unwrap();
        let neg_id = neg
            .create_negotiation("pool-1".to_string(), "pool-2".to_string(), 100.0)
            .unwrap();
        neg.advance_time(60000);
        let cleaned = neg.cleanup_expired();
        assert_eq!(cleaned, 1);
        assert_eq!(neg.stats().total_expired, 1);
    }

    #[test]
    fn test_find_best_pool() {
        let mut neg = CapacityNegotiator::default_config();
        neg.register_pool(make_pool("pool-1", 1000.0, 0.8)).unwrap();
        neg.register_pool(make_pool("pool-2", 500.0, 0.9)).unwrap();
        let best = neg.find_best_pool(600.0);
        assert!(best.is_some());
        assert_eq!(best.unwrap().pool_id, "pool-1");
    }

    #[test]
    fn test_pending_negotiations() {
        let mut neg = CapacityNegotiator::default_config();
        neg.register_pool(make_pool("pool-1", 1000.0, 0.8)).unwrap();
        neg.register_pool(make_pool("pool-2", 2000.0, 0.9)).unwrap();
        neg.create_negotiation("pool-1".to_string(), "pool-2".to_string(), 100.0)
            .unwrap();
        neg.create_negotiation("pool-1".to_string(), "pool-2".to_string(), 200.0)
            .unwrap();
        let pending = neg.pending_negotiations();
        assert_eq!(pending.len(), 2);
    }

    #[test]
    fn test_next_negotiation() {
        let mut neg = CapacityNegotiator::default_config();
        neg.register_pool(make_pool("pool-1", 1000.0, 0.8)).unwrap();
        neg.register_pool(make_pool("pool-2", 2000.0, 0.9)).unwrap();
        neg.create_negotiation("pool-1".to_string(), "pool-2".to_string(), 100.0)
            .unwrap();
        assert!(neg.next_negotiation().is_some());
    }

    #[test]
    fn test_completed_history() {
        let mut neg = CapacityNegotiator::default_config();
        neg.register_pool(make_pool("pool-1", 1000.0, 0.8)).unwrap();
        neg.register_pool(make_pool("pool-2", 2000.0, 0.9)).unwrap();
        let neg_id = neg
            .create_negotiation("pool-1".to_string(), "pool-2".to_string(), 100.0)
            .unwrap();
        neg.accept_offer(&neg_id, 100.0).unwrap();
        neg.complete_negotiation(&neg_id).unwrap();
        assert_eq!(neg.completed_history().len(), 1);
    }

    #[test]
    fn test_reset_stats() {
        let mut neg = CapacityNegotiator::default_config();
        neg.reset_stats();
        assert_eq!(neg.stats().total_negotiations, 0);
    }

    #[test]
    fn test_queue_full() {
        let mut config = CapacityNegotiatorConfig::default();
        config.max_pending_negotiations = 1;
        let mut neg = CapacityNegotiator::new(config);
        neg.register_pool(make_pool("pool-1", 1000.0, 0.8)).unwrap();
        neg.register_pool(make_pool("pool-2", 2000.0, 0.9)).unwrap();
        neg.create_negotiation("pool-1".to_string(), "pool-2".to_string(), 100.0)
            .unwrap();
        assert!(neg
            .create_negotiation("pool-1".to_string(), "pool-2".to_string(), 200.0)
            .is_err());
    }

    #[test]
    fn test_negotiation_status_display() {
        assert_eq!(NegotiationStatus::Pending.to_string(), "Pending");
        assert_eq!(NegotiationStatus::Accepted.to_string(), "Accepted");
        assert_eq!(NegotiationStatus::Completed.to_string(), "Completed");
        assert_eq!(NegotiationStatus::Rejected.to_string(), "Rejected");
        assert_eq!(NegotiationStatus::Expired.to_string(), "Expired");
    }

    #[test]
    fn test_error_display() {
        match NegotiatorError::PoolNotFound("x".to_string()) {
            e => assert!(!e.to_string().is_empty()),
        }
    }

    #[test]
    fn test_config_default() {
        let config = CapacityNegotiatorConfig::default();
        assert_eq!(config.max_pending_negotiations, 1024);
        assert_eq!(config.default_negotiation_window_ms, 30000);
        assert_eq!(config.min_capacity, 1.0);
    }

    #[test]
    fn test_stats_default() {
        let stats = NegotiatorStats::default();
        assert_eq!(stats.total_negotiations, 0);
        assert_eq!(stats.total_completed, 0);
    }

    #[test]
    fn test_negotiator_default() {
        let neg = CapacityNegotiator::default();
        assert_eq!(neg.stats().total_negotiations, 0);
    }

    #[test]
    fn test_negotiation_urgency() {
        let neg_req = NegotiationRequest::new(
            "n1".to_string(),
            "p1".to_string(),
            "p2".to_string(),
            100.0,
            10000,
            1000,
        );
        let urgency = neg_req.urgency(5000);
        assert!(urgency >= 0.0);
        assert!(urgency <= 1.0);
    }

    #[test]
    fn test_negotiation_time_remaining() {
        let neg_req = NegotiationRequest::new(
            "n1".to_string(),
            "p1".to_string(),
            "p2".to_string(),
            100.0,
            10000,
            1000,
        );
        assert_eq!(neg_req.time_remaining(5000), 6000);
    }

    #[test]
    fn test_offer_expired() {
        let offer = NegotiationOffer::new(
            "o1".to_string(),
            "n1".to_string(),
            "p1".to_string(),
            100.0,
            10000,
            1000,
        );
        assert!(!offer.is_expired(5000));
        assert!(offer.is_expired(15000));
    }

    #[test]
    fn test_auto_cleanup_disabled() {
        let mut config = CapacityNegotiatorConfig::default();
        config.auto_cleanup = false;
        let mut neg = CapacityNegotiator::new(config);
        neg.register_pool(make_pool("pool-1", 1000.0, 0.8)).unwrap();
        neg.register_pool(make_pool("pool-2", 2000.0, 0.9)).unwrap();
        neg.create_negotiation("pool-1".to_string(), "pool-2".to_string(), 100.0)
            .unwrap();
        neg.advance_time(60000);
        let cleaned = neg.auto_cleanup();
        assert_eq!(cleaned, 0);
    }

    #[test]
    fn test_capacity_negotiated_tracking() {
        let mut neg = CapacityNegotiator::default_config();
        neg.register_pool(make_pool("pool-1", 1000.0, 0.8)).unwrap();
        neg.register_pool(make_pool("pool-2", 2000.0, 0.9)).unwrap();
        let neg_id = neg
            .create_negotiation("pool-1".to_string(), "pool-2".to_string(), 100.0)
            .unwrap();
        neg.accept_offer(&neg_id, 150.0).unwrap();
        neg.complete_negotiation(&neg_id).unwrap();
        assert_eq!(neg.stats().total_capacity_negotiated, 150.0);
    }
}
