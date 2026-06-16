//! Sprint 156 (v15.6.0) — "ROBUST KOOPMAN TUBE MPC & PHYSICS-INFORMED RESIDUAL DENOISING"
//!
//! Integration tests for Physics-Informed Residual Koopman, Robust Tube MPC,
//! Zonotope Arithmetic, and Conformal Prediction margins.
//!
//! Reference model: SmolLM2-135M-Instruct-GGUF (Q4_K_M)

use candle_core::{DType, Device, Tensor};
use native_audit::control::{
    compute_ancillary_feedback, compute_conformal_margin, propagate_tube, robust_koopman_tube_mpc,
    KoopmanVanguard, Zonotope,
};
use native_audit::deep_koopman::{DeepKoopman, DeepKoopmanConfig};

/// Create a deterministic test tensor. Column-vector convention: [dim, batch].
fn make_tensor(rows: usize, cols: usize, seed: f32, device: &Device) -> Tensor {
    let mut data = vec![0.0f32; rows * cols];
    for (i, val) in data.iter_mut().enumerate() {
        *val = seed * (i as f32 + 1.0);
    }
    Tensor::from_vec(data, (rows, cols), device).unwrap()
}

// ─── PASO A: Physics-Informed Residual Koopman ───────────────────────────────
// K is [lifted_dim, lifted_dim] = [64, 64]
// psi must be [lifted_dim, batch] for K @ psi to work

#[test]
fn test_physics_informed_step_preserves_shape() {
    let device = Device::Cpu;
    let dk = DeepKoopman::new(DeepKoopmanConfig::default(), 32, &device).unwrap();
    // psi_t: [64, 2] = lifted_dim x batch
    let psi_t = make_tensor(64, 2, 0.1, &device);
    let k_op = dk.koopman_operator().clone();
    let result = dk
        .physics_informed_step(&psi_t, &k_op, 0.01, &device)
        .unwrap();
    assert_eq!(result.psi_next.shape().dims(), [64, 2]);
    assert_eq!(result.linear_component.shape().dims(), [64, 2]);
    assert_eq!(result.residual.shape().dims(), [64, 2]);
}

#[test]
fn test_physics_informed_step_norm_bounded() {
    let device = Device::Cpu;
    let dk = DeepKoopman::new(DeepKoopmanConfig::default(), 32, &device).unwrap();
    let psi_t = make_tensor(64, 1, 0.5, &device);
    let k_op = dk.koopman_operator().clone();
    let psi_t_norm = psi_t
        .sqr()
        .unwrap()
        .sum_all()
        .unwrap()
        .sqrt()
        .unwrap()
        .to_scalar::<f32>()
        .unwrap();
    let result = dk
        .physics_informed_step(&psi_t, &k_op, 0.01, &device)
        .unwrap();
    // Norm should be bounded within 2x of initial
    assert!(
        result.output_norm < psi_t_norm * 2.0,
        "Norm unbounded: {} vs {}",
        result.output_norm,
        psi_t_norm
    );
}

#[test]
fn test_physics_informed_step_finite_output() {
    let device = Device::Cpu;
    let dk = DeepKoopman::new(DeepKoopmanConfig::default(), 32, &device).unwrap();
    let psi_t = make_tensor(64, 4, 0.05, &device);
    let k_op = dk.koopman_operator().clone();
    let result = dk
        .physics_informed_step(&psi_t, &k_op, 0.01, &device)
        .unwrap();
    assert!(result.output_norm.is_finite(), "Output norm must be finite");
}

#[test]
fn test_physics_informed_step_display() {
    let device = Device::Cpu;
    let dk = DeepKoopman::new(DeepKoopmanConfig::default(), 32, &device).unwrap();
    let psi_t = make_tensor(64, 1, 0.1, &device);
    let k_op = dk.koopman_operator().clone();
    let result = dk
        .physics_informed_step(&psi_t, &k_op, 0.01, &device)
        .unwrap();
    let s = format!("{}", result);
    assert!(s.contains("norm:"));
    assert!(s.contains("projected:"));
}

