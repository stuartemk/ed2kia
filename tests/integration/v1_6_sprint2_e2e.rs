//! v1.6.0 Sprint 2 E2E Integration Tests
//!
//! Covers LP-145 (Scaling v7), LP-146 (Async ZKP v13 + Federation ZKP Bridge v6),
//! LP-147 (Dashboard v7 + WS Stream v2)

#[cfg(feature = "v1.6-sprint2")]
mod e2e {
    use std::time::Instant;

    // LP-145: Federation Scaling v7
    use ed2kia::federation::gradient_sync_v7::GradientSyncV7;
    use ed2kia::federation::scaling_v7::{ScalingV7, ScalingV7Config};

    // LP-146: Async ZKP v13 + Federation ZKP Bridge v6
    use ed2kia::bridge::federation_zkp_bridge_v6::{
        FederationZKPBridgeV6, FederationZKPBridgeV6Config,
    };
    use ed2kia::zkp::async_zkp_v13::{AsyncZKPV13, ProofPriority, ZKPV13Config};

    // LP-147: Dashboard v7 + WS Stream v2
    use ed2kia::ui::dashboard_v7::{DashboardV7, MetricV7, MetricValueV7};
    use ed2kia::web::ws_federation_stream_v2::{StreamCategory, WsFederationStreamV2};

    fn current_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    // === LP-146: Async ZKP v13 + Bridge v6 E2E ===

    #[test]
    fn test_e2e_zkp_v13_proof_lifecycle() {
        let mut engine = AsyncZKPV13::new(ZKPV13Config {
            max_batch_size: 10,
            max_pending_proofs: 20,
            ..ZKPV13Config::default()
        });

        // Register federations
        engine
            .register_federation("fed_alpha".to_string(), 0.95)
            .unwrap();
        engine
            .register_federation("fed_beta".to_string(), 0.85)
            .unwrap();

        // Submit proofs
        let ts = current_ms();
        engine
            .submit_proof(
                "p1".to_string(),
                ProofPriority::Critical,
                ts,
                "fed_alpha".to_string(),
            )
            .unwrap();
        engine
            .submit_proof(
                "p2".to_string(),
                ProofPriority::Normal,
                ts,
                "fed_alpha".to_string(),
            )
            .unwrap();
        engine
            .submit_proof(
                "p3".to_string(),
                ProofPriority::Low,
                ts,
                "fed_beta".to_string(),
            )
            .unwrap();

        // Create and populate batch — assign_proof_to_batch pops highest-priority proof
        let batch_id = engine.create_batch(ts);
        engine.assign_proof_to_batch(&batch_id).unwrap(); // pops Critical (p1)
        engine.assign_proof_to_batch(&batch_id).unwrap(); // pops Normal (p2)

        // Complete batch with timestamp
        engine.complete_batch(&batch_id, ts + 100).unwrap();

        // Verify proof — p1 is in the batch because Critical has highest priority
        let verified = engine.verify_proof("p1", ts + 200).unwrap();
        assert!(verified);
    }

    #[test]
    fn test_e2e_bridge_v6_cross_model_routing() {
        let mut bridge = FederationZKPBridgeV6::new(FederationZKPBridgeV6Config {
            max_proofs_in_flight: 100,
            consensus_threshold: 0.6,
            fallback_timeout_ms: 500,
            ..FederationZKPBridgeV6Config::default()
        });

        // Register federations
        bridge
            .register_federation("fed_src".to_string(), 0.9, 100.0)
            .unwrap();
        bridge
            .register_federation("fed_dst".to_string(), 0.85, 80.0)
            .unwrap();
        bridge
            .register_federation("fed_relay".to_string(), 0.8, 60.0)
            .unwrap();

        let ts = current_ms();

        // Submit proof for cross-model routing
        bridge
            .submit_proof(
                "proof_1".to_string(),
                "fed_src".to_string(),
                "fed_dst".to_string(),
                "hash_abc123".to_string(),
                ts,
            )
            .unwrap();

        // Select best federation (Option<&str> exclude)
        let best = bridge.select_best_federation(Some("fed_relay"));
        assert!(best.is_some());
        assert!(!best.unwrap().is_empty());

        // Verify proof
        let verified = bridge.verify_proof("proof_1", ts + 100).unwrap();
        assert!(verified);
    }

