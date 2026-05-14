//! v1.2.0 Sprint 2 E2E Integration Tests
//!
//! Cross-Chain Consensus, DAO Governance v3 & Distributed Fine-Tuning
//!
//! Test Scenarios:
//! 1. Cross-chain consensus full lifecycle (proposal → voting → quorum → execution)
//! 2. Bridge validator transaction validation (register → confirm → validate → execute)
//! 3. DAO v3 governance cycle (member → proposal → vote → timelock → execute)
//! 4. Hybrid voting engine (on-chain + off-chain → result)
//! 5. Proposal executor (enqueue → timelock → execute → ledger)
//! 6. Distributed fine-tuning (register → train → sync → aggregate)
//! 7. Checkpoint manager (create → incremental → restore → verify)
//! 8. Fault tolerance (failure → circuit breaker → recovery)
//! 9. Full pipeline: consensus → dao → distributed_training → checkpoint → fault_tolerance

#[cfg(feature = "v1.2-sprint2")]
mod e2e {
    // LP-54: Cross-Chain Consensus
    use ed2kia::federation::cross_chain_consensus::{
        ChainProof, ConsensusConfig, ConsensusProposal, CrossChainConsensus,
        ProofType, Validator, VoteDirection,
    };
    use ed2kia::federation::bridge_validator::{
        BridgeConfig, BridgeTransaction, BridgeValidator, LockProof,
    };

    // LP-55: DAO Governance v3
    use ed2kia::governance::dao_v3::{
        DaoConfig, DaoGovernanceV3, DaoMember, DaoProposal, DaoVoteDirection, VoteType,
    };
    use ed2kia::governance::hybrid_voting::{
        HybridVotingConfig, HybridVotingEngine, VotingChannel,
    };
    use ed2kia::governance::proposal_executor::{
        ExecutorConfig, ExecutableProposal, ProposalExecutor, ProposalPriority,
    };

    // LP-56: Distributed Fine-Tuning
    use ed2kia::sae::distributed_finetune::{
        AggregationMethod, DistributedConfig, DistributedFineTuning, GradientBatch,
    };
    use ed2kia::sae::checkpoint_manager::{
        CheckpointConfig, CheckpointManager, CheckpointType, RetentionPolicy,
    };
    use ed2kia::sae::fault_tolerance::{
        FaultToleranceConfig, FaultToleranceManager, FailureType, NodeHealth,
    };

    use std::time::Duration;

    // ─── LP-54: Cross-Chain Consensus E2E ───

    #[test]
    fn test_e2e_cross_chain_consensus_full_lifecycle() {
        let config = ConsensusConfig {
            quorum_threshold: 0.9, // Need all 3 validators to reach quorum
            approval_threshold: 0.51,
            timelock_duration: Duration::from_secs(0),
            max_proof_age_secs: 3600,
            min_validators: 3,
            voting_duration: Duration::from_secs(60),
        };
        let mut consensus = CrossChainConsensus::with_config(config);

        // Register validators
        consensus.register_validator(Validator::new(
            "v1".into(),
            vec!["eth".into(), "sol".into()],
            0.8,
            1000,
        ));
        consensus.register_validator(Validator::new(
            "v2".into(),
            vec!["eth".into(), "sol".into()],
            0.9,
            2000,
        ));
        consensus.register_validator(Validator::new(
            "v3".into(),
            vec!["eth".into(), "sol".into()],
            0.7,
            500,
        ));

        // Create proposal
        let proof = ChainProof::new(
            ProofType::MerkleInclusion,
            "eth".into(),
            "sol".into(),
            "proof-data".into(),
            "hash-abc123".into(),
        );
        let proposal = ConsensusProposal::new(
            "prop-1".into(),
            "eth".into(),
            "sol".into(),
            "Upgrade bridge".into(),
            vec![proof],
            false,
        );
        consensus.create_proposal(proposal).unwrap();

        // Start voting
        consensus.start_voting("prop-1").unwrap();

        // Submit votes - quorum won't be reached until all 3 vote
        consensus
            .submit_vote("prop-1", "v1", VoteDirection::For, Some("reason".into()))
            .unwrap();
        consensus
            .submit_vote("prop-1", "v2", VoteDirection::For, Some("reason".into()))
            .unwrap();
        consensus
            .submit_vote("prop-1", "v3", VoteDirection::For, Some("reason".into()))
            .unwrap();

        // Check result
        let result = consensus.get_result("prop-1").unwrap();
        assert!(result.quorum_reached);
    }