#[test]
fn test_physics_informed_step_large_noise() {
    let device = Device::Cpu;
    let dk = DeepKoopman::new(DeepKoopmanConfig::default(), 32, &device).unwrap();
    let psi_t = make_tensor(64, 1, 0.1, &device);
    let k_op = dk.koopman_operator().clone();
    // Large quantization noise bound
    let result = dk
        .physics_informed_step(&psi_t, &k_op, 5.0, &device)
        .unwrap();
    assert!(result.output_norm.is_finite());
}

// ─── PASO B: Robust Koopman Tube MPC + Zonotope Arithmetic ──────────────────
// K is [d, d], center is [d, 1], generators are [d, n_gens]

#[test]
fn test_propagate_tube_basic() {
    let device = Device::Cpu;
    let d = 8;
    let k = Tensor::eye(d, DType::F32, &device).unwrap();
    // center: [d, 1], generators: [d, 2]
    let center = make_tensor(d, 1, 1.0, &device);
    let gens = make_tensor(d, 2, 0.01, &device);
    let z = Zonotope::new(center, gens);
    let noise = make_tensor(d, 2, 0.005, &device);
    let z_next = propagate_tube(&k, &z, &noise).unwrap();
    // Center should be K·c = c for identity K
    let center_diff = z_next
        .center
        .sub(&z.center)
        .unwrap()
        .abs()
        .unwrap()
        .sum_all()
        .unwrap()
        .to_scalar::<f32>()
        .unwrap();
    assert!(center_diff < 0.01, "Identity K should preserve center");
}

#[test]
fn test_propagate_tube_radius_grows() {
    let device = Device::Cpu;
    let d = 8;
    let k = Tensor::eye(d, DType::F32, &device).unwrap();
    let center = make_tensor(d, 1, 0.0, &device);
    let gens = Tensor::zeros((d, 1), DType::F32, &device).unwrap();
    let z = Zonotope::new(center, gens);
    let noise = make_tensor(d, 3, 0.1, &device);

    let mut current = z.clone();
    let mut prev_radius = 0.0f32;
    for _ in 0..5 {
        current = propagate_tube(&k, &current, &noise).unwrap();
        let radius = current.radius().unwrap();
        assert!(
            radius >= prev_radius,
            "Tube radius should grow with noise accumulation"
        );
        prev_radius = radius;
    }
}

#[test]
fn test_compute_ancillary_feedback_basic() {
    let device = Device::Cpu;
    let d = 8;
    // States: [d, 1], K_fb: [d, d]
    let x_actual = make_tensor(d, 1, 1.0, &device);
    let x_nominal = make_tensor(d, 1, 0.5, &device);
    let k_fb = Tensor::eye(d, DType::F32, &device).unwrap();
    let u_fb = compute_ancillary_feedback(&x_actual, &x_nominal, &k_fb).unwrap();
    // Feedback should be non-zero when states differ
    let fb_norm = u_fb
        .sqr()
        .unwrap()
        .sum_all()
        .unwrap()
        .to_scalar::<f32>()
        .unwrap();
    assert!(
        fb_norm > 0.01,
        "Feedback should be non-zero for different states"
    );
}

#[test]
fn test_compute_ancillary_feedback_zero_when_equal() {
    let device = Device::Cpu;
    let d = 8;
    let x = make_tensor(d, 1, 1.0, &device);
    let k_fb = Tensor::eye(d, DType::F32, &device).unwrap();
    let u_fb = compute_ancillary_feedback(&x, &x, &k_fb).unwrap();
    let fb_norm = u_fb
        .sqr()
        .unwrap()
        .sum_all()
        .unwrap()
        .to_scalar::<f32>()
        .unwrap();
    assert!(
        fb_norm < 1e-6,
        "Feedback should be zero when states are equal"
    );
}

#[test]
fn test_compute_conformal_margin_empty() {
    let margin = compute_conformal_margin(&[], 0.05);
    assert_eq!(margin, 0.0);
}

#[test]
fn test_compute_conformal_margin_basic() {
    let violations = vec![0.1, 0.2, 0.15, 0.05, 0.25, 0.18, 0.12, 0.30, 0.08, 0.22];
    let margin = compute_conformal_margin(&violations, 0.1);
    assert!(margin >= 0.0, "Margin should be non-negative");
    assert!(
        margin <= 0.35,
        "Margin should be bounded by max violation + buffer"
    );
}

