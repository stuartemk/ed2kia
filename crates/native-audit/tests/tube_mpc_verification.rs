//! Formal Verification Tests for Tube MPC — Empirical Demonstration of Tube Contraction.
//!
//! These tests provide empirical verification of the mathematical guarantees:
//! - **Tube Contraction**: `e_{k+1} < e_k` under contractive `A_cl`
//! - **PINN Residual Loss**: `L = ||r_obs - r_pred||² + λ * ||dr/dt - N[r, ψ]||²`
//! - **Recursive Propagation**: `T_{k+1} = A_{cl}·T_k ⊕ R_k ⊕ α·W`

use candle_core::{Device, Tensor};
use native_audit::tube_mpc::{compute_pinn_residual_loss, propagate_tube_recursive, TubeMPCConfig};
use native_audit::zonotope::Zonotope;

/// Create a deterministic tensor filled with sequential values scaled by seed.
fn make_tensor(
    rows: usize,
    cols: usize,
    seed: f32,
    device: &Device,
) -> candle_core::Result<Tensor> {
    let mut data = vec![0.0f32; rows * cols];
    for (i, val) in data.iter_mut().enumerate() {
        *val = seed * (i as f32 + 1.0);
    }
    Tensor::from_vec(data, (rows, cols), device)
}

/// Formal verification: Demonstrate tube contraction under contractive A_cl.
///
/// **Theorem**: If `||A_cl|| < 1` and disturbances are bounded, then:
/// `e_{k+1} <= ρ·e_k + γ` with `ρ < 1`, implying exponential convergence.
#[test]
fn test_formal_tube_contraction_and_pinn() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let hidden_dim = 64;
    let num_generators = 10;

    // 1. Simulate initial zonotope T_k
    let center = make_tensor(1, hidden_dim, 1.0, &device)?;
    let generators = make_tensor(num_generators, hidden_dim, 0.01, &device)?;
    let zono_config = native_audit::zonotope::ZonotopeConfig::default();
    let _tube_k = Zonotope::new(center.clone(), generators.clone(), zono_config)?;

    // 2. Simulate contractive A_cl (diagonal 0.9 — spectral radius < 1)
    let a_cl = Tensor::eye(hidden_dim, candle_core::DType::F32, &device)?
        .broadcast_mul(&Tensor::full(0.9f32, (), &device)?)?;

    // 3. Simulate small residual and disturbance
    let residual_gens = Tensor::zeros((hidden_dim, hidden_dim), candle_core::DType::F32, &device)?;
    let dist_gens = Tensor::zeros((hidden_dim, hidden_dim), candle_core::DType::F32, &device)?;

    // 4. Execute propagation with contractive config
    let config = TubeMPCConfig {
        horizon: 10,
        rho: 0.92,
        gamma: 0.01,
        alpha: 0.5,
        max_gens: 64,
        verify_contraction: true,
        contraction_tolerance: 1e-4,
    };

    let result = propagate_tube_recursive(&a_cl, &center, &residual_gens, &dist_gens, &config)?;

    // 5. Verify e_next < e_k (tube contraction)
    assert!(
        result.final_error_bound < result.initial_error_bound,
        "Tube must contract: final {:.6} < initial {:.6}",
        result.final_error_bound,
        result.initial_error_bound,
    );

    // Verify all steps contracted
    assert!(
        result.all_contracted,
        "All steps must satisfy contractive constraint"
    );

    // Verify contraction ratio is below 1
    assert!(
        result.contraction_ratio < 1.0,
        "Contraction ratio {:.6} must be < 1.0",
        result.contraction_ratio,
    );

    // 6. Test PINN loss computation
    let r_obs = make_tensor(1, hidden_dim, 1.0, &device)?;
    let r_pred = make_tensor(1, hidden_dim, 0.9, &device)?;
    let dr_dt = make_tensor(1, hidden_dim, 0.1, &device)?;
    let n_physics = make_tensor(1, hidden_dim, 0.09, &device)?;

    let loss = compute_pinn_residual_loss(&r_obs, &r_pred, &dr_dt, &n_physics, 1.0)?;
    assert!(loss >= 0.0, "PINN loss must be non-negative: {:.6}", loss);
    assert!(loss.is_finite(), "PINN loss must be finite: {:.6}", loss);

    Ok(())
}

/// Verify that increasing contraction factor (lower rho) tightens the tube.
#[test]
fn test_stronger_contraction_tightens_tube() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 32;

    let center = Tensor::ones((1, dim), candle_core::DType::F32, &device)?;
    let residual_gens = Tensor::zeros((dim, dim), candle_core::DType::F32, &device)?;
    let dist_gens = Tensor::zeros((dim, dim), candle_core::DType::F32, &device)?;

    // Weak contraction (rho = 0.95)
    let a_weak = Tensor::eye(dim, candle_core::DType::F32, &device)?
        .broadcast_mul(&Tensor::full(0.95f32, (), &device)?)?;
    let config_weak = TubeMPCConfig {
        rho: 0.95,
        horizon: 10,
        ..Default::default()
    };
    let result_weak =
        propagate_tube_recursive(&a_weak, &center, &residual_gens, &dist_gens, &config_weak)?;

    // Strong contraction (rho = 0.85)
    let a_strong = Tensor::eye(dim, candle_core::DType::F32, &device)?
        .broadcast_mul(&Tensor::full(0.85f32, (), &device)?)?;
    let config_strong = TubeMPCConfig {
        rho: 0.85,
        horizon: 10,
        ..Default::default()
    };
    let result_strong = propagate_tube_recursive(
        &a_strong,
        &center,
        &residual_gens,
        &dist_gens,
        &config_strong,
    )?;

    assert!(
        result_strong.final_error_bound < result_weak.final_error_bound,
        "Stronger contraction must produce tighter tube: strong {:.6} < weak {:.6}",
        result_strong.final_error_bound,
        result_weak.final_error_bound,
    );

    Ok(())
}

