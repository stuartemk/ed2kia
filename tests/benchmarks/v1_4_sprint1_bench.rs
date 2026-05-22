//! v1.4.0 Sprint 1 Benchmarks
//!
//! Performance benchmarks for LP-98 through LP-101 modules.
//! Run with: cargo bench --features v1.4-sprint1

#[cfg(feature = "v1.4-sprint1")]
mod bench {
    use std::time::Instant;

    // LP-98
    use ed2kia::zkp::async_zkp_v5::{CircuitType, ZKPProof, ZKPStatement};
    use ed2kia::zkp::circuit_optimizer::{CircuitOptimizer, CircuitOptimizerConfig};
    use ed2kia::zkp::halo2_engine::{Halo2Engine, Halo2EngineConfig, HashBackend};
    use ed2kia::zkp::proof_aggregator::{AggregatorConfig, ProofAggregator};

    // LP-99
    use ed2kia::runtime::task_scheduler::{
        ScheduledTask, SchedulerConfig, TaskPriority, TaskScheduler,
    };
    use ed2kia::runtime::tokio_optimizer::{TokioOptimizer, TokioOptimizerConfig};
    use ed2kia::runtime::worker_pool::{LoadBalanceStrategy, WorkerPool, WorkerPoolConfig};

    // LP-100
    use ed2kia::storage::checkpoint_cache::{
        CheckpointCache, CheckpointCacheConfig, EvictionPolicy,
    };
    use ed2kia::storage::gradient_archive::{ArchiveConfig, GradientArchive};
    use ed2kia::storage::lz4_compressor::{LZ4Compressor, LZ4Config};

    // LP-101
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
    // Helper
    // =====================================================================

    fn make_statement(id: &str, complexity: f64) -> ZKPStatement {
        ZKPStatement {
            statement_id: id.to_string(),
            public_inputs: vec![1, 2, 3],
            private_inputs_hash: format!("hash-{}", id),
            circuit_type: CircuitType::Membership,
            source_pool: "bench-pool".to_string(),
            priority: 10,
            complexity_score: complexity,
        }
    }

    fn make_proof(id: &str) -> ZKPProof {
        ZKPProof {
            proof_id: id.to_string(),
            statement_id: format!("stmt-{}", id),
            proof_data: vec![1, 2, 3, 4, 5],
            proof_hash: format!("proof-hash-{}", id),
            generation_time_ms: 50,
            used_fallback: false,
            batch_id: None,
            source_pool: "bench".to_string(),
            priority: 10,
            accumulator_index: None,
            is_vrf_sample: false,
        }
    }

    // =====================================================================
    // LP-98 Benchmarks
    // =====================================================================

    pub fn bench_halo2_proof_generation_100() {
        let backend = HashBackend::new();
        let mut engine = Halo2Engine::new(backend, Halo2EngineConfig::default());

        let start = Instant::now();
        for i in 0..100 {
            let stmt = make_statement(&format!("bench-{}", i), 0.5);
            engine.generate_proof(&stmt).ok();
        }
        let elapsed = start.elapsed();
        println!(
            "LP-98: 100 proof generations in {:?} ({:.2} ms/proof)",
            elapsed,
            elapsed.as_secs_f64() * 1000.0 / 100.0
        );
    }

    pub fn bench_halo2_verification_100() {
        let backend = HashBackend::new();
        let mut engine = Halo2Engine::new(backend, Halo2EngineConfig::default());

        let stmt = make_statement("verify-bench", 0.5);
        let proof = engine.generate_proof(&stmt).unwrap();

        let start = Instant::now();
        for _ in 0..100 {
            engine.verify_proof(&proof, &stmt).ok();
        }
        let elapsed = start.elapsed();
        println!(
            "LP-98: 100 verifications in {:?} ({:.2} ms/verify)",
            elapsed,
            elapsed.as_secs_f64() * 1000.0 / 100.0
        );
    }

    pub fn bench_circuit_optimizer_1000() {
        let mut optimizer = CircuitOptimizer::new(CircuitOptimizerConfig::default());

        let start = Instant::now();
        for i in 0..1000 {
            let stmt = make_statement(&format!("opt-{}", i), (i % 100) as f64 / 100.0);
            let circuit = optimizer.select_circuit(&stmt);
            optimizer.record_result(&circuit, 50.0, 10.0, true);
        }
        let elapsed = start.elapsed();
        println!(
            "LP-98: 1000 circuit selections in {:?} ({:.2} us/select)",
            elapsed,
            elapsed.as_secs_f64() * 1_000_000.0 / 1000.0
        );
    }

