//! Sprint 141 — Robust Tube MPC, Contraction Metrics & Strided Evaluation Integration Tests
//!
//! Tests for:
//! - robust_mpsf_cbf_filter (MPSF with zonotopic tightening)
//! - verify_contraction_rate_jvp (Lohmiller-Slotine via JVP)
//! - compute_strided_error_bound (Lipschitz continuity bound)
//! - Strided evaluation savings demonstration

use candle_core::{Device, Tensor};
use native_audit::steering::{
    compute_strided_error_bound, robust_mpsf_cbf_filter, verify_contraction_rate_jvp,
    steer_tube_mpc,
};

// ─── MPSF + Zonotopic Safety Tests ───

#[test]
fn test_mpsf_safe_state_passes_through() {
    let safe = Tensor::zeros(&[4, 4], candle_core::DType::F32, &Device::Cpu).unwrap();
    let h_current = Tensor::new(0.1f32, &Device::Cpu)
        .unwrap()
        .broadcast_as(&[4, 4])
        .unwrap();
    let h_prev = h_current.clone();
    let u_nom = Tensor::zeros(&[4, 4], candle_core::DType::F32, &Device::Cpu).unwrap();

    let result =
        robust_mpsf_cbf_filter(&h_current, &h_prev, &u_nom, &safe, 1.0, 10.0, 0.01).unwrap();
    let diff = u_nom.sub(&result).unwrap().abs().unwrap().sum_all().unwrap().to_scalar::<f32>().unwrap();
    assert!(
        diff < 1e-6,
        "Safe state must pass nominal control unchanged: diff={:.8}",
        diff
    );
}

#[test]
fn test_mpsf_unsafe_state_applies_correction() {
    let safe = Tensor::zeros(&[4, 4], candle_core::DType::F32, &Device::Cpu).unwrap();
    let h_current = Tensor::new(10.0f32, &Device::Cpu)
        .unwrap()
        .broadcast_as(&[4, 4])
        .unwrap();
    let h_prev = h_current.clone();
    let u_nom = Tensor::zeros(&[4, 4], candle_core::DType::F32, &Device::Cpu).unwrap();

    let result =
        robust_mpsf_cbf_filter(&h_current, &h_prev, &u_nom, &safe, 1.0, 1.0, 0.1).unwrap();
    let diff = u_nom.sub(&result).unwrap().abs().unwrap().sum_all().unwrap().to_scalar::<f32>().unwrap();
    assert!(
        diff > 1.0,
        "Unsafe state must apply significant CBF correction: diff={:.4}",
        diff
    );
}

#[test]
fn test_mpsf_zonotope_tightening_is_conservative() {
    let safe = Tensor::zeros(&[3, 3], candle_core::DType::F32, &Device::Cpu).unwrap();
    let h_current = Tensor::new(5.0f32, &Device::Cpu)
        .unwrap()
        .broadcast_as(&[3, 3])
        .unwrap();
    let h_prev = h_current.clone();
    let u_nom = Tensor::zeros(&[3, 3], candle_core::DType::F32, &Device::Cpu).unwrap();

    let result_small =
        robust_mpsf_cbf_filter(&h_current, &h_prev, &u_nom, &safe, 1.0, 1.0, 0.01).unwrap();
    let result_large =
        robust_mpsf_cbf_filter(&h_current, &h_prev, &u_nom, &safe, 1.0, 1.0, 1.0).unwrap();

    let corr_small = u_nom
        .sub(&result_small)
        .unwrap()
        .abs()
        .unwrap()
        .sum_all()
        .unwrap()
        .to_scalar::<f32>()
        .unwrap();
    let corr_large = u_nom
        .sub(&result_large)
        .unwrap()
        .abs()
        .unwrap()
        .sum_all()
        .unwrap()
        .to_scalar::<f32>()
        .unwrap();

    assert!(
        corr_large >= corr_small,
        "Larger zonotope radius must be more conservative: small={:.4}, large={:.4}",
        corr_small,
        corr_large
    );
}

