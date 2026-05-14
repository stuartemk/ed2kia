//! v1.3.0 Sprint 2 E2E Integration Tests
//!
//! LP-81: Cross-Chain Resource Pools
//! LP-82: DAO Operational Ledger v2
//! LP-83: Async ZKP v4 & Cross-Pool Verification
//! LP-84: UI Dashboard v4 & Real-time Streams
//!
//! Test Scenarios:
//! 1. Cross-Chain Resource Pool (register -> allocate -> decay -> remove)
//! 2. DAO Ledger v2 (record events -> verify chain -> get entries)
//! 3. Async ZKP v4 (batch -> verify -> cross-pool)
//! 4. Dashboard v4 (metrics -> alerts -> snapshots)
//! 5. Pool Stream Engine (session -> publish -> subscribe -> broadcast)
//! 6. WebSocket Pool Stream (auth -> subscribe -> broadcast -> ping)
//! 7. Cross-module pipeline: Pool -> DAO -> ZKP -> Dashboard -> Stream

#[cfg(feature = "v1.3-sprint2")]
mod e2e {
    // LP-81: Cross-Chain Resource Pools
    use ed2kia::pools::cross_chain_resource_pool::{
        CrossChainResourcePool, PoolConfig, PoolRequest, ResourceType, ShardEntry,
    };

    // LP-82: DAO Operational Ledger v2
    use ed2kia::governance::dao_ledger_v2::{
        DaoLedgerV2, DaoLedgerConfig, DaoEventType,
    };
    use ed2kia::governance::technical_staking::{
        TechnicalStaking, StakingConfig,
    };
    use ed2kia::governance::proposal_tracker::{
        ProposalTracker, ProposalConfig, ProposalState,
    };

    // LP-83: Async ZKP v4
    use ed2kia::zkp::async_zkp_v4::{
        AsyncZKPV4, ZKPV4Config, ZKPStatement, CircuitType, PoolContext,
    };
    use ed2kia::pool_zkp_bridge::{
        PoolZKPBridge, PoolZKPConfig, BridgeProof,
    };

    // LP-84: Dashboard v4 & Streams
    use ed2kia::ui_v4::dashboard_v4::{
        DashboardV4State, DashboardV4Config, MetricV4,
    };
    use ed2kia::ui_v4::pool_stream_engine::{
        PoolStreamEngine, StreamEngineConfig, StreamCategory, PoolStreamEvent,
    };
    use ed2kia::ws_pool_stream::{
        WsPoolStream, WsPoolConfig, PoolCategory,
    };

    // ========================================================================
    // LP-81: Cross-Chain Resource Pool E2E
    // ========================================================================

    #[test]
    fn test_e2e_resource_pool_lifecycle() {
        let config = PoolConfig {
            max_pool_credits: 10_000.0,
            min_reputation: 0.5,
            max_shards: 50,
            credit_decay_rate: 0.02,
        };
        let mut pool = CrossChainResourcePool::new(config);

        // Register shards
        pool.register_shard(ShardEntry::new("shard-1".into(), ResourceType::ComputeCredit, 500.0, 0.95)).unwrap();
        pool.register_shard(ShardEntry::new("shard-2".into(), ResourceType::ComputeCredit, 300.0, 0.85)).unwrap();
        pool.register_shard(ShardEntry::new("shard-3".into(), ResourceType::Storage, 1000.0, 0.90)).unwrap();

        assert_eq!(pool.available_credits(&ResourceType::ComputeCredit), 800.0);

        // Allocate resources
        let request = PoolRequest {
            request_id: "req-1".into(),
            node_id: "node-1".into(),
            resource_type: ResourceType::ComputeCredit,
            required_credits: 200.0,
            priority: 8,
        };
        let alloc = pool.allocate(&request).unwrap();
        assert_eq!(alloc.request_id, "req-1");
        assert!(alloc.total_credits > 0.0);

        // Apply decay
        pool.apply_decay();
        let stats = pool.get_stats();
        assert_eq!(stats.successful_allocations, 1);

        // Remove shard
        let removed = pool.remove_shard("shard-2").unwrap();
        assert_eq!(removed.shard_id, "shard-2");
    }

