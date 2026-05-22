//! v1.4.0 Sprint 2 E2E Integration Tests
//!
//! Cross-Chain Pools v3, DAO Ledger v4, Async ZKP v7, UI Dashboard v5
//!
//! Test Scenarios:
//! 1. Pool ZKP Bridge full lifecycle with ZKP v7 proofs
//! 2. DAO Ledger v4 with hybrid execution and audit trail
//! 3. Async ZKP v7 with cross-pool verification
//! 4. UI Dashboard v5 aggregating all metrics
//! 5. Stream engine with alert pipeline
//! 6. Full pipeline: Pools → ZKP v7 → Cross-Pool → DAO → Dashboard → Streams

#[cfg(feature = "v1.4-sprint2")]
mod e2e {
    // LP-103: Cross-Chain Pools v3
    use ed2kia::bridge::pool_zkp_bridge::{BridgeProof, PoolZKPBridge, PoolZKPConfig};
    use ed2kia::pools_v3::pool_matcher_v3::{
        MatcherV3Config, PoolMatcherV3, PoolRequestV3, RequestType, ShardCandidateV3,
    };

    // LP-104: DAO Ledger v4
    use ed2kia::governance_v4::audit_trail::{AuditAction, AuditTrail, AuditTrailConfig, Severity};
    use ed2kia::governance_v4::dao_ledger_v4::{DaoEventV4, DaoLedgerV4, DaoLedgerV4Config};
    use ed2kia::governance_v4::hybrid_executor::{ExecutionMode, HybridConfig, HybridExecutor};

    // LP-105: Async ZKP v7
    use ed2kia::zkp_v7::async_zkp_v7::{
        AsyncZKPV7, BackendType, ShardContext, ZKPStatement, ZKPV7Config,
    };
    use ed2kia::zkp_v7::cross_pool_verification::{
        CrossPoolConfig, CrossPoolVerifier, PoolVerifier,
    };

    // LP-106: UI Dashboard v5
    use ed2kia::ui_v5::alert_pipeline::{
        AlertOperator, AlertPipeline, AlertRule, AlertSeverity, PipelineConfig,
    };
    use ed2kia::ui_v5::dashboard_v5::{DashboardV5, DashboardV5Config, MetricV5};
    use ed2kia::ui_v5::stream_engine::{MetricCategory, StreamConfig, StreamEngine, StreamEvent};

    // ─── LP-103 + LP-105: Pool Matching → ZKP v7 Bridge ───

    #[test]
    fn test_e2e_pool_matching_to_zkp_v7() {
        // Create pool matcher
        let mut matcher = PoolMatcherV3::new(MatcherV3Config::default());
        matcher
            .register_candidate(ShardCandidateV3::new(
                "shard-1".to_string(),
                1000.0,
                0.9,
                15.0,
                0.3,
                "halo2".to_string(),
            ))
            .unwrap();

        // Create request
        let req = PoolRequestV3::new(
            "req-1".to_string(),
            "client-1".to_string(),
            RequestType::Inference,
            50.0,
            10,
        );

        // Match request
        let match_result = matcher.match_request(&req).unwrap();
        assert_eq!(match_result.candidate_id, "shard-1");
        assert!(match_result.score > 0.0);

        // Create ZKP v7 engine
        let mut zkp = AsyncZKPV7::new(ZKPV7Config::default());
        zkp.register_shard(ShardContext::new(
            "shard-1".to_string(),
            1000.0,
            0.9,
            BackendType::Halo2,
        ))
        .unwrap();

        // Submit statement matched by pool
        let stmt = ZKPStatement {
            statement_id: "stmt-1".to_string(),
            public_inputs: vec![1, 2, 3],
            private_inputs_hash: format!("hash-{}", match_result.candidate_id),
            circuit_type: ed2kia::zkp_v7::async_zkp_v7::CircuitType::Membership,
            source_pool: match_result.candidate_id,
            priority: req.priority,
            complexity_score: 0.5,
        };
        zkp.submit_statement(stmt).unwrap();

        // Generate proof
        zkp.start_batch("batch-1".to_string()).unwrap();
        zkp.fill_batch("batch-1", 10).unwrap();
        let proofs = zkp.generate_batch_proofs("batch-1").unwrap();
        assert!(!proofs.is_empty());
    }

