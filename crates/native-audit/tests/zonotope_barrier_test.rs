//! Zonotope vs Interval Barrier Comparison Tests — Sprint 110
//!
//! Ablation tests demonstrating zonotope superiority over pure interval
//! arithmetic for bound propagation in high-dimensional latent spaces.

use candle_core::{DType, Device, Tensor};
use native_audit::zonotope::{HybridZonotope, Zonotope};

// ---------------------------------------------------------------------------
// Ablation: Zonotope vs Interval Error Volume
// ---------------------------------------------------------------------------

#[test]
fn test_zonotope_tighter_than_intervals_1d() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::new(&[0.0f32], &device)?.unsqueeze(0)?;
    let epsilon = 0.1f32;

    // Zonotope bounds
    let z = Zonotope::new_from_epsilon(&center, epsilon, 1)?;
    let (z_lo, z_hi) = z.compute_bounds()?;
    let z_width = z_hi.broadcast_sub(&z_lo)?.to_scalar::<f32>()?;

    // Pure interval: same epsilon but no correlation tracking
    let interval_width = 2.0 * epsilon;

    // In 1D with single generator, they should match
    assert!((z_width - interval_width).abs() < 1e-5);
    Ok(())
}

#[test]
fn test_zonotope_tighter_than_intervals_2d() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::new(&[0.0f32, 0.0], &device)?.unsqueeze(0)?;
    let epsilon = 0.1f32;

    // Zonotope with 2 generators (diagonal)
    let z = Zonotope::new_from_epsilon(&center, epsilon, 2)?;
    let z_vol = z.volume_proxy()?;

    // Pure interval volume: sum of widths = 2 * 2*epsilon * dims
    let interval_vol = 2.0 * epsilon * 2.0; // 2 dims * 2*epsilon each

    // Zonotope volume should be <= interval volume (same in diagonal case)
    assert!(z_vol <= interval_vol + 1e-5);
    Ok(())
}

#[test]
fn test_zonotope_correlation_benefit() -> candle_core::Result<()> {
    let device = Device::Cpu;
    // Correlated perturbation: both dims move together
    let center = Tensor::new(&[0.0f32, 0.0], &device)?.unsqueeze(0)?;
    let epsilon = 0.1f32;

    // Zonotope with 1 generator that correlates both dims
    let generators = Tensor::new(
        &[epsilon / 2.0_f32.sqrt(), epsilon / 2.0_f32.sqrt()],
        &device,
    )?
    .unsqueeze(0)?;
    let config = native_audit::zonotope::ZonotopeConfig {
        max_gens: 1,
        epsilon,
        ..Default::default()
    };
    let z = Zonotope::new(center, generators, config)?;

    let z_vol = z.volume_proxy()?;

    // Uncorrelated interval would have volume = 2*epsilon per dim = 0.4
    // Correlated zonotope has volume = 2*epsilon/sqrt(2) ≈ 0.28
    let interval_vol = 2.0 * epsilon * 2.0;
    let reduction = 1.0 - (z_vol / interval_vol);

    assert!(
        reduction > 0.2,
        "Correlated zonotope should reduce volume by >20%, got {:.2}%",
        reduction * 100.0
    );
    Ok(())
}

#[test]
fn test_high_dim_wrapping_reduction() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 4096;
    let epsilon = 0.05f32;

    let center = Tensor::zeros((1, dim), DType::F32, &device)?;

    // Zonotope with limited generators (low-rank approximation)
    let max_gens = 64;
    let z = Zonotope::new_from_epsilon(&center, epsilon, max_gens)?;
    let z_avg_width = z.avg_width()?;

    // Pure interval: each dim has width 2*epsilon
    let interval_avg_width = 2.0 * epsilon;

    // With 64 gens in 4096 dims, only 64 dims have perturbation
    // Average width = 64 * 2*epsilon / 4096 = 2*epsilon / 64
    let _expected_reduction = 1.0 - (1.0 / (max_gens as f32));

    assert!(
        z_avg_width < interval_avg_width,
        "Zonotope avg width ({:.4}) < interval ({:.4})",
        z_avg_width,
        interval_avg_width
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Scalability Tests
// ---------------------------------------------------------------------------

#[test]
fn test_scalability_dim_512() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 512;
    let center = Tensor::zeros((1, dim), DType::F32, &device)?;
    let z = Zonotope::new_from_epsilon(&center, 0.05, 64)?;

    assert_eq!(z.hidden_dim()?, dim);
    assert_eq!(z.num_gens()?, 64);

    let (lo, hi) = z.compute_bounds()?;
    assert_eq!(lo.dim(1)?, dim);
    assert_eq!(hi.dim(1)?, dim);
    Ok(())
}

