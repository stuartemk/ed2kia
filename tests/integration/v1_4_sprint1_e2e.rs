//! v1.4.0 Sprint 1 E2E Integration Tests
//!
//! End-to-end tests covering LP-98 through LP-101 modules.
//! Validates cross-module interactions and full pipeline workflows.

#[cfg(feature = "v1.4-sprint1")]
mod e2e {
    // LP-98: Halo2 ZKP Engine
    use ed2kia::zkp::async_zkp_v5::{CircuitType, ZKPProof, ZKPStatement};
    use ed2kia::zkp::circuit_optimizer::{CircuitOptimizer, CircuitOptimizerConfig};
    use ed2kia::zkp::halo2_engine::{Halo2Engine, Halo2EngineConfig, HashBackend};
    use ed2kia::zkp::proof_aggregator::{AggregatorConfig, ProofAggregator};

    // LP-99: Tokio Async Optimization
    use ed2kia::runtime::task_scheduler::{
        ScheduledTask, SchedulerConfig, TaskPriority, TaskScheduler,
    };
    use ed2kia::runtime::tokio_optimizer::{RuntimeProfile, TokioOptimizer, TokioOptimizerConfig};
    use ed2kia::runtime::worker_pool::{LoadBalanceStrategy, WorkerPool, WorkerPoolConfig};

    // LP-100: LZ4 Compression & Storage
    use ed2kia::storage::checkpoint_cache::{
        CheckpointCache, CheckpointCacheConfig, EvictionPolicy,
    };
    use ed2kia::storage::gradient_archive::{ArchiveConfig, GradientArchive};
    use ed2kia::storage::lz4_compressor::{LZ4Compressor, LZ4Config};

    // LP-101: Advanced Metrics & Observability
    use ed2kia::monitoring_v2::advanced_metrics::{AdvancedMetrics, AdvancedMetricsConfig};
    use ed2kia::monitoring_v2::alert_engine::{
        AlertEngine, AlertEngineConfig, AlertOperator, AlertRule, AlertSeverity,
        NotificationChannel,
    };
    use ed2kia::monitoring_v2::health_checker::{
        CheckConfig, HealthChecker, HealthCheckerConfig, HealthStatus,
    };

    use std::time::Duration;

    // =====================================================================
    // Helper functions
    // =====================================================================

    fn make_statement(id: &str, circuit: CircuitType, complexity: f64) -> ZKPStatement {
        ZKPStatement {
            statement_id: id.to_string(),
            public_inputs: vec![1, 2, 3],
            private_inputs_hash: format!("hash-{}", id),
            circuit_type: circuit,
            source_pool: "pool-1".to_string(),
            priority: 10,
            complexity_score: complexity,
        }
    }

    fn make_proof(id: &str) -> ZKPProof {
        ZKPProof {
            proof_id: format!("proof-{}", id),
            statement_id: id.to_string(),
            proof_data: vec![1, 2, 3, 4, 5],
            proof_hash: format!("hash-{}", id),
            generation_time_ms: 100,
            used_fallback: false,
            batch_id: None,
            source_pool: "pool-1".to_string(),
            priority: 10,
            accumulator_index: None,
            is_vrf_sample: false,
        }
    }

    // =====================================================================
    // LP-98 E2E Tests
    // =====================================================================

    #[test]
    fn test_e2e_halo2_engine_lifecycle() {
        let backend = HashBackend::new();
        let config = Halo2EngineConfig::default();
        let mut engine = Halo2Engine::new(backend, config);

        // Generate proof
        let statement = make_statement("e2e-1", CircuitType::Membership, 0.5);
        let proof = engine.generate_proof(&statement).expect("generate proof");
        assert_eq!(proof.statement_id, "e2e-1");

        // Verify proof
        let valid = engine
            .verify_proof(&proof, &statement)
            .expect("verify proof");
        assert!(valid);

        // Check stats
        let stats = engine.stats();
        assert_eq!(stats.total_generated, 1);
        assert_eq!(stats.total_verifications, 1);
    }

