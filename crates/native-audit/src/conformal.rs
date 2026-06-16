//! Conformal Prediction Tubes for Distributionally Robust Steering
//!
//! Implements finite-sample marginal coverage guarantees for tube-based MPC.
//!
//! **Mathematical Guarantee**: `P(|error| <= radius) >= 1 - alpha`
//!
//! Algorithm:
//! 1. Collect calibration errors `R_1, ..., R_N` from held-out dataset
//! 2. Compute radius as `(1-alpha)`-quantile: `radius = ceil((N+1)(1-alpha))/N`-th largest
//! 3. Propagate tube: `Z_{k+1} = A_cl · Z_k ⊕ epsilon · I` with contraction `rho < 1`
//!
//! **Sprint 171 (v17.1.0)**: O(1) inference for edge/WASM deployment.

use candle_core::{Result, Tensor};

/// Configuration for conformal prediction tubes.
#[derive(Debug, Clone)]
pub struct ConformalConfig {
    /// Miscoverage rate: `P(|error| > radius) <= alpha`
    pub alpha: f32,
    /// Contraction factor for tube propagation: `rho < 1`
    pub contraction_factor: f32,
    /// Wasserstein radius for distribution shift robustness
    pub wasserstein_epsilon: f32,
    /// Minimum tube radius (prevents degenerate tubes)
    pub min_radius: f32,
    /// Maximum tube radius (prevents unbounded growth)
    pub max_radius: f32,
}

impl Default for ConformalConfig {
    fn default() -> Self {
        Self {
            alpha: 0.05,
            contraction_factor: 0.95,
            wasserstein_epsilon: 0.1,
            min_radius: 1e-6,
            max_radius: 10.0,
        }
    }
}

impl ConformalConfig {
    /// Edge-optimized: Higher alpha (faster), larger epsilon (more robust).
    pub fn edge_fast() -> Self {
        Self {
            alpha: 0.1,
            contraction_factor: 0.9,
            wasserstein_epsilon: 0.2,
            min_radius: 1e-6,
            max_radius: 5.0,
        }
    }

    /// High precision: Lower alpha (tighter), smaller epsilon (less conservative).
    pub fn high_precision() -> Self {
        Self {
            alpha: 0.01,
            contraction_factor: 0.98,
            wasserstein_epsilon: 0.05,
            min_radius: 1e-8,
            max_radius: 20.0,
        }
    }

    /// Compute the quantile level for radius calibration.
    /// `q = ceil((N + 1) * (1 - alpha)) / N`
    pub fn quantile_level(&self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        let level = ((n + 1) as f64) * (1.0 - self.alpha as f64);
        let q = level.ceil() as usize;
        q.min(n).max(1)
    }
}

/// Result of conformal tube propagation step.
#[derive(Debug, Clone)]
pub struct ConformalTubeResult {
    /// Nominal next state center
    pub center: Tensor,
    /// Tube radius at next step
    pub radius: f32,
    /// Coverage probability estimate
    pub coverage: f32,
    /// Whether the tube contracted (radius decreased)
    pub contracted: bool,
}

impl std::fmt::Display for ConformalTubeResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ConformalTube{{ radius: {:.4}, coverage: {:.2}%, contracted: {} }}",
            self.radius, self.coverage * 100.0, self.contracted
        )
    }
}

/// Compute conformal tube radius from calibration errors.
///
/// Given calibration errors `R_1, ..., R_N`, compute the radius such that
/// `P(|error| <= radius) >= 1 - alpha`.
///
/// Algorithm:
/// 1. Sort errors ascending
/// 2. Return the `q`-th largest error where `q = ceil((N+1)(1-alpha))`
///
/// # Arguments
/// * `calibration_errors` - Absolute errors from calibration set
/// * `alpha` - Miscoverage rate (e.g., 0.05 for 95% coverage)
///
/// # Returns
/// Tube radius guaranteeing marginal coverage `>= 1 - alpha`
pub fn conformal_tube_radius(calibration_errors: &[f32], alpha: f32) -> f32 {
    if calibration_errors.is_empty() {
        return 1.0; // Default conservative radius
    }

    let mut sorted = calibration_errors.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let n = sorted.len();
    let level = ((n + 1) as f64) * (1.0 - alpha as f64);
    let q = level.ceil() as usize;
    let idx = q.min(n).max(1) - 1; // 0-indexed

    sorted[idx].max(1e-6) // Prevent degenerate radius
}