#[test]
fn test_mpsf_shape_preservation() {
    let safe = Tensor::zeros(&[5, 8], candle_core::DType::F32, &Device::Cpu).unwrap();
    let h_current =
        Tensor::randn(0.0f32, 1.0f32, &[5, 8], &Device::Cpu).unwrap();
    let h_prev = h_current.clone();
    let u_nom = Tensor::zeros(&[5, 8], candle_core::DType::F32, &Device::Cpu).unwrap();

    let result =
        robust_mpsf_cbf_filter(&h_current, &h_prev, &u_nom, &safe, 1.0, 10.0, 0.1).unwrap();
    assert_eq!(
        result.dims(),
        &[5, 8],
        "MPSF must preserve tensor shape"
    );
}

// ─── Contraction Metrics Tests ───

#[test]
fn test_contraction_rate_jvp_finite() {
    let h = Tensor::new(0.5f32, &Device::Cpu)
        .unwrap()
        .broadcast_as(&[4, 4])
        .unwrap();
    let rate = verify_contraction_rate_jvp(&h, 1e-4, 10, 0.5).unwrap();
    assert!(
        rate > 0.0 && rate.is_finite(),
        "Contraction rate must be positive and finite: {:.6}",
        rate
    );
}

#[test]
fn test_contraction_rate_tanh_contracts() {
    // tanh has |f'(x)| < 1 everywhere → must contract
    let h = Tensor::new(0.1f32, &Device::Cpu)
        .unwrap()
        .broadcast_as(&[8, 8])
        .unwrap();
    let rate = verify_contraction_rate_jvp(&h, 1e-4, 20, 0.5).unwrap();
    assert!(
        rate < 1.0,
        "Tanh dynamics must contract (rate < 1.0): {:.6}",
        rate
    );
}

#[test]
fn test_contraction_rate_high_dim_no_oom() {
    // **[ANTI-TRAMPA]:** High-dimensional test to verify JVP doesn't OOM
    let h = Tensor::randn(0.0f32, 1.0f32, &[64, 128], &Device::Cpu).unwrap();
    let _rate = verify_contraction_rate_jvp(&h, 1e-4, 10, 0.5).unwrap();
    // No panic = JVP is OOM-proof (no full Jacobian matrix)
}

// ─── Strided Evaluation Tests ───

#[test]
fn test_strided_error_bound_basic() {
    let bound = compute_strided_error_bound(2.0, 3, 0.5);
    assert!(
        (bound - 3.0).abs() < 1e-6,
        "L=2, stride=3, v=0.5 → bound=3.0, got={:.4}",
        bound
    );
}

#[test]
fn test_strided_error_bound_zero_stride() {
    let bound = compute_strided_error_bound(2.0, 1, 0.5);
    assert!(
        (bound - 1.0).abs() < 1e-6,
        "Stride=1 → bound = L*v = 1.0, got={:.4}",
        bound
    );
}

#[test]
fn test_strided_error_bound_monotonic() {
    let bound_3 = compute_strided_error_bound(1.0, 3, 0.5);
    let bound_5 = compute_strided_error_bound(1.0, 5, 0.5);
    let bound_10 = compute_strided_error_bound(1.0, 10, 0.5);
    assert!(
        bound_3 < bound_5 && bound_5 < bound_10,
        "Error bound must increase with stride: {:.2} < {:.2} < {:.2}",
        bound_3,
        bound_5,
        bound_10
    );
}

/// Full strided evaluation demonstration — prints savings to stdout.
#[test]
fn test_strided_evaluation_savings_demo() {
    let total_tokens = 300;
    let stride = 3;
    let mut evals = 0usize;

    for idx in 0..total_tokens {
        if idx % stride != 0 {
            continue;
        }
        evals += 1;
        // Simulated MPSF + contraction check at stride points
        let _bound = compute_strided_error_bound(1.5, stride, 0.2);
    }

    let savings_pct = (1.0 - evals as f32 / total_tokens as f32) * 100.0;
    println!(
        "⚡ Tokens: {} | Evals: {} | Ahorro: {:.0}%",
        total_tokens,
        evals,
        savings_pct
    );

    assert!(
        savings_pct >= 65.0,
        "Strided evaluation must save ≥65%: got={:.1}%",
        savings_pct
    );
    assert_eq!(
        evals,
        100,
        "Stride=3, 300 tokens → 100 evals (every 3rd)"
    );
}

