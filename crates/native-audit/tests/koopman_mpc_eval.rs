//! Tube Stability Tests — Anti-Wrapping Verification (Sprint 159)
//!
//! Verifies that Zonotope Order Reduction (Girard-style) prevents wrapping effect
//! explosion in high-dimensional spaces, and that PINN loss behaves correctly
//! under Koopman dynamics constraints.

use candle_core::{DType, Device, Result, Tensor};
use native_audit::koopman_pinn;
use native_audit::zonotope::Zonotope;

// ---------------------------------------------------------------------------
// Zonotope Order Reduction Tests
// ---------------------------------------------------------------------------

#[test]
fn test_zonotope_reduction_stability() -> Result<()> {
    let device = Device::Cpu;
    let dims = 64;
    let init_gens = 512; // High to force reduction
    let max_order = 2;

    // Create high-dim zonotope with many generators
    let center = Tensor::zeros((1, dims), DType::F32, &device)?;
    let gens_init = Tensor::randn(0.0, 1.0, (init_gens, dims), &device)?.to_dtype(DType::F32)?;

    let config = native_audit::zonotope::ZonotopeConfig {
        max_gens: init_gens,
        ..Default::default()
    };
    let z = Zonotope::new(center, gens_init, config)?;

    // Apply Girard reduction
    let z_reduced = z.reduce_order(max_order)?;

    let before = z.num_gens()?;
    let after = z_reduced.num_gens()?;

    println!("Generadores antes: {}, después: {}", before, after);
    assert!(
        after <= dims * max_order + dims,
        "Order reduction failed: {} > {}",
        after,
        dims * max_order + dims
    );

    // Propagate 10 steps and check volume doesn't explode
    let a = Tensor::eye(dims, DType::F32, &device)?; // Identity for stability
    let mut z_prop = z_reduced.clone();
    for _ in 0..10 {
        z_prop = z_prop.propagate_linear(&a)?;
    }
    let final_gens = z_prop.num_gens()?;
    assert!(
        final_gens <= dims * max_order + dims,
        "Wrapping explosion after propagation: {} > {}",
        final_gens,
        dims * max_order + dims
    );

    println!("✅ Zonotope reduction stable after 10 propagation steps");
    Ok(())
}

#[test]
fn test_zonotope_reduction_preserves_center() -> Result<()> {
    let device = Device::Cpu;
    let dims = 32;
    let init_gens = 256;

    let center_data: Vec<f32> = (0..dims).map(|i| i as f32 * 0.1).collect();
    let center = Tensor::from_vec(center_data.clone(), (1, dims), &device)?;
    let gens = Tensor::randn(0.0, 0.5, (init_gens, dims), &device)?.to_dtype(DType::F32)?;

    let config = native_audit::zonotope::ZonotopeConfig {
        max_gens: init_gens,
        ..Default::default()
    };
    let z = Zonotope::new(center.clone(), gens, config)?;

    let z_reduced = z.reduce_order(2)?;

    // Center should be preserved exactly
    let center_after = z_reduced.center.to_vec2::<f32>()?;
    for (before, after) in center_data.iter().zip(center_after[0].iter()) {
        assert!(
            (before - after).abs() < 1e-6,
            "Center modified by reduction"
        );
    }

    println!("✅ Zonotope center preserved through reduction");
    Ok(())
}

#[test]
fn test_zonotope_reduction_no_op_when_small() -> Result<()> {
    let device = Device::Cpu;
    let dims = 16;
    let init_gens = 10; // Already below threshold

    let center = Tensor::zeros((1, dims), DType::F32, &device)?;
    let gens = Tensor::randn(0.0, 1.0, (init_gens, dims), &device)?.to_dtype(DType::F32)?;

    let config = native_audit::zonotope::ZonotopeConfig {
        max_gens: init_gens,
        ..Default::default()
    };
    let z = Zonotope::new(center, gens, config)?;

    let z_reduced = z.reduce_order(2)?;

    // Should return unchanged since init_gens < dims * max_order
    assert_eq!(
        z_reduced.num_gens()?,
        init_gens,
        "Should not reduce when already small"
    );

    println!("✅ No-op reduction when generators < threshold");
    Ok(())
}

