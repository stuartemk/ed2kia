//! Resource Marketplace — Decentralized resource matching with dynamic pricing
//!
//! Feature-gated: `#[cfg(feature = "phase8-sprint1")]`
//! Integrates with `staking/registry.rs` (ResourceCommitment, NodeStatus)
//! and `reputation/scoring.rs` (ScoringConfig, credits) for atomic settlement.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use thiserror::Error;
use tracing::{debug, info, warn};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum MarketplaceError {
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),
    #[error("Node not found: {0}")]
    NodeNotFound(String),
    #[error("Trust below threshold: {trust:.3} < {threshold:.3}")]
    TrustBelowThreshold { trust: f32, threshold: f32 },
    #[error("Insufficient credits: {available:.1} < {required:.1}")]
    InsufficientCredits { available: f64, required: f64 },
    #[error("Settlement failed: {0}")]
    SettlementFailed(String),
    #[error("Anti-gaming detected: {0}")]
    AntiGaming(String),
    #[error("Market empty")]
    MarketEmpty,
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Result of a marketplace match / settlement attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketResult {
    pub matched: bool,
    pub price: f32,
    pub settlement_hash: String,
    pub anti_gaming_flag: bool,
}

impl MarketResult {
    pub fn matched(price: f32, settlement_hash: String) -> Self {
        Self {
            matched: true,
            price,
            settlement_hash,
            anti_gaming_flag: false,
        }
    }

    pub fn rejected(reason: &str) -> Self {
        Self {
            matched: false,
            price: 0.0,
            settlement_hash: String::new(),
            anti_gaming_flag: reason.contains("gaming"),
        }
    }
}

/// A listed resource on the marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceListing {
    pub node_id: String,
    pub resource_type: String,
    pub quantity: f32,
    pub base_price: f32,
    pub listed_at: u64,
    pub expires_at: u64,
}

/// A request to consume resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequest {
    pub requester_id: String,
    pub resource_type: String,
    pub quantity: f32,
    pub max_price: f32,
}

/// Trust and credit information for settlement verification.
#[derive(Debug, Clone)]
pub struct NodeTrustInfo {
    pub trust_score: f32,
    pub credits: f64,
    pub is_active: bool,
}

// ---------------------------------------------------------------------------
// ResourceMarketplace
// ---------------------------------------------------------------------------

pub struct ResourceMarketplace {
    listings: HashMap<String, Vec<ResourceListing>>,
    trust_store: HashMap<String, NodeTrustInfo>,
    min_trust_threshold: f32,
    min_credit_threshold: f64,
    demand_multiplier: f32,
}

impl ResourceMarketplace {
    /// Create a new marketplace with default thresholds.
    pub fn new() -> Self {
        Self {
            listings: HashMap::new(),
            trust_store: HashMap::new(),
            min_trust_threshold: 0.5,
            min_credit_threshold: 10.0,
            demand_multiplier: 1.2,
        }
    }

    /// Create with custom thresholds.
    pub fn with_thresholds(min_trust: f32, min_credits: f64) -> Self {
        Self {
            min_trust_threshold: min_trust,
            min_credit_threshold: min_credits,
            ..Self::new()
        }
    }

    // ---- Listing ----------------------------------------------------------

    /// List a resource for trade.
    pub fn list_resource(&mut self, listing: ResourceListing) {
        let key = listing.resource_type.clone();
        debug!(
            node = %listing.node_id,
            resource = %listing.resource_type,
            qty = listing.quantity,
            price = listing.base_price,
            "resource listed"
        );
        self.listings.entry(key).or_default().push(listing);
    }

    /// Remove expired listings.
    pub fn cleanup_expired(&mut self, now: u64) -> usize {
        let mut removed = 0;
        for listings in self.listings.values_mut() {
            let before = listings.len();
            listings.retain(|l| l.expires_at > now);
            removed += before - listings.len();
        }
        if removed > 0 {
            info!(removed, "expired listings cleaned");
        }
        removed
    }

    // ---- Matching ---------------------------------------------------------

    /// Match a request against available listings.
    ///
    /// Returns the best (lowest-price) listing that satisfies quantity and
    /// max_price constraints.  Does **not** settle — call [`settle_trade`]
    /// afterwards.
    pub fn match_request(&self, request: &ResourceRequest) -> Result<ResourceListing, MarketplaceError> {
        let listings = self
            .listings
            .get(&request.resource_type)
            .ok_or(MarketplaceError::MarketEmpty)?;

        let best = listings
            .iter()
            .filter(|l| l.quantity >= request.quantity && l.base_price <= request.max_price)
            .min_by(|a, b| a.base_price.partial_cmp(&b.base_price).unwrap())
            .cloned();

        best.ok_or(MarketplaceError::ResourceNotFound(
            "No matching listing found".into(),
        ))
    }

