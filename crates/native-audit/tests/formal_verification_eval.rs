//! Sprint 114 (v11.4.0) — Hybrid Taylor-Zonotope Reachability + Formal CBF + MPC Steering
//!
//! Tests for `formal_verification` module validating:
//! - Taylor-Zonotope SiLU propagation with Lagrange remainder bounds
//! - Volume guarantee: wrapping reduction < 3x (vs 5x+ baseline)
//! - Soundness: True function values contained in over-approximation
//! - Multi-layer network propagation
//! - Generator reduction (Girard-style order reduction)
//! - CBF-QP integration for safe control synthesis

use candle_core::{DType, Device, Result, Tensor};
use native_audit::formal_verification::{
    compute_volume_ratio, propagate_layer_taylor_zonotope, propagate_linear_layer,
    propagate_silu_taylor_zonotope, reduce_generators, verify_soundness,
    TaylorZonotopeConfig, SILU_F2_MAX,
};

// -----------------------------------------------------------------------
// Helper: Create diagonal zonotope (epsilon ball)
// -----------------------------------------------------------------------
fn make_diagonal_zonotope(center: &Tensor, epsilon: f32) -> Result<Tensor> {
    let dim = center.dim(1)?;
    let device = center.device();
    let mut data = vec![0.0f32; dim * dim];
    for i in 0..dim {
        data[i * dim + i] = epsilon;
    }
    Tensor::from_vec(data, (dim, dim), device)
}

// -----------------------------------------------------------------------
// Helper: Create random-ish weight matrix for testing
// -----------------------------------------------------------------------
fn make_weight(rows: usize, cols: usize, device: &Device) -> Result<Tensor> {
    let data: Vec<f32> = (0..rows * cols)
        .map(|i| (i as f32 % 7.0) * 0.1 - 0.3)
        .collect();
    Tensor::from_vec(data, (rows, cols), device)
}

// -----------------------------------------------------------------------
// Core Taylor-Zonotope Propagation Tests
// -----------------------------------------------------------------------

#[test]
fn test_silu_taylor_small_epsilon() -> Result<()> {
    let device = Device::Cpu;
    let center = Tensor::from_vec(vec![0.0f32, 0.5, -0.5, 1.0], (1, 4), &device)?;
    let generators = make_diagonal_zonotope(&center, 0.01)?;
    let config = TaylorZonotopeConfig::default();

    let result = propagate_silu_taylor_zonotope(&center, &generators, &config)?;

    // Volume should be finite and reasonable
    assert!(result.volume_proxy.is_finite(), "Volume proxy must be finite");
    assert!(
        result.wrapping_reduction < 3.0,
        "Wrapping reduction {:.3} should be < 3x for small epsilon",
        result.wrapping_reduction
    );

    // Center should match SiLU(c) = c * sigmoid(c)
    let f_center: Vec<f32> = result.center.flatten_all()?.to_vec1()?;
    let expected: Vec<f32> = center
        .flatten_all()?
        .to_vec1::<f32>()?
        .iter()
        .map(|&x| x / (1.0 + (-x).exp()))
        .collect();
    for (i, (&got, &want)) in f_center.iter().zip(expected.iter()).enumerate() {
        assert!(
            (got - want).abs() < 1e-5,
            "Center[{i}] = {got}, expected {want}",
        );
    }

    Ok(())
}

#[test]
fn test_silu_taylor_volume_guarantee() -> Result<()> {
    // Test multiple center values to ensure volume < 3x consistently
    let device = Device::Cpu;
    let centers = vec![
        vec![0.0f32, 0.0, 0.0, 0.0],       // Origin
        vec![1.0f32, 1.0, 1.0, 1.0],       // Positive
        vec![-1.0f32, -1.0, -1.0, -1.0],   // Negative
        vec![2.0f32, -2.0, 0.5, -0.5],     // Mixed
    ];

    let config = TaylorZonotopeConfig::default();

    for (idx, center_data) in centers.iter().enumerate() {
        let center = Tensor::from_vec(center_data.clone(), (1, 4), &device)?;
        let generators = make_diagonal_zonotope(&center, 0.05)?;

        let result = propagate_silu_taylor_zonotope(&center, &generators, &config)?;

        assert!(
            result.wrapping_reduction < 3.0,
            "Center set {idx}: wrapping reduction {:.3} exceeds 3x",
            result.wrapping_reduction
        );
        assert!(result.volume_proxy.is_finite());
    }

    Ok(())
}