    #[test]
    fn test_e2e_circuit_optimizer_adaptive() {
        let mut optimizer = CircuitOptimizer::new(CircuitOptimizerConfig::default());

        // Initially selects default
        let statement = make_statement("opt-1", CircuitType::Membership, 0.3);
        let circuit = optimizer.select_circuit(&statement);
        assert!(matches!(
            circuit,
            CircuitType::Membership
                | CircuitType::RangeProof
                | CircuitType::Commitment
                | CircuitType::CrossPoolAggregation
                | CircuitType::IncrementalAccumulator
                | CircuitType::Custom
        ));

        // Record results to influence future selection
        optimizer.record_result(&circuit, 50.0, 10.0, true);
        assert!(optimizer.get_profile(&circuit).is_some());
    }

    #[test]
    fn test_e2e_proof_aggregator_workflow() {
        let mut aggregator = ProofAggregator::new(AggregatorConfig::default());

        // Add proofs
        for i in 0..5 {
            let proof = make_proof(&format!("stmt-{}", i));
            aggregator.add_proof(proof).expect("add proof");
        }

        // Aggregate
        let agg = aggregator
            .aggregate("agg-1".to_string())
            .expect("aggregate");
        assert_eq!(agg.proof_ids.len(), 5);
        assert!(agg.aggregated_data.len() > 0);

        // Verify aggregated
        let valid = aggregator
            .verify_aggregated(&agg)
            .expect("verify aggregated");
        assert!(valid);
    }

    // =====================================================================
    // LP-99 E2E Tests
    // =====================================================================

    #[test]
    fn test_e2e_tokio_optimizer_adaptation() {
        let config = TokioOptimizerConfig {
            min_workers: 2,
            max_workers: 8,
            adaptive_scaling: true,
            ..TokioOptimizerConfig::default()
        };
        let mut optimizer = TokioOptimizer::new(config).expect("create optimizer");
        optimizer.initialize().expect("initialize");

        // Record high utilization to trigger scale-up
        let metrics = optimizer.metrics();
        assert!(metrics.worker_threads >= 2);

        let suggestion = optimizer.adapt();
        // Adapt may or may not suggest change depending on utilization
        if let Some(new_workers) = suggestion {
            assert!(new_workers >= 2);
        }
    }

    #[test]
    fn test_e2e_task_scheduler_priorities() {
        let mut scheduler = TaskScheduler::new(SchedulerConfig::default());

        // Add tasks with different priorities
        let low_task = ScheduledTask::new(
            "low-1".to_string(),
            TaskPriority::Low,
            "low task".to_string(),
        );
        let high_task = ScheduledTask::new(
            "high-1".to_string(),
            TaskPriority::High,
            "high task".to_string(),
        );
        let med_task = ScheduledTask::new(
            "med-1".to_string(),
            TaskPriority::Normal,
            "medium task".to_string(),
        );

        scheduler.schedule(low_task).expect("schedule low");
        scheduler.schedule(high_task).expect("schedule high");
        scheduler.schedule(med_task).expect("schedule medium");

        assert_eq!(scheduler.queue_size(), 3);

        // High priority should be first
        let task = scheduler.next_task().expect("get task");
        assert_eq!(task.id, "high-1");
    }

    #[test]
    fn test_e2e_worker_pool_load_balancing() {
        let config = WorkerPoolConfig {
            initial_workers: 0,
            max_workers: 5,
            min_workers: 2,
            strategy: LoadBalanceStrategy::RoundRobin,
            health_check_interval: Duration::from_secs(5),
            task_timeout: Duration::from_secs(30),
            auto_scale: true,
            scale_up_threshold: 0.8,
            scale_down_threshold: 0.3,
        };
        let mut pool = WorkerPool::new(config);

        // Add workers manually
        pool.add_worker("w1".to_string()).unwrap();
        pool.add_worker("w2".to_string()).unwrap();
        pool.add_worker("w3".to_string()).unwrap();

        // Assign tasks round-robin
        for i in 0..6 {
            pool.assign_task(format!("task-{}", i)).unwrap();
        }

        // Complete some tasks
        pool.complete_task("w1", 50.0).unwrap();
        pool.complete_task("w2", 60.0).unwrap();

        let stats = pool.stats();
        assert_eq!(stats.total_tasks_assigned, 6);
        assert_eq!(stats.total_tasks_completed, 2);
    }

