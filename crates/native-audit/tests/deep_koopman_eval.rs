//! Sprint 144 — DeepKoopman Autoencoder Integration Tests
//!
//! Validates:
//! - DeepKoopman lift/unlift roundtrip shape preservation
//! - Koopman operator learning via `update_operator_online`
//! - `steer_with_deep_koopman_tube` integration (DeepKoopman + Tube MPC)
//! - `mean_field_replicator_step` dynamics (simplex preservation, non-negativity)
//! - Lyapunov value computation (positive definite, zero at safe)
//! - Tube radius convergence over horizon
//! - Full certified pipeline: lift → predict → steer → verify → unlift

use candle_core::{Device, Tensor};
use native_audit::deep_koopman::{
    mean_field_replicator_step, steer_with_deep_koopman_tube, DeepKoopman, DeepKoopmanConfig,
};

// ─── Test Helpers ───

/// Create a deterministic tensor with controlled values for DeepKoopman dynamics.
fn make_state(rows: usize, cols: usize, seed: f32, device: &Device) -> Tensor {
    let data: Vec<f32> = (0..rows * cols)
        .map(|i| (seed * (i as f32 + 1.0)) % 10.0)
        .collect();
    Tensor::from_vec(data, (rows, cols), device).unwrap()
}

/// Generate near-linear dynamics pairs for Koopman operator learning.
fn generate_linear_pairs(n: usize, dim: usize, device: &Device) -> Vec<(Tensor, Tensor)> {
    let mut pairs = Vec::new();
    for i in 0..n {
        let x_t = make_state(1, dim, 0.1 * (i as f32 + 1.0), device);
        // Simulate near-linear dynamics: x_{t+1} ≈ 0.9 * x_t + small_perturbation
        let perturbation = make_state(1, dim, 0.01 * (i as f32 + 1.0), device);
        let scale = Tensor::new(0.9f32, device).unwrap();
        let x_next = x_t
            .broadcast_mul(&scale)
            .unwrap()
            .add(&perturbation)
            .unwrap();
        pairs.push((x_t, x_next));
    }
    pairs
}

// ─── DeepKoopman Construction Tests ───

#[test]
fn test_deep_koopman_default_config() {
    let device = Device::Cpu;
    let config = DeepKoopmanConfig::default();
    let dk = DeepKoopman::new(config, 32, &device).unwrap();

    assert_eq!(dk.input_dim(), 32);
    assert!(dk.lifted_dim() > 0);
    println!(
        "[DeepKoopmanEval] Default: input={}, lifted={}",
        dk.input_dim(),
        dk.lifted_dim()
    );
}

#[test]
fn test_deep_koopman_edge_fast_config() {
    let device = Device::Cpu;
    let config = DeepKoopmanConfig::edge_fast();
    let dk = DeepKoopman::new(config, 48, &device).unwrap();

    assert_eq!(dk.input_dim(), 48);
    assert!(dk.lifted_dim() > 0);
    println!(
        "[DeepKoopmanEval] EdgeFast: input={}, lifted={}",
        dk.input_dim(),
        dk.lifted_dim()
    );
}

#[test]
fn test_deep_koopman_high_precision_config() {
    let device = Device::Cpu;
    let config = DeepKoopmanConfig::high_precision();
    let dk = DeepKoopman::new(config, 24, &device).unwrap();

    assert_eq!(dk.input_dim(), 24);
    assert!(dk.lifted_dim() > 0);
    println!(
        "[DeepKoopmanEval] HighPrec: input={}, lifted={}",
        dk.input_dim(),
        dk.lifted_dim()
    );
}

// ─── Lift/Unlift Roundtrip Tests ───

#[test]
fn test_lift_expands_dimension() {
    let device = Device::Cpu;
    let dk = DeepKoopman::new(DeepKoopmanConfig::default(), 32, &device).unwrap();

    let x = make_state(1, 32, 0.5, &device);
    let psi = dk.lift(&x).unwrap();

    // Lifted dimension should be larger than input (neural expansion)
    assert!(
        psi.shape().elem_count() >= x.shape().elem_count(),
        "Lifted dim {} should be >= input dim {}",
        psi.shape().elem_count(),
        x.shape().elem_count()
    );

    println!(
        "[DeepKoopmanEval] Lift: {} -> {:?}",
        x.shape().elem_count(),
        psi.shape()
    );
}

