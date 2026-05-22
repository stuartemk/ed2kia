//! v1.1.0 Sprint 4 Benchmarks
//!
//! Benchmarks de rendimiento para Alignment Loop v2, Cross-Model Federation,
//! Trust Sync y streaming en tiempo real.
//!
//! Feature-gated: `--features v1.1-sprint4`
//! No harness: benchmarks manuales con timing explícito.

#[cfg(feature = "v1.1-sprint4")]
mod bench {
    use ed2kia::alignment::confidence_calculator::{ConfidenceCalculator, ConfidenceConfig};
    use ed2kia::alignment::loop_v2::{AlignmentLoopV2, FeedbackEntryV2, LoopV2Config};
    use ed2kia::alignment::steering_engine::{SteeringEngine, SteeringSignal};
    use ed2kia::federation::cross_model_scaler::{
        CrossModelScaler, ModelInfo, NodeGradientUpdate, ScalerConfig,
    };
    use ed2kia::federation::gradient_normalizer::{
        GradientBatch, GradientNormalizer, NodeCapacity, NormalizerConfig,
    };
    use ed2kia::federation::trust_sync::{TrustNodeRecord, TrustSyncConfig, TrustSyncEngine};
    use ed2kia::ui::realtime_backend::{BackendConfig, BackendEventType, RealtimeBackend};
    use ed2kia::web::sse_metrics::{
        MetricCategory, MetricPoint, SseMetricsConfig, SseMetricsStream,
    };
    use ed2kia::web::ws_alignment_stream::{
        AlignmentSignalType, WsAlignmentConfig, WsAlignmentStream,
    };
    use std::collections::HashMap;
    use std::time::Instant;

    // ============================================================================
    // Helpers
    // ============================================================================

    fn current_timestamp_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    fn make_feedback(
        layer_id: &str,
        feature_idx: u32,
        current: f32,
        desired: f32,
    ) -> FeedbackEntryV2 {
        FeedbackEntryV2 {
            entry_id: format!("fb-{}-{}-{}", layer_id, feature_idx, current_timestamp_ms()),
            layer_id: layer_id.to_string(),
            feature_idx,
            current_value: current,
            desired_value: desired,
            annotator_id: format!("annotator_{}", feature_idx % 10),
            annotator_signature: format!("sig_{:0>48}", feature_idx),
            timestamp_ms: current_timestamp_ms(),
            confidence: 0.9,
        }
    }

    fn make_steering_signal(layer_id: &str, confidence: f32) -> SteeringSignal {
        SteeringSignal {
            signal_id: format!("steer-{}-{}", layer_id, current_timestamp_ms()),
            layer_id: layer_id.to_string(),
            delta: vec![0.01, -0.02, 0.03],
            confidence,
            source_node: Some("node1".to_string()),
            timestamp_ms: current_timestamp_ms(),
        }
    }

    fn make_node_capacity(node_id: &str) -> NodeCapacity {
        NodeCapacity {
            node_id: node_id.to_string(),
            vram_gb: 80.0,
            gpu_count: 2,
            bandwidth_mbps: 10000,
        }
    }

    fn make_gradient_batch(node_id: &str, dim: usize, round: u64) -> GradientBatch {
        let data: Vec<f32> = (0..dim).map(|i| (i as f32) * 0.01).collect();
        GradientBatch::new(node_id.to_string(), data, round, dim)
    }

    fn make_trust_record(node_id: &str, network: &str) -> TrustNodeRecord {
        TrustNodeRecord::new(
            node_id.to_string(),
            format!("sig_{}", node_id),
            format!("pk_{}", node_id),
            network.to_string(),
        )
    }

    fn make_model_info(model_id: &str, dim: usize) -> ModelInfo {
        ModelInfo {
            model_id: model_id.to_string(),
            architecture: "transformer".to_string(),
            dimension: dim,
            parameter_count: dim * 1000,
        }
    }

    fn make_gradient_update(
        node_id: &str,
        model_id: &str,
        dim: usize,
        round: u64,
    ) -> NodeGradientUpdate {
        let gradients: Vec<f32> = (0..dim).map(|i| (i as f32) * 0.001).collect();
        NodeGradientUpdate {
            node_id: node_id.to_string(),
            model_id: model_id.to_string(),
            gradients,
            round,
            timestamp_ms: current_timestamp_ms(),
        }
    }

