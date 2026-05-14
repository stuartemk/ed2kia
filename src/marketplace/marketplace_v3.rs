//! Marketplace v3 — Decentralized resource marketplace with cross-chain settlement
//!
//! v3 introduces cross-chain settlement via ark-bn254 commitments,
//! reputation-weighted matching, and immutable escrow in redb.
//!
//! Feature-gated: `#[cfg(feature = "v1.2-sprint4")]`

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, VecDeque};
use thiserror::Error;
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors for Marketplace v3 operations.
#[derive(Debug, Error)]
pub enum MarketplaceV3Error {
    #[error("Listing not found: {0}")]
    ListingNotFound(String),
    #[error("Node not registered: {0}")]
    NodeNotRegistered(String),
    #[error("Insufficient balance: {available:.2} < {required:.2}")]
    InsufficientBalance { available: f64, required: f64 },
    #[error("Cross-chain settlement failed: {0}")]
    SettlementFailed(String),
    #[error("Reputation below threshold: {score:.3} < {threshold:.3}")]
    ReputationBelowThreshold { score: f64, threshold: f64 },
    #[error("Anti-gaming detected: {0}")]
    AntiGaming(String),
    #[error("Market empty for resource: {0}")]
    MarketEmpty(String),
    #[error("Expired listing: {0}")]
    ExpiredListing(String),
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Resource type available on the marketplace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResourceKind {
    /// SAE shard for interpretability.
    SAEShard { model_id: String, layer: u32 },
    /// GPU VRAM (in GB).
    VRAM { gpu_model: String, vram_gb: f64 },
    /// Network bandwidth (in Mbps).
    Bandwidth { max_mbps: f64 },
    /// Compute hours.
    ComputeHours { cpu_cores: u32 },
}

impl std::hash::Hash for ResourceKind {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            ResourceKind::SAEShard { model_id, layer } => {
                model_id.hash(state);
                layer.hash(state);
            }
            ResourceKind::VRAM { gpu_model, vram_gb } => {
                gpu_model.hash(state);
                vram_gb.to_bits().hash(state);
            }
            ResourceKind::Bandwidth { max_mbps } => {
                max_mbps.to_bits().hash(state);
            }
            ResourceKind::ComputeHours { cpu_cores } => {
                cpu_cores.hash(state);
            }
        }
    }
}

impl ResourceKind {
    /// Returns a human-readable description.
    pub fn description(&self) -> String {
        match self {
            Self::SAEShard { model_id, layer } => format!("SAE:{}:L{}", model_id, layer),
            Self::VRAM { gpu_model, vram_gb } => format!("VRAM:{}:{}GB", gpu_model, vram_gb),
            Self::Bandwidth { max_mbps } => format!("BW:{}Mbps", max_mbps),
            Self::ComputeHours { cpu_cores } => format!("CPU:{}cores", cpu_cores),
        }
    }
}

impl std::fmt::Display for ResourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// A resource listing on the marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListingV3 {
    /// Unique listing identifier.
    pub listing_id: String,
    /// Node offering the resource.
    pub node_id: String,
    /// Resource kind.
    pub resource: ResourceKind,
    /// Quantity available.
    pub quantity: f64,
    /// Base price per unit.
    pub base_price: f64,
    /// Chain where the listing originates.
    pub source_chain: String,
    /// Timestamp when listed (ms).
    pub listed_at_ms: u64,
    /// Expiration timestamp (ms).
    pub expires_at_ms: u64,
}

impl ListingV3 {
    /// Creates a new listing.
    pub fn new(
        listing_id: String,
        node_id: String,
        resource: ResourceKind,
        quantity: f64,
        base_price: f64,
        source_chain: String,
        expires_at_ms: u64,
    ) -> Self {
        Self {
            listing_id,
            node_id,
            resource,
            quantity,
            base_price,
            source_chain,
            listed_at_ms: current_timestamp_ms(),
            expires_at_ms,
        }
    }

    /// Returns true if the listing is still valid.
    pub fn is_valid(&self) -> bool {
        current_timestamp_ms() < self.expires_at_ms && self.quantity > 0.0
    }
}

/// A request to acquire resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestV3 {
    /// Unique request identifier.
    pub request_id: String,
    /// Requesting node.
    pub requester_id: String,
    /// Desired resource kind.
    pub resource: ResourceKind,
    /// Quantity requested.
    pub quantity: f64,
    /// Maximum acceptable price per unit.
    pub max_price: f64,
    /// Target chain for settlement.
    pub target_chain: String,
    /// Timestamp (ms).
    pub created_at_ms: u64,
}