/// Verify that disturbances increase tube radius.
#[test]
fn test_disturbance_increases_tube_radius() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 32;

    let center = Tensor::ones((1, dim), candle_core::DType::F32, &device)?;
    let residual_gens = Tensor::zeros((dim, dim), candle_core::DType::F32, &device)?;
    let a_cl = Tensor::eye(dim, candle_core::DType::F32, &device)?.broadcast_mul(&Tensor::full(
        0.9f32,
        (),
        &device,
    )?)?;

    let config = TubeMPCConfig::default();

    // No disturbance
    let dist_zero = Tensor::zeros((dim, dim), candle_core::DType::F32, &device)?;
    let result_clean =
        propagate_tube_recursive(&a_cl, &center, &residual_gens, &dist_zero, &config)?;

    // Large disturbance
    let dist_large = Tensor::full(0.1f32, (dim, dim), &device)?;
    let result_noisy =
        propagate_tube_recursive(&a_cl, &center, &residual_gens, &dist_large, &config)?;

    assert!(
        result_noisy.final_error_bound >= result_clean.final_error_bound,
        "Disturbance must increase or maintain tube radius: noisy {:.6} >= clean {:.6}",
        result_noisy.final_error_bound,
        result_clean.final_error_bound,
    );

    Ok(())
}

/// Verify PINN loss decreases as prediction improves.
#[test]
fn test_pinn_loss_decreases_with_accuracy() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let r_obs = Tensor::new(&[1.0f32, 2.0, 3.0, 4.0], &device)?;
    let dr_dt = Tensor::new(&[0.1f32, 0.2, 0.3, 0.4], &device)?;
    let n_physics = dr_dt.clone();

    // Poor prediction
    let r_pred_poor = Tensor::new(&[0.0f32, 0.0, 0.0, 0.0], &device)?;
    let loss_poor = compute_pinn_residual_loss(&r_obs, &r_pred_poor, &dr_dt, &n_physics, 1.0)?;

    // Good prediction (close to observation)
    let r_pred_good = Tensor::new(&[0.9f32, 1.9, 2.9, 3.9], &device)?;
    let loss_good = compute_pinn_residual_loss(&r_obs, &r_pred_good, &dr_dt, &n_physics, 1.0)?;

    // Perfect prediction
    let loss_perfect = compute_pinn_residual_loss(&r_obs, &r_obs, &dr_dt, &n_physics, 1.0)?;

    assert!(
        loss_good < loss_poor,
        "Better prediction must reduce loss: good {:.6} < poor {:.6}",
        loss_good,
        loss_poor,
    );

    assert!(
        loss_perfect < loss_good,
        "Perfect prediction must have lowest loss: perfect {:.6} < good {:.6}",
        loss_perfect,
        loss_good,
    );

    Ok(())
}

/// Verify the full recursive propagation pipeline end-to-end.
#[test]
fn test_full_verification_pipeline() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 16;
    let horizon = 20;

    // Contractive system with small disturbances
    let a_cl = Tensor::eye(dim, candle_core::DType::F32, &device)?.broadcast_mul(&Tensor::full(
        0.88f32,
        (),
        &device,
    )?)?;
    let center = make_tensor(1, dim, 0.5, &device)?;
    let residual_gens = Tensor::full(0.001f32, (dim, dim), &device)?;
    let dist_gens = Tensor::full(0.001f32, (dim, dim), &device)?;

    let config = TubeMPCConfig {
        horizon,
        rho: 0.9,
        gamma: 0.005,
        alpha: 0.3,
        max_gens: 64,
        verify_contraction: true,
        contraction_tolerance: 1e-4,
    };

    let result = propagate_tube_recursive(&a_cl, &center, &residual_gens, &dist_gens, &config)?;

    // Verify pipeline integrity
    assert_eq!(result.num_steps, horizon);
    assert_eq!(result.steps.len(), horizon);
    assert!(result.all_contracted, "All steps must contract");
    assert!(result.contraction_ratio < 1.0);
    assert!(result.final_error_bound.is_finite());
    assert!(result.initial_error_bound.is_finite());

    // Verify monotonic decrease (with tolerance for additive gamma)
    for i in 1..result.steps.len() {
        let prev = result.steps[i - 1].error_bound;
        let curr = result.steps[i].error_bound;
        // Allow small increase due to gamma
        assert!(
            curr < prev + config.gamma + config.contraction_tolerance,
            "Step {}: error {:.6} exceeds bound {:.6} + gamma {:.4}",
            i,
            curr,
            prev,
            config.gamma,
        );
    }

    Ok(())
}
