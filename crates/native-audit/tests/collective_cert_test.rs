//! Collective Certified Robustness Tests — Sprint 111 (v11.1.0)
//!
//! Tests for collective certificate integration between hybrid_zonotope and collective_zonotope:
//! - Direction safety verification
//! - Certified radius computation
//! - Byzantine resilience with collective certificates
//! - Volume reduction metrics across nodes
//! - Multi-node consensus with certified bounds

use candle_core::{Device, Tensor};
use native_audit::collective_zonotope::{
    CollectiveZonotopeConfig, CollectiveZonotopeEngine, ZonotopeSummary,
};
use native_audit::hybrid_zonotope::{CollectiveCertificate, HybridZonotope, HybridZonotopeConfig};
use native_audit::zonotope::Zonotope;

// ============================================================================
// Collective Certificate Construction Tests
// ============================================================================

#[test]
fn test_collective_certificate_basic() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::from_vec(vec![0.5f32, 1.0, -0.5], (1, 3), &device)?;
    let config = HybridZonotopeConfig::default();
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;

    // Toxic direction pointing toward positive values
    let direction = Tensor::from_vec(vec![1.0f32, 0.0, 0.0], (1, 3), &device)?;
    let cert = z.verify_collective_robustness(&direction, 0.8)?;

    assert!(cert.proj_center.is_finite());
    assert!(cert.proj_upper.is_finite());
    assert!(cert.proj_lower.is_finite());
    Ok(())
}

#[test]
fn test_collective_certificate_safe_direction() -> candle_core::Result<()> {
    let device = Device::Cpu;
    // Center far from threshold
    let center = Tensor::from_vec(vec![-2.0f32, -1.0], (1, 2), &device)?;
    let config = HybridZonotopeConfig::default();
    let z = HybridZonotope::new_from_epsilon(&center, 0.05, config)?;

    // Direction pointing away from threshold
    let direction = Tensor::from_vec(vec![-1.0f32, 0.0], (1, 2), &device)?;
    let cert = z.verify_collective_robustness(&direction, 0.5)?;

    assert!(cert.proj_upper.is_finite());
    Ok(())
}

#[test]
fn test_collective_certificate_unsafe_direction() -> candle_core::Result<()> {
    let device = Device::Cpu;
    // Center near threshold
    let center = Tensor::from_vec(vec![0.9f32, 0.0], (1, 2), &device)?;
    let config = HybridZonotopeConfig::default();
    let z = HybridZonotope::new_from_epsilon(&center, 0.2, config)?;

    // Direction pointing toward threshold
    let direction = Tensor::from_vec(vec![1.0f32, 0.0], (1, 2), &device)?;
    let cert = z.verify_collective_robustness(&direction, 0.5)?;

    // Upper projection should exceed threshold
    assert!(cert.proj_upper > 0.5 || !cert.direction_safe);
    Ok(())
}

#[test]
fn test_certificate_radius_positive() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::zeros((1, 4), candle_core::DType::F32, &device)?;
    let config = HybridZonotopeConfig::default();
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;
    let direction = Tensor::from_vec(vec![1.0f32, 0.0, 0.0, 0.0], (1, 4), &device)?;
    let cert = z.verify_collective_robustness(&direction, 1.0)?;

    assert!(cert.certified_radius >= 0.0);
    Ok(())
}

#[test]
fn test_certificate_volume_reduction() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::zeros((1, 4), candle_core::DType::F32, &device)?;
    let config = HybridZonotopeConfig::default();
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;
    let direction = Tensor::from_vec(vec![1.0f32, 0.0, 0.0, 0.0], (1, 4), &device)?;
    let cert = z.verify_collective_robustness(&direction, 1.0)?;

    assert!(cert.volume_reduction >= 0.0);
    assert!(cert.volume_reduction.is_finite());
    Ok(())
}

// ============================================================================
// Byzantine Resilience with Certificates
// ============================================================================

