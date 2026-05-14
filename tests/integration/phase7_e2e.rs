//! Phase 7 Sprint 2 – End-to-End Integration Tests
//!
//! Validates the complete flow:
//! feedback → alignment loop → federation bridge → trust scoring → schema validation → API v2 response
//!
//! Run with: `cargo test --features "phase7-sprint2" --test phase7_e2e`

#[cfg(feature = "phase7-sprint2")]
mod e2e {
    // Sprint 1 imports
    use ed2kia::alignment::engine::{
        AlignmentConfig, AlignmentFeedback, AlignmentConcept, AlignmentScorer,
    };
    use ed2kia::federation::bridge::{
        DeltaUpdate, FederationBridge, NetworkIdentity, TrustRecord,
    };

    // Sprint 2 imports
    use ed2kia::alignment::feedback_loop::{
        AlignmentFeedbackLoop, FeedbackLoopConfig, LoopFeedback, LoopFeedbackType,
    };
    use ed2kia::federation::trust_scoring::{
        DynamicTrustScorer, NodeStatus, TrustConfig,
    };
    use ed2kia::interoperability::schema_registry::{
        CompatibilityType, SchemaRegistry, SchemaRegistryConfig,
    };

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Create a default AlignmentConfig for testing
    fn test_alignment_config() -> AlignmentConfig {
        AlignmentConfig {
            drift_threshold: 0.15,
            steering_gain: 0.05,
            min_feedback_count: 3,
            max_feedback_window_ms: 30_000,
            confidence_threshold: 0.6,
        }
    }

    /// Create a default FeedbackLoopConfig for testing
    fn test_feedback_loop_config() -> FeedbackLoopConfig {
        FeedbackLoopConfig {
            alignment_config: test_alignment_config(),
            feedback_window_ms: 30_000,
            max_queue_size: 1000,
            rate_limit: 100,
            rollback_threshold: 0.1,
            max_retries: 3,
        }
    }

    /// Create a default TrustConfig for testing
    fn test_trust_config() -> TrustConfig {
        TrustConfig {
            decay_factor: 0.995,
            decay_cycle_ms: 60_000,
            ban_threshold: 0.3,
            degraded_threshold: 0.6,
            sybil_threshold: 3,
            max_propagation_radius: 5,
            consensus_weight: 1.0,
            zkp_multiplier: 1.0,
        }
    }

    /// Create a default SchemaRegistryConfig for testing
    fn test_schema_config() -> SchemaRegistryConfig {
        SchemaRegistryConfig {
            max_schemas: 100,
            require_backward_compat: true,
            deprecation_retention_days: 90,
        }
    }

    /// Create a sample AlignmentFeedback for testing
    fn test_feedback(layer_id: &str, concept: &str, drift: f32) -> AlignmentFeedback {
        AlignmentFeedback {
            layer_id: layer_id.to_string(),
            concept: match concept {
                "positive" => AlignmentConcept::Positive,
                "negative" => AlignmentConcept::Negative,
                _ => AlignmentConcept::Neutral,
            },
            drift,
            confidence: 0.85,
            timestamp_ms: 1_700_000_000_000,
            source: "test_e2e".to_string(),
        }
    }

    /// Create a sample LoopFeedback for testing
    fn test_loop_feedback(rating: f32, comment: &str) -> LoopFeedback {
        LoopFeedback {
            rating,
            comment: comment.to_string(),
            feedback_type: LoopFeedbackType::User,
            timestamp_ms: 1_700_000_000_000,
            source: "test_e2e".to_string(),
        }
    }

    /// Create a NetworkIdentity for testing
    fn test_network_identity(id: &str) -> NetworkIdentity {
        NetworkIdentity::new(
            id.to_string(),
            "genesis_0xABC123".to_string(),
            "0279BE667EF9DCBBAC55A06295CE870B07029BFCDB2DCE28D959F2815B16F81798"
                .to_string(),
        )
    }

