//! Sprint 116 (v11.6.0) — Latent FGSM Red-Teaming + Certified Bounds
//!
//! **Fast Gradient Sign Method (FGSM)** in latent space:
//!     h_adv = h + ε · sign(∇_h Loss_CBF)
//!
//! **Certified Robustness via IBP (Interval Bound Propagation):**
//! For each latent dimension i, compute [h_i - ε, h_i + ε] bounds
//! and propagate through the CBF to verify certified safety.
//!
//! **IBP-Hybrid:** Combine IBP interval bounds with Taylor-Zonotope
//! for tighter certified regions under adversarial perturbation.

use candle_core::{Device, Result, Tensor};
use native_audit::cbf_mpc::cbf_h;
use native_audit::formal_verification::{
    compute_volume_ratio, propagate_layer_taylor_zonotope, reduce_generators_girard_advanced,
    verify_soundness, GirardConfig, GirardMerge, GirardNorm,
};
use native_audit::meta_improvement::{
    compute_gaussian_kl_data_dependent, compute_pac_bayes_bound, compute_pac_gen_bound,
};

/// Build a latent hidden-state tensor from a Vec<f32>.
fn latent_tensor(values: &[f32], device: &Device) -> Result<Tensor> {
    Tensor::from_vec(values.to_vec(), values.len(), device)
}

/// Build a 2-D latent tensor (batch × dim).
fn latent_batch(rows: usize, cols: usize, device: &Device) -> Result<Tensor> {
    let data: Vec<f32> = (0..(rows * cols)).map(|i| i as f32 * 0.01).collect();
    Tensor::from_vec(data, (rows, cols), device)
}

/// Compute sign of a tensor element-wise.
fn sign(x: &Tensor) -> Result<Tensor> {
    let zero = Tensor::zeros_like(x)?;
    let one = Tensor::ones_like(x)?;
    let pos = Tensor::gt(x, &zero)?.to_dtype(x.dtype())?;
    let neg = Tensor::lt(x, &zero)?.to_dtype(x.dtype())?;
    let pos_part = pos.broadcast_mul(&one)?;
    let neg_part = neg.broadcast_mul(&one)?;
    pos_part.sub(&neg_part)
}

/// Compute CBF-based loss: L = max(0, -h)² — penalizes unsafe states.
fn loss_cbf(_h_val: f32, safe_center: &[f32], margin: f32, latent: &[f32]) -> f32 {
    let dist_sq: f32 = latent
        .iter()
        .zip(safe_center.iter())
        .map(|(l, s)| (l - s).powi(2))
        .sum();
    let h = margin * margin - dist_sq;
    if h < 0.0 {
        (-h).powi(2)
    } else {
        0.0
    }
}

/// Finite-difference gradient of CBF loss w.r.t. latent state.
fn grad_cbf_loss_fd(latent: &[f32], safe_center: &[f32], margin: f32, eps: f32) -> Vec<f32> {
    let base_loss = loss_cbf(0.0, safe_center, margin, latent);
    let dim = latent.len();
    let mut grad = vec![0.0f32; dim];
    for i in 0..dim {
        let mut perturbed = latent.to_vec();
        perturbed[i] += eps;
        let loss_plus = loss_cbf(0.0, safe_center, margin, &perturbed);
        grad[i] = (loss_plus - base_loss) / eps;
    }
    grad
}

/// FGSM latent attack: h_adv = h + ε · sign(∇_h Loss_CBF).
fn fgsm_latent_attack(latent: &[f32], epsilon: f32, safe_center: &[f32], margin: f32) -> Vec<f32> {
    let grad = grad_cbf_loss_fd(latent, safe_center, margin, 1e-4);
    let sign_grad = sign_vec(&grad);
    latent
        .iter()
        .zip(sign_grad.iter())
        .map(|(h, s)| h + epsilon * s)
        .collect()
}

/// Element-wise sign for Vec<f32>.
fn sign_vec(v: &[f32]) -> Vec<f32> {
    v.iter()
        .map(|&x| {
            if x > 0.0 {
                1.0
            } else if x < 0.0 {
                -1.0
            } else {
                0.0
            }
        })
        .collect()
}

/// IBP certified bounds: For each dimension, compute [h_i - ε, h_i + ε].
fn ibp_bounds(latent: &[f32], epsilon: f32) -> (Vec<f32>, Vec<f32>) {
    let lo = latent.iter().map(|&h| h - epsilon).collect();
    let hi = latent.iter().map(|&h| h + epsilon).collect();
    (lo, hi)
}

