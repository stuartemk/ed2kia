//! Meta-Active Inference Tests — Sprint 109
//!
//! Validates meta-hyperparameter optimization via Evolutionary Strategy
//! and finite-difference gradients for self-improving collective nodes.

use native_audit::meta_active_inference::{
    MetaActiveInferenceConfig, MetaActiveInferenceEngine, MetaHyperParams,
};

#[test]
fn test_meta_hyper_params_default() {
    let params = MetaHyperParams::default();
    assert!(params.lr > 0.0, "lr must be positive");
    assert!(params.lambda_ot >= 0.0 && params.lambda_ot <= 1.0);
    assert!(params.beta_cbf > 0.0);
    assert!(params.sae_sparsity > 0.0);
    assert!(params.lambda_cross >= 0.0 && params.lambda_cross <= 1.0);
    assert!(params.beta_cirl >= 0.0 && params.beta_cirl <= 1.0);
}

#[test]
fn test_meta_hyper_params_clamp() {
    let params = MetaHyperParams {
        lr: 999.0,
        lambda_ot: -5.0,
        beta_cbf: 0.001,
        sae_sparsity: 50.0,
        lambda_cross: -1.0,
        beta_cirl: 2.0,
    };

    let clamped = params.clamp();
    assert!(
        clamped.lr >= 1e-5 && clamped.lr <= 1e-1,
        "lr out of bounds: {}",
        clamped.lr
    );
    assert!(clamped.lambda_ot >= 0.0 && clamped.lambda_ot <= 1.0);
    assert!(clamped.beta_cbf >= 0.1 && clamped.beta_cbf <= 2.0);
    assert!(clamped.sae_sparsity >= 1e-4 && clamped.sae_sparsity <= 0.1);
    assert!(clamped.lambda_cross >= 0.0 && clamped.lambda_cross <= 1.0);
    assert!(clamped.beta_cirl >= 0.0 && clamped.beta_cirl <= 1.0);
}

#[test]
fn test_meta_hyper_params_bounds() {
    let params = MetaHyperParams::default();
    let bounds = params.bounds();
    assert_eq!(bounds.len(), 6, "bounds should match 6 hyperparameters");
    for (lo, hi) in &bounds {
        assert!(*lo < *hi, "bound min must be less than max");
    }
}

#[test]
fn test_engine_creation() {
    let config = MetaActiveInferenceConfig::default();
    let engine = MetaActiveInferenceEngine::new(&config);
    assert_eq!(engine.history().len(), 0, "fresh engine has no history");
}

#[test]
fn test_estimate_meta_vfe() {
    let engine = MetaActiveInferenceEngine::new(&MetaActiveInferenceConfig::default());
    let params = MetaHyperParams::default();
    let peer_vfes = vec![1.2, 0.8, 1.5, 0.9, 1.1];
    let meta_vfe = engine.estimate_meta_vfe(&params, &peer_vfes);
    assert!(meta_vfe > 0.0, "meta-VFE must be positive: {}", meta_vfe);
    println!("Estimated meta-VFE: {:.4}", meta_vfe);
}

#[test]
fn test_meta_optimize_single_round() {
    let mut engine = MetaActiveInferenceEngine::new(&MetaActiveInferenceConfig::default());
    let peer_vfes = vec![1.2, 0.8, 1.5, 0.9, 1.1];
    let reduction = engine.meta_optimize(&peer_vfes).unwrap();
    assert!(!reduction.is_nan(), "reduction should not be NaN");
    println!("Meta-opt reduction: {:.4}", reduction);
}

#[test]
fn test_meta_optimize_sequence() {
    let mut engine = MetaActiveInferenceEngine::new(&MetaActiveInferenceConfig::default());
    let peer_vfes = vec![1.2, 0.8, 1.5, 0.9, 1.1];
    let num_rounds = 5;
    let curve = engine
        .meta_optimize_sequence(num_rounds, &peer_vfes)
        .unwrap();
    assert_eq!(curve.len(), num_rounds);
    assert!(
        engine.history().len() >= num_rounds,
        "history should have at least {} entries",
        num_rounds
    );
    for (i, v) in curve.iter().enumerate() {
        assert!(!v.is_nan(), "curve[{}] is NaN", i);
    }
    println!(
        "After {} rounds, curve: {:?}, history len: {}",
        num_rounds,
        curve,
        engine.history().len()
    );
}

#[test]
fn test_improvement_ratio() {
    let mut engine = MetaActiveInferenceEngine::new(&MetaActiveInferenceConfig::default());
    let peer_vfes = vec![1.2, 0.8, 1.5, 0.9, 1.1];
    let _ = engine.meta_optimize_sequence(3, &peer_vfes).unwrap();

    let ratio = engine.improvement_ratio();
    assert!(ratio >= 0.0, "improvement ratio should be non-negative");
    assert!(!ratio.is_nan(), "improvement ratio is NaN");
    println!("Improvement ratio: {:.4}", ratio);
}

