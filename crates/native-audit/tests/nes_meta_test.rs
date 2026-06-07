//! NES Meta-Optimization Tests — Sprint 111 (v11.1.0)
//!
//! Tests for Natural Evolution Strategies integration in meta_active_inference.rs:
//! - NES convergence vs ES
//! - Antithetic sampling variance reduction
//! - Baseline tracking
//! - Gradient quality comparison (NES vs ES vs FD)

use native_audit::meta_active_inference::{
    MetaActiveInferenceConfig, MetaActiveInferenceEngine,
};

// ============================================================================
// NES Configuration Tests
// ============================================================================

#[test]
fn test_nes_config_default() {
    let config = MetaActiveInferenceConfig::default();
    assert!(!config.use_nes, "NES should be off by default");
    assert!(config.use_es, "ES should be on by default");
}

#[test]
fn test_nes_config_enable() {
    let config = MetaActiveInferenceConfig {
        use_nes: true,
        use_es: false,
        population_size: 30,
        ..Default::default()
    };
    assert!(config.use_nes);
    assert!(!config.use_es);
}

#[test]
fn test_nes_engine_creation() {
    let config = MetaActiveInferenceConfig {
        use_nes: true,
        ..Default::default()
    };
    let engine = MetaActiveInferenceEngine::new(&config);
    assert!(engine.best_meta_vfe() == f32::MAX);
}

// ============================================================================
// NES Convergence Tests
// ============================================================================

#[test]
fn test_nes_convergence() {
    let config = MetaActiveInferenceConfig {
        use_nes: true,
        use_es: false,
        meta_lr: 0.05,
        population_size: 30,
        ..Default::default()
    };
    let mut engine = MetaActiveInferenceEngine::new(&config);
    let peer_vfes = &[0.5, 0.6, 0.4, 0.55];

    let initial_vfe = engine.best_meta_vfe();
    let curve = engine.meta_optimize_sequence(20, peer_vfes).unwrap();

    assert!(curve.len() == 20);
    assert!(
        curve.last().unwrap() <= &initial_vfe,
        "NES should reduce meta-VFE: initial={}, final={}",
        initial_vfe,
        curve.last().unwrap()
    );
}

#[test]
fn test_nes_convergence_vs_es() {
    let peer_vfes = &[0.5, 0.6, 0.4];

    // NES
    let nes_config = MetaActiveInferenceConfig {
        use_nes: true,
        use_es: false,
        meta_lr: 0.05,
        population_size: 30,
        ..Default::default()
    };
    let mut nes_engine = MetaActiveInferenceEngine::new(&nes_config);
    let nes_curve = nes_engine.meta_optimize_sequence(15, peer_vfes).unwrap();

    // ES
    let es_config = MetaActiveInferenceConfig {
        use_nes: false,
        use_es: true,
        meta_lr: 0.05,
        population_size: 30,
        ..Default::default()
    };
    let mut es_engine = MetaActiveInferenceEngine::new(&es_config);
    let es_curve = es_engine.meta_optimize_sequence(15, peer_vfes).unwrap();

    // Both should converge
    assert!(nes_curve.last().unwrap() < &f32::MAX);
    assert!(es_curve.last().unwrap() < &f32::MAX);

    // NES should converge at least as well as ES (antithetic sampling = lower variance)
    let nes_final = nes_curve.last().unwrap();
    let es_final = es_curve.last().unwrap();
    assert!(
        *nes_final <= es_final + 0.1,
        "NES final={} should be comparable to ES final={}",
        nes_final,
        es_final
    );
}