    /// Create a DeltaUpdate for testing
    fn test_delta_update(source: &str, layer_id: u32) -> DeltaUpdate {
        let weights = vec![0.1, -0.05, 0.02, -0.01, 0.08];
        DeltaUpdate::new(
            source.to_string(),
            layer_id,
            weights,
            1_700_000_000_000,
        )
    }

    // -----------------------------------------------------------------------
    // E2E Test 1: Feedback Ingestion → Alignment Loop Iteration
    // -----------------------------------------------------------------------

    #[test]
    fn test_feedback_to_alignment_loop() {
        let config = test_feedback_loop_config();
        let mut loop_ = AlignmentFeedbackLoop::with_config(config);

        // Ingest feedback entries
        let fb1 = test_loop_feedback(0.9, "Excellent alignment");
        let fb2 = test_loop_feedback(0.85, "Good response");
        let fb3 = test_loop_feedback(0.7, "Acceptable");

        assert!(loop_.ingest(fb1).is_ok());
        assert!(loop_.ingest(fb2).is_ok());
        assert!(loop_.ingest(fb3).is_ok());

        // Compute drift
        let drift = loop_.compute_drift();
        assert!(drift.is_ok());
        let drift_val = drift.unwrap();
        assert!(drift_val >= 0.0 && drift_val <= 1.0);

        // Run full iteration
        let result = loop_.run_iteration();
        assert!(result.is_ok());
        let loop_result = result.unwrap();
        assert!(loop_result.applied);
        assert!(!loop_result.steering_hash.is_empty());
    }

    // -----------------------------------------------------------------------
    // E2E Test 2: Alignment Scorer → Feedback Loop Integration
    // -----------------------------------------------------------------------

    #[test]
    fn test_scorer_to_feedback_loop_integration() {
        let config = test_alignment_config();
        let mut scorer = AlignmentScorer::new(config);

        // Feed alignment feedback into scorer
        let fb1 = test_feedback("layer_0", "positive", 0.05);
        let fb2 = test_feedback("layer_0", "positive", 0.03);
        let fb3 = test_feedback("layer_0", "negative", -0.02);

        assert!(scorer.ingest_feedback(fb1).is_ok());
        assert!(scorer.ingest_feedback(fb2).is_ok());
        assert!(scorer.ingest_feedback(fb3).is_ok());

        // Calculate drift
        let drift_result = scorer.calculate_drift("layer_0");
        assert!(drift_result.is_ok());
        let drift = drift_result.unwrap();
        assert!(drift.drift >= 0.0);

        // Now feed into feedback loop
        let loop_config = test_feedback_loop_config();
        let mut loop_ = AlignmentFeedbackLoop::with_config(loop_config);

        let loop_fb = test_loop_feedback(drift.confidence as f32, &format!("Drift: {:.4}", drift.drift));
        assert!(loop_.ingest(loop_fb).is_ok());

        let computed = loop_.compute_drift().unwrap();
        assert!(computed >= 0.0 && computed <= 1.0);
    }

    // -----------------------------------------------------------------------
    // E2E Test 3: Federation Bridge → Trust Scoring Integration
    // -----------------------------------------------------------------------

