//! v1.4.0 Sprint 2 Benchmarks
//!
//! Performance benchmarks for LP-103 through LP-106 modules.
//! Run with: cargo bench --features v1.4-sprint2

#[cfg(feature = "v1.4-sprint2")]
mod bench {
    use std::time::Instant;

    // LP-103: Cross-Chain Pools v3
    use ed2kia::pools_v3::pool_matcher_v3::{
        MatcherV3Config, PoolMatcherV3, PoolRequestV3, RequestType, ShardCandidateV3,
    };

    // LP-104: DAO Ledger v4
    use ed2kia::governance_v4::audit_trail::{AuditAction, AuditTrail, AuditTrailConfig, Severity};
    use ed2kia::governance_v4::dao_ledger_v4::{DaoEventV4, DaoLedgerV4, DaoLedgerV4Config};
    use ed2kia::governance_v4::hybrid_executor::{ExecutionMode, HybridConfig, HybridExecutor};

    // LP-105: Async ZKP v7
    use ed2kia::zkp_v7::async_zkp_v7::{
        AsyncZKPV7, BackendType, CircuitType, ShardContext, ZKPStatement, ZKPV7Config,
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

    // =====================================================================
    // Helpers
    // =====================================================================

    fn make_candidate(id: &str, credits: f64) -> ShardCandidateV3 {
        ShardCandidateV3::new(id.to_string(), credits, 0.9, 15.0, 0.3, "halo2".to_string())
    }

    fn make_request(id: &str) -> PoolRequestV3 {
        PoolRequestV3::new(
            id.to_string(),
            "bench-client".to_string(),
            RequestType::Inference,
            50.0,
            10,
        )
    }

    fn make_zkp_statement(id: &str) -> ZKPStatement {
        ZKPStatement {
            statement_id: id.to_string(),
            public_inputs: vec![1, 2, 3],
            private_inputs_hash: format!("hash-{}", id),
            circuit_type: CircuitType::Membership,
            source_pool: "bench-shard".to_string(),
            priority: 10,
            complexity_score: 0.5,
        }
    }

    // =====================================================================
    // LP-103: Pool Matcher v3 Benchmarks
    // =====================================================================

    pub fn bench_pool_matching_1000() {
        let mut matcher = PoolMatcherV3::new(MatcherV3Config::default());

        // Register 20 candidates
        for i in 0..20 {
            matcher
                .register_candidate(make_candidate(
                    &format!("shard-{}", i),
                    1000.0 + i as f64 * 100.0,
                ))
                .ok();
        }

        let start = Instant::now();
        for i in 0..1000 {
            let req = make_request(&format!("req-{}", i));
            matcher.match_request(&req).ok();
        }
        let elapsed = start.elapsed();
        println!(
            "LP-103: 1000 pool matches in {:?} ({:.3} ms/match)",
            elapsed,
            elapsed.as_secs_f64() * 1000.0 / 1000.0
        );
    }

    pub fn bench_pool_matching_100_candidates() {
        let mut matcher = PoolMatcherV3::new(MatcherV3Config::default());

        for i in 0..100 {
            matcher
                .register_candidate(make_candidate(&format!("shard-{}", i), 500.0))
                .ok();
        }

        let start = Instant::now();
        for i in 0..100 {
            let req = make_request(&format!("req-{}", i));
            matcher.match_request(&req).ok();
        }
        let elapsed = start.elapsed();
        println!(
            "LP-103: 100 matches with 100 candidates in {:?} ({:.3} ms/match)",
            elapsed,
            elapsed.as_secs_f64() * 1000.0 / 100.0
        );
    }

    // =====================================================================
    // LP-104: DAO Ledger v4 Benchmarks
    // =====================================================================

    pub fn bench_dao_ledger_1000_events() {
        let mut ledger = DaoLedgerV4::new(DaoLedgerV4Config::default());

        let start = Instant::now();
        for i in 0..1000 {
            let event = DaoEventV4::new(
                format!("evt-{}", i),
                ed2kia::governance_v4::dao_ledger_v4::DaoEventTypeV4::ProposalCreated,
                format!("actor-{}", i % 10),
                format!("Event payload {}", i),
            );
            ledger.record_event(event).ok();
        }
        let elapsed = start.elapsed();
        println!(
            "LP-104: 1000 DAO events in {:?} ({:.3} ms/event)",
            elapsed,
            elapsed.as_secs_f64() * 1000.0 / 1000.0
        );
    }

    pub fn bench_audit_trail_500_entries() {
        let mut audit = AuditTrail::new(AuditTrailConfig::default());

        let start = Instant::now();
        for i in 0..500 {
            audit
                .record(
                    format!("audit-{}", i),
                    AuditAction::ProposalCreated,
                    Severity::Info,
                    format!("actor-{}", i % 5),
                    format!("target-{}", i % 20),
                    format!("Audit payload {}", i),
                )
                .ok();
        }
        let elapsed = start.elapsed();
        println!(
            "LP-104: 500 audit entries in {:?} ({:.3} ms/entry)",
            elapsed,
            elapsed.as_secs_f64() * 1000.0 / 500.0
        );
    }

    pub fn bench_hybrid_executor_200_executions() {
        let mut executor = HybridExecutor::new(HybridConfig::default());

        let start = Instant::now();
        for i in 0..200 {
            executor
                .execute(
                    format!("prop-{}", i),
                    if i % 2 == 0 {
                        ExecutionMode::OnChain
                    } else {
                        ExecutionMode::OffChain
                    },
                    format!("Execute proposal {}", i),
                )
                .ok();
        }
        let elapsed = start.elapsed();
        println!(
            "LP-104: 200 hybrid executions in {:?} ({:.3} ms/exec)",
            elapsed,
            elapsed.as_secs_f64() * 1000.0 / 200.0
        );
    }

    // =====================================================================
    // LP-105: ZKP v7 Benchmarks
    // =====================================================================

    pub fn bench_zkp_v7_proof_generation_100() {
        let mut zkp = AsyncZKPV7::new(ZKPV7Config::default());
        zkp.register_shard(ShardContext::new(
            "bench-shard".to_string(),
            5000.0,
            0.95,
            BackendType::Halo2,
        ))
        .ok();

        let start = Instant::now();
        for i in 0..100 {
            let stmt = make_zkp_statement(&format!("zkp-{}", i));
            zkp.submit_statement(stmt).ok();
        }
        zkp.start_batch("bench-batch".to_string()).ok();
        zkp.fill_batch("bench-batch", 100).ok();
        let proofs = zkp.generate_batch_proofs("bench-batch").unwrap();
        let elapsed = start.elapsed();
        println!(
            "LP-105: 100 ZKP v7 proofs in {:?} ({:.3} ms/proof)",
            elapsed,
            elapsed.as_secs_f64() * 1000.0 / proofs.len() as f64
        );
    }

    pub fn bench_zkp_v7_aggregation_50() {
        let mut zkp = AsyncZKPV7::new(ZKPV7Config::default());
        zkp.register_shard(ShardContext::new(
            "agg-shard".to_string(),
            3000.0,
            0.9,
            BackendType::Halo2,
        ))
        .ok();

        for i in 0..50 {
            zkp.submit_statement(make_zkp_statement(&format!("agg-{}", i)))
                .ok();
        }
        zkp.start_batch("agg-batch".to_string()).ok();
        zkp.fill_batch("agg-batch", 50).ok();
        let proofs = zkp.generate_batch_proofs("agg-batch").unwrap();

        let start = Instant::now();
        let aggregated = zkp.aggregate_proofs(&proofs).unwrap();
        let elapsed = start.elapsed();
        println!(
            "LP-105: Aggregate {} proofs in {:?} ({:.3} ms/proof)",
            proofs.len(),
            elapsed,
            elapsed.as_secs_f64() * 1000.0 / proofs.len() as f64
        );
    }

    pub fn bench_cross_pool_verification_50_sessions() {
        let mut cross_pool = CrossPoolVerifier::new(CrossPoolConfig::default());

        for i in 0..10 {
            cross_pool
                .register_pool(PoolVerifier::new(
                    format!("pool-{}", i),
                    0.9,
                    "halo2".to_string(),
                ))
                .ok();
        }

        let start = Instant::now();
        for i in 0..50 {
            cross_pool
                .create_session(format!("session-{}", i), format!("proof-{}", i))
                .ok();
            // Submit votes from all pools
            for j in 0..10 {
                cross_pool
                    .submit_vote(&format!("session-{}", i), &format!("pool-{}", j), true)
                    .ok();
            }
        }
        let elapsed = start.elapsed();
        println!(
            "LP-105: 50 cross-pool sessions (10 pools each) in {:?} ({:.3} ms/session)",
            elapsed,
            elapsed.as_secs_f64() * 1000.0 / 50.0
        );
    }

    // =====================================================================
    // LP-106: Dashboard v5 Benchmarks
    // =====================================================================

    pub fn bench_dashboard_1000_metrics() {
        let mut dashboard = DashboardV5::new(DashboardV5Config::default());

        let start = Instant::now();
        for i in 0..1000 {
            let metric = match i % 8 {
                0 => MetricV5::ZkpProofsGenerated,
                1 => MetricV5::ZkpBatchesCompleted,
                2 => MetricV5::CrossPoolActiveSessions,
                3 => MetricV5::CrossPoolAvgReputation,
                4 => MetricV5::GovernanceAuditEntries,
                5 => MetricV5::GovernanceComplianceScore,
                6 => MetricV5::SystemCpuUsage,
                _ => MetricV5::SystemMemoryUsage,
            };
            dashboard.record_metric(metric, (i % 100) as f64, None);
        }
        let elapsed = start.elapsed();
        println!(
            "LP-106: 1000 dashboard metrics in {:?} ({:.3} ms/metric)",
            elapsed,
            elapsed.as_secs_f64() * 1000.0 / 1000.0
        );
    }

    pub fn bench_stream_engine_500_events() {
        let mut stream = StreamEngine::new(StreamConfig::default());

        // Add 3 subscribers
        for i in 0..3 {
            let sub_config = ed2kia::ui_v5::stream_engine::SubscriberConfig {
                subscriber_id: format!("sub-{}", i),
                categories: vec![MetricCategory::Zkp, MetricCategory::System],
                buffer_size: 50,
                pause_threshold: 0.8,
                resume_threshold: 0.3,
            };
            stream.subscribe(sub_config).ok();
        }

        let start = Instant::now();
        for i in 0..500 {
            let category = match i % 3 {
                0 => MetricCategory::Zkp,
                1 => MetricCategory::System,
                _ => MetricCategory::Network,
            };
            stream
                .publish(StreamEvent::new(category, format!("event-{}", i), i as f64))
                .ok();
        }
        stream.process_queue();
        let elapsed = start.elapsed();
        println!(
            "LP-106: 500 stream events (3 subs) in {:?} ({:.3} ms/event)",
            elapsed,
            elapsed.as_secs_f64() * 1000.0 / 500.0
        );
    }

    pub fn bench_alert_pipeline_200_metrics() {
        let mut pipeline = AlertPipeline::new(PipelineConfig::default());

        pipeline
            .add_rule(AlertRule::new(
                "rule-cpu".to_string(),
                ed2kia::ui_v5::alert_pipeline::AlertCategory::CpuUsage,
                AlertSeverity::Warning,
                AlertOperator::Above,
                80.0,
            ))
            .ok();

        pipeline
            .add_rule(AlertRule::new(
                "rule-mem".to_string(),
                ed2kia::ui_v5::alert_pipeline::AlertCategory::MemoryUsage,
                AlertSeverity::Critical,
                AlertOperator::Above,
                90.0,
            ))
            .ok();

        let start = Instant::now();
        for i in 0..200 {
            pipeline
                .process_metric(format!("metric-{}", i % 5), (i % 100) as f64)
                .ok();
        }
        let elapsed = start.elapsed();
        println!(
            "LP-106: 200 alert pipeline metrics in {:?} ({:.3} ms/metric)",
            elapsed,
            elapsed.as_secs_f64() * 1000.0 / 200.0
        );
    }

    // =====================================================================
    // Combined Pipeline Benchmark
    // =====================================================================

    pub fn bench_full_sprint2_pipeline() {
        let start = Instant::now();

        // Pool matching
        let mut matcher = PoolMatcherV3::new(MatcherV3Config::default());
        matcher
            .register_candidate(make_candidate("shard-1", 2000.0))
            .ok();
        let req = make_request("bench-req");
        matcher.match_request(&req).ok();

        // ZKP v7
        let mut zkp = AsyncZKPV7::new(ZKPV7Config::default());
        zkp.register_shard(ShardContext::new(
            "shard-1".to_string(),
            2000.0,
            0.95,
            BackendType::Halo2,
        ))
        .ok();
        zkp.submit_statement(make_zkp_statement("bench-stmt")).ok();
        zkp.start_batch("bench-batch".to_string()).ok();
        zkp.fill_batch("bench-batch", 10).ok();
        let proofs = zkp.generate_batch_proofs("bench-batch").unwrap();

        // Cross-pool
        let mut cross_pool = CrossPoolVerifier::new(CrossPoolConfig::default());
        cross_pool
            .register_pool(PoolVerifier::new(
                "v1".to_string(),
                0.95,
                "halo2".to_string(),
            ))
            .ok();
        cross_pool
            .create_session("bench-session".to_string(), proofs[0].proof_id.clone())
            .ok();
        cross_pool.submit_vote("bench-session", "v1", true).ok();

        // DAO Ledger
        let mut ledger = DaoLedgerV4::new(DaoLedgerV4Config::default());
        ledger
            .record_event(DaoEventV4::new(
                "bench-evt".to_string(),
                ed2kia::governance_v4::dao_ledger_v4::DaoEventTypeV4::ProofVerified,
                "bench".to_string(),
                "Benchmark event".to_string(),
            ))
            .ok();

        // Dashboard
        let mut dashboard = DashboardV5::new(DashboardV5Config::default());
        dashboard.record_metric(MetricV5::ZkpProofsGenerated, proofs.len() as f64, None);
        dashboard.get_snapshot().ok();

        let elapsed = start.elapsed();
        println!("LP-107: Full Sprint 2 pipeline in {:?}", elapsed);
    }
}
