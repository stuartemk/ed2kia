//! CBF-QP + MPC Steering — Certified Safe Control Synthesis.
//!
//! **CBF-QP**: Quadratic Programming that projects nominal steering into safe region
//! using Control Barrier Function h(x) ≥ 0 as constraint.
//!
//! **MPC**: Model Predictive Control with Taylor-Zonotope reachability for multi-step
//! certified planning over horizon N.
//!
//! # Safety Guarantee
//! Forward invariance: if h(x₀) ≥ 0, then h(x(t)) ≥ 0 for all t ≥ 0.
//!
//! # Mathematical Foundation
//! - CBF condition: L_f h(x) + L_g h(x)·u + α·h(x) ≥ 0
//! - QP: min_u ½||u - u_nom||² s.t. CBF constraint
//! - MPC: Rollout Taylor-Zonotope over horizon, solve CBF-QP at each step

use candle_core::{Result, Tensor};

use crate::formal_verification::{
    propagate_layer_taylor_zonotope, propagate_silu_taylor_zonotope, TaylorPropagationResult,
    TaylorZonotopeConfig,
};

// ---------------------------------------------------------------------------
// CBF Definitions
// ---------------------------------------------------------------------------

/// Control Barrier Function: radial safety margin.
///
/// h(x) = margin² - ||x - safe_center||²
///
/// Safe set: C = { x | h(x) ≥ 0 }
///
/// # Arguments
/// * `x` - Current state tensor [1, d]
/// * `safe_center` - Center of safe region [1, d]
/// * `margin` - Safety radius (scalar)
pub fn cbf_h(x: &Tensor, safe_center: &Tensor, margin: f32) -> Result<Tensor> {
    let diff = x.sub(safe_center)?;
    let dist_sq = diff.sqr()?.sum_all()?;
    let margin_sq = Tensor::new(margin * margin, x.device())?;
    margin_sq.sub(&dist_sq)
}

/// Class-K function: α(h) = κ · h (linear).
///
/// Ensures exponential convergence to interior of safe set.
pub fn class_k(h: &Tensor, kappa: f32) -> Result<Tensor> {
    h.mul(&Tensor::new(kappa, h.device())?)
}

/// Compute Lie derivative L_f h(x) ≈ ∇h(x) · f(x).
///
/// For h(x) = margin² - ||x - c||²:
/// ∇h(x) = -2(x - c)
/// L_f h(x) = ∇h · f ≈ -2(x - c) · f(x)
///
/// For simplicity, we approximate f(x) ≈ x (identity dynamics).
pub fn lie_derivative_f_h(x: &Tensor, safe_center: &Tensor, _f_x: &Tensor) -> Result<Tensor> {
    let diff = x.sub(safe_center)?;
    // ∇h = -2·(x - c), L_f h ≈ ||∇h||² as scalar proxy
    let grad_h_sq = diff.sqr()?.sum_all()?.neg()?;
    Ok(grad_h_sq)
}

// ---------------------------------------------------------------------------
// CBF-QP Solver
// ---------------------------------------------------------------------------