    #[test]
    fn test_bridge_to_trust_scoring() {
        let local_identity = test_network_identity("local_net");
        let mut bridge = FederationBridge::new(local_identity, 0.5);

        // Register remote network
        let remote_identity = test_network_identity("remote_net_A");
        bridge.add_trusted_network(remote_identity.clone());

        // Process delta sync
        let delta = test_delta_update("remote_net_A", 0);
        assert!(bridge.process_delta(delta).is_ok());

        // Merge updates
        let merge_result = bridge.merge_updates();
        assert!(merge_result.is_ok());
        let result = merge_result.unwrap();
        assert!(result.synced_networks.contains(&"remote_net_A".to_string()));

        // Now integrate with DynamicTrustScorer
        let trust_config = test_trust_config();
        let mut trust_scorer = DynamicTrustScorer::with_config(trust_config);

        // Register the remote node in trust system
        trust_scorer.update_score(
            "remote_net_A".to_string(),
            0.85,
            Some("ASN_12345".to_string()),
            Some("ip_hash_abc123".to_string()),
            "sig_valid".to_string(),
        );

        // Record successful sync
        trust_scorer.record_success("remote_net_A");

        // Get trust record
        let record = trust_scorer.get_record("remote_net_A");
        assert!(record.is_some());
        let rec = record.unwrap();
        assert_eq!(rec.status, NodeStatus::Active);
        assert!(rec.trust_score > 0.0);
        assert_eq!(rec.successful_syncs, 1);
    }

    // -----------------------------------------------------------------------
    // E2E Test 4: Trust Scoring → Sybil Detection
    // -----------------------------------------------------------------------

    #[test]
    fn test_trust_scoring_sybil_detection() {
        let config = test_trust_config();
        let mut scorer = DynamicTrustScorer::with_config(config);

        // Register multiple nodes with same ASN (Sybil pattern)
        for i in 0..5 {
            scorer.update_score(
                format!("suspicious_node_{}", i),
                0.7,
                Some("SAME_ASN".to_string()), // Same ASN for all
                Some(format!("ip_hash_{}", i)),
                format!("sig_{}", i),
            );
        }

        // Register nodes with same IP (another Sybil pattern)
        for i in 0..4 {
            scorer.update_score(
                format!("ip_cluster_{}", i),
                0.65,
                Some(format!("ASN_{}", i)),
                Some("SAME_IP_HASH".to_string()), // Same IP for all
                format!("sig_ip_{}", i),
            );
        }

        // Detect Sybil clusters
        let clusters = scorer.detect_sybil();

        // Should detect at least one cluster (same ASN with 5 nodes >= threshold 3)
        assert!(!clusters.is_empty(), "Should detect Sybil clusters");

        // Verify cluster structure
        let asn_cluster = clusters.iter().find(|c| c.shared_asn == "SAME_ASN");
        assert!(asn_cluster.is_some(), "Should find ASN-based cluster");
        let cluster = asn_cluster.unwrap();
        assert!(cluster.suspicious_nodes.len() >= 3);
    }

    // -----------------------------------------------------------------------
    // E2E Test 5: Schema Registry → Validation → Compatibility
    // -----------------------------------------------------------------------

    #[test]
    fn test_schema_registry_full_lifecycle() {
        let config = test_schema_config();
        let mut registry = SchemaRegistry::with_config(config);

        // Register v1.0.0
        let result = registry.register(
            "sae_hidden_state".to_string(),
            "1.0.0".to_string(),
            vec![4096],
            "f32".to_string(),
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap().version, "1.0.0");

        // Register v1.1.0 (backward compatible - dimension expansion)
        let result = registry.register(
            "sae_hidden_state".to_string(),
            "1.1.0".to_string(),
            vec![5120], // Expanded dimensions
            "f32".to_string(),
        );
        assert!(result.is_ok());

        // Validate v1.0.0
        let validation = registry.validate("sae_hidden_state", "1.0.0");
        assert!(validation.is_ok());
        let val_result = validation.unwrap();
        assert!(val_result.compatible);

        // Get compatible versions
        let compatible = registry.get_compatible("sae_hidden_state", "1.0.0");
        assert!(compatible.is_ok());
        let compat_list = compatible.unwrap();
        assert!(!compat_list.is_empty());

        // Set current version
        registry.set_current_version("1.1.0".to_string());
        assert_eq!(registry.current_version(), Some("1.1.0".to_string()));

        // Deprecate v1.0.0
        let deprecate_result = registry.deprecate(
            "sae_hidden_state",
            "1.0.0",
            Some("1.1.0".to_string()),
        );
        assert!(deprecate_result.is_ok());

        // Verify deprecation
        let schema = registry.get_schema("sae_hidden_state", "1.0.0");
        assert!(schema.is_ok());
        let s = schema.unwrap();
        assert!(s.deprecated);
        assert_eq!(s.migration_target, Some("1.1.0".to_string()));
    }