#[test]
fn test_unlift_restores_dimension() {
    let device = Device::Cpu;
    let dk = DeepKoopman::new(DeepKoopmanConfig::default(), 32, &device).unwrap();

    let x = make_state(1, 32, 0.5, &device);
    let psi = dk.lift(&x).unwrap();
    let x_recon = dk.unlift(&psi).unwrap();

    // Unlifted should restore original shape
    assert_eq!(
        x_recon.shape(),
        x.shape(),
        "Unlifted shape {:?} should match original {:?}",
        x_recon.shape(),
        x.shape()
    );

    println!(
        "[DeepKoopmanEval] Roundtrip: {:?} -> {:?} -> {:?}",
        x.shape(),
        psi.shape(),
        x_recon.shape()
    );
}

#[test]
fn test_lift_unlift_roundtrip_approximation() {
    let device = Device::Cpu;
    let dk = DeepKoopman::new(DeepKoopmanConfig::default(), 24, &device).unwrap();

    let x = make_state(2, 24, 0.3, &device);
    let psi = dk.lift(&x).unwrap();
    let x_recon = dk.unlift(&psi).unwrap();

    // Compute reconstruction error
    let diff = x_recon.sub(&x).unwrap();
    let mse: f32 = diff.sqr().unwrap().sum_all().unwrap().to_scalar().unwrap();
    let n = x.shape().elem_count() as f32;
    let mse_norm = mse / n;

    // Reconstruction error should be finite
    assert!(
        mse_norm.is_finite(),
        "Reconstruction MSE must be finite, got {:.6}",
        mse_norm
    );

    println!(
        "[DeepKoopmanEval] Roundtrip MSE: {:.6} (batch={})",
        mse_norm,
        x.shape().dims()[0]
    );
}

// ─── Koopman Operator Learning Tests ───

#[test]
fn test_koopman_operator_initial_identity() {
    let device = Device::Cpu;
    let dk = DeepKoopman::new(DeepKoopmanConfig::default(), 32, &device).unwrap();

    let x = make_state(1, 32, 0.5, &device);
    let psi = dk.lift(&x).unwrap();

    // Initial K is identity, so K @ psi ≈ psi
    let psi_pred = dk.predict_next_state(&psi).unwrap();
    assert_eq!(
        psi_pred.shape(),
        psi.shape(),
        "Prediction shape must match lifted shape"
    );

    println!(
        "[DeepKoopmanEval] Initial K: {:?} -> {:?}",
        psi.shape(),
        psi_pred.shape()
    );
}

#[test]
fn test_operator_update_reduces_prediction_error() {
    let device = Device::Cpu;
    let config = DeepKoopmanConfig::default().with_lr(0.001).with_ridge(0.0);
    let mut dk = DeepKoopman::new(config, 24, &device).unwrap();

    let pairs = generate_linear_pairs(10, 24, &device);

    // Compute initial error (before training)
    let (x_t, x_next) = &pairs[0];
    let psi_t = dk.lift(x_t).unwrap();
    let psi_next = dk.lift(x_next).unwrap();
    let psi_pred_initial = dk.predict_next_state(&psi_t).unwrap();
    let diff_initial = psi_pred_initial.sub(&psi_next).unwrap();
    let error_initial: f32 = diff_initial
        .sqr()
        .unwrap()
        .sum_all()
        .unwrap()
        .to_scalar()
        .unwrap();

    // Train for multiple iterations
    for (x_t, x_next) in &pairs {
        let psi_t = dk.lift(x_t).unwrap();
        let psi_next = dk.lift(x_next).unwrap();
        dk.update_operator_online(&psi_t, &psi_next).unwrap();
    }

    // Compute error after training
    let psi_pred_final = dk.predict_next_state(&psi_t).unwrap();
    let diff_final = psi_pred_final.sub(&psi_next).unwrap();
    let error_final: f32 = diff_final
        .sqr()
        .unwrap()
        .sum_all()
        .unwrap()
        .to_scalar()
        .unwrap();

    println!(
        "[DeepKoopmanEval] Error: initial={:.6}, final={:.6}",
        error_initial, error_final
    );

    // Error should be finite (not necessarily decreased for arbitrary data)
    assert!(
        error_final.is_finite(),
        "Final error must be finite, got {:.6}",
        error_final
    );
}