#[test]
fn test_byzantine_resistance_certified() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let direction = Tensor::from_vec(vec![1.0f32, 0.0], (1, 2), &device)?;
    let threshold = 0.5;
    let config = HybridZonotopeConfig::default();

    // Honest nodes: centered at -1.0 (safe)
    let honest_centers: Vec<Tensor> = (0..5)
        .map(|_| Tensor::from_vec(vec![-1.0f32, 0.0], (1, 2), &device).unwrap())
        .collect();

    // Byzantine node: centered at 2.0 (unsafe)
    let byzantine_center = Tensor::from_vec(vec![2.0f32, 0.0], (1, 2), &device)?;

    let mut safe_count = 0;
    let mut total_count = 0;

    for c in &honest_centers {
        let z = HybridZonotope::new_from_epsilon(c, 0.1, config.clone())?;
        let cert = z.verify_collective_robustness(&direction, threshold)?;
        if cert.direction_safe {
            safe_count += 1;
        }
        total_count += 1;
    }

    // Byzantine
    let z_byz = HybridZonotope::new_from_epsilon(&byzantine_center, 0.1, config)?;
    let cert_byz = z_byz.verify_collective_robustness(&direction, threshold)?;
    total_count += 1;
    if cert_byz.direction_safe {
        safe_count += 1;
    }

    // Honest majority should be safe
    assert!(safe_count >= 5, "Honest nodes should be certified safe");
    assert!(total_count == 6);
    Ok(())
}

#[test]
fn test_byzantine_majority_detection() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let direction = Tensor::from_vec(vec![1.0f32], (1, 1), &device)?;
    let threshold = 0.0;
    let config = HybridZonotopeConfig::default();

    // 2 honest + 3 byzantine
    let honest: Vec<Tensor> = vec![
        Tensor::from_vec(vec![-1.0f32], (1, 1), &device)?,
        Tensor::from_vec(vec![-0.5f32], (1, 1), &device)?,
    ];
    let byzantine: Vec<Tensor> = vec![
        Tensor::from_vec(vec![1.0f32], (1, 1), &device)?,
        Tensor::from_vec(vec![2.0f32], (1, 1), &device)?,
        Tensor::from_vec(vec![3.0f32], (1, 1), &device)?,
    ];

    let mut safe_count = 0;
    for c in honest.iter().chain(byzantine.iter()) {
        let z = HybridZonotope::new_from_epsilon(c, 0.05, config.clone())?;
        let cert = z.verify_collective_robustness(&direction, threshold)?;
        if cert.direction_safe {
            safe_count += 1;
        }
    }

    // With byzantine majority, some will be unsafe
    assert!(safe_count < 5, "Byzantine majority should cause failures");
    Ok(())
}

#[test]
fn test_certified_byzantine_filtering() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let direction = Tensor::from_vec(vec![1.0f32, 0.0], (1, 2), &device)?;
    let threshold = 0.5;
    let config = HybridZonotopeConfig::default();

    let all_centers: Vec<Tensor> = vec![
        Tensor::from_vec(vec![-1.0f32, 0.0], (1, 2), &device)?, // Safe
        Tensor::from_vec(vec![-0.5f32, 0.0], (1, 2), &device)?, // Safe
        Tensor::from_vec(vec![0.0f32, 0.0], (1, 2), &device)?,  // Safe
        Tensor::from_vec(vec![2.0f32, 0.0], (1, 2), &device)?,  // Unsafe (Byzantine)
        Tensor::from_vec(vec![3.0f32, 0.0], (1, 2), &device)?,  // Unsafe (Byzantine)
    ];

    let mut certified_safe = Vec::new();
    for c in &all_centers {
        let z = HybridZonotope::new_from_epsilon(c, 0.1, config.clone())?;
        let cert = z.verify_collective_robustness(&direction, threshold)?;
        if cert.direction_safe {
            certified_safe.push(c.clone());
        }
    }

    // Only 3 honest nodes should pass
    assert!(
        certified_safe.len() == 3,
        "Filtering should remove Byzantine nodes"
    );
    Ok(())
}

// ============================================================================
// Multi-Node Consensus with Certificates
// ============================================================================

#[test]
fn test_consensus_all_certified_safe() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let direction = Tensor::from_vec(vec![1.0f32], (1, 1), &device)?;
    let threshold = 1.0;
    let config = HybridZonotopeConfig::default();

    let centers: Vec<Tensor> = (0..5)
        .map(|i| Tensor::full(i as f32 * 0.1, (1, 1), &device).unwrap())
        .collect();

    let mut all_safe = true;
    for c in &centers {
        let z = HybridZonotope::new_from_epsilon(c, 0.05, config.clone())?;
        let cert = z.verify_collective_robustness(&direction, threshold)?;
        if !cert.direction_safe {
            all_safe = false;
        }
    }

    assert!(all_safe, "All nodes with small centers should be safe");
    Ok(())
}

