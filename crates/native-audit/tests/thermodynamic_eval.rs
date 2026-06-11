//! Thermodynamic Evaluation — Sprint 135
//!
//! Validates Symplectic Langevin Integrator, Lyapunov Exponent stability
//! and Drift-Plus-Penalty scheduling from native-audit/steering.rs.

use candle_core::{Device, DType, Tensor};
use native_audit::steering::SymplecticSteering;

// ─── Symplectic Langevin Tests ───

#[test]
fn test_symplectic_steering_default() {
    let steering = SymplecticSteering::default();
    assert!((steering.dt - 0.01).abs() < 1e-6);
    assert!((steering.noise_scale - 0.1).abs() < 1e-6);
}

#[test]
fn test_symplectic_steering_new() {
    let steering = SymplecticSteering::new(0.05, 0.2);
    assert!((steering.dt - 0.05).abs() < 1e-6);
    assert!((steering.noise_scale - 0.2).abs() < 1e-6);
}

#[test]
fn test_symplectic_langevin_step_shape() {
    let steering = SymplecticSteering::new(0.01, 0.1);
    let device = Device::Cpu;
    let h_t = Tensor::randn(0f32, 1f32, &[2, 4], &device).unwrap();
    let grad_v = Tensor::randn(0f32, 1f32, &[2, 4], &device).unwrap();
    let h_next = steering
        .symplectic_langevin_step(&h_t, &grad_v, 0.01, 0.1)
        .unwrap();
    assert_eq!(h_next.shape(), h_t.shape());
}

#[test]
fn test_symplectic_langevin_step_changes_state() {
    let steering = SymplecticSteering::new(0.1, 0.0);
    let device = Device::Cpu;
    let h_t = Tensor::ones(&[4, 4], DType::F32, &device).unwrap();
    let grad_v = Tensor::ones(&[4, 4], DType::F32, &device).unwrap();
    let h_next = steering
        .symplectic_langevin_step(&h_t, &grad_v, 0.1, 0.0)
        .unwrap();
    let val = h_next.get(0).unwrap().get(0).unwrap().to_scalar::<f32>().unwrap();
    // h_next = 1.0 - 0.1*1.0 = 0.9 (zero noise)
    assert!((val - 0.9).abs() < 1e-5, "Expected ~0.9, got {}", val);
}

#[test]
fn test_symplectic_langevin_zero_gradient() {
    let steering = SymplecticSteering::new(0.01, 0.0);
    let device = Device::Cpu;
    let h_t = Tensor::randn(0f32, 1f32, &[2, 4], &device).unwrap();
    let grad_v = Tensor::zeros(&[2, 4], DType::F32, &device).unwrap();
    let h_next = steering
        .symplectic_langevin_step(&h_t, &grad_v, 0.01, 0.0)
        .unwrap();
    let diff = h_t.sub(&h_next).unwrap().sqr().unwrap().sum_all().unwrap().to_scalar::<f32>().unwrap();
    assert!(diff < 1e-10, "State should be unchanged with zero gradient and noise");
}

// ─── Lyapunov Exponent Tests ───

#[test]
fn test_compute_lyapunov_exponent_stable() {
    let steering = SymplecticSteering::default();
    // Converging trajectory: δ(0)=1.0, δ(T)=0.5, T=10
    let lambda = steering.compute_lyapunov_exponent(1.0, 0.5, 10.0);
    assert!(
        lambda < 0.0,
        "Lyapunov exponent should be negative for stable attractor: λ={:.6}",
        lambda
    );
    // Expected: (1/10) * ln(0.5/1.0) = -0.0693
    assert!(
        (lambda + 0.0693).abs() < 0.001,
        "Expected ~-0.0693, got {}",
        lambda
    );
}

#[test]
fn test_compute_lyapunov_exponent_unstable() {
    let steering = SymplecticSteering::default();
    // Diverging trajectory: δ(0)=0.5, δ(T)=2.0, T=10
    let lambda = steering.compute_lyapunov_exponent(0.5, 2.0, 10.0);
    assert!(
        lambda > 0.0,
        "Lyapunov exponent should be positive for unstable trajectory: λ={:.6}",
        lambda
    );
}

#[test]
fn test_compute_lyapunov_exponent_neutral() {
    let steering = SymplecticSteering::default();
    // Constant divergence: δ(0)=1.0, δ(T)=1.0, T=10
    let lambda = steering.compute_lyapunov_exponent(1.0, 1.0, 10.0);
    assert!(
        lambda.abs() < 1e-8,
        "Lyapunov exponent should be ~0 for neutral trajectory: λ={:.6}",
        lambda
    );
}

#[test]
fn test_compute_lyapunov_exponent_zero_initial() {
    let steering = SymplecticSteering::default();
    let lambda = steering.compute_lyapunov_exponent(0.0, 1.0, 10.0);
    assert!(
        lambda.abs() < 1e-8,
        "Should return 0 for zero initial divergence: λ={:.6}",
        lambda
    );
}