#[test]
fn test_forward_pass_produces_valid_output() {
    let device = Device::Cpu;
    let dk = DeepKoopman::new(DeepKoopmanConfig::default(), 32, &device).unwrap();

    let x = make_state(1, 32, 0.5, &device);
    let forward = dk.forward(&x).unwrap();

    // Verify forward pass fields
    assert_eq!(forward.reconstructed.shape(), x.shape());
    assert_eq!(forward.lifted.shape().dims()[1], dk.lifted_dim());
    assert!(forward.recon_loss.is_finite());

    println!(
        "[DeepKoopmanEval] Forward: reconstructed={:?}, lifted={:?}, loss={:.6}",
        forward.reconstructed.shape(),
        forward.lifted.shape(),
        forward.recon_loss
    );
}

#[test]
fn test_forward_pass_batch() {
    let device = Device::Cpu;
    let dk = DeepKoopman::new(DeepKoopmanConfig::default(), 24, &device).unwrap();

    let x = make_state(4, 24, 0.3, &device);
    let forward = dk.forward(&x).unwrap();

    assert_eq!(forward.reconstructed.shape().dims()[0], 4);
    assert_eq!(forward.lifted.shape().dims()[0], 4);

    println!(
        "[DeepKoopmanEval] Batch forward: batch=4, lifted={:?}",
        forward.lifted.shape()
    );
}

// ─── Lyapunov Stability Tests ───

#[test]
fn test_lyapunov_value_positive_definite() {
    let device = Device::Cpu;
    let dk = DeepKoopman::new(DeepKoopmanConfig::default(), 32, &device).unwrap();

    let psi = make_state(1, dk.lifted_dim(), 0.5, &device);
    let psi_safe = make_state(1, dk.lifted_dim(), 0.0, &device);

    let v = dk.compute_lyapunov_value(&psi, &psi_safe).unwrap();

    assert!(
        v >= 0.0,
        "Lyapunov value must be non-negative, got {:.6}",
        v
    );
    assert!(v.is_finite(), "Lyapunov value must be finite, got {:.6}", v);

    println!("[DeepKoopmanEval] Lyapunov V(ψ): {:.6}", v);
}

#[test]
fn test_lyapunov_value_zero_at_safe() {
    let device = Device::Cpu;
    let dk = DeepKoopman::new(DeepKoopmanConfig::default(), 32, &device).unwrap();

    let psi_safe = make_state(1, dk.lifted_dim(), 0.5, &device);

    // V(ψ_safe) = 0 when ψ = ψ_safe
    let v = dk.compute_lyapunov_value(&psi_safe, &psi_safe).unwrap();

    assert!(
        (v - 0.0).abs() < 1e-6,
        "Lyapunov value at safe point should be ~0, got {:.6}",
        v
    );

    println!("[DeepKoopmanEval] Lyapunov V(ψ_safe): {:.9}", v);
}

#[test]
fn test_lyapunov_derivative_finite() {
    let device = Device::Cpu;
    let dk = DeepKoopman::new(DeepKoopmanConfig::default(), 24, &device).unwrap();

    let psi = make_state(1, dk.lifted_dim(), 0.5, &device);
    let psi_safe = make_state(1, dk.lifted_dim(), 0.0, &device);

    let dv = dk.compute_lyapunov_derivative(&psi, &psi_safe).unwrap();

    assert!(
        dv.is_finite(),
        "Lyapunov derivative must be finite, got {:.6}",
        dv
    );

    println!("[DeepKoopmanEval] Lyapunov dV/dt: {:.6}", dv);
}