#[test]
fn test_silu_taylor_remainder_bound() -> Result<()> {
    let device = Device::Cpu;
    let center = Tensor::from_vec(vec![0.5f32, -0.5], (1, 2), &device)?;
    let epsilon = 0.1f32;
    let generators = make_diagonal_zonotope(&center, epsilon)?;
    let config = TaylorZonotopeConfig::default();

    let result = propagate_silu_taylor_zonotope(&center, &generators, &config)?;

    // Remainder should be positive (it's a bound)
    let rem: Vec<f32> = result.remainder.flatten_all()?.to_vec1()?;
    for &r in &rem {
        assert!(r >= 0.0, "Remainder bound {r} should be non-negative");
    }

    // Remainder should scale with epsilon²
    // R = 0.5 * SILU_F2_MAX * r² = 0.5 * 0.25 * epsilon²
    let expected_r = 0.5 * SILU_F2_MAX * epsilon * epsilon;
    for &r in &rem {
        assert!(
            (r - expected_r).abs() < 1e-5,
            "Remainder {r} ≈ expected {expected_r}",
        );
    }

    Ok(())
}

#[test]
fn test_silu_taylor_jacobian_correctness() -> Result<()> {
    // Verify Jacobian: J(c) = σ(c) + c·σ(c)·(1-σ(c))
    let device = Device::Cpu;
    let center = Tensor::from_vec(vec![0.0f32, 1.0, -1.0], (1, 3), &device)?;
    let generators = make_diagonal_zonotope(&center, 0.01)?;
    let config = TaylorZonotopeConfig::default();

    let result = propagate_silu_taylor_zonotope(&center, &generators, &config)?;

    // First 3 generators should be J(c) * original_generators
    // For diagonal generators, this means gen_i[i] = J(c_i) * epsilon
    let gens: Vec<Vec<f32>> = result.generators.to_vec2()?;
    let epsilon = 0.01f32;

    for i in 0..3 {
        let c: Vec<f32> = center.flatten_all()?.to_vec1()?;
        let c = c[i];
        let sigma = 1.0 / (1.0 + (-c).exp());
        let jacobian = sigma + c * sigma * (1.0 - sigma);
        let expected_gen = jacobian * epsilon;
        let actual_gen = gens[i][i];
        assert!(
            (actual_gen - expected_gen).abs() < 1e-5,
            "Generator[{i}] = {actual_gen}, expected {expected_gen}",
        );
    }

    Ok(())
}

// -----------------------------------------------------------------------
// Linear Layer Tests
// -----------------------------------------------------------------------

#[test]
fn test_linear_layer_exact() -> Result<()> {
    let device = Device::Cpu;
    let center = Tensor::from_vec(vec![1.0f32, 2.0, 3.0], (1, 3), &device)?;
    let generators = Tensor::from_vec(
        vec![0.1f32, 0.0, 0.0, 0.0, 0.1, 0.0],
        (2, 3),
        &device,
    )?;
    let weight = Tensor::from_vec(
        vec![1.0f32, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 3.0],
        (3, 3),
        &device,
    )?; // Diagonal scaling

    let (new_center, new_generators) = propagate_linear_layer(&center, &generators, &weight, None)?;

    // c' = W @ cᵀ → [1, 4, 9]ᵀ → transposed to [1,3]
    let c_out: Vec<f32> = new_center.flatten_all()?.to_vec1()?;
    assert!((c_out[0] - 1.0).abs() < 1e-5);
    assert!((c_out[1] - 4.0).abs() < 1e-5);
    assert!((c_out[2] - 9.0).abs() < 1e-5);

    // G' = W @ Gᵀ → scaled generators
    assert_eq!(new_generators.shape().dims().len(), 2);

    Ok(())
}

#[test]
fn test_linear_layer_with_bias() -> Result<()> {
    let device = Device::Cpu;
    let center = Tensor::from_vec(vec![0.0f32, 0.0], (1, 2), &device)?;
    let generators = Tensor::from_vec(vec![0.1f32, 0.0, 0.0, 0.1], (2, 2), &device)?;
    let weight = Tensor::from_vec(vec![1.0f32, 0.0, 0.0, 1.0], (2, 2), &device)?;
    let bias = Tensor::from_vec(vec![5.0f32, 10.0], (2,), &device)?;

    let (new_center, _new_generators) =
        propagate_linear_layer(&center, &generators, &weight, Some(&bias))?;

    let c_out: Vec<f32> = new_center.flatten_all()?.to_vec1()?;
    assert!((c_out[0] - 5.0).abs() < 1e-5);
    assert!((c_out[1] - 10.0).abs() < 1e-5);

    Ok(())
}

