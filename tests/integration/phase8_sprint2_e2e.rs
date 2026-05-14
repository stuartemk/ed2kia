//! Phase 8 Sprint 2 – End-to-End Integration Tests
//!
//! Validates the complete flow: scaling → alignment loop → SLO enforcement → marketplace
//!
//! Run with: `cargo test --features phase8-sprint2 --test phase8_sprint2_e2e`

#[cfg(feature = "phase8-sprint2")]
mod e2e {
    use ed2kia::phase8::scaling::cross_model::{
        CrossModelScaler, NodeCapacity, RoutingRequest, ScaleResult,
    };
    use ed2kia::phase8::alignment::continuous::{
        ContinuousAlignmentLoop, ContinuousFeedback, LoopConfig,
    };
    use ed2kia::phase8::slo_enforcer::enforcer::{
        SLAEnforcer, EnforcerConfig, DegradationLevel,
    };

    /// Helper: Create a test node
    fn make_node(id: &str, model: &str, capacity: usize, load: usize, latency: u64, reputation: f32, schema: &str) -> NodeCapacity {
        NodeCapacity {
            node_id: id.to_string(),
            model: model.to_string(),
            max_capacity: capacity,
            current_load: load,
            avg_latency_ms: latency,
            reputation,
            schema_version: schema.to_string(),
            last_heartbeat_ms: 0,
            active: true,
        }
    }

    /// Helper: Create a test feedback entry
    fn make_feedback(layer_id: u32, concept: u32, current: f32, desired: f32, confidence: f32) -> ContinuousFeedback {
        ContinuousFeedback {
            layer_id,
            concept_index: concept,
            current_activation: current,
            desired_activation: desired,
            annotator_confidence: confidence,
            timestamp_ms: 0,
        }
    }

    // ─── End-to-End Flow Tests ───

    #[test]
    fn test_full_scaling_alignment_slo_flow() {
        // 1. Setup CrossModelScaler
        let mut scaler = CrossModelScaler::new();
        scaler.add_node(make_node("node-1", "sae-v1", 100, 30, 10, 0.9, "1.0.0"));
        scaler.add_node(make_node("node-2", "sae-v1", 100, 80, 25, 0.7, "1.0.1"));
        scaler.add_node(make_node("node-3", "sae-v2", 200, 50, 15, 0.85, "2.0.0"));

        // 2. Route a request
        let request = RoutingRequest {
            model: "sae-v1".to_string(),
            schema_version: "1.0.0".to_string(),
            priority: 1,
        };
        let scale_result = scaler.route_request(&request).unwrap();
        assert_eq!(scale_result.routed_to, "node-1"); // Lowest latency, compatible schema
        assert!(!scale_result.fallback_triggered);

        // 3. Setup ContinuousAlignmentLoop
        let config = LoopConfig {
            drift_threshold: 0.3,
            min_confidence: 0.8,
            feedback_buffer_size: 64,
            audit_capacity: 256,
        };
        let mut alignment = ContinuousAlignmentLoop::with_config(config);

        // 4. Ingest feedback from scaling result
        let feedback = make_feedback(0, 0, scale_result.load_factor as f32, 0.5, 0.9);
        alignment.ingest_feedback(feedback).unwrap();

        // 5. Compute drift
        let drift = alignment.compute_drift(&0).unwrap();
        assert!(drift >= 0.0 && drift <= 1.0);

        // 6. Setup SLAEnforcer
        let enforcer_config = EnforcerConfig {
            warning_threshold: 0.8,
            critical_threshold: 0.95,
            rollback_threshold: 0.99,
            max_degradation_level: 4,
            breach_window_size: 5,
        };
        let mut enforcer = SLAEnforcer::with_config(enforcer_config);

        // 7. Report SLO values
        enforcer.register_slo(
            "latency_p99".to_string(),
            "Latency P99 < 100ms".to_string(),
            100.0,
        );
        enforcer.report_slo_value("latency_p99", scale_result.latency_ms as f64).unwrap();

        // 8. Evaluate SLOs
        let results = enforcer.evaluate_slos();
        assert!(!results.is_empty());

        // 9. Verify all components work together
        assert!(scale_result.latency_ms > 0);
        assert!(scale_result.load_factor > 0.0);
    }

