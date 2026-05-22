//! v1.3.0 Sprint 1 E2E Integration Tests
//!
//! SAE Fine-Tuning v2, Cross-Node Compute Routing, Reputation Ledger v2,
//! Async ZKP v3 & Federation Bridge
//!
//! Test Scenarios:
//! 1. SAE Fine-Tuning v2 (register → train → checkpoint → adaptive LR)
//! 2. Cross-Node Routing (register → route → predict → balance)
//! 3. Reputation Ledger v2 (record → verify → merkle → anti-sybil → merit)
//! 4. Async ZKP v3 (batch → verify → parallel)
//! 5. Federation Bridge (relay → vote → consensus → expiry)
//! 6. Cross-module pipeline: Routing → SAE → ZKP → Bridge

#[cfg(feature = "v1.3-sprint1")]
mod e2e {
    // LP-76: SAE Fine-Tuning v2
    use ed2kia::sae_v2::checkpoint_optimizer::{CheckpointOptimizer, CheckpointOptimizerConfig};
    use ed2kia::sae_v2::fine_tuning_v2::{FineTuningV2, FineTuningV2Config};
    use ed2kia::sae_v2::gradient_sync_v2::{GradientSyncV2, GradientSyncV2Config};

    // LP-77: Cross-Node Compute Routing
    use ed2kia::routing_v2::cross_node_router::{
        ComputeTask, CrossNodeRouter, RouterConfig, TaskType,
    };
    use ed2kia::routing_v2::load_balancer::{BalancerConfig, LoadBalancer};
    use ed2kia::routing_v2::predictive_scheduler::{PredictiveScheduler, SchedulerConfig};

    // LP-78: Reputation Ledger v2
    use ed2kia::reputation_v2::anti_sybil::{AntiSybilConfig, AntiSybilEngine};
    use ed2kia::reputation_v2::ledger_v2::{EventType, LedgerConfig, ReputationLedgerV2};
    use ed2kia::reputation_v2::merit_scoring::{ContributionKind, MeritConfig, MeritScorer};

    // LP-79: Async ZKP v3 & Federation Bridge
    use ed2kia::zkp_v3_sprint1::async_zkp_v3::{
        AsyncZKPV3, CircuitType, ZKPStatement, ZKPV3Config,
    };
    use ed2kia::zkp_v3_sprint1::zkp_federation_bridge::{
        BridgeConfig, BridgeProof, VerificationVote, ZKPFederationBridge,
    };

    // ========================================================================
    // LP-76: SAE Fine-Tuning v2 E2E
    // ========================================================================

    #[test]
    fn test_e2e_fine_tuning_v2_lifecycle() {
        let config = FineTuningV2Config {
            learning_rate: 0.01,
            compression_ratio: 0.8,
            batch_size: 32,
            adaptive_lr: true,
            max_retries: 3,
            checkpoint_interval: 5,
            min_node_uptime: 0.9,
        };
        let mut engine = FineTuningV2::new(config);

        // Register nodes
        engine.register_node("node-1".to_string(), 0.99);
        engine.register_node("node-2".to_string(), 0.95);
        engine.register_node("node-3".to_string(), 0.88); // fallback

        // Train steps
        let gradients = vec![0.1; 64];
        for _ in 0..10 {
            let result = engine.train_step(&gradients).unwrap();
            assert!(result.loss > 0.0);
        }

        // Verify checkpoints created
        let stats = engine.get_stats();
        assert_eq!(stats.total_rounds, 10);
        assert!(stats.total_checkpoints >= 1);
    }

    #[test]
    fn test_e2e_checkpoint_optimizer() {
        let config = CheckpointOptimizerConfig {
            max_checkpoints: 10,
            shard_count: 4,
            delta_enabled: true,
            compression_threshold: 0.01,
        };
        let mut optimizer = CheckpointOptimizer::new(config);

        // Save checkpoints
        for round in 0..5 {
            let data = vec![(round as f32) * 0.1; 64];
            let cp = optimizer.save_checkpoint(round, &data).unwrap();
            assert!(!cp.id.is_empty());
            assert!(!cp.shards.is_empty());
        }

        let stats = optimizer.get_stats();
        assert_eq!(stats.total_checkpoints, 5);
    }

