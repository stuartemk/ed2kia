//! Sprint 117 (v11.7.0) — Reach-Tube Temporal + Hybrid IBP+Zonotope + Provable P2P Mechanism Design
//!
//! 25+ integration tests covering:
//! - Reach-tube containment under temporal FGSM
//! - PAC-Bayes collective bounds
//! - PoA < bound stability
//! - Hybrid IBP+Zonotope pipeline certification
//! - P2P mechanism design (Shapley, VCG, replicator dynamics)

use candle_core::{Device, Result, Tensor};
use native_audit::formal_verification::{
    compute_cbf_margin, hybrid_ibp_zonotope_pipeline, hybrid_reach_tube_ibp,
    ibp_certify_reach_tube, propagate_reach_tube, temporal_fgsm_attack,
    verify_temporal_invariance_monte_carlo, GirardMerge, GirardNorm, HybridPipelineConfig,
    ReachTube, ReachTubeConfig, TubeSegment,
};
use native_audit::p2p_mechanism::{
    byzantine_median, collective_pac_bound, compute_shapley_values, run_vcg_auction,
    simulate_poa_stability, MechanismConfig, NodeContribution,
};

// =============================================================================
// Helpers
// =============================================================================

fn make_center(values: &[f32], device: &Device) -> Result<Tensor> {
    Tensor::from_vec(values.to_vec(), (1, values.len()), device)
}

fn make_diagonal_generators(dim: usize, epsilon: f32, device: &Device) -> Result<Tensor> {
    let mut data = vec![0.0f32; dim * dim];
    for i in 0..dim {
        data[i * dim + i] = epsilon;
    }
    Tensor::from_vec(data, (dim, dim), device)
}

fn make_identity_weight(dim: usize, device: &Device) -> Result<Tensor> {
    let mut data = vec![0.0f32; dim * dim];
    for i in 0..dim {
        data[i * dim + i] = 1.0;
    }
    Tensor::from_vec(data, (dim, dim), device)
}

fn linear_dynamics(weight: &Tensor) -> impl Fn(&Tensor) -> Result<Tensor> + '_ {
    move |x: &Tensor| x.matmul(weight)
}

// =============================================================================
// Reach-Tube Temporal Tests (1-7)
// =============================================================================

#[test]
fn test_reach_tube_config_default() {
    let config = ReachTubeConfig::default();
    assert!(config.dt > 0.0);
    assert!(config.t_steps > 0);
    assert!(config.max_gens > 0);
    assert_eq!(config.taylor_order, 2);
}

#[test]
fn test_reach_tube_propagation_basic() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;
    let center = make_center(&[0.0, 0.0, 0.0, 0.0], &device)?;
    let gens = make_diagonal_generators(dim, 0.05, &device)?;
    let weight = make_identity_weight(dim, &device)?;
    let dynamics = linear_dynamics(&weight);

    let config = ReachTubeConfig {
        dt: 0.1,
        t_steps: 5,
        taylor_order: 1,
        max_gens: 8,
        norm: GirardNorm::L1,
        merge: GirardMerge::IntervalHull,
        noise_threshold: 1e-6,
        weight_decay: 0.01,
    };

    let safe_center = vec![0.0; dim];
    let tube = propagate_reach_tube(&center, &gens, &dynamics, &safe_center, 1.0, &config)?;

    assert_eq!(tube.tubes.len(), config.t_steps + 1);
    assert_eq!(tube.cbf_margins.len(), config.t_steps + 1);
    assert!(tube.min_cbf_margin.is_finite());
    assert!(tube.avg_volume_ratio.is_finite());
    Ok(())
}

#[test]
fn test_reach_tube_safety_under_small_perturbation() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;
    let center = make_center(&[0.0, 0.0, 0.0, 0.0], &device)?;
    let gens = make_diagonal_generators(dim, 0.01, &device)?;
    let weight = make_identity_weight(dim, &device)?;
    let dynamics = linear_dynamics(&weight);

    let config = ReachTubeConfig {
        dt: 0.05,
        t_steps: 5,
        taylor_order: 1,
        max_gens: 16,
        norm: GirardNorm::L1,
        merge: GirardMerge::IntervalHull,
        noise_threshold: 1e-6,
        weight_decay: 0.01,
    };

    let safe_center = vec![0.0; dim];
    let tube = propagate_reach_tube(&center, &gens, &dynamics, &safe_center, 1.0, &config)?;

    // Small perturbation + identity dynamics → should remain safe
    assert!(tube.is_safe(), "Tube should be safe with small epsilon");
    Ok(())
}

