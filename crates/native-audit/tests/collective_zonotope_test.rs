//! Collective Zonotope Tests — Sprint 110
//!
//! Tests for distributed zonotope gossip, robust aggregation,
//! trust-weighted fusion, and consensus verification.

use candle_core::{DType, Device, Tensor};
use native_audit::collective_zonotope::{
    CollectiveZonotopeConfig, CollectiveZonotopeEngine, ZonotopeSummary,
};
use native_audit::zonotope::Zonotope;

// ---------------------------------------------------------------------------
// Summary Compression Tests
// ---------------------------------------------------------------------------

#[test]
fn test_summary_compression() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::new(&[1.0f32, 2.0, 3.0, 4.0], &device)?.unsqueeze(0)?;
    let z = Zonotope::new_from_epsilon(&center, 0.1, 4)?;

    let summary = ZonotopeSummary::from_zonotope(&z, "peer_1", 2)?;

    assert_eq!(summary.center.len(), 4);
    assert_eq!(summary.generators.len(), 2);
    assert_eq!(summary.peer_id, "peer_1");
    assert!(summary.volume_proxy > 0.0);
    Ok(())
}

#[test]
fn test_summary_full_preservation() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::new(&[1.0f32, 2.0, 3.0], &device)?.unsqueeze(0)?;
    let z = Zonotope::new_from_epsilon(&center, 0.1, 3)?;

    let summary = ZonotopeSummary::from_zonotope(&z, "test", 10)?;
    // Should keep all 3 generators (k=10 > num_gens=3)
    assert_eq!(summary.generators.len(), 3);
    Ok(())
}

#[test]
fn test_summary_reconstruction() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::new(&[1.0f32, 2.0, 3.0], &device)?.unsqueeze(0)?;
    let z = Zonotope::new_from_epsilon(&center, 0.1, 3)?;

    let summary = ZonotopeSummary::from_zonotope(&z, "test", 3)?;
    let z_reconstructed = summary.to_zonotope(&device)?;

    assert_eq!(z_reconstructed.hidden_dim()?, 3);
    assert!(z_reconstructed.num_gens()? > 0);

    // Center should match
    let orig_c: Vec<f32> = z.center.flatten_all()?.to_vec1()?;
    let recon_c: Vec<f32> = z_reconstructed.center.flatten_all()?.to_vec1()?;
    for i in 0..3 {
        assert!((orig_c[i] - recon_c[i]).abs() < 1e-5);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Robust Aggregation Tests
// ---------------------------------------------------------------------------

#[test]
fn test_robust_aggregation_basic() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let config = CollectiveZonotopeConfig::default();
    let engine = CollectiveZonotopeEngine::with_device(&config, &device);

    let summaries: Vec<ZonotopeSummary> = (0..5)
        .map(|i| {
            let c = vec![i as f32, (i + 1) as f32, (i + 2) as f32];
            ZonotopeSummary {
                peer_id: format!("peer_{}", i),
                center: c,
                generators: vec![[0.1f32, 0.1, 0.1].to_vec()],
                volume_proxy: 0.3,
                trust_score: 1.0,
            }
        })
        .collect();

    let aggregated = engine.robust_aggregate(&summaries)?;
    assert_eq!(aggregated.hidden_dim()?, 3);
    assert!(aggregated.num_gens()? > 0);
    Ok(())
}

#[test]
fn test_robust_aggregation_byzantine_resistance() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let config = CollectiveZonotopeConfig::default();
    let engine = CollectiveZonotopeEngine::with_device(&config, &device);

    // 3 honest peers at [1,1,1], 2 Byzantine at [1000,1000,1000]
    let summaries: Vec<ZonotopeSummary> = vec![
        // Honest
        ZonotopeSummary {
            peer_id: "h1".into(),
            center: vec![1.0, 1.0, 1.0],
            generators: vec![[0.1, 0.1, 0.1].to_vec()],
            volume_proxy: 0.3,
            trust_score: 1.0,
        },
        ZonotopeSummary {
            peer_id: "h2".into(),
            center: vec![1.0, 1.0, 1.0],
            generators: vec![[0.1, 0.1, 0.1].to_vec()],
            volume_proxy: 0.3,
            trust_score: 1.0,
        },
        ZonotopeSummary {
            peer_id: "h3".into(),
            center: vec![1.0, 1.0, 1.0],
            generators: vec![[0.1, 0.1, 0.1].to_vec()],
            volume_proxy: 0.3,
            trust_score: 1.0,
        },
        // Byzantine
        ZonotopeSummary {
            peer_id: "b1".into(),
            center: vec![1000.0, 1000.0, 1000.0],
            generators: vec![[0.1, 0.1, 0.1].to_vec()],
            volume_proxy: 0.3,
            trust_score: 1.0,
        },
        ZonotopeSummary {
            peer_id: "b2".into(),
            center: vec![1000.0, 1000.0, 1000.0],
            generators: vec![[0.1, 0.1, 0.1].to_vec()],
            volume_proxy: 0.3,
            trust_score: 1.0,
        },
    ];

    let aggregated = engine.robust_aggregate(&summaries)?;
    let c: Vec<f32> = aggregated.center.flatten_all()?.to_vec1()?;

    // Geometric median should be closer to honest cluster
    for (d, &val) in c.iter().enumerate().take(3) {
        assert!(
            val < 100.0,
            "Byzantine resistance failed: center[{}] = {}",
            d,
            val
        );
    }
    Ok(())
}