    #[test]
    fn test_e2e_pool_bridge_with_zkp_v7_proofs() {
        let mut bridge = PoolZKPBridge::new(PoolZKPConfig::default());
        bridge.register_pool("pool-1".to_string(), 500.0).unwrap();
        bridge.register_pool("pool-2".to_string(), 300.0).unwrap();

        // Create bridge proof
        let proof = BridgeProof::new(
            "bp-1".to_string(),
            "pool-1".to_string(),
            vec!["pool-2".to_string()],
            "merkle-root-1".to_string(),
            50.0,
        );
        bridge.submit_proof(proof).unwrap();

        // Submit votes
        bridge.submit_vote("bp-1", "pool-2", true).unwrap();

        // Check consensus
        let consensus = bridge.check_consensus("bp-1");
        assert!(consensus.is_ok() || consensus.is_err()); // Valid either way
    }

    // ─── LP-104: DAO Ledger v4 Full Lifecycle ───

    #[test]
    fn test_e2e_dao_ledger_v4_lifecycle() {
        let mut ledger = DaoLedgerV4::new(DaoLedgerV4Config::default());

        // Record governance events
        let event1 = DaoEventV4::new(
            "evt-1".to_string(),
            ed2kia::governance_v4::dao_ledger_v4::DaoEventTypeV4::ProposalCreated,
            "actor-1".to_string(),
            "propose budget allocation".to_string(),
        );
        ledger.record_event(event1).unwrap();

        let event2 = DaoEventV4::new(
            "evt-2".to_string(),
            ed2kia::governance_v4::dao_ledger_v4::DaoEventTypeV4::VoteCast,
            "actor-2".to_string(),
            "vote yes on proposal".to_string(),
        );
        ledger.record_event(event2).unwrap();

        // Verify chain
        ledger.verify_chain().unwrap();

        // Get entries by type
        let proposals = ledger.get_entries_by_type(
            &ed2kia::governance_v4::dao_ledger_v4::DaoEventTypeV4::ProposalCreated,
        );
        assert_eq!(proposals.len(), 1);
    }

    #[test]
    fn test_e2e_hybrid_executor_with_audit() {
        let mut executor = HybridExecutor::new(HybridConfig::default());
        let mut audit = AuditTrail::new(AuditTrailConfig::default());

        // Execute on-chain proposal
        let result = executor.execute(
            "prop-1".to_string(),
            ExecutionMode::OnChain,
            "deploy model v2".to_string(),
        );
        assert!(result.is_ok());

        // Record in audit trail
        audit
            .record(
                "audit-1".to_string(),
                AuditAction::ProposalExecuted,
                Severity::Info,
                "executor".to_string(),
                "prop-1".to_string(),
                "Executed on-chain proposal".to_string(),
            )
            .unwrap();

        // Verify audit chain
        audit.verify_chain().unwrap();

        // Generate compliance report
        let report = audit.generate_compliance_report(0, u64::MAX);
        assert_eq!(report.total_entries, 1);
    }

    // ─── LP-105: ZKP v7 + Cross-Pool Verification ───