    #[test]
    fn test_e2e_bridge_v6_fallback_verification() {
        let mut bridge = FederationZKPBridgeV6::new(FederationZKPBridgeV6Config {
            fallback_timeout_ms: 100,
            ..FederationZKPBridgeV6Config::default()
        });

        bridge
            .register_federation("fed_a".to_string(), 0.9, 100.0)
            .unwrap();
        bridge
            .register_federation("fed_b".to_string(), 0.85, 80.0)
            .unwrap();

        let ts = current_ms();
        bridge
            .submit_proof(
                "proof_fallback".to_string(),
                "fed_a".to_string(),
                "fed_b".to_string(),
                "hash_xyz".to_string(),
                ts,
            )
            .unwrap();

        // Verify after timeout triggers fallback
        let verified = bridge.verify_proof("proof_fallback", ts + 200).unwrap();
        assert!(verified);
    }

    #[test]
    fn test_e2e_zkp_bridge_integration() {
        // Full integration: ZKP v13 generates proofs, Bridge v6 routes them
        let mut zkp = AsyncZKPV13::default();
        let mut bridge = FederationZKPBridgeV6::default();

        let ts = current_ms();

        // Setup ZKP
        zkp.register_federation("zkp_fed".to_string(), 0.95)
            .unwrap();
        zkp.submit_proof(
            "zkp_proof".to_string(),
            ProofPriority::Critical,
            ts,
            "zkp_fed".to_string(),
        )
        .unwrap();

        // Setup Bridge
        bridge
            .register_federation("zkp_fed".to_string(), 0.95, 100.0)
            .unwrap();
        bridge
            .register_federation("target_fed".to_string(), 0.9, 80.0)
            .unwrap();

        // Route proof through bridge
        bridge
            .submit_proof(
                "routed_proof".to_string(),
                "zkp_fed".to_string(),
                "target_fed".to_string(),
                "payload_hash".to_string(),
                ts,
            )
            .unwrap();

        // Verify through bridge
        let verified = bridge.verify_proof("routed_proof", ts + 50).unwrap();
        assert!(verified);
    }

    // === LP-147: Dashboard v7 + WS Stream v2 E2E ===

    #[test]
    fn test_e2e_dashboard_v7_snapshot_generation() {
        let mut dashboard = DashboardV7::new();

        // Simulate scaling v7 update
        let mut scaling_summary = ed2kia::ui::dashboard_v7::ScalingV7Summary::default();
        scaling_summary.nodes_active = 50;
        scaling_summary.shards_active = 10;
        scaling_summary.partition_health = 0.99;
        scaling_summary.predictive_load = 0.65;
        scaling_summary.gradient_alignment = 0.88;
        dashboard.update_scaling_v7(scaling_summary);

        // Simulate ZKP v13 update
        let mut zkp_summary = ed2kia::ui::dashboard_v7::ZkpV13Summary::default();
        zkp_summary.proofs_submitted = 1000;
        zkp_summary.proofs_verified = 950;
        zkp_summary.fallback_rate = 0.12;
        dashboard.update_zkp_v13(zkp_summary);

        // Simulate Bridge v6 update
        let mut bridge_summary = ed2kia::ui::dashboard_v7::BridgeV6Summary::default();
        bridge_summary.proofs_routed = 500;
        bridge_summary.proofs_verified = 480;
        bridge_summary.avg_credibility = 0.88;
        dashboard.update_bridge_v6(bridge_summary);

        // Generate snapshot
        let snapshot = dashboard.generate_snapshot();
        assert_eq!(snapshot.scaling_v7.nodes_active, 50);
        assert_eq!(snapshot.zkp_v13.proofs_submitted, 1000);
        assert_eq!(snapshot.bridge_v6.proofs_routed, 500);
        assert!(snapshot.alerts.is_empty()); // All healthy
    }