    #[test]
    fn test_e2e_pool_multi_resource() {
        let mut pool = CrossChainResourcePool::default();
        pool.register_shard(ShardEntry::new("s1".into(), ResourceType::ComputeCredit, 100.0, 0.9)).unwrap();
        pool.register_shard(ShardEntry::new("s2".into(), ResourceType::Storage, 200.0, 0.8)).unwrap();
        pool.register_shard(ShardEntry::new("s3".into(), ResourceType::SaeShard, 150.0, 0.85)).unwrap();

        assert_eq!(pool.available_credits(&ResourceType::ComputeCredit), 100.0);
        assert_eq!(pool.available_credits(&ResourceType::Storage), 200.0);
        assert_eq!(pool.available_credits(&ResourceType::SaeShard), 150.0);

        let shards = pool.get_shards_by_type(&ResourceType::ComputeCredit);
        assert_eq!(shards.len(), 1);
    }

    // ========================================================================
    // LP-82: DAO Ledger v2 E2E
    // ========================================================================

    #[test]
    fn test_e2e_dao_ledger_lifecycle() {
        let config = DaoLedgerConfig {
            max_entries: 1000,
            auto_verify: true,
            retention_roots: 50,
        };
        let mut ledger = DaoLedgerV2::new(config);

        // Record operational events
        let e1 = ledger.record_event(
            "e1".into(),
            DaoEventType::ResourceAllocated,
            "node-1".into(),
            "shard-1".into(),
            "allocated 200 credits".into(),
        ).unwrap();
        assert_eq!(e1.entry_id, "e1");
        assert_eq!(e1.sequence, 1);

        let e2 = ledger.record_event(
            "e2".into(),
            DaoEventType::ShardRegistered,
            "node-2".into(),
            "shard-2".into(),
            "registered new shard".into(),
        ).unwrap();
        assert_eq!(e2.entry_id, "e2");
        assert_eq!(e2.sequence, 2);

        // Verify chain integrity
        ledger.verify_chain().unwrap();

        // Get entries by type
        let resource_entries = ledger.get_entries_by_type(&DaoEventType::ResourceAllocated);
        assert_eq!(resource_entries.len(), 1);

        // Get entries by actor
        let node1_entries = ledger.get_entries_by_actor("node-1");
        assert_eq!(node1_entries.len(), 1);

        // Check stats
        let stats = ledger.get_stats();
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.total_verifications, 1);