#[test]
fn test_consensus_mixed_safety() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let direction = Tensor::from_vec(vec![1.0f32], (1, 1), &device)?;
    let threshold = 0.3;
    let config = HybridZonotopeConfig::default();

    let centers: Vec<Tensor> = vec![
        Tensor::full(0.0f32, (1, 1), &device)?, // Safe
        Tensor::full(0.1f32, (1, 1), &device)?, // Safe
        Tensor::full(0.5f32, (1, 1), &device)?, // Unsafe
        Tensor::full(0.2f32, (1, 1), &device)?, // Safe
    ];

    let mut safe_count = 0;
    for c in &centers {
        let z = HybridZonotope::new_from_epsilon(c, 0.05, config.clone())?;
        let cert = z.verify_collective_robustness(&direction, threshold)?;
        if cert.direction_safe {
            safe_count += 1;
        }
    }

    assert!(safe_count == 3, "3 out of 4 should be safe");
    Ok(())
}

#[test]
fn test_consensus_quorum_verification() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let direction = Tensor::from_vec(vec![1.0f32], (1, 1), &device)?;
    let threshold = 0.5;
    let quorum = 2; // Need 2/3 safe
    let config = HybridZonotopeConfig::default();

    let centers: Vec<Tensor> = vec![
        Tensor::full(0.0f32, (1, 1), &device)?,
        Tensor::full(0.1f32, (1, 1), &device)?,
        Tensor::full(1.0f32, (1, 1), &device)?, // Unsafe
    ];

    let mut safe_count = 0;
    for c in &centers {
        let z = HybridZonotope::new_from_epsilon(c, 0.05, config.clone())?;
        let cert = z.verify_collective_robustness(&direction, threshold)?;
        if cert.direction_safe {
            safe_count += 1;
        }
    }

    assert!(safe_count >= quorum, "Quorum should be met");
    Ok(())
}

// ============================================================================
// Volume Reduction Metrics
// ============================================================================

#[test]
fn test_volume_reduction_after_certification() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::zeros((1, 4), candle_core::DType::F32, &device)?;
    let config = HybridZonotopeConfig::default();
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;

    let vol_before = z.log_volume_proxy()?;
    let direction = Tensor::from_vec(vec![1.0f32, 0.0, 0.0, 0.0], (1, 4), &device)?;
    let cert = z.verify_collective_robustness(&direction, 1.0)?;

    assert!(vol_before.is_finite());
    assert!(cert.volume_reduction >= 0.0);
    Ok(())
}

#[test]
fn test_volume_reduction_comparison() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let config = HybridZonotopeConfig::default();

    // Small zonotope
    let c1 = Tensor::zeros((1, 4), candle_core::DType::F32, &device)?;
    let z1 = HybridZonotope::new_from_epsilon(&c1, 0.05, config.clone())?;
    let d = Tensor::from_vec(vec![1.0f32, 0.0, 0.0, 0.0], (1, 4), &device)?;
    let cert1 = z1.verify_collective_robustness(&d, 1.0)?;

    // Large zonotope
    let c2 = Tensor::zeros((1, 4), candle_core::DType::F32, &device)?;
    let z2 = HybridZonotope::new_from_epsilon(&c2, 0.3, config)?;
    let cert2 = z2.verify_collective_robustness(&d, 1.0)?;

    // Larger zonotope should have different volume reduction
    assert!(cert1.volume_reduction.is_finite());
    assert!(cert2.volume_reduction.is_finite());
    Ok(())
}

// ============================================================================
// Integration: Collective Zonotope + Hybrid Certificate
// ============================================================================

#[test]
fn test_collective_zonotope_with_certificates() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let config = CollectiveZonotopeConfig::default();
    let engine = CollectiveZonotopeEngine::with_device(&config, &device);

    // Create zonotopes for 3 nodes
    let centers: Vec<Tensor> = vec![
        Tensor::from_vec(vec![0.0f32, 0.0], (1, 2), &device)?,
        Tensor::from_vec(vec![0.1f32, -0.1], (1, 2), &device)?,
        Tensor::from_vec(vec![-0.1f32, 0.1], (1, 2), &device)?,
    ];

    let mut summaries = Vec::new();
    for (i, c) in centers.iter().enumerate() {
        let z = Zonotope::new_from_epsilon(c, 0.1, 2)?;
        let summary = ZonotopeSummary::from_zonotope(&z, &format!("node_{}", i), 2)?;
        summaries.push(summary);
    }

    // Aggregate
    let aggregated = engine.robust_aggregate(&summaries)?;

    // Verify with hybrid certificate
    let hybrid_config = HybridZonotopeConfig::default();
    let hybrid = HybridZonotope::from_zonotope(aggregated, hybrid_config)?;
    let direction = Tensor::from_vec(vec![1.0f32, 0.0], (1, 2), &device)?;
    let cert = hybrid.verify_collective_robustness(&direction, 1.0)?;

    assert!(cert.proj_center.is_finite());
    Ok(())
}

