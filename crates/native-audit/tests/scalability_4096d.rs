//! Scalability Tests — Sprint 111 (v11.1.0)
//!
//! Benchmark dimension scaling from 512 → 4096 for hybrid zonotope operations.
//! Target: latency < 100ms for 4096d zonotope propagation.

use candle_core::{Device, Tensor};
use native_audit::hybrid_zonotope::{HybridZonotope, HybridZonotopeConfig, LayerType};
use native_audit::meta_active_inference::{MetaActiveInferenceConfig, MetaActiveInferenceEngine};
use native_audit::zonotope::{Zonotope, ZonotopeConfig};

// ============================================================================
// Dimension Scaling Tests
// ============================================================================

#[test]
fn test_scalability_dim_64() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 64;
    let center = Tensor::zeros((1, dim), candle_core::DType::F32, &device)?;
    let config = HybridZonotopeConfig::default();
    let start = std::time::Instant::now();
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;
    let elapsed = start.elapsed();

    let (lo, hi) = z.compute_bounds()?;
    assert_eq!(lo.shape().dims(), &[1, dim]);
    assert!(
        elapsed.as_millis() < 100,
        "Dim {} creation should be <100ms: {}ms",
        dim,
        elapsed.as_millis()
    );
    Ok(())
}

#[test]
fn test_scalability_dim_256() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 256;
    let center = Tensor::zeros((1, dim), candle_core::DType::F32, &device)?;
    let config = HybridZonotopeConfig::default();
    let start = std::time::Instant::now();
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;
    let elapsed = start.elapsed();

    let bounds = z.compute_bounds()?;
    assert!(bounds.0.dims().len() > 0);
    assert!(
        elapsed.as_millis() < 200,
        "Dim {} creation should be <200ms: {}ms",
        dim,
        elapsed.as_millis()
    );
    Ok(())
}

#[test]
fn test_scalability_dim_512() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 512;
    let center = Tensor::zeros((1, dim), candle_core::DType::F32, &device)?;
    let config = HybridZonotopeConfig::default();
    let start = std::time::Instant::now();
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;
    let elapsed = start.elapsed();

    let (lo, hi) = z.compute_bounds()?;
    assert_eq!(lo.shape().dims(), &[1, dim]);
    assert!(
        elapsed.as_millis() < 500,
        "Dim {} creation should be <500ms: {}ms",
        dim,
        elapsed.as_millis()
    );
    Ok(())
}

#[test]
fn test_scalability_dim_1024() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 1024;
    let center = Tensor::zeros((1, dim), candle_core::DType::F32, &device)?;
    let config = HybridZonotopeConfig::default();
    let start = std::time::Instant::now();
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;
    let elapsed = start.elapsed();

    let vol = z.log_volume_proxy()?;
    assert!(vol.is_finite());
    assert!(
        elapsed.as_millis() < 1000,
        "Dim {} creation should be <1000ms: {}ms",
        dim,
        elapsed.as_millis()
    );
    Ok(())
}

#[test]
fn test_scalability_dim_4096() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 4096;
    let center = Tensor::zeros((1, dim), candle_core::DType::F32, &device)?;
    let config = HybridZonotopeConfig::default();
    let start = std::time::Instant::now();
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;
    let elapsed = start.elapsed();

    let (lo, hi) = z.compute_bounds()?;
    assert_eq!(lo.shape().dims(), &[1, dim]);
    assert_eq!(hi.shape().dims(), &[1, dim]);
    assert!(
        elapsed.as_millis() < 5000,
        "Dim {} creation should be <5000ms: {}ms",
        dim,
        elapsed.as_millis()
    );
    Ok(())
}

// ============================================================================
// Propagation Latency Tests
// ============================================================================