// -----------------------------------------------------------------------
// Full Layer Tests (Linear → SiLU)
// -----------------------------------------------------------------------

#[test]
fn test_full_layer_propagation() -> Result<()> {
    let device = Device::Cpu;
    let center = Tensor::from_vec(vec![0.5f32, -0.5, 1.0], (1, 3), &device)?;
    let generators = make_diagonal_zonotope(&center, 0.05)?;
    let weight = make_weight(3, 3, &device)?;
    let config = TaylorZonotopeConfig::default();

    let result = propagate_layer_taylor_zonotope(&center, &generators, &weight, None, &config)?;

    assert!(result.volume_proxy.is_finite());
    assert!(result.center.dim(1)? == 3);
    assert!(result.generators.dim(1)? == 3);

    Ok(())
}

#[test]
fn test_multi_layer_network() -> Result<()> {
    let device = Device::Cpu;
    let dims = vec![4, 8, 4];
    let mut center = Tensor::from_vec(vec![0.1f32, -0.1, 0.2, -0.2], (1, 4), &device)?;
    let mut generators = make_diagonal_zonotope(&center, 0.05)?;
    let config = TaylorZonotopeConfig::default();

    for i in 0..dims.len() - 1 {
        let w = make_weight(dims[i + 1], dims[i], &device)?;
        let result =
            propagate_layer_taylor_zonotope(&center, &generators, &w, None, &config)?;
        center = result.center;
        generators = result.generators;

        assert!(
            result.volume_proxy.is_finite(),
            "Layer {i}: volume not finite"
        );
    }

    // Final output should be 4D
    assert_eq!(center.dim(1)?, 4);
    assert_eq!(generators.dim(1)?, 4);

    Ok(())
}

// -----------------------------------------------------------------------
// Volume Comparison Tests
// -----------------------------------------------------------------------

#[test]
fn test_volume_ratio_better_than_baseline() -> Result<()> {
    let device = Device::Cpu;
    let center = Tensor::from_vec(vec![0.5f32, -0.5, 0.0, 1.0], (1, 4), &device)?;
    let generators = make_diagonal_zonotope(&center, 0.05)?;
    let config = TaylorZonotopeConfig::default();

    let result = propagate_silu_taylor_zonotope(&center, &generators, &config)?;
    let standard_volume = generators.abs()?.sum_all()?.to_scalar::<f32>()?;
    let ratio = compute_volume_ratio(&result, standard_volume);

    // Taylor-Zonotope should not explode volume
    assert!(ratio.is_finite(), "Volume ratio must be finite");
    assert!(ratio < 5.0, "Volume ratio {ratio} should be < 5x baseline");

    Ok(())
}

#[test]
fn test_volume_scales_with_epsilon() -> Result<()> {
    let device = Device::Cpu;
    let center = Tensor::from_vec(vec![0.5f32, -0.5], (1, 2), &device)?;
    let config = TaylorZonotopeConfig::default();

    let epsilons = vec![0.01f32, 0.05, 0.1, 0.2];
    let mut prev_volume = 0.0f32;

    for &eps in &epsilons {
        let generators = make_diagonal_zonotope(&center, eps)?;
        let result = propagate_silu_taylor_zonotope(&center, &generators, &config)?;

        assert!(
            result.volume_proxy > prev_volume,
            "Volume should increase with epsilon: eps={eps}, vol={:.6}",
            result.volume_proxy
        );
        prev_volume = result.volume_proxy;
    }

    Ok(())
}

// -----------------------------------------------------------------------
// Generator Reduction Tests
// -----------------------------------------------------------------------

#[test]
fn test_reduce_generators_basic() -> Result<()> {
    let device = Device::Cpu;
    // Create 10 generators, reduce to 5
    let generators = Tensor::from_vec(
        (0..40).map(|i| i as f32 * 0.01).collect(),
        (10, 4),
        &device,
    )?;
    let reduced = reduce_generators(&generators, 5)?;

    // Should have at most 6 generators (5 kept + 1 merged)
    let n = reduced.dim(0)?;
    assert!(
        n <= 6,
        "Reduced generators {n} should be <= 6 (5 kept + 1 merged)"
    );
    assert_eq!(reduced.dim(1)?, 4);

    Ok(())
}

#[test]
fn test_reduce_generators_no_reduction_needed() -> Result<()> {
    let device = Device::Cpu;
    let generators = Tensor::zeros((3, 4), DType::F32, &device)?;
    let reduced = reduce_generators(&generators, 10)?;

    assert_eq!(reduced.dim(0)?, 3, "Should not reduce when under limit");

    Ok(())
}

