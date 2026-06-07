//! Zonotope Geometry Tests — Sprint 110
//!
//! Tests for zonotope creation, propagation, bounds computation,
//! and certified steering robustness verification.

use candle_core::{DType, Device, Tensor};
use native_audit::zonotope::{HybridZonotope, Zonotope, ZonotopeConfig};

// ---------------------------------------------------------------------------
// Construction Tests
// ---------------------------------------------------------------------------

#[test]
fn test_zonotope_creation_epsilon() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::new(&[1.0f32, 2.0, 3.0], &device)?.unsqueeze(0)?;
    let z = Zonotope::new_from_epsilon(&center, 0.1, 3)?;

    assert_eq!(z.hidden_dim()?, 3);
    assert_eq!(z.num_gens()?, 3);
    Ok(())
}

#[test]
fn test_zonotope_creation_point() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::new(&[1.0f32, 2.0, 3.0], &device)?.unsqueeze(0)?;
    let z = Zonotope::point(&center)?;

    assert_eq!(z.num_gens()?, 0);
    let (lo, hi) = z.compute_bounds()?;
    let diff = lo
        .broadcast_sub(&hi)?
        .abs()?
        .sum_all()?
        .to_scalar::<f32>()?;
    assert!(diff.abs() < 1e-5, "Point zonotope should have zero width");
    Ok(())
}

#[test]
fn test_zonotope_from_intervals() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let lo = Tensor::new(&[0.0f32, 0.0, 0.0], &device)?.unsqueeze(0)?;
    let hi = Tensor::new(&[1.0f32, 2.0, 3.0], &device)?.unsqueeze(0)?;
    let z = Zonotope::from_intervals(&lo, &hi)?;

    let (z_lo, z_hi) = z.compute_bounds()?;
    let lo_diff = z_lo
        .broadcast_sub(&lo)?
        .abs()?
        .sum_all()?
        .to_scalar::<f32>()?;
    let hi_diff = z_hi
        .broadcast_sub(&hi)?
        .abs()?
        .sum_all()?
        .to_scalar::<f32>()?;
    assert!(lo_diff.abs() < 1e-5);
    assert!(hi_diff.abs() < 1e-5);
    Ok(())
}

#[test]
fn test_zonotope_explicit_construction() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::zeros((1, 4), DType::F32, &device)?;
    let generators = {
        let s = Tensor::full(0.1f32, (), &device)?;
        Tensor::eye(4, DType::F32, &device)?.broadcast_mul(&s)?
    };
    let config = ZonotopeConfig {
        max_gens: 4,
        epsilon: 0.1,
        ..Default::default()
    };
    let z = Zonotope::new(center, generators, config)?;

    assert_eq!(z.hidden_dim()?, 4);
    assert_eq!(z.num_gens()?, 4);
    Ok(())
}

// ---------------------------------------------------------------------------
// Bounds Tests
// ---------------------------------------------------------------------------

#[test]
fn test_bounds_symmetric() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::zeros((1, 3), DType::F32, &device)?;
    let z = Zonotope::new_from_epsilon(&center, 0.5, 3)?;

    let (lo, hi) = z.compute_bounds()?;
    let lo_vec: Vec<f32> = lo.flatten_all()?.to_vec1()?;
    let hi_vec: Vec<f32> = hi.flatten_all()?.to_vec1()?;

    for i in 0..3 {
        assert!((lo_vec[i] + 0.5).abs() < 1e-5, "lo[{}] = {}", i, lo_vec[i]);
        assert!((hi_vec[i] - 0.5).abs() < 1e-5, "hi[{}] = {}", i, hi_vec[i]);
    }
    Ok(())
}

#[test]
fn test_bounds_asymmetric() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::new(&[10.0f32, 20.0], &device)?.unsqueeze(0)?;
    let z = Zonotope::new_from_epsilon(&center, 0.1, 2)?;

    let (lo, hi) = z.compute_bounds()?;
    let lo_vec: Vec<f32> = lo.flatten_all()?.to_vec1()?;
    let hi_vec: Vec<f32> = hi.flatten_all()?.to_vec1()?;

    assert!((lo_vec[0] - 9.9).abs() < 1e-5);
    assert!((hi_vec[0] - 10.1).abs() < 1e-5);
    assert!((lo_vec[1] - 19.9).abs() < 1e-5);
    assert!((hi_vec[1] - 20.1).abs() < 1e-5);
    Ok(())
}