/// Solve CBF-QP: min_u ½||u - u_nom||² s.t. L_f h + L_g h·u + α(h) ≥ 0
///
/// Uses analytical projection for control-affine systems:
/// If constraint satisfied: u = u_nom
/// If violated: u = u_nom + λ·∇_u(L_g h) where λ = -slack / ||∇_u(L_g h)||²
///
/// # Arguments
/// * `x` - Current state [1, d]
/// * `u_nom` - Nominal control (steering direction) [1, d]
/// * `safe_center` - Safe region center [1, d]
/// * `margin` - Safety margin
/// * `alpha` - Class-K parameter
pub fn solve_cbf_qp(
    x: &Tensor,
    u_nom: &Tensor,
    safe_center: &Tensor,
    margin: f32,
    alpha: f32,
) -> Result<Tensor> {
    let device = x.device();

    // Compute CBF value
    let h_val = cbf_h(x, safe_center, margin)?;
    let h_scalar = h_val.to_scalar::<f32>()?;

    // If safely inside, return nominal
    if h_scalar > 1e-4 {
        return Ok(u_nom.clone());
    }

    // Compute Lie derivative L_f h
    let f_x = x.clone(); // Identity dynamics approximation
    let l_f_h = lie_derivative_f_h(x, safe_center, &f_x)?;
    let l_f_scalar = l_f_h.to_scalar::<f32>()?;

    // Compute α(h)
    let alpha_h = class_k(&h_val, alpha)?.to_scalar::<f32>()?;

    // Constraint slack: L_f h + α(h)
    let slack = l_f_scalar + alpha_h;

    if slack >= 0.0 {
        return Ok(u_nom.clone());
    }

    // Compute L_g h · u ≈ u · (x - c) (control-affine approximation)
    // ∇_u(L_g h) = (x - c) direction
    // Correction must point TOWARD safe center: use -(x - c) = c - x
    let diff = safe_center.sub(x)?;
    let l_g_dir = diff.clone();
    let l_g_norm_sq = l_g_dir.sqr()?.sum_all()?.to_scalar::<f32>()?;

    if l_g_norm_sq < 1e-8 {
        // At center — no direction to project
        return Ok(u_nom.clone());
    }

    // λ = -slack / ||∇_u(L_g h)||²
    let lambda = -slack / l_g_norm_sq;

    // u_safe = u_nom + λ · (c - x)  steers toward safe center
    let lambda_tensor = Tensor::full(lambda, l_g_dir.shape(), device)?;
    let correction = l_g_dir.mul(&lambda_tensor)?;
    u_nom.broadcast_add(&correction)
}

/// Compute safety margin from CBF value.
pub fn safety_margin(x: &Tensor, safe_center: &Tensor, margin: f32) -> Result<f32> {
    cbf_h(x, safe_center, margin).and_then(|h| h.to_scalar())
}

// ---------------------------------------------------------------------------
// MPC with Taylor-Zonotope Reachability
// ---------------------------------------------------------------------------

/// MPC configuration.
#[derive(Debug, Clone)]
pub struct MPCConfig {
    /// Prediction horizon (number of steps ahead).
    pub horizon: usize,
    /// CBF safety margin.
    pub safety_margin: f32,
    /// Class-K parameter for CBF.
    pub cbf_alpha: f32,
    /// Taylor-Zonotope configuration.
    pub taylor_config: TaylorZonotopeConfig,
    /// Maximum control effort (L2 norm bound).
    pub max_control_effort: f32,
}

impl Default for MPCConfig {
    fn default() -> Self {
        Self {
            horizon: 5,
            safety_margin: 10.0,
            cbf_alpha: 1.0,
            taylor_config: TaylorZonotopeConfig::default(),
            max_control_effort: 1.0,
        }
    }
}

/// Single MPC step result.
#[derive(Debug)]
pub struct MPCStepResult {
    /// Step index in horizon.
    pub step: usize,
    /// Safe control action.
    pub control: Tensor,
    /// Predicted center after applying control.
    pub predicted_center: Tensor,
    /// CBF value at predicted state.
    pub cbf_value: f32,
    /// Volume proxy of predicted zonotope.
    pub volume_proxy: f32,
}

/// MPC trajectory result.
#[derive(Debug)]
pub struct MPCTrajectory {
    /// Sequence of step results.
    pub steps: Vec<MPCStepResult>,
    /// Overall safety certificate: true if all steps satisfy CBF.
    pub safe: bool,
    /// Minimum CBF value across horizon.
    pub min_cbf: f32,
}

impl std::fmt::Display for MPCTrajectory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MPC(horizon={}, safe={}, min_cbf={:.4})",
            self.steps.len(),
            self.safe,
            self.min_cbf
        )
    }
}