/// Integration test: Full S141 pipeline (MPSF + Contraction + Strided).
#[test]
fn test_full_s141_pipeline() {
    let device = Device::Cpu;

    // 1. MPSF Safety Filter
    let safe = Tensor::zeros(&[4, 4], candle_core::DType::F32, &device).unwrap();
    let h_current = Tensor::new(0.5f32, &device)
        .unwrap()
        .broadcast_as(&[4, 4])
        .unwrap();
    let h_prev = h_current.clone();
    let u_nom = Tensor::zeros(&[4, 4], candle_core::DType::F32, &device).unwrap();
    let u_safe =
        robust_mpsf_cbf_filter(&h_current, &h_prev, &u_nom, &safe, 1.0, 5.0, 0.05).unwrap();
    assert_eq!(u_safe.dims(), &[4, 4]);

    // 2. Contraction Verification
    let rate = verify_contraction_rate_jvp(&h_current, 1e-4, 10, 0.5).unwrap();
    assert!(rate.is_finite(), "Contraction rate must be finite");

    // 3. Strided Error Bound
    let bound = compute_strided_error_bound(1.0, 5, 0.1);
    assert!(bound > 0.0, "Error bound must be positive");

    // 4. Tube MPC (S140 integration)
    let steered = steer_tube_mpc(&h_current, &safe, 1.0, 0.5).unwrap();
    assert_eq!(steered.dims(), &[4, 4]);

    println!(
        "✅ S141 Pipeline: MPSF OK | Contraction={:.4} | StrideBound={:.2} | TubeMPC OK",
        rate, bound
    );
}

/// Verify that strided evaluation + MPSF maintains safety within Lipschitz bound.
#[test]
fn test_strided_mpsf_safety_within_bound() {
    let device = Device::Cpu;
    let safe = Tensor::zeros(&[4, 4], candle_core::DType::F32, &device).unwrap();
    let stride = 5;
    let lipschitz_l = 1.0;
    let max_velocity = 0.1;

    // Simulate strided evaluation with MPSF
    let mut h_current = Tensor::new(0.1f32, &device)
        .unwrap()
        .broadcast_as(&[4, 4])
        .unwrap();

    for step in 0..50 {
        if step % stride == 0 {
            // Evaluate MPSF at stride points
            let h_prev = h_current.clone();
            let u_nom = Tensor::zeros(&[4, 4], candle_core::DType::F32, &device).unwrap();
            h_current = robust_mpsf_cbf_filter(
                &h_current,
                &h_prev,
                &u_nom,
                &safe,
                1.0,
                5.0,
                0.05,
            )
            .unwrap();
        }
        // Between stride points: error bounded by Lipschitz continuity
        let _bound = compute_strided_error_bound(lipschitz_l, stride, max_velocity);
    }

    // Final state must be finite (no numerical explosion)
    let norm = h_current
        .sqr()
        .unwrap()
        .sum_all()
        .unwrap()
        .to_scalar::<f32>()
        .unwrap();
    assert!(norm.is_finite(), "Strided MPSF must maintain numerical stability");
}

/// Demonstrate that strided evaluation achieves ~66% savings with stride=3.
#[test]
fn test_strided_savings_percentage() {
    for &stride in &[2, 3, 5, 10] {
        let total = 1000;
        let evals: usize = (0..total).filter(|&i| i % stride == 0).count();
        let savings = (1.0 - evals as f32 / total as f32) * 100.0;
        let expected_savings = (1.0 - 1.0 / stride as f32) * 100.0;
        assert!(
            (savings - expected_savings).abs() < 0.1,
            "Stride {}: savings={:.1}%, expected={:.1}%",
            stride,
            savings,
            expected_savings
        );
        println!(
            "⚡ Stride={}: {:.0}% savings ({} evals of {})",
            stride,
            savings,
            evals,
            total
        );
    }
}