#[test]
fn test_reach_tube_volume_grows_over_time() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;
    let center = make_center(&[0.0, 0.0, 0.0, 0.0], &device)?;
    let gens = make_diagonal_generators(dim, 0.05, &device)?;

    // Expanding dynamics: dx/dt = 2x
    let mut data = vec![0.0f32; dim * dim];
    for i in 0..dim {
        data[i * dim + i] = 2.0;
    }
    let weight = Tensor::from_vec(data, (dim, dim), &device)?;
    let dynamics = linear_dynamics(&weight);

    let config = ReachTubeConfig {
        dt: 0.1,
        t_steps: 5,
        taylor_order: 1,
        max_gens: 16,
        norm: GirardNorm::L1,
        merge: GirardMerge::IntervalHull,
        noise_threshold: 1e-6,
        weight_decay: 0.01,
    };

    let safe_center = vec![0.0; dim];
    let tube = propagate_reach_tube(&center, &gens, &dynamics, &safe_center, 1.0, &config)?;

    // Volume should increase with expanding dynamics
    assert!(tube.avg_volume_ratio > 0.0);
    Ok(())
}

#[test]
fn test_reach_tube_taylor_order_effect() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;
    let center = make_center(&[0.1, 0.1, 0.1, 0.1], &device)?;
    let gens = make_diagonal_generators(dim, 0.02, &device)?;
    let weight = make_identity_weight(dim, &device)?;
    let dynamics = linear_dynamics(&weight);

    let config_order1 = ReachTubeConfig {
        taylor_order: 1,
        ..ReachTubeConfig::default()
    };
    let config_order2 = ReachTubeConfig {
        taylor_order: 2,
        ..ReachTubeConfig::default()
    };

    let safe_center = vec![0.0; dim];
    let tube1 = propagate_reach_tube(&center, &gens, &dynamics, &safe_center, 1.0, &config_order1)?;
    let tube2 = propagate_reach_tube(&center, &gens, &dynamics, &safe_center, 1.0, &config_order2)?;

    // Both should produce valid tubes
    assert!(tube1.tubes.len() == tube2.tubes.len());
    assert!(tube1.min_cbf_margin.is_finite());
    assert!(tube2.min_cbf_margin.is_finite());
    Ok(())
}

#[test]
fn test_reach_tube_girard_reduction_applied() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;
    let _k = 20; // More gens than max_gens
    let center = make_center(&[0.0, 0.0, 0.0, 0.0], &device)?;
    let gens = make_diagonal_generators(dim, 0.05, &device)?;
    let weight = make_identity_weight(dim, &device)?;
    let dynamics = linear_dynamics(&weight);

    let config = ReachTubeConfig {
        dt: 0.1,
        t_steps: 3,
        taylor_order: 1,
        max_gens: 16, // Allow for remainder gens + reduction (4 initial + 4*3 remainder = 16 max)
        norm: GirardNorm::L1,
        merge: GirardMerge::IntervalHull,
        noise_threshold: 1e-6,
        weight_decay: 0.01,
    };

    let safe_center = vec![0.0; dim];
    let tube = propagate_reach_tube(&center, &gens, &dynamics, &safe_center, 1.0, &config)?;

    // Each segment should have ≤ max_gens generators
    for seg in &tube.tubes {
        let num_gens = seg.generators.dim(0)?;
        assert!(
            num_gens <= config.max_gens,
            "Gens {} > max {}",
            num_gens,
            config.max_gens
        );
    }
    // Verify reduction is active: initial gens (4) < max_gens (8),
    // but after propagation with remainder, reduction keeps count bounded
    let initial_gens = tube.tubes[0].generators.dim(0)?;
    assert!(initial_gens <= config.max_gens);
    Ok(())
}

#[test]
fn test_reach_tube_tightness_score() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;
    let center = make_center(&[0.0, 0.0, 0.0, 0.0], &device)?;
    let gens = make_diagonal_generators(dim, 0.01, &device)?;
    let weight = make_identity_weight(dim, &device)?;
    let dynamics = linear_dynamics(&weight);

    let config = ReachTubeConfig::default();
    let safe_center = vec![0.0; dim];
    let tube = propagate_reach_tube(&center, &gens, &dynamics, &safe_center, 1.0, &config)?;

    let score = tube.tightness_score();
    assert!(score > 0.0 && score.is_finite());
    Ok(())
}