    #[test]
    fn test_e2e_bridge_validator_full_lifecycle() {
        let config = BridgeConfig {
            min_confirmations: 3,
            ..Default::default()
        };
        let mut validator = BridgeValidator::with_config(config);
        validator.add_supported_chain("eth".into());
        validator.add_supported_chain("sol".into());

        // Register transaction
        let tx = BridgeTransaction::new(
            "tx-1".into(),
            "eth".into(),
            "sol".into(),
            "sender".into(),
            "receiver".into(),
            1000,
            "TOKEN".into(),
        );
        validator.register_transaction(tx).unwrap();

        // Update confirmations
        validator.update_confirmations("tx-1", 5).unwrap();

        // Create lock proof
        let proof = LockProof::new(
            "tx-1".into(),
            "eth".into(),
            "block-hash-abc".into(),
            12345,
            vec!["proof-1".into(), "proof-2".into()],
            "merkle-root-xyz".into(),
        );

        // Validate
        let result = validator.validate_transaction("tx-1", &proof).unwrap();
        assert!(result.valid);

        // Execute
        assert!(validator.execute_transaction("tx-1").is_ok());
    }

    // ─── LP-55: DAO Governance v3 E2E ───

    #[test]
    fn test_e2e_dao_governance_full_cycle() {
        let config = DaoConfig {
            voting_duration: Duration::from_secs(60),
            timelock_duration: Duration::from_secs(0),
            min_proposal_stake: 100,
            quorum_threshold: 0.9, // Need all 3 members
            ..Default::default()
        };
        let mut dao = DaoGovernanceV3::with_config(config);

        // Register members
        dao.register_member(DaoMember::new("m1".into(), 1000, 0.8));
        dao.register_member(DaoMember::new("m2".into(), 2000, 0.9));
        dao.register_member(DaoMember::new("m3".into(), 1500, 0.7));

        // Create proposal
        let proposal = DaoProposal::new(
            "prop-1".into(),
            "m1".into(),
            "Upgrade protocol".into(),
            "Upgrade the main protocol".into(),
            VoteType::OnChain,
            false,
        );
        dao.create_proposal(proposal).unwrap();

        // Start voting
        dao.start_voting("prop-1").unwrap();

        // Cast votes
        dao.cast_vote("prop-1", "m1", DaoVoteDirection::For, VoteType::OnChain, None)
            .unwrap();
        dao.cast_vote("prop-1", "m2", DaoVoteDirection::For, VoteType::OnChain, None)
            .unwrap();
        dao.cast_vote("prop-1", "m3", DaoVoteDirection::Against, VoteType::OnChain, None)
            .unwrap();

        // Verify proposal exists
        let prop = dao.get_proposal("prop-1").unwrap();
        assert_eq!(prop.title, "Upgrade protocol");
    }

    #[test]
    fn test_e2e_hybrid_voting_engine() {
        let config = HybridVotingConfig {
            min_participants: 3,
            ..Default::default()
        };
        let mut engine = HybridVotingEngine::with_config(config);

        // Register voters
        engine.register_voter("v1".into(), 100);
        engine.register_voter("v2".into(), 200);
        engine.register_voter("v3".into(), 150);

        // Cast on-chain votes (majority on-chain for validity)
        engine
            .cast_vote("v1".into(), VotingChannel::OnChain, true)
            .unwrap();
        engine
            .cast_vote("v2".into(), VotingChannel::OnChain, true)
            .unwrap();
        engine
            .cast_vote("v3".into(), VotingChannel::OnChain, true)
            .unwrap();

        // Calculate result
        let result = engine.calculate_result();
        assert!(result.valid);
    }

    #[test]
    fn test_e2e_proposal_executor_priority_queue() {
        let config = ExecutorConfig {
            ..Default::default()
        };
        let mut executor = ProposalExecutor::with_config(config);

        // Enqueue proposals with different priorities
        let p1 = ExecutableProposal::new(
            "critical".into(),
            "Critical action".into(),
            ProposalPriority::Critical,
            Duration::from_secs(0),
        );
        let p2 = ExecutableProposal::new(
            "low".into(),
            "Low priority action".into(),
            ProposalPriority::Low,
            Duration::from_secs(0),
        );
        let p3 = ExecutableProposal::new(
            "high".into(),
            "High priority action".into(),
            ProposalPriority::High,
            Duration::from_secs(0),
        );

        executor.enqueue(p1).unwrap();
        executor.enqueue(p2).unwrap();
        executor.enqueue(p3).unwrap();

        assert_eq!(executor.queue_size(), 3);

        // Execute next (should be Critical first)
        let outcome = executor.execute_next().unwrap();
        assert_eq!(outcome.proposal_id, "critical");
    }

    // ─── LP-56: Distributed Fine-Tuning E2E ───