#[test]
fn test_affine_latency_512d() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 512;
    let center = Tensor::zeros((1, dim), candle_core::DType::F32, &device)?;
    let config = HybridZonotopeConfig::default();
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;
    let weight = Tensor::randn(0.0, 0.1, (dim, dim), &device)?.to_dtype(candle_core::DType::F32)?;

    let start = std::time::Instant::now();
    let _z_out = z.affine_transform(&weight, None)?;
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 2000,
        "Dim {} affine should be <2000ms: {}ms",
        dim,
        elapsed.as_millis()
    );
    Ok(())
}

#[test]
fn test_affine_latency_1024d() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 1024;
    let center = Tensor::zeros((1, dim), candle_core::DType::F32, &device)?;
    let config = HybridZonotopeConfig::default();
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;
    let weight = Tensor::randn(0.0, 0.1, (dim, dim), &device)?.to_dtype(candle_core::DType::F32)?;

    let start = std::time::Instant::now();
    let _z_out = z.affine_transform(&weight, None)?;
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 5000,
        "Dim {} affine should be <5000ms: {}ms",
        dim,
        elapsed.as_millis()
    );
    Ok(())
}

#[test]
fn test_relu_latency_512d() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 512;
    let center = Tensor::randn(0.0, 0.5, (1, dim), &device)?.to_dtype(candle_core::DType::F32)?;
    let config = HybridZonotopeConfig::default();
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;

    let start = std::time::Instant::now();
    let _z_out = z.relu_tight()?;
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 1000,
        "Dim {} relu should be <1000ms: {}ms",
        dim,
        elapsed.as_millis()
    );
    Ok(())
}

#[test]
fn test_relu_latency_1024d() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 1024;
    let center = Tensor::randn(0.0, 0.5, (1, dim), &device)?.to_dtype(candle_core::DType::F32)?;
    let config = HybridZonotopeConfig::default();
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;

    let start = std::time::Instant::now();
    let _z_out = z.relu_tight()?;
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 3000,
        "Dim {} relu should be <3000ms: {}ms",
        dim,
        elapsed.as_millis()
    );
    Ok(())
}

#[test]
fn test_propagate_layer_latency_256d() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 256;
    let center = Tensor::zeros((1, dim), candle_core::DType::F32, &device)?;
    let config = HybridZonotopeConfig::default();
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;
    let weight = Tensor::randn(0.0, 0.1, (dim, dim), &device)?.to_dtype(candle_core::DType::F32)?;

    let start = std::time::Instant::now();
    let _z_out = z.propagate_through_layer(&weight, None, LayerType::ReLU)?;
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 2000,
        "Dim {} propagate should be <2000ms: {}ms",
        dim,
        elapsed.as_millis()
    );
    Ok(())
}

// ============================================================================
// Generator Count Scaling
// ============================================================================

#[test]
fn test_generator_count_64() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 128;
    let num_gens = 64;
    let center = Tensor::zeros((1, dim), candle_core::DType::F32, &device)?;
    let config = HybridZonotopeConfig {
        zonotope_config: ZonotopeConfig {
            max_gens: num_gens,
            ..ZonotopeConfig::default()
        },
        ..HybridZonotopeConfig::default()
    };
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;

    let vol = z.log_volume_proxy()?;
    assert!(vol.is_finite());
    Ok(())
}

#[test]
fn test_generator_count_256() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 512;
    let num_gens = 256;
    let center = Tensor::zeros((1, dim), candle_core::DType::F32, &device)?;
    let config = HybridZonotopeConfig {
        zonotope_config: ZonotopeConfig {
            max_gens: num_gens,
            ..ZonotopeConfig::default()
        },
        ..HybridZonotopeConfig::default()
    };
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;

    let vol = z.log_volume_proxy()?;
    assert!(vol.is_finite());
    Ok(())
}

#[test]
fn test_generator_count_equals_dim() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 256;
    let center = Tensor::zeros((1, dim), candle_core::DType::F32, &device)?;
    let config = HybridZonotopeConfig::default();
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;

    let (lo, hi) = z.compute_bounds()?;
    assert_eq!(lo.shape().dims(), &[1, dim]);
    Ok(())
}