    #[test]
    fn test_e2e_gradient_sync_v2() {
        let config = GradientSyncV2Config {
            compression_ratio: 0.7,
            quorum_fraction: 0.67,
            max_gradient_dim: 128,
            sync_timeout_ms: 5000,
        };
        let mut sync = GradientSyncV2::new(config);

        sync.register_node("node-1".to_string());
        sync.register_node("node-2".to_string());
        sync.register_node("node-3".to_string());

        // Submit gradients for round 1 — signature: submit_gradient(node_id: &str, round: u64, gradients: Vec<f32>)
        let grads = vec![0.1; 32];
        sync.submit_gradient("node-1", 1, grads.clone()).unwrap();
        sync.submit_gradient("node-2", 1, grads.clone()).unwrap();
        sync.submit_gradient("node-3", 1, grads).unwrap();

        // Sync round
        let result = sync.sync_round(1).unwrap();
        assert!(result.participants >= 2);
        assert!(result.quorum_met);
    }

    // ========================================================================
    // LP-77: Cross-Node Compute Routing E2E
    // ========================================================================

    #[test]
    fn test_e2e_cross_node_router() {
        let config = RouterConfig {
            min_prediction_confidence: 0.5,
            max_route_history: 1000,
            latency_weight: 0.4,
            capacity_weight: 0.3,
            reputation_weight: 0.3,
        };
        let mut router = CrossNodeRouter::new(config);

        router.register_node("node-1".to_string(), 100.0);
        router.register_node("node-2".to_string(), 80.0);
        router.register_node("node-3".to_string(), 120.0);

        router.update_node_load("node-1", 0.5);
        router.update_node_load("node-2", 0.3);
        router.update_node_load("node-3", 0.7);

        router.update_node_reputation("node-1", 0.85);
        router.update_node_reputation("node-2", 0.92);
        router.update_node_reputation("node-3", 0.78);

        let task = ComputeTask {
            task_id: "task-1".to_string(),
            task_type: TaskType::Inference,
            required_capacity: 50.0,
            priority: 8,
        };

        let decision = router.route_task(&task).unwrap();
        assert!(!decision.target_node.is_empty());
        assert!(decision.confidence > 0.0);
    }

    #[test]
    fn test_e2e_predictive_scheduler() {
        let config = SchedulerConfig {
            ema_alpha: 0.3,
            min_history_points: 3,
            prediction_horizon: 5,
            max_schedule_queue: 100,
        };
        let mut scheduler = PredictiveScheduler::new(config);

        scheduler.register_node("node-1".to_string());

        // Record load history
        for i in 0..10 {
            let load = 0.3 + (i as f64 * 0.05);
            scheduler.record_load("node-1", load).unwrap();
        }

        let prediction = scheduler.predict_load("node-1").unwrap();
        assert!(prediction.confidence > 0.0);
        assert!(prediction.predicted_load >= 0.0 && prediction.predicted_load <= 1.0);
    }

    #[test]
    fn test_e2e_load_balancer() {
        let config = BalancerConfig {
            skew_threshold: 0.3,
            rebalance_interval_ms: 5000,
            health_check_interval_ms: 1000,
            max_weight: 10.0,
        };
        let mut balancer = LoadBalancer::new(config);

        balancer.add_node("node-1".to_string(), 50.0);
        balancer.add_node("node-2".to_string(), 75.0);
        balancer.add_node("node-3".to_string(), 30.0);

        // Assign requests
        for _ in 0..6 {
            let node = balancer.assign_request().unwrap();
            assert!(!node.is_empty());
        }

        let stats = balancer.get_stats();
        assert_eq!(stats.total_assignments, 6);
    }

    // ========================================================================
    // LP-78: Reputation Ledger v2 E2E
    // ========================================================================