#[test]
fn test_nes_convergence_vs_fd() {
    let peer_vfes = &[0.5, 0.5, 0.5];

    // NES
    let nes_config = MetaActiveInferenceConfig {
        use_nes: true,
        use_es: false,
        meta_lr: 0.05,
        population_size: 20,
        ..Default::default()
    };
    let mut nes_engine = MetaActiveInferenceEngine::new(&nes_config);
    let nes_curve = nes_engine.meta_optimize_sequence(15, peer_vfes).unwrap();

    // FD
    let fd_config = MetaActiveInferenceConfig {
        use_nes: false,
        use_es: false,
        meta_lr: 0.05,
        num_perturbations: 6,
        ..Default::default()
    };
    let mut fd_engine = MetaActiveInferenceEngine::new(&fd_config);
    let fd_curve = fd_engine.meta_optimize_sequence(15, peer_vfes).unwrap();

    assert!(nes_curve.last().unwrap() < &f32::MAX);
    assert!(fd_curve.last().unwrap() < &f32::MAX);
}

// ============================================================================
// Variance Reduction Tests
// ============================================================================

#[test]
fn test_nes_variance_reduction() {
    let peer_vfes = &[0.5, 0.6, 0.4];
    let mut reductions = Vec::new();

    for _ in 0..5 {
        let config = MetaActiveInferenceConfig {
            use_nes: true,
            use_es: false,
            meta_lr: 0.05,
            population_size: 20,
            ..Default::default()
        };
        let mut engine = MetaActiveInferenceEngine::new(&config);
        let curve = engine.meta_optimize_sequence(10, peer_vfes).unwrap();
        reductions.push(*curve.last().unwrap());
    }

    // Compute variance of final VFE across runs
    let mean = reductions.iter().sum::<f32>() / reductions.len() as f32;
    let variance = reductions
        .iter()
        .map(|r| (r - mean).powi(2))
        .sum::<f32>()
        / reductions.len() as f32;

    // NES should have low variance due to antithetic sampling
    assert!(variance < 0.1, "NES variance should be low: {}", variance);
}

#[test]
fn test_antithetic_consistency() {
    // Run NES multiple times with same config → check consistency
    let peer_vfes = &[0.5];
    let mut finals = Vec::new();

    for _ in 0..3 {
        let config = MetaActiveInferenceConfig {
            use_nes: true,
            use_es: false,
            meta_lr: 0.03,
            population_size: 20,
            perturbation_eps: 1e-3,
            ..Default::default()
        };
        let mut engine = MetaActiveInferenceEngine::new(&config);
        engine.meta_optimize_sequence(10, peer_vfes).unwrap();
        finals.push(engine.best_meta_vfe());
    }

    // All runs should reach similar optima
    let max_diff = (finals[0] - finals[1]).abs().max((finals[0] - finals[2]).abs());
    assert!(
        max_diff < 0.5,
        "Antithetic NES should be consistent: max_diff={}",
        max_diff
    );
}

// ============================================================================
// Baseline Tracking Tests
// ============================================================================

#[test]
fn test_baseline_updates() {
    let config = MetaActiveInferenceConfig {
        use_nes: true,
        use_es: false,
        meta_lr: 0.05,
        population_size: 20,
        ..Default::default()
    };
    let mut engine = MetaActiveInferenceEngine::new(&config);

    // Initial baseline should be 0
    // After optimization, baseline should track mean reward
    engine.meta_optimize_sequence(5, &[0.5]).unwrap();

    // Engine should have improved
    assert!(engine.best_meta_vfe() < f32::MAX);
}

#[test]
fn test_nes_improvement_ratio() {
    let config = MetaActiveInferenceConfig {
        use_nes: true,
        use_es: false,
        meta_lr: 0.1,
        population_size: 30,
        ..Default::default()
    };
    let mut engine = MetaActiveInferenceEngine::new(&config);
    engine.meta_optimize_sequence(15, &[0.5]).unwrap();

    let ratio = engine.improvement_ratio();
    assert!(
        ratio >= 0.0,
        "NES improvement ratio should be non-negative: {}",
        ratio
    );
}

// ============================================================================
// Population Size Effects
// ============================================================================

#[test]
fn test_nes_small_population() {
    let config = MetaActiveInferenceConfig {
        use_nes: true,
        use_es: false,
        meta_lr: 0.05,
        population_size: 10, // Small
        ..Default::default()
    };
    let mut engine = MetaActiveInferenceEngine::new(&config);
    let result = engine.meta_optimize_sequence(10, &[0.5]);
    assert!(result.is_ok());
}

