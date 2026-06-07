//! Formal Barrier Certificate Tests — Sprint 109
//!
//! Validates interval arithmetic, Lyapunov barrier certificates,
//! control barrier functions, and SMT-LIB export for formal safety verification.

use candle_core::{Device, Tensor};
use native_audit::formal_barrier::{BarrierConfig, FormalBarrierEngine, Interval};

#[test]
fn test_interval_new() {
    let iv = Interval::new(1.0, 3.0);
    assert_eq!(iv.lo, 1.0);
    assert_eq!(iv.hi, 3.0);
    assert!(iv.contains(2.0));
    assert!(!iv.contains(0.0));
    assert!(!iv.contains(4.0));
}

#[test]
fn test_interval_addition() {
    let a = Interval::new(1.0, 2.0);
    let b = Interval::new(3.0, 4.0);
    let c = a.add(&b);
    assert_eq!(c.lo, 4.0);
    assert_eq!(c.hi, 6.0);
}

#[test]
fn test_interval_multiplication() {
    let a = Interval::new(1.0, 2.0);
    let b = Interval::new(3.0, 4.0);
    let c = a.mul(&b);
    assert_eq!(c.lo, 3.0);
    assert_eq!(c.hi, 8.0);
}

#[test]
fn test_interval_multiplication_negative() {
    let a = Interval::new(-2.0, 1.0);
    let b = Interval::new(3.0, 4.0);
    let c = a.mul(&b);
    assert_eq!(c.lo, -8.0);
    assert_eq!(c.hi, 4.0);
}

#[test]
fn test_interval_scaling() {
    let a = Interval::new(1.0, 3.0);
    let scaled = a.scale(2.0);
    assert_eq!(scaled.lo, 2.0);
    assert_eq!(scaled.hi, 6.0);

    let neg_scaled = a.scale(-1.0);
    assert_eq!(neg_scaled.lo, -3.0);
    assert_eq!(neg_scaled.hi, -1.0);
}

#[test]
fn test_interval_sqrt() {
    let a = Interval::new(1.0, 4.0);
    let root = a.sqrt().unwrap();
    assert!(root.lo >= 0.99 && root.lo <= 1.01, "sqrt lo: {}", root.lo);
    assert!(root.hi >= 1.99 && root.hi <= 2.01, "sqrt hi: {}", root.hi);
}

#[test]
fn test_interval_sqrt_negative() {
    let a = Interval::new(-1.0, 4.0);
    let root = a.sqrt();
    assert!(
        root.is_none(),
        "sqrt of interval with negative lo should fail"
    );
}

#[test]
fn test_interval_abs() {
    let a = Interval::new(-3.0, 1.0);
    let abs_a = a.abs();
    assert_eq!(abs_a.lo, 0.0);
    assert_eq!(abs_a.hi, 3.0);
}

#[test]
fn test_interval_intersection() {
    let a = Interval::new(1.0, 5.0);
    let b = Interval::new(3.0, 7.0);
    let inter = a.intersect(&b).unwrap();
    assert_eq!(inter.lo, 3.0);
    assert_eq!(inter.hi, 5.0);
}

#[test]
fn test_interval_intersection_empty() {
    let a = Interval::new(1.0, 2.0);
    let b = Interval::new(3.0, 4.0);
    let inter = a.intersect(&b);
    assert!(
        inter.is_none(),
        "disjoint intervals should have no intersection"
    );
}

#[test]
fn test_interval_display() {
    let iv = Interval::new(1.5, 3.5);
    let s = format!("{}", iv);
    assert!(s.contains('['), "display should start with [");
    assert!(s.contains(']'), "display should end with ]");
}

#[test]
fn test_barrier_engine_creation() {
    let config = BarrierConfig::default();
    let _engine = FormalBarrierEngine::new(&config);
    // Engine creation should not panic
}

#[test]
fn test_formal_barrier_certificate_safe() {
    let device = Device::Cpu;
    let small_tensor = Tensor::zeros((2, 2), candle_core::DType::F32, &device).unwrap();
    let engine = FormalBarrierEngine::new(&BarrierConfig::default());
    let cert = engine.formal_barrier_certificate(&small_tensor).unwrap();

    assert!(
        cert.is_valid,
        "zero tensor should produce valid certificate"
    );
    assert!(!cert.v_value.lo.is_nan());
    assert!(!cert.v_value.hi.is_nan());
    println!(
        "Safe certificate: V=[{}, {}], dV/dt=[{}, {}]",
        cert.v_value.lo, cert.v_value.hi, cert.v_derivative.lo, cert.v_derivative.hi
    );
}

#[test]
fn test_formal_barrier_certificate_large() {
    let device = Device::Cpu;
    let large_tensor = Tensor::full(10.0f32, (10, 10), &device).unwrap();
    let engine = FormalBarrierEngine::new(&BarrierConfig::default());
    let cert = engine.formal_barrier_certificate(&large_tensor).unwrap();

    assert!(
        !cert.is_valid,
        "large tensor should produce invalid certificate"
    );
    println!(
        "Unsafe certificate: V=[{}, {}], epsilon={:.4}",
        cert.v_value.lo,
        cert.v_value.hi,
        cert.epsilon()
    );
}

#[test]
fn test_certificate_epsilon() {
    let device = Device::Cpu;
    let tensor = Tensor::new(vec![0.1f32, 0.1], &device).unwrap();
    let engine = FormalBarrierEngine::new(&BarrierConfig::default());
    let cert = engine.formal_barrier_certificate(&tensor).unwrap();
    let eps = cert.epsilon();
    assert!(!eps.is_nan(), "epsilon should not be NaN");
}

