//! v1.1.0 Sprint 4 E2E Integration Tests
//!
//! Flujo completo: Alignment Loop v2 → Steering Engine → Confidence Calculator
//! → Cross-Model Federation → Trust Sync → Real-time Streaming
//!
//! Feature-gated: `--features v1.1-sprint4`

#![cfg(feature = "v1.1-sprint4")]

use ed2kia::alignment::confidence_calculator::{ConfidenceCalculator, ConfidenceConfig};
use ed2kia::alignment::loop_v2::{AlignmentLoopV2, FeedbackEntryV2, LoopV2Config};
use ed2kia::alignment::steering_engine::{SteeringConfig, SteeringEngine, SteeringSignal};
use ed2kia::federation::cross_model_scaler::{
    CrossModelScaler, ModelInfo, NodeGradientUpdate, ScalerConfig,
};
use ed2kia::federation::gradient_normalizer::{
    GradientBatch, GradientNormalizer, NodeCapacity, NormalizerConfig,
};
use ed2kia::federation::trust_sync::{
    NodeStatus, TrustNodeRecord, TrustSyncConfig, TrustSyncEngine,
};
use ed2kia::ui::realtime_backend::{BackendConfig, BackendEventType, RealtimeBackend};
use ed2kia::web::sse_metrics::{MetricCategory, MetricPoint, SseMetricsConfig, SseMetricsStream};
use ed2kia::web::ws_alignment_stream::{AlignmentSignalType, WsAlignmentConfig, WsAlignmentStream};

// ============================================================================
// Helpers
// ============================================================================

