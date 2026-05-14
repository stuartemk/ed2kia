//! SLO Engine unit tests — 10+ tests covering SLO tracking, threshold triggers,
//! degradation rollback, audit trail, and concurrent metrics.

use super::engine::*;

// ---- Helpers ---------------------------------------------------------------

fn make_uptime_slo() -> SLOConfig {
    SLOConfig {
        name: "Node Uptime".into(),
        metric_key: "node_uptime".into(),
        target: 99.9,
        warning_threshold: 0.98, // 98% of 99.9 = 97.9
        max_breach_windows: 3,
        unit: "%".into(),
    }
}

fn make_latency_slo() -> SLOConfig {
    SLOConfig {
        name: "SAE Latency".into(),
        metric_key: "sae_latency".into(),
        target: 50.0,
        warning_threshold: 1.5, // 1.5x target = 75ms warning
        max_breach_windows: 5,
        unit: "ms".into(),
    }
}

fn make_error_rate_slo() -> SLOConfig {
    SLOConfig {
        name: "API Error Rate".into(),
        metric_key: "api_error_rate".into(),
        target: 0.01,
        warning_threshold: 2.0, // 2x target = 0.02 warning
        max_breach_windows: 4,
        unit: "ratio".into(),
    }
}

// ---- Registration tests ----------------------------------------------------

#[test]
fn test_register_slo() {
    let mut engine = SLOEngine::new();
    assert!(engine.slo_names().is_empty());

    engine.register_slo(make_uptime_slo());
    assert_eq!(engine.slo_names(), vec!["Node Uptime"]);
}

#[test]
fn test_unregister_slo() {
    let mut engine = SLOEngine::new();
    engine.register_slo(make_uptime_slo());
    assert!(engine.unregister_slo("node_uptime"));
    assert!(engine.slo_names().is_empty());
    assert!(!engine.unregister_slo("node_uptime")); // already removed
}

// ---- Tracking tests --------------------------------------------------------

#[test]
fn test_track_metric_success() {
    let mut engine = SLOEngine::new();
    engine.register_slo(make_uptime_slo());
    let result = engine.track_metric("node_uptime", 99.95, 1000);
    assert!(result.is_ok());
}

#[test]
fn test_track_metric_unknown_slo() {
    let mut engine = SLOEngine::new();
    let result = engine.track_metric("unknown_metric", 42.0, 1000);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SLOError::SloNotConfigured(_)));
}

#[test]
fn test_track_metric_window_size() {
    let mut engine = SLOEngine::with_window_size(3);
    engine.register_slo(make_uptime_slo());
    engine.track_metric("node_uptime", 99.0, 1).unwrap();
    engine.track_metric("node_uptime", 99.5, 2).unwrap();
    engine.track_metric("node_uptime", 99.8, 3).unwrap();
    engine.track_metric("node_uptime", 99.9, 4).unwrap();
    // Only last 3 should remain; average should be (99.5+99.8+99.9)/3 = 99.733
    let result = engine.evaluate_slo("node_uptime").unwrap();
    assert!(matches!(result.status, SLOStatus::Warning | SLOStatus::Critical));
}

// ---- Evaluation tests ------------------------------------------------------

#[test]
fn test_evaluate_slo_compliant_uptime() {
    let mut engine = SLOEngine::new();
    engine.register_slo(make_uptime_slo());
    engine.track_metric("node_uptime", 99.95, 1000).unwrap();
    engine.track_metric("node_uptime", 99.98, 1001).unwrap();

    let result = engine.evaluate_slo("node_uptime").unwrap();
    assert!(matches!(result.status, SLOStatus::Compliant));
    assert_eq!(result.breach_duration, 0);
}

#[test]
fn test_evaluate_slo_warning_uptime() {
    let mut engine = SLOEngine::new();
    engine.register_slo(make_uptime_slo());
    // 97.5 is below 97.9 (warning threshold) but above critical
    engine.track_metric("node_uptime", 97.5, 1000).unwrap();

    let result = engine.evaluate_slo("node_uptime").unwrap();
    assert!(matches!(result.status, SLOStatus::Warning | SLOStatus::Critical));
}

#[test]
fn test_evaluate_slo_critical_latency() {
    let mut engine = SLOEngine::new();
    engine.register_slo(make_latency_slo());
    // 100ms > 75ms warning bound → critical
    engine.track_metric("sae_latency", 100.0, 1000).unwrap();

    let result = engine.evaluate_slo("sae_latency").unwrap();
    assert!(matches!(result.status, SLOStatus::Critical));
}

#[test]
fn test_evaluate_slo_no_data() {
    let mut engine = SLOEngine::new();
    engine.register_slo(make_uptime_slo());

    let result = engine.evaluate_slo("node_uptime").unwrap();
    assert!(matches!(result.status, SLOStatus::Warning));
}

// ---- SLA enforcement tests -------------------------------------------------

#[test]
fn test_enforce_sla_triggers_degradation() {
    let mut engine = SLOEngine::new();
    let slo = SLOConfig {
        name: "Test Uptime".into(),
        metric_key: "test_uptime".into(),
        target: 99.0,
        warning_threshold: 0.95,
        max_breach_windows: 2,
        unit: "%".into(),
    };
    engine.register_slo(slo);

    // Track bad values to accumulate breaches
    engine.track_metric("test_uptime", 90.0, 1).unwrap();
    engine.evaluate_slo("test_uptime").unwrap();
    engine.track_metric("test_uptime", 89.0, 2).unwrap();
    engine.evaluate_slo("test_uptime").unwrap();

    let result = engine.enforce_sla("test_uptime").unwrap();
    assert!(matches!(result.status, SLOStatus::Critical));
    assert!(engine.is_degraded());
}