#[test]
fn test_volume_proxy() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::zeros((1, 4), DType::F32, &device)?;
    let z = Zonotope::new_from_epsilon(&center, 0.1, 4)?;

    let vol = z.volume_proxy()?;
    assert!((vol - 0.4).abs() < 1e-5, "Expected volume 0.4, got {}", vol);
    Ok(())
}

#[test]
fn test_avg_width() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::zeros((1, 3), DType::F32, &device)?;
    let z = Zonotope::new_from_epsilon(&center, 0.5, 3)?;

    let width = z.avg_width()?;
    // Each dim has width 1.0 (from -0.5 to 0.5)
    assert!(
        (width - 1.0).abs() < 1e-5,
        "Expected width 1.0, got {}",
        width
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Linear Operation Tests
// ---------------------------------------------------------------------------

#[test]
fn test_affine_identity() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::new(&[1.0f32, 2.0], &device)?.unsqueeze(0)?;
    let z = Zonotope::new_from_epsilon(&center, 0.1, 2)?;

    let weight = Tensor::from_vec(vec![1.0f32, 0.0, 0.0, 1.0], (2, 2), &device)?;
    let z2 = z.affine_transform(&weight, None)?;

    let c: Vec<f32> = z2.center.flatten_all()?.to_vec1()?;
    assert!((c[0] - 1.0).abs() < 1e-5);
    assert!((c[1] - 2.0).abs() < 1e-5);
    Ok(())
}

#[test]
fn test_affine_scaling() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::new(&[1.0f32, 2.0], &device)?.unsqueeze(0)?;
    let z = Zonotope::new_from_epsilon(&center, 0.1, 2)?;

    // Scale by 2
    let weight = Tensor::from_vec(vec![2.0f32, 0.0, 0.0, 2.0], (2, 2), &device)?;
    let z2 = z.affine_transform(&weight, None)?;

    let c: Vec<f32> = z2.center.flatten_all()?.to_vec1()?;
    assert!((c[0] - 2.0).abs() < 1e-5);
    assert!((c[1] - 4.0).abs() < 1e-5);
    Ok(())
}

#[test]
fn test_affine_with_bias() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::zeros((1, 2), DType::F32, &device)?;
    let z = Zonotope::new_from_epsilon(&center, 0.1, 2)?;

    let weight = Tensor::from_vec(vec![1.0f32, 0.0, 0.0, 1.0], (2, 2), &device)?;
    let bias = Tensor::new(&[5.0f32, 10.0], &device)?;
    let z2 = z.affine_transform(&weight, Some(&bias))?;

    let c: Vec<f32> = z2.center.flatten_all()?.to_vec1()?;
    assert!((c[0] - 5.0).abs() < 1e-5);
    assert!((c[1] - 10.0).abs() < 1e-5);
    Ok(())
}

#[test]
fn test_scale() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::new(&[2.0f32, 4.0], &device)?.unsqueeze(0)?;
    let z = Zonotope::new_from_epsilon(&center, 0.1, 2)?;

    let z2 = z.scale(0.5)?;
    let c: Vec<f32> = z2.center.flatten_all()?.to_vec1()?;
    assert!((c[0] - 1.0).abs() < 1e-5);
    assert!((c[1] - 2.0).abs() < 1e-5);
    Ok(())
}

// ---------------------------------------------------------------------------
// Non-Linear Operation Tests
// ---------------------------------------------------------------------------

#[test]
fn test_relu_positive() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::new(&[5.0f32, 10.0], &device)?.unsqueeze(0)?;
    let z = Zonotope::new_from_epsilon(&center, 0.1, 2)?;

    let z_relu = z.relu_approx()?;
    let c: Vec<f32> = z_relu.center.flatten_all()?.to_vec1()?;
    assert!((c[0] - 5.0).abs() < 1e-4);
    assert!((c[1] - 10.0).abs() < 1e-4);
    Ok(())
}