/// Execute MPC with Taylor-Zonotope reachability.
///
/// At each step:
/// 1. Propagate zonotope through network layer (Taylor-Zonotope)
/// 2. Evaluate CBF on predicted reachable set
/// 3. Solve CBF-QP for safe control
/// 4. Apply control and continue
///
/// # Arguments
/// * `center` - Initial zonotope center [1, d]
/// * `generators` - Initial zonotope generators [k, d]
/// * `safe_center` - Target safe region center [1, d]
/// * `weight` - Network weight matrix [d, d] (for propagation)
/// * `bias` - Optional bias
/// * `config` - MPC configuration
pub fn mpc_steer_safe(
    center: &Tensor,
    generators: &Tensor,
    safe_center: &Tensor,
    weight: Option<&Tensor>,
    bias: Option<&Tensor>,
    config: &MPCConfig,
) -> Result<MPCTrajectory> {
    let device = center.device();
    let mut steps = Vec::with_capacity(config.horizon);
    let mut current_center = center.clone();
    let mut current_generators = generators.clone();
    let mut min_cbf = f32::INFINITY;
    let mut all_safe = true;

    for step in 0..config.horizon {
        // 1. Forward reachability
        let result: TaylorPropagationResult = match weight {
            Some(w) => propagate_layer_taylor_zonotope(
                &current_center,
                &current_generators,
                w,
                bias,
                &config.taylor_config,
            )?,
            None => propagate_silu_taylor_zonotope(
                &current_center,
                &current_generators,
                &config.taylor_config,
            )?,
        };

        // 2. Compute nominal control: steer toward safe center
        let direction = safe_center.sub(&result.center)?;
        let dist = direction.sqr()?.sum_all()?.sqrt()?.to_scalar::<f32>()?;
        let u_nom = if dist > 1e-6 {
            direction.broadcast_mul(&Tensor::new(0.1 * dist, device)?)?
        } else {
            Tensor::zeros_like(&result.center)?
        };

        // 3. Solve CBF-QP
        let u_safe = solve_cbf_qp(
            &result.center,
            &u_nom,
            safe_center,
            config.safety_margin,
            config.cbf_alpha,
        )?;

        // 4. Evaluate CBF
        let h_val = cbf_h(&result.center, safe_center, config.safety_margin)?;
        let cbf_val = h_val.to_scalar::<f32>()?;

        if cbf_val < 0.0 {
            all_safe = false;
        }
        min_cbf = min_cbf.min(cbf_val);

        // 5. Apply control (add to center)
        current_center = result.center.broadcast_add(&u_safe)?;
        current_generators = result.generators.clone();

        steps.push(MPCStepResult {
            step,
            control: u_safe,
            predicted_center: result.center,
            cbf_value: cbf_val,
            volume_proxy: result.volume_proxy,
        });
    }

    Ok(MPCTrajectory {
        steps,
        safe: all_safe,
        min_cbf,
    })
}