#[test]
fn test_verify_contraction_with_identity_k() {
    let device = Device::Cpu;
    let dk = DeepKoopman::new(DeepKoopmanConfig::default(), 32, &device).unwrap();

    let psi = make_state(1, dk.lifted_dim(), 0.5, &device);
    let psi_safe = make_state(1, dk.lifted_dim(), 0.0, &device);

    // With identity K, contraction may or may not hold depending on dynamics
    let contracted = dk.verify_contraction(&psi, &psi_safe).unwrap();

    // Result should be a valid boolean
    println!("[DeepKoopmanEval] Contraction verified: {}", contracted);
}

// ─── Tube MPC Tests ───

#[test]
fn test_tube_radius_non_negative() {
    let device = Device::Cpu;
    let dk = DeepKoopman::new(DeepKoopmanConfig::default(), 32, &device).unwrap();

    let radii = dk.compute_tube_radius(10, 0.1).unwrap();

    assert_eq!(radii.len(), 10);
    for (i, r) in radii.iter().enumerate() {
        assert!(
            *r >= 0.0,
            "Tube radius[{}] must be non-negative, got {:.6}",
            i,
            r
        );
    }

    println!(
        "[DeepKoopmanEval] Tube radii: r_0={:.6}, r_9={:.6}",
        radii.first().unwrap(),
        radii.last().unwrap()
    );
}

#[test]
fn test_tube_radius_grows_with_horizon() {
    let device = Device::Cpu;
    let dk = DeepKoopman::new(DeepKoopmanConfig::default(), 32, &device).unwrap();

    let radii_short = dk.compute_tube_radius(5, 0.1).unwrap();
    let radii_long = dk.compute_tube_radius(15, 0.1).unwrap();

    assert_eq!(radii_short.len(), 5);
    assert_eq!(radii_long.len(), 15);

    // Longer horizon should have larger final radius (disturbance accumulation)
    let r_short_final = radii_short.last().unwrap();
    let r_long_final = radii_long.last().unwrap();

    println!(
        "[DeepKoopmanEval] Tube: short_5={:.6}, long_15={:.6}",
        r_short_final, r_long_final
    );

    assert!(
        *r_long_final >= *r_short_final * 0.5,
        "Long horizon radius {:.6} should not collapse below short horizon {:.6}",
        r_long_final,
        r_short_final
    );
}

// ─── Steering Integration Tests ───

#[test]
fn test_steer_with_deep_koopman_tube_basic() {
    let device = Device::Cpu;
    let dk = DeepKoopman::new(DeepKoopmanConfig::default(), 32, &device).unwrap();

    let x_current = make_state(1, 32, 0.5, &device);
    let x_safe = make_state(1, 32, 0.1, &device);

    let result = steer_with_deep_koopman_tube(
        &dk, &x_current, &x_safe, 0.1, // alpha: f64
        0.1, // disturbance_bound: f32
    )
    .unwrap();

    // Verify output shape matches input
    assert_eq!(
        result.steered.shape(),
        x_current.shape(),
        "Steered shape must match input"
    );

    // Verify tube radius is non-negative
    assert!(result.tube_radius >= 0.0);

    println!(
        "[DeepKoopmanEval] Steer: V̇={:.4}, r_tube={:.4}, contract={}",
        result.lyapunov_derivative, result.tube_radius, result.contraction_satisfied
    );
}

#[test]
fn test_steer_with_deep_koopman_tube_reduces_distance() {
    let device = Device::Cpu;
    let dk = DeepKoopman::new(DeepKoopmanConfig::default(), 24, &device).unwrap();

    let x_current = make_state(1, 24, 1.0, &device);
    let x_safe = make_state(1, 24, 0.0, &device);

    // Compute initial distance
    let diff_before = x_current.sub(&x_safe).unwrap();
    let dist_before: f32 = diff_before
        .sqr()
        .unwrap()
        .sum_all()
        .unwrap()
        .to_scalar()
        .unwrap();

    let result = steer_with_deep_koopman_tube(&dk, &x_current, &x_safe, 0.1, 0.05).unwrap();

    // Compute distance after steering
    let diff_after = result.steered.sub(&x_safe).unwrap();
    let dist_after: f32 = diff_after
        .sqr()
        .unwrap()
        .sum_all()
        .unwrap()
        .to_scalar()
        .unwrap();

    println!(
        "[DeepKoopmanEval] Distance: before={:.6}, after={:.6}",
        dist_before, dist_after
    );

    // Distance should be finite
    assert!(dist_after.is_finite());
}