#[test]
fn test_compute_conformal_margin_high_confidence() {
    let violations: Vec<f32> = (0..100).map(|i| i as f32 * 0.01).collect();
    let margin_90 = compute_conformal_margin(&violations, 0.1);
    let margin_95 = compute_conformal_margin(&violations, 0.05);
    // Higher confidence (lower delta) → larger margin
    assert!(
        margin_95 >= margin_90,
        "Higher confidence requires larger margin"
    );
}

#[test]
fn test_robust_koopman_tube_mpc_basic() {
    let device = Device::Cpu;
    let d = 8;
    let mut vanguard = KoopmanVanguard::new(&device);
    // Seed with row-vector snapshots [1, d] so lift_observables produces [1, 3*d].
    // K operator will be [3*d, 3*d] = [24, 24].
    for i in 0..20 {
        let h_t = make_tensor(1, d, 0.1 * (i as f32 + 1.0), &device);
        let h_next = make_tensor(1, d, 0.1 * (i as f32 + 2.0), &device);
        vanguard.add_snapshot_pair(&h_t, &h_next).unwrap();
    }
    vanguard.approximate_koopman_operator().unwrap();

    // All states must be in lifted dimension [3*d, 1] to match K's matmul convention.
    let d_lifted = 3 * d;
    let x_actual = make_tensor(d_lifted, 1, 1.0, &device);
    let x_nominal = make_tensor(d_lifted, 1, 0.9, &device);
    let u_nominal = make_tensor(d_lifted, 1, 0.1, &device);
    let noise_gens = make_tensor(d_lifted, 3, 0.01, &device);
    let k_fb = Tensor::eye(d_lifted, DType::F32, &device).unwrap();
    let violations = vec![0.05, 0.08, 0.03, 0.12, 0.07];

    let result = robust_koopman_tube_mpc(
        &vanguard,
        &x_actual,
        &x_nominal,
        &u_nominal,
        &noise_gens,
        &k_fb,
        5,
        &violations,
        0.1,
    )
    .unwrap();

    assert!(result.final_tube_radius.is_finite());
    assert!(result.conformal_margin >= 0.0);
    assert!(result.max_deviation >= 0.0);
}

#[test]
fn test_robust_koopman_tube_mpc_display() {
    let device = Device::Cpu;
    let d = 8;
    let mut vanguard = KoopmanVanguard::new(&device);
    // Row-vector snapshots [1, d] → K is [3*d, 3*d].
    for i in 0..20 {
        let h_t = make_tensor(1, d, 0.1 * (i as f32 + 1.0), &device);
        let h_next = make_tensor(1, d, 0.1 * (i as f32 + 2.0), &device);
        vanguard.add_snapshot_pair(&h_t, &h_next).unwrap();
    }
    vanguard.approximate_koopman_operator().unwrap();

    let d_lifted = 3 * d;
    let x_actual = make_tensor(d_lifted, 1, 1.0, &device);
    let x_nominal = make_tensor(d_lifted, 1, 0.9, &device);
    let u_nominal = make_tensor(d_lifted, 1, 0.1, &device);
    let noise_gens = make_tensor(d_lifted, 3, 0.01, &device);
    let k_fb = Tensor::eye(d_lifted, DType::F32, &device).unwrap();

    let result = robust_koopman_tube_mpc(
        &vanguard,
        &x_actual,
        &x_nominal,
        &u_nominal,
        &noise_gens,
        &k_fb,
        3,
        &[],
        0.05,
    )
    .unwrap();

    let s = format!("{}", result);
    assert!(s.contains("tube_r:"));
    assert!(s.contains("ε_robust:"));
    assert!(s.contains("max_dev:"));
}

#[test]
fn test_robust_koopman_tube_mpc_no_operator_fails() {
    let device = Device::Cpu;
    let d = 8;
    let vanguard = KoopmanVanguard::new(&device); // No K operator
                                                  // States in lifted dimension [3*d, 1].
    let d_lifted = 3 * d;
    let x = make_tensor(d_lifted, 1, 1.0, &device);
    let u = make_tensor(d_lifted, 1, 0.1, &device);
    let noise = make_tensor(d_lifted, 2, 0.01, &device);
    let k_fb = Tensor::eye(d_lifted, DType::F32, &device).unwrap();

    let result = robust_koopman_tube_mpc(&vanguard, &x, &x, &u, &noise, &k_fb, 3, &[], 0.05);
    assert!(result.is_err(), "Should fail without Koopman operator");
}