#[test]
fn test_empty_aggregation_fails() -> candle_core::Result<()> {
    let config = CollectiveZonotopeConfig::default();
    let engine = CollectiveZonotopeEngine::new(&config);

    let result = engine.robust_aggregate(&[]);
    assert!(result.is_err());
    Ok(())
}

// ---------------------------------------------------------------------------
// Weiszfeld Median Tests
// ---------------------------------------------------------------------------

#[test]
fn test_weiszfeld_1d_median() -> candle_core::Result<()> {
    let config = CollectiveZonotopeConfig::default();
    let engine = CollectiveZonotopeEngine::new(&config);

    let c0: &[f32] = &[0.0f32];
    let c1: &[f32] = &[2.0];
    let c2: &[f32] = &[4.0];
    let centers: &[&[f32]] = &[c0, c1, c2];
    let weights = vec![1.0, 1.0, 1.0];

    let median = engine.weiszfeld_median(centers, &weights)?;
    // Median of [0, 2, 4] should be near 2
    assert!((median[0] - 2.0).abs() < 0.1);
    Ok(())
}

#[test]
fn test_weiszfeld_weighted() -> candle_core::Result<()> {
    let config = CollectiveZonotopeConfig::default();
    let engine = CollectiveZonotopeEngine::new(&config);

    // Heavy weight on [10, 10]
    let c0: &[f32] = &[0.0f32, 0.0];
    let c1: &[f32] = &[10.0, 10.0];
    let centers: &[&[f32]] = &[c0, c1];
    let weights = vec![0.1, 10.0];

    let median = engine.weiszfeld_median(centers, &weights)?;
    assert!(median[0] > 5.0, "Should be pulled toward heavy weight");
    assert!(median[1] > 5.0);
    Ok(())
}

#[test]
fn test_weiszfeld_2d() -> candle_core::Result<()> {
    let config = CollectiveZonotopeConfig::default();
    let engine = CollectiveZonotopeEngine::new(&config);

    let c0: &[f32] = &[0.0f32, 0.0];
    let c1: &[f32] = &[2.0, 0.0];
    let c2: &[f32] = &[1.0, 2.0];
    let centers: &[&[f32]] = &[c0, c1, c2];
    let weights = vec![1.0, 1.0, 1.0];

    let median = engine.weiszfeld_median(centers, &weights)?;
    assert!(median.len() == 2);
    // Fermat point of triangle — should be inside
    assert!(median[0] > 0.0 && median[0] < 2.0);
    assert!(median[1] >= 0.0 && median[1] < 2.0);
    Ok(())
}

// ---------------------------------------------------------------------------
// Trust-Weighted Fusion Tests
// ---------------------------------------------------------------------------

#[test]
fn test_trust_weighted_fusion() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let config = CollectiveZonotopeConfig::default();
    let engine = CollectiveZonotopeEngine::with_device(&config, &device);

    let local_center = Tensor::new(&[1.0f32, 1.0], &device)?.unsqueeze(0)?;
    let local_z = Zonotope::new_from_epsilon(&local_center, 0.1, 2)?;

    let summaries = vec![
        ZonotopeSummary {
            peer_id: "p1".into(),
            center: vec![2.0, 2.0],
            generators: vec![[0.05, 0.05].to_vec()],
            volume_proxy: 0.1,
            trust_score: 0.9,
        },
        ZonotopeSummary {
            peer_id: "p2".into(),
            center: vec![3.0, 3.0],
            generators: vec![[0.05, 0.05].to_vec()],
            volume_proxy: 0.1,
            trust_score: 0.7,
        },
    ];

    let fused = engine.trust_weighted_fusion(&local_z, &summaries)?;
    assert_eq!(fused.hidden_dim()?, 2);
    Ok(())
}

#[test]
fn test_fusion_no_peers() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let config = CollectiveZonotopeConfig::default();
    let engine = CollectiveZonotopeEngine::with_device(&config, &device);

    let local_center = Tensor::new(&[1.0f32, 2.0], &device)?.unsqueeze(0)?;
    let local_z = Zonotope::new_from_epsilon(&local_center, 0.1, 2)?;

    let fused = engine.trust_weighted_fusion(&local_z, &[])?;
    // Should return local zonotope unchanged
    let c: Vec<f32> = fused.center.flatten_all()?.to_vec1()?;
    assert!((c[0] - 1.0).abs() < 1e-5);
    assert!((c[1] - 2.0).abs() < 1e-5);
    Ok(())
}