    // ============================================================================
    // Benchmark: Alignment Loop v2
    // ============================================================================

    fn bench_alignment_loop_single_cycle() {
        let mut loop_v2 = AlignmentLoopV2::new();

        // Ingestar 10 feedbacks
        for i in 0..10 {
            loop_v2
                .ingest_feedback(make_feedback("layer_1", i, 0.5, 0.8))
                .unwrap();
        }

        let activations: Vec<f32> = vec![0.5; 100];
        let start = Instant::now();
        let result = loop_v2.run_cycle("layer_1", &activations);
        let elapsed = start.elapsed();

        match result {
            Ok(r) => {
                println!(
                    "  Alignment Loop (single cycle, 10 feedback): {} ms (drift={:.4})",
                    elapsed.as_secs_f64() * 1000.0,
                    r.drift
                );
            }
            Err(e) => {
                println!("  Alignment Loop (single cycle): ERROR - {}", e);
            }
        }
    }

    fn bench_alignment_loop_100_cycles() {
        let config = LoopV2Config {
            max_feedback_per_cycle: 50,
            ..LoopV2Config::default()
        };
        let mut loop_v2 = AlignmentLoopV2::with_config(config);
        let activations: Vec<f32> = vec![0.5; 100];

        let start = Instant::now();
        let mut success_count = 0;
        for i in 0..100 {
            // Ingestar feedback para cada ciclo
            for j in 0..10 {
                loop_v2
                    .ingest_feedback(make_feedback(
                        &format!("layer_{}", i % 5),
                        j,
                        0.5 + j as f32 * 0.05,
                        0.8,
                    ))
                    .unwrap();
            }
            if loop_v2
                .run_cycle(&format!("layer_{}", i % 5), &activations)
                .is_ok()
            {
                success_count += 1;
            }
        }
        let elapsed = start.elapsed();

        println!(
            "  Alignment Loop (100 cycles): {} ms total, {} ms avg, {} success",
            elapsed.as_secs_f64() * 1000.0,
            elapsed.as_secs_f64() * 10.0,
            success_count
        );
    }

    fn bench_steering_engine_batch_application() {
        let mut engine = SteeringEngine::new();

        // Encolar 50 señales
        for i in 0..50 {
            engine
                .queue_signal(make_steering_signal(&format!("layer_{}", i % 10), 0.9))
                .unwrap();
        }

        let activations: Vec<f32> = vec![0.5; 3];
        let start = Instant::now();
        let results = engine.apply_all(&activations);
        let elapsed = start.elapsed();

        match results {
            Ok(r) => {
                println!(
                    "  Steering Engine (50 signals batch): {} ms, {} applied",
                    elapsed.as_secs_f64() * 1000.0,
                    r.len()
                );
            }
            Err(e) => {
                println!("  Steering Engine (batch): ERROR - {}", e);
            }
        }
    }

    // ============================================================================
    // Benchmark: Confidence Calculator
    // ============================================================================

    fn bench_confidence_calculator_100_annotators() {
        let config = ConfidenceConfig {
            alpha: 0.3,
            decay_rate: 0.01,
            ..ConfidenceConfig::default()
        };
        let mut calc = ConfidenceCalculator::with_config(config);

        // Registrar 100 anotadores
        for i in 0..100 {
            calc.register_annotator(format!("annotator_{}", i), 0.5 + (i % 10) as f32 * 0.05);
        }

        let start = Instant::now();
        let weights = calc.compute_weighted_confidence();
        let elapsed = start.elapsed();

        println!(
            "  Confidence Calculator (100 annotators): {} ms, {} weights computed",
            elapsed.as_secs_f64() * 1000.0,
            weights.len()
        );
    }

    fn bench_confidence_calculator_decay_stress() {
        let mut calc = ConfidenceCalculator::new();

        for i in 0..50 {
            calc.register_annotator(format!("a_{}", i), 0.7);
        }

        let start = Instant::now();
        for _ in 0..100 {
            calc.apply_decay_to_all(1.0);
        }
        let elapsed = start.elapsed();

        println!(
            "  Confidence Calculator (100 decay iterations, 50 annotators): {} ms",
            elapsed.as_secs_f64() * 1000.0
        );
    }