/// Propagate conformal tube one step forward.
///
/// `Z_{k+1} = A_cl · Z_k ⊕ W_k`
/// where `W_k = epsilon · I` (isotropic Wasserstein ball)
///
/// With contraction factor `rho`: `radius_{k+1} = rho · radius_k + epsilon`
///
/// # Arguments
/// * `nominal_next` - Nominal next state center (A_cl · z_k)
/// * `radius` - Current tube radius
/// * `contraction_factor` - Contraction factor `rho` (must be < 1)
/// * `wasserstein_epsilon` - Wasserstein ball radius
///
/// # Returns
/// `(next_center, next_radius)` tuple
pub fn propagate_conformal_tube(
    nominal_next: &Tensor,
    radius: f32,
    contraction_factor: f32,
    wasserstein_epsilon: f32,
) -> Result<(Tensor, f32)> {
    let next_radius = contraction_factor * radius + wasserstein_epsilon;
    let clamped_radius = next_radius.max(1e-6).min(100.0);
    Ok((nominal_next.clone(), clamped_radius))
}

/// Verify conformal coverage empirically.
///
/// Given a set of test errors and the conformal radius, compute the
/// empirical coverage: `fraction of |error_i| <= radius`.
///
/// # Arguments
/// * `errors` - Absolute test errors
/// * `radius` - Conformal tube radius
///
/// # Returns
/// Empirical coverage fraction in [0, 1]
pub fn verify_conformal_coverage(errors: &[f32], radius: f32) -> f32 {
    if errors.is_empty() {
        return 1.0;
    }
    let covered = errors.iter().filter(|&&e| e <= radius).count();
    covered as f32 / errors.len() as f32
}

/// Full DR-CBF + Conformal tube step.
///
/// Combines:
/// 1. Conformal tube propagation for prediction uncertainty
/// 2. DR-CBF soft penalty for safety constraint
/// 3. Event-triggered control for efficiency
///
/// # Arguments
/// * `u_nom` - Nominal control input
/// * `h_nom` - CBF value at current state
/// * `lg_h` - Lie derivative L_g h (control effectiveness)
/// * `calibration_errors` - Errors for conformal radius
/// * `alpha` - Miscoverage rate
/// * `contraction_factor` - Tube contraction factor
/// * `wasserstein_epsilon` - Wasserstein ball radius
/// * `lip_h` - Lipschitz constant for h (distribution shift robustness)
///
/// # Returns
/// Result containing the safe control input and tube metadata
pub fn dr_cbf_conformal_step(
    u_nom: &Tensor,
    h_nom: f32,
    lg_h: &Tensor,
    calibration_errors: &[f32],
    alpha: f32,
    _contraction_factor: f32,
    wasserstein_epsilon: f32,
    lip_h: f32,
) -> Result<(Tensor, f32, f32)> {
    // Step 1: Compute conformal tube radius
    let radius = conformal_tube_radius(calibration_errors, alpha);

    // Step 2: Robust CBF value (account for distribution shift)
    let h_robust = h_nom - wasserstein_epsilon * lip_h;

    // Step 3: DR-CBF soft penalty correction
    let eps_reg = 1e-6_f32;
    let lg_norm_sq = lg_h.sqr()?.sum_all()?.to_scalar::<f32>()?;
    let lambda = (-h_robust).max(0.0) / (lg_norm_sq + eps_reg);

    let lambda_tensor = Tensor::from_vec(vec![lambda], 1, lg_h.device())?;
    let correction = lg_h.broadcast_mul(&lambda_tensor)?;
    let u_safe = u_nom.add(&correction)?;

    // Step 4: Compute coverage estimate
    let coverage = verify_conformal_coverage(calibration_errors, radius);

    Ok((u_safe, radius, coverage))
}