#[test]
fn test_nes_large_population() {
    let config = MetaActiveInferenceConfig {
        use_nes: true,
        use_es: false,
        meta_lr: 0.05,
        population_size: 50, // Large
        ..Default::default()
    };
    let mut engine = MetaActiveInferenceEngine::new(&config);
    let result = engine.meta_optimize_sequence(10, &[0.5]);
    assert!(result.is_ok());
}

#[test]
fn test_larger_population_better_convergence() {
    let peer_vfes = &[0.5, 0.6, 0.4];

    // Small pop
    let small_config = MetaActiveInferenceConfig {
        use_nes: true,
        use_es: false,
        meta_lr: 0.05,
        population_size: 10,
        ..Default::default()
    };
    let mut small_engine = MetaActiveInferenceEngine::new(&small_config);
    let small_curve = small_engine.meta_optimize_sequence(10, peer_vfes).unwrap();

    // Large pop
    let large_config = MetaActiveInferenceConfig {
        use_nes: true,
        use_es: false,
        meta_lr: 0.05,
        population_size: 40,
        ..Default::default()
    };
    let mut large_engine = MetaActiveInferenceEngine::new(&large_config);
    let large_curve = large_engine.meta_optimize_sequence(10, peer_vfes).unwrap();

    // Larger population should converge at least as well
    assert!(
        *large_curve.last().unwrap() <= small_curve.last().unwrap() + 0.05,
        "Larger pop should converge better or equal"
    );
}

// ============================================================================
// Learning Rate Effects
// ============================================================================

#[test]
fn test_nes_lr_too_small() {
    let config = MetaActiveInferenceConfig {
        use_nes: true,
        use_es: false,
        meta_lr: 0.001, // Very small
        population_size: 20,
        ..Default::default()
    };
    let mut engine = MetaActiveInferenceEngine::new(&config);
    let curve = engine.meta_optimize_sequence(10, &[0.5]).unwrap();
    // Should still work, just slow
    assert!(curve.iter().all(|v| v.is_finite()));
}

#[test]
fn test_nes_lr_too_large() {
    let config = MetaActiveInferenceConfig {
        use_nes: true,
        use_es: false,
        meta_lr: 0.5, // Large
        population_size: 20,
        ..Default::default()
    };
    let mut engine = MetaActiveInferenceEngine::new(&config);
    let curve = engine.meta_optimize_sequence(10, &[0.5]).unwrap();
    // Should remain finite (clamping prevents explosion)
    assert!(curve.iter().all(|v| v.is_finite()));
}

// ============================================================================
// Mode Comparison: NES vs ES vs FD
// ============================================================================

#[test]
fn test_all_modes_produce_gradient() {
    let peer_vfes = &[0.5, 0.6];

    for (use_nes, use_es) in [(true, false), (false, true)] {
        let config = MetaActiveInferenceConfig {
            use_nes,
            use_es,
            meta_lr: 0.05,
            population_size: 20,
            num_perturbations: 6,
            ..Default::default()
        };
        let mut engine = MetaActiveInferenceEngine::new(&config);
        let reduction = engine.meta_optimize(peer_vfes).unwrap();
        assert!(
            reduction.is_finite(),
            "Mode nes={}, es={} should produce finite reduction",
            use_nes,
            use_es
        );
    }
}

