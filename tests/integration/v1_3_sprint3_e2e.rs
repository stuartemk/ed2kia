//! v1.3.0 Sprint 3 E2E Integration Tests
//!
//! LP-86: SAE Fine-Tuning v3 Engine
//! LP-87: Federation Scaling v3 & Sharding Adaptativo
//! LP-88: Async ZKP v5 & Cross-Pool Verification
//!
//! Test Scenarios:
//! 1. SAE Fine-Tuning v3 (register -> train -> checkpoint -> align)
//! 2. Federation Scaling v3 (register -> evaluate -> scale -> assign)
//! 3. Async ZKP v5 (batch -> verify -> fallback -> VRF sampling)
//! 4. Federation ZKP Bridge (register -> submit -> vote -> consensus)
//! 5. Cross-module pipeline: SAE -> Federation -> ZKP v5 -> Bridge

#[cfg(feature = "v1.3-sprint3")]
mod e2e {
    // LP-86: SAE Fine-Tuning v3
    use ed2kia::sae::fine_tuning_v3::{
        FineTuningV3, FineTuningV3Config,
    };
    use ed2kia::sae::cross_model_aligner::{
        CrossModelAligner, AlignerConfig,
    };

    // LP-87: Federation Scaling v3
    use ed2kia::federation_scaling_v3::scaling_v3::{
        FederationScalingV3, ScalingV3Config, NodeCapabilityV3, ScalingDecisionType,
    };

    // LP-88: Async ZKP v5
    use ed2kia::zkp::async_zkp_v5::{
        AsyncZKPV5, ZKPV5Config, ZKPStatement, CircuitType, PoolContext, BatchStatus,
    };
    use ed2kia::federation_zkp_bridge::{
        FederationZKPBridge, FederationZKPConfig, FederationProof,
    };

    // ========================================================================
    // LP-86: SAE Fine-Tuning v3 E2E
    // ========================================================================

    #[test]
    fn test_e2e_fine_tuning_v3_lifecycle() {
        let config = FineTuningV3Config {
            learning_rate: 1e-4,
            compression_ratio: 4.0,
            batch_size: 32,
            adaptive_lr: true,
            max_retries: 3,
            checkpoint_interval: 100,
            min_node_uptime: 0.7,
            alignment_threshold: 0.85,
            max_models: 10,
            lz4_compression: true,
        };
        let mut engine = FineTuningV3::new(config);

        // Register nodes
        engine.register_node("node-1".into(), 0.95, 0.9);
        engine.register_node("node-2".into(), 0.88, 0.85);
        engine.register_reserve("reserve-1".into(), 0.80, 0.75);

        // Register model
        engine.register_model(
            "model-alpha".into(),
            "node-1".into(),
            128,
        ).unwrap();

        // Train step — returns aligned gradients
        let gradients: Vec<f32> = (0..128).map(|i| (i % 10) as f32 * 0.1).collect();
        let aligned = engine.train_step(&gradients).unwrap();
        assert_eq!(aligned.len(), 128);

        // Create checkpoint
        let checkpoint = engine.create_checkpoint().unwrap();
        assert_eq!(checkpoint.round, 1);
        assert!(checkpoint.compression_ratio() > 0.0);

        let stats = engine.get_stats();
        assert_eq!(stats.total_rounds, 1);
        assert_eq!(stats.total_checkpoints, 1);
    }

    #[test]
    fn test_e2e_cross_model_alignment() {
        let config = AlignerConfig {
            min_similarity: 0.7,
            score_decay: 0.95,
            max_models: 10,
            adaptive_normalization: true,
        };
        let mut aligner = CrossModelAligner::new(config);

        aligner.register_model("model-a".into(), 64).unwrap();
        aligner.register_model("model-b".into(), 64).unwrap();

        let grads: Vec<f32> = (0..64).map(|i| i as f32 * 0.05).collect();
        let result = aligner.align(&grads).unwrap();
        assert_eq!(result.models_aligned, 2);
        assert!(result.similarity_score > 0.0);
    }