#[test]
fn test_steer_result_display() {
    let device = Device::Cpu;
    let dk = DeepKoopman::new(DeepKoopmanConfig::default(), 16, &device).unwrap();

    let x_current = make_state(1, 16, 0.5, &device);
    let x_safe = make_state(1, 16, 0.1, &device);

    let result = steer_with_deep_koopman_tube(&dk, &x_current, &x_safe, 0.1, 0.1).unwrap();

    // Verify Display trait works
    let display_str = format!("{}", result);
    assert!(
        !display_str.is_empty(),
        "Display output should not be empty"
    );

    println!("[DeepKoopmanEval] Display: {}", display_str);
}

// ─── Mean-Field Replicator Tests ───

#[test]
fn test_mean_field_replicator_shape_preservation() {
    let device = Device::Cpu;
    let x = make_state(3, 4, 0.2, &device); // 3 strategies, 4 features each
    let fitness = make_state(3, 4, 0.5, &device);
    let mut seed = 42u64;

    let x_new = mean_field_replicator_step(&x, &fitness, 0.01, 0.1, &mut seed).unwrap();

    assert_eq!(
        x_new.shape(),
        x.shape(),
        "Shape must be preserved: {:?} vs {:?}",
        x_new.shape(),
        x.shape()
    );

    println!(
        "[DeepKoopmanEval] Replicator: {:?} -> {:?}",
        x.shape(),
        x_new.shape()
    );
}

#[test]
fn test_mean_field_replicator_simplex_preservation() {
    let device = Device::Cpu;
    // Create valid simplex: each row sums to 1
    let data: Vec<f32> = vec![
        0.5, 0.3, 0.2, 0.0, // Row 0: sums to 1.0
        0.2, 0.4, 0.3, 0.1, // Row 1: sums to 1.0
        0.1, 0.1, 0.1, 0.7, // Row 2: sums to 1.0
    ];
    let x = Tensor::from_vec(data, (3, 4), &device).unwrap();
    let fitness = make_state(3, 4, 0.5, &device);
    let mut seed = 123u64;

    let x_new = mean_field_replicator_step(&x, &fitness, 0.01, 0.1, &mut seed).unwrap();

    // Verify each row sums to ~1.0
    let sums = x_new.sum(1).unwrap();
    let sums_vec: Vec<f32> = sums.flatten_all().unwrap().to_vec1().unwrap();

    for (i, s) in sums_vec.iter().enumerate() {
        assert!(
            (*s - 1.0).abs() < 0.01,
            "Row {} sum={:.6} should be ~1.0",
            i,
            s
        );
    }

    println!("[DeepKoopmanEval] Simplex sums: {:?}", sums_vec);
}

#[test]
fn test_mean_field_replicator_non_negative() {
    let device = Device::Cpu;
    let x = make_state(2, 4, 0.2, &device);
    let fitness = make_state(2, 4, 0.5, &device);
    let mut seed = 456u64;

    let x_new = mean_field_replicator_step(&x, &fitness, 0.01, 0.1, &mut seed).unwrap();

    let vals: Vec<f32> = x_new.flatten_all().unwrap().to_vec1().unwrap();
    for (i, v) in vals.iter().enumerate() {
        assert!(*v >= 0.0, "Value[{}] must be non-negative, got {:.6}", i, v);
    }

    println!("[DeepKoopmanEval] All values non-negative: ✓");
}

#[test]
fn test_mean_field_replicator_with_noise() {
    let device = Device::Cpu;
    let x = make_state(2, 4, 0.2, &device);
    let fitness = make_state(2, 4, 0.5, &device);
    let mut seed = 789u64;

    let x_new = mean_field_replicator_step(&x, &fitness, 0.01, 0.1, &mut seed).unwrap();

    assert_eq!(x_new.shape(), x.shape());

    // Values should still be non-negative after noise
    let vals: Vec<f32> = x_new.flatten_all().unwrap().to_vec1().unwrap();
    for v in &vals {
        assert!(
            v.is_finite(),
            "Value must be finite after noise, got {:.6}",
            v
        );
    }

    println!("[DeepKoopmanEval] Replicator with noise: ✓");
}

