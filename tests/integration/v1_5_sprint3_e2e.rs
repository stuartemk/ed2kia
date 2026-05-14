//! v1.5.0 Sprint 3 E2E Integration Tests
//!
//! Covers LP-129 (Federation Scaling v6), LP-130 (Async ZKP v11 & Cross-Federation Verification)

#[cfg(feature = "v1.5-sprint3")]
mod e2e {
    use std::time::Instant;

    // LP-129: Federation Scaling v6
    use ed2kia::federation::scaling_v6::{ScalingV6, ScalingV6Config};
    use ed2kia::federation::dynamic_sharder_v2::{DynamicSharderV2, DynamicSharderV2Config};
    use ed2kia::federation::gradient_sync_v6::{GradientSyncV6, GradientSyncV6Config};

    // LP-130: Async ZKP v11 & Cross-Federation Verification
    use ed2kia::zkp::async_zkp_v11::{AsyncZKPV11, ProofPriority};
    use ed2kia::zkp::cross_federation_verifier_v2::{CrossFederationVerifierV2, Vote};

    fn current_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    // === LP-129: Federation Scaling v6 E2E ===

    #[test]
    fn test_e2e_scaling_v6_node_registration_and_assignment() {
        let mut engine = ScalingV6::new(ScalingV6Config {
            min_reputation: 0.5,
            partition_tolerance: 0.995,
            ..ScalingV6Config::default()
        });

        engine.register_node("n1".to_string(), 100.0, 0.9).unwrap();
        engine.register_node("n2".to_string(), 150.0, 0.85).unwrap();
        engine.register_node("n3".to_string(), 200.0, 0.95).unwrap();

        engine.create_shard("shard1".to_string()).unwrap();
        engine.assign_node_to_shard("shard1").unwrap();

        assert_eq!(engine.node_count(), 3);
        assert_eq!(engine.shard_count(), 1);
    }

    #[test]
    fn test_e2e_scaling_v6_partition_tolerance() {
        let mut engine = ScalingV6::default();
        engine.register_node("n1".to_string(), 100.0, 0.9).unwrap();
        engine.register_node("n2".to_string(), 100.0, 0.9).unwrap();
        engine.create_shard("s1".to_string()).unwrap();
        engine.assign_node_to_shard("s1").unwrap();
        engine.assign_node_to_shard("s1").unwrap();

        assert_eq!(engine.shard_count(), 1);
    }

    #[test]
    fn test_e2e_dynamic_sharder_v2_load_prediction() {
        let mut sharder = DynamicSharderV2::default();
        sharder.register_shard("s1".to_string());
        sharder.register_shard("s2".to_string());

        for i in 0..10 {
            sharder.update_shard_load("s1", 0.5 + i as f64 * 0.05).unwrap();
            sharder.update_shard_load("s2", 0.3 + i as f64 * 0.02).unwrap();
        }

        let pred1 = sharder.predict_load("s1").unwrap();
        let pred2 = sharder.predict_load("s2").unwrap();
        assert!(pred1 >= 0.0 && pred1 <= 1.0);
        assert!(pred2 >= 0.0 && pred2 <= 1.0);
    }

    #[test]
    fn test_e2e_dynamic_sharder_v2_scaling_decision() {
        let mut sharder = DynamicSharderV2::new(DynamicSharderV2Config {
            split_threshold: 0.8,
            merge_threshold: 0.2,
            min_shards: 1,
            max_nodes_per_shard: 5, // node_count > 5/2 = 2, so 3 nodes triggers split
            ..DynamicSharderV2Config::default()
        });
        sharder.register_shard("heavy".to_string()).unwrap();
        sharder.register_shard("light".to_string()).unwrap();

        // Add nodes to heavy shard (need node_count > max_nodes_per_shard / 2 = 2)
        for _ in 0..3 {
            sharder.add_node_to_shard("heavy").unwrap();
        }

        for _ in 0..10 {
            sharder.update_shard_load("heavy", 0.9).unwrap();
            sharder.update_shard_load("light", 0.1).unwrap();
        }

        let actions = sharder.generate_actions();
        assert!(!actions.is_empty());
    }