    // ============================================================================
    // Benchmark: Gradient Normalizer
    // ============================================================================

    fn bench_gradient_normalizer_single() {
        let config = NormalizerConfig {
            target_dim: 64,
            ..NormalizerConfig::default()
        };
        let mut normalizer = GradientNormalizer::with_config(config);
        normalizer
            .register_node(make_node_capacity("node1"))
            .unwrap();

        let batch = make_gradient_batch("node1", 1024, 1);
        let start = Instant::now();
        let result = normalizer.normalize(batch);
        let elapsed = start.elapsed();

        match result {
            Ok(data) => {
                println!(
                    "  Gradient Normalizer (1024→64 dim): {} ms, output_dim={}",
                    elapsed.as_secs_f64() * 1000.0,
                    data.len()
                );
            }
            Err(e) => {
                println!("  Gradient Normalizer: ERROR - {}", e);
            }
        }
    }

    fn bench_gradient_normalizer_batch_100() {
        let config = NormalizerConfig {
            target_dim: 128,
            ..NormalizerConfig::default()
        };
        let mut normalizer = GradientNormalizer::with_config(config);

        for i in 0..10 {
            normalizer
                .register_node(make_node_capacity(&format!("node_{}", i)))
                .unwrap();
        }

        let batches: Vec<GradientBatch> = (0..100)
            .map(|i| make_gradient_batch(&format!("node_{}", i % 10), 512, i as u64))
            .collect();

        let start = Instant::now();
        let results = normalizer.normalize_batch(batches);
        let elapsed = start.elapsed();

        match results {
            Ok(data) => {
                println!(
                    "  Gradient Normalizer (batch 100 x 512→128): {} ms, {} normalized",
                    elapsed.as_secs_f64() * 1000.0,
                    data.len()
                );
            }
            Err(e) => {
                println!("  Gradient Normalizer (batch): ERROR - {}", e);
            }
        }
    }

    // ============================================================================
    // Benchmark: Trust Sync
    // ============================================================================

    fn bench_trust_sync_100_nodes() {
        let mut engine = TrustSyncEngine::new();

        for i in 0..100 {
            engine
                .register_node(make_trust_record(&format!("node_{}", i), "net_a"))
                .unwrap();
        }

        let start = Instant::now();
        let results = engine.sync_cycle();
        let elapsed = start.elapsed();

        println!(
            "  Trust Sync (100 nodes, 1 cycle): {} ms, {} results",
            elapsed.as_secs_f64() * 1000.0,
            results.len()
        );
    }

    fn bench_trust_sync_10_cycles() {
        let mut engine = TrustSyncEngine::new();

        for i in 0..50 {
            engine
                .register_node(make_trust_record(&format!("node_{}", i), "net_a"))
                .unwrap();
        }

        let start = Instant::now();
        for _ in 0..10 {
            engine.sync_cycle();
        }
        let elapsed = start.elapsed();

        println!(
            "  Trust Sync (50 nodes, 10 cycles): {} ms, {} ms avg",
            elapsed.as_secs_f64() * 1000.0,
            elapsed.as_secs_f64() * 100.0
        );
    }

    // ============================================================================
    // Benchmark: Cross-Model Scaler
    // ============================================================================

    fn bench_cross_model_scaler_heterogeneous() {
        let mut scaler = CrossModelScaler::new();

        scaler.register_model(make_model_info("model_a", 512));
        scaler.register_model(make_model_info("model_b", 256));
        scaler.register_model(make_model_info("model_c", 128));

        for i in 0..20 {
            let model = match i % 3 {
                0 => "model_a",
                1 => "model_b",
                _ => "model_c",
            };
            scaler
                .register_node(format!("node_{}", i), model.to_string(), 0.85)
                .unwrap();
        }

        let start = Instant::now();
        for i in 0..20 {
            let model = match i % 3 {
                0 => ("model_a", 512),
                1 => ("model_b", 256),
                _ => ("model_c", 128),
            };
            scaler
                .receive_update(make_gradient_update(
                    &format!("node_{}", i),
                    model.0,
                    model.1,
                    1,
                ))
                .unwrap();
        }
        let sync_result = scaler.sync();
        let elapsed = start.elapsed();

        match sync_result {
            Ok(r) => {
                println!(
                    "  Cross-Model Scaler (20 nodes, 3 models): {} ms, gradient_dim={}",
                    elapsed.as_secs_f64() * 1000.0,
                    r.aggregated_gradient.len()
                );
            }
            Err(e) => {
                println!("  Cross-Model Scaler: ERROR - {}", e);
            }
        }
    }