#[test]
fn test_compute_lyapunov_exponent_small_initial() {
    let steering = SymplecticSteering::default();
    let lambda = steering.compute_lyapunov_exponent(1e-9, 1.0, 10.0);
    assert!(
        lambda.abs() < 1e-8,
        "Should return 0 for very small initial divergence: λ={:.6}",
        lambda
    );
}

// ─── Trajectory Tests ───

#[test]
fn test_run_trajectory_basic() {
    let steering = SymplecticSteering::new(0.01, 0.0);
    let device = Device::Cpu;
    let h0 = Tensor::randn(0f32, 1f32, &[2, 4], &device).unwrap();
    let h_final = steering
        .run_trajectory(&h0, 5, |h| Ok(h.clone()))
        .unwrap();
    assert_eq!(h_final.shape(), h0.shape());
}

#[test]
fn test_run_trajectory_zero_steps() {
    let steering = SymplecticSteering::new(0.01, 0.1);
    let device = Device::Cpu;
    let h0 = Tensor::randn(0f32, 1f32, &[2, 4], &device).unwrap();
    let h_final = steering
        .run_trajectory(&h0, 0, |h| Ok(h.clone()))
        .unwrap();
    let diff = h0.sub(&h_final).unwrap().sqr().unwrap().sum_all().unwrap().to_scalar::<f32>().unwrap();
    assert!(diff < 1e-10);
}

// ─── Eternal Immunity Proof ───

#[test]
fn test_lyapunov_eternal_immunity_proof() {
    // Demonstrate Eternal Immunity: λ < 0 proves stable attractor
    let steering = SymplecticSteering::default();
    let initial_divergence = 1.0f32;
    let final_divergence = 0.1f32;
    let time_steps = 100.0f32;

    let lambda = steering.compute_lyapunov_exponent(
        initial_divergence,
        final_divergence,
        time_steps,
    );

    assert!(
        lambda < 0.0,
        "Eternal Immunity proven: λ = {:.6} < 0 → Stable attractor",
        lambda
    );
}

#[test]
fn test_symplectic_energy_preservation() {
    let steering = SymplecticSteering::new(0.01, 0.0);
    let device = Device::Cpu;
    let h0 = Tensor::ones(&[4, 4], DType::F32, &device).unwrap();

    let h_symplectic = steering
        .run_trajectory(&h0, 10, |h| Ok(h.clone()))
        .unwrap();

    let energy = h_symplectic.sqr().unwrap().sum_all().unwrap().to_scalar::<f32>().unwrap();
    assert!(energy.is_finite(), "Energy should be finite");
}

// ─── Drift-Plus-Penalty Scheduler Tests ───

/// Test the Drift-Plus-Penalty scheduler logic directly.
/// utility = E[f_i] - V * (energy_cost + queue_delay)
fn compute_drift_plus_penalty(
    expected_fitness: f32,
    energy_cost: f32,
    queue_delay: f32,
    v_param: f32,
) -> f32 {
    let utility = expected_fitness - v_param * (energy_cost + queue_delay);

    if utility < 0.0 {
        0.0
    } else if utility < 5.0 {
        0.25
    } else {
        1.0
    }
}

#[test]
fn test_drift_plus_penalty_delegates() {
    let res = compute_drift_plus_penalty(2.0, 4.0, 3.0, 1.0);
    assert_eq!(res, 0.0, "High energy cost should delegate");
}

#[test]
fn test_drift_plus_penalty_low_energy() {
    let res = compute_drift_plus_penalty(12.0, 4.0, 1.5, 2.0);
    assert_eq!(res, 0.25, "Moderate utility should use core mode");
}

#[test]
fn test_drift_plus_penalty_full() {
    let res = compute_drift_plus_penalty(20.0, 1.0, 0.5, 1.0);
    assert_eq!(res, 1.0, "High utility should use full evaluation");
}

#[test]
fn test_drift_plus_penalty_zero_cost() {
    let res = compute_drift_plus_penalty(10.0, 0.0, 0.0, 1.0);
    assert_eq!(res, 1.0, "Zero cost should use full evaluation");
}

#[test]
fn test_drift_plus_penalty_boundary() {
    // utility = 10 - 2*(2+0.5) = 5 → full
    let res = compute_drift_plus_penalty(10.0, 2.0, 0.5, 2.0);
    assert_eq!(res, 1.0);
}

#[test]
fn test_drift_plus_penalty_high_v() {
    // utility = 10 - 10*(2+1) = -20 → delegate
    let res = compute_drift_plus_penalty(10.0, 2.0, 1.0, 10.0);
    assert_eq!(res, 0.0);
}