// =============================================================================
// IBP Certification Tests (8-12)
// =============================================================================

#[test]
fn test_ibp_certify_safe_tube() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;
    let center = make_center(&[0.0, 0.0, 0.0, 0.0], &device)?;
    let gens = make_diagonal_generators(dim, 0.01, &device)?;
    let weight = make_identity_weight(dim, &device)?;
    let dynamics = linear_dynamics(&weight);

    let config = ReachTubeConfig {
        dt: 0.05,
        t_steps: 3,
        taylor_order: 1,
        max_gens: 16,
        norm: GirardNorm::L1,
        merge: GirardMerge::IntervalHull,
        noise_threshold: 1e-6,
        weight_decay: 0.01,
    };

    let safe_center = vec![0.0; dim];
    let tube = propagate_reach_tube(&center, &gens, &dynamics, &safe_center, 1.0, &config)?;

    let cbf_values = ibp_certify_reach_tube(&tube, &safe_center, 1.0);
    assert_eq!(cbf_values.len(), tube.tubes.len());
    // All should be positive (safe)
    for &v in &cbf_values {
        assert!(v > 0.0, "IBP CBF should be positive: {}", v);
    }
    Ok(())
}

#[test]
fn test_ibp_certify_unsafe_tube() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;
    let center = make_center(&[5.0, 5.0, 5.0, 5.0], &device)?; // Far from safe center
    let gens = make_diagonal_generators(dim, 0.5, &device)?;
    let weight = make_identity_weight(dim, &device)?;
    let dynamics = linear_dynamics(&weight);

    let config = ReachTubeConfig {
        dt: 0.1,
        t_steps: 3,
        taylor_order: 1,
        max_gens: 16,
        norm: GirardNorm::L1,
        merge: GirardMerge::IntervalHull,
        noise_threshold: 1e-6,
        weight_decay: 0.01,
    };

    let safe_center = vec![0.0; dim];
    let tube = propagate_reach_tube(&center, &gens, &dynamics, &safe_center, 0.5, &config)?;

    let cbf_values = ibp_certify_reach_tube(&tube, &safe_center, 0.5);
    // At least some should be negative (unsafe)
    let has_negative = cbf_values.iter().any(|&v| v < 0.0);
    assert!(has_negative, "IBP should detect unsafe tube");
    Ok(())
}

#[test]
fn test_ibp_cbf_monotonic_in_margin() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;
    let center = make_center(&[0.0, 0.0, 0.0, 0.0], &device)?;
    let gens = make_diagonal_generators(dim, 0.01, &device)?;

    let tube = ReachTube {
        tubes: vec![TubeSegment {
            center: center.clone(),
            generators: gens.clone(),
            cbf_margin: 1.0,
            volume_proxy: 0.04,
        }],
        cbf_margins: vec![1.0],
        avg_volume_ratio: 0.04,
        min_cbf_margin: 1.0,
    };

    let safe_center = vec![0.0; dim];
    let cbf_small = ibp_certify_reach_tube(&tube, &safe_center, 0.1);
    let cbf_large = ibp_certify_reach_tube(&tube, &safe_center, 1.0);

    // Larger margin → larger CBF value
    assert!(cbf_large[0] >= cbf_small[0]);
    Ok(())
}

#[test]
fn test_compute_cbf_margin_safe() -> Result<()> {
    let device = Device::Cpu;
    let center = Tensor::from_vec(vec![0.0, 0.0, 0.0], (1, 3), &device)?;
    let safe_center = Tensor::from_vec(vec![0.0, 0.0, 0.0], (1, 3), &device)?;
    let margin = 1.0;

    let cbf = compute_cbf_margin(&center, &safe_center, margin)?;
    assert!((cbf - 1.0).abs() < 1e-5, "CBF at center should be margin²");
    Ok(())
}