    fn bench_cross_model_scaler_dimension_scaling() {
        let mut scaler = CrossModelScaler::new();
        scaler.register_model(make_model_info("model_a", 1024));

        let gradients: Vec<f32> = (0..1024).map(|i| i as f32 * 0.01).collect();

        let start = Instant::now();
        for _ in 0..100 {
            let _scaled = scaler.scale_gradients(&gradients, 1024, 64);
        }
        let elapsed = start.elapsed();

        println!(
            "  Cross-Model Scaler (100 x 1024→64 scale): {} ms, {} ms avg",
            elapsed.as_secs_f64() * 1000.0,
            elapsed.as_secs_f64() * 10.0
        );
    }

    // ============================================================================
    // Benchmark: Realtime Backend
    // ============================================================================

    fn bench_realtime_backend_publish_1000() {
        let mut backend = RealtimeBackend::new();

        for i in 0..20 {
            backend
                .create_session(
                    format!("session_{}", i),
                    vec![BackendEventType::AlignmentFeedback],
                )
                .unwrap();
        }

        let start = Instant::now();
        for i in 0..1000 {
            backend.publish_event(
                BackendEventType::AlignmentFeedback,
                serde_json::json!({"idx": i, "value": i as f64 * 0.1}),
                Some(format!("node_{}", i % 5)),
            );
        }
        let elapsed = start.elapsed();

        let stats = backend.get_stats();
        println!(
            "  Realtime Backend (1000 events, 20 sessions): {} ms, {} sent, {} ms avg",
            elapsed.as_secs_f64() * 1000.0,
            stats.total_events_sent,
            elapsed.as_secs_f64() * 100.0
        );
    }

    fn bench_realtime_backend_multi_subscription() {
        let mut backend = RealtimeBackend::new();

        // 10 sesiones por tipo de evento
        for i in 0..10 {
            backend
                .create_session(
                    format!("align_{}", i),
                    vec![BackendEventType::AlignmentFeedback],
                )
                .unwrap();
            backend
                .create_session(
                    format!("fed_{}", i),
                    vec![BackendEventType::FederationGradientSync],
                )
                .unwrap();
        }

        let start = Instant::now();
        for i in 0..500 {
            backend.publish_event(
                if i % 2 == 0 {
                    BackendEventType::AlignmentFeedback
                } else {
                    BackendEventType::FederationGradientSync
                },
                serde_json::json!({"idx": i}),
                None,
            );
        }
        let elapsed = start.elapsed();

        println!(
            "  Realtime Backend (500 events, 20 sessions multi-sub): {} ms",
            elapsed.as_secs_f64() * 1000.0
        );
    }

    // ============================================================================
    // Benchmark: WS Alignment Stream
    // ============================================================================

    fn bench_ws_alignment_stream_emit_1000() {
        let mut stream = WsAlignmentStream::new();

        for i in 0..10 {
            stream
                .create_connection(
                    format!("conn_{}", i),
                    vec![
                        AlignmentSignalType::Feedback,
                        AlignmentSignalType::Steering,
                        AlignmentSignalType::Drift,
                    ],
                )
                .unwrap();
        }

        let start = Instant::now();
        for i in 0..1000 {
            stream.emit_signal(
                match i % 3 {
                    0 => AlignmentSignalType::Feedback,
                    1 => AlignmentSignalType::Steering,
                    _ => AlignmentSignalType::Drift,
                },
                format!("layer_{}", i % 5),
                serde_json::json!({"idx": i}),
                Some(format!("node_{}", i % 3)),
                0.9,
            );
        }
        let elapsed = start.elapsed();

        let stats = stream.get_stats();
        println!(
            "  WS Alignment Stream (1000 signals, 10 conns): {} ms, {} sent",
            elapsed.as_secs_f64() * 1000.0,
            stats.total_signals_sent
        );
    }