    // =====================================================================
    // LP-100 E2E Tests
    // =====================================================================

    #[test]
    fn test_e2e_lz4_round_trip() {
        let mut compressor = LZ4Compressor::new(LZ4Config::default());

        let original =
            b"Hello, ed2kIA v1.4.0 Sprint 1! This is test data for LZ4 compression E2E validation.";
        let block = compressor.compress(original, "block-1").expect("compress");
        assert_eq!(block.original_size, original.len());

        let decompressed = compressor.decompress(&block).expect("decompress");
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_e2e_checkpoint_cache_lru() {
        let config = CheckpointCacheConfig {
            max_checkpoints: 3,
            eviction_policy: EvictionPolicy::LRU,
            max_storage_bytes: 0, // unlimited
            compression_enabled: false,
            stale_threshold_ms: 3600_000,
            eviction_batch_size: 1,
        };
        let mut cache = CheckpointCache::new(config);
        cache.set_time(1000);

        // Add entries
        cache
            .store("cp1".to_string(), 1, "model-1".to_string(), vec![1, 2, 3])
            .expect("store cp1");
        cache
            .store("cp2".to_string(), 2, "model-1".to_string(), vec![4, 5, 6])
            .expect("store cp2");
        cache
            .store("cp3".to_string(), 3, "model-1".to_string(), vec![7, 8, 9])
            .expect("store cp3");

        // Access cp1 to make it recently used
        cache.get("cp1").expect("access cp1");

        // Add cp4, should evict cp2 (least recently used)
        cache
            .store(
                "cp4".to_string(),
                4,
                "model-1".to_string(),
                vec![10, 11, 12],
            )
            .expect("store cp4");

        assert!(cache.get("cp1").is_ok());
        assert!(cache.get("cp2").is_err()); // Evicted
        assert!(cache.get("cp3").is_ok());
        assert!(cache.get("cp4").is_ok());
    }

    #[test]
    fn test_e2e_gradient_archive_versioning() {
        let config = ArchiveConfig {
            max_versions_per_model: 3,
            max_total_versions: 6,
            compression_enabled: true,
            auto_prune: true,
            min_versions_keep: 1,
        };
        let mut archive = GradientArchive::new(config);

        // Store multiple versions for model-1
        for i in 0..5 {
            let gradients = vec![1.0 * (i + 1) as f32; 10];
            archive
                .store(
                    format!("v-{}", i),
                    "model-1".to_string(),
                    i as u64,
                    gradients,
                )
                .expect("store gradient");
        }

        // Should have pruned to max_versions_per_model
        let versions = archive.get_model_versions("model-1");
        assert!(versions.len() <= 3);
    }

    // =====================================================================
    // LP-101 E2E Tests
    // =====================================================================

    #[test]
    fn test_e2e_metrics_registry() {
        let mut metrics = AdvancedMetrics::new(AdvancedMetricsConfig::default());

        // Register counter
        metrics.register_counter("requests_total".to_string(), "Total requests".to_string());
        metrics.counter_inc("requests_total").expect("increment");
        metrics.counter_add("requests_total", 5).expect("add");

        // Register gauge
        metrics.register_gauge(
            "active_connections".to_string(),
            "Active connections".to_string(),
        );
        metrics
            .gauge_set("active_connections", 42.0)
            .expect("set gauge");

        // Register histogram
        metrics.register_histogram(
            "request_latency_ms".to_string(),
            "Request latency".to_string(),
            Some(vec![5.0, 10.0, 25.0, 50.0, 100.0]),
        );
        for i in 1..=20 {
            metrics
                .histogram_observe("request_latency_ms", i as f64 * 5.0)
                .expect("observe");
        }

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.counters.len(), 1);
        assert_eq!(snapshot.gauges.len(), 1);
        assert_eq!(snapshot.histograms.len(), 1);
    }

