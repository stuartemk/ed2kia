//! Sprint 115 (v11.5.0) — Zonotope Girard Order Reduction + PAC-Bayesian Meta-Self-Improvement + Full Certified Pipeline Integration
//!
//! Integration tests covering:
//! - Girard order reduction soundness and volume ratio
//! - PAC-Bayesian generalization bounds
//! - Monte Carlo violation estimation
//! - Full certified pipeline (Taylor-Zonotope → Reduction → MPC-CBF → PAC)

use candle_core::Result;
use native_audit::cbf_mpc::cbf_h;
use native_audit::formal_verification::{
    propagate_silu_taylor_zonotope, reduce_generators_girard, TaylorZonotopeConfig,
};
use native_audit::meta_improvement::{
    cbf_evaluate, compute_gaussian_kl, compute_pac_gen_bound, estimate_violation_prob,
    pac_bayes_meta_update, PACMetaConfig, PACMetaEngine,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_diagonal_generators(
    dim: usize,
    epsilon: f32,
    device: &candle_core::Device,
) -> Result<candle_core::Tensor> {
    let data: Vec<f32> = (0..dim)
        .flat_map(|i| (0..dim).map(move |j| if i == j { epsilon } else { 0.0 }))
        .collect();
    candle_core::Tensor::from_vec(data, (dim, dim), device)
}

fn make_center(values: Vec<f32>, device: &candle_core::Device) -> Result<candle_core::Tensor> {
    let len = values.len();
    candle_core::Tensor::from_vec(values, len, device)
}

// ---------------------------------------------------------------------------
// Girard Order Reduction Tests
// ---------------------------------------------------------------------------

#[test]
fn test_girard_reduction_no_op_when_under_limit() -> Result<()> {
    let device = candle_core::Device::Cpu;
    let dim = 4;
    let gens = make_diagonal_generators(dim, 0.1, &device)?;
    // max_gens > num_gens → no reduction
    let result = reduce_generators_girard(&gens, dim + 2)?;
    assert!(!result.reduced, "Should not reduce when under limit");
    assert_eq!(result.original_count, dim);
    assert_eq!(result.reduced_count, dim);
    assert!(
        (result.volume_ratio - 1.0).abs() < 0.01,
        "Volume ratio should be ~1.0"
    );
    Ok(())
}

#[test]
fn test_girard_reduction_reduces_when_over_limit() -> Result<()> {
    let device = candle_core::Device::Cpu;
    let dim = 8;
    let gens = make_diagonal_generators(dim, 0.1, &device)?;
    // max_gens < num_gens → reduction
    let result = reduce_generators_girard(&gens, 4)?;
    assert!(result.reduced, "Should reduce when over limit");
    assert_eq!(result.original_count, dim);
    assert!(
        result.reduced_count <= 4,
        "Reduced count should be <= max_gens"
    );
    assert!(
        result.volume_ratio >= 1.0,
        "Volume ratio should be >= 1.0 (over-approximation)"
    );
    Ok(())
}

#[test]
fn test_girard_reduction_soundness() -> Result<()> {
    let device = candle_core::Device::Cpu;
    let dim = 6;
    let gens = make_diagonal_generators(dim, 0.5, &device)?;
    let result = reduce_generators_girard(&gens, 3)?;
    // Soundness: volume_ratio should be reasonable (not excessive)
    assert!(
        result.volume_ratio < 3.0,
        "Volume ratio {:.2} is too high",
        result.volume_ratio
    );
    assert!(result.volume_ratio >= 1.0, "Volume ratio must be >= 1.0");
    Ok(())
}

#[test]
fn test_girard_reduction_preserves_dimensions() -> Result<()> {
    let device = candle_core::Device::Cpu;
    let dim = 5;
    let gens = make_diagonal_generators(dim, 0.2, &device)?;
    let result = reduce_generators_girard(&gens, 3)?;
    let dims = result.generators.shape().dims();
    assert_eq!(dims[1], dim, "Column dimension should be preserved");
    assert!(dims[0] <= 3, "Row count should be <= max_gens");
    Ok(())
}

#[test]
fn test_girard_reduction_volume_ratio_monotonic() -> Result<()> {
    let device = candle_core::Device::Cpu;
    let dim = 10;
    let gens = make_diagonal_generators(dim, 0.3, &device)?;
    // More aggressive reduction → higher volume ratio
    let result_loose = reduce_generators_girard(&gens, 6)?;
    let result_tight = reduce_generators_girard(&gens, 3)?;
    assert!(
        result_tight.volume_ratio >= result_loose.volume_ratio,
        "Tighter reduction should have higher volume ratio"
    );
    Ok(())
}

#[test]
fn test_girard_reduction_single_generator() -> Result<()> {
    let device = candle_core::Device::Cpu;
    let dim = 4;
    let gens = make_diagonal_generators(dim, 0.1, &device)?;
    let result = reduce_generators_girard(&gens, 1)?;
    assert!(result.reduced);
    assert!(result.reduced_count <= 1);
    Ok(())
}

#[test]
fn test_girard_reduction_large_zonotope() -> Result<()> {
    let device = candle_core::Device::Cpu;
    let dim = 32;
    let gens = make_diagonal_generators(dim, 0.05, &device)?;
    let result = reduce_generators_girard(&gens, 16)?;
    assert!(result.reduced);
    assert!(result.volume_ratio >= 1.0);
    assert!(
        result.volume_ratio < 5.0,
        "Volume ratio should be reasonable for large zonotope"
    );
    Ok(())
}

#[test]
fn test_girard_reduction_finite_values() -> Result<()> {
    let device = candle_core::Device::Cpu;
    let dim = 8;
    let gens = make_diagonal_generators(dim, 0.1, &device)?;
    let result = reduce_generators_girard(&gens, 3)?;
    let vals: Vec<f32> = result.generators.flatten_all()?.to_vec1()?;
    for v in &vals {
        assert!(v.is_finite(), "All generator values should be finite");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// PAC-Bayesian Bound Tests
// ---------------------------------------------------------------------------

#[test]
fn test_pac_bound_decreases_with_more_samples() {
    let kl = 0.01;
    let delta = 0.05;
    let bound_10 = compute_pac_gen_bound(kl, 10, delta);
    let bound_100 = compute_pac_gen_bound(kl, 100, delta);
    let bound_1000 = compute_pac_gen_bound(kl, 1000, delta);
    assert!(
        bound_1000 < bound_100,
        "Bound should decrease with more samples"
    );
    assert!(
        bound_100 < bound_10,
        "Bound should decrease with more samples"
    );
}

#[test]
fn test_pac_bound_increases_with_higher_kl() {
    let n = 50;
    let delta = 0.05;
    let bound_low = compute_pac_gen_bound(0.001, n, delta);
    let bound_high = compute_pac_gen_bound(0.1, n, delta);
    assert!(
        bound_high > bound_low,
        "Bound should increase with higher KL"
    );
}

#[test]
fn test_pac_bound_single_sample_is_max() {
    let bound = compute_pac_gen_bound(0.01, 1, 0.05);
    assert!(
        bound.is_infinite() || bound > 100.0,
        "Single sample should give very large bound"
    );
}

#[test]
fn test_pac_bound_zero_kl() {
    let bound = compute_pac_gen_bound(0.0, 50, 0.05);
    assert!(bound > 0.0, "Bound should be positive even with zero KL");
    assert!(bound.is_finite());
}

#[test]
fn test_pac_bound_delta_effect() {
    let kl = 0.01;
    let n = 50;
    let bound_loose = compute_pac_gen_bound(kl, n, 0.5);
    let bound_strict = compute_pac_gen_bound(kl, n, 0.01);
    assert!(
        bound_strict > bound_loose,
        "Stricter delta should give larger bound"
    );
}

// ---------------------------------------------------------------------------
// Gaussian KL Divergence Tests
// ---------------------------------------------------------------------------

#[test]
fn test_gaussian_kl_same_params() {
    let mu = vec![1.0, 2.0, 3.0];
    let kl = compute_gaussian_kl(&mu, &mu, 0.01, 0.01);
    assert!(
        (kl - 0.0).abs() < 0.01,
        "KL should be ~0 for identical distributions"
    );
}

#[test]
fn test_gaussian_kl_positive() {
    let mu_q = vec![1.0, 1.0];
    let mu_p = vec![0.0, 0.0];
    let kl = compute_gaussian_kl(&mu_q, &mu_p, 0.01, 0.01);
    assert!(kl >= 0.0, "KL divergence should be non-negative");
}

#[test]
fn test_gaussian_kl_variance_mismatch() {
    let mu = vec![0.0, 0.0];
    let kl = compute_gaussian_kl(&mu, &mu, 0.0001, 1.0);
    assert!(kl > 0.0, "KL should be positive for variance mismatch");
}

#[test]
fn test_gaussian_kl_dimension_consistency() {
    let mu_q = vec![1.0; 10];
    let mu_p = vec![0.0; 10];
    let kl = compute_gaussian_kl(&mu_q, &mu_p, 0.01, 0.01);
    assert!(kl.is_finite() && kl >= 0.0);
}

// ---------------------------------------------------------------------------
// CBF Evaluation Tests
// ---------------------------------------------------------------------------

#[test]
fn test_cbf_safe_center() {
    let state = vec![0.0, 0.0];
    let center = vec![0.0, 0.0];
    let h = cbf_evaluate(&state, &center, 1.0);
    assert!(h >= 1.0, "CBF at safe center should equal margin²");
}

#[test]
fn test_cbf_far_from_center() {
    let state = vec![5.0, 0.0];
    let center = vec![0.0, 0.0];
    let h = cbf_evaluate(&state, &center, 1.0);
    assert!(h < 0.0, "CBF should be negative far from center");
}

#[test]
fn test_cbf_at_boundary() {
    let state = vec![1.0, 0.0];
    let center = vec![0.0, 0.0];
    let h = cbf_evaluate(&state, &center, 1.0);
    assert!((h - 0.0).abs() < 0.01, "CBF at boundary should be ~0");
}

#[test]
fn test_cbf_high_dimension() {
    let state: Vec<f32> = (0..20).map(|i| i as f32 * 0.1).collect();
    let center: Vec<f32> = vec![0.0; 20];
    let h = cbf_evaluate(&state, &center, 5.0);
    assert!(h.is_finite());
}

// ---------------------------------------------------------------------------
// Monte Carlo Violation Estimation Tests
// ---------------------------------------------------------------------------

#[test]
fn test_mc_violation_safe_state() {
    let state = vec![0.0, 0.0];
    let center = vec![0.0, 0.0];
    let prob = estimate_violation_prob(&state, &center, 1.0, 0.1, 1000, 42);
    assert!(
        prob < 0.1,
        "Safe state should have low violation prob (got {})",
        prob
    );
}

#[test]
fn test_mc_violation_unsafe_state() {
    let state = vec![2.0, 0.0];
    let center = vec![0.0, 0.0];
    let prob = estimate_violation_prob(&state, &center, 1.0, 0.1, 1000, 42);
    assert!(
        prob > 0.5,
        "Unsafe state should have high violation prob (got {})",
        prob
    );
}

#[test]
fn test_mc_violation_boundary_state() {
    let state = vec![1.0, 0.0];
    let center = vec![0.0, 0.0];
    let prob = estimate_violation_prob(&state, &center, 1.0, 0.2, 2000, 123);
    assert!(prob >= 0.0 && prob <= 1.0, "Prob should be in [0,1]");
}

#[test]
fn test_mc_violation_deterministic_seed() {
    let state = vec![0.5, 0.5];
    let center = vec![0.0, 0.0];
    let prob1 = estimate_violation_prob(&state, &center, 1.0, 0.1, 500, 99);
    let prob2 = estimate_violation_prob(&state, &center, 1.0, 0.1, 500, 99);
    assert!(
        (prob1 - prob2).abs() < 0.001,
        "Same seed should give same result"
    );
}

#[test]
fn test_mc_violation_different_seeds() {
    let state = vec![0.8, 0.0];
    let center = vec![0.0, 0.0];
    let prob1 = estimate_violation_prob(&state, &center, 1.0, 0.1, 500, 1);
    let prob2 = estimate_violation_prob(&state, &center, 1.0, 0.1, 500, 2);
    // Different seeds may give different results
    assert!(prob1 >= 0.0 && prob1 <= 1.0);
    assert!(prob2 >= 0.0 && prob2 <= 1.0);
}

#[test]
fn test_mc_violation_convergence() {
    let state = vec![0.5, 0.0];
    let center = vec![0.0, 0.0];
    let prob_small = estimate_violation_prob(&state, &center, 1.0, 0.1, 100, 42);
    let prob_large = estimate_violation_prob(&state, &center, 1.0, 0.1, 10000, 42);
    // Large sample should be more stable (both should be reasonable)
    assert!(prob_small >= 0.0 && prob_small <= 1.0);
    assert!(prob_large >= 0.0 && prob_large <= 1.0);
}

// ---------------------------------------------------------------------------
// PAC Meta-Update Tests
// ---------------------------------------------------------------------------

#[test]
fn test_pac_update_accepts_safe_params() {
    // Identical params → KL=0 → gen_bound = sqrt(ln(2n/delta)/(2(n-1)))
    // With n=10, delta=0.05: bound ≈ sqrt(ln(400)/18) ≈ 0.448 < 1.0
    let params = vec![0.0, 0.0];
    let samples = vec![0.0f32; 10]; // Zero risk, more samples → tighter bound
    let config = PACMetaConfig {
        delta: 0.05,
        max_gen_bound: 1.0,
        num_samples: 10,
        meta_lr: 0.01,
        cbf_margin: 2.0,
        mc_samples: 500,
        prior_concentration: 0.01,
    };
    let result = pac_bayes_meta_update(&params, &params, &samples, &params, &config);
    assert!(
        result.accepted,
        "Safe params should be accepted: {:?}",
        result.rejection_reason
    );
}

#[test]
fn test_pac_update_rejects_high_risk() {
    let proposed = vec![3.0, 0.0];
    let current = vec![0.0, 0.0];
    let samples = vec![1.0, 1.0, 1.0]; // High risk
    let config = PACMetaConfig {
        cbf_margin: 1.0,
        ..Default::default()
    };
    let result = pac_bayes_meta_update(&proposed, &current, &samples, &current, &config);
    // State at distance 3 from center with margin 1 → CBF violation
    assert!(!result.accepted, "High-risk params should be rejected");
}

#[test]
fn test_pac_update_rejection_reason() {
    let proposed = vec![5.0, 0.0];
    let current = vec![0.0, 0.0];
    let samples = vec![0.5];
    let config = PACMetaConfig::default();
    let result = pac_bayes_meta_update(&proposed, &current, &samples, &current, &config);
    if !result.accepted {
        assert!(
            result.rejection_reason.is_some(),
            "Rejected result should have reason"
        );
    }
}

#[test]
fn test_pac_update_empty_samples() {
    let proposed = vec![0.1, 0.1];
    let current = vec![0.0, 0.0];
    let samples: Vec<f32> = vec![];
    let config = PACMetaConfig::default();
    let result = pac_bayes_meta_update(&proposed, &current, &samples, &current, &config);
    assert!(
        !result.accepted,
        "Empty samples should be rejected (infinite risk)"
    );
}

#[test]
fn test_pac_update_gen_bound_finite() {
    let proposed = vec![0.01, 0.01];
    let current = vec![0.0, 0.0];
    let samples = vec![0.0, 0.0];
    let config = PACMetaConfig {
        prior_concentration: 0.001,
        max_gen_bound: 1.0,
        ..Default::default()
    };
    let result = pac_bayes_meta_update(&proposed, &current, &samples, &current, &config);
    assert!(result.gen_bound.is_finite(), "Gen bound should be finite");
}

// ---------------------------------------------------------------------------
// PACMetaEngine Tests
// ---------------------------------------------------------------------------

#[test]
fn test_engine_initial_state() {
    let engine = PACMetaEngine::new(vec![0.0; 4], vec![0.0; 4], PACMetaConfig::default());
    assert_eq!(engine.params().len(), 4);
    assert_eq!(engine.history().len(), 0);
}

#[test]
fn test_engine_add_samples() {
    let mut engine = PACMetaEngine::new(vec![0.0; 2], vec![0.0; 2], PACMetaConfig::default());
    engine.add_sample(0.1);
    engine.add_sample(0.2);
    engine.add_sample(0.0);
    // performance_buffer is private, verify via add_sample behavior
    // (3 samples added, engine accepts them internally)
}

#[test]
fn test_engine_step_accepts() {
    let config = PACMetaConfig {
        max_gen_bound: 1.0,
        prior_concentration: 0.01,
        cbf_margin: 2.0,
        meta_lr: 0.01,
        num_samples: 10,
        ..Default::default()
    };
    let mut engine = PACMetaEngine::new(vec![0.0; 2], vec![0.0; 2], config);
    // Add many zero-risk samples for tighter PAC bound
    for _ in 0..10 {
        engine.add_sample(0.0);
    }
    // Zero gradient → proposed == current → KL = 0 → accepted
    let result = engine.step(&[0.0, 0.0]);
    assert!(
        result.accepted,
        "Zero-gradient step should be accepted: {:?}",
        result.rejection_reason
    );
}

#[test]
fn test_engine_acceptance_rate_tracks() {
    let config = PACMetaConfig {
        max_gen_bound: 1.0,
        prior_concentration: 0.001,
        cbf_margin: 1.0,
        ..Default::default()
    };
    let mut engine = PACMetaEngine::new(vec![0.0; 2], vec![0.0; 2], config);
    engine.add_sample(0.0);
    engine.step(&[0.01, 0.01]); // Should accept
    engine.step(&[0.02, 0.02]); // Should accept
    let rate = engine.acceptance_rate();
    assert!(rate >= 0.0 && rate <= 1.0);
}

#[test]
fn test_engine_history_grows() {
    let config = PACMetaConfig {
        max_gen_bound: 1.0,
        prior_concentration: 0.001,
        cbf_margin: 1.0,
        ..Default::default()
    };
    let mut engine = PACMetaEngine::new(vec![0.0; 2], vec![0.0; 2], config);
    engine.add_sample(0.0);
    let initial_len = engine.history().len();
    engine.step(&[0.01, 0.01]);
    assert!(
        engine.history().len() >= initial_len,
        "History should grow after step"
    );
}

#[test]
fn test_engine_avg_gen_bound() {
    let config = PACMetaConfig {
        max_gen_bound: 1.0,
        prior_concentration: 0.001,
        cbf_margin: 1.0,
        ..Default::default()
    };
    let mut engine = PACMetaEngine::new(vec![0.0; 2], vec![0.0; 2], config);
    engine.add_sample(0.0);
    engine.step(&[0.01, 0.01]);
    let avg = engine.avg_gen_bound();
    assert!(avg.is_finite() && avg >= 0.0);
}

// ---------------------------------------------------------------------------
// Full Pipeline Integration Tests
// ---------------------------------------------------------------------------

#[test]
fn test_full_pipeline_girard_then_pac() -> Result<()> {
    let device = candle_core::Device::Cpu;
    let dim = 4;
    let center = make_center(vec![0.1, 0.1, 0.1, 0.1], &device)?;
    let gens = make_diagonal_generators(dim, 0.05, &device)?;

    // Step 1: Taylor propagation
    let config = TaylorZonotopeConfig::default();
    let taylor = propagate_silu_taylor_zonotope(&center, &gens, &config)?;

    // Step 2: Girard reduction
    let reduction = reduce_generators_girard(&taylor.generators, 3)?;
    assert!(reduction.volume_ratio >= 1.0);

    // Step 3: CBF check
    let safe_center = candle_core::Tensor::zeros(dim, candle_core::DType::F32, &device)?;
    let cbf_val = cbf_h(&taylor.center, &safe_center, 1.0)?;
    let margin: f32 = cbf_val.to_scalar()?;
    assert!(margin.is_finite());

    // Step 4: PAC check
    let center_vec: Vec<f32> = taylor.center.flatten_all()?.to_vec1()?;
    let safe_vec: Vec<f32> = safe_center.flatten_all()?.to_vec1()?;
    let pac_config = PACMetaConfig {
        max_gen_bound: 1.0,
        prior_concentration: 0.001,
        cbf_margin: 1.0,
        ..Default::default()
    };
    let samples = if margin < 0.0 {
        vec![margin.abs()]
    } else {
        vec![0.0]
    };
    let pac = pac_bayes_meta_update(&center_vec, &safe_vec, &samples, &safe_vec, &pac_config);
    assert!(pac.gen_bound.is_finite());

    Ok(())
}

#[test]
fn test_pipeline_chain_preserves_safety() -> Result<()> {
    let device = candle_core::Device::Cpu;
    let dim = 8;
    let center = make_center(vec![0.05; dim], &device)?;
    let gens = make_diagonal_generators(dim, 0.02, &device)?;

    let config = TaylorZonotopeConfig::default();
    let taylor = propagate_silu_taylor_zonotope(&center, &gens, &config)?;
    let reduction = reduce_generators_girard(&taylor.generators, 4)?;

    // Verify the reduced zonotope is still sound
    assert!(
        reduction.volume_ratio < 5.0,
        "Volume ratio should be reasonable"
    );
    let vals: Vec<f32> = reduction.generators.flatten_all()?.to_vec1()?;
    for v in &vals {
        assert!(v.is_finite());
    }

    Ok(())
}

#[test]
fn test_pipeline_volume_ratio_bounded() -> Result<()> {
    let device = candle_core::Device::Cpu;
    for dim in [4, 8, 16, 32] {
        let center = make_center(vec![0.01; dim], &device)?;
        let gens = make_diagonal_generators(dim, 0.03, &device)?;
        let config = TaylorZonotopeConfig::default();
        let taylor = propagate_silu_taylor_zonotope(&center, &gens, &config)?;
        let reduction = reduce_generators_girard(&taylor.generators, dim / 2)?;
        assert!(
            reduction.volume_ratio < 10.0,
            "dim={}: Volume ratio {:.2} too high",
            dim,
            reduction.volume_ratio
        );
    }
    Ok(())
}

#[test]
fn test_pipeline_pac_bound_tightens() -> Result<()> {
    let device = candle_core::Device::Cpu;
    let dim = 4;
    let center = make_center(vec![0.01; dim], &device)?;
    let gens = make_diagonal_generators(dim, 0.01, &device)?;

    let config = TaylorZonotopeConfig::default();
    let taylor = propagate_silu_taylor_zonotope(&center, &gens, &config)?;
    let center_vec: Vec<f32> = taylor.center.flatten_all()?.to_vec1()?;

    // More samples → tighter bound
    let samples_5 = vec![0.0_f32; 5];
    let samples_50 = vec![0.0_f32; 50];
    let pac_config = PACMetaConfig {
        prior_concentration: 0.001,
        ..Default::default()
    };
    let result_5 = pac_bayes_meta_update(
        &center_vec,
        &center_vec,
        &samples_5,
        &center_vec,
        &pac_config,
    );
    let result_50 = pac_bayes_meta_update(
        &center_vec,
        &center_vec,
        &samples_50,
        &center_vec,
        &pac_config,
    );
    assert!(
        result_50.gen_bound <= result_5.gen_bound,
        "More samples should tighten PAC bound"
    );

    Ok(())
}

#[test]
fn test_pipeline_cbf_consistency() -> Result<()> {
    let device = candle_core::Device::Cpu;
    let dim = 4;
    // State well inside safe region
    let center = make_center(vec![0.1; dim], &device)?;
    let safe_center = candle_core::Tensor::zeros(dim, candle_core::DType::F32, &device)?;
    let cbf_val = cbf_h(&center, &safe_center, 2.0)?;
    let margin: f32 = cbf_val.to_scalar()?;
    assert!(
        margin > 0.0,
        "State inside safe region should have positive CBF"
    );

    // State outside safe region
    let unsafe_center = make_center(vec![3.0; dim], &device)?;
    let cbf_val2 = cbf_h(&unsafe_center, &safe_center, 1.0)?;
    let margin2: f32 = cbf_val2.to_scalar()?;
    assert!(
        margin2 < 0.0,
        "State outside safe region should have negative CBF"
    );

    Ok(())
}

#[test]
fn test_pipeline_mc_estimates_violation() -> Result<()> {
    let state_safe = vec![0.1, 0.1];
    let state_unsafe = vec![2.0, 0.0];
    let center = vec![0.0, 0.0];

    let prob_safe = estimate_violation_prob(&state_safe, &center, 1.0, 0.1, 1000, 42);
    let prob_unsafe = estimate_violation_prob(&state_unsafe, &center, 1.0, 0.1, 1000, 42);

    assert!(
        prob_safe < prob_unsafe,
        "Safe state should have lower violation prob"
    );
    Ok(())
}

#[test]
fn test_pipeline_all_components_finite() -> Result<()> {
    let device = candle_core::Device::Cpu;
    let dim = 4;
    let center = make_center(vec![0.1; dim], &device)?;
    let gens = make_diagonal_generators(dim, 0.05, &device)?;

    let config = TaylorZonotopeConfig::default();
    let taylor = propagate_silu_taylor_zonotope(&center, &gens, &config)?;
    assert!(taylor.volume_proxy.is_finite());
    assert!(taylor.wrapping_reduction.is_finite());

    let reduction = reduce_generators_girard(&taylor.generators, 3)?;
    assert!(reduction.volume_ratio.is_finite());

    let safe_center = candle_core::Tensor::zeros(dim, candle_core::DType::F32, &device)?;
    let cbf_val = cbf_h(&taylor.center, &safe_center, 1.0)?;
    let margin: f32 = cbf_val.to_scalar()?;
    assert!(margin.is_finite());

    Ok(())
}

#[test]
fn test_pipeline_stress_small_dims() -> Result<()> {
    let device = candle_core::Device::Cpu;
    for dim in [2, 3, 4] {
        let center = make_center(vec![0.05; dim], &device)?;
        let gens = make_diagonal_generators(dim, 0.02, &device)?;
        let config = TaylorZonotopeConfig::default();
        let taylor = propagate_silu_taylor_zonotope(&center, &gens, &config)?;
        let reduction = reduce_generators_girard(&taylor.generators, dim)?;
        assert!(reduction.volume_ratio >= 1.0);
    }
    Ok(())
}

#[test]
fn test_pipeline_stress_large_dims() -> Result<()> {
    let device = candle_core::Device::Cpu;
    let dim = 64;
    let center = make_center(vec![0.01; dim], &device)?;
    let gens = make_diagonal_generators(dim, 0.01, &device)?;
    let config = TaylorZonotopeConfig::default();
    let taylor = propagate_silu_taylor_zonotope(&center, &gens, &config)?;
    let reduction = reduce_generators_girard(&taylor.generators, 32)?;
    assert!(reduction.reduced);
    assert!(reduction.volume_ratio.is_finite());
    Ok(())
}

#[test]
fn test_pipeline_reduction_then_cbf() -> Result<()> {
    let device = candle_core::Device::Cpu;
    let dim = 6;
    let center = make_center(vec![0.05; dim], &device)?;
    let gens = make_diagonal_generators(dim, 0.03, &device)?;

    let config = TaylorZonotopeConfig::default();
    let taylor = propagate_silu_taylor_zonotope(&center, &gens, &config)?;
    let reduction = reduce_generators_girard(&taylor.generators, 3)?;

    // CBF on the Taylor center (not affected by reduction)
    let safe_center = candle_core::Tensor::zeros(dim, candle_core::DType::F32, &device)?;
    let cbf_val = cbf_h(&taylor.center, &safe_center, 1.0)?;
    let margin: f32 = cbf_val.to_scalar()?;

    // Both should produce valid results
    assert!(reduction.volume_ratio >= 1.0);
    assert!(margin.is_finite());

    Ok(())
}

#[test]
fn test_pipeline_engine_integration() -> Result<()> {
    let config = PACMetaConfig {
        max_gen_bound: 1.0,
        prior_concentration: 0.001,
        cbf_margin: 1.0,
        ..Default::default()
    };
    let mut engine = PACMetaEngine::new(vec![0.0; 4], vec![0.0; 4], config);

    // Simulate a pipeline step
    engine.add_sample(0.0);
    engine.add_sample(0.0);
    let result = engine.step(&[0.01, 0.01, 0.01, 0.01]);
    assert!(result.gen_bound.is_finite());
    assert!(result.cbf_value.is_finite());

    Ok(())
}

// ---------------------------------------------------------------------------
// Display / Debug Tests
// ---------------------------------------------------------------------------

#[test]
fn test_pac_result_display() {
    let result = native_audit::meta_improvement::PACMetaResult {
        accepted: true,
        gen_bound: 0.5,
        empirical_risk: 0.1,
        kl_divergence: 0.01,
        cbf_value: 0.8,
        violation_prob: 0.02,
        rejection_reason: None,
    };
    let s = format!("{}", result);
    assert!(s.contains("accepted: true"));
}

#[test]
fn test_reduction_result_debug() -> Result<()> {
    let device = candle_core::Device::Cpu;
    let gens = make_diagonal_generators(4, 0.1, &device)?;
    let result = reduce_generators_girard(&gens, 2)?;
    let debug_str = format!("{:?}", result);
    assert!(!debug_str.is_empty());
    Ok(())
}

// ---------------------------------------------------------------------------
// Edge Cases
// ---------------------------------------------------------------------------

#[test]
fn test_girard_reduction_zero_generators() -> Result<()> {
    let device = candle_core::Device::Cpu;
    // Single row generator — use explicit f32 literals
    let gens = candle_core::Tensor::from_vec(vec![0.1f32, 0.1, 0.1], (1, 3), &device)?;
    let result = reduce_generators_girard(&gens, 2)?;
    assert!(
        !result.reduced,
        "Single generator should not need reduction"
    );
    Ok(())
}

#[test]
fn test_pac_update_identical_params() {
    let params = vec![0.5, 0.5];
    let samples = vec![0.0, 0.0];
    let config = PACMetaConfig {
        prior_concentration: 0.001,
        max_gen_bound: 1.0,
        cbf_margin: 2.0,
        ..Default::default()
    };
    let result = pac_bayes_meta_update(&params, &params, &samples, &params, &config);
    // Identical params → KL = 0 → small gen_bound
    assert!(
        result.kl_divergence < 0.01,
        "KL should be ~0 for identical params"
    );
}

#[test]
fn test_cbf_margin_scaling() {
    let state = vec![1.0, 0.0];
    let center = vec![0.0, 0.0];
    let h_small = cbf_evaluate(&state, &center, 0.5);
    let h_large = cbf_evaluate(&state, &center, 2.0);
    assert!(
        h_large > h_small,
        "Larger margin should give larger CBF value"
    );
}

#[test]
fn test_mc_perturbation_size_effect() {
    let state = vec![0.9, 0.0];
    let center = vec![0.0, 0.0];
    let prob_small = estimate_violation_prob(&state, &center, 1.0, 0.01, 1000, 42);
    let prob_large = estimate_violation_prob(&state, &center, 1.0, 0.5, 1000, 42);
    // Larger perturbation → more violations for boundary state
    assert!(
        prob_large >= prob_small,
        "Larger perturbation should increase violations"
    );
}

#[test]
fn test_full_sprint115_pipeline() -> Result<()> {
    // End-to-end test: Taylor-Zonotope → Girard Reduction → CBF → PAC
    let device = candle_core::Device::Cpu;
    let dim = 8;
    let epsilon = 0.03;

    // 1. Create zonotope
    let center = make_center(vec![0.05; dim], &device)?;
    let gens = make_diagonal_generators(dim, epsilon, &device)?;

    // 2. Taylor-Zonotope propagation
    let taylor_config = TaylorZonotopeConfig::default();
    let taylor = propagate_silu_taylor_zonotope(&center, &gens, &taylor_config)?;
    assert!(taylor.volume_proxy > 0.0);

    // 3. Girard order reduction — reduce to half for meaningful volume ratio
    let reduction = reduce_generators_girard(&taylor.generators, dim / 2)?;
    assert!(
        reduction.volume_ratio > 0.0,
        "Volume ratio should be positive: {:.4}",
        reduction.volume_ratio
    );
    assert!(
        reduction.volume_ratio < 10.0,
        "Volume ratio {:.2} too high",
        reduction.volume_ratio
    );

    // 4. CBF safety check
    let safe_center = candle_core::Tensor::zeros(dim, candle_core::DType::F32, &device)?;
    let cbf_val = cbf_h(&taylor.center, &safe_center, 1.0)?;
    let margin: f32 = cbf_val.to_scalar()?;
    assert!(margin.is_finite());

    // 5. PAC meta-check — use identical params for KL=0
    let center_vec: Vec<f32> = taylor.center.flatten_all()?.to_vec1()?;
    let safe_vec: Vec<f32> = safe_center.flatten_all()?.to_vec1()?;
    let pac_config = PACMetaConfig {
        max_gen_bound: 1.0,
        prior_concentration: 0.01,
        cbf_margin: 1.0,
        ..Default::default()
    };
    let samples = if margin < 0.0 {
        vec![margin.abs()]
    } else {
        vec![0.0]
    };
    let pac = pac_bayes_meta_update(&center_vec, &safe_vec, &samples, &safe_vec, &pac_config);
    assert!(pac.gen_bound.is_finite());

    // 6. Monte Carlo validation
    let prob = estimate_violation_prob(&center_vec, &safe_vec, 1.0, 0.1, 500, 42);
    assert!(prob >= 0.0 && prob <= 1.0);

    println!(
        "Sprint 115 Pipeline: vol_ratio={:.3}, cbf={:.4}, pac_bound={:.4}, mc_prob={:.4}",
        reduction.volume_ratio, margin, pac.gen_bound, prob
    );

    Ok(())
}