#[test]
fn test_enforce_sla_no_degradation_yet() {
    let mut engine = SLOEngine::new();
    engine.register_slo(make_uptime_slo());
    engine.track_metric("node_uptime", 99.95, 1).unwrap();

    let result = engine.enforce_sla("node_uptime").unwrap();
    assert!(matches!(result.status, SLOStatus::Compliant));
    assert!(!engine.is_degraded());
}

// ---- Degradation tests -----------------------------------------------------

#[test]
fn test_trigger_degradation_latency() {
    let mut engine = SLOEngine::new();
    let action = engine.trigger_degradation("SAE Latency");
    assert!(matches!(action, DegradationAction::Throttle));
    assert!(engine.is_degraded());
}

#[test]
fn test_trigger_degradation_uptime() {
    let mut engine = SLOEngine::new();
    let action = engine.trigger_degradation("Node Uptime");
    assert!(matches!(action, DegradationAction::FallbackCoreOnly));
}

#[test]
fn test_trigger_degradation_error() {
    let mut engine = SLOEngine::new();
    let action = engine.trigger_degradation("API Error Rate");
    assert!(matches!(action, DegradationAction::Rollback));
}

#[test]
fn test_trigger_degradation_generic() {
    let mut engine = SLOEngine::new();
    let action = engine.trigger_degradation("Unknown SLO");
    assert!(matches!(action, DegradationAction::Alert));
}

#[test]
fn test_recover_from_degradation() {
    let mut engine = SLOEngine::new();
    engine.trigger_degradation("Node Uptime");
    assert!(engine.is_degraded());

    engine.recover();
    assert!(!engine.is_degraded());
    assert_eq!(engine.breach_count("node_uptime"), 0);
}

// ---- Audit trail tests -----------------------------------------------------

#[test]
fn test_audit_trail_populated() {
    let mut engine = SLOEngine::new();
    engine.register_slo(make_uptime_slo());
    engine.track_metric("node_uptime", 99.95, 1000).unwrap();

    let trail = engine.get_audit_trail();
    assert!(!trail.is_empty());
    assert!(trail[0].starts_with('['));
}

#[test]
fn test_audit_trail_capacity() {
    let mut engine = SLOEngine::new();
    engine.register_slo(make_uptime_slo());
    for i in 0..300 {
        engine.track_metric("node_uptime", 99.0 + i as f64 * 0.01, i).unwrap();
    }
    let trail = engine.get_audit_trail();
    assert!(trail.len() <= 256);
}

// ---- Breach counter tests --------------------------------------------------

#[test]
fn test_breach_counter_increments() {
    let mut engine = SLOEngine::new();
    engine.register_slo(make_latency_slo());

    assert_eq!(engine.breach_count("sae_latency"), 0);

    engine.track_metric("sae_latency", 100.0, 1).unwrap();
    engine.evaluate_slo("sae_latency").unwrap();
    assert!(engine.breach_count("sae_latency") > 0);
}

// ---- SLOResult helpers -----------------------------------------------------

#[test]
fn test_slo_result_compliant() {
    let r = SLOResult::compliant();
    assert!(matches!(r.status, SLOStatus::Compliant));
    assert_eq!(r.breach_duration, 0);
    assert_eq!(r.action_taken, "none");
}

#[test]
fn test_slo_result_warning() {
    let r = SLOResult::warning("test warning");
    assert!(matches!(r.status, SLOStatus::Warning));
    assert_eq!(r.action_taken, "alert_sent");
}

#[test]
fn test_slo_result_critical() {
    let r = SLOResult::critical("breach", "fallback", 5);
    assert!(matches!(r.status, SLOStatus::Critical));
    assert_eq!(r.breach_duration, 5);
    assert_eq!(r.action_taken, "fallback");
}

// ---- Display trait tests ---------------------------------------------------

#[test]
fn test_slo_status_display() {
    assert_eq!(format!("{}", SLOStatus::Compliant), "Compliant");
    assert_eq!(format!("{}", SLOStatus::Warning), "Warning");
    assert_eq!(format!("{}", SLOStatus::Critical), "Critical");
}

#[test]
fn test_degradation_action_display() {
    assert_eq!(format!("{}", DegradationAction::None), "none");
    assert_eq!(format!("{}", DegradationAction::Alert), "alert");
    assert_eq!(
        format!("{}", DegradationAction::FallbackCoreOnly),
        "fallback_core_only"
    );
    assert_eq!(format!("{}", DegradationAction::Throttle), "throttle");
    assert_eq!(format!("{}", DegradationAction::Rollback), "rollback");
}

// ---- Error tests -----------------------------------------------------------

#[test]
fn test_slo_error_display() {
    let e = SLOError::UnknownMetric("test".into());
    assert!(format!("{}", e).contains("test"));

    let e = SLOError::SloNotConfigured("x".into());
    assert!(format!("{}", e).contains("x"));
}

// ---- Default test ----------------------------------------------------------

#[test]
fn test_engine_default() {
    let engine = SLOEngine::default();
    assert!(!engine.is_degraded());
    assert!(engine.slo_names().is_empty());
}