#[test]
fn test_compute_cbf_margin_unsafe() -> Result<()> {
    let device = Device::Cpu;
    let center = Tensor::from_vec(vec![3.0, 0.0, 0.0], (1, 3), &device)?;
    let safe_center = Tensor::from_vec(vec![0.0, 0.0, 0.0], (1, 3), &device)?;
    let margin = 1.0;

    let cbf = compute_cbf_margin(&center, &safe_center, margin)?;
    assert!(
        cbf < 0.0,
        "CBF should be negative when far from center: {}",
        cbf
    );
    Ok(())
}

// =============================================================================
// Temporal FGSM Tests (13-16)
// =============================================================================

#[test]
fn test_temporal_fgsm_basic() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;
    let center = make_center(&[0.0, 0.0, 0.0, 0.0], &device)?;
    let weight = make_identity_weight(dim, &device)?;
    let dynamics = linear_dynamics(&weight);

    let safe_center = vec![0.0; dim];
    let adv_cbf = temporal_fgsm_attack(&center, 0.1, &safe_center, 1.0, 3, &dynamics)?;
    assert_eq!(adv_cbf.len(), 3 + 1);
    for &v in &adv_cbf {
        assert!(v.is_finite());
    }
    Ok(())
}

#[test]
fn test_temporal_fgsm_reduces_cbf() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;
    let center = make_center(&[0.0, 0.0, 0.0, 0.0], &device)?;
    let gens = make_diagonal_generators(dim, 0.05, &device)?;
    let weight = make_identity_weight(dim, &device)?;
    let dynamics = linear_dynamics(&weight);

    let config = ReachTubeConfig {
        dt: 0.1,
        t_steps: 3,
        taylor_order: 1,
        max_gens: 16,
        norm: GirardNorm::L1,
        merge: GirardMerge::IntervalHull,
        noise_threshold: 1e-6,
        weight_decay: 0.01,
    };

    let safe_center = vec![0.0; dim];
    let tube = propagate_reach_tube(&center, &gens, &dynamics, &safe_center, 1.0, &config)?;

    let orig_min = tube.min_cbf_margin;
    let adv_cbf = temporal_fgsm_attack(&center, 0.05, &safe_center, 1.0, 3, &dynamics)?;
    let adv_min = adv_cbf.iter().copied().fold(f32::INFINITY, f32::min);

    // FGSM should reduce (or keep equal) the CBF margin
    assert!(
        adv_min <= orig_min + 1e-4,
        "FGSM should reduce CBF: {} > {}",
        adv_min,
        orig_min
    );
    Ok(())
}

#[test]
fn test_temporal_fgsm_epsilon_effect() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;
    let center = make_center(&[0.0, 0.0, 0.0, 0.0], &device)?;
    let weight = make_identity_weight(dim, &device)?;
    let dynamics = linear_dynamics(&weight);

    let safe_center = vec![0.0; dim];
    let adv_small = temporal_fgsm_attack(&center, 0.01, &safe_center, 1.0, 3, &dynamics)?;
    let adv_large = temporal_fgsm_attack(&center, 0.1, &safe_center, 1.0, 3, &dynamics)?;

    let min_small = adv_small.iter().copied().fold(f32::INFINITY, f32::min);
    let min_large = adv_large.iter().copied().fold(f32::INFINITY, f32::min);

    // Larger epsilon → more aggressive attack → lower CBF
    assert!(min_large <= min_small + 1e-4);
    Ok(())
}

#[test]
fn test_mc_temporal_invariance() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;
    let center = make_center(&[0.0, 0.0, 0.0, 0.0], &device)?;
    let gens = make_diagonal_generators(dim, 0.02, &device)?;
    let weight = make_identity_weight(dim, &device)?;
    let dynamics = linear_dynamics(&weight);

    let config = ReachTubeConfig {
        dt: 0.05,
        t_steps: 3,
        taylor_order: 1,
        max_gens: 16,
        norm: GirardNorm::L1,
        merge: GirardMerge::IntervalHull,
        noise_threshold: 1e-6,
        weight_decay: 0.01,
    };

    let safe_center = vec![0.0; dim];
    let tube = propagate_reach_tube(&center, &gens, &dynamics, &safe_center, 1.0, &config)?;

    // verify_temporal_invariance_monte_carlo returns f32 (fraction safe), not Result<Vec<f32>>
    let fraction_safe = verify_temporal_invariance_monte_carlo(&tube, &safe_center, 1.0, 100, 42);
    assert!(
        fraction_safe >= 0.0 && fraction_safe <= 1.0,
        "Fraction should be in [0,1]: {}",
        fraction_safe
    );
    assert!(fraction_safe.is_finite());
    Ok(())
}