    // ========================================================================
    // LP-87: Federation Scaling v3 E2E
    // ========================================================================

    #[test]
    fn test_e2e_federation_scaling_lifecycle() {
        let config = ScalingV3Config {
            max_shards: 20,
            min_nodes_per_shard: 2,
            scale_up_threshold: 0.8,
            scale_down_threshold: 0.3,
            ..ScalingV3Config::default()
        };
        let mut scaling = FederationScalingV3::with_config(config);

        // Register nodes
        for i in 0..5 {
            let node = NodeCapabilityV3::new(
                format!("node-{}", i),
                100.0,
                16.0,
                80.0,
                1000.0,
            );
            scaling.register_node(node);
        }

        // Evaluate scaling decisions
        let decisions = scaling.evaluate();
        // With low load, no scale-up needed
        assert!(decisions.is_empty() || decisions.iter().all(|d| d.confidence >= 0.0));

        // Assign node to shard
        let shard_id = scaling.assign_node_to_shard("node-0").unwrap();
        assert!(!shard_id.is_empty());

        let stats = scaling.get_stats();
        assert_eq!(stats.active_nodes, 5);
    }

    #[test]
    fn test_e2e_shard_rebalancing() {
        let mut scaling = FederationScalingV3::default();

        // Register high-load nodes to trigger scale-up
        for i in 0..5 {
            let node = NodeCapabilityV3::new(
                format!("node-{}", i),
                100.0,
                16.0,
                80.0,
                1000.0,
            );
            scaling.register_node(node);
            scaling.update_node(&format!("node-{}", i), 0.9, 100.0).unwrap();
        }

        let decisions = scaling.evaluate();
        // Should trigger scale-up due to high load
        let add_shard: Vec<_> = decisions.iter()
            .filter(|d| matches!(d.decision_type, ScalingDecisionType::AddShard))
            .collect();
        assert!(!add_shard.is_empty());
    }

    // ========================================================================
    // LP-88: Async ZKP v5 E2E
    // ========================================================================

    #[test]
    fn test_e2e_zkp_v5_lifecycle() {
        let config = ZKPV5Config {
            max_batch_size: 256,
            parallel_workers: 8,
            vrf_sampling_rate: 0.3,
            accumulator_window: 32,
            fallback_enabled: true,
            ..ZKPV5Config::default()
        };
        let mut engine = AsyncZKPV5::new(config);

        // Register pools
        let pool1 = PoolContext::new("pool-alpha".into(), 500.0, 0.95);
        let pool2 = PoolContext::new("pool-beta".into(), 300.0, 0.88);
        engine.register_pool(pool1).unwrap();
        engine.register_pool(pool2).unwrap();

        // Start batch
        engine.start_batch("batch-e2e".into()).unwrap();

        // Submit statements
        for i in 0..20 {
            let stmt = ZKPStatement {
                statement_id: format!("stmt-{}", i),
                public_inputs: vec![i, i + 1, i + 2],
                private_inputs_hash: format!("hash-{}", i),
                circuit_type: CircuitType::Membership,
                source_pool: if i % 2 == 0 { "pool-alpha".into() } else { "pool-beta".into() },
                priority: 10,
                complexity_score: 0.5,
            };
            engine.submit_statement(stmt).unwrap();
        }

        // Add to batch
        let added = engine.add_to_batch(50).unwrap();
        assert_eq!(added, 20);

        // Generate proofs
        let batch = engine.generate_batch_proofs().unwrap();
        assert_eq!(batch.proofs.len(), 20);
        assert!(matches!(batch.status, BatchStatus::Complete));

        // Verify batch — returns Vec<VerificationResult>
        let verified = engine.verify_batch(&batch).unwrap();
        assert_eq!(verified.len(), 20);

        let stats = engine.get_stats();
        assert_eq!(stats.total_proofs_generated, 20);
        assert_eq!(stats.total_batches_processed, 1);
    }