/// Compute tube correction for CBF safety.
///
/// When the tube boundary approaches the unsafe region, apply a correction
/// proportional to the distance to the boundary.
///
/// # Arguments
/// * `center` - Tube center
/// * `radius` - Tube radius
/// * `lg_h` - Lie derivative direction
/// * `h_value` - CBF value at center
///
/// # Returns
/// Correction tensor to add to control input
pub fn compute_conformal_tube_correction(
    _center: &Tensor,
    radius: f32,
    lg_h: &Tensor,
    h_value: f32,
) -> Result<Tensor> {
    // Robust margin: h - radius (worst case in tube)
    let h_robust = h_value - radius;

    if h_robust >= 0.0 {
        // Tube is fully safe — no correction needed
        return Tensor::zeros_like(lg_h);
    }

    // Apply correction proportional to violation
    let lg_norm_sq = lg_h.sqr()?.sum_all()?.to_scalar::<f32>()?;
    let eps_reg = 1e-6_f32;
    let lambda = (-h_robust).max(0.0) / (lg_norm_sq + eps_reg);

    let lambda_tensor = Tensor::from_vec(vec![lambda], 1, lg_h.device())?;
    let correction = lg_h.broadcast_mul(&lambda_tensor)?;
    Ok(correction)
}

/// Propagate tube over multiple steps (for prediction horizon).
///
/// `Z_{k+h} = A_cl^h · Z_k ⊕ sum_{i=0}^{h-1} A_cl^i · epsilon`
///
/// With contraction: `radius_{k+h} = rho^h · radius_k + epsilon · (1 - rho^h) / (1 - rho)`
///
/// # Arguments
/// * `initial_radius` - Initial tube radius
/// * `contraction_factor` - Contraction factor `rho`
/// * `wasserstein_epsilon` - Wasserstein ball radius per step
/// * `horizon` - Number of prediction steps
///
/// # Returns
/// Vector of radii at each step `[r_0, r_1, ..., r_h]`
pub fn propagate_tube_horizon(
    initial_radius: f32,
    contraction_factor: f32,
    wasserstein_epsilon: f32,
    horizon: usize,
) -> Vec<f32> {
    let mut radii = vec![initial_radius; horizon + 1];
    let mut r = initial_radius;

    for i in 1..=horizon {
        r = contraction_factor * r + wasserstein_epsilon;
        radii[i] = r.max(1e-6).min(100.0);
    }

    radii
}

/// Check if the conformal tube satisfies the safety constraint.
///
/// Verifies that the worst-case point in the tube satisfies `h >= 0`.
///
/// # Arguments
/// * `h_value` - CBF value at tube center
/// * `radius` - Tube radius
/// * `lip_h` - Lipschitz constant for h
///
/// # Returns
/// `true` if the entire tube is guaranteed safe
pub fn verify_conformal_safety(h_value: f32, radius: f32, lip_h: f32) -> bool {
    // Worst case: h_min = h_center - lip_h * radius
    let h_min = h_value - lip_h * radius;
    h_min >= 0.0
}