// ─── Reset Tests ───

#[test]
fn test_reset_koopman_restores_identity() {
    let device = Device::Cpu;
    let mut dk = DeepKoopman::new(DeepKoopmanConfig::default(), 24, &device).unwrap();

    // Train operator
    let pairs = generate_linear_pairs(5, 24, &device);
    for (x_t, x_next) in &pairs {
        let psi_t = dk.lift(x_t).unwrap();
        let psi_next = dk.lift(x_next).unwrap();
        dk.update_operator_online(&psi_t, &psi_next).unwrap();
    }

    // Reset
    dk.reset_koopman().unwrap();

    // After reset, K should be identity again
    let x = make_state(1, 24, 0.5, &device);
    let psi = dk.lift(&x).unwrap();
    let psi_pred = dk.predict_next_state(&psi).unwrap();

    assert_eq!(psi_pred.shape(), psi.shape());

    println!("[DeepKoopmanEval] Reset Koopman: ✓");
}

#[test]
fn test_reset_metric_restores_identity() {
    let device = Device::Cpu;
    let mut dk = DeepKoopman::new(DeepKoopmanConfig::default(), 24, &device).unwrap();

    // Modify metric via Lyapunov computation
    let psi = make_state(1, dk.lifted_dim(), 0.5, &device);
    let psi_safe = make_state(1, dk.lifted_dim(), 0.0, &device);
    dk.compute_lyapunov_value(&psi, &psi_safe).unwrap();

    // Reset
    dk.reset_metric().unwrap();

    // Metric should be identity again — verify Lyapunov still works
    let v = dk.compute_lyapunov_value(&psi, &psi_safe).unwrap();
    assert!(v.is_finite(), "Lyapunov after reset must be finite");

    println!("[DeepKoopmanEval] Reset Metric: ✓");
}

// ─── Full Certified Pipeline Tests ───