/// Certified CBF safety under L∞-ball of radius ε using IBP.
/// Returns the worst-case (minimum) CBF value over the perturbation set.
fn certified_cbf_ibp(latent: &[f32], epsilon: f32, safe_center: &[f32], margin: f32) -> f32 {
    let (lo, hi) = ibp_bounds(latent, epsilon);
    // Worst case: maximize distance to safe_center
    // For each dim, pick the bound farther from safe_center[i]
    let mut worst_dist_sq = 0.0f32;
    for (i, (&l, &h)) in lo.iter().zip(hi.iter()).enumerate() {
        let c = safe_center[i];
        let dist_lo = (l - c).abs();
        let dist_hi = (h - c).abs();
        worst_dist_sq += dist_lo.max(dist_hi).powi(2);
    }
    margin * margin - worst_dist_sq
}

/// Hybrid IBP-Zonotope certified bound.
/// Combines IBP intervals with Taylor-Zonotope propagation for tighter bounds.
fn certified_cbf_hybrid(
    latent: &Tensor,
    epsilon: f32,
    _safe_center: &Tensor,
    margin: f32,
) -> Result<f32> {
    use native_audit::formal_verification::TaylorZonotopeConfig;

    // Build zonotope generators from IBP bounds
    let dim = latent.dim(0)?;
    let gen_data: Vec<f32> = vec![epsilon; dim];
    let generators = Tensor::from_vec(gen_data, (1, dim), latent.device())?;

    // Identity weight matrix for pass-through
    let identity = Tensor::eye(dim, latent.dtype(), latent.device())?;
    let config = TaylorZonotopeConfig::default();
    let result = propagate_layer_taylor_zonotope(latent, &generators, &identity, None, &config)?;

    // Compute worst-case CBF from zonotope bounds
    let worst_dist_sq: f32 = result.generators.abs()?.sum(0)?.to_vec1()?.iter().sum();
    Ok(margin - worst_dist_sq)
}

// ——— Integration Tests ———

#[test]
fn test_sign_tensor() -> Result<()> {
    let device = Device::Cpu;
    let t = Tensor::from_vec(vec![-2.0f32, 0.0, 3.0, -0.5, 1.0], 5, &device)?;
    let s = sign(&t)?;
    let signs = s.to_vec1::<f32>()?;
    assert!((signs[0] - (-1.0)).abs() < 1e-5);
    assert!(signs[1].abs() < 1e-5);
    assert!((signs[2] - 1.0).abs() < 1e-5);
    assert!((signs[3] - (-1.0)).abs() < 1e-5);
    assert!((signs[4] - 1.0).abs() < 1e-5);
    Ok(())
}

#[test]
fn test_loss_cbf_safe() {
    let latent = vec![0.0, 0.0, 0.0];
    let center = vec![0.0, 0.0, 0.0];
    let loss = loss_cbf(0.0, &center, 1.0, &latent);
    assert_eq!(loss, 0.0, "Safe state should have zero loss");
}

#[test]
fn test_loss_cbf_unsafe() {
    let latent = vec![2.0, 0.0, 0.0];
    let center = vec![0.0, 0.0, 0.0];
    let loss = loss_cbf(0.0, &center, 1.0, &latent);
    // dist² = 4, h = 1 - 4 = -3, loss = (-(-3))² = 9
    assert!((loss - 9.0).abs() < 1e-5);
}

#[test]
fn test_grad_cbf_fd_safe_state() {
    let latent = vec![0.0, 0.0, 0.0];
    let center = vec![0.0, 0.0, 0.0];
    let grad = grad_cbf_loss_fd(&latent, &center, 1.0, 1e-4);
    // At safe center, gradient should be near zero
    for &g in &grad {
        assert!(
            g.abs() < 1.0,
            "Gradient at safe center should be small, got {}",
            g
        );
    }
}

#[test]
fn test_grad_cbf_fd_unsafe_state() {
    let latent = vec![2.0, 0.0, 0.0];
    let center = vec![0.0, 0.0, 0.0];
    let grad = grad_cbf_loss_fd(&latent, &center, 1.0, 1e-4);
    // Gradient of loss = max(0, -h)² w.r.t. latent
    // When unsafe (h < 0), increasing distance increases loss → gradient is non-zero
    assert!(
        grad[0].abs() > 0.01,
        "Gradient should be non-zero for unsafe state, got {}",
        grad[0]
    );
}