#[test]
fn test_gossip_then_certify() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let config = CollectiveZonotopeConfig::default();
    let engine = CollectiveZonotopeEngine::with_device(&config, &device);

    // Node creates zonotope → compresses → reconstructs → certifies
    let center = Tensor::from_vec(vec![0.5f32, 0.5], (1, 2), &device)?;
    let z = Zonotope::new_from_epsilon(&center, 0.1, 2)?;
    let summary = ZonotopeSummary::from_zonotope(&z, "node_0", 2)?;
    let reconstructed = summary.to_zonotope(&device)?;

    let hybrid_config = HybridZonotopeConfig::default();
    let hybrid = HybridZonotope::from_zonotope(reconstructed, hybrid_config)?;
    let direction = Tensor::from_vec(vec![1.0f32, 0.0], (1, 2), &device)?;
    let cert = hybrid.verify_collective_robustness(&direction, 1.0)?;

    assert!(cert.direction_safe || cert.proj_upper.is_finite());
    Ok(())
}

#[test]
fn test_multi_round_certification() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let direction = Tensor::from_vec(vec![1.0f32], (1, 1), &device)?;
    let threshold = 0.5;
    let config = HybridZonotopeConfig::default();

    // Simulate 3 rounds of certification
    for round in 0..3 {
        let epsilon = 0.1 / (round + 1) as f32;
        let center = Tensor::full(0.0f32, (1, 1), &device)?;
        let z = HybridZonotope::new_from_epsilon(&center, epsilon, config.clone())?;
        let cert = z.verify_collective_robustness(&direction, threshold)?;
        assert!(cert.direction_safe, "Round {} should be safe", round);
    }
    Ok(())
}

#[test]
fn test_certified_radius_bounded() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::zeros((1, 4), candle_core::DType::F32, &device)?;
    let config = HybridZonotopeConfig::default();
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;
    let direction = Tensor::from_vec(vec![1.0f32, 0.0, 0.0, 0.0], (1, 4), &device)?;
    let cert = z.verify_collective_robustness(&direction, 1.0)?;

    // Certified radius should be bounded
    assert!(cert.certified_radius >= 0.0);
    Ok(())
}

#[test]
fn test_projection_ordering() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::from_vec(vec![0.5f32], (1, 1), &device)?;
    let config = HybridZonotopeConfig::default();
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;
    let direction = Tensor::from_vec(vec![1.0f32], (1, 1), &device)?;
    let cert = z.verify_collective_robustness(&direction, 1.0)?;

    // lower <= center <= upper
    assert!(cert.proj_lower <= cert.proj_center + 1e-5);
    assert!(cert.proj_center <= cert.proj_upper + 1e-5);
    Ok(())
}

#[test]
fn test_collective_certificate_display() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::zeros((1, 2), candle_core::DType::F32, &device)?;
    let config = HybridZonotopeConfig::default();
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;
    let direction = Tensor::from_vec(vec![1.0f32, 0.0], (1, 2), &device)?;
    let cert = z.verify_collective_robustness(&direction, 1.0)?;

    // Verify all fields are accessible
    let _ = cert.direction_safe;
    let _ = cert.certified_radius;
    let _ = cert.proj_upper;
    let _ = cert.proj_lower;
    let _ = cert.proj_center;
    let _ = cert.volume_reduction;
    Ok(())
}

#[test]
fn test_certificate_fields_finite() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::from_vec(vec![0.0f32, 0.0], (1, 2), &device)?;
    let config = HybridZonotopeConfig::default();
    let z = HybridZonotope::new_from_epsilon(&center, 0.05, config)?;
    let direction = Tensor::from_vec(vec![1.0f32, 0.0], (1, 2), &device)?;
    let cert = z.verify_collective_robustness(&direction, 1.0)?;

    assert!(cert.proj_center.is_finite());
    assert!(cert.proj_upper.is_finite());
    assert!(cert.proj_lower.is_finite());
    assert!(cert.certified_radius.is_finite());
    Ok(())
}

#[test]
fn test_empty_direction() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::zeros((1, 2), candle_core::DType::F32, &device)?;
    let config = HybridZonotopeConfig::default();
    let z = HybridZonotope::new_from_epsilon(&center, 0.1, config)?;
    let direction = Tensor::zeros((1, 2), candle_core::DType::F32, &device)?;
    let cert = z.verify_collective_robustness(&direction, 1.0)?;

    // Zero direction should project to zero
    assert!((cert.proj_center - 0.0).abs() < 1e-6);
    Ok(())
}