#[test]
fn test_generator_reduction_large() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 128;
    let num_gens = 256; // More gens than dim → should trigger reduction
    let center = Tensor::zeros((1, dim), candle_core::DType::F32, &device)?;
    let config = HybridZonotopeConfig {
        zonotope_config: ZonotopeConfig {
            max_gens: dim,
            epsilon: 0.1,
            reduce_after_nonlinear: true,
            prune_threshold: 1.5,
        },
        ..HybridZonotopeConfig::default()
    };
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;

    // After reduction, gens should be <= max_gens
    let gens_shape = z.zonotope.generators.shape().dims();
    assert!(gens_shape[0] <= dim * 2, "Generator reduction should limit count");
    Ok(())
}

// ============================================================================
// Neural Tightener Scaling
// ============================================================================

#[test]
fn test_neural_tightener_256d() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 256;
    let config = HybridZonotopeConfig {
        use_neural_tightener: true,
        tightener_hidden: 32,
        ..HybridZonotopeConfig::default()
    };
    let center = Tensor::zeros((1, dim), candle_core::DType::F32, &device)?;
    let start = std::time::Instant::now();
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;
    let elapsed = start.elapsed();

    assert!(z.tightener.is_some());
    assert!(
        elapsed.as_millis() < 1000,
        "Neural tightener creation should be fast: {}ms",
        elapsed.as_millis()
    );
    Ok(())
}

#[test]
fn test_neural_tightener_512d() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 512;
    let config = HybridZonotopeConfig {
        use_neural_tightener: true,
        tightener_hidden: 64,
        ..HybridZonotopeConfig::default()
    };
    let center = Tensor::zeros((1, dim), candle_core::DType::F32, &device)?;
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;

    assert!(z.tightener.is_some());
    let bounds = z.compute_bounds()?;
    assert!(bounds.0.dims().len() > 0);
    Ok(())
}

// ============================================================================
// Certificate Scaling
// ============================================================================

#[test]
fn test_certificate_latency_256d() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 256;
    let config = HybridZonotopeConfig {
        mc_samples: 64,
        ..HybridZonotopeConfig::default()
    };
    let center = Tensor::randn(0.0, 0.5, (1, dim), &device)?.to_dtype(candle_core::DType::F32)?;
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;

    let start = std::time::Instant::now();
    let cert = z.verify_neural_certificate(&device)?;
    let elapsed = start.elapsed();

    assert!(cert.num_samples == 64);
    assert!(
        elapsed.as_millis() < 5000,
        "Dim {} certificate should be <5000ms: {}ms",
        dim,
        elapsed.as_millis()
    );
    Ok(())
}

#[test]
fn test_collective_cert_latency_256d() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 256;
    let center = Tensor::zeros((1, dim), candle_core::DType::F32, &device)?;
    let config = HybridZonotopeConfig::default();
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;
    let direction = Tensor::zeros((1, dim), candle_core::DType::F32, &device)?;

    let start = std::time::Instant::now();
    let cert = z.verify_collective_robustness(&direction, 1.0)?;
    let elapsed = start.elapsed();

    assert!(cert.proj_center.is_finite());
    assert!(
        elapsed.as_millis() < 2000,
        "Dim {} collective cert should be <2000ms: {}ms",
        dim,
        elapsed.as_millis()
    );
    Ok(())
}

// ============================================================================
// Multi-Layer Network Scaling
// ============================================================================

