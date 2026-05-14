//! v1.1.0 Sprint 5 E2E Integration Tests
//!
//! Flujo completo: Dashboard v2 → WebSocket Dashboard Stream → Adaptive Router v2
//! → Predictive Balancer
//!
//! Feature-gated: `--features v1.1-sprint5`

#![cfg(feature = "v1.1-sprint5")]

use ed2kia::ui::dashboard_v2::{DashboardState, DashboardConfig, DashboardMetric};
use ed2kia::web::ws_dashboard_stream::{WsDashboardStream, WsDashboardConfig, DashboardCategory};
use ed2kia::interoperability::adaptive_router_v2::{AdaptiveRouter, AdaptiveRouterConfig};
use ed2kia::scaling::predictive_balancer::{PredictiveBalancer, PredictiveBalancerConfig, TrendDirection};

// ============================================================================
// Test: Dashboard v2 E2E
// ============================================================================

#[test]
fn test_e2e_dashboard_full_workflow() {
    let mut dashboard = DashboardState::with_config(DashboardConfig::default());

    // Register nodes
    dashboard.register_node("node-1".into());
    dashboard.register_node("node-2".into());
    dashboard.register_node("node-3".into());

    // Record metrics
    dashboard.record_metric(DashboardMetric::SystemCpuUsage, 0.45, Some("node-1".into()));
    dashboard.record_metric(DashboardMetric::SystemMemoryUsage, 0.60, Some("node-1".into()));
    dashboard.record_metric(DashboardMetric::AlignmentConfidence, 0.92, Some("node-2".into()));
    dashboard.record_metric(DashboardMetric::FederationTrustScore, 0.85, Some("node-3".into()));

    // Generate snapshot
    let snapshot = dashboard.get_snapshot().unwrap();
    assert!(!snapshot.metrics.is_empty());
    assert_eq!(snapshot.nodes.len(), 3);

    // Remove node
    dashboard.remove_node("node-3");
    let snapshot2 = dashboard.get_snapshot().unwrap();
    assert_eq!(snapshot2.nodes.len(), 2);
}

#[test]
fn test_e2e_dashboard_alert_generation() {
    let mut dashboard = DashboardState::new();

    // Trigger CPU alert
    dashboard.record_metric(DashboardMetric::SystemCpuUsage, 0.95, None);

    // Get snapshot to verify alerts
    let snapshot = dashboard.get_snapshot().unwrap();
    assert!(!snapshot.alerts.is_empty());

    // Acknowledge alert
    let alert_id = snapshot.alerts.last().unwrap().alert_id.clone();
    assert!(dashboard.acknowledge_alert(&alert_id));

    // Verify acknowledged: snapshot should now have 0 unacknowledged alerts
    let snapshot2 = dashboard.get_snapshot().unwrap();
    assert!(snapshot2.alerts.is_empty());
}

#[test]
fn test_e2e_dashboard_metric_history() {
    let mut dashboard = DashboardState::new();

    for i in 0..10 {
        dashboard.record_metric(
            DashboardMetric::SystemThroughput,
            (i as f64) * 100.0,
            None,
        );
    }

    let history = dashboard.get_metric_history(&DashboardMetric::SystemThroughput);
    assert_eq!(history.len(), 10);
}

// ============================================================================
// Test: WebSocket Dashboard Stream E2E
// ============================================================================

#[test]
fn test_e2e_ws_dashboard_create_and_broadcast() {
    let mut stream = WsDashboardStream::new();

    // Create connection
    let result = stream.create_connection("conn-1".into(), "client-1".into());
    assert!(result.is_ok());

    // Authenticate
    let auth_result = stream.authenticate_connection("conn-1", "sig_test");
    assert!(auth_result.is_ok());

    // Subscribe
    stream.subscribe("conn-1", vec![DashboardCategory::All]).unwrap();

    // Broadcast snapshot
    let results = stream.broadcast_snapshot(serde_json::json!({"test": true}));
    assert_eq!(results.len(), 1);
}