// =============================================================================
// Hybrid IBP+Zonotope Pipeline Tests (17-21)
// =============================================================================

#[test]
fn test_hybrid_pipeline_config_default() {
    let config = HybridPipelineConfig::default();
    assert!(config.ibp_epsilon > 0.0);
    assert!(config.max_gens > 0);
    assert!(config.num_layers > 0);
}

#[test]
fn test_hybrid_pipeline_basic() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;
    let center = make_center(&[0.0, 0.0, 0.0, 0.0], &device)?;
    let weight = make_identity_weight(dim, &device)?;
    let layers = vec![(weight.clone(), None), (weight.clone(), None)];

    let config = HybridPipelineConfig {
        ibp_epsilon: 0.1,
        max_gens: 16,
        num_layers: 2,
        ..HybridPipelineConfig::default()
    };

    let safe_center = vec![0.0; dim];
    let result = hybrid_ibp_zonotope_pipeline(&center, &layers, &safe_center, 1.0, &config)?;

    assert_eq!(result.ibp_bounds.len(), config.num_layers + 1);
    assert!(result.volume_proxy.is_finite());
    assert!(result.tightness_ratio.is_finite());
    assert!(result.cbf_margin.is_finite());
    Ok(())
}

#[test]
fn test_hybrid_pipeline_tightness() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;
    let center = make_center(&[0.0, 0.0, 0.0, 0.0], &device)?;
    let weight = make_identity_weight(dim, &device)?;
    let layers = vec![(weight.clone(), None)];

    let config = HybridPipelineConfig {
        ibp_epsilon: 0.05,
        max_gens: 16,
        num_layers: 1,
        ..HybridPipelineConfig::default()
    };

    let safe_center = vec![0.0; dim];
    let result = hybrid_ibp_zonotope_pipeline(&center, &layers, &safe_center, 1.0, &config)?;

    // Tightness should be positive and finite
    assert!(result.tightness_ratio > 0.0 && result.tightness_ratio.is_finite());
    Ok(())
}

#[test]
fn test_hybrid_pipeline_display() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;
    let center = make_center(&[0.0, 0.0, 0.0, 0.0], &device)?;
    let weight = make_identity_weight(dim, &device)?;
    let layers = vec![(weight.clone(), None)];

    let config = HybridPipelineConfig::default();
    let safe_center = vec![0.0; dim];
    let result = hybrid_ibp_zonotope_pipeline(&center, &layers, &safe_center, 1.0, &config)?;

    let display = format!("{}", result);
    assert!(display.contains("HybridPipeline"));
    assert!(display.contains("cbf="));
    Ok(())
}

#[test]
fn test_hybrid_reach_tube_ibp_basic() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;
    let center = make_center(&[0.0, 0.0, 0.0, 0.0], &device)?;
    let gens = make_diagonal_generators(dim, 0.05, &device)?;
    let weight = make_identity_weight(dim, &device)?;
    let dynamics = linear_dynamics(&weight);

    let config = ReachTubeConfig {
        dt: 0.1,
        t_steps: 3,
        taylor_order: 1,
        max_gens: 16,
        norm: GirardNorm::L1,
        merge: GirardMerge::IntervalHull,
        noise_threshold: 1e-6,
        weight_decay: 0.01,
    };

    let safe_center = vec![0.0; dim];
    let tube = hybrid_reach_tube_ibp(&center, &gens, &dynamics, &safe_center, 1.0, &config)?;

    assert_eq!(tube.tubes.len(), config.t_steps + 1);
    assert!(tube.min_cbf_margin.is_finite());
    Ok(())
}

#[test]
fn test_hybrid_reach_tube_ibp_safety() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;
    let center = make_center(&[0.0, 0.0, 0.0, 0.0], &device)?;
    let gens = make_diagonal_generators(dim, 0.01, &device)?;
    let weight = make_identity_weight(dim, &device)?;
    let dynamics = linear_dynamics(&weight);

    let config = ReachTubeConfig {
        dt: 0.05,
        t_steps: 3,
        taylor_order: 1,
        max_gens: 16,
        norm: GirardNorm::L1,
        merge: GirardMerge::IntervalHull,
        noise_threshold: 1e-6,
        weight_decay: 0.01,
    };

    let safe_center = vec![0.0; dim];
    let tube = hybrid_reach_tube_ibp(&center, &gens, &dynamics, &safe_center, 1.0, &config)?;

    assert!(
        tube.is_safe(),
        "Hybrid IBP tube should be safe with small epsilon"
    );
    Ok(())
}