    pub fn bench_proof_aggregation_100() {
        let mut aggregator = ProofAggregator::new(AggregatorConfig::default());

        for i in 0..100 {
            let proof = make_proof(&format!("p-{}", i));
            aggregator.add_proof(proof).ok();
        }

        let start = Instant::now();
        let agg = aggregator.aggregate("bench-agg".to_string()).unwrap();
        aggregator.verify_aggregated(&agg).ok();
        let elapsed = start.elapsed();
        println!("LP-98: Aggregate + verify 100 proofs in {:?}", elapsed);
    }

    // =====================================================================
    // LP-99 Benchmarks
    // =====================================================================

    pub fn bench_tokio_optimizer_adapt_1000() {
        let config = TokioOptimizerConfig {
            min_workers: 2,
            max_workers: 16,
            adaptive_scaling: true,
            ..TokioOptimizerConfig::default()
        };
        let mut optimizer = TokioOptimizer::new(config).unwrap();
        optimizer.initialize().ok();

        let start = Instant::now();
        for _ in 0..1000 {
            optimizer.adapt();
        }
        let elapsed = start.elapsed();
        println!(
            "LP-99: 1000 adaptation cycles in {:?} ({:.2} us/cycle)",
            elapsed,
            elapsed.as_secs_f64() * 1_000_000.0 / 1000.0
        );
    }

    pub fn bench_task_scheduler_10000() {
        let mut scheduler = TaskScheduler::new(SchedulerConfig::default());

        let start = Instant::now();
        for i in 0..10000 {
            let priority = match i % 3 {
                0 => TaskPriority::High,
                1 => TaskPriority::Normal,
                _ => TaskPriority::Low,
            };
            let task = ScheduledTask::new(format!("task-{}", i), priority, "bench".to_string());
            scheduler.schedule(task).ok();
            if i % 10 == 0 {
                scheduler.next_task();
            }
        }
        let elapsed = start.elapsed();
        println!(
            "LP-99: 10000 task operations in {:?} ({:.2} us/op)",
            elapsed,
            elapsed.as_secs_f64() * 1_000_000.0 / 10000.0
        );
    }

    pub fn bench_worker_pool_10000() {
        let config = WorkerPoolConfig {
            initial_workers: 0,
            max_workers: 8,
            min_workers: 4,
            strategy: LoadBalanceStrategy::RoundRobin,
            health_check_interval: Duration::from_secs(5),
            task_timeout: Duration::from_secs(30),
            auto_scale: true,
            scale_up_threshold: 0.8,
            scale_down_threshold: 0.3,
        };
        let mut pool = WorkerPool::new(config);
        for i in 0..4 {
            pool.add_worker(format!("w{}", i)).ok();
        }

        let start = Instant::now();
        for i in 0..10000 {
            pool.assign_task(format!("task-{}", i)).ok();
            pool.complete_task(&format!("w{}", i % 4), 50.0).ok();
        }
        let elapsed = start.elapsed();
        println!(
            "LP-99: 10000 worker pool ops in {:?} ({:.2} us/op)",
            elapsed,
            elapsed.as_secs_f64() * 1_000_000.0 / 10000.0
        );
    }

    // =====================================================================
    // LP-100 Benchmarks
    // =====================================================================

    pub fn bench_lz4_compression_1mb() {
        let mut compressor = LZ4Compressor::new(LZ4Config::default());
        let data = vec![0u8; 1024 * 1024]; // 1 MB

        let start = Instant::now();
        let compressed = compressor.compress(&data, "bench-1mb").unwrap();
        let elapsed = start.elapsed();
        let ratio = data.len() as f64 / compressed.compressed_data.len() as f64;
        println!(
            "LP-100: Compress 1MB in {:?} (ratio: {:.2}x)",
            elapsed, ratio
        );

        let start = Instant::now();
        compressor.decompress(&compressed).unwrap();
        let elapsed = start.elapsed();
        println!("LP-100: Decompress 1MB in {:?}", elapsed);
    }

    pub fn bench_checkpoint_cache_10000() {
        let config = CheckpointCacheConfig {
            max_checkpoints: 1000,
            eviction_policy: EvictionPolicy::LRU,
            max_storage_bytes: 0,
            compression_enabled: false,
            stale_threshold_ms: 60000,
            eviction_batch_size: 10,
        };
        let mut cache = CheckpointCache::new(config);

        let start = Instant::now();
        for i in 0..10000 {
            cache
                .store(
                    format!("key-{}", i),
                    1,
                    "model-1".to_string(),
                    vec![1u8, 2u8, 3u8],
                )
                .ok();
            cache.get(&format!("key-{}", i % 1000)).ok();
        }
        let elapsed = start.elapsed();
        println!(
            "LP-100: 10000 cache ops in {:?} ({:.2} us/op)",
            elapsed,
            elapsed.as_secs_f64() * 1_000_000.0 / 10000.0
        );
    }