#[test]
fn test_fusion_high_trust_peer() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let config = CollectiveZonotopeConfig {
        local_trust: 0.1, // Low local trust
        ..Default::default()
    };
    let engine = CollectiveZonotopeEngine::with_device(&config, &device);

    let local_center = Tensor::new(&[0.0f32, 0.0], &device)?.unsqueeze(0)?;
    let local_z = Zonotope::new_from_epsilon(&local_center, 0.1, 2)?;

    let summaries = vec![ZonotopeSummary {
        peer_id: "trusted".into(),
        center: vec![10.0, 10.0],
        generators: vec![[0.05, 0.05].to_vec()],
        volume_proxy: 0.1,
        trust_score: 1.0,
    }];

    let fused = engine.trust_weighted_fusion(&local_z, &summaries)?;
    let c: Vec<f32> = fused.center.flatten_all()?.to_vec1()?;
    // With low local trust, should be pulled toward peer
    assert!(c[0] > 5.0, "Should be pulled toward trusted peer");
    Ok(())
}

// ---------------------------------------------------------------------------
// Consensus Verification Tests
// ---------------------------------------------------------------------------

#[test]
fn test_consensus_all_safe() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let config = CollectiveZonotopeConfig::default();
    let engine = CollectiveZonotopeEngine::with_device(&config, &device);

    // Safe centroid at origin, toxic far in positive direction
    // Zonotopes must be near origin with small epsilon so upper bound projection <= 0
    let safe = Tensor::zeros((1, 3), DType::F32, &device)?;
    let toxic = Tensor::new(&[10.0f32, 10.0, 10.0], &device)?.unsqueeze(0)?;

    // Place zonotopes very close to origin with tiny epsilon
    // so both direction_safe (proj <= 0) and distance_safe (dist <= cbf_beta)
    let summaries: Vec<ZonotopeSummary> = (0..5)
        .map(|i| {
            let center_val = -(i as f32 + 1.0) * 0.001;
            let z = Zonotope::new_from_epsilon(
                &Tensor::full(center_val, (1, 3), &device).unwrap(),
                0.001,
                2,
            )
            .unwrap();
            ZonotopeSummary::from_zonotope(&z, &format!("peer_{}", i), 2).unwrap()
        })
        .collect();

    let result = engine.consensus_verify(&summaries, &safe, &toxic, 0.1)?;
    assert_eq!(result.num_peers, 5);
    assert_eq!(result.num_safe, 5);
    assert_eq!(result.num_unsafe, 0);
    assert!(result.consensus);
    Ok(())
}

#[test]
fn test_consensus_empty() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let config = CollectiveZonotopeConfig::default();
    let engine = CollectiveZonotopeEngine::with_device(&config, &device);

    let safe = Tensor::zeros((1, 3), DType::F32, &device)?;
    let toxic = Tensor::ones((1, 3), DType::F32, &device)?;

    let result = engine.consensus_verify(&[], &safe, &toxic, 1.0)?;
    assert_eq!(result.num_peers, 0);
    assert!(result.consensus); // Empty = trivially safe
    Ok(())
}

#[test]
fn test_consensus_display() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let config = CollectiveZonotopeConfig::default();
    let engine = CollectiveZonotopeEngine::with_device(&config, &device);

    let safe = Tensor::zeros((1, 3), DType::F32, &device)?;
    let toxic = Tensor::ones((1, 3), DType::F32, &device)?;

    let z = Zonotope::new_from_epsilon(&safe, 0.01, 2)?;
    let summary = ZonotopeSummary::from_zonotope(&z, "p1", 2)?;

    let result = engine.consensus_verify(&[summary], &safe, &toxic, 1.0)?;
    let display = format!("{}", result);
    assert!(display.contains("consensus="));
    assert!(display.contains("peers="));
    Ok(())
}

// ---------------------------------------------------------------------------
// Gossip Compression Tests
// ---------------------------------------------------------------------------

#[test]
fn test_compress_for_gossip() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let config = CollectiveZonotopeConfig {
        gossip_gens: 16,
        ..Default::default()
    };
    let engine = CollectiveZonotopeEngine::with_device(&config, &device);

    let center = Tensor::zeros((1, 256), DType::F32, &device)?;
    let z = Zonotope::new_from_epsilon(&center, 0.05, 64)?;

    let summary = engine.compress_for_gossip(&z, "node_42")?;
    assert_eq!(summary.center.len(), 256);
    assert!(summary.generators.len() <= 16);
    assert_eq!(summary.peer_id, "node_42");
    Ok(())
}

#[test]
fn test_gossip_roundtrip() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let config = CollectiveZonotopeConfig::default();
    let engine = CollectiveZonotopeEngine::with_device(&config, &device);

    let center = Tensor::new(&[1.0f32, 2.0, 3.0], &device)?.unsqueeze(0)?;
    let z = Zonotope::new_from_epsilon(&center, 0.1, 3)?;

    let summary = engine.compress_for_gossip(&z, "roundtrip")?;
    let z_back = summary.to_zonotope(&device)?;

    assert_eq!(z_back.hidden_dim()?, 3);
    let c: Vec<f32> = z_back.center.flatten_all()?.to_vec1()?;
    assert!((c[0] - 1.0).abs() < 1e-5);
    Ok(())
}