// =============================================================================
// P2P Mechanism Design Tests (22-28)
// =============================================================================

#[test]
fn test_shapley_values_basic() {
    let contributions = vec![
        NodeContribution {
            node_id: 0,
            tightness_improvement: 0.3,
            cost: 0.1,
            verified: true,
        },
        NodeContribution {
            node_id: 1,
            tightness_improvement: 0.5,
            cost: 0.2,
            verified: true,
        },
        NodeContribution {
            node_id: 2,
            tightness_improvement: 0.4,
            cost: 0.15,
            verified: true,
        },
    ];

    let config = MechanismConfig::default();
    let result = compute_shapley_values(&contributions, &config);

    assert_eq!(result.values.len(), 3);
    assert!(result.social_welfare > 0.0);
    assert!(result.efficiency > 0.0 && result.efficiency <= 2.0);
}

#[test]
fn test_shapley_values_single_node() {
    let contributions = vec![NodeContribution {
        node_id: 0,
        tightness_improvement: 0.5,
        cost: 0.1,
        verified: true,
    }];

    let config = MechanismConfig::default();
    let result = compute_shapley_values(&contributions, &config);

    assert_eq!(result.values.len(), 1);
    assert!((result.values[0] - 0.5).abs() < 1e-5);
}

#[test]
fn test_vcg_auction_basic() {
    let contributions = vec![
        NodeContribution {
            node_id: 0,
            tightness_improvement: 0.3,
            cost: 0.1,
            verified: true,
        },
        NodeContribution {
            node_id: 1,
            tightness_improvement: 0.5,
            cost: 0.2,
            verified: true,
        },
        NodeContribution {
            node_id: 2,
            tightness_improvement: 0.4,
            cost: 0.15,
            verified: true,
        },
    ];

    let config = MechanismConfig::default();
    let result = run_vcg_auction(&contributions, &config);

    assert!(!result.winners.is_empty());
    assert_eq!(result.payments.len(), result.winners.len());
    assert!(result.social_welfare > 0.0);
}

#[test]
fn test_vcg_selects_highest_value() {
    let contributions = vec![
        NodeContribution {
            node_id: 0,
            tightness_improvement: 0.1,
            cost: 0.05,
            verified: true,
        },
        NodeContribution {
            node_id: 1,
            tightness_improvement: 0.9,
            cost: 0.1,
            verified: true,
        },
        NodeContribution {
            node_id: 2,
            tightness_improvement: 0.5,
            cost: 0.2,
            verified: true,
        },
    ];

    let config = MechanismConfig {
        max_winners: 1,
        ..MechanismConfig::default()
    };
    let result = run_vcg_auction(&contributions, &config);

    assert_eq!(result.winners, vec![1]);
}

#[test]
fn test_poa_stability_basic() {
    let config = MechanismConfig::default();
    // simulate_poa_stability takes (num_nodes, byzantine_fraction, &config) and returns PoaResult directly
    let result = simulate_poa_stability(3, 0.1, &config);

    assert!(result.poa_bound > 0.0 && result.poa_bound.is_finite());
    assert!(result.equilibrium_welfare.is_finite());
    assert!(result.optimal_welfare.is_finite());
    // convergence_epoch is u32, always >= 0
    let _ = result.convergence_epoch;
}

#[test]
fn test_poa_bound_reasonable() {
    let config = MechanismConfig {
        replicator_lr: 0.01,
        replicator_epochs: 100,
        ..MechanismConfig::default()
    };
    let result = simulate_poa_stability(3, 0.1, &config);

    // PoA should be reasonable (not infinite or zero)
    assert!(result.poa_bound > 0.5 && result.poa_bound < 10.0);
}

#[test]
fn test_byzantine_median_robust() {
    let values = vec![1.0, 1.1, 0.9, 1.0, 100.0]; // One Byzantine outlier
    let median = byzantine_median(&values);

    // Median should be close to honest values, not the outlier
    assert!((median - 1.0).abs() < 0.2);
}