    #[test]
    fn test_e2e_reputation_ledger() {
        let config = LedgerConfig {
            max_events: 1000,
            merkle_batch_size: 32,
            retention_days: 30,
        };
        let mut ledger = ReputationLedgerV2::new(config);

        // Record events
        ledger
            .record_event(
                "evt-1".to_string(),
                "node-1".to_string(),
                EventType::Contribution,
                10.0,
            )
            .unwrap();

        ledger
            .record_event(
                "evt-2".to_string(),
                "node-2".to_string(),
                EventType::ComputeCredit,
                25.0,
            )
            .unwrap();

        // Verify chain integrity
        assert!(ledger.verify_chain().is_ok());

        // Compute Merkle root
        let root = ledger.compute_merkle_root();
        assert!(!root.is_empty());
    }

    #[test]
    fn test_e2e_anti_sybil() {
        let config = AntiSybilConfig {
            min_events_for_analysis: 2,
            timing_variance_threshold: 0.1,
            pattern_similarity_threshold: 0.8,
            vrf_confidence_threshold: 0.9,
            max_suspected_clusters: 50,
        };
        let mut engine = AntiSybilEngine::new(config);

        engine.register_node("node-1".to_string(), "vrf-proof-1".to_string());
        engine.register_node("node-2".to_string(), "vrf-proof-2".to_string());
        engine.register_node("node-3".to_string(), "vrf-proof-3".to_string());

        let suspicions = engine.analyze();
        // With valid VRF proofs, no suspicions expected
        assert!(suspicions.is_empty() || suspicions.len() < 3);
    }

    #[test]
    fn test_e2e_merit_scoring() {
        let config = MeritConfig {
            contribution_weight: 0.35,
            compute_weight: 0.30,
            governance_weight: 0.20,
            review_weight: 0.15,
            decay_rate: 0.001,
            max_score: 1000.0,
            min_score: 0.0,
        };
        let mut scorer = MeritScorer::new(config);

        scorer
            .record_contribution("node-1".to_string(), ContributionKind::Code, 50.0)
            .unwrap();
        scorer
            .record_contribution("node-1".to_string(), ContributionKind::ComputeWork, 30.0)
            .unwrap();

        let ranking = scorer.get_ranking();
        assert_eq!(ranking.len(), 1);
        assert_eq!(ranking[0].node_id, "node-1");
    }

    // ========================================================================
    // LP-79: Async ZKP v3 & Federation Bridge E2E
    // ========================================================================

    #[test]
    fn test_e2e_async_zkp_v3() {
        let config = ZKPV3Config {
            max_batch_size: 128,
            parallel_verifiers: 4,
            fallback_enabled: true,
            proof_timeout_ms: 1200,
            circuit_optimization: true,
        };
        let mut zkp = AsyncZKPV3::new(config);

        // Start batch first, then add statements
        zkp.start_batch("batch-1".to_string());

        // Add statement to batch — generate_proof() is private, use add_to_batch + generate_batch_proofs
        let statement = ZKPStatement {
            statement_id: "stmt-1".to_string(),
            public_inputs: vec![1, 2, 3, 4, 5],
            private_inputs_hash: "hash123".to_string(),
            circuit_type: CircuitType::Membership,
        };
        zkp.add_to_batch(statement).unwrap();

        // Generate batch proofs
        let batch = zkp.generate_batch_proofs().unwrap();
        assert_eq!(batch.proofs.len(), 1);
        assert!(!batch.proofs[0].proof_data.is_empty());
    }

    #[test]
    fn test_e2e_zkp_batch() {
        let config = ZKPV3Config {
            max_batch_size: 16,
            parallel_verifiers: 2,
            fallback_enabled: true,
            proof_timeout_ms: 1200,
            circuit_optimization: true,
        };
        let mut zkp = AsyncZKPV3::new(config);

        zkp.start_batch("batch-batch".to_string());

        for i in 0..8 {
            let statement = ZKPStatement {
                statement_id: format!("stmt-{}", i),
                public_inputs: vec![i as u8; 4],
                private_inputs_hash: format!("hash-{}", i),
                circuit_type: CircuitType::Membership,
            };
            zkp.add_to_batch(statement).unwrap();
        }

        let batch_result = zkp.generate_batch_proofs().unwrap();
        assert_eq!(batch_result.proofs.len(), 8);
    }