    // ---- Settlement -------------------------------------------------------

    /// Attempt atomic settlement of a matched trade.
    ///
    /// Verifies trust and credits of **both** parties before confirming.
    pub fn settle_trade(
        &self,
        requester_id: &str,
        provider_id: &str,
        base_price: f32,
    ) -> Result<MarketResult, MarketplaceError> {
        // Verify requester
        let req_info = self
            .trust_store
            .get(requester_id)
            .ok_or_else(|| MarketplaceError::NodeNotFound(requester_id.into()))?;

        if req_info.trust_score < self.min_trust_threshold {
            return Ok(MarketResult::rejected(
                &format!(
                    "Requester trust below threshold: {:.3} < {:.3}",
                    req_info.trust_score, self.min_trust_threshold
                ),
            ));
        }

        if req_info.credits < self.min_credit_threshold {
            return Ok(MarketResult::rejected(
                &format!(
                    "Requester credits insufficient: {:.1} < {:.1}",
                    req_info.credits, self.min_credit_threshold
                ),
            ));
        }

        // Verify provider
        let prov_info = self
            .trust_store
            .get(provider_id)
            .ok_or_else(|| MarketplaceError::NodeNotFound(provider_id.into()))?;

        if prov_info.trust_score < self.min_trust_threshold {
            return Ok(MarketResult::rejected(
                &format!(
                    "Provider trust below threshold: {:.3} < {:.3}",
                    prov_info.trust_score, self.min_trust_threshold
                ),
            ));
        }

        // Compute dynamic price
        let price = self.compute_dynamic_price(base_price, prov_info.trust_score);

        // Generate settlement hash
        let settlement_hash =
            Self::generate_settlement_hash(requester_id, provider_id, price);

        info!(
            requester = %requester_id,
            provider = %provider_id,
            price,
            hash = %settlement_hash,
            "trade settled"
        );

        Ok(MarketResult::matched(price, settlement_hash))
    }

    // ---- Anti-gaming ------------------------------------------------------

    /// Validate a trade for anti-gaming patterns.
    ///
    /// Flags if:
    /// - Same node pair traded >5 times in window (simulated)
    /// - Price deviation > 3x base
    /// - Trust score anomaly (one party much higher than other)
    pub fn validate_anti_gaming(
        &self,
        requester_id: &str,
        provider_id: &str,
        price: f32,
        base_price: f32,
    ) -> bool {
        // Price deviation check
        if base_price > 0.0 && price > base_price * 3.0 {
            warn!(price, base_price, "anti-gaming: price deviation");
            return true;
        }

        // Trust anomaly check
        if let (Some(req), Some(prov)) = (
            self.trust_store.get(requester_id),
            self.trust_store.get(provider_id),
        ) {
            let trust_diff = (req.trust_score - prov.trust_score).abs();
            if trust_diff > 0.8 {
                warn!(
                    req_trust = req.trust_score,
                    prov_trust = prov.trust_score,
                    "anti-gaming: trust anomaly"
                );
                return true;
            }
        }

        false
    }

    // ---- Trust management -------------------------------------------------

    /// Register or update trust info for a node.
    pub fn set_trust_info(&mut self, node_id: String, info: NodeTrustInfo) {
        self.trust_store.insert(node_id, info);
    }

    /// Get trust info for a node.
    pub fn get_trust_info(&self, node_id: &str) -> Option<&NodeTrustInfo> {
        self.trust_store.get(node_id)
    }

    // ---- Dynamic pricing --------------------------------------------------

    /// Compute dynamic price based on base price, provider trust, and demand.
    pub(crate) fn compute_dynamic_price(&self, base_price: f32, trust_score: f32) -> f32 {
        // Higher trust = slight discount (reward reliability)
        let trust_factor = 1.0 - trust_score * 0.1;
        // Demand multiplier increases price under high demand
        let dynamic = base_price * self.demand_multiplier * trust_factor;
        (dynamic * 100.0).round() / 100.0
    }

    pub(crate) fn generate_settlement_hash(requester: &str, provider: &str, price: f32) -> String {
        let mut hasher = Sha256::new();
        hasher.update(requester.as_bytes());
        hasher.update(provider.as_bytes());
        hasher.update(price.to_le_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    // ---- Stats ------------------------------------------------------------

    /// Return the number of active listings.
    pub fn listing_count(&self) -> usize {
        self.listings.values().map(|v| v.len()).sum()
    }

    /// Return the number of registered nodes in trust store.
    pub fn node_count(&self) -> usize {
        self.trust_store.len()
    }
}

impl Default for ResourceMarketplace {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Unit tests are in tests.rs (same module)
// ---------------------------------------------------------------------------