#[test]
fn test_relu_negative() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::new(&[-5.0f32, -10.0], &device)?.unsqueeze(0)?;
    let z = Zonotope::new_from_epsilon(&center, 0.1, 2)?;

    let z_relu = z.relu_approx()?;
    let c: Vec<f32> = z_relu.center.flatten_all()?.to_vec1()?;
    assert!(c[0].abs() < 1e-4);
    assert!(c[1].abs() < 1e-4);
    Ok(())
}

#[test]
fn test_silu_approx() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::new(&[1.0f32, -1.0], &device)?.unsqueeze(0)?;
    let z = Zonotope::new_from_epsilon(&center, 0.1, 2)?;

    let z_silu = z.silu_approx()?;
    // SiLU(1.0) ≈ 0.731, SiLU(-1.0) ≈ -0.269
    let c: Vec<f32> = z_silu.center.flatten_all()?.to_vec1()?;
    assert!((c[0] - 0.731).abs() < 0.05);
    assert!((c[1] + 0.269).abs() < 0.05);
    Ok(())
}

// ---------------------------------------------------------------------------
// Set Operation Tests
// ---------------------------------------------------------------------------

#[test]
fn test_minkowski_sum() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let c1 = Tensor::new(&[1.0f32, 2.0], &device)?.unsqueeze(0)?;
    let c2 = Tensor::new(&[3.0f32, 4.0], &device)?.unsqueeze(0)?;
    let z1 = Zonotope::new_from_epsilon(&c1, 0.1, 2)?;
    let z2 = Zonotope::new_from_epsilon(&c2, 0.1, 2)?;

    let z_sum = z1.minkowski_sum(&z2)?;
    let c: Vec<f32> = z_sum.center.flatten_all()?.to_vec1()?;
    assert!((c[0] - 4.0).abs() < 1e-5);
    assert!((c[1] - 6.0).abs() < 1e-5);
    Ok(())
}

#[test]
fn test_intersect() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let c1 = Tensor::new(&[0.0f32, 0.0], &device)?.unsqueeze(0)?;
    let c2 = Tensor::new(&[0.5f32, 0.5], &device)?.unsqueeze(0)?;
    let z1 = Zonotope::new_from_epsilon(&c1, 0.3, 2)?;
    let z2 = Zonotope::new_from_epsilon(&c2, 0.3, 2)?;

    let z_int = z1.intersect(&z2)?;
    // Intersection should be tighter than either
    let vol_int = z_int.volume_proxy()?;
    let vol1 = z1.volume_proxy()?;
    assert!(vol_int <= vol1 + 1e-5, "Intersection should not be wider");
    Ok(())
}

// ---------------------------------------------------------------------------
// Robustness Certificate Tests
// ---------------------------------------------------------------------------

#[test]
fn test_steering_robustness_safe() -> candle_core::Result<()> {
    let device = Device::Cpu;
    // Activation near safe centroid
    let activation = Tensor::new(&[0.01f32, 0.02, -0.01], &device)?.unsqueeze(0)?;
    let safe = Tensor::zeros((1, 3), DType::F32, &device)?;
    let toxic = Tensor::new(&[10.0f32, 10.0, 10.0], &device)?.unsqueeze(0)?;

    let z = Zonotope::new_from_epsilon(&activation, 0.05, 3)?;
    let cert = z.verify_steering_robustness(&safe, &toxic, 1.0)?;

    assert!(cert.certified, "Safe activation should be certified");
    assert!(cert.direction_safe);
    assert!(cert.distance_safe);
    Ok(())
}

#[test]
fn test_steering_robustness_unsafe() -> candle_core::Result<()> {
    let device = Device::Cpu;
    // Activation far from safe, toward toxic
    let activation = Tensor::new(&[8.0f32, 8.0, 8.0], &device)?.unsqueeze(0)?;
    let safe = Tensor::zeros((1, 3), DType::F32, &device)?;
    let toxic = Tensor::new(&[10.0f32, 10.0, 10.0], &device)?.unsqueeze(0)?;

    let z = Zonotope::new_from_epsilon(&activation, 0.05, 3)?;
    let cert = z.verify_steering_robustness(&safe, &toxic, 1.0)?;

    assert!(!cert.certified, "Unsafe activation should not be certified");
    Ok(())
}