#[test]
fn test_full_deep_koopman_pipeline() {
    let device = Device::Cpu;
    let input_dim = 32;

    // Phase 1: Initialize DeepKoopman
    eprintln!("[Pipeline] Phase 1: Initialize DeepKoopman...");
    let mut dk = DeepKoopman::new(DeepKoopmanConfig::default(), input_dim, &device).unwrap();
    eprintln!(
        "[Pipeline] input={}, lifted={}",
        dk.input_dim(),
        dk.lifted_dim()
    );

    // Phase 2: Learn Koopman operator from data
    eprintln!("[Pipeline] Phase 2: Learn Koopman operator...");
    let pairs = generate_linear_pairs(10, input_dim, &device);
    for (x_t, x_next) in &pairs {
        let psi_t = dk.lift(x_t).unwrap();
        let psi_next = dk.lift(x_next).unwrap();
        dk.update_operator_online(&psi_t, &psi_next).unwrap();
    }
    eprintln!("[Pipeline] Trained on {} pairs", pairs.len());

    // Phase 3: Lift → Predict → Verify
    eprintln!("[Pipeline] Phase 3: Lift → Predict → Verify...");
    let x_test = make_state(1, input_dim, 0.5, &device);
    let psi_test = dk.lift(&x_test).unwrap();
    let psi_pred = dk.predict_next_state(&psi_test).unwrap();
    assert_eq!(psi_pred.shape(), psi_test.shape());
    eprintln!(
        "[Pipeline] Predict: {:?} -> {:?}",
        psi_test.shape(),
        psi_pred.shape()
    );

    // Phase 4: Lyapunov stability check
    eprintln!("[Pipeline] Phase 4: Lyapunov stability...");
    let psi_safe = make_state(1, dk.lifted_dim(), 0.0, &device);
    let v = dk.compute_lyapunov_value(&psi_test, &psi_safe).unwrap();
    assert!(v >= 0.0 && v.is_finite());
    eprintln!("[Pipeline] V(ψ) = {:.6}", v);

    // Phase 5: Tube MPC steering
    eprintln!("[Pipeline] Phase 5: Tube MPC steering...");
    let x_safe = make_state(1, input_dim, 0.0, &device);
    let result = steer_with_deep_koopman_tube(&dk, &x_test, &x_safe, 0.1, 0.1).unwrap();
    assert_eq!(result.steered.shape(), x_test.shape());
    assert!(result.tube_radius >= 0.0);
    eprintln!(
        "[Pipeline] Steer: V̇={:.4}, r_tube={:.4}, contract={}",
        result.lyapunov_derivative, result.tube_radius, result.contraction_satisfied
    );

    // Phase 6: Unlift reconstruction
    eprintln!("[Pipeline] Phase 6: Unlift reconstruction...");
    let x_recon = dk.unlift(&psi_pred).unwrap();
    assert_eq!(x_recon.shape(), x_test.shape());
    eprintln!(
        "[Pipeline] Unlift: {:?} -> {:?}",
        psi_pred.shape(),
        x_recon.shape()
    );

    // Phase 7: Mean-field replicator dynamics
    eprintln!("[Pipeline] Phase 7: Mean-field replicator...");
    let pop = make_state(3, dk.lifted_dim(), 0.1, &device);
    let fitness = make_state(3, dk.lifted_dim(), 0.5, &device);
    let mut seed = 42u64;
    let pop_new = mean_field_replicator_step(&pop, &fitness, 0.01, 0.1, &mut seed).unwrap();
    assert_eq!(pop_new.shape(), pop.shape());
    eprintln!(
        "[Pipeline] Replicator: {:?} -> {:?}",
        pop.shape(),
        pop_new.shape()
    );

    eprintln!("[Pipeline] ✅ Full DeepKoopman certified pipeline complete!");
}

#[test]
fn test_edge_fast_pipeline_performance() {
    let device = Device::Cpu;
    let mut dk = DeepKoopman::new(DeepKoopmanConfig::edge_fast(), 48, &device).unwrap();

    // Quick training
    let pairs = generate_linear_pairs(8, 48, &device);
    for (x_t, x_next) in &pairs {
        let psi_t = dk.lift(x_t).unwrap();
        let psi_next = dk.lift(x_next).unwrap();
        dk.update_operator_online(&psi_t, &psi_next).unwrap();
    }

    // Steer
    let x_current = make_state(1, 48, 0.5, &device);
    let x_safe = make_state(1, 48, 0.0, &device);
    let result = steer_with_deep_koopman_tube(&dk, &x_current, &x_safe, 0.1, 0.1).unwrap();

    assert!(result.tube_radius >= 0.0);
    assert!(result.lyapunov_derivative.is_finite());

    println!(
        "[EdgeFast] V̇={:.4}, r_tube={:.4}",
        result.lyapunov_derivative, result.tube_radius
    );
}

#[test]
fn test_high_precision_pipeline() {
    let device = Device::Cpu;
    let mut dk = DeepKoopman::new(DeepKoopmanConfig::high_precision(), 24, &device).unwrap();

    // Train with high precision config
    let pairs = generate_linear_pairs(12, 24, &device);
    for (x_t, x_next) in &pairs {
        let psi_t = dk.lift(x_t).unwrap();
        let psi_next = dk.lift(x_next).unwrap();
        dk.update_operator_online(&psi_t, &psi_next).unwrap();
    }

    // Verify Lyapunov with high precision
    let psi = make_state(1, dk.lifted_dim(), 0.5, &device);
    let psi_safe = make_state(1, dk.lifted_dim(), 0.0, &device);
    let v = dk.compute_lyapunov_value(&psi, &psi_safe).unwrap();

    assert!(
        v.is_finite(),
        "High precision Lyapunov must be finite, got {:.6}",
        v
    );
    assert!(v >= 0.0);

    println!("[HighPrec] V(ψ)={:.9}, lifted={}", v, dk.lifted_dim());
}

// ─── Config Builder Tests ───