    #[test]
    fn test_e2e_zkp_v5_fallback_and_vrf() {
        let config = ZKPV5Config {
            max_batch_size: 10,
            vrf_sampling_rate: 1.0,
            ..ZKPV5Config::default()
        };
        let mut engine = AsyncZKPV5::new(config);
        engine.register_pool(PoolContext::new("pool1".into(), 500.0, 0.9)).unwrap();
        engine.start_batch("batch-fallback".into()).unwrap();

        // Submit baseline statements
        for i in 0..15 {
            let stmt = ZKPStatement {
                statement_id: format!("base-{}", i),
                public_inputs: vec![1, 2, 3],
                private_inputs_hash: format!("hash-base-{}", i),
                circuit_type: CircuitType::RangeProof,
                source_pool: "pool1".into(),
                priority: 10,
                complexity_score: 0.5,
            };
            engine.submit_statement(stmt).unwrap();
        }

        // Submit high-complexity statement to trigger fallback
        let high_complexity = ZKPStatement {
            statement_id: "high-cx".into(),
            public_inputs: vec![100u8, 200, 150],
            private_inputs_hash: "hash-high".into(),
            circuit_type: CircuitType::RangeProof,
            source_pool: "pool1".into(),
            priority: 20,
            complexity_score: 15.0,
        };
        engine.submit_statement(high_complexity).unwrap();

        engine.add_to_batch(30).unwrap();
        let batch = engine.generate_batch_proofs().unwrap();

        let fallback_count = batch.proofs.iter().filter(|p| p.used_fallback).count();
        assert!(fallback_count > 0, "Fallback should be triggered for high complexity");

        let sampled_count = batch.proofs.iter().filter(|p| p.is_vrf_sample).count();
        assert!(sampled_count > 0, "VRF sampling should be active");
    }

    // ========================================================================
    // Federation ZKP Bridge E2E
    // ========================================================================

    #[test]
    fn test_e2e_federation_bridge_lifecycle() {
        let config = FederationZKPConfig {
            max_proofs_in_flight: 128,
            consensus_threshold: 0.67,
            max_shards: 32,
            routing_strategy: 1, // capacity-based
            proof_ttl_ms: 60_000,
            cross_shard_aggregation: true,
            max_verification_hops: 3,
            merkle_sync_interval_ms: 5000,
            resource_cost_per_proof: 10.0,
        };
        let mut bridge = FederationZKPBridge::new(config);

        // Register shards
        bridge.register_shard("shard-1".into(), 500.0, 0.95).unwrap();
        bridge.register_shard("shard-2".into(), 300.0, 0.88).unwrap();
        bridge.register_shard("shard-3".into(), 400.0, 0.92).unwrap();

        // Submit proof
        let proof = FederationProof::new(
            "proof-e2e".into(),
            "shard-1".into(),
            vec!["shard-2".into(), "shard-3".into()],
            "proof-hash-abc".into(),
        );
        bridge.submit_proof(proof).unwrap();

        // Submit votes
        bridge.submit_vote("proof-e2e", "shard-2", true).unwrap();
        bridge.submit_vote("proof-e2e", "shard-3", true).unwrap();

        // Check consensus
        let consensus = bridge.check_consensus("proof-e2e").unwrap();
        assert!(consensus, "Consensus should be reached with 2/2 yes votes");

        // Complete verification
        bridge.complete_verification("proof-e2e", true).unwrap();

        let stats = bridge.get_stats();
        assert_eq!(stats.total_proofs_verified, 1);
        assert_eq!(stats.total_consensus_reached, 1);
    }