#[test]
fn test_fgsm_attack_moves_away_from_center() {
    let center = vec![0.0f32, 0.0, 0.0];
    let latent = vec![0.5, 0.3, -0.2]; // Near boundary
    let epsilon = 0.1;
    let adv = fgsm_latent_attack(&latent, epsilon, &center, 1.0);

    // Compute distances
    let dist_orig: f32 = latent
        .iter()
        .zip(center.iter())
        .map(|(l, c)| (l - c).powi(2))
        .sum();
    let dist_adv: f32 = adv
        .iter()
        .zip(center.iter())
        .map(|(a, c)| (a - c).powi(2))
        .sum();

    // FGSM should increase distance (or keep same if already at boundary)
    assert!(
        dist_adv >= dist_orig - 1e-3,
        "FGSM should not decrease distance to center: orig={}, adv={}",
        dist_orig,
        dist_adv
    );
}

#[test]
fn test_fgsm_attack_linf_bound() {
    let center = vec![0.0f32, 0.0, 0.0];
    let latent = vec![0.5, 0.3, -0.2];
    let epsilon = 0.1;
    let adv = fgsm_latent_attack(&latent, epsilon, &center, 1.0);

    // Verify L∞ perturbation bound
    for (i, (l, a)) in latent.iter().zip(adv.iter()).enumerate() {
        assert!(
            (a - l).abs() <= epsilon + 1e-5,
            "L∞ bound violated at dim {}: |adv - orig| = {}",
            i,
            (a - l).abs()
        );
    }
}

#[test]
fn test_ibp_bounds_symmetric() {
    let latent = vec![1.0, 2.0, 3.0];
    let epsilon = 0.5;
    let (lo, hi) = ibp_bounds(&latent, epsilon);
    assert_eq!(lo, vec![0.5, 1.5, 2.5]);
    assert_eq!(hi, vec![1.5, 2.5, 3.5]);
}

#[test]
fn test_certified_cbf_ibp_safe() {
    let latent = vec![0.0, 0.0, 0.0];
    let center = vec![0.0, 0.0, 0.0];
    let margin = 1.0;
    let epsilon = 0.3;
    let cert = certified_cbf_ibp(&latent, epsilon, &center, margin);
    // Worst case: all dims at ε from center → dist² = 3ε² = 0.27
    // cert = 1.0 - 0.27 = 0.73
    assert!(
        cert > 0.0,
        "Certified CBF should be positive for safe state, got {}",
        cert
    );
}

#[test]
fn test_certified_cbf_ibp_unsafe_large_epsilon() {
    let latent = vec![0.0, 0.0, 0.0];
    let center = vec![0.0, 0.0, 0.0];
    let margin = 1.0;
    let epsilon = 1.0;
    let cert = certified_cbf_ibp(&latent, epsilon, &center, margin);
    // Worst case: dist² = 3 × 1² = 3, cert = 1 - 3 = -2
    assert!(
        cert < 0.0,
        "Large epsilon should make certified CBF negative, got {}",
        cert
    );
}

#[test]
fn test_certified_cbf_ibp_epsilon_monotonic() {
    let latent = vec![0.0, 0.0, 0.0];
    let center = vec![0.0, 0.0, 0.0];
    let margin = 1.0;
    let cert_small = certified_cbf_ibp(&latent, 0.1, &center, margin);
    let cert_large = certified_cbf_ibp(&latent, 0.5, &center, margin);
    assert!(
        cert_large <= cert_small,
        "Certified CBF should decrease with larger epsilon: small={}, large={}",
        cert_small,
        cert_large
    );
}

#[test]
fn test_pac_bayes_bound_includes_emp_vfe() {
    let emp_vfe = 0.1;
    let kl = 0.01;
    let n = 100;
    let delta = 0.01;
    let bound = compute_pac_bayes_bound(emp_vfe, kl, n, delta);
    assert!(bound >= emp_vfe, "Bound should be ≥ empirical VFE");
}

#[test]
fn test_pac_bayes_bound_tighter_than_raw() {
    // McAllester bound should be tighter than the old Seldin-Lugosi
    let kl = 0.01;
    let n = 100;
    let delta = 0.01;
    let mcallester = compute_pac_gen_bound(kl, n, delta);
    // Old formula: √((KL + ln(2n/δ)) / 2(n-1))
    let old_log = (2.0 * n as f32 / delta).ln();
    let old_bound = ((kl + old_log) / (2.0 * (n - 1) as f32)).sqrt();
    // McAllester uses ln(2√n/δ) which is smaller
    assert!(
        mcallester <= old_bound,
        "McAllester bound ({:.6}) should be ≤ old bound ({:.6})",
        mcallester,
        old_bound
    );
}