    #[test]
    fn test_e2e_dashboard_v7_alert_generation() {
        let mut dashboard = DashboardV7::new();

        // Trigger partition health alert
        let mut scaling_summary = ed2kia::ui::dashboard_v7::ScalingV7Summary::default();
        scaling_summary.partition_health = 0.92; // Below 95% threshold
        dashboard.update_scaling_v7(scaling_summary);

        // Trigger ZKP fallback alert
        let mut zkp_summary = ed2kia::ui::dashboard_v7::ZkpV13Summary::default();
        zkp_summary.fallback_rate = 0.4; // Above 30% threshold
        dashboard.update_zkp_v13(zkp_summary);

        // Trigger bridge credibility alert
        let mut bridge_summary = ed2kia::ui::dashboard_v7::BridgeV6Summary::default();
        bridge_summary.avg_credibility = 0.5; // Below 60% threshold
        dashboard.update_bridge_v6(bridge_summary);

        let snapshot = dashboard.generate_snapshot();
        assert!(snapshot.alerts.len() >= 3);
    }

    #[test]
    fn test_e2e_ws_stream_v2_lifecycle() {
        let mut stream = WsFederationStreamV2::new();
        let ts = current_ms();

        // Authenticate connection — returns connection_id: String
        let conn_id = stream
            .authenticate("client_1".to_string(), "sig_abc".to_string(), ts)
            .unwrap();
        assert!(!conn_id.is_empty());

        // Subscribe to categories
        stream
            .subscribe(&conn_id, vec![StreamCategory::Scaling, StreamCategory::Zkp])
            .unwrap();

        // Publish scaling event (current_ms, nodes_active:usize, shards_active:usize, partition_health, predictive_load, gradient_alignment)
        stream.publish_scaling(ts, 50, 10, 0.99, 0.65, 0.88);

        // Publish ZKP event (current_ms, submitted, verified, batches, fallback, avg_ms)
        stream.publish_zkp(ts, 100, 95, 5, 0.05, 120.0);

        // Get catchup
        let catchup = stream.get_catchup(0);
        assert!(catchup.len() >= 2);

        // Handle ping
        let pong = stream.handle_ping(&conn_id, ts).unwrap();
        assert_eq!(pong, ts);
    }

    #[test]
    fn test_e2e_ws_stream_v2_rate_limiting() {
        let mut stream = WsFederationStreamV2::new();
        let ts = current_ms();

        let conn_id = stream
            .authenticate("rate_client".to_string(), "sig".to_string(), ts)
            .unwrap();
        stream
            .subscribe(&conn_id, vec![StreamCategory::All])
            .unwrap();

        // Publish within rate limit
        for i in 0..10 {
            stream.publish_scaling(ts + i * 10, 50, 10, 0.99, 0.65, 0.88);
        }

        // Verify events published
        let catchup = stream.get_catchup(0);
        assert!(catchup.len() >= 10);
    }

    // === LP-145 + LP-146: Scaling v7 + ZKP v13 Integration ===

    #[test]
    fn test_e2e_scaling_zkp_integration() {
        let mut scaling = ScalingV7::default();
        let mut zkp = AsyncZKPV13::default();

        let ts = current_ms();

        // Register nodes in scaling
        scaling
            .register_node("node_1".to_string(), "model_a".to_string(), 100.0)
            .unwrap();
        scaling
            .register_node("node_2".to_string(), "model_a".to_string(), 80.0)
            .unwrap();

        // Register federations in ZKP
        zkp.register_federation("fed_a".to_string(), 0.95).unwrap();

        // Submit proof
        zkp.submit_proof(
            "scaling_proof".to_string(),
            ProofPriority::Critical,
            ts,
            "fed_a".to_string(),
        )
        .unwrap();

        // Generate scaling actions (may be empty when no scaling needed)
        let _actions = scaling.generate_actions();

        // Verify proof - need to assign to batch first
        let batch_id = zkp.create_batch(ts);
        zkp.assign_proof_to_batch(&batch_id).unwrap();
        zkp.complete_batch(&batch_id, ts + 100).unwrap();
        let verified = zkp.verify_proof("scaling_proof", ts + 50).unwrap();
        assert!(verified);
    }