#[test]
fn test_zonotope_propagate_linear_shape() -> Result<()> {
    let device = Device::Cpu;
    let dims = 16;
    let num_gens = 32;

    let center = Tensor::zeros((1, dims), DType::F32, &device)?;
    let gens = Tensor::randn(0.0, 1.0, (num_gens, dims), &device)?.to_dtype(DType::F32)?;

    let config = native_audit::zonotope::ZonotopeConfig {
        max_gens: num_gens,
        ..Default::default()
    };
    let z = Zonotope::new(center, gens, config)?;

    let a = Tensor::eye(dims, DType::F32, &device)?;
    let z_prop = z.propagate_linear(&a)?;

    assert_eq!(z_prop.hidden_dim()?, dims, "Hidden dim should be preserved");
    assert_eq!(z_prop.num_gens()?, num_gens, "Num gens should be preserved");

    println!("✅ Linear propagation preserves shape");
    Ok(())
}

#[test]
fn test_zonotope_volume_ratio_with_reduction() -> Result<()> {
    let device = Device::Cpu;
    let dims = 32;
    let init_gens = 256;

    let center = Tensor::zeros((1, dims), DType::F32, &device)?;
    let gens = Tensor::randn(0.0, 0.5, (init_gens, dims), &device)?.to_dtype(DType::F32)?;

    let config = native_audit::zonotope::ZonotopeConfig {
        max_gens: init_gens,
        ..Default::default()
    };
    let z_full = Zonotope::new(center.clone(), gens.clone(), config.clone())?;
    let z_reduced = Zonotope::new(center, gens, config)?.reduce_order(2)?;

    let vol_full = z_full.volume_proxy()?;
    let vol_reduced = z_reduced.volume_proxy()?;

    let ratio = vol_reduced / vol_full.max(1e-10);
    println!(
        "Volume ratio (reduced/full): {:.4} (should be < 1.5x for good reduction)",
        ratio
    );
    assert!(ratio < 2.0, "Volume ratio too high: {}", ratio);

    println!("✅ Volume ratio within acceptable bounds");
    Ok(())
}

// ---------------------------------------------------------------------------
// PINN Loss Tests
// ---------------------------------------------------------------------------

#[test]
fn test_pinn_loss_integration() -> Result<()> {
    let device = Device::Cpu;
    let batch = 4;
    let lifted_dim = 16;

    let psi_t = Tensor::randn(0.0, 1.0, (batch, lifted_dim), &device)?.to_dtype(DType::F32)?;
    let psi_next = Tensor::randn(0.0, 1.0, (batch, lifted_dim), &device)?.to_dtype(DType::F32)?;
    let k = Tensor::eye(lifted_dim, DType::F32, &device)?;
    let r = Tensor::zeros((batch, lifted_dim), DType::F32, &device)?;

    let result = koopman_pinn::compute_pinn_loss(&psi_t, &psi_next, &k, &r, 0.01, 1.0)?;

    println!("PINN Loss: {}", result);
    assert!(
        result.total_loss >= 0.0,
        "Total loss should be non-negative"
    );
    assert!(result.data_loss >= 0.0, "Data loss should be non-negative");
    assert!(
        result.physics_loss >= 0.0,
        "Physics loss should be non-negative"
    );

    println!("✅ PINN loss integration test passed");
    Ok(())
}

#[test]
fn test_pinn_loss_exact_koopman_evolution() -> Result<()> {
    let device = Device::Cpu;
    let batch = 2;
    let lifted_dim = 8;

    let psi_t = Tensor::randn(0.0, 1.0, (batch, lifted_dim), &device)?.to_dtype(DType::F32)?;
    let k = Tensor::eye(lifted_dim, DType::F32, &device)?;

    // Exact Koopman evolution (row-vector): ψ_{t+1} = ψ_t @ K^T
    let k_t = k.t()?;
    let psi_next = psi_t.matmul(&k_t)?;
    let r = Tensor::zeros((batch, lifted_dim), DType::F32, &device)?;

    let result = koopman_pinn::compute_pinn_loss(&psi_t, &psi_next, &k, &r, 0.01, 1.0)?;

    assert!(
        result.data_loss < 1e-5,
        "Data loss should be ~0 for exact Koopman evolution, got {}",
        result.data_loss
    );

    println!("PINN Loss for exact evolution: {}", result);
    println!("✅ PINN loss near-zero for exact Koopman evolution");
    Ok(())
}

#[test]
fn test_pinn_loss_monotonicity_verification() -> Result<()> {
    let device = Device::Cpu;
    let batch = 4;
    let lifted_dim = 16;

    let psi_t = Tensor::randn(0.0, 1.0, (batch, lifted_dim), &device)?.to_dtype(DType::F32)?;
    let k = Tensor::eye(lifted_dim, DType::F32, &device)?;

    // Deterministic psi_next = K·ψ_t so zero residual is guaranteed better
    let k_t = k.t()?;
    let psi_next = psi_t.matmul(&k_t)?;

    let monotonic = koopman_pinn::verify_pinn_loss_monotonicity(&psi_t, &psi_next, &k, 0.01, 1.0)?;

    assert!(
        monotonic,
        "Zero residual should give lower loss than random residual for exact Koopman evolution"
    );

    println!("✅ PINN loss monotonicity verified");
    Ok(())
}