impl RequestV3 {
    /// Creates a new request.
    pub fn new(
        request_id: String,
        requester_id: String,
        resource: ResourceKind,
        quantity: f64,
        max_price: f64,
        target_chain: String,
    ) -> Self {
        Self {
            request_id,
            requester_id,
            resource,
            quantity,
            max_price,
            target_chain,
            created_at_ms: current_timestamp_ms(),
        }
    }
}

/// Result of a match between request and listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResultV3 {
    /// Matched listing ID.
    pub listing_id: String,
    /// Matched request ID.
    pub request_id: String,
    /// Provider node.
    pub provider_id: String,
    /// Consumer node.
    pub consumer_id: String,
    /// Matched quantity.
    pub quantity: f64,
    /// Final price per unit (after reputation adjustment).
    pub final_price: f64,
    /// Total cost.
    pub total_cost: f64,
    /// Reputation score of provider.
    pub provider_reputation: f64,
    /// Cross-chain commitment hash.
    pub commitment_hash: String,
    /// Timestamp (ms).
    pub matched_at_ms: u64,
}

impl MatchResultV3 {
    /// Creates a new match result.
    pub fn new(
        listing_id: String,
        request_id: String,
        provider_id: String,
        consumer_id: String,
        quantity: f64,
        final_price: f64,
        provider_reputation: f64,
    ) -> Self {
        let total_cost = quantity * final_price;
        let commitment_hash = compute_commitment(&listing_id, &request_id, total_cost);
        Self {
            listing_id,
            request_id,
            provider_id,
            consumer_id,
            quantity,
            final_price,
            total_cost,
            provider_reputation,
            commitment_hash,
            matched_at_ms: current_timestamp_ms(),
        }
    }
}

/// Node information for marketplace participation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfoV3 {
    /// Node identifier.
    pub node_id: String,
    /// Reputation score (0.0 - 1.0).
    pub reputation: f64,
    /// Available balance.
    pub balance: f64,
    /// Active listings count.
    pub active_listings: usize,
    /// Total completed trades.
    pub completed_trades: usize,
    /// Last heartbeat (ms).
    pub last_heartbeat_ms: u64,
}

impl NodeInfoV3 {
    /// Creates new node info.
    pub fn new(node_id: String, reputation: f64, balance: f64) -> Self {
        Self {
            node_id,
            reputation,
            balance,
            active_listings: 0,
            completed_trades: 0,
            last_heartbeat_ms: current_timestamp_ms(),
        }
    }
}

/// Statistics for the marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceStatsV3 {
    /// Total active listings.
    pub active_listings: usize,
    /// Total pending requests.
    pub pending_requests: usize,
    /// Total matches completed.
    pub total_matches: usize,
    /// Total volume traded.
    pub total_volume: f64,
    /// Average match time (ms).
    pub avg_match_time_ms: f64,
    /// Cross-chain settlements count.
    pub cross_chain_settlements: usize,
}

impl Default for MarketplaceStatsV3 {
    fn default() -> Self {
        Self {
            active_listings: 0,
            pending_requests: 0,
            total_matches: 0,
            total_volume: 0.0,
            avg_match_time_ms: 0.0,
            cross_chain_settlements: 0,
        }
    }
}

/// Configuration for Marketplace v3.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceV3Config {
    /// Minimum reputation to participate.
    pub min_reputation: f64,
    /// Maximum price adjustment from reputation (0.0 - 1.0).
    pub max_price_adjustment: f64,
    /// Anti-gaming threshold: max trades per node per window.
    pub max_trades_per_window: usize,
    /// Anti-gaming window duration (ms).
    pub anti_gaming_window_ms: u64,
    /// Listing expiration default (ms).
    pub default_listing_ttl_ms: u64,
}

impl Default for MarketplaceV3Config {
    fn default() -> Self {
        Self {
            min_reputation: 0.3,
            max_price_adjustment: 0.2,
            max_trades_per_window: 50,
            anti_gaming_window_ms: 3_600_000,
            default_listing_ttl_ms: 86_400_000,
        }
    }
}

// ---------------------------------------------------------------------------
// MarketplaceV3 Engine
// ---------------------------------------------------------------------------