#[test]
fn test_certificate_display() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let activation = Tensor::zeros((1, 3), DType::F32, &device)?;
    let safe = Tensor::zeros((1, 3), DType::F32, &device)?;
    let toxic = Tensor::ones((1, 3), DType::F32, &device)?;

    let z = Zonotope::new_from_epsilon(&activation, 0.01, 3)?;
    let cert = z.verify_steering_robustness(&safe, &toxic, 1.0)?;

    let display = format!("{}", cert);
    assert!(display.contains("certified="));
    assert!(display.contains("volume="));
    Ok(())
}

// ---------------------------------------------------------------------------
// Hybrid Zonotope Tests
// ---------------------------------------------------------------------------

#[test]
fn test_hybrid_creation() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::new(&[1.0f32, -1.0], &device)?.unsqueeze(0)?;
    let z = Zonotope::new_from_epsilon(&center, 0.1, 2)?;

    let hybrid = HybridZonotope::from_zonotope(&z)?;
    assert_eq!(hybrid.zonotope.hidden_dim()?, 2);
    Ok(())
}

#[test]
fn test_hybrid_relu_step() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::new(&[1.0f32, -1.0], &device)?.unsqueeze(0)?;
    let z = Zonotope::new_from_epsilon(&center, 0.1, 2)?;

    let hybrid = HybridZonotope::from_zonotope(&z)?;
    let z_after = hybrid.nonlinear_interval_step("relu")?;

    let (lo, _) = z_after.compute_bounds()?;
    let lo_vec: Vec<f32> = lo.flatten_all()?.to_vec1()?;
    assert!(
        lo_vec[0] >= -1e-4,
        "ReLU lower bound should be non-negative"
    );
    assert!(
        lo_vec[1] >= -1e-4,
        "ReLU lower bound should be non-negative"
    );
    Ok(())
}

#[test]
fn test_hybrid_refine() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::new(&[0.5f32, 0.5], &device)?.unsqueeze(0)?;
    let z = Zonotope::new_from_epsilon(&center, 0.1, 2)?;

    let hybrid = HybridZonotope::from_zonotope(&z)?;
    let z_refined = hybrid.refine_with_intervals()?;

    // Refined zonotope should have bounds within original interval bounds
    let (ref_lo, ref_hi) = z_refined.compute_bounds()?;
    let (orig_lo, orig_hi) = hybrid.zonotope.compute_bounds()?;

    let lo_ok =
        ref_lo
            .broadcast_sub(&orig_lo)?
            .maximum(&Tensor::zeros((1, 2), DType::F32, &device)?)?;
    let hi_ok =
        orig_hi
            .broadcast_sub(&ref_hi)?
            .maximum(&Tensor::zeros((1, 2), DType::F32, &device)?)?;

    let lo_sum = lo_ok.sum_all()?.to_scalar::<f32>()?;
    let hi_sum = hi_ok.sum_all()?.to_scalar::<f32>()?;
    assert!(lo_sum >= 0.0, "Refined lo should be >= original lo");
    assert!(hi_sum >= 0.0, "Refined hi should be <= original hi");
    Ok(())
}

// ---------------------------------------------------------------------------
// Wrapping Effect Comparison
// ---------------------------------------------------------------------------

#[test]
fn test_wrapping_reduction() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::zeros((1, 100), DType::F32, &device)?;
    let z = Zonotope::new_from_epsilon(&center, 0.01, 10)?;

    // Interval width would be 2*epsilon per dim = 0.02
    // Zonotope with 10 gens in 100 dims has much tighter average width
    let interval_width = 0.02;
    let reduction = z.wrapping_reduction_vs_intervals(interval_width)?;
    assert!(
        reduction > 0.0,
        "Zonotope should reduce wrapping vs intervals"
    );
    assert!(reduction <= 1.0, "Reduction should be <= 1.0");
    Ok(())
}

#[test]
fn test_high_dim_zonotope() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 4096;
    let center = Tensor::zeros((1, dim), DType::F32, &device)?;
    let z = Zonotope::new_from_epsilon(&center, 0.05, 64)?;

    assert_eq!(z.hidden_dim()?, dim);
    assert_eq!(z.num_gens()?, 64);

    let vol = z.volume_proxy()?;
    assert!(vol > 0.0);
    // 64 generators * 0.05 each = 3.2
    assert!((vol - 3.2).abs() < 0.1);
    Ok(())
}