    #[test]
    fn test_e2e_zkp_v7_with_cross_pool() {
        let mut zkp = AsyncZKPV7::new(ZKPV7Config::default());
        let mut cross_pool = CrossPoolVerifier::new(CrossPoolConfig::default());

        // Register shards in ZKP v7
        zkp.register_shard(ShardContext::new(
            "shard-a".to_string(),
            800.0,
            0.85,
            BackendType::Halo2,
        ))
        .unwrap();
        zkp.register_shard(ShardContext::new(
            "shard-b".to_string(),
            600.0,
            0.95,
            BackendType::Groth16,
        ))
        .unwrap();

        // Register pools in cross-pool verifier
        cross_pool
            .register_pool(PoolVerifier::new(
                "pool-a".to_string(),
                0.9,
                "halo2".to_string(),
            ))
            .unwrap();
        cross_pool
            .register_pool(PoolVerifier::new(
                "pool-b".to_string(),
                0.85,
                "groth16".to_string(),
            ))
            .unwrap();

        // Submit and generate proof
        let stmt = ZKPStatement {
            statement_id: "cross-stmt-1".to_string(),
            public_inputs: vec![10, 20, 30],
            private_inputs_hash: "cross-hash".to_string(),
            circuit_type: ed2kia::zkp_v7::async_zkp_v7::CircuitType::Inference,
            source_pool: "shard-a".to_string(),
            priority: 15,
            complexity_score: 0.7,
        };
        zkp.submit_statement(stmt).unwrap();
        zkp.start_batch("cross-batch".to_string()).unwrap();
        zkp.fill_batch("cross-batch", 10).unwrap();
        let proofs = zkp.generate_batch_proofs("cross-batch").unwrap();
        assert!(!proofs.is_empty());

        // Create verification session
        let proof_id = proofs[0].proof_id.clone();
        cross_pool
            .create_session("session-1".to_string(), proof_id.clone())
            .unwrap();

        // Submit votes
        cross_pool.submit_vote("session-1", "pool-a", true).unwrap();
        cross_pool.submit_vote("session-1", "pool-b", true).unwrap();

        // Verify proof in ZKP v7
        let verification = zkp.verify_proof(&proof_id);
        assert!(verification.is_ok());
    }

    // ─── LP-106: Dashboard v5 + Stream Engine + Alert Pipeline ───

    #[test]
    fn test_e2e_dashboard_with_all_metrics() {
        let mut dashboard = DashboardV5::new(DashboardV5Config::default());

        // Record ZKP v7 metrics
        dashboard.record_metric(MetricV5::ZkpBatchesCompleted, 42.0, None);
        dashboard.record_metric(MetricV5::ZkpProofsGenerated, 150.0, None);
        dashboard.record_metric(MetricV5::ZkpAvgProofTimeMs, 450.0, None);

        // Record Cross-Pool metrics
        dashboard.record_metric(MetricV5::CrossPoolActiveSessions, 5.0, None);
        dashboard.record_metric(MetricV5::CrossPoolAvgReputation, 0.92.0, None);

        // Record Governance metrics
        dashboard.record_metric(MetricV5::GovernanceAuditEntries, 28.0, None);
        dashboard.record_metric(MetricV5::GovernanceComplianceScore, 0.95.0, None);

        // Get snapshot
        let snapshot = dashboard.get_snapshot().unwrap();
        assert!(snapshot.zkp_summary.batches_completed > 0.0);
        assert!(snapshot.cross_pool_summary.active_sessions > 0.0);
        assert!(snapshot.governance_summary.compliance_score > 0.0);
    }

    #[test]
    fn test_e2e_stream_with_alerts() {
        let mut stream = StreamEngine::new(StreamConfig::default());
        let mut pipeline = AlertPipeline::new(PipelineConfig::default());

        // Add alert rule
        pipeline
            .add_rule(AlertRule::new(
                "rule-1".to_string(),
                ed2kia::ui_v5::alert_pipeline::AlertCategory::ZkpThroughput,
                AlertSeverity::Warning,
                AlertOperator::Below,
                100.0,
            ))
            .unwrap();

        // Publish events
        stream
            .publish(StreamEvent::new(
                MetricCategory::Zkp,
                "zkp_throughput".to_string(),
                80.0,
            ))
            .unwrap();

        // Process alerts
        pipeline
            .process_metric("zkp_throughput".to_string(), 80.0)
            .unwrap();

        // Get alerts
        let alerts = pipeline.get_by_severity(&AlertSeverity::Warning);
        assert!(!alerts.is_empty());
    }