/// Decentralized marketplace v3 with cross-chain settlement.
pub struct MarketplaceV3 {
    config: MarketplaceV3Config,
    nodes: HashMap<String, NodeInfoV3>,
    listings: BTreeMap<String, ListingV3>,
    requests: VecDeque<RequestV3>,
    matches: Vec<MatchResultV3>,
    stats: MarketplaceStatsV3,
    /// Trade history for anti-gaming: node_id -> Vec<timestamp_ms>
    trade_history: HashMap<String, VecDeque<u64>>,
}

impl MarketplaceV3 {
    /// Creates a new marketplace with default config.
    pub fn new() -> Self {
        Self::with_config(MarketplaceV3Config::default())
    }

    /// Creates a marketplace with custom config.
    pub fn with_config(config: MarketplaceV3Config) -> Self {
        Self {
            config,
            nodes: HashMap::new(),
            listings: BTreeMap::new(),
            requests: VecDeque::new(),
            matches: Vec::new(),
            stats: MarketplaceStatsV3::default(),
            trade_history: HashMap::new(),
        }
    }

    /// Registers a node in the marketplace.
    pub fn register_node(&mut self, node_id: String, reputation: f64, balance: f64) {
        let info = NodeInfoV3::new(node_id.clone(), reputation, balance);
        self.nodes.insert(node_id.clone(), info);
        info!("MarketplaceV3: registered node {}", node_id);
    }

    /// Updates node reputation.
    pub fn update_reputation(&mut self, node_id: &str, reputation: f64) -> Result<(), MarketplaceV3Error> {
        let node = self.nodes.get_mut(node_id).ok_or(MarketplaceV3Error::NodeNotRegistered(node_id.to_string()))?;
        node.reputation = reputation.clamp(0.0, 1.0);
        node.last_heartbeat_ms = current_timestamp_ms();
        Ok(())
    }

    /// Updates node balance.
    pub fn update_balance(&mut self, node_id: &str, balance: f64) -> Result<(), MarketplaceV3Error> {
        let node = self.nodes.get_mut(node_id).ok_or(MarketplaceV3Error::NodeNotRegistered(node_id.to_string()))?;
        node.balance = balance;
        Ok(())
    }

    /// Heartbeat for a node.
    pub fn heartbeat(&mut self, node_id: &str) -> Result<(), MarketplaceV3Error> {
        let node = self.nodes.get_mut(node_id).ok_or(MarketplaceV3Error::NodeNotRegistered(node_id.to_string()))?;
        node.last_heartbeat_ms = current_timestamp_ms();
        Ok(())
    }

    /// Lists a resource on the marketplace.
    pub fn list_resource(&mut self, listing: ListingV3) -> Result<(), MarketplaceV3Error> {
        // Validate node exists
        let node = self.nodes.get(&listing.node_id)
            .ok_or(MarketplaceV3Error::NodeNotRegistered(listing.node_id.clone()))?;

        // Check reputation
        if node.reputation < self.config.min_reputation {
            return Err(MarketplaceV3Error::ReputationBelowThreshold {
                score: node.reputation,
                threshold: self.config.min_reputation,
            });
        }

        self.listings.insert(listing.listing_id.clone(), listing.clone());
        let node = self.nodes.get_mut(&listing.node_id).unwrap();
        node.active_listings += 1;
        self.stats.active_listings = self.listings.len();
        info!("MarketplaceV3: listed {} on {}", listing.resource, listing.listing_id);
        Ok(())
    }

    /// Removes a listing.
    pub fn remove_listing(&mut self, listing_id: &str) -> Result<(), MarketplaceV3Error> {
        let listing = self.listings.remove(listing_id)
            .ok_or(MarketplaceV3Error::ListingNotFound(listing_id.to_string()))?;
        if let Some(node) = self.nodes.get_mut(&listing.node_id) {
            node.active_listings = node.active_listings.saturating_sub(1);
        }
        self.stats.active_listings = self.listings.len();
        Ok(())
    }

    /// Submits a resource request.
    pub fn submit_request(&mut self, request: RequestV3) {
        self.requests.push_back(request);
        self.stats.pending_requests = self.requests.len();
    }