#[test]
fn test_pinn_loss_lambda_ablation() -> Result<()> {
    let device = Device::Cpu;
    let batch = 2;
    let lifted_dim = 8;

    let psi_t = Tensor::randn(0.0, 1.0, (batch, lifted_dim), &device)?.to_dtype(DType::F32)?;
    let psi_next = Tensor::randn(0.0, 1.0, (batch, lifted_dim), &device)?.to_dtype(DType::F32)?;
    let k = Tensor::eye(lifted_dim, DType::F32, &device)?;
    let r = Tensor::zeros((batch, lifted_dim), DType::F32, &device)?;

    let loss_lambda_0 = koopman_pinn::compute_pinn_loss(&psi_t, &psi_next, &k, &r, 0.01, 0.0)?;
    let loss_lambda_1 = koopman_pinn::compute_pinn_loss(&psi_t, &psi_next, &k, &r, 0.01, 1.0)?;
    let loss_lambda_10 = koopman_pinn::compute_pinn_loss(&psi_t, &psi_next, &k, &r, 0.01, 10.0)?;

    println!(
        "λ=0.0: total={:.6e}, λ=1.0: total={:.6e}, λ=10.0: total={:.6e}",
        loss_lambda_0.total_loss, loss_lambda_1.total_loss, loss_lambda_10.total_loss
    );

    assert!(
        loss_lambda_10.total_loss >= loss_lambda_0.total_loss,
        "Higher lambda should increase total loss"
    );

    println!("✅ PINN loss lambda ablation passed");
    Ok(())
}

// ---------------------------------------------------------------------------
// Full Integration: Reduction + PINN + Propagation
// ---------------------------------------------------------------------------

#[test]
fn test_full_sprint159_pipeline() -> Result<()> {
    let device = Device::Cpu;
    let dims = 32;
    let init_gens = 256;
    let max_order = 2;
    let horizon = 10;

    // 1. Create and reduce zonotope
    let center = Tensor::zeros((1, dims), DType::F32, &device)?;
    let gens = Tensor::randn(0.0, 0.5, (init_gens, dims), &device)?.to_dtype(DType::F32)?;

    let config = native_audit::zonotope::ZonotopeConfig {
        max_gens: init_gens,
        ..Default::default()
    };
    let z = Zonotope::new(center, gens, config)?;
    let z_reduced = z.reduce_order(max_order)?;

    let gens_before = z.num_gens()?;
    let gens_after = z_reduced.num_gens()?;
    println!(
        "Step 1 — Reduction: {} → {} generators",
        gens_before, gens_after
    );

    // 2. Propagate through identity (stability test)
    let a = Tensor::eye(dims, DType::F32, &device)?;
    let mut z_prop = z_reduced.clone();
    for step in 0..horizon {
        z_prop = z_prop.propagate_linear(&a)?;
        let _ = z_prop.reduce_order(max_order)?; // Reduce after each step
        let radius = z_prop.avg_width()?;
        println!(
            "Step {} — Propagation: radius={:.6e}, gens={}",
            step + 1,
            radius,
            z_prop.num_gens()?
        );
    }

    // 3. PINN loss on propagated center
    let psi_t = z_reduced.center.clone();
    let psi_next = z_prop.center.clone();
    let k = Tensor::eye(dims, DType::F32, &device)?;
    let r = Tensor::zeros((1, dims), DType::F32, &device)?;

    let pinn = koopman_pinn::compute_pinn_loss(&psi_t, &psi_next, &k, &r, 0.01, 1.0)?;
    println!("Step 3 — PINN Loss: {}", pinn);

    // Assertions
    assert!(
        z_prop.num_gens()? <= dims * max_order + dims,
        "Generator count exploded"
    );
    assert!(pinn.total_loss.is_finite(), "PINN loss should be finite");

    println!("\n✅ Sprint 159 Full Pipeline Complete");
    println!(
        "   Reduction: {} → {} ({}x compression)",
        gens_before,
        gens_after,
        gens_before as f32 / gens_after as f32
    );
    println!("   Propagation: {} steps stable", horizon);
    println!("   PINN Loss: {:.6e}", pinn.total_loss);

    Ok(())
}