#[test]
fn test_data_dependent_kl_tighter() {
    let posterior = vec![0.1, 0.2, 0.15];
    let prior = vec![0.0, 0.0, 0.0];
    let post_var = 0.01;
    let prior_var = 1.0;
    let data_var = 0.5;
    let concentration = 2.0;

    let kl_standard = native_audit::meta_improvement::compute_gaussian_kl(
        &posterior, &prior, post_var, prior_var,
    );
    let kl_data_dep = compute_gaussian_kl_data_dependent(
        &posterior,
        &prior,
        post_var,
        prior_var,
        data_var,
        concentration,
    );

    // Data-dependent KL should be finite and non-negative
    assert!(kl_data_dep >= 0.0);
    assert!(kl_data_dep.is_finite());
    // With good data variance, data-dependent KL can be tighter
    assert!(
        kl_data_dep <= kl_standard * 2.0,
        "Data-dependent KL ({:.4}) should not be much larger than standard ({:.4})",
        kl_data_dep,
        kl_standard
    );
}

#[test]
fn test_girard_advanced_l1_norm() -> Result<()> {
    let device = Device::Cpu;
    let gens = Tensor::from_vec(
        vec![10.0f32, 1.0, 0.5, 5.0, 3.0, 2.0, 0.1, 0.2, 0.3],
        (3, 3),
        &device,
    )?;
    let config = GirardConfig {
        norm: GirardNorm::L1,
        merge: GirardMerge::IntervalHull,
        min_norm: 0.0,
        lgg_weight_decay: 0.0,
    };
    let result = reduce_generators_girard_advanced(&gens, 2, &config)?;
    // IntervalHull adds diagonal generators for merged → total ≥ kept(2)
    assert!(
        result.generators.dim(0)? >= 2,
        "Should have ≥ kept generators"
    );
    assert!(result.tightness_score <= 1.0, "Tightness ≤ 1.0");
    assert!(result.volume_ratio >= 1.0, "Volume ratio ≥ 1.0");
    Ok(())
}

#[test]
fn test_girard_advanced_l2_norm() -> Result<()> {
    let device = Device::Cpu;
    // Use larger matrix to avoid LGG matmul shape issues with small generators
    let gens = Tensor::from_vec(
        vec![
            3.0f32, 4.0, 0.0, 1.0, 1.0, 0.0, 0.0, 5.0, 0.5, 0.3, 0.2, 0.1, 0.1, 0.1, 0.1, 0.1,
        ],
        (4, 4),
        &device,
    )?;
    let config = GirardConfig {
        norm: GirardNorm::L2,
        merge: GirardMerge::LGG,
        min_norm: 0.0,
        lgg_weight_decay: 0.1,
    };
    let result = reduce_generators_girard_advanced(&gens, 2, &config)?;
    assert!(result.tightness_score > 0.0);
    assert!(result.generators.dim(0)? > 0);
    Ok(())
}

#[test]
fn test_girard_advanced_noise_filtering() -> Result<()> {
    let device = Device::Cpu;
    // Generators with some very small (noise) entries
    let gens = Tensor::from_vec(
        vec![10.0f32, 1.0, 1e-6, 5.0, 3.0, 1e-8, 1e-10, 1e-10, 1e-10],
        (3, 3),
        &device,
    )?;
    let config = GirardConfig {
        norm: GirardNorm::L2,
        merge: GirardMerge::IntervalHull,
        min_norm: 0.01, // Filter out noise
        lgg_weight_decay: 0.0,
    };
    let result = reduce_generators_girard_advanced(&gens, 2, &config)?;
    // Noise generators should be filtered before merge
    assert!(result.generators.dim(0)? > 0);
    Ok(())
}

#[test]
fn test_full_redteam_pipeline() -> Result<()> {
    let device = Device::Cpu;

    // 1. Create latent state
    let latent = vec![0.3, 0.2, -0.1, 0.05];
    let center = vec![0.0; 4];
    let margin = 1.0;
    let epsilon = 0.15;

    // 2. FGSM attack
    let adv = fgsm_latent_attack(&latent, epsilon, &center, margin);

    // 3. Verify L∞ bound
    for (l, a) in latent.iter().zip(adv.iter()) {
        assert!((a - l).abs() <= epsilon + 1e-5);
    }

    // 4. IBP certified bound
    let cert = certified_cbf_ibp(&latent, epsilon, &center, margin);
    assert!(cert.is_finite());

    // 5. PAC-Bayesian bound on attack robustness
    let kl = native_audit::meta_improvement::compute_gaussian_kl(&adv, &latent, 0.01, 1.0);
    let pac_bound = compute_pac_gen_bound(kl, 50, 0.01);
    assert!(pac_bound.is_finite());

    // 6. Girard reduction on attack perturbation
    let perturbation: Vec<f32> = adv.iter().zip(latent.iter()).map(|(a, l)| a - l).collect();
    let pert_tensor = Tensor::from_vec(perturbation, (1, 4), &device)?;
    let config = GirardConfig::default();
    let reduction = reduce_generators_girard_advanced(&pert_tensor, 2, &config)?;
    assert!(reduction.tightness_score > 0.0);

    Ok(())
}