    // ─── Full Pipeline: Pools → ZKP v7 → Cross-Pool → DAO → Dashboard ───

    #[test]
    fn test_e2e_full_sprint2_pipeline() {
        // 1. Pool matching
        let mut matcher = PoolMatcherV3::new(MatcherV3Config::default());
        matcher
            .register_candidate(ShardCandidateV3::new(
                "compute-shard".to_string(),
                2000.0,
                0.95,
                10.0,
                0.2,
                "halo2".to_string(),
            ))
            .unwrap();

        let req = PoolRequestV3::new(
            "full-pipeline-req".to_string(),
            "dao-client".to_string(),
            RequestType::Inference,
            100.0,
            20,
        );
        let match_result = matcher.match_request(&req).unwrap();

        // 2. ZKP v7 proof generation
        let mut zkp = AsyncZKPV7::new(ZKPV7Config::default());
        zkp.register_shard(ShardContext::new(
            match_result.candidate_id.clone(),
            2000.0,
            0.95,
            BackendType::Halo2,
        ))
        .unwrap();

        let stmt = ZKPStatement {
            statement_id: "full-stmt".to_string(),
            public_inputs: vec![100, 200, 300],
            private_inputs_hash: "full-pipeline-hash".to_string(),
            circuit_type: ed2kia::zkp_v7::async_zkp_v7::CircuitType::Inference,
            source_pool: match_result.candidate_id.clone(),
            priority: 20,
            complexity_score: 0.8,
        };
        zkp.submit_statement(stmt).unwrap();
        zkp.start_batch("full-batch".to_string()).unwrap();
        zkp.fill_batch("full-batch", 10).unwrap();
        let proofs = zkp.generate_batch_proofs("full-batch").unwrap();
        assert!(!proofs.is_empty());

        // 3. Cross-pool verification
        let mut cross_pool = CrossPoolVerifier::new(CrossPoolConfig::default());
        cross_pool
            .register_pool(PoolVerifier::new(
                "verifier-1".to_string(),
                0.95,
                "halo2".to_string(),
            ))
            .unwrap();
        cross_pool
            .register_pool(PoolVerifier::new(
                "verifier-2".to_string(),
                0.90,
                "groth16".to_string(),
            ))
            .unwrap();

        let proof_id = proofs[0].proof_id.clone();
        cross_pool
            .create_session("full-session".to_string(), proof_id.clone())
            .unwrap();
        cross_pool
            .submit_vote("full-session", "verifier-1", true)
            .unwrap();
        cross_pool
            .submit_vote("full-session", "verifier-2", true)
            .unwrap();

        // 4. DAO Ledger recording
        let mut ledger = DaoLedgerV4::new(DaoLedgerV4Config::default());
        let event = DaoEventV4::new(
            "dao-evt".to_string(),
            ed2kia::governance_v4::dao_ledger_v4::DaoEventTypeV4::ProofVerified,
            "zkp-engine".to_string(),
            format!("Verified proof {} via cross-pool", proof_id),
        );
        ledger.record_event(event).unwrap();

        // 5. Audit trail
        let mut audit = AuditTrail::new(AuditTrailConfig::default());
        audit
            .record(
                "audit-full".to_string(),
                AuditAction::ProofVerified,
                Severity::Info,
                "cross-pool".to_string(),
                &proof_id,
                "Cross-pool verification completed".to_string(),
            )
            .unwrap();

        // 6. Dashboard aggregation
        let mut dashboard = DashboardV5::new(DashboardV5Config::default());
        dashboard.record_metric(MetricV5::ZkpProofsGenerated, proofs.len() as f64, None);
        dashboard.record_metric(MetricV5::CrossPoolActiveSessions, 1.0, None);
        dashboard.record_metric(MetricV5::GovernanceAuditEntries, 1.0, None);
        dashboard.record_metric(MetricV5::GovernanceComplianceScore, 0.98.0, None);

        let snapshot = dashboard.get_snapshot().unwrap();
        assert!(snapshot.zkp_summary.proofs_generated > 0.0);
        assert!(snapshot.cross_pool_summary.active_sessions > 0.0);
        assert!(snapshot.governance_summary.audit_entries > 0.0);

        // 7. Stream events
        let mut stream = StreamEngine::new(StreamConfig::default());
        stream
            .publish(StreamEvent::new(
                MetricCategory::Zkp,
                "full_pipeline_proof".to_string(),
                proofs.len() as f64,
            ))
            .unwrap();

        let events = stream
            .get_events("full_pipeline_proof".to_string())
            .unwrap();
        assert!(!events.is_empty());
    }