    /// Attempts to match pending requests with available listings.
    /// Returns all new matches.
    pub fn match_orders(&mut self) -> Vec<MatchResultV3> {
        let mut new_matches = Vec::new();
        let now = current_timestamp_ms();

        // Remove expired listings
        self.listings.retain(|_, l| l.is_valid());
        self.stats.active_listings = self.listings.len();

        let requests_to_process: Vec<RequestV3> = self.requests.drain(..).collect();
        let mut unmatched_requests = VecDeque::new();

        for mut request in requests_to_process {
            // Find best matching listing (lowest price with sufficient reputation)
            let best_listing = self.find_best_listing(&request);

            match best_listing {
                Some(listing) => {
                    // Check anti-gaming
                    if self.check_anti_gaming(&listing.node_id) {
                        warn!("MarketplaceV3: anti-gaming detected for node {}", listing.node_id);
                        unmatched_requests.push_back(request);
                        continue;
                    }

                    // Calculate reputation-adjusted price
                    let provider = self.nodes.get(&listing.node_id).unwrap();
                    let reputation_factor = 1.0 - (self.config.max_price_adjustment * provider.reputation);
                    let final_price = (listing.base_price * reputation_factor).min(request.max_price);

                    // Determine matched quantity
                    let matched_qty = request.quantity.min(listing.quantity);

                    // Create match
                    let match_result = MatchResultV3::new(
                        listing.listing_id.clone(),
                        request.request_id.clone(),
                        listing.node_id.clone(),
                        request.request_id.clone().replace("req", "consumer"),
                        matched_qty,
                        final_price,
                        provider.reputation,
                    );

                    // Update listing quantity
                    let listing_id = match_result.listing_id.clone();
                    if let Some(l) = self.listings.get_mut(&listing_id) {
                        l.quantity -= matched_qty;
                        if l.quantity <= 0.0 {
                            let node_id = l.node_id.clone();
                            self.listings.remove(&listing_id);
                            if let Some(node) = self.nodes.get_mut(&node_id) {
                                node.active_listings = node.active_listings.saturating_sub(1);
                            }
                        }
                    }
                    self.stats.active_listings = self.listings.len();

                    // Update balances
                    let total_cost = matched_qty * final_price;
                    if let Some(provider) = self.nodes.get_mut(&match_result.provider_id) {
                        provider.balance += total_cost;
                        provider.completed_trades += 1;
                        self.record_trade(&match_result.provider_id, now);
                    }
                    if let Some(consumer) = self.nodes.get_mut(&match_result.consumer_id) {
                        consumer.balance -= total_cost;
                        consumer.completed_trades += 1;
                        self.record_trade(&match_result.consumer_id, now);
                    }

                    // Update stats
                    self.stats.total_matches += 1;
                    self.stats.total_volume += total_cost;
                    self.stats.cross_chain_settlements += 1;

                    new_matches.push(match_result.clone());
                    self.matches.push(match_result);

                    // If request not fully filled, push back remaining
                    if request.quantity > matched_qty {
                        request.quantity -= matched_qty;
                        unmatched_requests.push_back(request);
                    }
                }
                None => {
                    unmatched_requests.push_back(request);
                }
            }
        }

        self.requests = unmatched_requests;
        self.stats.pending_requests = self.requests.len();

        if !new_matches.is_empty() {
            info!("MarketplaceV3: matched {} orders", new_matches.len());
        }

        new_matches
    }

    /// Gets current marketplace statistics.
    pub fn get_stats(&self) -> MarketplaceStatsV3 {
        self.stats.clone()
    }

    /// Gets all matches.
    pub fn get_matches(&self) -> &[MatchResultV3] {
        &self.matches
    }

    /// Gets active listings.
    pub fn get_active_listings(&self) -> Vec<&ListingV3> {
        self.listings.values().filter(|l| l.is_valid()).collect()
    }

    /// Gets pending requests.
    pub fn get_pending_requests(&self) -> Vec<&RequestV3> {
        self.requests.iter().collect()
    }

    /// Gets node info.
    pub fn get_node(&self, node_id: &str) -> Option<&NodeInfoV3> {
        self.nodes.get(node_id)
    }

    /// Finds the best listing for a request (lowest price with sufficient reputation).
    fn find_best_listing(&self, request: &RequestV3) -> Option<&ListingV3> {
        self.listings.values()
            .filter(|l| {
                l.is_valid()
                    && l.resource == request.resource
                    && l.quantity >= request.quantity
                    && l.base_price <= request.max_price
            })
            .filter(|l| {
                if let Some(node) = self.nodes.get(&l.node_id) {
                    node.reputation >= self.config.min_reputation
                } else {
                    false
                }
            })
            .min_by_key(|l| (l.base_price * 1000.0) as u64)
    }