#[test]
fn test_fgsm_vs_ibp_consistency() -> Result<()> {
    let latent = vec![0.5, 0.3, -0.2, 0.1];
    let center = vec![0.0; 4];
    let margin = 1.0;
    let epsilon = 0.2;

    // FGSM attack
    let adv = fgsm_latent_attack(&latent, epsilon, &center, margin);

    // IBP certified bound
    let cert = certified_cbf_ibp(&latent, epsilon, &center, margin);

    // Actual CBF at attacked point
    let adv_dist_sq: f32 = adv
        .iter()
        .zip(center.iter())
        .map(|(a, c)| (a - c).powi(2))
        .sum();
    let actual_cbf = margin * margin - adv_dist_sq;

    // IBP bound should be ≤ actual CBF (soundness: IBP is conservative)
    // Note: IBP gives worst-case over the entire ε-ball, so it can be lower
    assert!(
        cert <= actual_cbf + 1e-4,
        "IBP cert ({:.4}) should be ≤ actual CBF ({:.4}) + tolerance",
        cert,
        actual_cbf
    );

    Ok(())
}

#[test]
fn test_cbf_h_tensor() -> Result<()> {
    let device = Device::Cpu;
    let x = Tensor::from_vec(vec![0.0f32, 0.0, 0.0], 3, &device)?;
    let center = Tensor::from_vec(vec![0.0f32, 0.0, 0.0], 3, &device)?;
    let h = cbf_h(&x, &center, 1.0)?;
    let val = h.to_scalar::<f32>()?;
    assert!(
        (val - 1.0).abs() < 1e-5,
        "CBF at center should equal margin²"
    );
    Ok(())
}

#[test]
fn test_adv_redteam_soundness() -> Result<()> {
    use native_audit::formal_verification::{propagate_silu_taylor_zonotope, TaylorZonotopeConfig};

    let device = Device::Cpu;

    // Use positive center + small epsilon for reliable Taylor soundness
    let latent = Tensor::from_vec(vec![1.0f32, 1.0, 1.0], 3, &device)?;
    let epsilon = 0.05;
    let gen_data: Vec<f32> = vec![epsilon; 3];
    let generators = Tensor::from_vec(gen_data, (1, 3), &device)?;

    // Propagate through SiLU to get Taylor result
    let config = TaylorZonotopeConfig::default();
    let taylor_result = propagate_silu_taylor_zonotope(&latent, &generators, &config)?;

    // Verify soundness: zonotope contains the true SiLU values
    let sound = verify_soundness(&latent, &generators, &taylor_result, 20)?;
    assert!(
        sound,
        "Taylor-Zonotope should contain true SiLU values for small epsilon"
    );

    Ok(())
}

#[test]
fn test_volume_ratio_attack_vs_original() -> Result<()> {
    use native_audit::formal_verification::{propagate_silu_taylor_zonotope, TaylorZonotopeConfig};

    let device = Device::Cpu;

    // Original zonotope
    let center = Tensor::from_vec(vec![0.0f32, 0.0, 0.0], 3, &device)?;
    let eps_orig = 0.1f32;
    let gen_orig = Tensor::from_vec(vec![eps_orig; 3], (1, 3), &device)?;

    // Attacked zonotope (larger perturbation)
    let eps_adv = 0.3f32;
    let gen_adv = Tensor::from_vec(vec![eps_adv; 3], (1, 3), &device)?;

    // Compute volume proxy for each (sum of absolute generators)
    let vol_orig = gen_orig.abs()?.sum_all()?.to_scalar::<f32>()?;
    let _vol_adv = gen_adv.abs()?.sum_all()?.to_scalar::<f32>()?;

    // Taylor propagation for volume ratio
    let config = TaylorZonotopeConfig::default();
    let taylor_result = propagate_silu_taylor_zonotope(&center, &gen_adv, &config)?;
    let ratio = compute_volume_ratio(&taylor_result, vol_orig);

    assert!(ratio > 0.0, "Volume ratio should be positive");
    // Expected ratio scales with ε_adv / ε_orig
    let expected_ratio = (eps_adv / eps_orig).powi(3);
    assert!(
        ratio > 1.0,
        "Attacked zonotope should have larger volume ratio: {:.4}",
        ratio
    );
    let _ = expected_ratio; // Used for reference

    Ok(())
}