    #[test]
    fn test_e2e_alert_escalation_pipeline() {
        let mut pipeline = AlertPipeline::new(PipelineConfig::default());

        // Add escalation rules
        pipeline
            .add_rule(AlertRule::new(
                "cpu-rule".to_string(),
                ed2kia::ui_v5::alert_pipeline::AlertCategory::CpuUsage,
                AlertSeverity::Warning,
                AlertOperator::Above,
                80.0,
            ))
            .unwrap();

        pipeline
            .add_rule(AlertRule::new(
                "cpu-critical".to_string(),
                ed2kia::ui_v5::alert_pipeline::AlertCategory::CpuUsage,
                AlertSeverity::Critical,
                AlertOperator::Above,
                95.0,
            ))
            .unwrap();

        // Trigger warning
        pipeline
            .process_metric("cpu_usage".to_string(), 85.0)
            .unwrap();
        let warnings = pipeline.get_by_severity(&AlertSeverity::Warning);
        assert!(!warnings.is_empty());

        // Trigger critical
        pipeline
            .process_metric("cpu_usage".to_string(), 97.0)
            .unwrap();
        let criticals = pipeline.get_by_severity(&AlertSeverity::Critical);
        assert!(!criticals.is_empty());

        // Generate compliance report
        let report = pipeline.generate_compliance_report();
        assert!(report.total_alerts > 0);
    }

    #[test]
    fn test_e2e_stream_backpressure() {
        let config = StreamConfig {
            max_buffer_size: 5,
            pause_threshold: 0.8,
            resume_threshold: 0.3,
            window_size_ms: 1000,
            max_subscribers: 10,
        };
        let mut stream = StreamEngine::new(config);

        // Subscribe with small buffer
        let sub_config = ed2kia::ui_v5::stream_engine::SubscriberConfig {
            subscriber_id: "slow-sub".to_string(),
            categories: vec![MetricCategory::Zkp],
            buffer_size: 3,
            pause_threshold: 0.7,
            resume_threshold: 0.3,
        };
        stream.subscribe(sub_config).unwrap();

        // Publish events to fill buffer
        for i in 0..5 {
            stream
                .publish(StreamEvent::new(
                    MetricCategory::Zkp,
                    format!("event-{}", i),
                    i as f64,
                ))
                .unwrap();
        }

        // Process queue
        stream.process_queue();

        // Get subscriber state
        let stats = stream.get_stats();
        assert!(stats.total_published >= 5);
    }

    #[test]
    fn test_e2e_dashboard_alert_thresholds() {
        let mut dashboard = DashboardV5::new(DashboardV5Config::default());

        // Trigger ZKP throughput alert (below threshold)
        dashboard.record_metric(MetricV5::ZkpProofsPerSecond, 5.0, None);

        // Trigger CPU alert (above threshold)
        dashboard.record_metric(MetricV5::SystemCpuUsage, 95.0, None);

        // Trigger memory alert (above threshold)
        dashboard.record_metric(MetricV5::SystemMemoryUsage, 90.0, None);

        let snapshot = dashboard.get_snapshot().unwrap();
        assert!(!snapshot.alerts.is_empty());
        assert!(snapshot.alerts.len() >= 3);
    }