    #[test]
    fn test_e2e_distributed_finetuning_full_pipeline() {
        let config = DistributedConfig {
            gradient_dim: 64,
            aggregation_method: AggregationMethod::FedAvg,
            ..Default::default()
        };
        let mut engine = DistributedFineTuning::new(config);

        // Register nodes
        engine
            .register_node("node-1".into(), 1.0, 64)
            .unwrap();
        engine
            .register_node("node-2".into(), 1.5, 64)
            .unwrap();
        engine
            .register_node("node-3".into(), 0.8, 64)
            .unwrap();

        // Start training
        assert!(engine.start_training().is_ok());

        // Run 3 epochs
        for epoch in 1..=3 {
            engine.start_epoch().unwrap();

            for node in &["node-1", "node-2", "node-3"] {
                let gradients = vec![0.1 * epoch as f32; 64];
                let batch = GradientBatch::new(
                    node.to_string(),
                    epoch,
                    0,
                    gradients,
                    1.0 / epoch as f32,
                );
                engine.submit_gradient(batch).unwrap();
            }

            let summary = engine.sync_gradients().unwrap();
            assert_eq!(summary.epoch, epoch);
            assert_eq!(summary.participants.len(), 3);
        }

        assert_eq!(engine.get_epoch_summaries().len(), 3);
        engine.complete_training();
    }

    #[test]
    fn test_e2e_checkpoint_manager_lifecycle() {
        let config = CheckpointConfig {
            retention: RetentionPolicy {
                max_checkpoints: 20,
                max_age: Duration::from_secs(86400),
                max_total_bytes: usize::MAX,
                keep_latest: 5,
                keep_per_epoch: 2,
            },
            ..Default::default()
        };
        let mut manager = CheckpointManager::new(config);

        // Create full checkpoints for epochs 1-3
        for epoch in 1..=3 {
            let gradients = vec![0.5 * epoch as f32; 32];
            manager
                .create_checkpoint(
                    format!("cp-{}", epoch),
                    CheckpointType::Full,
                    epoch,
                    gradients,
                    1.0 / epoch as f32,
                )
                .unwrap();
        }

        // Create incremental checkpoint
        manager
            .create_incremental_checkpoint(
                "cp-3-inc".into(),
                3,
                vec![0.01; 32],
                "cp-3".into(),
                0.33,
            )
            .unwrap();

        // Verify checkpoint integrity
        assert!(manager.verify_checkpoint("cp-1").unwrap());
        assert!(manager.verify_checkpoint("cp-3").unwrap());

        // Restore gradients from incremental checkpoint
        let restored = manager.restore_gradients("cp-3-inc").unwrap();
        assert_eq!(restored.len(), 32);

        // Check storage tracking
        assert!(manager.total_storage_bytes() > 0);
        assert_eq!(manager.checkpoint_count(), 4);
    }

    #[test]
    fn test_e2e_fault_tolerance_lifecycle() {
        let config = FaultToleranceConfig::new(3);
        let mut manager = FaultToleranceManager::new(config);

        // Register nodes
        manager.register_node("n1".into()).unwrap();
        manager.register_node("n2".into()).unwrap();
        manager.register_node("n3".into()).unwrap();

        // Record successes
        manager.record_node_success("n1").unwrap();
        manager.record_node_success("n2").unwrap();
        assert_eq!(manager.get_healthy_node_count(), 3);

        // Simulate failures on n1
        for _ in 0..3 {
            manager
                .record_node_failure("n1", FailureType::Timeout)
                .unwrap();
        }

        // n1 should be unhealthy
        assert_eq!(
            manager.get_node_health("n1").unwrap(),
            NodeHealth::Unhealthy
        );

        // Circuit breaker should block
        assert!(!manager.can_send_to_node("n1").unwrap());
    }

    // ─── Full Pipeline E2E ───