    // === Full Sprint 2 E2E: All modules together ===

    #[test]
    fn test_e2e_full_sprint2_pipeline() {
        let ts = current_ms();

        // Initialize all modules
        let mut scaling = ScalingV7::default();
        let mut gradient_sync = GradientSyncV7::default();
        let mut zkp = AsyncZKPV13::default();
        let mut bridge = FederationZKPBridgeV6::default();
        let mut dashboard = DashboardV7::new();
        let mut stream = WsFederationStreamV2::new();

        // Setup Scaling v7
        scaling
            .register_node("node_1".to_string(), "model_a".to_string(), 100.0)
            .unwrap();
        scaling
            .register_node("node_2".to_string(), "model_b".to_string(), 80.0)
            .unwrap();
        scaling
            .register_shard("shard_1".to_string(), "model_a".to_string())
            .unwrap();
        scaling.assign_node_to_shard("node_1", "shard_1").unwrap();

        // Setup Gradient Sync v7
        gradient_sync
            .register_model("model_a".to_string(), vec![0.1, 0.2, 0.3])
            .unwrap();
        gradient_sync
            .register_model("model_b".to_string(), vec![0.15, 0.25, 0.35])
            .unwrap();

        // Setup ZKP v13
        zkp.register_federation("fed_scaling".to_string(), 0.95)
            .unwrap();
        zkp.submit_proof(
            "proof_1".to_string(),
            ProofPriority::Critical,
            ts,
            "fed_scaling".to_string(),
        )
        .unwrap();

        // Setup Bridge v6
        bridge
            .register_federation("fed_scaling".to_string(), 0.95, 100.0)
            .unwrap();
        bridge
            .register_federation("fed_external".to_string(), 0.9, 80.0)
            .unwrap();
        bridge
            .submit_proof(
                "bridge_proof".to_string(),
                "fed_scaling".to_string(),
                "fed_external".to_string(),
                "hash_123".to_string(),
                ts,
            )
            .unwrap();

        // Update Dashboard v7
        let mut scaling_summary = ed2kia::ui::dashboard_v7::ScalingV7Summary::default();
        scaling_summary.nodes_active = 2;
        scaling_summary.shards_active = 1;
        dashboard.update_scaling_v7(scaling_summary);

        let mut zkp_summary = ed2kia::ui::dashboard_v7::ZkpV13Summary::default();
        zkp_summary.proofs_submitted = 1;
        dashboard.update_zkp_v13(zkp_summary);

        let mut bridge_summary = ed2kia::ui::dashboard_v7::BridgeV6Summary::default();
        bridge_summary.proofs_routed = 1;
        dashboard.update_bridge_v6(bridge_summary);

        // Setup WS Stream v2
        let stream_conn = stream
            .authenticate("dashboard".to_string(), "sig".to_string(), ts)
            .unwrap();
        stream
            .subscribe(&stream_conn, vec![StreamCategory::All])
            .unwrap();

        // Publish to stream
        stream.publish_scaling(ts, 50, 10, 0.99, 0.65, 0.88);
        stream.publish_zkp(ts, 1, 0, 0, 0.0, 0.0);
        stream.publish_bridge(ts, 1, 0, 0, 0.95);

        // Validate pipeline
        let snapshot = dashboard.generate_snapshot();
        assert_eq!(snapshot.scaling_v7.nodes_active, 2);
        assert_eq!(snapshot.zkp_v13.proofs_submitted, 1);
        assert_eq!(snapshot.bridge_v6.proofs_routed, 1);

        let catchup = stream.get_catchup(0);
        assert!(catchup.len() >= 3);

        // Verify ZKP proof - assign to batch first
        let batch_id = zkp.create_batch(ts);
        zkp.assign_proof_to_batch(&batch_id).unwrap();
        zkp.complete_batch(&batch_id, ts + 100).unwrap();
        let verified = zkp.verify_proof("proof_1", ts + 50).unwrap();
        assert!(verified);

        // Verify bridge proof
        let bridge_verified = bridge.verify_proof("bridge_proof", ts + 50).unwrap();
        assert!(bridge_verified);
    }

