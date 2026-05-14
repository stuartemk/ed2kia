//! Marketplace unit tests — 12+ tests covering listing, matching, settlement,
//! anti-gaming, trust thresholds, empty market, and error propagation.

use super::engine::*;

// ---- Helpers ---------------------------------------------------------------

fn make_listing(node_id: &str, rtype: &str, qty: f32, price: f32) -> ResourceListing {
    ResourceListing {
        node_id: node_id.into(),
        resource_type: rtype.into(),
        quantity: qty,
        base_price: price,
        listed_at: 1000,
        expires_at: 2000,
    }
}

fn make_request(requester: &str, rtype: &str, qty: f32, max_price: f32) -> ResourceRequest {
    ResourceRequest {
        requester_id: requester.into(),
        resource_type: rtype.into(),
        quantity: qty,
        max_price,
    }
}

fn make_trust(_node_id: &str, trust: f32, credits: f64) -> NodeTrustInfo {
    NodeTrustInfo {
        trust_score: trust,
        credits,
        is_active: true,
    }
}

// ---- Listing tests ---------------------------------------------------------

#[test]
fn test_list_resource() {
    let mut market = ResourceMarketplace::new();
    assert_eq!(market.listing_count(), 0);

    market.list_resource(make_listing("node1", "gpu", 4.0, 100.0));
    assert_eq!(market.listing_count(), 1);
}

#[test]
fn test_list_multiple_resources() {
    let mut market = ResourceMarketplace::new();
    market.list_resource(make_listing("node1", "gpu", 4.0, 100.0));
    market.list_resource(make_listing("node2", "gpu", 8.0, 150.0));
    market.list_resource(make_listing("node3", "cpu", 16.0, 50.0));
    assert_eq!(market.listing_count(), 3);
}

#[test]
fn test_cleanup_expired() {
    let mut market = ResourceMarketplace::new();
    market.list_resource(make_listing("node1", "gpu", 4.0, 100.0));
    let expired = ResourceListing {
        node_id: "node2".into(),
        resource_type: "gpu".into(),
        quantity: 2.0,
        base_price: 80.0,
        listed_at: 100,
        expires_at: 500, // expired
    };
    market.list_resource(expired);
    assert_eq!(market.listing_count(), 2);

    let removed = market.cleanup_expired(1500);
    assert_eq!(removed, 1);
    assert_eq!(market.listing_count(), 1);
}

// ---- Matching tests --------------------------------------------------------

#[test]
fn test_match_request_success() {
    let mut market = ResourceMarketplace::new();
    market.list_resource(make_listing("node1", "gpu", 4.0, 100.0));
    let request = make_request("buyer", "gpu", 2.0, 120.0);
    let matched = market.match_request(&request).unwrap();
    assert_eq!(matched.node_id, "node1");
    assert_eq!(matched.base_price, 100.0);
}

#[test]
fn test_match_request_selects_lowest_price() {
    let mut market = ResourceMarketplace::new();
    market.list_resource(make_listing("cheap", "gpu", 4.0, 80.0));
    market.list_resource(make_listing("expensive", "gpu", 4.0, 120.0));
    let request = make_request("buyer", "gpu", 4.0, 150.0);
    let matched = market.match_request(&request).unwrap();
    assert_eq!(matched.node_id, "cheap");
}

#[test]
fn test_match_request_quantity_insufficient() {
    let mut market = ResourceMarketplace::new();
    market.list_resource(make_listing("node1", "gpu", 2.0, 100.0));
    let request = make_request("buyer", "gpu", 4.0, 120.0);
    let err = market.match_request(&request).unwrap_err();
    assert!(matches!(err, MarketplaceError::ResourceNotFound(_)));
}

#[test]
fn test_match_request_exceeds_max_price() {
    let mut market = ResourceMarketplace::new();
    market.list_resource(make_listing("node1", "gpu", 4.0, 200.0));
    let request = make_request("buyer", "gpu", 4.0, 150.0);
    let err = market.match_request(&request).unwrap_err();
    assert!(matches!(err, MarketplaceError::ResourceNotFound(_)));
}

#[test]
fn test_match_request_empty_market() {
    let market = ResourceMarketplace::new();
    let request = make_request("buyer", "gpu", 4.0, 200.0);
    let err = market.match_request(&request).unwrap_err();
    assert!(matches!(err, MarketplaceError::MarketEmpty));
}

// ---- Settlement tests ------------------------------------------------------

#[test]
fn test_settle_trade_success() {
    let mut market = ResourceMarketplace::new();
    market.set_trust_info("buyer".into(), make_trust("buyer", 0.9, 50.0));
    market.set_trust_info("seller".into(), make_trust("seller", 0.85, 30.0));

    let result = market.settle_trade("buyer", "seller", 100.0).unwrap();
    assert!(result.matched);
    assert!(!result.settlement_hash.is_empty());
    assert!(!result.anti_gaming_flag);
}