    pub fn bench_gradient_archive_1000() {
        let config = ArchiveConfig {
            max_versions_per_model: 10,
            max_total_versions: 100,
            auto_prune: true,
            ..ArchiveConfig::default()
        };
        let mut archive = GradientArchive::new(config);

        let start = Instant::now();
        for i in 0..1000 {
            let gradients = vec![1.0 * (i as f32); 100];
            archive
                .store(
                    format!("v-{}", i),
                    format!("model-{}", i % 10),
                    i as u64,
                    gradients,
                )
                .ok();
        }
        let elapsed = start.elapsed();
        println!(
            "LP-100: 1000 gradient stores in {:?} ({:.2} us/store)",
            elapsed,
            elapsed.as_secs_f64() * 1_000_000.0 / 1000.0
        );
    }

    // =====================================================================
    // LP-101 Benchmarks
    // =====================================================================

    pub fn bench_metrics_10000() {
        let mut metrics = AdvancedMetrics::new(AdvancedMetricsConfig::default());
        metrics
            .register_counter("bench_counter".to_string(), "Benchmark".to_string())
            .ok();
        metrics
            .register_gauge("bench_gauge".to_string(), "Benchmark".to_string())
            .ok();
        metrics
            .register_histogram(
                "bench_histogram".to_string(),
                "Benchmark".to_string(),
                Some(vec![10.0, 25.0, 50.0, 100.0]),
            )
            .ok();

        let start = Instant::now();
        for i in 0..10000 {
            metrics.counter_inc("bench_counter").ok();
            metrics.gauge_set("bench_gauge", i as f64).ok();
            metrics.histogram_observe("bench_histogram", i as f64).ok();
        }
        let elapsed = start.elapsed();
        println!(
            "LP-101: 10000 metric ops in {:?} ({:.2} us/op)",
            elapsed,
            elapsed.as_secs_f64() * 1_000_000.0 / 10000.0
        );
    }

    pub fn bench_health_checker_10000() {
        let config = HealthCheckerConfig {
            max_checks: 100,
            sla_tracking: true,
            ..HealthCheckerConfig::default()
        };
        let mut checker = HealthChecker::new(config);
        for i in 0..100 {
            let check = CheckConfig {
                name: format!("check-{}", i),
                recovery_threshold: 1,
                ..CheckConfig::default()
            };
            checker.register_check(check).ok();
        }

        let start = Instant::now();
        for i in 0..10000 {
            checker
                .record_check(
                    &format!("check-{}", i % 100),
                    HealthStatus::Healthy,
                    "ok".to_string(),
                    1.0,
                )
                .ok();
        }
        let elapsed = start.elapsed();
        println!(
            "LP-101: 10000 health checks in {:?} ({:.2} us/check)",
            elapsed,
            elapsed.as_secs_f64() * 1_000_000.0 / 10000.0
        );
    }

    pub fn bench_alert_engine_10000() {
        let mut engine = AlertEngine::new(AlertEngineConfig::default());
        for i in 0..10 {
            engine.add_rule(AlertRule::new(
                format!("rule-{}", i),
                format!("Rule {}", i),
                format!("metric-{}", i),
                50.0,
                AlertOperator::GreaterThan,
                AlertSeverity::Warning,
                0,
                1,
                vec![NotificationChannel::Log],
            ));
        }

        let start = Instant::now();
        for i in 0..10000 {
            engine.evaluate(&format!("metric-{}", i % 10), (i % 100) as f64);
        }
        let elapsed = start.elapsed();
        println!(
            "LP-101: 10000 alert evaluations in {:?} ({:.2} us/eval)",
            elapsed,
            elapsed.as_secs_f64() * 1_000_000.0 / 10000.0
        );
    }

    // =====================================================================
    // Runner
    // =====================================================================

    pub fn run_all() {
        println!("=== ed2kIA v1.4.0 Sprint 1 Benchmarks ===\n");

        println!("--- LP-98: Halo2 ZKP Engine ---");
        bench_halo2_proof_generation_100();
        bench_halo2_verification_100();
        bench_circuit_optimizer_1000();
        bench_proof_aggregation_100();

        println!("\n--- LP-99: Tokio Async Optimization ---");
        bench_tokio_optimizer_adapt_1000();
        bench_task_scheduler_10000();
        bench_worker_pool_10000();

        println!("\n--- LP-100: LZ4 Compression & Storage ---");
        bench_lz4_compression_1mb();
        bench_checkpoint_cache_10000();
        bench_gradient_archive_1000();

        println!("\n--- LP-101: Advanced Metrics & Observability ---");
        bench_metrics_10000();
        bench_health_checker_10000();
        bench_alert_engine_10000();

        println!("\n=== Benchmarks Complete ===");
    }
}

#[cfg(feature = "v1.4-sprint1")]
fn main() {
    bench::run_all();
}

#[cfg(not(feature = "v1.4-sprint1"))]
fn main() {
    println!("Run with: --features v1.4-sprint1");
}