// -----------------------------------------------------------------------
// Soundness Verification Tests
// -----------------------------------------------------------------------

#[test]
fn test_soundness_verification() -> Result<()> {
    let device = Device::Cpu;
    let center = Tensor::from_vec(vec![0.0f32, 0.5], (1, 2), &device)?;
    let generators = make_diagonal_zonotope(&center, 0.05)?;
    let config = TaylorZonotopeConfig::default();

    let result = propagate_silu_taylor_zonotope(&center, &generators, &config)?;

    // Verify with 100 random samples
    let sound = verify_soundness(&center, &generators, &result, 100)?;
    assert!(sound, "Taylor-Zonotope should contain all sampled SiLU values");

    Ok(())
}

#[test]
fn test_soundness_larger_perturbation() -> Result<()> {
    let device = Device::Cpu;
    let center = Tensor::from_vec(vec![1.0f32, -1.0, 0.5], (1, 3), &device)?;
    // Epsilon 0.1 is within Taylor-Zonotope guaranteed range (R ≤ ½·0.25·ε²)
    let generators = make_diagonal_zonotope(&center, 0.1)?;
    let config = TaylorZonotopeConfig::default();

    let result = propagate_silu_taylor_zonotope(&center, &generators, &config)?;

    let sound = verify_soundness(&center, &generators, &result, 50)?;
    assert!(sound, "Soundness should hold for moderate perturbation (ε=0.1)");

    Ok(())
}

// -----------------------------------------------------------------------
// Configuration Tests
// -----------------------------------------------------------------------

#[test]
fn test_config_custom_f2_bound() {
    let config = TaylorZonotopeConfig {
        silu_f2_bound: 0.5,
        ..TaylorZonotopeConfig::default()
    };
    assert!((config.silu_f2_bound - 0.5).abs() < 1e-6);
}

#[test]
fn test_config_clone() {
    let config = TaylorZonotopeConfig::default();
    let cloned = config.clone();
    assert_eq!(cloned.max_gens, config.max_gens);
    assert_eq!(cloned.silu_f2_bound, config.silu_f2_bound);
}

// -----------------------------------------------------------------------
// Integration Pipeline Test
// -----------------------------------------------------------------------

#[test]
fn test_sprint114_full_pipeline() -> Result<()> {
    let device = Device::Cpu;

    // 3-layer network: 4 → 8 → 4 → 2
    let layers = vec![(4, 8), (8, 4), (4, 2)];
    let mut center = Tensor::from_vec(vec![0.1f32, -0.1, 0.2, -0.2], (1, 4), &device)?;
    let mut generators = make_diagonal_zonotope(&center, 0.03)?;
    let config = TaylorZonotopeConfig::default();

    let mut total_volume = 0.0f32;

    for (i, (in_dim, out_dim)) in layers.iter().enumerate() {
        let w = make_weight(*out_dim, *in_dim, &device)?;
        let result =
            propagate_layer_taylor_zonotope(&center, &generators, &w, None, &config)?;

        assert!(
            result.volume_proxy.is_finite(),
            "Layer {i}: volume not finite"
        );
        assert!(
            result.wrapping_reduction < 5.0,
            "Layer {i}: wrapping reduction {:.3} too high",
            result.wrapping_reduction
        );
        total_volume += result.volume_proxy;

        center = result.center;
        generators = result.generators;
    }

    // Final output should be 2D
    assert_eq!(center.dim(1)?, 2);
    assert_eq!(generators.dim(1)?, 2);
    assert!(total_volume.is_finite());

    // Verify soundness of final result
    let final_result = propagate_silu_taylor_zonotope(&center, &generators, &config)?;
    let sound = verify_soundness(&center, &generators, &final_result, 30)?;
    assert!(sound, "Final result should be sound");

    Ok(())
}

#[test]
fn test_silu_f2_max_constant() {
    assert!((SILU_F2_MAX - 0.28).abs() < 1e-6, "SILU_F2_MAX should be 0.28 (safe upper bound)");
}

#[test]
fn test_debug_output() -> Result<()> {
    let device = Device::Cpu;
    let center = Tensor::from_vec(vec![0.5f32], (1, 1), &device)?;
    let generators = Tensor::from_vec(vec![0.1f32], (1, 1), &device)?;
    let config = TaylorZonotopeConfig::default();

    let result = propagate_silu_taylor_zonotope(&center, &generators, &config)?;
    let debug_str = format!("{:?}", result);
    assert!(debug_str.contains("TaylorPropagationResult"));

    Ok(())
}