#[test]
fn test_settle_trade_requester_trust_low() {
    let mut market = ResourceMarketplace::new();
    market.set_trust_info("buyer".into(), make_trust("buyer", 0.3, 50.0));
    market.set_trust_info("seller".into(), make_trust("seller", 0.85, 30.0));

    let result = market.settle_trade("buyer", "seller", 100.0).unwrap();
    assert!(!result.matched);
    assert!(result.anti_gaming_flag == false);
}

#[test]
fn test_settle_trade_requester_credits_low() {
    let mut market = ResourceMarketplace::new();
    market.set_trust_info("buyer".into(), make_trust("buyer", 0.9, 5.0));
    market.set_trust_info("seller".into(), make_trust("seller", 0.85, 30.0));

    let result = market.settle_trade("buyer", "seller", 100.0).unwrap();
    assert!(!result.matched);
}

#[test]
fn test_settle_trade_provider_not_found() {
    let mut market = ResourceMarketplace::new();
    market.set_trust_info("buyer".into(), make_trust("buyer", 0.9, 50.0));

    let err = market.settle_trade("buyer", "unknown", 100.0).unwrap_err();
    assert!(matches!(err, MarketplaceError::NodeNotFound(_)));
}

// ---- Anti-gaming tests -----------------------------------------------------

#[test]
fn test_validate_anti_gaming_price_deviation() {
    let mut market = ResourceMarketplace::new();
    market.set_trust_info("buyer".into(), make_trust("buyer", 0.9, 50.0));
    market.set_trust_info("seller".into(), make_trust("seller", 0.85, 30.0));

    let flagged = market.validate_anti_gaming("buyer", "seller", 400.0, 100.0);
    assert!(flagged);
}

#[test]
fn test_validate_anti_gaming_trust_anomaly() {
    let mut market = ResourceMarketplace::new();
    market.set_trust_info("buyer".into(), make_trust("buyer", 0.95, 50.0));
    market.set_trust_info("seller".into(), make_trust("seller", 0.1, 30.0));

    let flagged = market.validate_anti_gaming("buyer", "seller", 100.0, 100.0);
    assert!(flagged);
}

#[test]
fn test_validate_anti_gaming_clean() {
    let mut market = ResourceMarketplace::new();
    market.set_trust_info("buyer".into(), make_trust("buyer", 0.8, 50.0));
    market.set_trust_info("seller".into(), make_trust("seller", 0.75, 30.0));

    let flagged = market.validate_anti_gaming("buyer", "seller", 110.0, 100.0);
    assert!(!flagged);
}

// ---- Dynamic pricing tests -------------------------------------------------

#[test]
fn test_dynamic_price_high_trust_discount() {
    let market = ResourceMarketplace::new();
    let price_low = market.compute_dynamic_price(100.0, 0.5);
    let price_high = market.compute_dynamic_price(100.0, 0.95);
    assert!(price_high < price_low);
}

// ---- Stats tests -----------------------------------------------------------

#[test]
fn test_node_count() {
    let mut market = ResourceMarketplace::new();
    assert_eq!(market.node_count(), 0);
    market.set_trust_info("n1".into(), make_trust("n1", 0.8, 20.0));
    assert_eq!(market.node_count(), 1);
}

#[test]
fn test_settlement_hash_deterministic() {
    let h1 = ResourceMarketplace::generate_settlement_hash("a", "b", 100.0);
    let h2 = ResourceMarketplace::generate_settlement_hash("a", "b", 100.0);
    assert_eq!(h1, h2);

    let h3 = ResourceMarketplace::generate_settlement_hash("a", "b", 200.0);
    assert_ne!(h1, h3);
}

// ---- MarketResult helpers --------------------------------------------------

#[test]
fn test_market_result_matched() {
    let r = MarketResult::matched(99.5, "hash123".into());
    assert!(r.matched);
    assert_eq!(r.price, 99.5);
    assert_eq!(r.settlement_hash, "hash123");
    assert!(!r.anti_gaming_flag);
}

#[test]
fn test_market_result_rejected_gaming() {
    let r = MarketResult::rejected("Anti-gaming detected: price spike");
    assert!(!r.matched);
    assert!(r.anti_gaming_flag);
}

#[test]
fn test_market_result_rejected_other() {
    let r = MarketResult::rejected("Trust below threshold");
    assert!(!r.matched);
    assert!(!r.anti_gaming_flag);
}

// ---- Custom thresholds -----------------------------------------------------

#[test]
fn test_custom_thresholds() {
    let mut market = ResourceMarketplace::with_thresholds(0.7, 25.0);
    market.set_trust_info("buyer".into(), make_trust("buyer", 0.65, 50.0));
    market.set_trust_info("seller".into(), make_trust("seller", 0.85, 30.0));

    let result = market.settle_trade("buyer", "seller", 100.0).unwrap();
    assert!(!result.matched); // 0.65 < 0.7 threshold
}