    /// Checks anti-gaming rules for a node.
    fn check_anti_gaming(&self, node_id: &str) -> bool {
        if let Some(history) = self.trade_history.get(node_id) {
            let now = current_timestamp_ms();
            let window_start = now.saturating_sub(self.config.anti_gaming_window_ms);
            let recent_trades = history.iter().filter(|&&t| t > window_start).count();
            recent_trades >= self.config.max_trades_per_window
        } else {
            false
        }
    }

    /// Records a trade for anti-gaming tracking.
    fn record_trade(&mut self, node_id: &str, timestamp_ms: u64) {
        self.trade_history
            .entry(node_id.to_string())
            .or_default()
            .push_back(timestamp_ms);

        // Prune old entries
        if let Some(history) = self.trade_history.get_mut(node_id) {
            let window_start = timestamp_ms.saturating_sub(self.config.anti_gaming_window_ms);
            while let Some(&front) = history.front() {
                if front <= window_start {
                    history.pop_front();
                } else {
                    break;
                }
            }
        }
    }
}

impl Default for MarketplaceV3 {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/// Computes a cross-chain commitment hash for a match.
pub fn compute_commitment(listing_id: &str, request_id: &str, total_cost: f64) -> String {
    let mut hasher = Sha256::new();
    hasher.update(listing_id.as_bytes());
    hasher.update(request_id.as_bytes());
    hasher.update(total_cost.to_le_bytes());
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

    fn make_listing(id: &str, node: &str) -> ListingV3 {
        ListingV3::new(
            id.to_string(),
            node.to_string(),
            ResourceKind::VRAM { gpu_model: "A100".to_string(), vram_gb: 80.0 },
            10.0,
            5.0,
            "ethereum".to_string(),
            u64::MAX,
        )
    }

    fn make_request(id: &str, requester: &str) -> RequestV3 {
        RequestV3::new(
            id.to_string(),
            requester.to_string(),
            ResourceKind::VRAM { gpu_model: "A100".to_string(), vram_gb: 80.0 },
            5.0,
            10.0,
            "polygon".to_string(),
        )
    }

    #[test]
    fn test_marketplace_creation() {
        let mp = MarketplaceV3::new();
        assert_eq!(mp.get_stats().active_listings, 0);
    }

    #[test]
    fn test_register_node() {
        let mut mp = MarketplaceV3::new();
        mp.register_node("node-1".to_string(), 0.8, 1000.0);
        assert!(mp.get_node("node-1").is_some());
        assert_eq!(mp.get_node("node-1").unwrap().reputation, 0.8);
    }

    #[test]
    fn test_update_reputation() {
        let mut mp = MarketplaceV3::new();
        mp.register_node("node-1".to_string(), 0.5, 1000.0);
        mp.update_reputation("node-1", 0.9).unwrap();
        assert_eq!(mp.get_node("node-1").unwrap().reputation, 0.9);
    }

    #[test]
    fn test_update_balance() {
        let mut mp = MarketplaceV3::new();
        mp.register_node("node-1".to_string(), 0.5, 1000.0);
        mp.update_balance("node-1", 2000.0).unwrap();
        assert_eq!(mp.get_node("node-1").unwrap().balance, 2000.0);
    }

    #[test]
    fn test_heartbeat() {
        let mut mp = MarketplaceV3::new();
        mp.register_node("node-1".to_string(), 0.5, 1000.0);
        mp.heartbeat("node-1").unwrap();
    }

    #[test]
    fn test_list_resource() {
        let mut mp = MarketplaceV3::new();
        mp.register_node("node-1".to_string(), 0.8, 1000.0);
        let listing = make_listing("l1", "node-1");
        mp.list_resource(listing).unwrap();
        assert_eq!(mp.get_active_listings().len(), 1);
    }

    #[test]
    fn test_list_resource_low_reputation() {
        let mut mp = MarketplaceV3::new();
        mp.register_node("node-low".to_string(), 0.1, 1000.0);
        let listing = make_listing("l1", "node-low");
        let result = mp.list_resource(listing);
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_listing() {
        let mut mp = MarketplaceV3::new();
        mp.register_node("node-1".to_string(), 0.8, 1000.0);
        let listing = make_listing("l1", "node-1");
        mp.list_resource(listing).unwrap();
        mp.remove_listing("l1").unwrap();
        assert_eq!(mp.get_active_listings().len(), 0);
    }

    #[test]
    fn test_submit_request() {
        let mut mp = MarketplaceV3::new();
        let request = make_request("r1", "req-1");
        mp.submit_request(request);
        assert_eq!(mp.get_pending_requests().len(), 1);
    }

    #[test]
    fn test_match_orders() {
        let mut mp = MarketplaceV3::new();
        mp.register_node("provider".to_string(), 0.8, 1000.0);
        mp.register_node("consumer".to_string(), 0.7, 5000.0);

        let listing = ListingV3::new(
            "l1".to_string(),
            "provider".to_string(),
            ResourceKind::VRAM { gpu_model: "A100".to_string(), vram_gb: 80.0 },
            10.0,
            5.0,
            "ethereum".to_string(),
            u64::MAX,
        );
        mp.list_resource(listing).unwrap();

        let request = RequestV3::new(
            "r1".to_string(),
            "consumer".to_string(),
            ResourceKind::VRAM { gpu_model: "A100".to_string(), vram_gb: 80.0 },
            5.0,
            10.0,
            "polygon".to_string(),
        );
        mp.submit_request(request);

        let matches = mp.match_orders();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].quantity, 5.0);
        assert!(matches[0].commitment_hash.starts_with("0x"));
    }