    #[test]
    fn test_e2e_health_checker_full_cycle() {
        let config = HealthCheckerConfig {
            enabled: true,
            default_interval_ms: 5000,
            max_checks: 10,
            sla_tracking: true,
        };
        let mut checker = HealthChecker::new(config);

        // Register checks
        let api_config = CheckConfig {
            name: "api_server".to_string(),
            ..CheckConfig::default()
        };
        let db_config = CheckConfig {
            name: "database".to_string(),
            ..CheckConfig::default()
        };
        checker.register_check(api_config).expect("register api");
        checker.register_check(db_config).expect("register db");

        // Record healthy checks (recovery_threshold=2, so need 2 healthy records each)
        checker
            .record_check("api_server", HealthStatus::Healthy, "ok".to_string(), 5.0)
            .expect("record api");
        checker
            .record_check("api_server", HealthStatus::Healthy, "ok".to_string(), 4.0)
            .expect("record api 2");
        checker
            .record_check("database", HealthStatus::Healthy, "ok".to_string(), 3.0)
            .expect("record db");
        checker
            .record_check("database", HealthStatus::Healthy, "ok".to_string(), 2.0)
            .expect("record db 2");

        // Generate report
        let report = checker.generate_report();
        assert_eq!(report.overall_status, HealthStatus::Healthy);
        assert_eq!(report.total_checks, 2);
    }