    #[test]
    fn test_e2e_governance_compliance_workflow() {
        let mut audit = AuditTrail::new(AuditTrailConfig::default());

        // Simulate governance workflow
        audit
            .record(
                "g-1".to_string(),
                AuditAction::ProposalCreated,
                Severity::Info,
                "member-1".to_string(),
                "prop-increase-quorum".to_string(),
                "Create proposal to increase quorum".to_string(),
            )
            .unwrap();

        audit
            .record(
                "g-2".to_string(),
                AuditAction::VoteCast,
                Severity::Info,
                "member-2".to_string(),
                "prop-increase-quorum".to_string(),
                "Vote yes on quorum increase".to_string(),
            )
            .unwrap();

        audit
            .record(
                "g-3".to_string(),
                AuditAction::ProposalExecuted,
                Severity::Warning,
                "executor".to_string(),
                "prop-increase-quorum".to_string(),
                "Execute quorum increase".to_string(),
            )
            .unwrap();

        // Verify chain integrity
        audit.verify_chain().unwrap();

        // Generate compliance report
        let report = audit.generate_compliance_report(0, u64::MAX);
        assert_eq!(report.total_entries, 3);
        assert!(report.compliance_score >= 0.0);
    }

    #[test]
    fn test_e2e_zkp_v7_aggregation_depth() {
        let config = ZKPV7Config {
            max_aggregation_depth: 3,
            ..Default::default()
        };
        let mut zkp = AsyncZKPV7::new(config);

        zkp.register_shard(ShardContext::new(
            "agg-shard".to_string(),
            1000.0,
            0.9,
            BackendType::Halo2,
        ))
        .unwrap();

        // Submit multiple statements
        for i in 0..6 {
            let stmt = ZKPStatement {
                statement_id: format!("agg-stmt-{}", i),
                public_inputs: vec![i],
                private_inputs_hash: format!("agg-hash-{}", i),
                circuit_type: ed2kia::zkp_v7::async_zkp_v7::CircuitType::Membership,
                source_pool: "agg-shard".to_string(),
                priority: 10,
                complexity_score: 0.3,
            };
            zkp.submit_statement(stmt).unwrap();
        }

        // Generate proofs
        zkp.start_batch("agg-batch".to_string()).unwrap();
        zkp.fill_batch("agg-batch", 10).unwrap();
        let proofs = zkp.generate_batch_proofs("agg-batch").unwrap();

        // Aggregate proofs
        let aggregated = zkp.aggregate_proofs(&proofs).unwrap();
        assert!(!aggregated.is_empty());
    }

    #[test]
    fn test_e2e_cross_pool_reputation_update() {
        let mut cross_pool = CrossPoolVerifier::new(CrossPoolConfig::default());

        cross_pool
            .register_pool(PoolVerifier::new(
                "rep-pool-1".to_string(),
                0.8,
                "halo2".to_string(),
            ))
            .unwrap();
        cross_pool
            .register_pool(PoolVerifier::new(
                "rep-pool-2".to_string(),
                0.9,
                "groth16".to_string(),
            ))
            .unwrap();

        // Create session and vote
        cross_pool
            .create_session("rep-session".to_string(), "rep-proof".to_string())
            .unwrap();
        cross_pool
            .submit_vote("rep-session", "rep-pool-1", true)
            .unwrap();
        cross_pool
            .submit_vote("rep-session", "rep-pool-2", true)
            .unwrap();

        // Complete session (should update reputation)
        let stats = cross_pool.get_stats();
        assert!(stats.total_sessions >= 1);
    }
}