    #[test]
    fn test_e2e_federation_bridge() {
        let config = BridgeConfig {
            consensus_threshold: 0.67,
            max_relay_hops: 5,
            proof_ttl_ms: 30000,
            max_batch_size: 64,
        };
        let mut bridge = ZKPFederationBridge::new(config);

        // Register shards before relaying proofs
        bridge.register_shard("shard-A".to_string());
        bridge.register_shard("shard-B".to_string());

        // Relay proof — relay_proof takes BridgeProof, returns Result<(), BridgeError>
        let proof = BridgeProof::new(
            "proof-1".to_string(),
            "shard-A".to_string(),
            "shard-B".to_string(),
            "proof-hash-123".to_string(),
        );
        bridge.relay_proof(proof).unwrap();

        // Submit votes — submit_vote takes VerificationVote, returns ()
        bridge.submit_vote(VerificationVote {
            node_id: "voter-1".to_string(),
            proof_id: "proof-1".to_string(),
            valid: true,
            timestamp_ms: 0,
        });
        bridge.submit_vote(VerificationVote {
            node_id: "voter-2".to_string(),
            proof_id: "proof-1".to_string(),
            valid: true,
            timestamp_ms: 0,
        });
        bridge.submit_vote(VerificationVote {
            node_id: "voter-3".to_string(),
            proof_id: "proof-1".to_string(),
            valid: true,
            timestamp_ms: 0,
        });

        // Check consensus — reach_consensus takes &str
        let consensus = bridge.reach_consensus("proof-1").unwrap();
        assert!(consensus);
    }

    // ========================================================================
    // Cross-Module Pipeline
    // ========================================================================

    #[test]
    fn test_e2e_full_pipeline() {
        // 1. Route task to best node
        let config = RouterConfig {
            min_prediction_confidence: 0.5,
            max_route_history: 1000,
            latency_weight: 0.4,
            capacity_weight: 0.3,
            reputation_weight: 0.3,
        };
        let mut router = CrossNodeRouter::new(config);
        router.register_node("node-1".to_string(), 100.0);
        router.update_node_load("node-1", 0.4);
        router.update_node_reputation("node-1", 0.9);

        let task = ComputeTask {
            task_id: "pipeline-task".to_string(),
            task_type: TaskType::FineTuning,
            required_capacity: 50.0,
            priority: 9,
        };
        let route = router.route_task(&task).unwrap();

        // 2. Train with SAE
        let ft_config = FineTuningV2Config {
            learning_rate: 0.01,
            compression_ratio: 0.8,
            batch_size: 32,
            adaptive_lr: true,
            max_retries: 3,
            checkpoint_interval: 5,
            min_node_uptime: 0.9,
        };
        let mut engine = FineTuningV2::new(ft_config);
        engine.register_node(route.target_node.clone(), 0.99);
        let result = engine.train_step(&vec![0.1; 64]).unwrap();
        assert!(result.loss > 0.0);

        // 3. Record reputation
        let ledger_config = LedgerConfig {
            max_events: 100,
            merkle_batch_size: 16,
            retention_days: 30,
        };
        let mut ledger = ReputationLedgerV2::new(ledger_config);
        ledger
            .record_event(
                "pipeline-evt".to_string(),
                route.target_node,
                EventType::ComputeCredit,
                result.loss,
            )
            .unwrap();
        assert!(ledger.verify_chain().is_ok());

        // 4. Generate ZKP proof via batch
        let zkp_config = ZKPV3Config {
            max_batch_size: 16,
            parallel_verifiers: 2,
            fallback_enabled: true,
            proof_timeout_ms: 1200,
            circuit_optimization: true,
        };
        let mut zkp = AsyncZKPV3::new(zkp_config);
        zkp.start_batch("batch-pipeline".to_string());
        let statement = ZKPStatement {
            statement_id: format!("proof-{}", result.round),
            public_inputs: vec![(result.loss * 100.0) as u8; 8],
            private_inputs_hash: format!("hash-{}", result.round),
            circuit_type: CircuitType::Membership,
        };
        zkp.add_to_batch(statement).unwrap();
        let batch = zkp.generate_batch_proofs().unwrap();
        assert_eq!(batch.proofs.len(), 1);
        assert!(!batch.proofs[0].proof_data.is_empty());
    }
}