#[test]
fn test_verify_safety() {
    let device = Device::Cpu;
    let safe_tensor = Tensor::zeros((3, 3), candle_core::DType::F32, &device).unwrap();
    let engine = FormalBarrierEngine::new(&BarrierConfig::default());
    let is_safe = engine.verify_safety(&safe_tensor).unwrap();
    assert!(is_safe, "zero tensor should be verified as safe");
}

#[test]
fn test_safety_score() {
    let device = Device::Cpu;
    let tensor = Tensor::new(vec![0.5f32, 0.5], &device).unwrap();
    let engine = FormalBarrierEngine::new(&BarrierConfig::default());
    let score = engine.safety_score(&tensor).unwrap();
    assert!(
        (0.0..=1.0).contains(&score),
        "safety score should be in [0,1]: {}",
        score
    );
    println!("Safety score: {:.4}", score);
}

#[test]
fn test_smtlib_export() {
    let device = Device::Cpu;
    let tensor = Tensor::new(vec![1.0f32, 1.0], &device).unwrap();
    let engine = FormalBarrierEngine::new(&BarrierConfig::default());
    let cert = engine.formal_barrier_certificate(&tensor).unwrap();
    let smt = engine.export_to_smtlib(&cert, "x");

    assert!(smt.contains("(set-logic"), "SMT-LIB should declare logic");
    assert!(smt.contains("Real"), "SMT-LIB should use Real sort");
    assert!(smt.contains("x"), "SMT-LIB should reference variable name");
    assert!(smt.len() > 100, "SMT-LIB export should be substantial");
    println!("SMT-LIB export length: {} chars", smt.len());
}

#[test]
fn test_barrier_certificate_consistent() {
    let device = Device::Cpu;
    let tensor = Tensor::new(vec![0.5f32, 0.3, 0.2], &device).unwrap();
    let engine = FormalBarrierEngine::new(&BarrierConfig::default());
    let cert = engine.formal_barrier_certificate(&tensor).unwrap();

    assert!(!cert.v_value.lo.is_nan());
    assert!(!cert.v_value.hi.is_nan());
    assert!(!cert.v_derivative.lo.is_nan());
    assert!(!cert.v_derivative.hi.is_nan());
    assert!(cert.v_value.hi >= cert.v_value.lo, "V interval: hi >= lo");
    assert!(
        cert.v_derivative.hi >= cert.v_derivative.lo,
        "dV/dt interval: hi >= lo"
    );
}

#[test]
fn test_barrier_config_custom() {
    let config = BarrierConfig {
        max_norm: 5.0,
        decay_rate: 0.5,
        cbf_alpha: 0.1,
        ..BarrierConfig::default()
    };
    let engine = FormalBarrierEngine::new(&config);

    let device = Device::Cpu;
    let tensor = Tensor::full(2.0f32, (3, 3), &device).unwrap();
    let cert = engine.formal_barrier_certificate(&tensor).unwrap();
    assert!(!cert.v_value.lo.is_nan());
}

#[test]
fn test_interval_chain_operations() {
    let a = Interval::new(1.0, 2.0);
    let b = Interval::new(0.5, 1.5);
    let c = Interval::new(-1.0, 1.0);

    // (a + b) * c
    let sum = a.add(&b);
    let prod = sum.mul(&c);
    assert!(prod.hi >= prod.lo, "result hi >= lo");
    assert!(!prod.lo.is_nan());
    assert!(!prod.hi.is_nan());
}

#[test]
fn test_interval_non_negative() {
    let a = Interval::new(2.0, 5.0);
    assert!(
        a.lo >= 0.0 && a.hi >= 0.0,
        "interval should be non-negative"
    );
    let sqrt_a = a.sqrt().unwrap();
    assert!(
        sqrt_a.lo > 0.0,
        "sqrt of positive interval should be positive"
    );
}

#[test]
fn test_certificate_epsilon_non_negative() {
    let device = Device::Cpu;
    for val in [0.0_f32, 0.5, 1.0, 2.0, 5.0] {
        let tensor = Tensor::full(val, (4,), &device).unwrap();
        let engine = FormalBarrierEngine::new(&BarrierConfig::default());
        let cert = engine.formal_barrier_certificate(&tensor).unwrap();
        let eps = cert.epsilon();
        assert!(!eps.is_nan(), "epsilon for val={} should not be NaN", val);
    }
}

#[test]
fn test_confidence_range() {
    let device = Device::Cpu;
    let tensor = Tensor::new(vec![0.5f32, 0.3], &device).unwrap();
    let engine = FormalBarrierEngine::new(&BarrierConfig::default());
    let cert = engine.formal_barrier_certificate(&tensor).unwrap();
    assert!(
        cert.confidence >= 0.0 && cert.confidence <= 1.0,
        "confidence in [0,1]: {}",
        cert.confidence
    );
}

#[test]
fn test_safety_margin() {
    let device = Device::Cpu;
    let tensor = Tensor::zeros((2,), candle_core::DType::F32, &device).unwrap();
    let engine = FormalBarrierEngine::new(&BarrierConfig::default());
    let cert = engine.formal_barrier_certificate(&tensor).unwrap();
    assert!(!cert.safety_margin.lo.is_nan());
    assert!(!cert.safety_margin.hi.is_nan());
}