        // Compute Merkle root
        let merkle = ledger.compute_merkle_root();
        assert!(!merkle.is_empty());
    }

    #[test]
    fn test_e2e_technical_staking() {
        let config = StakingConfig {
            min_reputation: 0.3,
            min_credits: 10.0,
            max_stake_weight: 0.8,
            inactivity_decay: 0.05,
            epoch_duration_ms: 3_600_000,
        };
        let mut staking = TechnicalStaking::new(config);

        staking.register_node("node-1".into(), 0.95, 500.0);
        staking.register_node("node-2".into(), 0.85, 300.0);

        staking.place_stake("node-1", 0.3, 100.0).unwrap();
        staking.place_stake("node-2", 0.3, 50.0).unwrap();

        let profile1 = staking.get_profile("node-1").unwrap();
        assert_eq!(profile1.node_id, "node-1");

        staking.advance_epoch();
        staking.record_activity("node-1").unwrap();

        let stats = staking.get_stats();
        assert_eq!(stats.total_nodes, 2);
    }

    #[test]
    fn test_e2e_proposal_tracker() {
        let config = ProposalConfig {
            default_voting_duration_ms: 86_400_000,
            min_proposer_weight: 0.05,
            quorum_threshold: 0.3,
            approval_threshold: 0.5,
            max_active_proposals: 50,
        };
        let mut tracker = ProposalTracker::new(config);

        // Register voters
        tracker.register_voter("proposer".into(), 0.5);
        tracker.register_voter("voter-1".into(), 0.3);
        tracker.register_voter("voter-2".into(), 0.25);
        tracker.register_voter("voter-3".into(), 0.15);

        // Create proposal
        let proposal = tracker.create_proposal(
            "p1".into(),
            "Add new shard".into(),
            "Register shard-4 for compute".into(),
            "proposer",
        ).unwrap();
        assert_eq!(proposal.state, ProposalState::Draft);

        // Open voting
        tracker.open_voting("p1").unwrap();

        // Cast votes
        tracker.cast_vote("p1", "voter-1", true).unwrap();
        tracker.cast_vote("p1", "voter-2", true).unwrap();
        tracker.cast_vote("p1", "voter-3", false).unwrap();

        let proposal = tracker.get_proposal("p1").unwrap();
        assert_eq!(proposal.vote_count(), 3);
        assert!(proposal.yes_weight > proposal.no_weight);

        // Tally
        let result = tracker.tally_proposal("p1").unwrap();
        assert_eq!(result, ProposalState::Passed);

        // Execute
        tracker.execute_proposal("p1").unwrap();
        let proposal = tracker.get_proposal("p1").unwrap();
        assert_eq!(proposal.state, ProposalState::Executed);
    }

    // ========================================================================
    // LP-83: Async ZKP v4 E2E
    // ========================================================================

    #[test]
    fn test_e2e_zkp_v4_lifecycle() {
        let config = ZKPV4Config {
            max_batch_size: 32,
            parallel_verifiers: 4,
            fallback_enabled: true,
            proof_timeout_ms: 1500,
            circuit_optimization: true,
            max_pools: 16,
            vrf_sampling_rate: 0.3,
            min_pool_credits: 50.0,
        };
        let mut zkp = AsyncZKPV4::new(config);

        // Register pools
        zkp.register_pool(PoolContext::new("pool-1".into(), 500.0, 0.95)).unwrap();
        zkp.register_pool(PoolContext::new("pool-2".into(), 300.0, 0.85)).unwrap();

        // Submit statements
        for i in 0..5 {
            let stmt = ZKPStatement {
                statement_id: format!("stmt-{}", i),
                public_inputs: vec![i as u8; 16],
                private_inputs_hash: format!("hash-{}", i),
                circuit_type: CircuitType::Membership,
                source_pool: "pool-1".into(),
                priority: 10,
            };
            zkp.submit_statement(stmt).unwrap();
        }

        // Start batch and add statements
        zkp.start_batch("batch-e2e".into()).unwrap();
        let added = zkp.add_to_batch(5).unwrap();
        assert_eq!(added, 5);

        // Generate proofs
        let batch = zkp.generate_batch_proofs().unwrap();
        assert_eq!(batch.proofs.len(), 5);

        // Verify a proof
        let proof = &batch.proofs[0];
        let stmt = &batch.statements[0];
        let result = zkp.verify_proof(proof, stmt).unwrap();
        assert!(result.valid);

        let stats = zkp.get_stats();
        assert_eq!(stats.total_proofs_generated, 5);
        assert_eq!(stats.total_proofs_verified, 1);
    }

    #[test]
    fn test_e2e_pool_zkp_bridge() {
        let config = PoolZKPConfig {
            max_proofs_in_flight: 128,
            consensus_threshold: 0.67,
            proof_ttl_ms: 60_000,
            max_verification_hops: 3,
            resource_cost_per_proof: 5.0,
            cross_pool_aggregation: true,
        };
        let mut bridge = PoolZKPBridge::new(config);

        // Register pools
        bridge.register_pool("pool-a".into(), 500.0).unwrap();
        bridge.register_pool("pool-b".into(), 300.0).unwrap();
        bridge.register_pool("pool-c".into(), 400.0).unwrap();

        assert_eq!(bridge.pool_count(), 3);

        // Submit proof for cross-pool verification
        let proof = BridgeProof::new(
            "proof-1".into(),
            "pool-a".into(),
            vec!["pool-b".into(), "pool-c".into()],
            "hash_proof_1".into(),
        );
        bridge.submit_proof(proof).unwrap();
        assert_eq!(bridge.active_proof_count(), 1);

        // Submit verification votes
        bridge.submit_vote("proof-1", "pool-b".into(), true).unwrap();
        bridge.submit_vote("proof-1", "pool-c".into(), true).unwrap();

        // Check consensus
        let result = bridge.check_consensus("proof-1").unwrap();
        assert!(result);

        let stats = bridge.get_stats();
        assert_eq!(stats.total_consensus_reached, 1);
    }

    // ========================================================================
    // LP-84: Dashboard v4 & Streams E2E
    // ========================================================================

    #[test]
    fn test_e2e_dashboard_v4_lifecycle() {
        let config = DashboardV4Config {
            max_metrics_history: 500,
            rate_limit_per_second: 50,
            alert_threshold_pool_credits: 50.0,
            alert_threshold_zkp_fallback: 0.4,
            alert_threshold_dao_quorum: 0.6,
            alert_threshold_latency_ms: 300.0,
            alert_threshold_error_rate: 0.08,
        };
        let mut dashboard = DashboardV4State::with_config(config);

        // Record pool metrics
        dashboard.record_metric(MetricV4::PoolActiveShards, 12.0, None);
        dashboard.record_metric(MetricV4::PoolAvailableCredits, 45.0, None);
        dashboard.record_metric(MetricV4::PoolAllocationsActive, 8.0, None);

        // Record ZKP metrics
        dashboard.record_metric(MetricV4::ZkpStatementsQueued, 25.0, None);
        dashboard.record_metric(MetricV4::ZkpBatchesGenerated, 3.0, None);

        // Record DAO metrics
        dashboard.record_metric(MetricV4::DaoActiveProposals, 5.0, None);
        dashboard.record_metric(MetricV4::DaoVoterParticipation, 0.72, None);

        let snapshot = dashboard.get_snapshot().unwrap();
        assert_eq!(snapshot.pool.active_shards, 12usize);
        assert_eq!(snapshot.zkp.statements_queued, 25usize);
        assert_eq!(snapshot.dao.active_proposals, 5usize);

        // Check alerts triggered (pool credits low)
        let snap_with_alerts = dashboard.get_snapshot().unwrap();
        assert!(snap_with_alerts.alert_count > 0);
    }

    #[test]
    fn test_e2e_pool_stream_engine() {
        let config = StreamEngineConfig {
            max_sessions: 50,
            rate_limit_per_sec: 100,
            event_history_size: 200,
            max_session_age_secs: 3600,
            enable_backpressure: true,
            backpressure_threshold: 80,
        };
        let mut engine = PoolStreamEngine::with_config(config);

        // Create session
        let session = engine.create_session(
            "sess-1".into(),
            vec![StreamCategory::Pool, StreamCategory::Zkp],
        ).unwrap();
        assert_eq!(session.session_id, "sess-1");

        // Publish events
        let result = engine.publish_event(
            PoolStreamEvent::PoolShardRegistered,
            serde_json::json!({"shard_id": "shard-1", "credits": 500.0}),
            None,
        );
        assert_eq!(result.events_sent, 1);

        // Get pending events
        let pending = engine.get_pending_events("sess-1").unwrap();
        assert_eq!(pending.len(), 1);

        let stats = engine.stats;
        assert_eq!(stats.total_broadcasts, 1);
    }

    #[test]
    fn test_e2e_ws_pool_stream() {
        let config = WsPoolConfig {
            max_connections: 30,
            rate_limit_per_sec: 50,
            connection_timeout_ms: 60000,
            heartbeat_interval_ms: 15000,
            snapshot_interval_ms: 2000,
            max_buffer_size: 100,
            auth_required: true,
        };
        let mut stream = WsPoolStream::with_config(config);

        // Authenticate
        let auth_result = stream.authenticate("ws-client".into(), "sig-1".into()).unwrap();
        assert!(!auth_result.connection_id.is_empty());

        // Subscribe
        let conn_id = &auth_result.connection_id;
        let _ = stream.subscribe(conn_id, vec![PoolCategory::Pool, PoolCategory::Zkp]);

        // Broadcast event
        let result = stream.broadcast_event(
            PoolCategory::Pool,
            "pool_update".into(),
            "shards: 10".into(),
        );
        assert_eq!(result.messages_sent, 1);

        // Broadcast snapshot
        let result = stream.broadcast_snapshot("snapshot-data".into());
        assert_eq!(result.messages_sent, 1);

        let stats = stream.stats;
        assert_eq!(stats.active_connections, 1);
    }

    // ========================================================================
    // CROSS-MODULE PIPELINE: Pool -> DAO -> ZKP -> Dashboard -> Stream
    // ========================================================================

    #[test]
    fn test_e2e_cross_module_pipeline() {
        // 1. Resource Pool: allocate compute
        let mut pool = CrossChainResourcePool::default();
        pool.register_shard(ShardEntry::new("shard-1".into(), ResourceType::ComputeCredit, 500.0, 0.95)).unwrap();
        let request = PoolRequest {
            request_id: "pipe-req".into(),
            node_id: "node-1".into(),
            resource_type: ResourceType::ComputeCredit,
            required_credits: 100.0,
            priority: 7,
        };
        let alloc = pool.allocate(&request).unwrap();
        assert!(alloc.total_credits > 0.0);

        // 2. DAO: record resource allocation event
        let dao_config = DaoLedgerConfig {
            max_entries: 500,
            auto_verify: true,
            retention_roots: 20,
        };
        let mut dao = DaoLedgerV2::new(dao_config);
        dao.record_event(
            "dao-e1".into(),
            DaoEventType::ResourceAllocated,
            "node-1".into(),
            "shard-1".into(),
            format!("allocated {} credits", alloc.total_credits),
        ).unwrap();
        dao.record_event(
            "dao-e2".into(),
            DaoEventType::GovernanceAction,
            "node-2".into(),
            "shard-1".into(),
            "approved allocation".into(),
        ).unwrap();
        dao.verify_chain().unwrap();
        assert_eq!(dao.entry_count(), 2);

        // 3. ZKP: generate proof of allocation
        let zkp_config = ZKPV4Config {
            max_batch_size: 16,
            parallel_verifiers: 2,
            fallback_enabled: true,
            proof_timeout_ms: 1000,
            circuit_optimization: true,
            max_pools: 8,
            vrf_sampling_rate: 0.3,
            min_pool_credits: 50.0,
        };
        let mut zkp = AsyncZKPV4::new(zkp_config);
        zkp.register_pool(PoolContext::new("pool-1".into(), 500.0, 0.95)).unwrap();

        let stmt = ZKPStatement {
            statement_id: "alloc-proof".into(),
            public_inputs: vec![alloc.total_credits as u8; 8],
            private_inputs_hash: "alloc-hash".into(),
            circuit_type: CircuitType::RangeProof,
            source_pool: "pool-1".into(),
            priority: 10,
        };
        zkp.submit_statement(stmt).unwrap();
        zkp.start_batch("pipe-batch".into()).unwrap();
        zkp.add_to_batch(1).unwrap();
        let batch = zkp.generate_batch_proofs().unwrap();
        assert_eq!(batch.proofs.len(), 1);

        // 4. Dashboard: record metrics from pipeline
        let mut dashboard = DashboardV4State::default();
        dashboard.record_metric(MetricV4::PoolActiveShards, 1.0, Some("pipeline".into()));
        dashboard.record_metric(MetricV4::PoolAllocationsActive, 1.0, Some("pipeline".into()));
        dashboard.record_metric(MetricV4::ZkpBatchesGenerated, 1.0, Some("pipeline".into()));
        dashboard.record_metric(MetricV4::DaoActiveProposals, 1.0, Some("pipeline".into()));
        let snapshot = dashboard.get_snapshot().unwrap();
        assert_eq!(snapshot.pool.allocations_active, 1usize);
        assert_eq!(snapshot.zkp.batches_generated, 1usize);
        assert_eq!(snapshot.dao.active_proposals, 1usize);

        // 5. Stream: broadcast pipeline events
        let mut engine = PoolStreamEngine::default();
        engine.create_session(
            "pipe-session".into(),
            vec![StreamCategory::All],
        ).unwrap();
        let result = engine.publish_event(
            PoolStreamEvent::PoolAllocationCreated,
            serde_json::json!({"request_id": alloc.request_id, "credits": alloc.total_credits}),
            Some("pipeline".into()),
        );
        assert_eq!(result.events_sent, 1);

        let pending = engine.get_pending_events("pipe-session").unwrap();
        assert!(!pending.is_empty());
    }
}