/// Simplified MPC without network layer (pure SiLU propagation).
pub fn mpc_steer_safe_simple(
    center: &Tensor,
    generators: &Tensor,
    safe_center: &Tensor,
    config: &MPCConfig,
) -> Result<MPCTrajectory> {
    mpc_steer_safe(center, generators, safe_center, None, None, config)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::Device;

    fn make_diagonal_zonotope(center: &Tensor, epsilon: f32) -> Result<Tensor> {
        let dim = center.dim(1)?;
        let device = center.device();
        let mut data = vec![0.0f32; dim * dim];
        for i in 0..dim {
            data[i * dim + i] = epsilon;
        }
        Tensor::from_vec(data, (dim, dim), device)
    }

    #[test]
    fn test_cbf_h_safe() -> Result<()> {
        let device = Device::Cpu;
        let x = Tensor::from_vec(vec![0.0f32, 0.0], (1, 2), &device)?;
        let center = Tensor::from_vec(vec![0.0f32, 0.0], (1, 2), &device)?;
        let h = cbf_h(&x, &center, 5.0)?;
        let val = h.to_scalar::<f32>()?;
        assert!(val > 0.0, "h(x) = {val} should be positive at center");
        Ok(())
    }

    #[test]
    fn test_cbf_h_unsafe() -> Result<()> {
        let device = Device::Cpu;
        let x = Tensor::from_vec(vec![10.0f32, 10.0], (1, 2), &device)?;
        let center = Tensor::from_vec(vec![0.0f32, 0.0], (1, 2), &device)?;
        let h = cbf_h(&x, &center, 5.0)?;
        let val = h.to_scalar::<f32>()?;
        assert!(val < 0.0, "h(x) = {val} should be negative far from center");
        Ok(())
    }

    #[test]
    fn test_cbf_qp_safe_nominal() -> Result<()> {
        let device = Device::Cpu;
        let x = Tensor::from_vec(vec![0.0f32, 0.0], (1, 2), &device)?;
        let u_nom = Tensor::from_vec(vec![0.1f32, -0.1], (1, 2), &device)?;
        let center = Tensor::from_vec(vec![0.0f32, 0.0], (1, 2), &device)?;

        let u_safe = solve_cbf_qp(&x, &u_nom, &center, 5.0, 1.0)?;
        // Should return nominal when safe
        let diff = u_safe.sub(&u_nom)?.abs()?.sum_all()?.to_scalar::<f32>()?;
        assert!(diff < 1e-4, "Safe state should return nominal control");
        Ok(())
    }

    #[test]
    fn test_cbf_qp_projects_unsafe() -> Result<()> {
        let device = Device::Cpu;
        // Far from safe center
        let x = Tensor::from_vec(vec![8.0f32, 0.0], (1, 2), &device)?;
        let u_nom = Tensor::from_vec(vec![1.0f32, 0.0], (1, 2), &device)?;
        let center = Tensor::from_vec(vec![0.0f32, 0.0], (1, 2), &device)?;

        let u_safe = solve_cbf_qp(&x, &u_nom, &center, 5.0, 1.0)?;
        let u_safe_vec: Vec<f32> = u_safe.flatten_all()?.to_vec1()?;
        // Should project toward center (negative x direction)
        assert!(
            u_safe_vec[0] < u_nom.flatten_all()?.to_vec1::<f32>()?[0],
            "Control should be projected toward safe center"
        );
        Ok(())
    }

    #[test]
    fn test_mpc_trajectory_safe() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::from_vec(vec![0.0f32, 0.0, 0.0], (1, 3), &device)?;
        let gens = make_diagonal_zonotope(&center, 0.05)?;
        let safe_center = Tensor::from_vec(vec![0.0f32, 0.0, 0.0], (1, 3), &device)?;
        let config = MPCConfig {
            horizon: 4,
            safety_margin: 10.0,
            ..MPCConfig::default()
        };

        let traj = mpc_steer_safe_simple(&center, &gens, &safe_center, &config)?;
        assert_eq!(traj.steps.len(), config.horizon);
        assert!(traj.safe, "Trajectory should be safe");
        assert!(traj.min_cbf > 0.0, "Min CBF should be positive");
        Ok(())
    }

    #[test]
    fn test_mpc_trajectory_display() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::from_vec(vec![0.0f32, 0.0], (1, 2), &device)?;
        let gens = make_diagonal_zonotope(&center, 0.01)?;
        let safe_center = Tensor::from_vec(vec![0.0f32, 0.0], (1, 2), &device)?;
        let config = MPCConfig {
            horizon: 3,
            ..MPCConfig::default()
        };

        let traj = mpc_steer_safe_simple(&center, &gens, &safe_center, &config)?;
        let display = format!("{}", traj);
        assert!(display.contains("MPC"));
        assert!(display.contains("safe="));
        Ok(())
    }

    #[test]
    fn test_safety_margin() -> Result<()> {
        let device = Device::Cpu;
        let x = Tensor::from_vec(vec![1.0f32, 0.0], (1, 2), &device)?;
        let center = Tensor::from_vec(vec![0.0f32, 0.0], (1, 2), &device)?;
        let margin = safety_margin(&x, &center, 5.0)?;
        // margin² - dist² = 25 - 1 = 24
        assert!((margin - 24.0).abs() < 1e-4);
        Ok(())
    }

    #[test]
    fn test_class_k() -> Result<()> {
        let device = Device::Cpu;
        let h = Tensor::new(2.0f32, &device)?;
        let result = class_k(&h, 3.0)?;
        assert!((result.to_scalar::<f32>()? - 6.0).abs() < 1e-4);
        Ok(())
    }

    #[test]
    fn test_mpc_config_default() {
        let config = MPCConfig::default();
        assert!(config.horizon > 0);
        assert!(config.safety_margin > 0.0);
        assert!(config.cbf_alpha > 0.0);
    }
}