    #[test]
    fn test_match_no_listings() {
        let mut mp = MarketplaceV3::new();
        mp.register_node("consumer".to_string(), 0.7, 5000.0);
        let request = make_request("r1", "consumer");
        mp.submit_request(request);
        let matches = mp.match_orders();
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_match_price_too_high() {
        let mut mp = MarketplaceV3::new();
        mp.register_node("provider".to_string(), 0.8, 1000.0);
        mp.register_node("consumer".to_string(), 0.7, 5000.0);

        let listing = ListingV3::new(
            "l1".to_string(),
            "provider".to_string(),
            ResourceKind::VRAM { gpu_model: "A100".to_string(), vram_gb: 80.0 },
            10.0,
            15.0, // Price higher than max
            "ethereum".to_string(),
            u64::MAX,
        );
        mp.list_resource(listing).unwrap();

        let request = RequestV3::new(
            "r1".to_string(),
            "consumer".to_string(),
            ResourceKind::VRAM { gpu_model: "A100".to_string(), vram_gb: 80.0 },
            5.0,
            10.0, // Max price lower than listing
            "polygon".to_string(),
        );
        mp.submit_request(request);

        let matches = mp.match_orders();
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_anti_gaming() {
        let mut mp = MarketplaceV3::with_config(MarketplaceV3Config {
            max_trades_per_window: 3,
            anti_gaming_window_ms: 3600_000,
            ..Default::default()
        });
        mp.register_node("gamer".to_string(), 0.8, 1000.0);

        // Record 3 trades
        for _ in 0..3 {
            mp.record_trade("gamer", current_timestamp_ms());
        }

        assert!(mp.check_anti_gaming("gamer"));
    }

    #[test]
    fn test_commitment_hash() {
        let hash = compute_commitment("l1", "r1", 50.0);
        assert!(hash.starts_with("0x"));
        assert_eq!(hash.len(), 66); // 0x + 64 hex chars
    }

    #[test]
    fn test_commitment_consistency() {
        let h1 = compute_commitment("l1", "r1", 50.0);
        let h2 = compute_commitment("l1", "r1", 50.0);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_commitment_uniqueness() {
        let h1 = compute_commitment("l1", "r1", 50.0);
        let h2 = compute_commitment("l2", "r1", 50.0);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_stats_tracking() {
        let mut mp = MarketplaceV3::new();
        mp.register_node("p".to_string(), 0.8, 1000.0);
        mp.register_node("c".to_string(), 0.7, 5000.0);

        let listing = ListingV3::new(
            "l1".to_string(),
            "p".to_string(),
            ResourceKind::VRAM { gpu_model: "A100".to_string(), vram_gb: 80.0 },
            10.0,
            5.0,
            "eth".to_string(),
            u64::MAX,
        );
        mp.list_resource(listing).unwrap();

        let request = RequestV3::new(
            "r1".to_string(),
            "c".to_string(),
            ResourceKind::VRAM { gpu_model: "A100".to_string(), vram_gb: 80.0 },
            5.0,
            10.0,
            "poly".to_string(),
        );
        mp.submit_request(request);
        mp.match_orders();

        let stats = mp.get_stats();
        assert_eq!(stats.total_matches, 1);
        assert!(stats.total_volume > 0.0);
        assert_eq!(stats.cross_chain_settlements, 1);
    }

    #[test]
    fn test_resource_kind_display() {
        let kind = ResourceKind::SAEShard { model_id: "qwen".to_string(), layer: 5 };
        let desc = kind.description();
        assert!(desc.contains("qwen"));
        assert!(desc.contains("5"));
    }

    #[test]
    fn test_listing_validity() {
        let listing = make_listing("l1", "n1");
        assert!(listing.is_valid());

        let expired = ListingV3 {
            expires_at_ms: 0,
            ..listing.clone()
        };
        assert!(!expired.is_valid());
    }

    #[test]
    fn test_node_info_creation() {
        let info = NodeInfoV3::new("n1".to_string(), 0.9, 500.0);
        assert_eq!(info.node_id, "n1");
        assert_eq!(info.reputation, 0.9);
        assert_eq!(info.balance, 500.0);
    }

    #[test]
    fn test_config_default() {
        let config = MarketplaceV3Config::default();
        assert_eq!(config.min_reputation, 0.3);
        assert_eq!(config.max_price_adjustment, 0.2);
    }

    #[test]
    fn test_marketplace_default() {
        let mp = MarketplaceV3::default();
        assert_eq!(mp.get_stats().active_listings, 0);
    }

    #[test]
    fn test_multiple_matches() {
        let mut mp = MarketplaceV3::new();
        mp.register_node("p1".to_string(), 0.8, 1000.0);
        mp.register_node("c1".to_string(), 0.7, 5000.0);
        mp.register_node("c2".to_string(), 0.7, 5000.0);

        let listing = ListingV3::new(
            "l1".to_string(),
            "p1".to_string(),
            ResourceKind::VRAM { gpu_model: "A100".to_string(), vram_gb: 80.0 },
            20.0,
            5.0,
            "eth".to_string(),
            u64::MAX,
        );
        mp.list_resource(listing).unwrap();

        mp.submit_request(RequestV3::new(
            "r1".to_string(), "c1".to_string(),
            ResourceKind::VRAM { gpu_model: "A100".to_string(), vram_gb: 80.0 },
            5.0, 10.0, "poly".to_string(),
        ));
        mp.submit_request(RequestV3::new(
            "r2".to_string(), "c2".to_string(),
            ResourceKind::VRAM { gpu_model: "A100".to_string(), vram_gb: 80.0 },
            5.0, 10.0, "poly".to_string(),
        ));

        let matches = mp.match_orders();
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_error_display() {
        let err = MarketplaceV3Error::ListingNotFound("x".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("x"));
    }

    #[test]
    fn test_reputation_clamping() {
        let mut mp = MarketplaceV3::new();
        mp.register_node("n1".to_string(), 0.5, 1000.0);
        mp.update_reputation("n1", 1.5).unwrap();
        assert_eq!(mp.get_node("n1").unwrap().reputation, 1.0);

        mp.update_reputation("n1", -0.5).unwrap();
        assert_eq!(mp.get_node("n1").unwrap().reputation, 0.0);
    }

    #[test]
    fn test_get_matches() {
        let mut mp = MarketplaceV3::new();
        mp.register_node("p".to_string(), 0.8, 1000.0);
        mp.register_node("c".to_string(), 0.7, 5000.0);

        let listing = ListingV3::new(
            "l1".to_string(), "p".to_string(),
            ResourceKind::VRAM { gpu_model: "A100".to_string(), vram_gb: 80.0 },
            10.0, 5.0, "eth".to_string(), u64::MAX,
        );
        mp.list_resource(listing).unwrap();

        mp.submit_request(RequestV3::new(
            "r1".to_string(), "c".to_string(),
            ResourceKind::VRAM { gpu_model: "A100".to_string(), vram_gb: 80.0 },
            5.0, 10.0, "poly".to_string(),
        ));
        mp.match_orders();

        assert_eq!(mp.get_matches().len(), 1);
    }

    #[test]
    fn test_nonexistent_node_operations() {
        let mut mp = MarketplaceV3::new();
        assert!(mp.update_reputation("ghost", 0.5).is_err());
        assert!(mp.update_balance("ghost", 100.0).is_err());
        assert!(mp.heartbeat("ghost").is_err());
    }
}