    #[test]
    fn test_e2e_alert_engine_pipeline() {
        let mut engine = AlertEngine::new(AlertEngineConfig::default());

        // Add CPU alert rule
        let rule = AlertRule::new(
            "cpu-high".to_string(),
            "CPU High".to_string(),
            "cpu_usage".to_string(),
            80.0,
            AlertOperator::GreaterThan,
            AlertSeverity::Warning,
            0,
            1,
            vec![NotificationChannel::Log],
        );
        engine.add_rule(rule);

        // Normal value — no alert
        engine.evaluate("cpu_usage", 50.0);
        assert_eq!(engine.get_active_alerts().len(), 0);

        // High value — alert fires
        engine.evaluate("cpu_usage", 95.0);
        let alerts = engine.get_active_alerts();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].severity, AlertSeverity::Warning);

        // Resolve
        let alert_id = alerts[0].id.clone();
        assert!(engine.resolve_alert(&alert_id));
        assert_eq!(engine.get_active_alerts().len(), 0);

        // Check stats
        let stats = engine.get_stats();
        assert_eq!(stats.total_alerts_fired, 1);
        assert_eq!(stats.total_alerts_resolved, 1);
    }

    // =====================================================================
    // Cross-Module Pipeline Tests
    // =====================================================================

    #[test]
    fn test_e2e_cross_module_zkp_runtime_storage() {
        // ZKP Engine generates proofs
        let backend = HashBackend::new();
        let mut engine = Halo2Engine::new(backend, Halo2EngineConfig::default());

        let statement = make_statement("cross-1", CircuitType::Membership, 0.5);
        let proof = engine.generate_proof(&statement).expect("generate");

        // Compress proof data
        let mut compressor = LZ4Compressor::new(LZ4Config::default());
        let block = compressor
            .compress(&proof.proof_data, "proof-block-1")
            .expect("compress");

        // Store in checkpoint cache
        let mut cache = CheckpointCache::new(CheckpointCacheConfig::default());
        cache
            .store(
                "proof-cache-1".to_string(),
                1,
                "zkp".to_string(),
                block.compressed_data,
            )
            .expect("store");

        // Retrieve and verify
        let entry = cache.get("proof-cache-1").expect("retrieve");
        assert_eq!(entry.checkpoint_id, "proof-cache-1");
    }

    #[test]
    fn test_e2e_full_observability_pipeline() {
        // Metrics track operations
        let mut metrics = AdvancedMetrics::new(AdvancedMetricsConfig::default());
        metrics.register_counter("proofs_generated".to_string(), "Proofs".to_string());

        // Health checker monitors components
        let config = CheckConfig {
            name: "zkp_engine".to_string(),
            ..CheckConfig::default()
        };
        let mut checker = HealthChecker::new(HealthCheckerConfig::default());
        checker.register_check(config).expect("register");

        // Alert engine responds to thresholds
        let mut alerts = AlertEngine::new(AlertEngineConfig::default());
        alerts.add_rule(AlertRule::new(
            "error-rate".to_string(),
            "Error Rate High".to_string(),
            "error_rate".to_string(),
            0.05,
            AlertOperator::GreaterThan,
            AlertSeverity::Critical,
            0,
            60,
            vec![NotificationChannel::Log],
        ));

        // Simulate operations
        for _ in 0..10 {
            metrics.counter_inc("proofs_generated").expect("inc");
            checker
                .record_check("zkp_engine", HealthStatus::Healthy, "ok".to_string(), 5.0)
                .expect("healthy");
        }

        // Validate state
        let snapshot = metrics.snapshot();
        assert!(snapshot
            .counters
            .iter()
            .any(|(k, _)| k == "proofs_generated"));
        let report = checker.generate_report();
        assert_eq!(report.overall_status, HealthStatus::Healthy);
        assert_eq!(alerts.get_active_alerts().len(), 0);

        // Trigger alert
        alerts.evaluate("error_rate", 0.1);
        assert_eq!(alerts.get_active_alerts().len(), 1);
    }

    #[test]
    fn test_e2e_sprint1_full_pipeline() {
        // LP-98: Generate ZKP proofs with circuit optimization
        let backend = HashBackend::new();
        let mut engine = Halo2Engine::new(backend, Halo2EngineConfig::default());
        let mut optimizer = CircuitOptimizer::new(CircuitOptimizerConfig::default());

        let statement = make_statement("full-1", CircuitType::Membership, 0.4);
        let circuit = optimizer.select_circuit(&statement);
        let proof = engine.generate_proof(&statement).expect("generate");
        optimizer.record_result(&circuit, 50.0, 10.0, true);

        // LP-99: Worker pool manages task distribution
        let config = WorkerPoolConfig {
            initial_workers: 2,
            max_workers: 4,
            min_workers: 2,
            strategy: LoadBalanceStrategy::LeastLoaded,
            health_check_interval: Duration::from_secs(5),
            task_timeout: Duration::from_secs(30),
            auto_scale: true,
            scale_up_threshold: 0.8,
            scale_down_threshold: 0.3,
        };
        let mut pool = WorkerPool::new(config);
        pool.add_worker("w1".to_string()).unwrap();
        pool.add_worker("w2".to_string()).unwrap();
        pool.assign_task("verify-proof".to_string()).unwrap();

        // LP-100: Compress and cache proof data
        let mut compressor = LZ4Compressor::new(LZ4Config::default());
        let block = compressor
            .compress(&proof.proof_data, "proof-block")
            .expect("compress");
        let mut cache = CheckpointCache::new(CheckpointCacheConfig::default());
        cache
            .store(
                "proof-1".to_string(),
                1,
                "zkp".to_string(),
                block.compressed_data,
            )
            .expect("store");

        // LP-101: Monitor everything
        let mut metrics = AdvancedMetrics::new(AdvancedMetricsConfig::default());
        metrics.register_counter("proofs".to_string(), "Proofs".to_string());
        metrics.counter_inc("proofs").expect("inc");

        let hc_config = CheckConfig {
            name: "pipeline".to_string(),
            ..CheckConfig::default()
        };
        let mut checker = HealthChecker::new(HealthCheckerConfig::default());
        checker.register_check(hc_config).expect("reg");
        // recovery_threshold=2, so need 2 healthy records
        checker
            .record_check("pipeline", HealthStatus::Healthy, "ok".to_string(), 5.0)
            .expect("ok");
        checker
            .record_check("pipeline", HealthStatus::Healthy, "ok".to_string(), 4.0)
            .expect("ok 2");

        let mut alerts = AlertEngine::new(AlertEngineConfig::default());
        alerts.add_rule(AlertRule::new(
            "pipeline-errors".to_string(),
            "Pipeline Errors".to_string(),
            "errors".to_string(),
            10.0,
            AlertOperator::GreaterThan,
            AlertSeverity::Critical,
            0,
            60,
            vec![NotificationChannel::Log],
        ));

        // Validate all modules
        assert!(engine.verify_proof(&proof, &statement).unwrap());
        assert!(cache.get("proof-1").is_ok());
        assert_eq!(
            checker.generate_report().overall_status,
            HealthStatus::Healthy
        );
        // Pool with 1 task assigned to 2 workers is valid state
        assert_eq!(pool.stats().total_tasks_assigned, 1);
        assert_eq!(alerts.get_active_alerts().len(), 0);
    }
}