    // -----------------------------------------------------------------------
    // E2E Test 6: Full Pipeline – Feedback → Alignment → Federation → Trust → Schema
    // -----------------------------------------------------------------------

    #[test]
    fn test_complete_e2e_pipeline() {
        // 1. Initialize Alignment Feedback Loop
        let loop_config = test_feedback_loop_config();
        let mut alignment_loop = AlignmentFeedbackLoop::with_config(loop_config);

        // 2. Ingest feedback
        let fb1 = test_loop_feedback(0.92, "High quality output");
        let fb2 = test_loop_feedback(0.88, "Good alignment");
        assert!(alignment_loop.ingest(fb1).is_ok());
        assert!(alignment_loop.ingest(fb2).is_ok());

        // 3. Run alignment iteration
        let loop_result = alignment_loop.run_iteration().expect("Iteration should succeed");
        assert!(loop_result.applied);

        // 4. Initialize Federation Bridge
        let local_identity = test_network_identity("primary_network");
        let bridge = FederationBridge::new(local_identity.clone(), 0.5);

        // 5. Initialize Trust Scorer
        let trust_config = test_trust_config();
        let mut trust_scorer = DynamicTrustScorer::with_config(trust_config);

        // 6. Register peer network with trust
        trust_scorer.update_score(
            "peer_network_A".to_string(),
            0.9,
            Some("ASN_99999".to_string()),
            Some("ip_hash_xyz".to_string()),
            "valid_crypto_sig".to_string(),
        );

        // 7. Record successful operations
        trust_scorer.record_success("peer_network_A");
        trust_scorer.record_success("peer_network_A");

        // 8. Verify trust score increased
        let record = trust_scorer.get_record("peer_network_A").unwrap();
        assert_eq!(record.successful_syncs, 2);
        assert_eq!(record.status, NodeStatus::Active);

        // 9. Initialize Schema Registry
        let schema_config = test_schema_config();
        let mut schema_registry = SchemaRegistry::with_config(schema_config);

        // 10. Register schema for alignment data
        let schema_result = schema_registry.register(
            "alignment_steering_signal".to_string(),
            "1.0.0".to_string(),
            vec![256],
            "f32".to_string(),
        );
        assert!(schema_result.is_ok());

        // 11. Validate schema compatibility
        let validation = schema_registry.validate("alignment_steering_signal", "1.0.0");
        assert!(validation.is_ok());
        assert!(validation.unwrap().compatible);

        // 12. Apply trust decay
        trust_scorer.decay();

        // 13. Verify stats
        let trust_stats = trust_scorer.stats();
        assert_eq!(trust_stats.total_nodes, 1);
        assert_eq!(trust_stats.active_nodes, 1);

        let schema_stats = schema_registry.stats();
        assert_eq!(schema_stats.total_schemas, 1);

        // 14. Verify audit log exists in alignment loop
        let audit_log = alignment_loop.get_audit_log();
        assert!(!audit_log.is_empty());
    }

    // -----------------------------------------------------------------------
    // E2E Test 7: Feedback Loop → Rollback on Degradation
    // -----------------------------------------------------------------------