    // === Performance E2E Tests ===

    #[test]
    fn test_e2e_scaling_v7_100_nodes() {
        let mut scaling = ScalingV7::new(ScalingV7Config {
            max_nodes_per_shard: 500,
            ..ScalingV7Config::default()
        });

        let start = Instant::now();
        for i in 0..100 {
            scaling
                .register_node(
                    format!("node_{}", i),
                    format!("model_{}", i % 5),
                    100.0 - (i as f64 * 0.5),
                )
                .unwrap();
        }
        let elapsed = start.elapsed();

        eprintln!("test_e2e_scaling_v7_100_nodes: {:?}", elapsed);
        assert!(elapsed.as_millis() < 500); // < 500ms for 100 nodes
    }

    #[test]
    fn test_e2e_zkp_v13_200_proofs() {
        let mut engine = AsyncZKPV13::new(ZKPV13Config {
            max_pending_proofs: 500,
            max_batch_size: 50,
            ..ZKPV13Config::default()
        });

        engine
            .register_federation("fed_perf".to_string(), 0.95)
            .unwrap();

        let ts = current_ms();
        let start = Instant::now();
        for i in 0..200 {
            engine
                .submit_proof(
                    format!("p_{}", i),
                    ProofPriority::Normal,
                    ts,
                    "fed_perf".to_string(),
                )
                .unwrap();
        }
        let elapsed = start.elapsed();

        eprintln!("test_e2e_zkp_v13_200_proofs: {:?}", elapsed);
        assert!(elapsed.as_millis() < 500); // < 500ms for 200 proofs
    }

    #[test]
    fn test_e2e_bridge_v6_100_routes() {
        let mut bridge = FederationZKPBridgeV6::new(FederationZKPBridgeV6Config {
            max_proofs_in_flight: 500,
            ..FederationZKPBridgeV6Config::default()
        });

        bridge
            .register_federation("fed_src".to_string(), 0.95, 200.0)
            .unwrap();
        bridge
            .register_federation("fed_dst".to_string(), 0.9, 150.0)
            .unwrap();

        let ts = current_ms();
        let start = Instant::now();
        for i in 0..100 {
            bridge
                .submit_proof(
                    format!("proof_{}", i),
                    "fed_src".to_string(),
                    "fed_dst".to_string(),
                    format!("hash_{}", i),
                    ts,
                )
                .unwrap();
        }
        let elapsed = start.elapsed();

        eprintln!("test_e2e_bridge_v6_100_routes: {:?}", elapsed);
        assert!(elapsed.as_millis() < 500); // < 500ms for 100 routes
    }

    #[test]
    fn test_e2e_dashboard_v7_1000_metrics() {
        let mut dashboard = DashboardV7::new();

        let start = Instant::now();
        for i in 0..1000 {
            let metric = MetricValueV7::new(
                MetricV7::ScalingV7PartitionHealth,
                0.95 + (i as f64 * 0.00001),
                None,
            );
            dashboard.record_metric(metric);
        }
        let elapsed = start.elapsed();

        eprintln!("test_e2e_dashboard_v7_1000_metrics: {:?}", elapsed);
        assert!(elapsed.as_millis() < 200); // < 200ms for 1000 metrics
    }

    #[test]
    fn test_e2e_ws_stream_v2_50_connections() {
        let mut stream = WsFederationStreamV2::new();
        let ts = current_ms();

        let start = Instant::now();
        for i in 0..50 {
            let conn_id = stream
                .authenticate(format!("client_{}", i), format!("sig_{}", i), ts)
                .unwrap();
            stream
                .subscribe(&conn_id, vec![StreamCategory::All])
                .unwrap();
        }
        let elapsed = start.elapsed();

        eprintln!("test_e2e_ws_stream_v2_50_connections: {:?}", elapsed);
        assert!(elapsed.as_millis() < 200); // < 200ms for 50 connections
    }
}