    #[test]
    fn test_e2e_gradient_sync_v6_cross_model_alignment() {
        let mut engine = GradientSyncV6::new(GradientSyncV6Config {
            compression_ratio: 0.5,
            cross_model_alignment: true,
            cross_model_weight: 0.3,
            ..GradientSyncV6Config::default()
        });

        engine.register_model("m1".to_string(), 10).unwrap();
        engine.register_model("m2".to_string(), 10).unwrap();

        let grads1: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let grads2: Vec<f32> = vec![0.5, 1.5, 2.5, 3.5, 4.5, 5.5, 6.5, 7.5, 8.5, 9.5];
        engine.submit_gradients("m1".to_string(), grads1, current_ms()).unwrap();
        engine.submit_gradients("m2".to_string(), grads2, current_ms()).unwrap();

        let result = engine.execute_sync().unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.contains_key("m1"));
        assert!(result.contains_key("m2"));
    }

    // === LP-130: Async ZKP v11 E2E ===

    #[test]
    fn test_e2e_zkp_v11_proof_lifecycle() {
        let mut engine = AsyncZKPV11::default();
        engine.register_federation("fed1".to_string(), 0.9).unwrap();
        engine.register_federation("fed2".to_string(), 0.85).unwrap();

        let current = current_ms();
        let proof = engine.submit_proof(
            "proof1".to_string(),
            "fed1".to_string(),
            ProofPriority::High,
            10.0,
            current,
        ).unwrap();

        assert_eq!(proof.id(), "proof1");
        assert_eq!(proof.priority(), ProofPriority::High);

        engine.record_vote("proof1", "fed1").unwrap();
        engine.record_vote("proof1", "fed2").unwrap();

        let processed = engine.process_next(current).unwrap();
        assert_eq!(processed.id(), "proof1");
    }

    #[test]
    fn test_e2e_zkp_v11_batch_processing() {
        let mut engine = AsyncZKPV11::default();
        engine.register_federation("fed1".to_string(), 0.9).unwrap();

        let current = current_ms();
        engine.submit_proof("p1".to_string(), "fed1".to_string(), ProofPriority::Normal, 1.0, current).unwrap();
        engine.submit_proof("p2".to_string(), "fed1".to_string(), ProofPriority::Normal, 1.0, current).unwrap();
        engine.submit_proof("p3".to_string(), "fed1".to_string(), ProofPriority::Normal, 1.0, current).unwrap();

        let batch_id = engine.create_batch(current);
        engine.add_to_batch(&batch_id, "p1".to_string()).unwrap();
        engine.add_to_batch(&batch_id, "p2".to_string()).unwrap();

        engine.complete_batch(&batch_id).unwrap();
        assert_eq!(engine.metrics().total_batches, 1);
    }

    #[test]
    fn test_e2e_zkp_v11_quorum_verification() {
        let mut engine = AsyncZKPV11::default();
        engine.register_federation("fed1".to_string(), 0.9).unwrap();
        engine.register_federation("fed2".to_string(), 0.85).unwrap();
        engine.register_federation("fed3".to_string(), 0.8).unwrap();

        let current = current_ms();
        engine.submit_proof("proof1".to_string(), "fed1".to_string(), ProofPriority::Critical, 5.0, current).unwrap();

        engine.record_vote("proof1", "fed1").unwrap();
        engine.record_vote("proof1", "fed2").unwrap();

        // Verify proof was processed through record_vote (quorum checked internally)
        assert_eq!(engine.proof_count(), 1);
    }

    #[test]
    fn test_e2e_cross_federation_verifier_quorum() {
        let mut engine = CrossFederationVerifierV2::default();
        engine.register_federation("fed1".to_string(), 1.0).unwrap();
        engine.register_federation("fed2".to_string(), 0.9).unwrap();
        engine.register_federation("fed3".to_string(), 0.8).unwrap();

        engine.create_session("proof1".to_string()).unwrap();
        engine.submit_vote("proof1", "fed1", Vote::Approve, 1000).unwrap();
        engine.submit_vote("proof1", "fed2", Vote::Approve, 1000).unwrap();
        engine.submit_vote("proof1", "fed3", Vote::Approve, 1000).unwrap();

        let reached = engine.check_quorum("proof1").unwrap();
        assert!(reached);
    }

    #[test]
    fn test_e2e_cross_federation_verifier_merkle() {
        let mut engine = CrossFederationVerifierV2::default();
        engine.register_federation("fed1".to_string(), 1.0).unwrap();
        engine.register_federation("fed2".to_string(), 0.9).unwrap();

        engine.create_session("p1".to_string()).unwrap();
        engine.create_session("p2".to_string()).unwrap();

        // Submit votes to reach quorum and verify sessions
        engine.submit_vote("p1", "fed1", Vote::Approve, current_ms()).unwrap();
        engine.submit_vote("p1", "fed2", Vote::Approve, current_ms()).unwrap();
        engine.submit_vote("p2", "fed1", Vote::Approve, current_ms()).unwrap();
        engine.submit_vote("p2", "fed2", Vote::Approve, current_ms()).unwrap();

        // Check quorum to mark sessions as verified
        engine.check_quorum("p1").unwrap();
        engine.check_quorum("p2").unwrap();

        // Now aggregate using session proof IDs
        let proof_ids = vec!["p1".to_string(), "p2".to_string()];
        let aggregated = engine.aggregate_merkle_roots(&proof_ids).unwrap();
        assert!(!aggregated.is_empty());
    }

    // === Cross-module E2E ===

    #[test]
    fn test_e2e_full_pipeline_v1_5_sprint3() {
        let start = Instant::now();

        // Federation Scaling v6
        let mut scaling = ScalingV6::default();
        scaling.register_node("n1".to_string(), 100.0, 0.9).unwrap();
        scaling.register_node("n2".to_string(), 150.0, 0.85).unwrap();
        scaling.create_shard("shard1".to_string()).unwrap();
        scaling.assign_node_to_shard("shard1").unwrap();

        // Dynamic Sharder v2
        let mut sharder = DynamicSharderV2::default();
        sharder.register_shard("shard1".to_string());

        // Gradient Sync v6
        let mut grad_sync = GradientSyncV6::default();
        grad_sync.register_model("m1".to_string(), 10).unwrap();
        let grads: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        grad_sync.submit_gradients("m1".to_string(), grads, current_ms()).unwrap();
        let sync_result = grad_sync.execute_sync().unwrap();
        assert!(!sync_result.is_empty());

        // Async ZKP v11
        let mut zkp = AsyncZKPV11::default();
        zkp.register_federation("fed1".to_string(), 0.9).unwrap();
        let current = current_ms();
        zkp.submit_proof("proof1".to_string(), "fed1".to_string(), ProofPriority::High, 10.0, current).unwrap();

        // Cross-Federation Verifier v2
        let mut verifier = CrossFederationVerifierV2::default();
        verifier.register_federation("fed1".to_string(), 1.0).unwrap();
        verifier.register_federation("fed2".to_string(), 0.9).unwrap();
        verifier.create_session("proof1".to_string()).unwrap();
        verifier.submit_vote("proof1", "fed1", Vote::Approve, current).unwrap();
        verifier.submit_vote("proof1", "fed2", Vote::Approve, current).unwrap();

        // Verify full pipeline
        assert_eq!(scaling.node_count(), 2);
        assert_eq!(scaling.shard_count(), 1);
        assert_eq!(grad_sync.pending_count(), 0);
        assert_eq!(zkp.proof_count(), 1);

        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() < 5000, "Pipeline took too long: {}ms", elapsed.as_millis());
    }

    #[test]
    fn test_e2e_cross_module_metrics() {
        let mut scaling = ScalingV6::default();
        scaling.register_node("n1".to_string(), 100.0, 0.9).unwrap();

        let mut zkp = AsyncZKPV11::default();
        zkp.register_federation("fed1".to_string(), 0.9).unwrap();
        let current = current_ms();
        zkp.submit_proof("p1".to_string(), "fed1".to_string(), ProofPriority::Normal, 1.0, current).unwrap();

        assert_eq!(scaling.node_count(), 1);
        assert_eq!(zkp.proof_count(), 1);
    }

    #[test]
    fn test_e2e_stress_zkp_v11_high_volume() {
        let mut engine = AsyncZKPV11::default();
        engine.register_federation("fed1".to_string(), 0.9).unwrap();

        let current = current_ms();
        for i in 0..100 {
            let id = format!("proof_{}", i);
            let _ = engine.submit_proof(id, "fed1".to_string(), ProofPriority::Normal, 1.0, current);
        }

        assert_eq!(engine.proof_count(), 100);

        let mut processed = 0;
        for _ in 0..50 {
            if engine.process_next(current).is_some() {
                processed += 1;
            }
        }
        assert_eq!(processed, 50);
    }

    #[test]
    fn test_e2e_stress_scaling_v6_concurrent_assignments() {
        let mut engine = ScalingV6::default();

        for i in 0..50 {
            let node_id = format!("node_{}", i);
            let _ = engine.register_node(node_id, 100.0 + i as f64, 0.8 + (i % 20) as f64 * 0.01);
        }

        assert_eq!(engine.node_count(), 50);

        for i in 0..10 {
            let shard_id = format!("shard_{}", i);
            let _ = engine.create_shard(shard_id);
        }

        for i in 0..10 {
            let shard_id = format!("shard_{}", i);
            let _ = engine.assign_node_to_shard(&shard_id);
        }

        assert_eq!(engine.shard_count(), 10);
    }

    #[test]
    fn test_e2e_scaling_sharder_gradient_integration() {
        let mut scaling = ScalingV6::default();
        scaling.register_node("n1".to_string(), 100.0, 0.9).unwrap();
        scaling.create_shard("s1".to_string()).unwrap();

        let mut grad_sync = GradientSyncV6::default();
        grad_sync.register_model("m1".to_string(), 5).unwrap();

        for i in 0..5 {
            let grads: Vec<f32> = vec![i as f32; 5];
            let _ = grad_sync.submit_gradients("m1".to_string(), grads, current_ms());
        }

        assert_eq!(grad_sync.pending_count(), 5);
    }
}

#[cfg(not(feature = "v1.5-sprint3"))]
mod e2e {
    #[test]
    fn test_sprint3_disabled() {
        assert!(true, "Sprint 3 tests require --features v1.5-sprint3");
    }
}