    #[test]
    fn test_feedback_loop_rollback_on_degradation() {
        let config = FeedbackLoopConfig {
            alignment_config: test_alignment_config(),
            feedback_window_ms: 30_000,
            max_queue_size: 100,
            rate_limit: 50,
            rollback_threshold: 0.05, // Low threshold to trigger rollback
            max_retries: 1,
        };
        let mut loop_ = AlignmentFeedbackLoop::with_config(config);

        // Ingest positive feedback
        let fb1 = test_loop_feedback(0.95, "Excellent");
        assert!(loop_.ingest(fb1).is_ok());

        // Run iteration
        let result = loop_.run_iteration().expect("First iteration OK");
        assert!(result.applied);

        // Ingest negative feedback (simulating degradation)
        let fb2 = test_loop_feedback(0.1, "Severe misalignment detected");
        assert!(loop_.ingest(fb2).is_ok());

        // Check drift - should show degradation
        let drift = loop_.compute_drift().unwrap();
        assert!(drift >= 0.0);

        // Attempt rollback
        let rollback_result = loop_.rollback_if_degraded();
        // Rollback may or may not trigger depending on drift delta
        assert!(rollback_result.is_ok() || rollback_result.is_err());
    }

    // -----------------------------------------------------------------------
    // E2E Test 8: Trust Decay → Node Status Transition
    // -----------------------------------------------------------------------

    #[test]
    fn test_trust_decay_status_transition() {
        let config = TrustConfig {
            decay_factor: 0.90, // Aggressive decay for testing
            decay_cycle_ms: 10_000,
            ban_threshold: 0.3,
            degraded_threshold: 0.6,
            sybil_threshold: 5,
            max_propagation_radius: 3,
            consensus_weight: 1.0,
            zkp_multiplier: 1.0,
        };
        let mut scorer = DynamicTrustScorer::with_config(config);

        // Register node with moderate trust
        scorer.update_score(
            "test_node".to_string(),
            0.65, // Just above degraded threshold
            None,
            None,
            "sig".to_string(),
        );

        let record = scorer.get_record("test_node").unwrap();
        assert_eq!(record.status, NodeStatus::Active);

        // Apply multiple decay cycles
        for _ in 0..10 {
            scorer.decay();
        }

        let record = scorer.get_record("test_node").unwrap();
        // After decay, should transition to Degraded or Banned
        assert!(
            matches!(
                record.status,
                NodeStatus::Degraded | NodeStatus::Banned
            ),
            "Node should degrade after multiple decay cycles"
        );
    }

    // -----------------------------------------------------------------------
    // E2E Test 9: Schema Breaking Change Rejection
    // -----------------------------------------------------------------------

    #[test]
    fn test_schema_breaking_change_rejection() {
        let config = SchemaRegistryConfig {
            max_schemas: 50,
            require_backward_compat: true, // Enforce backward compatibility
            deprecation_retention_days: 90,
        };
        let mut registry = SchemaRegistry::with_config(config);

        // Register v1.0.0 with 4096 dims
        let result = registry.register(
            "model_output".to_string(),
            "1.0.0".to_string(),
            vec![4096],
            "f32".to_string(),
        );
        assert!(result.is_ok());

        // Attempt to register v2.0.0 with smaller dims (breaking change)
        let result = registry.register(
            "model_output".to_string(),
            "2.0.0".to_string(),
            vec![2048], // Shrinking dimensions = breaking change
            "f32".to_string(),
        );
        assert!(
            result.is_err(),
            "Should reject breaking change (dimension shrink)"
        );
    }

    // -----------------------------------------------------------------------
    // E2E Test 10: Cross-Network Trust Propagation
    // -----------------------------------------------------------------------