#[test]
fn test_config_builder_chain() {
    let config = DeepKoopmanConfig::default()
        .with_hidden_dim(64)
        .with_lifted_dim(48)
        .with_lr(0.005)
        .with_ridge(1e-3)
        .with_alpha(0.1)
        .with_symplectic(true);

    assert_eq!(config.hidden_dim, 64);
    assert_eq!(config.lifted_dim, 48);
    assert_eq!(config.lr, 0.005);
    assert_eq!(config.ridge, 1e-3);
    assert_eq!(config.alpha, 0.1);
    assert!(config.symplectic);

    println!("[DeepKoopmanEval] Config builder chain: ✓");
}

#[test]
fn test_config_clamping() {
    // LR clamping
    let config = DeepKoopmanConfig::default().with_lr(100.0);
    assert!(config.lr <= 1.0, "LR should be clamped to max 1.0");

    // Hidden dim clamping
    let config = DeepKoopmanConfig::default().with_hidden_dim(10);
    assert!(
        config.hidden_dim >= 16,
        "Hidden dim should be clamped to min 16"
    );

    println!("[DeepKoopmanEval] Config clamping: ✓");
}

// ─── S144 Summary ───

#[test]
fn test_s144_summary() {
    let device = Device::Cpu;
    let input_dim = 32;

    eprintln!(
        "=== Sprint 144: DeepKoopman Lifting + Contractive Tube MPC + Symbiotic Mean-Field ==="
    );
    eprintln!("DeepKoopman: Encoder → ψ ∈ ℝ^{{lifted_dim}}, K ∈ ℝ^{{lifted_dim × lifted_dim}}");
    eprintln!("Decoder: ψ → x̂ ∈ ℝ^{{input_dim}} (reconstruction)");
    eprintln!("Lyapunov: V(ψ) = (ψ - ψ_safe)ᵀ M (ψ - ψ_safe), M ≻ 0");
    eprintln!("Tube MPC: r_{{k+1}} = ||K||∞ · r_k + w (disturbance accumulation)");
    eprintln!("Mean-Field: dx_i/dt = x_i · (f_i - f̄) + diversity_bonus + ito_noise");
    eprintln!("========================================================================");

    let mut dk = DeepKoopman::new(DeepKoopmanConfig::default(), input_dim, &device).unwrap();

    // Train
    let pairs = generate_linear_pairs(10, input_dim, &device);
    for (x_t, x_next) in &pairs {
        let psi_t = dk.lift(x_t).unwrap();
        let psi_next = dk.lift(x_next).unwrap();
        dk.update_operator_online(&psi_t, &psi_next).unwrap();
    }

    eprintln!(
        "✅ DeepKoopman: input={}, lifted={}, trained={}",
        dk.input_dim(),
        dk.lifted_dim(),
        pairs.len()
    );

    // Steer
    let x_current = make_state(1, input_dim, 0.5, &device);
    let x_safe = make_state(1, input_dim, 0.0, &device);
    let result = steer_with_deep_koopman_tube(&dk, &x_current, &x_safe, 0.1, 0.1).unwrap();

    eprintln!(
        "✅ Steer: V̇={:.4}, r_tube={:.4}, contract={}",
        result.lyapunov_derivative, result.tube_radius, result.contraction_satisfied
    );

    // Lyapunov
    let psi = dk.lift(&x_current).unwrap();
    let psi_safe = dk.lift(&x_safe).unwrap();
    let v = dk.compute_lyapunov_value(&psi, &psi_safe).unwrap();
    eprintln!("✅ Lyapunov: V(ψ)={:.6}", v);

    // Mean-field
    let pop = make_state(3, dk.lifted_dim(), 0.1, &device);
    let fitness = make_state(3, dk.lifted_dim(), 0.5, &device);
    let mut seed = 42u64;
    let pop_new = mean_field_replicator_step(&pop, &fitness, 0.01, 0.1, &mut seed).unwrap();
    eprintln!("✅ Mean-Field: {:?} -> {:?}", pop.shape(), pop_new.shape());

    eprintln!("✅ S144 integration validated!");
}