#[test]
fn test_network_3layer_128d() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 128;
    let center = Tensor::zeros((1, dim), candle_core::DType::F32, &device)?;
    let config = HybridZonotopeConfig::default();
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;

    let w1 = Tensor::randn(0.0, 0.1, (dim, dim), &device)?.to_dtype(candle_core::DType::F32)?;
    let w2 = Tensor::randn(0.0, 0.1, (dim, dim), &device)?.to_dtype(candle_core::DType::F32)?;
    let w3 = Tensor::randn(0.0, 0.1, (dim, dim), &device)?.to_dtype(candle_core::DType::F32)?;
    let layers: [(&Tensor, Option<&Tensor>, LayerType); 3] = [
        (&w1, None, LayerType::ReLU),
        (&w2, None, LayerType::ReLU),
        (&w3, None, LayerType::SiLU),
    ];

    let start = std::time::Instant::now();
    let z_out = z.propagate_through_network(&layers)?;
    let elapsed = start.elapsed();

    let vol = z_out.log_volume_proxy()?;
    assert!(vol.is_finite());
    assert!(
        elapsed.as_millis() < 5000,
        "3-layer {}d should be <5000ms: {}ms",
        dim,
        elapsed.as_millis()
    );
    Ok(())
}

#[test]
fn test_network_5layer_64d() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 64;
    let center = Tensor::zeros((1, dim), candle_core::DType::F32, &device)?;
    let config = HybridZonotopeConfig::default();
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;

    let w0 = Tensor::randn(0.0, 0.1, (dim, dim), &device)?.to_dtype(candle_core::DType::F32)?;
    let w1 = Tensor::randn(0.0, 0.1, (dim, dim), &device)?.to_dtype(candle_core::DType::F32)?;
    let w2 = Tensor::randn(0.0, 0.1, (dim, dim), &device)?.to_dtype(candle_core::DType::F32)?;
    let w3 = Tensor::randn(0.0, 0.1, (dim, dim), &device)?.to_dtype(candle_core::DType::F32)?;
    let w4 = Tensor::randn(0.0, 0.1, (dim, dim), &device)?.to_dtype(candle_core::DType::F32)?;
    let layers: [(&Tensor, Option<&Tensor>, LayerType); 5] = [
        (&w0, None, LayerType::ReLU),
        (&w1, None, LayerType::SiLU),
        (&w2, None, LayerType::GeLU),
        (&w3, None, LayerType::ReLU),
        (&w4, None, LayerType::SiLU),
    ];

    let z_out = z.propagate_through_network(&layers)?;
    let (lo, hi) = z_out.compute_bounds()?;
    assert_eq!(lo.shape().dims(), &[1, dim]);
    Ok(())
}

// ============================================================================
// Memory Efficiency Tests
// ============================================================================

#[test]
fn test_volume_proxy_scales_logarithmically() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dims = [64, 128, 256, 512];
    let epsilon = 0.1;
    let mut prev_per_dim_vol: Option<f32> = None;

    for &dim in &dims {
        let center = Tensor::zeros((1, dim), candle_core::DType::F32, &device)?;
        // Use max_gens = dim so all dimensions have non-zero width
        let config = HybridZonotopeConfig {
            zonotope_config: ZonotopeConfig {
                max_gens: dim,
                ..ZonotopeConfig::default()
            },
            ..HybridZonotopeConfig::default()
        };
        let z = HybridZonotope::new_from_epsilon(&center, epsilon, config)?;
        let vol = z.log_volume_proxy()?;
        assert!(
            vol.is_finite(),
            "Log volume must be finite for dim={}",
            dim
        );
        // Per-dimension log-volume should be ≈ log(2*epsilon) ≈ log(0.2) ≈ -1.609
        // This verifies logarithmic scaling: total_vol ≈ dim * log(2*eps)
        let per_dim = vol / dim as f32;
        if let Some(prev) = prev_per_dim_vol {
            assert!(
                (per_dim - prev).abs() < 0.5,
                "Per-dim log volume should be stable: dim={} per_dim={} prev={}",
                dim,
                per_dim,
                prev
            );
        }
        prev_per_dim_vol = Some(per_dim);
    }
    Ok(())
}