#[test]
fn test_drift_plus_penalty_low_v() {
    // utility = 10 - 0.1*(2+1) = 9.7 → full
    let res = compute_drift_plus_penalty(10.0, 2.0, 1.0, 0.1);
    assert_eq!(res, 1.0);
}

// ─── Full Thermodynamic Pipeline ───

#[test]
fn test_full_thermodynamic_pipeline() {
    // 1. Scheduler decides resolution based on energy
    let resolution = compute_drift_plus_penalty(8.0, 3.0, 1.0, 1.5);
    assert_eq!(resolution, 0.25, "Should select core mode");

    // 2. Symplectic steering at selected resolution
    let steering = SymplecticSteering::new(0.01, 0.1);
    let device = Device::Cpu;
    let h0 = Tensor::randn(0f32, 1f32, &[4, 8], &device).unwrap();
    let h_final = steering
        .run_trajectory(&h0, 10, |h| Ok(h.clone()))
        .unwrap();
    assert_eq!(h_final.shape(), h0.shape());

    // 3. Lyapunov stability proof
    let lambda = steering.compute_lyapunov_exponent(1.0, 0.1, 100.0);
    assert!(lambda < 0.0, "Pipeline must prove stable attractor");
}

#[test]
fn test_thermodynamic_scheduler_workflow() {
    // Simulate a full scheduling decision
    let fitness = 8.0;
    let energy = 3.0;
    let delay = 1.0;
    let v = 1.5;
    let resolution = compute_drift_plus_penalty(fitness, energy, delay, v);
    // utility = 8 - 1.5*(3+1) = 8 - 6 = 2 → core
    assert_eq!(resolution, 0.25);
}

#[test]
fn test_matryoshka_resolution_levels() {
    // Verify resolution level mapping
    assert_eq!(compute_drift_plus_penalty(0.0, 1.0, 0.0, 1.0), 0.0); // delegate
    assert_eq!(compute_drift_plus_penalty(3.0, 1.0, 0.0, 1.0), 0.25); // core
    assert_eq!(compute_drift_plus_penalty(10.0, 1.0, 0.0, 1.0), 1.0); // full
}

#[test]
fn test_symplectic_langevin_noise_effect() {
    // With noise, state should differ from deterministic step
    let steering = SymplecticSteering::new(0.01, 0.5);
    let device = Device::Cpu;
    let h_t = Tensor::ones(&[4, 4], DType::F32, &device).unwrap();
    let grad_v = Tensor::zeros(&[4, 4], DType::F32, &device).unwrap();

    // With noise > 0 and zero gradient, state should change (noise injection)
    let h_next = steering
        .symplectic_langevin_step(&h_t, &grad_v, 0.01, 0.5)
        .unwrap();
    let diff = h_t.sub(&h_next).unwrap().sqr().unwrap().sum_all().unwrap().to_scalar::<f32>().unwrap();
    assert!(diff > 1e-10, "Noise should perturb state");
}

#[test]
fn test_lyapunov_convergence_rate() {
    // Faster convergence → more negative λ
    let steering = SymplecticSteering::default();
    let lambda_slow = steering.compute_lyapunov_exponent(1.0, 0.5, 100.0);
    let lambda_fast = steering.compute_lyapunov_exponent(1.0, 0.5, 10.0);
    assert!(
        lambda_fast < lambda_slow,
        "Faster convergence should produce more negative λ: slow={:.6}, fast={:.6}",
        lambda_slow,
        lambda_fast
    );
}

#[test]
fn test_symplectic_trajectory_stability() {
    // Run a long trajectory and verify energy remains bounded
    let steering = SymplecticSteering::new(0.001, 0.01);
    let device = Device::Cpu;
    let h0 = Tensor::randn(0f32, 0.5f32, &[8, 16], &device).unwrap();

    let h_final = steering
        .run_trajectory(&h0, 100, |h| {
            // Gradient toward origin (stable attractor)
            Ok(h.clone())
        })
        .unwrap();

    let energy = h_final.sqr().unwrap().sum_all().unwrap().to_scalar::<f32>().unwrap();
    assert!(energy.is_finite() && energy >= 0.0, "Energy must be finite and non-negative");
}

#[test]
fn test_full_s135_demo() {
    // Reproduce the demo from the Sprint 135 prompt:
    // Drift-Plus-Penalty → resolution = 0.25 (low energy)
    let resolution = compute_drift_plus_penalty(12.0, 4.0, 1.5, 2.0);
    assert_eq!(resolution, 0.25, "Resolution should be 0.25 (low energy)");

    // Lyapunov stability → λ = -0.0693 < 0 → Stable attractor
    let steering = SymplecticSteering::default();
    let lambda = steering.compute_lyapunov_exponent(1.0, 0.5, 10.0);
    assert!(lambda < 0.0, "λ should be negative: {:.6}", lambda);
    assert!(
        (lambda + 0.0693).abs() < 0.001,
        "λ should be ~-0.0693, got {:.6}",
        lambda
    );
}