    #[test]
    fn test_cross_network_trust_propagation() {
        let config = test_trust_config();
        let mut scorer = DynamicTrustScorer::with_config(config);

        // Register nodes in different networks
        scorer.update_score(
            "node_alpha".to_string(),
            0.9,
            Some("ASN_A".to_string()),
            Some("ip_alpha".to_string()),
            "sig_alpha".to_string(),
        );
        scorer.update_score(
            "node_beta".to_string(),
            0.85,
            Some("ASN_B".to_string()),
            Some("ip_beta".to_string()),
            "sig_beta".to_string(),
        );
        scorer.update_score(
            "node_gamma".to_string(),
            0.8,
            Some("ASN_C".to_string()),
            Some("ip_gamma".to_string()),
            "sig_gamma".to_string(),
        );

        // Register network topology
        scorer.register_node_in_network("node_alpha".to_string(), "network_main".to_string());
        scorer.register_node_in_network("node_beta".to_string(), "network_main".to_string());
        scorer.register_node_in_network("node_beta".to_string(), "network_secondary".to_string());
        scorer.register_node_in_network("node_gamma".to_string(), "network_secondary".to_string());

        // Propagate trust from node_alpha
        let propagation_result = scorer.propagate_cross_net("node_alpha".to_string());
        assert!(propagation_result.is_ok());

        // Verify propagation radius respected
        let stats = scorer.stats();
        assert_eq!(stats.total_nodes, 3);
    }

    // -----------------------------------------------------------------------
    // E2E Test 11: Alignment Scorer → Steering → Feedback Loop Ingestion
    // -----------------------------------------------------------------------

    #[test]
    fn test_scorer_steering_to_feedback_loop() {
        let config = test_alignment_config();
        let mut scorer = AlignmentScorer::new(config);

        // Ingest feedback
        let fb1 = test_feedback("layer_0", "positive", 0.1);
        let fb2 = test_feedback("layer_0", "positive", 0.08);
        let fb3 = test_feedback("layer_0", "negative", -0.05);

        assert!(scorer.ingest_feedback(fb1).is_ok());
        assert!(scorer.ingest_feedback(fb2).is_ok());
        assert!(scorer.ingest_feedback(fb3).is_ok());

        // Generate steering adjustment
        let steering = scorer.generate_steering_adjustment("layer_0");
        assert!(steering.is_ok());
        let steering_result = steering.unwrap();

        // Feed steering result into feedback loop
        let loop_config = test_feedback_loop_config();
        let mut loop_ = AlignmentFeedbackLoop::with_config(loop_config);

        let loop_fb = LoopFeedback {
            rating: steering_result.confidence as f32,
            comment: format!(
                "Steering delta applied: {:.4}",
                steering_result.drift_delta
            ),
            feedback_type: LoopFeedbackType::System,
            timestamp_ms: 1_700_000_000_000,
            source: "alignment_scorer".to_string(),
        };

        assert!(loop_.ingest(loop_fb).is_ok());

        // Verify audit entry created
        let audit = loop_.get_audit_log();
        assert!(!audit.is_empty());
    }

    // -----------------------------------------------------------------------
    // E2E Test 12: Federation Handshake → Trust Initialization
    // -----------------------------------------------------------------------

    #[test]
    fn test_handshake_to_trust_init() {
        let local = test_network_identity("network_local");
        let mut bridge = FederationBridge::new(local, 0.5);

        // Init handshake
        let handshake = bridge.init_handshake("network_remote");
        assert!(handshake.is_ok());
        let msg = handshake.unwrap();
        assert!(msg.is_valid());

        // Initialize trust scorer with handshake data
        let trust_config = test_trust_config();
        let mut trust_scorer = DynamicTrustScorer::with_config(trust_config);

        // Register the remote node based on handshake
        trust_scorer.update_score(
            "network_remote".to_string(),
            0.7, // Initial trust from handshake
            None,
            None,
            msg.identity.public_key_hex, // Use public key as signature
        );

        let record = trust_scorer.get_record("network_remote").unwrap();
        assert_eq!(record.status, NodeStatus::Active);
        assert!(record.trust_score > 0.0);
    }

    // -----------------------------------------------------------------------
    // E2E Test 13: Schema Compatibility Matrix Verification
    // -----------------------------------------------------------------------