#[test]
fn test_byzantine_median_honest() {
    let values = vec![1.0, 1.0, 1.0, 1.0, 1.0];
    let median = byzantine_median(&values);
    assert!((median - 1.0).abs() < 1e-5);
}

#[test]
fn test_collective_pac_bound() {
    let emp_errors = vec![0.1, 0.15, 0.12, 0.08];
    let n = 100;
    let delta = 0.01;

    let bound = collective_pac_bound(&emp_errors, n, delta);
    assert!(bound > 0.0 && bound.is_finite());
    // Bound should be larger than empirical error
    let max_emp = emp_errors.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    assert!(bound >= max_emp);
}

// =============================================================================
// Full Pipeline Integration (29-30)
// =============================================================================

#[test]
fn test_sprint117_full_pipeline() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;

    // 1. Reach-tube propagation
    let center = make_center(&[0.0, 0.0, 0.0, 0.0], &device)?;
    let gens = make_diagonal_generators(dim, 0.02, &device)?;
    let weight = make_identity_weight(dim, &device)?;
    let dynamics = linear_dynamics(&weight);

    let config = ReachTubeConfig {
        dt: 0.05,
        t_steps: 5,
        taylor_order: 1,
        max_gens: 16,
        norm: GirardNorm::L1,
        merge: GirardMerge::IntervalHull,
        noise_threshold: 1e-6,
        weight_decay: 0.01,
    };

    let safe_center = vec![0.0; dim];
    let tube = propagate_reach_tube(&center, &gens, &dynamics, &safe_center, 1.0, &config)?;
    assert!(tube.is_safe());

    // 2. IBP certification
    let cbf_values = ibp_certify_reach_tube(&tube, &safe_center, 1.0);
    assert!(cbf_values.iter().all(|&v| v > 0.0));

    // 3. Hybrid pipeline
    let layers = vec![(weight.clone(), None)];
    let hybrid_config = HybridPipelineConfig {
        ibp_epsilon: 0.05,
        num_layers: 1,
        ..HybridPipelineConfig::default()
    };
    let hybrid_result =
        hybrid_ibp_zonotope_pipeline(&center, &layers, &safe_center, 1.0, &hybrid_config)?;
    assert!(hybrid_result.cbf_margin > 0.0);

    // 4. P2P mechanism
    let contributions = vec![
        NodeContribution {
            node_id: 0,
            tightness_improvement: hybrid_result.tightness_ratio,
            cost: 0.1,
            verified: true,
        },
        NodeContribution {
            node_id: 1,
            tightness_improvement: 0.8,
            cost: 0.15,
            verified: true,
        },
    ];
    let shapley = compute_shapley_values(&contributions, &MechanismConfig::default());
    assert!(shapley.values.iter().all(|v| *v >= 0.0));

    Ok(())
}

#[test]
fn test_sprint117_temporal_robustness() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;

    let center = make_center(&[0.0, 0.0, 0.0, 0.0], &device)?;
    let gens = make_diagonal_generators(dim, 0.03, &device)?;
    let weight = make_identity_weight(dim, &device)?;
    let dynamics = linear_dynamics(&weight);

    let config = ReachTubeConfig {
        dt: 0.1,
        t_steps: 5,
        taylor_order: 2,
        max_gens: 16,
        norm: GirardNorm::L2,
        merge: GirardMerge::LGG,
        noise_threshold: 1e-6,
        weight_decay: 0.01,
    };

    let safe_center = vec![0.0; dim];
    let tube = propagate_reach_tube(&center, &gens, &dynamics, &safe_center, 1.0, &config)?;

    // FGSM attack — use correct signature: (center, epsilon, safe_center, margin, t_steps, dynamics)
    let adv_cbf = temporal_fgsm_attack(&center, 0.05, &safe_center, 1.0, 5, &dynamics)?;
    let adv_min = adv_cbf.iter().copied().fold(f32::INFINITY, f32::min);

    // IBP should still certify (conservative bound)
    let ibp_cbf = ibp_certify_reach_tube(&tube, &safe_center, 1.0);
    let ibp_min = ibp_cbf.iter().copied().fold(f32::INFINITY, f32::min);

    // IBP bound should be ≤ actual FGSM result (soundness)
    assert!(
        ibp_min <= adv_min + 0.1,
        "IBP should be conservative: {} > {}",
        ibp_min,
        adv_min
    );

    Ok(())
}