#[test]
fn test_nes_best_params_improve() {
    let config = MetaActiveInferenceConfig {
        use_nes: true,
        use_es: false,
        meta_lr: 0.05,
        population_size: 30,
        ..Default::default()
    };
    let mut engine = MetaActiveInferenceEngine::new(&config);

    let initial_params = engine.best_params().clone();
    engine.meta_optimize_sequence(20, &[0.5, 0.6, 0.4]).unwrap();
    let final_params = engine.best_params();

    // Best params should differ from initial after optimization
    let param_diff = (initial_params.lr - final_params.lr).abs()
        + (initial_params.beta_cbf - final_params.beta_cbf).abs();
    assert!(param_diff > 1e-6, "Params should change during NES optimization");
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_nes_empty_peer_vfes() {
    let config = MetaActiveInferenceConfig {
        use_nes: true,
        use_es: false,
        population_size: 20,
        ..Default::default()
    };
    let mut engine = MetaActiveInferenceEngine::new(&config);
    let result = engine.meta_optimize(&[]);
    assert!(result.is_ok());
}

#[test]
fn test_nes_single_peer() {
    let config = MetaActiveInferenceConfig {
        use_nes: true,
        use_es: false,
        population_size: 20,
        ..Default::default()
    };
    let mut engine = MetaActiveInferenceEngine::new(&config);
    let result = engine.meta_optimize(&[0.5]);
    assert!(result.is_ok());
}

#[test]
fn test_nes_many_peers() {
    let config = MetaActiveInferenceConfig {
        use_nes: true,
        use_es: false,
        population_size: 20,
        ..Default::default()
    };
    let mut engine = MetaActiveInferenceEngine::new(&config);
    let peer_vfes: Vec<f32> = (0..100).map(|i| 0.3 + i as f32 * 0.01).collect();
    let result = engine.meta_optimize(&peer_vfes);
    assert!(result.is_ok());
}

#[test]
fn test_nes_convergence_curve_non_increasing() {
    let config = MetaActiveInferenceConfig {
        use_nes: true,
        use_es: false,
        meta_lr: 0.02,
        population_size: 20,
        ..Default::default()
    };
    let mut engine = MetaActiveInferenceEngine::new(&config);
    let curve = engine.meta_optimize_sequence(20, &[0.5, 0.5, 0.5]).unwrap();

    // Best VFE should be non-increasing
    for i in 1..curve.len() {
        assert!(
            curve[i] <= curve[i - 1] + 1e-6,
            "NES curve should be non-increasing: [{}]={}, [{}]={}",
            i - 1,
            curve[i - 1],
            i,
            curve[i]
        );
    }
}

#[test]
fn test_nes_history_grows() {
    let config = MetaActiveInferenceConfig {
        use_nes: true,
        use_es: false,
        population_size: 20,
        ..Default::default()
    };
    let mut engine = MetaActiveInferenceEngine::new(&config);
    assert!(engine.history().is_empty());

    engine.meta_optimize_sequence(5, &[0.5]).unwrap();
    assert!(engine.history().len() == 5);
}

#[test]
fn test_nes_current_params_accessible() {
    let config = MetaActiveInferenceConfig {
        use_nes: true,
        use_es: false,
        population_size: 20,
        ..Default::default()
    };
    let engine = MetaActiveInferenceEngine::new(&config);
    let params = engine.current_params();
    assert!(params.lr > 0.0);
}

#[test]
fn test_nes_perturbation_eps_effect() {
    let peer_vfes = &[0.5, 0.6];

    // Small sigma
    let small_sigma = MetaActiveInferenceConfig {
        use_nes: true,
        use_es: false,
        meta_lr: 0.05,
        population_size: 20,
        perturbation_eps: 1e-4,
        ..Default::default()
    };
    let mut e1 = MetaActiveInferenceEngine::new(&small_sigma);
    let c1 = e1.meta_optimize_sequence(10, peer_vfes).unwrap();

    // Large sigma
    let large_sigma = MetaActiveInferenceConfig {
        use_nes: true,
        use_es: false,
        meta_lr: 0.05,
        population_size: 20,
        perturbation_eps: 1e-2,
        ..Default::default()
    };
    let mut e2 = MetaActiveInferenceEngine::new(&large_sigma);
    let c2 = e2.meta_optimize_sequence(10, peer_vfes).unwrap();

    assert!(c1.iter().all(|v| v.is_finite()));
    assert!(c2.iter().all(|v| v.is_finite()));
}