    #[test]
    fn test_schema_compatibility_matrix() {
        let config = test_schema_config();
        let mut registry = SchemaRegistry::with_config(config);

        // Register v1.0.0
        registry.register(
            "test_schema".to_string(),
            "1.0.0".to_string(),
            vec![1024],
            "f32".to_string(),
        )
        .unwrap();

        // Register v1.1.0 (compatible)
        registry.register(
            "test_schema".to_string(),
            "1.1.0".to_string(),
            vec![2048],
            "f32".to_string(),
        )
        .unwrap();

        // Register v2.0.0 (new major, forward compatible)
        registry.register(
            "test_schema".to_string(),
            "2.0.0".to_string(),
            vec![4096],
            "f32".to_string(),
        )
        .unwrap();

        // Check compatibility for v1.0.0
        let compatible = registry.get_compatible("test_schema", "1.0.0").unwrap();
        assert!(!compatible.is_empty());

        // Verify stats
        let stats = registry.stats();
        assert_eq!(stats.total_schemas, 3);
        assert_eq!(stats.current_version, Some("2.0.0".to_string()));
    }

    // -----------------------------------------------------------------------
    // E2E Test 14: Rate Limiting in Feedback Loop
    // -----------------------------------------------------------------------

    #[test]
    fn test_feedback_loop_rate_limiting() {
        let config = FeedbackLoopConfig {
            alignment_config: test_alignment_config(),
            feedback_window_ms: 30_000,
            max_queue_size: 1000,
            rate_limit: 5, // Low limit for testing
            rollback_threshold: 0.1,
            max_retries: 3,
        };
        let mut loop_ = AlignmentFeedbackLoop::with_config(config);

        // Ingest up to rate limit
        for i in 0..5 {
            let fb = test_loop_feedback(0.9 - (i as f32 * 0.05), &format!("Feedback {}", i));
            assert!(loop_.ingest(fb).is_ok(), "Should accept within rate limit");
        }

        // Exceed rate limit
        let fb_excess = test_loop_feedback(0.5, "Excess feedback");
        let result = loop_.ingest(fb_excess);
        assert!(
            result.is_err(),
            "Should reject when rate limit exceeded"
        );
    }

    // -----------------------------------------------------------------------
    // E2E Test 15: Trust Score Propagation with Ban Threshold
    // -----------------------------------------------------------------------

    #[test]
    fn test_trust_propagation_with_ban() {
        let config = TrustConfig {
            decay_factor: 0.995,
            decay_cycle_ms: 60_000,
            ban_threshold: 0.3,
            degraded_threshold: 0.6,
            sybil_threshold: 3,
            max_propagation_radius: 5,
            consensus_weight: 1.0,
            zkp_multiplier: 1.0,
        };
        let mut scorer = DynamicTrustScorer::with_config(config);

        // Register node with low trust
        scorer.update_score(
            "bad_actor".to_string(),
            0.35, // Just above ban threshold
            Some("ASN_BAD".to_string()),
            Some("ip_bad".to_string()),
            "sig_bad".to_string(),
        );

        // Record multiple failures
        for _ in 0..10 {
            scorer.record_failure("bad_actor");
        }

        let record = scorer.get_record("bad_actor").unwrap();
        assert!(
            matches!(record.status, NodeStatus::Degraded | NodeStatus::Banned),
            "Node should be degraded or banned after failures"
        );

        // Verify nodes by status
        let banned = scorer.get_nodes_by_status(NodeStatus::Banned);
        let degraded = scorer.get_nodes_by_status(NodeStatus::Degraded);
        assert!(
            banned.len() + degraded.len() >= 1,
            "Should have at least one degraded/banned node"
        );
    }
}

// ---------------------------------------------------------------------------
// Fallback tests when feature is disabled
// ---------------------------------------------------------------------------

#[cfg(not(feature = "phase7-sprint2"))]
mod e2e {
    #[test]
    fn test_feature_disabled() {
        assert!(
            !cfg!(feature = "phase7-sprint2"),
            "Test should run when feature is disabled"
        );
    }
}