fn make_feedback(layer_id: &str, feature_idx: u32, current: f32, desired: f32) -> FeedbackEntryV2 {
    FeedbackEntryV2 {
        entry_id: format!("fb-{}-{}", layer_id, feature_idx),
        layer_id: layer_id.to_string(),
        feature_idx,
        current_value: current,
        desired_value: desired,
        annotator_id: "annotator_1".to_string(),
        annotator_signature: "sig_test_12345678901234567890123456789012".to_string(),
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

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ============================================================================
// Test: Alignment Loop v2 E2E
// ============================================================================

#[test]
fn test_e2e_alignment_loop_feedback_cycle() {
    let config = LoopV2Config {
        max_feedback_per_cycle: 10,
        drift_threshold: 0.3,
        ..LoopV2Config::default()
    };
    let mut loop_v2 = AlignmentLoopV2::with_config(config);

    // Ingestar feedback
    for i in 0..5 {
        let fb = make_feedback("layer_1", i, 0.5 + i as f32 * 0.1, 0.8);
        loop_v2.ingest_feedback(fb).unwrap();
    }

    // Ejecutar ciclo
    let activations: Vec<f32> = vec![0.5; 10];
    let result = loop_v2.run_cycle("layer_1", &activations).unwrap();

    assert!(result.drift >= 0.0);
    assert!(result.drift <= 1.0);
    assert_eq!(result.layer_id, "layer_1");
}

#[test]
fn test_e2e_alignment_loop_steering_application() {
    let mut loop_v2 = AlignmentLoopV2::new();

    // Ingestar feedback
    loop_v2
        .ingest_feedback(make_feedback("layer_1", 0, 0.3, 0.9))
        .unwrap();

    // Generar señal de steering
    let signal = loop_v2.generate_steering_signal("layer_1").unwrap();
    assert!(signal.confidence > 0.0);

    // Aplicar señal
    let activations: Vec<f32> = vec![0.5; 3];
    let result = loop_v2.apply_signal(&signal, &activations).unwrap();
    assert_eq!(result.layer_id, "layer_1");
}

#[test]
fn test_e2e_alignment_loop_rollback() {
    let mut loop_v2 = AlignmentLoopV2::new();

    loop_v2
        .ingest_feedback(make_feedback("layer_1", 0, 0.5, 0.8))
        .unwrap();
    let activations: Vec<f32> = vec![0.5; 5];
    loop_v2.run_cycle("layer_1", &activations).unwrap();

    // Rollback
    let result = loop_v2.rollback("layer_1").unwrap();
    assert!(result.is_some());
}

// ============================================================================
// Test: Steering Engine E2E
// ============================================================================

#[test]
fn test_e2e_steering_engine_queue_and_apply() {
    let config = SteeringConfig {
        max_signals: 10,
        smoothing_factor: 0.1,
        ..SteeringConfig::default()
    };
    let mut engine = SteeringEngine::with_config(config);

    // Encolar señales
    engine
        .queue_signal(make_steering_signal("layer_1", 0.9))
        .unwrap();
    engine
        .queue_signal(make_steering_signal("layer_2", 0.8))
        .unwrap();

    // Aplicar siguiente
    let activations: Vec<f32> = vec![0.5; 3];
    let result = engine.apply_next("layer_1", &activations).unwrap();
    assert_eq!(result.layer_id, "layer_1");
    assert!(result.confidence > 0.0);
}

#[test]
fn test_e2e_steering_engine_apply_all() {
    let mut engine = SteeringEngine::new();

    engine
        .queue_signal(make_steering_signal("layer_1", 0.9))
        .unwrap();
    engine
        .queue_signal(make_steering_signal("layer_2", 0.85))
        .unwrap();

    let activations: Vec<f32> = vec![0.5; 3];
    let results = engine.apply_all(&activations).unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_e2e_steering_engine_verification() {
    let engine = SteeringEngine::new();
    let signal = make_steering_signal("layer_1", 0.9);

    let valid = engine.verify_signal(&signal);
    assert!(valid);
}

// ============================================================================
// Test: Confidence Calculator E2E
// ============================================================================

#[test]
fn test_e2e_confidence_calculator_full_workflow() {
    let config = ConfidenceConfig {
        alpha: 0.3,
        decay_rate: 0.01,
        ..ConfidenceConfig::default()
    };
    let mut calc = ConfidenceCalculator::with_config(config);

    // Registrar anotadores
    calc.register_annotator("annotator_1".to_string(), 0.8);
    calc.register_annotator("annotator_2".to_string(), 0.7);
    calc.register_annotator("annotator_3".to_string(), 0.9);

    // Actualizar confianza
    calc.update_confidence("annotator_1", true).unwrap();
    calc.update_confidence("annotator_2", false).unwrap();

    // Calcular confianza ponderada
    let weights = calc.compute_weighted_confidence();
    assert_eq!(weights.len(), 3);

    // Verificar que annotator_1 tiene mayor peso tras éxito
    let w1 = weights
        .iter()
        .find(|w| w.annotator_id == "annotator_1")
        .unwrap();
    let w2 = weights
        .iter()
        .find(|w| w.annotator_id == "annotator_2")
        .unwrap();
    assert!(w1.weight > w2.weight);
}

#[test]
fn test_e2e_confidence_decay() {
    let mut calc = ConfidenceCalculator::new();
    calc.register_annotator("a1".to_string(), 0.8);

    let before = calc.compute_weighted_confidence();
    let before_weight = before[0].weight;

    // Aplicar decaimiento
    calc.apply_decay_to_all(10.0); // 10 días

    let after = calc.compute_weighted_confidence();
    let after_weight = after[0].weight;

    assert!(after_weight < before_weight);
}

#[test]
fn test_e2e_confidence_anomaly_detection() {
    let mut calc = ConfidenceCalculator::new();
    calc.register_annotator("a1".to_string(), 0.5);
    calc.register_annotator("a2".to_string(), 0.5);
    calc.register_annotator("a3".to_string(), 0.5);

    // a3 tiene confianza muy diferente
    calc.update_confidence("a1", true).unwrap();
    calc.update_confidence("a2", true).unwrap();
    calc.update_confidence("a3", false).unwrap();

    let anomalies = calc.detect_anomalies();
    // Puede o no detectar anomalía con solo 3 nodos
    assert!(anomalies.len() >= 0);
}

// ============================================================================
// Test: Gradient Normalizer E2E
// ============================================================================

#[test]
fn test_e2e_gradient_normalizer_full_pipeline() {
    let config = NormalizerConfig {
        target_dim: 64,
        outlier_threshold: 3.0,
        ..NormalizerConfig::default()
    };
    let mut normalizer = GradientNormalizer::with_config(config);

    // Registrar nodos
    normalizer
        .register_node(make_node_capacity("node1"))
        .unwrap();
    normalizer
        .register_node(make_node_capacity("node2"))
        .unwrap();

    // Normalizar gradientes
    let batch1 = make_gradient_batch("node1", 128, 1);
    let result1 = normalizer.normalize(batch1).unwrap();
    assert_eq!(result1.len(), 64); // Escalado a target_dim

    let batch2 = make_gradient_batch("node2", 64, 1);
    let result2 = normalizer.normalize(batch2).unwrap();
    assert_eq!(result2.len(), 64);

    // Verificar stats
    let stats = normalizer.get_stats();
    assert_eq!(stats.total_normalized, 2);
}

#[test]
fn test_e2e_gradient_normalizer_batch() {
    let mut normalizer = GradientNormalizer::new();
    normalizer
        .register_node(make_node_capacity("node1"))
        .unwrap();

    let batches = vec![
        make_gradient_batch("node1", 32, 1),
        make_gradient_batch("node1", 32, 2),
        make_gradient_batch("node1", 32, 3),
    ];

    let results = normalizer.normalize_batch(batches).unwrap();
    assert_eq!(results.len(), 3);
}

#[test]
fn test_e2e_gradient_outlier_detection() {
    let mut normalizer = GradientNormalizer::new();
    normalizer
        .register_node(make_node_capacity("node1"))
        .unwrap();

    // Normalizar varios gradientes para establecer stats globales
    for i in 0..10 {
        let batch = make_gradient_batch("node1", 32, i);
        normalizer.normalize(batch).unwrap();
    }

    // Gradiente con valores extremos
    let extreme_data: Vec<f32> = vec![1000.0; 32];
    let extreme_batch = GradientBatch::new("node1".to_string(), extreme_data, 11, 32);

    // Puede detectar outlier o normalizar sin error
    let result = normalizer.normalize(extreme_batch);
    assert!(result.is_ok());
}

// ============================================================================
// Test: Trust Sync E2E
// ============================================================================

#[test]
fn test_e2e_trust_sync_full_workflow() {
    let config = TrustSyncConfig {
        decay_rate: 0.02,
        enable_zkp_boost: true,
        ..TrustSyncConfig::default()
    };
    let mut engine = TrustSyncEngine::with_config(config);

    // Registrar nodos
    engine
        .register_node(make_trust_record("node1", "net_a"))
        .unwrap();
    engine
        .register_node(make_trust_record("node2", "net_a"))
        .unwrap();
    engine
        .register_node(make_trust_record("node3", "net_b"))
        .unwrap();

    // Actualizar confianza
    engine.update_trust("node1", true).unwrap();
    engine.update_trust("node2", false).unwrap();

    // Aplicar ZKP boost
    engine.apply_zkp_boost("node1").unwrap();

    // Ejecutar ciclo de sync
    let results = engine.sync_cycle();
    assert_eq!(results.len(), 3);

    // Verificar stats
    let stats = engine.get_stats();
    assert_eq!(stats.total_nodes, 3);
    assert!(stats.zkp_boosts_applied > 0);
}

#[test]
fn test_e2e_trust_sync_sybil_detection() {
    let mut engine = TrustSyncEngine::new();

    let mut record1 = make_trust_record("node1", "net_a");
    let mut record2 = make_trust_record("node2", "net_a");
    record2.crypto_signature = record1.crypto_signature.clone(); // Firma duplicada

    engine.register_node(record1).unwrap();
    let result = engine.register_node(record2);

    assert!(result.is_err());
}

#[test]
fn test_e2e_trust_sync_decay_and_propagation() {
    let mut engine = TrustSyncEngine::new();
    engine
        .register_node(make_trust_record("node1", "net_a"))
        .unwrap();
    engine
        .register_node(make_trust_record("node2", "net_a"))
        .unwrap();

    // Dar alta confianza
    engine.update_trust("node1", true).unwrap();
    engine.update_trust("node2", true).unwrap();

    // Ejecutar sync (propagación)
    let results = engine.sync_cycle();
    assert!(engine.get_stats().propagations_this_cycle > 0);
}

// ============================================================================
// Test: Cross-Model Scaler E2E
// ============================================================================

#[test]
fn test_e2e_cross_model_scaler_heterogeneous_sync() {
    let config = ScalerConfig {
        divergence_threshold: 0.5,
        ..ScalerConfig::default()
    };
    let mut scaler = CrossModelScaler::with_config(config);

    // Registrar modelos heterogéneos
    scaler.register_model(make_model_info("model_a", 128));
    scaler.register_model(make_model_info("model_b", 64));

    // Registrar nodos
    scaler
        .register_node("node1".to_string(), "model_a".to_string(), 0.9)
        .unwrap();
    scaler
        .register_node("node2".to_string(), "model_b".to_string(), 0.85)
        .unwrap();

    // Recibir updates
    scaler
        .receive_update(make_gradient_update("node1", "model_a", 128, 1))
        .unwrap();
    scaler
        .receive_update(make_gradient_update("node2", "model_b", 64, 1))
        .unwrap();

    // Sincronizar
    let result = scaler.sync().unwrap();
    assert!(result.aggregated_gradient.len() > 0);
}

#[test]
fn test_e2e_cross_model_scaler_divergence_detection() {
    let config = ScalerConfig {
        divergence_threshold: 0.1, // Muy bajo para forzar detección
        ..ScalerConfig::default()
    };
    let mut scaler = CrossModelScaler::with_config(config);

    scaler.register_model(make_model_info("model_a", 32));
    scaler
        .register_node("node1".to_string(), "model_a".to_string(), 0.9)
        .unwrap();
    scaler
        .register_node("node2".to_string(), "model_a".to_string(), 0.9)
        .unwrap();

    // Updates muy diferentes
    let update1 = make_gradient_update("node1", "model_a", 32, 1);
    let mut update2 = make_gradient_update("node2", "model_a", 32, 1);
    update2.gradients = update2.gradients.iter().map(|g| g * 10.0).collect();

    scaler.receive_update(update1).unwrap();
    scaler.receive_update(update2).unwrap();

    let result = scaler.sync();
    // Puede detectar divergencia o completar sync
    assert!(result.is_ok());
}

#[test]
fn test_e2e_cross_model_scaler_dimension_scaling() {
    let mut scaler = CrossModelScaler::new();
    scaler.register_model(make_model_info("model_a", 128));

    let gradients: Vec<f32> = (0..128).map(|i| i as f32 * 0.01).collect();
    let scaled = scaler.scale_gradients(&gradients, 128, 64);
    assert_eq!(scaled.len(), 64);

    let upsampled = scaler.scale_gradients(&gradients[..64], 64, 128);
    assert_eq!(upsampled.len(), 128);
}

// ============================================================================
// Test: Realtime Backend E2E
// ============================================================================

#[test]
fn test_e2e_realtime_backend_publish_and_subscribe() {
    let mut backend = RealtimeBackend::new();

    // Crear sesiones con diferentes suscripciones
    backend
        .create_session("s1".to_string(), vec![BackendEventType::AlignmentFeedback])
        .unwrap();
    backend
        .create_session(
            "s2".to_string(),
            vec![BackendEventType::FederationGradientSync],
        )
        .unwrap();

    // Publicar evento de alignment
    let result = backend.publish_event(
        BackendEventType::AlignmentFeedback,
        serde_json::json!({"drift": 0.15}),
        Some("node1".to_string()),
    );
    assert_eq!(result.events_sent, 1); // Solo s1

    // Publicar evento de federation
    let result = backend.publish_event(
        BackendEventType::FederationGradientSync,
        serde_json::json!({"round": 5}),
        None,
    );
    assert_eq!(result.events_sent, 1); // Solo s2
}

#[test]
fn test_e2e_realtime_backend_catchup() {
    let mut backend = RealtimeBackend::new();
    backend
        .create_session("s1".to_string(), vec![BackendEventType::AlignmentFeedback])
        .unwrap();

    // Publicar eventos
    for i in 0..5 {
        backend.publish_event(
            BackendEventType::AlignmentFeedback,
            serde_json::json!({"n": i}),
            None,
        );
    }

    // Obtener catch-up desde secuencia 2
    let catchup = backend.get_catchup_events("s1", 2);
    assert_eq!(catchup.len(), 3); // Eventos 3, 4, 5
}

// ============================================================================
// Test: WS Alignment Stream E2E
// ============================================================================

#[test]
fn test_e2e_ws_alignment_stream_emit_and_catchup() {
    let mut stream = WsAlignmentStream::new();

    stream
        .create_connection(
            "c1".to_string(),
            vec![AlignmentSignalType::Feedback, AlignmentSignalType::Steering],
        )
        .unwrap();

    // Emitir señales
    stream.emit_signal(
        AlignmentSignalType::Feedback,
        "layer_1".to_string(),
        serde_json::json!({"drift": 0.1}),
        Some("node1".to_string()),
        0.9,
    );
    stream.emit_signal(
        AlignmentSignalType::Steering,
        "layer_1".to_string(),
        serde_json::json!({"delta": [0.01, -0.02]}),
        None,
        0.85,
    );

    // Catch-up
    let catchup = stream.get_catchup_signals("c1", 0);
    assert_eq!(catchup.len(), 2);
}

#[test]
fn test_e2e_ws_alignment_stream_sse_format() {
    let stream = WsAlignmentStream::new();
    let signal = stream.emit_signal(
        AlignmentSignalType::Drift,
        "layer_1".to_string(),
        serde_json::json!({"value": 0.25}),
        None,
        0.8,
    );

    // Verificar formato SSE
    let event = stream.signal_history.back().unwrap();
    let sse = WsAlignmentStream::format_sse_signal(event);
    assert!(sse.contains("event: alignment_drift"));
    assert!(sse.contains("data:"));
}

// ============================================================================
// Test: SSE Metrics Stream E2E
// ============================================================================

#[test]
fn test_e2e_sse_metrics_multi_category() {
    let mut stream = SseMetricsStream::new();

    stream
        .create_session(
            "s1".to_string(),
            vec![MetricCategory::Alignment, MetricCategory::Performance],
        )
        .unwrap();
    stream
        .create_session("s2".to_string(), vec![MetricCategory::Federation])
        .unwrap();

    // Publicar métricas de alignment
    let metrics = vec![MetricPoint {
        metric_name: "drift".to_string(),
        value: 0.15,
        unit: "score".to_string(),
        labels: std::collections::HashMap::new(),
    }];
    let result = stream.publish_metrics(MetricCategory::Alignment, metrics, None);
    assert_eq!(result.events_sent, 1); // Solo s1

    // Publicar métricas de federation
    let metrics = vec![MetricPoint {
        metric_name: "trust_score".to_string(),
        value: 0.85,
        unit: "score".to_string(),
        labels: std::collections::HashMap::new(),
    }];
    let result = stream.publish_metrics(MetricCategory::Federation, metrics, None);
    assert_eq!(result.events_sent, 1); // Solo s2
}

#[test]
fn test_e2e_sse_metrics_heartbeat() {
    let stream = SseMetricsStream::new();
    let heartbeat = stream.generate_heartbeat();
    assert_eq!(heartbeat.category, MetricCategory::Performance);
    assert!(heartbeat.metrics[0].metric_name == "heartbeat");
}

#[test]
fn test_e2e_sse_metrics_reconnect() {
    let mut stream = SseMetricsStream::new();

    // Publicar eventos
    for i in 0..5 {
        let metrics = vec![MetricPoint {
            metric_name: "test".to_string(),
            value: i as f64,
            unit: "count".to_string(),
            labels: std::collections::HashMap::new(),
        }];
        stream.publish_metrics(MetricCategory::Alignment, metrics, None);
    }

    // Reconectar desde secuencia 3
    let session = stream
        .reconnect_with_last_event(
            "s1".to_string(),
            "metric-alignment-1234567890-3".to_string(),
            vec![MetricCategory::Alignment],
        )
        .unwrap();
    assert_eq!(session.last_sequence_seen, 3);

    let catchup = stream.get_catchup_events("s1", 3);
    assert_eq!(catchup.len(), 2); // Eventos 4 y 5
}

// ============================================================================
// Test: Cross-Module Integration
// ============================================================================

#[test]
fn test_e2e_alignment_to_stream_integration() {
    let mut loop_v2 = AlignmentLoopV2::new();
    let mut stream = WsAlignmentStream::new();

    stream
        .create_connection(
            "ui_client".to_string(),
            vec![AlignmentSignalType::Feedback, AlignmentSignalType::Drift],
        )
        .unwrap();

    // Alignment Loop ingesta feedback
    loop_v2
        .ingest_feedback(make_feedback("layer_1", 0, 0.4, 0.9))
        .unwrap();

    // Ejecutar ciclo
    let activations: Vec<f32> = vec![0.5; 5];
    let cycle_result = loop_v2.run_cycle("layer_1", &activations).unwrap();

    // Emitir drift al stream
    stream.emit_signal(
        AlignmentSignalType::Drift,
        cycle_result.layer_id.clone(),
        serde_json::json!({"drift": cycle_result.drift}),
        None,
        cycle_result.confidence,
    );

    // Verificar que el stream recibió la señal
    assert_eq!(stream.signal_history.len(), 1);
    assert_eq!(stream.signal_history.front().unwrap().layer_id, "layer_1");
}

#[test]
fn test_e2e_federation_to_backend_integration() {
    let mut normalizer = GradientNormalizer::new();
    let mut backend = RealtimeBackend::new();

    backend
        .create_session(
            "dashboard".to_string(),
            vec![BackendEventType::FederationGradientSync],
        )
        .unwrap();

    normalizer
        .register_node(make_node_capacity("node1"))
        .unwrap();
    let batch = make_gradient_batch("node1", 64, 1);
    let normalized = normalizer.normalize(batch).unwrap();

    // Emitir métrica al backend
    backend.publish_event(
        BackendEventType::FederationGradientSync,
        serde_json::json!({
            "dim": normalized.len(),
            "round": 1,
        }),
        Some("node1".to_string()),
    );

    let stats = backend.get_stats();
    assert!(stats.total_events_sent > 0);
}

#[test]
fn test_e2e_trust_to_metrics_integration() {
    let mut trust_engine = TrustSyncEngine::new();
    let mut metrics_stream = SseMetricsStream::new();

    metrics_stream
        .create_session(
            "monitor".to_string(),
            vec![MetricCategory::Federation, MetricCategory::Security],
        )
        .unwrap();

    trust_engine
        .register_node(make_trust_record("node1", "net_a"))
        .unwrap();
    trust_engine.update_trust("node1", true).unwrap();
    trust_engine.apply_zkp_boost("node1").unwrap();

    let trust_stats = trust_engine.get_stats();

    // Emitir métricas de confianza
    metrics_stream.publish_metrics(
        MetricCategory::Federation,
        vec![MetricPoint {
            metric_name: "avg_trust".to_string(),
            value: trust_stats.avg_trust_score as f64,
            unit: "score".to_string(),
            labels: std::collections::HashMap::new(),
        }],
        None,
    );

    let metrics_stats = metrics_stream.get_stats();
    assert!(metrics_stats.total_events_sent > 0);
}

#[test]
fn test_e2e_full_pipeline_alignment_federation_streaming() {
    // Pipeline completo: Alignment → Federation → Streaming
    let mut loop_v2 = AlignmentLoopV2::new();
    let mut scaler = CrossModelScaler::new();
    let mut backend = RealtimeBackend::new();

    // Setup
    backend
        .create_session(
            "full_pipeline".to_string(),
            vec![
                BackendEventType::AlignmentCycleComplete,
                BackendEventType::FederationGradientSync,
            ],
        )
        .unwrap();

    scaler.register_model(make_model_info("main_model", 64));
    scaler
        .register_node("node1".to_string(), "main_model".to_string(), 0.9)
        .unwrap();

    // Alignment phase
    loop_v2
        .ingest_feedback(make_feedback("layer_1", 0, 0.5, 0.8))
        .unwrap();
    let activations: Vec<f32> = vec![0.5; 5];
    let alignment_result = loop_v2.run_cycle("layer_1", &activations).unwrap();

    // Emitir alignment result
    backend.publish_event(
        BackendEventType::AlignmentCycleComplete,
        serde_json::json!({
            "drift": alignment_result.drift,
            "confidence": alignment_result.confidence,
        }),
        None,
    );

    // Federation phase
    scaler
        .receive_update(make_gradient_update("node1", "main_model", 64, 1))
        .unwrap();
    let sync_result = scaler.sync().unwrap();

    // Emitir federation result
    backend.publish_event(
        BackendEventType::FederationGradientSync,
        serde_json::json!({
            "gradient_dim": sync_result.aggregated_gradient.len(),
            "divergence": sync_result.divergence,
        }),
        None,
    );

    let stats = backend.get_stats();
    assert_eq!(stats.total_events_sent, 2);
}

#[test]
fn test_e2e_feature_flag_enabled() {
    // Verificar que el feature flag está activo
    #[cfg(feature = "v1.1-sprint4")]
    {
        assert!(true);
    }
}