    // ============================================================================
    // Benchmark: SSE Metrics Stream
    // ============================================================================

    fn bench_sse_metrics_publish_1000() {
        let mut stream = SseMetricsStream::new();

        for i in 0..15 {
            stream
                .create_session(
                    format!("session_{}", i),
                    vec![MetricCategory::Alignment, MetricCategory::Performance],
                )
                .unwrap();
        }

        let labels = HashMap::new();
        let start = Instant::now();
        for i in 0..1000 {
            stream.publish_metrics(
                match i % 2 {
                    0 => MetricCategory::Alignment,
                    _ => MetricCategory::Performance,
                },
                vec![MetricPoint {
                    metric_name: "value".to_string(),
                    value: i as f64 * 0.1,
                    unit: "score".to_string(),
                    labels: labels.clone(),
                }],
                Some(format!("node_{}", i % 5)),
            );
        }
        let elapsed = start.elapsed();

        let stats = stream.get_stats();
        println!(
            "  SSE Metrics (1000 events, 15 sessions): {} ms, {} sent",
            elapsed.as_secs_f64() * 1000.0,
            stats.total_events_sent
        );
    }

    // ============================================================================
    // Comparison Benchmarks
    // ============================================================================

    fn bench_comparison_alignment_pipeline() {
        let mut loop_v2 = AlignmentLoopV2::new();
        let mut engine = SteeringEngine::new();
        let mut calc = ConfidenceCalculator::new();

        // Setup
        for i in 0..10 {
            calc.register_annotator(format!("a_{}", i), 0.7);
            loop_v2
                .ingest_feedback(make_feedback("layer_1", i, 0.5, 0.8))
                .unwrap();
        }

        let start = Instant::now();

        // Alignment cycle
        let activations: Vec<f32> = vec![0.5; 100];
        let result = loop_v2.run_cycle("layer_1", &activations).unwrap();

        // Steering
        let signal = make_steering_signal("layer_1", result.confidence);
        engine.queue_signal(signal).unwrap();
        engine.apply_next("layer_1", &activations[..3]).unwrap();

        // Confidence
        let _weights = calc.compute_weighted_confidence();

        let elapsed = start.elapsed();
        println!(
            "  Full Alignment Pipeline (feedback→cycle→steering→confidence): {} ms",
            elapsed.as_secs_f64() * 1000.0
        );
    }

    fn bench_comparison_federation_pipeline() {
        let mut normalizer = GradientNormalizer::new();
        let mut trust_engine = TrustSyncEngine::new();
        let mut scaler = CrossModelScaler::new();

        // Setup
        for i in 0..10 {
            normalizer
                .register_node(make_node_capacity(&format!("node_{}", i)))
                .unwrap();
            trust_engine
                .register_node(make_trust_record(&format!("node_{}", i), "net_a"))
                .unwrap();
        }
        scaler.register_model(make_model_info("model_a", 128));
        for i in 0..10 {
            scaler
                .register_node(format!("node_{}", i), "model_a".to_string(), 0.85)
                .unwrap();
        }

        let start = Instant::now();

        // Normalize
        for i in 0..10 {
            normalizer
                .normalize(make_gradient_batch(&format!("node_{}", i), 256, 1))
                .unwrap();
        }

        // Trust sync
        trust_engine.sync_cycle();

        // Scaler sync
        for i in 0..10 {
            scaler
                .receive_update(make_gradient_update(
                    &format!("node_{}", i),
                    "model_a",
                    128,
                    1,
                ))
                .unwrap();
        }
        scaler.sync().unwrap();

        let elapsed = start.elapsed();
        println!(
            "  Full Federation Pipeline (normalize→trust→scale): {} ms",
            elapsed.as_secs_f64() * 1000.0
        );
    }