#[test]
fn test_history_grows() {
    let mut engine = MetaActiveInferenceEngine::new(&MetaActiveInferenceConfig::default());
    assert_eq!(engine.history().len(), 0);

    let peer_vfes = vec![1.0, 0.9, 1.1];
    let _ = engine.meta_optimize_sequence(3, &peer_vfes).unwrap();

    assert!(
        engine.history().len() >= 3,
        "history should grow after optimization"
    );
    for (i, entry) in engine.history().iter().enumerate() {
        assert!(!entry.meta_vfe.is_nan(), "meta_vfe at index {} is NaN", i);
    }
}

#[test]
fn test_empty_peer_vfes() {
    let engine = MetaActiveInferenceEngine::new(&MetaActiveInferenceConfig::default());
    let params = MetaHyperParams::default();
    let meta_vfe = engine.estimate_meta_vfe(&params, &[]);
    assert!(
        meta_vfe >= 0.0,
        "meta-VFE with empty peers should use fallback"
    );
}

#[test]
fn test_config_es_flag() {
    let config = MetaActiveInferenceConfig::default();
    assert!(config.use_es, "default config should use ES");
    assert!(config.population_size > 0);

    let config_fd = MetaActiveInferenceConfig {
        use_es: false,
        ..MetaActiveInferenceConfig::default()
    };
    assert!(!config_fd.use_es);
    assert!(config_fd.num_perturbations > 0);
}

#[test]
fn test_meta_lr_effect() {
    let peer_vfes = vec![1.0, 0.9, 1.1];

    // High meta-lr
    let config_high = MetaActiveInferenceConfig {
        meta_lr: 1e-1,
        ..MetaActiveInferenceConfig::default()
    };
    let mut engine_high = MetaActiveInferenceEngine::new(&config_high);
    let initial_high = engine_high.current_params().lr;
    let _ = engine_high.meta_optimize(&peer_vfes).unwrap();
    let final_high = engine_high.current_params().lr;

    // Low meta-lr
    let config_low = MetaActiveInferenceConfig {
        meta_lr: 1e-5,
        ..MetaActiveInferenceConfig::default()
    };
    let mut engine_low = MetaActiveInferenceEngine::new(&config_low);
    let initial_low = engine_low.current_params().lr;
    let _ = engine_low.meta_optimize(&peer_vfes).unwrap();
    let final_low = engine_low.current_params().lr;

    let diff_high = (final_high - initial_high).abs();
    let diff_low = (final_low - initial_low).abs();

    assert!(
        diff_high >= diff_low,
        "high meta-lr should produce larger parameter changes: high={:.6}, low={:.6}",
        diff_high,
        diff_low
    );
}

#[test]
fn test_best_params_improve() {
    let mut engine = MetaActiveInferenceEngine::new(&MetaActiveInferenceConfig::default());
    let peer_vfes = vec![1.0, 0.9, 1.1, 0.85, 1.05];

    let initial_vfe = engine.best_meta_vfe();
    let _ = engine.meta_optimize_sequence(10, &peer_vfes).unwrap();
    let final_vfe = engine.best_meta_vfe();

    assert!(
        final_vfe <= initial_vfe,
        "best VFE should not worsen: initial={:.4}, final={:.4}",
        initial_vfe,
        final_vfe
    );
}

#[test]
fn test_current_params_accessible() {
    let engine = MetaActiveInferenceEngine::new(&MetaActiveInferenceConfig::default());
    let params = engine.current_params();
    assert!(params.lr > 0.0);
}

#[test]
fn test_convergence_curve_non_increasing() {
    let mut engine = MetaActiveInferenceEngine::new(&MetaActiveInferenceConfig::default());
    let peer_vfes = vec![0.5, 0.5, 0.5];

    let curve = engine.meta_optimize_sequence(20, &peer_vfes).unwrap();

    // Best VFE curve should be non-increasing (best only improves or stays same)
    for i in 1..curve.len() {
        assert!(
            curve[i] <= curve[i - 1] + 1e-6,
            "Curve should be non-increasing: [{}]={:.6}, [{}]={:.6}",
            i - 1,
            curve[i - 1],
            i,
            curve[i]
        );
    }
}

#[test]
fn test_es_vs_fd_both_work() {
    let peer_vfes = vec![1.0, 0.9, 1.1, 0.85, 1.05];

    // ES mode
    let config_es = MetaActiveInferenceConfig {
        use_es: true,
        population_size: 20,
        ..MetaActiveInferenceConfig::default()
    };
    let mut engine_es = MetaActiveInferenceEngine::new(&config_es);
    let reduction_es = engine_es.meta_optimize(&peer_vfes).unwrap();

    // FD mode
    let config_fd = MetaActiveInferenceConfig {
        use_es: false,
        num_perturbations: 6,
        ..MetaActiveInferenceConfig::default()
    };
    let mut engine_fd = MetaActiveInferenceEngine::new(&config_fd);
    let reduction_fd = engine_fd.meta_optimize(&peer_vfes).unwrap();

    assert!(!reduction_es.is_nan(), "ES reduction is NaN");
    assert!(!reduction_fd.is_nan(), "FD reduction is NaN");
    println!(
        "ES reduction: {:.4}, FD reduction: {:.4}",
        reduction_es, reduction_fd
    );
}