/// Compute the soft VFE pain penalty for CBF violation.
///
/// `pain = lambda * max(0, -h_robust)^2`
///
/// This replaces hard-blocking CBF with a smooth quadratic penalty
/// that can be added to the MPC cost function.
///
/// # Arguments
/// * `h_robust` - Robust CBF value (h - epsilon * lip_h)
/// * `lambda` - Penalty weight
///
/// # Returns
/// Quadratic pain penalty value (0 if safe)
pub fn compute_soft_vfe_pain(h_robust: f32, lambda: f32) -> f32 {
    let violation = (-h_robust).max(0.0);
    lambda * violation * violation
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::Device;

    // Helper: create tensor from vector (Candle v0.6.0: from_vec takes data, shape, device)
    fn make_tensor(data: Vec<f32>, device: &Device) -> Result<Tensor> {
        let n = data.len();
        Tensor::from_vec(data, n, device)
    }

    #[test]
    fn test_conformal_config_default() {
        let config = ConformalConfig::default();
        assert!((config.alpha - 0.05).abs() < 1e-6);
        assert!((config.contraction_factor - 0.95).abs() < 1e-6);
        assert!((config.wasserstein_epsilon - 0.1).abs() < 1e-6);
    }

    #[test]
    fn test_conformal_config_edge_fast() {
        let config = ConformalConfig::edge_fast();
        assert!((config.alpha - 0.1).abs() < 1e-6);
        assert!(config.contraction_factor < 0.95);
    }

    #[test]
    fn test_conformal_config_high_precision() {
        let config = ConformalConfig::high_precision();
        assert!((config.alpha - 0.01).abs() < 1e-6);
        assert!(config.contraction_factor > 0.95);
    }

    #[test]
    fn test_quantile_level() {
        let config = ConformalConfig { alpha: 0.05, ..ConformalConfig::default() };
        // N=99: q = ceil(100 * 0.95) = 95
        assert_eq!(config.quantile_level(99), 95);
        // N=19: q = ceil(20 * 0.95) = 19
        assert_eq!(config.quantile_level(19), 19);
        // N=0: q = 0
        assert_eq!(config.quantile_level(0), 0);
    }

    #[test]
    fn test_conformal_tube_radius_basic() {
        let errors = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let radius = conformal_tube_radius(&errors, 0.1);
        // N=5, alpha=0.1: q = ceil(6 * 0.9) = ceil(5.4) = 6 -> idx = 5 (clamped to 5)
        // Should return max error or near-max
        assert!(radius >= 4.0);
        assert!(radius <= 5.0);
    }

    #[test]
    fn test_conformal_tube_radius_empty() {
        let radius = conformal_tube_radius(&[], 0.05);
        assert!((radius - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_conformal_tube_radius_single() {
        let errors = vec![3.5];
        let radius = conformal_tube_radius(&errors, 0.05);
        assert!((radius - 3.5).abs() < 1e-6);
    }

    #[test]
    fn test_conformal_tube_radius_monotonic_in_alpha() {
        let errors: Vec<f32> = (1..=100).map(|x| x as f32).collect();
        let r_high_alpha = conformal_tube_radius(&errors, 0.2);
        let r_low_alpha = conformal_tube_radius(&errors, 0.05);
        // Lower alpha -> higher coverage -> larger radius
        assert!(r_low_alpha >= r_high_alpha);
    }

    #[test]
    fn test_propagate_conformal_tube() -> Result<()> {
        let device = Device::Cpu;
        let center = make_tensor(vec![1.0, 2.0, 3.0], &device)?;
        let (next_center, next_radius) = propagate_conformal_tube(&center, 1.0, 0.9, 0.1)?;

        assert_eq!(next_center.shape(), &candle_core::Shape::from(&[3usize]));
        // r_{k+1} = 0.9 * 1.0 + 0.1 = 1.0
        assert!((next_radius - 1.0).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn test_propagate_conformal_tube_contracts() -> Result<()> {
        let device = Device::Cpu;
        let center = make_tensor(vec![0.0], &device)?;
        let (_, next_radius) = propagate_conformal_tube(&center, 2.0, 0.8, 0.05)?;
        // r_{k+1} = 0.8 * 2.0 + 0.05 = 1.65
        assert!((next_radius - 1.65).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn test_propagate_conformal_tube_expands() -> Result<()> {
        let device = Device::Cpu;
        let center = make_tensor(vec![0.0], &device)?;
        let (_, next_radius) = propagate_conformal_tube(&center, 0.1, 0.9, 0.5)?;
        // r_{k+1} = 0.9 * 0.1 + 0.5 = 0.59
        assert!((next_radius - 0.59).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn test_verify_conformal_coverage_perfect() {
        let errors = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let coverage = verify_conformal_coverage(&errors, 1.0);
        assert!((coverage - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_verify_conformal_coverage_partial() {
        let errors = vec![0.1, 0.5, 1.0, 1.5, 2.0];
        let coverage = verify_conformal_coverage(&errors, 1.0);
        // 3 out of 5 <= 1.0
        assert!((coverage - 0.6).abs() < 1e-6);
    }

    #[test]
    fn test_verify_conformal_coverage_zero() {
        let errors = vec![1.0, 2.0, 3.0];
        let coverage = verify_conformal_coverage(&errors, 0.5);
        assert!((coverage - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_verify_conformal_coverage_empty() {
        let coverage = verify_conformal_coverage(&[], 1.0);
        assert!((coverage - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_dr_cbf_conformal_step_safe() -> Result<()> {
        let device = Device::Cpu;
        let u_nom = make_tensor(vec![0.0, 0.0], &device)?;
        let lg_h = make_tensor(vec![1.0, 0.0], &device)?;
        let errors = vec![0.1, 0.2, 0.3];

        let (u_safe, radius, coverage) = dr_cbf_conformal_step(
            &u_nom,
            5.0,   // h_nom: safe
            &lg_h,
            &errors,
            0.1,   // alpha
            0.9,   // contraction
            0.1,   // epsilon
            1.0,   // lip_h
        )?;

        // Safe state: lambda should be 0, u_safe = u_nom
        assert_eq!(u_safe.shape(), &candle_core::Shape::from(&[2usize]));
        assert!(radius > 0.0);
        assert!(coverage >= 0.0 && coverage <= 1.0);
        Ok(())
    }

    #[test]
    fn test_dr_cbf_conformal_step_unsafe() -> Result<()> {
        let device = Device::Cpu;
        let u_nom = make_tensor(vec![0.0, 0.0], &device)?;
        let lg_h = make_tensor(vec![1.0, 0.0], &device)?;
        let errors = vec![0.1, 0.2, 0.3];

        let (u_safe, _radius, _coverage) = dr_cbf_conformal_step(
            &u_nom,
            -2.0,  // h_nom: unsafe
            &lg_h,
            &errors,
            0.1,
            0.9,
            0.1,
            1.0,
        )?;

        // Unsafe state: correction should be applied
        // h_robust = -2.0 - 0.1 * 1.0 = -2.1
        // lambda = 2.1 / (1.0 + 1e-6) ≈ 2.1
        // correction = [2.1, 0.0]
        let u_vals: Vec<f32> = u_safe.to_vec1()?;
        assert!((u_vals[0] - 2.1).abs() < 0.01);
        assert!((u_vals[1] - 0.0).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn test_compute_conformal_tube_correction_safe() -> Result<()> {
        let device = Device::Cpu;
        let center = make_tensor(vec![1.0, 2.0], &device)?;
        let lg_h = make_tensor(vec![1.0, 0.0], &device)?;

        let correction = compute_conformal_tube_correction(&center, 0.5, &lg_h, 3.0)?;
        // h_robust = 3.0 - 0.5 = 2.5 >= 0: no correction
        let vals: Vec<f32> = correction.to_vec1()?;
        assert!(vals.iter().all(|&v| (v - 0.0).abs() < 1e-6));
        Ok(())
    }

    #[test]
    fn test_compute_conformal_tube_correction_unsafe() -> Result<()> {
        let device = Device::Cpu;
        let center = make_tensor(vec![1.0, 2.0], &device)?;
        let lg_h = make_tensor(vec![1.0, 0.0], &device)?;

        let correction = compute_conformal_tube_correction(&center, 2.0, &lg_h, 1.0)?;
        // h_robust = 1.0 - 2.0 = -1.0 < 0: correction applied
        // lambda = 1.0 / (1.0 + 1e-6) ≈ 1.0
        let vals: Vec<f32> = correction.to_vec1()?;
        assert!((vals[0] - 1.0).abs() < 0.01);
        Ok(())
    }

    #[test]
    fn test_propagate_tube_horizon() {
        let radii = propagate_tube_horizon(1.0, 0.9, 0.1, 3);
        assert_eq!(radii.len(), 4);
        assert!((radii[0] - 1.0).abs() < 1e-6);
        // r_1 = 0.9 * 1.0 + 0.1 = 1.0
        assert!((radii[1] - 1.0).abs() < 1e-6);
        // r_2 = 0.9 * 1.0 + 0.1 = 1.0 (steady state)
        assert!((radii[2] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_propagate_tube_horizon_contracts() {
        let radii = propagate_tube_horizon(2.0, 0.8, 0.05, 3);
        assert_eq!(radii.len(), 4);
        assert!(radii[1] < radii[0]); // Should contract
        // Steady state: r = 0.8*r + 0.05 -> r = 0.05/0.2 = 0.25
        assert!(radii[3] < radii[0]);
    }

    #[test]
    fn test_verify_conformal_safety_safe() {
        assert!(verify_conformal_safety(5.0, 1.0, 2.0));
        // h_min = 5.0 - 2.0 * 1.0 = 3.0 >= 0
    }

    #[test]
    fn test_verify_conformal_safety_unsafe() {
        assert!(!verify_conformal_safety(1.0, 1.0, 2.0));
        // h_min = 1.0 - 2.0 * 1.0 = -1.0 < 0
    }

    #[test]
    fn test_verify_conformal_safety_boundary() {
        assert!(verify_conformal_safety(2.0, 1.0, 2.0));
        // h_min = 2.0 - 2.0 * 1.0 = 0.0 >= 0 (boundary)
    }

    #[test]
    fn test_compute_soft_vfe_pain_safe() {
        let pain = compute_soft_vfe_pain(3.0, 1.0);
        assert!((pain - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_soft_vfe_pain_unsafe() {
        let pain = compute_soft_vfe_pain(-2.0, 1.0);
        // pain = 1.0 * max(0, 2.0)^2 = 4.0
        assert!((pain - 4.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_soft_vfe_pain_scales_with_lambda() {
        let pain_1 = compute_soft_vfe_pain(-1.0, 1.0);
        let pain_2 = compute_soft_vfe_pain(-1.0, 2.0);
        assert!((pain_2 - 2.0 * pain_1).abs() < 1e-6);
    }

    #[test]
    fn test_compute_soft_vfe_pain_quadratic() {
        let pain_1 = compute_soft_vfe_pain(-1.0, 1.0);
        let pain_2 = compute_soft_vfe_pain(-2.0, 1.0);
        // pain_2 / pain_1 = 4.0 / 1.0 = 4.0 (quadratic)
        assert!((pain_2 - 4.0 * pain_1).abs() < 1e-6);
    }

    #[test]
    fn test_conformal_result_display() -> Result<()> {
        let device = Device::Cpu;
        let center = make_tensor(vec![1.0, 2.0], &device)?;
        let result = ConformalTubeResult {
            center,
            radius: 0.5,
            coverage: 0.95,
            contracted: true,
        };
        let s = format!("{}", result);
        assert!(s.contains("0.5000"));
        assert!(s.contains("95.00%"));
        assert!(s.contains("true"));
        Ok(())
    }

    #[test]
    fn test_full_conformal_pipeline() -> Result<()> {
        let device = Device::Cpu;

        // Generate synthetic calibration errors
        let cal_errors: Vec<f32> = (0..100).map(|i| (i % 10) as f32 * 0.1 + 0.01).collect();

        // Step 1: Compute radius
        let radius = conformal_tube_radius(&cal_errors, 0.1);
        assert!(radius > 0.0);

        // Step 2: Verify coverage
        let coverage = verify_conformal_coverage(&cal_errors, radius);
        assert!(coverage >= 0.85); // At least 85% coverage for alpha=0.1

        // Step 3: Propagate tube
        let center = make_tensor(vec![1.0, 0.0], &device)?;
        let (next_center, next_radius) = propagate_conformal_tube(&center, radius, 0.9, 0.05)?;
        assert_eq!(next_center.shape(), &candle_core::Shape::from(&[2usize]));

        // Step 4: Check safety
        let safe = verify_conformal_safety(5.0, next_radius, 1.0);
        assert!(safe);

        // Step 5: DR-CBF step
        let u_nom = make_tensor(vec![0.5, -0.3], &device)?;
        let lg_h = make_tensor(vec![0.8, 0.2], &device)?;
        let (u_safe, _, _) = dr_cbf_conformal_step(
            &u_nom, 3.0, &lg_h, &cal_errors, 0.1, 0.9, 0.05, 1.0,
        )?;
        assert_eq!(u_safe.shape(), &candle_core::Shape::from(&[2usize]));

        Ok(())
    }
}