    fn bench_comparison_full_sprint4_pipeline() {
        // Pipeline completo: Alignment → Federation → Streaming
        let mut loop_v2 = AlignmentLoopV2::new();
        let mut scaler = CrossModelScaler::new();
        let mut backend = RealtimeBackend::new();
        let mut ws_stream = WsAlignmentStream::new();
        let mut sse_stream = SseMetricsStream::new();

        // Setup
        backend
            .create_session(
                "dashboard".to_string(),
                vec![
                    BackendEventType::AlignmentCycleComplete,
                    BackendEventType::FederationGradientSync,
                ],
            )
            .unwrap();

        ws_stream
            .create_connection(
                "ui_client".to_string(),
                vec![AlignmentSignalType::Feedback, AlignmentSignalType::Drift],
            )
            .unwrap();

        sse_stream
            .create_session(
                "metrics_client".to_string(),
                vec![MetricCategory::Alignment, MetricCategory::Federation],
            )
            .unwrap();

        scaler.register_model(make_model_info("main_model", 128));
        for i in 0..5 {
            scaler
                .register_node(format!("node_{}", i), "main_model".to_string(), 0.9)
                .unwrap();
        }

        let start = Instant::now();

        // Alignment
        for i in 0..20 {
            loop_v2
                .ingest_feedback(make_feedback("layer_1", i, 0.5, 0.8))
                .unwrap();
        }
        let activations: Vec<f32> = vec![0.5; 50];
        let alignment_result = loop_v2.run_cycle("layer_1", &activations).unwrap();

        // Federation
        for i in 0..5 {
            scaler
                .receive_update(make_gradient_update(
                    &format!("node_{}", i),
                    "main_model",
                    128,
                    1,
                ))
                .unwrap();
        }
        let sync_result = scaler.sync().unwrap();

        // Streaming
        backend.publish_event(
            BackendEventType::AlignmentCycleComplete,
            serde_json::json!({"drift": alignment_result.drift}),
            None,
        );
        backend.publish_event(
            BackendEventType::FederationGradientSync,
            serde_json::json!({"dim": sync_result.aggregated_gradient.len()}),
            None,
        );

        ws_stream.emit_signal(
            AlignmentSignalType::Drift,
            "layer_1".to_string(),
            serde_json::json!({"drift": alignment_result.drift}),
            None,
            alignment_result.confidence,
        );

        sse_stream.publish_metrics(
            MetricCategory::Alignment,
            vec![MetricPoint {
                metric_name: "drift".to_string(),
                value: alignment_result.drift as f64,
                unit: "score".to_string(),
                labels: HashMap::new(),
            }],
            None,
        );

        let elapsed = start.elapsed();
        println!(
            "  Full Sprint 4 Pipeline (alignment→federation→streaming): {} ms",
            elapsed.as_secs_f64() * 1000.0
        );
    }

    // ============================================================================
    // Main
    // ============================================================================

    fn main() {
        println!("=== ed2kIA v1.1.0 Sprint 4 Benchmarks ===\n");

        println!("--- Alignment Loop v2 ---");
        bench_alignment_loop_single_cycle();
        bench_alignment_loop_100_cycles();
        bench_steering_engine_batch_application();

        println!("\n--- Confidence Calculator ---");
        bench_confidence_calculator_100_annotators();
        bench_confidence_calculator_decay_stress();

        println!("\n--- Gradient Normalizer ---");
        bench_gradient_normalizer_single();
        bench_gradient_normalizer_batch_100();

        println!("\n--- Trust Sync ---");
        bench_trust_sync_100_nodes();
        bench_trust_sync_10_cycles();

        println!("\n--- Cross-Model Scaler ---");
        bench_cross_model_scaler_heterogeneous();
        bench_cross_model_scaler_dimension_scaling();

        println!("\n--- Realtime Backend ---");
        bench_realtime_backend_publish_1000();
        bench_realtime_backend_multi_subscription();

        println!("\n--- WS Alignment Stream ---");
        bench_ws_alignment_stream_emit_1000();

        println!("\n--- SSE Metrics Stream ---");
        bench_sse_metrics_publish_1000();

        println!("\n--- Comparison Pipelines ---");
        bench_comparison_alignment_pipeline();
        bench_comparison_federation_pipeline();
        bench_comparison_full_sprint4_pipeline();

        println!("\n=== Benchmarks completados ===");
    }
} // end mod bench

// Fallback main when v1.1-sprint4 feature is not enabled
#[cfg(not(feature = "v1.1-sprint4"))]
fn main() {
    println!("sprint4_bench: v1.1-sprint4 feature not enabled, skipping benchmarks");
}