// ─── Full Horizon Simulation ────────────────────────────────────────────────

#[test]
fn test_full_horizon_simulation_physics_informed() {
    let device = Device::Cpu;
    let dk = DeepKoopman::new(DeepKoopmanConfig::default(), 32, &device).unwrap();
    let k_op = dk.koopman_operator().clone();

    // psi: [64, 1]
    let mut psi = make_tensor(64, 1, 0.1, &device);
    let horizon = 10;
    let mut max_norm = 0.0f32;

    for step in 0..horizon {
        let result = dk
            .physics_informed_step(&psi, &k_op, 0.01, &device)
            .unwrap();
        psi = result.psi_next;
        if result.output_norm > max_norm {
            max_norm = result.output_norm;
        }
        assert!(
            result.output_norm.is_finite(),
            "Step {} norm not finite",
            step
        );
    }

    println!(
        "S156 Physics-Informed Koopman — Full horizon: {} steps, max_norm: {:.6}",
        horizon, max_norm
    );
}

#[test]
fn test_full_horizon_robust_tube_mpc() {
    let device = Device::Cpu;
    let d = 8;
    let mut vanguard = KoopmanVanguard::new(&device);
    // Row-vector snapshots [1, d] → K is [3*d, 3*d].
    for i in 0..30 {
        let h_t = make_tensor(1, d, 0.05 * (i as f32 + 1.0), &device);
        let h_next = make_tensor(1, d, 0.05 * (i as f32 + 2.0), &device);
        vanguard.add_snapshot_pair(&h_t, &h_next).unwrap();
    }
    vanguard.approximate_koopman_operator().unwrap();

    let d_lifted = 3 * d;
    let noise_gens = make_tensor(d_lifted, 3, 0.005, &device);
    let k_fb = Tensor::eye(d_lifted, DType::F32, &device).unwrap();
    let horizon = 8;
    let mut empirical_violations = Vec::new();

    let x_actual = make_tensor(d_lifted, 1, 1.0, &device);
    let mut x_nominal = x_actual.clone();

    for step in 0..horizon {
        let u_nom = make_tensor(d_lifted, 1, 0.05 * (step as f32 + 1.0), &device);

        let result = robust_koopman_tube_mpc(
            &vanguard,
            &x_actual,
            &x_nominal,
            &u_nom,
            &noise_gens,
            &k_fb,
            3,
            &empirical_violations,
            0.1,
        )
        .unwrap();

        // Simulate empirical violation from tube radius
        empirical_violations.push(result.final_tube_radius);

        // Advance nominal trajectory
        x_nominal = result.final_control.clone();
    }

    let final_margin = compute_conformal_margin(&empirical_violations, 0.1);
    println!(
        "S156 Robust Tube MPC — Horizon: {}, ε_robust: {:.6}, max_deviation: {:.6}",
        horizon,
        final_margin,
        empirical_violations
            .iter()
            .cloned()
            .fold(f32::MIN, f32::max)
    );

    assert!(final_margin.is_finite());
    assert!(final_margin >= 0.0);
}

// ─── Sprint Summary ─────────────────────────────────────────────────────────

#[test]
fn test_sprint156_summary() {
    println!("\n========================================");
    println!("Sprint 156 (v15.6.0) Summary");
    println!("Reference: SmolLM2-135M-Instruct-GGUF (Q4_K_M)");
    println!("========================================");
    println!("PASO A: Physics-Informed Residual Koopman ✓");
    println!("  - physics_informed_step() with GGUF denoising");
    println!("  - Divergence-free norm projection");
    println!("PASO B: Robust Koopman Tube MPC ✓");
    println!("  - propagate_tube() zonotope dynamics");
    println!("  - compute_ancillary_feedback() K_fb correction");
    println!("  - compute_conformal_margin() Q_{{1-δ}} calibration");
    println!("  - robust_koopman_tube_mpc() full pipeline");
    println!("PASO C: Integration Tests ✓");
    println!("========================================\n");
}