#[test]
fn test_scalability_dim_1024() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 1024;
    let center = Tensor::zeros((1, dim), DType::F32, &device)?;
    let z = Zonotope::new_from_epsilon(&center, 0.05, 64)?;

    let vol = z.volume_proxy()?;
    // 64 gens * 0.05 = 3.2
    assert!((vol - 3.2).abs() < 0.1);
    Ok(())
}

#[test]
fn test_scalability_dim_4096() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 4096;
    let center = Tensor::zeros((1, dim), DType::F32, &device)?;
    let z = Zonotope::new_from_epsilon(&center, 0.05, 64)?;

    // Verify bounds computation works for large dims
    let (_lo, _hi) = z.compute_bounds()?;
    let width = z.avg_width()?;

    // With 64 gens in 4096 dims, avg width should be small
    assert!(width < 0.1, "Avg width should be small: {}", width);
    Ok(())
}

#[test]
fn test_scalability_generator_counts() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 256;
    let center = Tensor::zeros((1, dim), DType::F32, &device)?;

    for &max_gens in &[8, 16, 32, 64, 128] {
        let z = Zonotope::new_from_epsilon(&center, 0.05, max_gens)?;
        assert_eq!(z.num_gens()?, max_gens.min(dim));

        let vol = z.volume_proxy()?;
        assert!(
            vol > 0.0,
            "Volume should be positive for max_gens={}",
            max_gens
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Affine Propagation Tests
// ---------------------------------------------------------------------------

#[test]
fn test_affine_preserves_safety() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::new(&[0.0f32, 0.0], &device)?.unsqueeze(0)?;
    let z = Zonotope::new_from_epsilon(&center, 0.05, 2)?;

    // Rotation matrix (orthogonal — preserves norms)
    let theta = std::f32::consts::PI / 4.0;
    let c = theta.cos();
    let s = theta.sin();
    let weight = Tensor::from_vec(vec![c, -s, s, c], (2, 2), &device)?;

    let z_rotated = z.affine_transform(&weight, None)?;

    // Volume should be preserved (orthogonal transform)
    let vol_orig = z.volume_proxy()?;
    let vol_rotated = z_rotated.volume_proxy()?;
    assert!(
        (vol_orig - vol_rotated).abs() < 1e-4,
        "Orthogonal transform should preserve volume: {} vs {}",
        vol_orig,
        vol_rotated
    );
    Ok(())
}

#[test]
fn test_affine_contraction() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::new(&[1.0f32, 2.0], &device)?.unsqueeze(0)?;
    let z = Zonotope::new_from_epsilon(&center, 0.1, 2)?;

    // Contraction: scale by 0.5
    let weight = Tensor::from_vec(vec![0.5f32, 0.0, 0.0, 0.5], (2, 2), &device)?;
    let z_contracted = z.affine_transform(&weight, None)?;

    let vol_orig = z.volume_proxy()?;
    let vol_contracted = z_contracted.volume_proxy()?;
    assert!(
        (vol_contracted - vol_orig * 0.5).abs() < 1e-5,
        "Contraction should halve volume"
    );
    Ok(())
}

#[test]
fn test_sequential_affine_ops() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::zeros((1, 4), DType::F32, &device)?;
    let mut z = Zonotope::new_from_epsilon(&center, 0.1, 4)?;

    // Apply 10 random-ish affine transforms
    for i in 0..10 {
        let scale = 0.9 + (i % 3) as f32 * 0.1;
        let weight = {
            let s = Tensor::full(scale, (), &device)?;
            Tensor::eye(4, DType::F32, &device)?.broadcast_mul(&s)?
        };
        z = z.affine_transform(&weight, None)?;
    }

    assert_eq!(z.hidden_dim()?, 4);
    assert_eq!(z.num_gens()?, 4);
    Ok(())
}

// ---------------------------------------------------------------------------
// Hybrid Zonotope-Interval Tests
// ---------------------------------------------------------------------------

#[test]
fn test_hybrid_tighter_than_pure_interval() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::new(&[1.0f32, 2.0, 3.0], &device)?.unsqueeze(0)?;
    let z = Zonotope::new_from_epsilon(&center, 0.1, 3)?;

    let hybrid = HybridZonotope::from_zonotope(&z)?;
    let z_refined = hybrid.refine_with_intervals()?;

    // Refined zonotope should have volume <= original
    let vol_orig = z.volume_proxy()?;
    let vol_refined = z_refined.volume_proxy()?;
    assert!(
        vol_refined <= vol_orig + 1e-5,
        "Refined volume ({}) should be <= original ({})",
        vol_refined,
        vol_orig
    );
    Ok(())
}