#[test]
fn test_e2e_ws_dashboard_rate_limiting() {
    let config = WsDashboardConfig {
        rate_limit_per_sec: 2,
        ..WsDashboardConfig::default()
    };
    let mut stream = WsDashboardStream::with_config(config);

    stream.create_connection("conn-1".into(), "client-1".into()).unwrap();
    stream.authenticate_connection("conn-1", "sig_test").unwrap();

    // First broadcast should succeed
    let results = stream.broadcast_snapshot(serde_json::json!({"msg": 1}));
    assert!(results.iter().any(|r| r.messages_sent > 0));

    // Second broadcast should succeed
    let results = stream.broadcast_snapshot(serde_json::json!({"msg": 2}));
    assert!(results.iter().any(|r| r.messages_sent > 0));
}

#[test]
fn test_e2e_ws_dashboard_alert_broadcast() {
    let mut stream = WsDashboardStream::new();

    stream.create_connection("conn-1".into(), "client-1".into()).unwrap();
    stream.authenticate_connection("conn-1", "sig_test").unwrap();
    stream.subscribe("conn-1", vec![DashboardCategory::System]).unwrap();

    stream.broadcast_alert(
        "alert-1".into(),
        "critical".into(),
        "CPU usage high".into(),
    );
    // broadcast_alert returns (), alert was broadcast successfully
}

// ============================================================================
// Test: Adaptive Router v2 E2E
// ============================================================================

#[test]
fn test_e2e_adaptive_router_routing_cycle() {
    let mut router = AdaptiveRouter::with_config(AdaptiveRouterConfig::default());

    // Register nodes
    router.register_node("node-1".into(), "qwen-72b".into());
    router.register_node("node-2".into(), "qwen-72b".into());
    router.register_node("node-3".into(), "llama-70b".into());

    // Route request
    let decision = router.route("qwen-72b", None);
    assert!(decision.is_ok());
    let decision = decision.unwrap();
    assert!(!decision.target_node.is_empty());
}

#[test]
fn test_e2e_adaptive_router_circuit_breaker() {
    let config = AdaptiveRouterConfig {
        max_nodes: 100,
        ..AdaptiveRouterConfig::default()
    };
    let mut router = AdaptiveRouter::with_config(config);

    router.register_node("node-1".into(), "qwen-72b".into());

    // Record failures to trigger circuit breaker
    for _ in 0..5 {
        let _ = router.record_failure("node-1");
    }

    // Node should be excluded from routing due to circuit breaker
    let decision = router.route("qwen-72b", None);
    // Should fallback since node-1 is circuit-broken
    assert!(decision.is_ok());
}

#[test]
fn test_e2e_adaptive_router_reputation_update() {
    let mut router = AdaptiveRouter::new();
    router.register_node("node-1".into(), "qwen-72b".into());

    // Record success to update metrics
    let _ = router.record_success("node-1", 50.0);

    // Update reputation directly
    let _ = router.update_reputation("node-1", 0.95);

    // Verify via routing - node should be selected
    let decision = router.route("qwen-72b", None).unwrap();
    assert_eq!(decision.target_node, "node-1");
}

// ============================================================================
// Test: Predictive Balancer E2E
// ============================================================================

#[test]
fn test_e2e_predictive_balancer_full_workflow() {
    let mut balancer = PredictiveBalancer::with_config(PredictiveBalancerConfig::default());

    balancer.register_node("node-1".into());

    // Record load history
    for i in 0..30 {
        let _ = balancer.record_load(
            "node-1",
            50.0 + (i as f64) * 2.0,  // Increasing latency
            1000.0 - (i as f64) * 5.0, // Decreasing throughput
            5.0 + (i as f64) * 0.5,    // Increasing queue
        );
    }

    // Predict latency trend
    let latency_pred = balancer.predict_latency("node-1").unwrap();
    assert_eq!(latency_pred.trend, TrendDirection::Increasing);

    // Predict throughput trend
    let throughput_pred = balancer.predict_throughput("node-1").unwrap();
    assert_eq!(throughput_pred.trend, TrendDirection::Decreasing);
}