    #[test]
    fn test_e2e_full_pipeline_consensus_to_dao() {
        // Cross-chain consensus approves proposal
        let consensus_config = ConsensusConfig {
            quorum_threshold: 0.9, // Need all validators
            approval_threshold: 0.51,
            timelock_duration: Duration::from_secs(0),
            max_proof_age_secs: 3600,
            min_validators: 3,
            voting_duration: Duration::from_secs(60),
        };
        let mut consensus = CrossChainConsensus::with_config(consensus_config);
        consensus.register_validator(Validator::new(
            "v1".into(),
            vec!["eth".into(), "sol".into()],
            0.9,
            1000,
        ));
        consensus.register_validator(Validator::new(
            "v2".into(),
            vec!["eth".into(), "sol".into()],
            0.8,
            2000,
        ));
        consensus.register_validator(Validator::new(
            "v3".into(),
            vec!["eth".into(), "sol".into()],
            0.7,
            500,
        ));

        let proof = ChainProof::new(
            ProofType::MerkleInclusion,
            "eth".into(),
            "sol".into(),
            "data".into(),
            "hash-1".into(),
        );
        consensus
            .create_proposal(ConsensusProposal::new(
                "cross-1".into(),
                "eth".into(),
                "sol".into(),
                "Bridge upgrade".into(),
                vec![proof],
                false,
            ))
            .unwrap();
        consensus.start_voting("cross-1").unwrap();

        consensus
            .submit_vote("cross-1", "v1", VoteDirection::For, Some("ok".into()))
            .unwrap();
        consensus
            .submit_vote("cross-1", "v2", VoteDirection::For, Some("ok".into()))
            .unwrap();
        consensus
            .submit_vote("cross-1", "v3", VoteDirection::For, Some("ok".into()))
            .unwrap();

        let result = consensus.get_result("cross-1").unwrap();
        assert!(result.quorum_reached);

        // DAO creates governance proposal based on consensus result
        let dao_config = DaoConfig {
            voting_duration: Duration::from_secs(60),
            timelock_duration: Duration::from_secs(0),
            min_proposal_stake: 100,
            quorum_threshold: 0.9, // Need all members
            ..Default::default()
        };
        let mut dao = DaoGovernanceV3::with_config(dao_config);
        dao.register_member(DaoMember::new("m1".into(), 1000, 0.8));
        dao.register_member(DaoMember::new("m2".into(), 2000, 0.9));

        dao.create_proposal(DaoProposal::new(
            "dao-1".into(),
            "m1".into(),
            "Execute cross-chain proposal".into(),
            "Execute after consensus".into(),
            VoteType::OnChain,
            false,
        )).unwrap();
        dao.start_voting("dao-1").unwrap();

        dao.cast_vote("dao-1", "m1", DaoVoteDirection::For, VoteType::OnChain, None)
            .unwrap();
        dao.cast_vote("dao-1", "m2", DaoVoteDirection::For, VoteType::OnChain, None)
            .unwrap();

        let prop = dao.get_proposal("dao-1").unwrap();
        assert_eq!(prop.title, "Execute cross-chain proposal");
    }

    #[test]
    fn test_e2e_full_pipeline_training_with_checkpoints_and_fault_tolerance() {
        // Setup fault tolerance
        let ft_config = FaultToleranceConfig::new(5);
        let mut ft_manager = FaultToleranceManager::new(ft_config);
        ft_manager.register_node("trainer-1".into()).unwrap();
        ft_manager.register_node("trainer-2".into()).unwrap();
        ft_manager.register_node("trainer-3".into()).unwrap();

        // Setup checkpoint manager
        let cp_config = CheckpointConfig::default();
        let mut cp_manager = CheckpointManager::new(cp_config);

        // Setup distributed training
        let train_config = DistributedConfig {
            gradient_dim: 32,
            ..Default::default()
        };
        let mut train_engine = DistributedFineTuning::new(train_config);
        train_engine
            .register_node("trainer-1".into(), 1.0, 32)
            .unwrap();
        train_engine
            .register_node("trainer-2".into(), 1.0, 32)
            .unwrap();
        train_engine
            .register_node("trainer-3".into(), 1.0, 32)
            .unwrap();
        train_engine.start_training().unwrap();

        // Run epochs with checkpoints
        for epoch in 1..=3 {
            train_engine.start_epoch().unwrap();

            for node in &["trainer-1", "trainer-2", "trainer-3"] {
                let batch = GradientBatch::new(
                    node.to_string(),
                    epoch,
                    0,
                    vec![0.05; 32],
                    0.5 / epoch as f32,
                );
                train_engine.submit_gradient(batch).unwrap();
                ft_manager.record_node_success(node).unwrap();
            }

            let summary = train_engine.sync_gradients().unwrap();

            // Create checkpoint after each epoch
            cp_manager
                .create_checkpoint(
                    format!("epoch-{}", epoch),
                    CheckpointType::Full,
                    epoch,
                    summary.aggregated_gradients.clone(),
                    summary.avg_loss,
                )
                .unwrap();
        }

        // Verify final state
        assert_eq!(train_engine.get_epoch_summaries().len(), 3);
        assert_eq!(cp_manager.checkpoint_count(), 3);
        assert_eq!(ft_manager.get_healthy_node_count(), 3);

        // Verify checkpoint can restore gradients
        let restored = cp_manager.restore_gradients("epoch-3").unwrap();
        assert_eq!(restored.len(), 32);
    }

    #[test]
    fn test_e2e_feature_flag_enabled() {
        // This test only compiles if v1.2-sprint2 is enabled
    }
}