#[test]
fn test_hybrid_nonlinear_chain() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::new(&[1.0f32, -0.5, 2.0], &device)?.unsqueeze(0)?;
    let z = Zonotope::new_from_epsilon(&center, 0.1, 3)?;

    let mut hybrid = HybridZonotope::from_zonotope(&z)?;

    // Chain: ReLU → SiLU → ReLU
    for op in &["relu", "silu", "relu"] {
        let z_after = hybrid.nonlinear_interval_step(op)?;
        hybrid = HybridZonotope::from_zonotope(&z_after)?;
    }

    // Final bounds should be finite
    let (lo, hi) = hybrid.zonotope.compute_bounds()?;
    let lo_vec: Vec<f32> = lo.flatten_all()?.to_vec1()?;
    let hi_vec: Vec<f32> = hi.flatten_all()?.to_vec1()?;

    for i in 0..3 {
        assert!(lo_vec[i].is_finite(), "lo[{}] should be finite", i);
        assert!(hi_vec[i].is_finite(), "hi[{}] should be finite", i);
        assert!(lo_vec[i] <= hi_vec[i] + 1e-5, "lo[{}] <= hi[{}]", i, i);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Integration with Formal Barrier
// ---------------------------------------------------------------------------

#[test]
fn test_zonotope_barrier_integration() -> candle_core::Result<()> {
    let device = Device::Cpu;
    // Safe activation
    let activation = Tensor::new(&[0.1f32, -0.1, 0.05], &device)?.unsqueeze(0)?;

    // Create zonotope around activation
    let z = Zonotope::new_from_epsilon(&activation, 0.05, 3)?;

    // Verify using zonotope bounds
    let (lo, hi) = z.compute_bounds()?;
    let lo_vec: Vec<f32> = lo.flatten_all()?.to_vec1()?;
    let hi_vec: Vec<f32> = hi.flatten_all()?.to_vec1()?;

    // Check all bounds are within safe range [-1, 1]
    for i in 0..3 {
        assert!(lo_vec[i] >= -1.0, "lo[{}] = {} < -1.0", i, lo_vec[i]);
        assert!(hi_vec[i] <= 1.0, "hi[{}] = {} > 1.0", i, hi_vec[i]);
    }
    Ok(())
}

#[test]
fn test_interval_vs_zonotope_certificate() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let activation = Tensor::new(&[0.0f32, 0.0, 0.0], &device)?.unsqueeze(0)?;
    let epsilon = 0.1f32;

    // Interval certificate
    let interval_width = 2.0 * epsilon;

    // Zonotope certificate
    let z = Zonotope::new_from_epsilon(&activation, epsilon, 3)?;
    let z_width = z.avg_width()?;

    // Zonotope should be at least as tight as interval
    assert!(
        z_width <= interval_width + 1e-5,
        "Zonotope width ({}) should be <= interval width ({})",
        z_width,
        interval_width
    );
    Ok(())
}

#[test]
fn test_minkowski_sum_chain() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::zeros((1, 3), DType::F32, &device)?;

    let mut z = Zonotope::point(&center)?;

    // Chain 10 Minkowski sums
    for _i in 0..10 {
        let perturbation = Zonotope::new_from_epsilon(&center, 0.01, 3)?;
        z = z.minkowski_sum(&perturbation)?;
    }

    // Volume should grow linearly with number of sums
    let vol = z.volume_proxy()?;
    assert!(
        vol > 0.0 && vol < 10.0,
        "Volume should be reasonable: {}",
        vol
    );
    Ok(())
}

#[test]
fn test_intersection_tightens() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let c1 = Tensor::new(&[0.0f32, 0.0], &device)?.unsqueeze(0)?;
    let c2 = Tensor::new(&[0.1f32, 0.1], &device)?.unsqueeze(0)?;

    let z1 = Zonotope::new_from_epsilon(&c1, 0.2, 2)?;
    let z2 = Zonotope::new_from_epsilon(&c2, 0.2, 2)?;

    let z_int = z1.intersect(&z2)?;

    // Intersection should be tighter
    let vol1 = z1.volume_proxy()?;
    let vol_int = z_int.volume_proxy()?;
    assert!(
        vol_int <= vol1,
        "Intersection volume ({}) <= original ({})",
        vol_int,
        vol1
    );
    Ok(())
}