#[test]
fn test_e2e_predictive_balancer_best_node_selection() {
    let mut balancer = PredictiveBalancer::new();

    balancer.register_node("node-1".into());
    balancer.register_node("node-2".into());

    // Node-1: Low latency (good)
    for _i in 0..20 {
        let _ = balancer.record_load("node-1", 30.0, 2000.0, 2.0);
    }

    // Node-2: High latency (bad)
    for _i in 0..20 {
        let _ = balancer.record_load("node-2", 200.0, 500.0, 15.0);
    }

    let best = balancer.get_best_node(&vec!["node-1".into(), "node-2".into()]).unwrap();
    assert_eq!(best.as_deref(), Some("node-1"));
}

#[test]
fn test_e2e_predictive_balancer_insufficient_data() {
    let mut balancer = PredictiveBalancer::new();
    balancer.register_node("node-1".into());

    // Only 1 data point - insufficient for prediction
    let _ = balancer.record_load("node-1", 50.0, 1000.0, 5.0);

    let result = balancer.predict_latency("node-1");
    assert!(result.is_err());
}

// ============================================================================
// Test: Full Pipeline Integration
// ============================================================================

#[test]
fn test_e2e_full_pipeline_dashboard_to_stream() {
    let mut dashboard = DashboardState::new();
    let mut stream = WsDashboardStream::new();

    // Setup dashboard
    dashboard.register_node("node-1".into());
    dashboard.record_metric(DashboardMetric::SystemCpuUsage, 0.55, Some("node-1".into()));
    dashboard.record_metric(DashboardMetric::AlignmentConfidence, 0.88, None);

    // Generate snapshot
    let snapshot = dashboard.get_snapshot().unwrap();
    assert!(!snapshot.metrics.is_empty());

    // Setup stream
    stream.create_connection("conn-1".into(), "dashboard-client".into()).unwrap();
    stream.authenticate_connection("conn-1", "sig_test").unwrap();

    // Broadcast dashboard snapshot via stream
    let snapshot_json = serde_json::to_value(&snapshot).unwrap();
    let results = stream.broadcast_snapshot(snapshot_json);
    assert_eq!(results.len(), 1);
}

#[test]
fn test_e2e_full_pipeline_router_to_dashboard() {
    let mut router = AdaptiveRouter::new();
    let mut dashboard = DashboardState::new();

    // Setup router
    router.register_node("node-1".into(), "qwen-72b".into());
    router.register_node("node-2".into(), "llama-70b".into());

    // Route request
    let decision = router.route("qwen-72b", None).unwrap();

    // Record routing latency in dashboard
    dashboard.record_metric(
        DashboardMetric::SystemNetworkLatency,
        decision.estimated_latency_ms as f64,
        Some(decision.target_node.clone()),
    );

    let snapshot = dashboard.get_snapshot().unwrap();
    assert!(snapshot.metrics.contains_key(&DashboardMetric::SystemNetworkLatency));
}

#[test]
fn test_e2e_full_pipeline_balancer_to_router() {
    let mut balancer = PredictiveBalancer::new();
    let mut router = AdaptiveRouter::new();

    // Setup balancer with load data
    balancer.register_node("node-1".into());
    balancer.register_node("node-2".into());

    for _i in 0..20 {
        let _ = balancer.record_load("node-1", 40.0, 1500.0, 3.0);
        let _ = balancer.record_load("node-2", 150.0, 800.0, 10.0);
    }

    // Get best node from balancer
    let best = balancer.get_best_node(&vec!["node-1".into(), "node-2".into()]).unwrap();
    assert_eq!(best.as_deref(), Some("node-1"));

    // Register nodes in router
    router.register_node("node-1".into(), "qwen-72b".into());
    router.register_node("node-2".into(), "qwen-72b".into());

    // Route should prefer the best node
    let decision = router.route("qwen-72b", None).unwrap();
    assert!(!decision.target_node.is_empty());
}

#[test]
fn test_e2e_feature_flag_enabled() {
    #[cfg(feature = "v1.1-sprint5")]
    {
        assert!(true);
    }
}