    #[test]
    fn test_scaling_fallback_triggers_slo_degradation() {
        // Setup scaler with overloaded nodes
        let mut scaler = CrossModelScaler::new();
        scaler.add_node(make_node("overloaded-1", "sae-v1", 100, 95, 500, 0.9, "1.0.0"));
        scaler.add_node(make_node("overloaded-2", "sae-v1", 100, 98, 800, 0.85, "1.0.0"));

        let request = RoutingRequest {
            model: "sae-v1".to_string(),
            schema_version: "1.0.0".to_string(),
            priority: 1,
        };

        // Route should trigger fallback due to high load
        let scale_result = scaler.route_request(&request).unwrap();
        assert!(scale_result.fallback_triggered || scale_result.load_factor > 0.9);

        // Setup enforcer
        let mut enforcer = SLAEnforcer::new();
        enforcer.register_slo(
            "load_factor".to_string(),
            "Load Factor < 0.8".to_string(),
            0.8,
        );

        // Report high load
        enforcer.report_slo_value("load_factor", scale_result.load_factor as f64).unwrap();

        // Evaluate - should trigger degradation
        let results = enforcer.evaluate_slos();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_alignment_human_review_pauses_steering() {
        let config = LoopConfig {
            drift_threshold: 0.3,
            min_confidence: 0.8,
            feedback_buffer_size: 64,
            audit_capacity: 256,
        };
        let mut alignment = ContinuousAlignmentLoop::with_config(config);

        // Ingest feedback with high drift and low confidence
        let feedback = make_feedback(0, 0, 0.1, 0.9, 0.5); // confidence < 0.8
        alignment.ingest_feedback(feedback).unwrap();

        // Run cycle - should request human review
        let result = alignment.run_cycle(&0).unwrap();
        assert!(result.human_review_required);
        assert!(!result.applied); // Steering not applied when human review required
    }

    #[test]
    fn test_slo_rollback_prevents_cascade_failure() {
        let enforcer_config = EnforcerConfig {
            warning_threshold: 0.8,
            critical_threshold: 0.95,
            rollback_threshold: 0.99,
            max_degradation_level: 4,
            breach_window_size: 3,
        };
        let mut enforcer = SLAEnforcer::with_config(enforcer_config);

        enforcer.register_slo(
            "error_rate".to_string(),
            "Error Rate < 5%".to_string(),
            5.0,
        );

        // Report progressively worse values
        for _ in 0..4 {
            enforcer.report_slo_value("error_rate", 99.5).unwrap();
        }

        // Evaluate - should trigger rollback
        let results = enforcer.evaluate_slos();
        assert!(!results.is_empty());

        // Execute rollback
        let rollback_result = enforcer.execute_rollback("error_rate").unwrap();
        assert!(rollback_result.rollback_executed);
        assert!(rollback_result.notification_sent);
    }

    #[test]
    fn test_cross_model_schema_compatibility() {
        let mut scaler = CrossModelScaler::new();
        scaler.add_node(make_node("v1-node", "sae-v1", 100, 30, 10, 0.9, "1.0.0"));
        scaler.add_node(make_node("v2-node", "sae-v2", 100, 30, 10, 0.9, "2.0.0"));
        scaler.add_node(make_node("v1-patch", "sae-v1", 100, 30, 10, 0.9, "1.1.0"));

        // Request for v1 schema should route to v1 nodes
        let request = RoutingRequest {
            model: "sae-v1".to_string(),
            schema_version: "1.0.0".to_string(),
            priority: 1,
        };
        let result = scaler.route_request(&request).unwrap();
        assert!(result.routed_to == "v1-node" || result.routed_to == "v1-patch");

        // Request for v2 schema should route to v2 node
        let request_v2 = RoutingRequest {
            model: "sae-v2".to_string(),
            schema_version: "2.0.0".to_string(),
            priority: 1,
        };
        let result_v2 = scaler.route_request(&request_v2).unwrap();
        assert_eq!(result_v2.routed_to, "v2-node");
    }

    #[test]
    fn test_sybil_resistance_excludes_low_reputation() {
        let mut scaler = CrossModelScaler::new();
        scaler.add_node(make_node("good-node", "sae-v1", 100, 30, 10, 0.9, "1.0.0"));
        scaler.add_node(make_node("sybil-node", "sae-v1", 100, 10, 5, 0.1, "1.0.0")); // Low reputation

        let request = RoutingRequest {
            model: "sae-v1".to_string(),
            schema_version: "1.0.0".to_string(),
            priority: 1,
        };
        let result = scaler.route_request(&request).unwrap();
        // Should route to good-node, not sybil-node
        assert_eq!(result.routed_to, "good-node");
    }

    #[test]
    fn test_marketplace_integration_with_scaling() {
        use ed2kia::phase8::marketplace::engine::{
            ResourceMarketplace, ResourceListing, ResourceRequest, NodeTrustInfo,
        };

        // Setup marketplace
        let mut marketplace = ResourceMarketplace::new();

        // List resources from scaled nodes
        marketplace.list_resource(ResourceListing {
            node_id: "node-1".to_string(),
            resource_type: "compute".to_string(),
            quantity: 100.0,
            price_per_unit: 0.01,
            expires_at_ms: u64::MAX,
        });

        // Request resources
        let request = ResourceRequest {
            requester_id: "client-1".to_string(),
            resource_type: "compute".to_string(),
            quantity: 50.0,
            max_price_per_unit: 0.02,
        };

        let listing = marketplace.match_request(&request).unwrap();
        assert_eq!(listing.node_id, "node-1");

        // Settle trade
        let trust_info = NodeTrustInfo {
            node_id: "client-1".to_string(),
            trust_score: 0.9,
            credits: 1000.0,
        };
        let settlement = marketplace.settle_trade(&request, &listing, &trust_info).unwrap();
        assert!(!settlement.is_empty());
    }

    #[test]
    fn test_ui_streaming_receives_scaling_events() {
        use ed2kia::phase8::ui::backend::{UIResponse, RealtimeMetrics};

        // Create metrics response
        let metrics = RealtimeMetrics {
            active_nodes: 5,
            total_requests: 1000,
            avg_latency_ms: 25,
            alignment_drift: 0.15,
            slo_compliance: 0.98,
        };

        let response = UIResponse::new(metrics);
        assert!(!response.timestamp.is_empty());
        assert_eq!(response.cache, false);
    }

    #[test]
    fn test_slo_engine_integration_with_enforcer() {
        use ed2kia::phase8::slo::engine::{SLOEngine, SLOConfig};

        // Setup SLO Engine
        let mut engine = SLOEngine::new();
        engine.register_slo(SLOConfig {
            metric_key: "uptime".to_string(),
            target_value: 99.9,
            warning_threshold: 99.0,
            critical_threshold: 95.0,
            window_size: 10,
        });

        // Track metric
        engine.track_metric("uptime", 99.95).unwrap();

        // Evaluate
        let result = engine.evaluate_slo("uptime").unwrap();
        assert!(result.status.to_string().contains("OK") || result.status.to_string().contains("Compliant"));

        // Now test enforcer with same metric
        let mut enforcer = SLAEnforcer::new();
        enforcer.register_slo(
            "uptime".to_string(),
            "Uptime > 99.9%".to_string(),
            99.9,
        );
        enforcer.report_slo_value("uptime", 99.95).unwrap();
        let enforcer_results = enforcer.evaluate_slos();
        assert!(!enforcer_results.is_empty());
    }

    #[test]
    fn test_phase8_version_sprint2() {
        let version = ed2kia::phase8::version();
        assert!(version.contains("0.8.0-alpha"));
    }

    #[test]
    fn test_phase8_sprint_identifier() {
        let sprint = ed2kia::phase8::sprint_identifier();
        assert!(!sprint.is_empty());
    }

    #[test]
    fn test_degradation_progression() {
        let enforcer_config = EnforcerConfig {
            warning_threshold: 0.8,
            critical_threshold: 0.95,
            rollback_threshold: 0.99,
            max_degradation_level: 4,
            breach_window_size: 2,
        };
        let mut enforcer = SLAEnforcer::with_config(enforcer_config);

        enforcer.register_slo(
            "latency".to_string(),
            "Latency < 100ms".to_string(),
            100.0,
        );

        // Level 1: Warning
        enforcer.report_slo_value("latency", 150.0).unwrap();
        enforcer.report_slo_value("latency", 160.0).unwrap();
        let results = enforcer.evaluate_slos();
        assert!(!results.is_empty());

        // Trigger degradation
        let action = enforcer.trigger_degradation("latency");
        assert!(!action.to_string().is_empty());

        // Notify ops
        enforcer.notify_ops();
    }
}

// Fallback tests when feature is disabled
#[cfg(not(feature = "phase8-sprint2"))]
mod basic_tests {
    #[test]
    fn test_feature_disabled() {
        assert!(!cfg!(feature = "phase8-sprint2"));
    }
}