    #[test]
    fn test_e2e_merkle_root_sync() {
        let config = FederationZKPConfig {
            max_proofs_in_flight: 100,
            consensus_threshold: 0.67,
            max_shards: 10,
            routing_strategy: 1,
            proof_ttl_ms: 60_000,
            cross_shard_aggregation: true,
            max_verification_hops: 3,
            merkle_sync_interval_ms: 5000,
            resource_cost_per_proof: 10.0,
        };
        let mut bridge = FederationZKPBridge::new(config);
        bridge.register_shard("shard-a".into(), 500.0, 0.95).unwrap();
        bridge.register_shard("shard-b".into(), 300.0, 0.88).unwrap();
        bridge.register_shard("shard-c".into(), 400.0, 0.92).unwrap();

        // Sync Merkle root between two shards
        bridge.sync_merkle_root("shard-a", "shard-b", "merkle-root-abc123".into()).unwrap();

        // Broadcast to all shards
        let count = bridge.broadcast_merkle_root("shard-a", "merkle-root-xyz".into()).unwrap();
        assert_eq!(count, 2); // shard-b and shard-c

        let stats = bridge.get_stats();
        assert!(stats.total_merkle_syncs > 0);
    }

    // ========================================================================
    // Cross-Module Pipeline
    // ========================================================================

    #[test]
    fn test_e2e_cross_module_pipeline() {
        // 1. SAE Fine-Tuning: Train model
        let mut sae = FineTuningV3::default();
        sae.register_node("sae-node".into(), 0.95, 0.9);
        sae.register_model("model-x".into(), "sae-node".into(), 64).unwrap();
        let grads: Vec<f32> = (0..64).map(|i| i as f32 * 0.1).collect();
        let aligned = sae.train_step(&grads).unwrap();
        assert_eq!(aligned.len(), 64);

        // 2. Federation Scaling: Register and evaluate
        let mut scaling = FederationScalingV3::default();
        let node = NodeCapabilityV3::new("fed-node".into(), 200.0, 16.0, 80.0, 1000.0);
        scaling.register_node(node);
        let _ = scaling.evaluate();
        assert_eq!(scaling.get_stats().active_nodes, 1);

        // 3. ZKP v5: Generate proofs
        let mut zkp = AsyncZKPV5::with_defaults();
        zkp.register_pool(PoolContext::new("zkp-pool".into(), 500.0, 0.9)).unwrap();
        zkp.start_batch("pipeline-batch".into()).unwrap();
        for i in 0..10 {
            let stmt = ZKPStatement {
                statement_id: format!("pipe-{}", i),
                public_inputs: vec![i],
                private_inputs_hash: format!("pipe-hash-{}", i),
                circuit_type: CircuitType::Membership,
                source_pool: "zkp-pool".into(),
                priority: 10,
                complexity_score: 0.5,
            };
            zkp.submit_statement(stmt).unwrap();
        }
        zkp.add_to_batch(20).unwrap();
        let batch = zkp.generate_batch_proofs().unwrap();
        assert_eq!(batch.proofs.len(), 10);

        // 4. Federation Bridge: Verify cross-shard
        let config = FederationZKPConfig {
            max_proofs_in_flight: 50,
            consensus_threshold: 0.67,
            max_shards: 10,
            routing_strategy: 1,
            proof_ttl_ms: 60_000,
            cross_shard_aggregation: true,
            max_verification_hops: 3,
            merkle_sync_interval_ms: 5000,
            resource_cost_per_proof: 10.0,
        };
        let mut bridge = FederationZKPBridge::new(config);
        bridge.register_shard("shard-x".into(), 500.0, 0.95).unwrap();
        bridge.register_shard("shard-y".into(), 300.0, 0.88).unwrap();
        let proof = FederationProof::new(
            "bridge-proof".into(),
            "shard-x".into(),
            vec!["shard-y".into()],
            "bridge-hash".into(),
        );
        bridge.submit_proof(proof).unwrap();
        bridge.submit_vote("bridge-proof", "shard-y", true).unwrap();
        let consensus = bridge.check_consensus("bridge-proof").unwrap();
        assert!(consensus);

        // Verify all stats are consistent
        assert_eq!(sae.get_stats().total_rounds, 1);
        assert_eq!(scaling.get_stats().active_nodes, 1);
        assert_eq!(zkp.get_stats().total_proofs_generated, 10);
        assert_eq!(bridge.get_stats().total_proofs_bridged, 1);
    }
}