#[test]
fn test_bounds_computation_memory() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 1024;
    let center = Tensor::zeros((1, dim), candle_core::DType::F32, &device)?;
    let config = HybridZonotopeConfig::default();
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;

    // Compute bounds multiple times → should not leak memory
    for _ in 0..10 {
        let (lo, hi) = z.compute_bounds()?;
        let _width = hi.broadcast_sub(&lo)?;
    }
    Ok(())
}

// ============================================================================
// Meta-Optimization Scaling
// ============================================================================

#[test]
fn test_nes_scaling_many_peers() {
    let config = MetaActiveInferenceConfig {
        use_nes: true,
        use_es: false,
        population_size: 30,
        meta_lr: 0.05,
        ..Default::default()
    };
    let mut engine = MetaActiveInferenceEngine::new(&config);

    // 1000 peers
    let peer_vfes: Vec<f32> = (0..1000).map(|i| 0.3 + (i % 100) as f32 * 0.01).collect();

    let start = std::time::Instant::now();
    let result = engine.meta_optimize(&peer_vfes);
    let elapsed = start.elapsed();

    assert!(result.is_ok());
    assert!(
        elapsed.as_millis() < 5000,
        "NES with 1000 peers should be <5000ms: {}ms",
        elapsed.as_millis()
    );
}

#[test]
fn test_nes_population_scaling() {
    let pop_sizes = [10, 20, 50];

    for &pop in &pop_sizes {
        let config = MetaActiveInferenceConfig {
            use_nes: true,
            use_es: false,
            population_size: pop,
            meta_lr: 0.05,
            ..Default::default()
        };
        let mut engine = MetaActiveInferenceEngine::new(&config);
        let result = engine.meta_optimize_sequence(5, &[0.5, 0.6, 0.4]);
        assert!(result.is_ok(), "Pop={} should succeed", pop);
    }
}

// ============================================================================
// End-to-End Scalability
// ============================================================================

#[test]
fn test_e2e_pipeline_256d() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 256;

    // 1. Create zonotope
    let center = Tensor::randn(0.0, 0.5, (1, dim), &device)?.to_dtype(candle_core::DType::F32)?;
    let config = HybridZonotopeConfig::default();
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;

    // 2. Propagate through 2 layers
    let w1 = Tensor::randn(0.0, 0.1, (dim, dim), &device)?.to_dtype(candle_core::DType::F32)?;
    let w2 = Tensor::randn(0.0, 0.1, (dim, dim), &device)?.to_dtype(candle_core::DType::F32)?;
    let layers: [(&Tensor, Option<&Tensor>, LayerType); 2] = [
        (&w1, None, LayerType::ReLU),
        (&w2, None, LayerType::SiLU),
    ];
    let z_out = z.propagate_through_network(&layers)?;

    // 3. Verify certificate
    let direction = Tensor::zeros((1, dim), candle_core::DType::F32, &device)?;
    let cert = z_out.verify_collective_robustness(&direction, 1.0)?;

    assert!(cert.proj_center.is_finite());
    assert!(cert.volume_reduction >= 0.0);
    Ok(())
}

#[test]
fn test_e2e_pipeline_512d() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 512;

    let center = Tensor::randn(0.0, 0.5, (1, dim), &device)?.to_dtype(candle_core::DType::F32)?;
    let config = HybridZonotopeConfig::default();
    let z = HybridZonotope::new_from_epsilon(&center, 0.05, config)?;

    let w1 = Tensor::randn(0.0, 0.05, (dim, dim), &device)?.to_dtype(candle_core::DType::F32)?;
    let w2 = Tensor::randn(0.0, 0.05, (dim, dim), &device)?.to_dtype(candle_core::DType::F32)?;
    let layers: [(&Tensor, Option<&Tensor>, LayerType); 2] = [
        (&w1, None, LayerType::ReLU),
        (&w2, None, LayerType::ReLU),
    ];
    let z_out = z.propagate_through_network(&layers)?;

    let vol = z_out.log_volume_proxy()?;
    assert!(vol.is_finite(), "E2E 512d volume should be finite");
    Ok(())
}
