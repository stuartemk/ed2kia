//! True Koopman Tube MPC — Certified Recursive Zonotope Propagation.
//!
//! Implements the core Tube MPC algorithm with:
//! - **Recursive zonotope propagation**: `T_{k+1} = A_{cl}·T_k ⊕ R_k ⊕ αW`
//! - **Contractive error verification**: `||e_{k+1}||_inf <= ρ·||e_k||_inf + γ` with `ρ < 1`
//! - **PINN-bound residual zonotope**: `R_k` from neural residual bounds
//! - **Disturbance set scaling**: `αW` where α is the contraction factor
//!
//! **Nuclear Equation:**
//! ```text
//! T_{k+1} = A_{cl} · T_k ⊕ R_k ⊕ α·W
//! where:
//!   A_{cl} = K + B·K_gain    (closed-loop Koopman operator)
//!   T_k    = zonotope tube at step k
//!   R_k    = PINN-bound residual zonotope
//!   α      = contraction factor (ρ < 1)
//!   W      = disturbance set zonotope
//! ```
//!
//! **Contractive Constraint:**
//! ```text
//! ||e_{k+1}||_inf <= ρ·||e_k||_inf + γ
//! where ρ < 1 (e.g., 0.92) ensures exponential convergence
//! ```

use candle_core::{DType, Result, Tensor};
use std::fmt;

// Re-export Zonotope for convenience
pub use crate::zonotope::Zonotope;

/// Configuration for True Koopman Tube MPC.
#[derive(Debug, Clone)]
pub struct TubeMPCConfig {
    /// Contraction factor ρ (must be < 1 for stability).
    pub rho: f32,
    /// Additive error bound γ in contractive constraint.
    pub gamma: f32,
    /// Disturbance set scaling factor α.
    pub alpha: f32,
    /// Maximum horizon for tube propagation.
    pub horizon: usize,
    /// Maximum generators per zonotope (controls precision vs. speed).
    pub max_gens: usize,
    /// Enable contractive verification at each step.
    pub verify_contraction: bool,
    /// Tolerance for numerical errors in contraction check.
    pub contraction_tolerance: f32,
}

impl Default for TubeMPCConfig {
    fn default() -> Self {
        Self {
            rho: 0.92,
            gamma: 0.01,
            alpha: 0.5,
            horizon: 10,
            max_gens: 64,
            verify_contraction: true,
            contraction_tolerance: 1e-4,
        }
    }
}

impl TubeMPCConfig {
    /// Create a fast configuration for edge deployment.
    pub fn edge_fast() -> Self {
        Self {
            rho: 0.95,
            gamma: 0.02,
            alpha: 0.3,
            horizon: 5,
            max_gens: 32,
            verify_contraction: true,
            contraction_tolerance: 1e-3,
        }
    }

    /// Create a high-precision configuration for offline verification.
    pub fn high_precision() -> Self {
        Self {
            rho: 0.88,
            gamma: 0.005,
            alpha: 0.7,
            horizon: 20,
            max_gens: 128,
            verify_contraction: true,
            contraction_tolerance: 1e-6,
        }
    }

    /// Set contraction factor.
    pub fn with_rho(mut self, rho: f32) -> Self {
        self.rho = rho.clamp(0.0, 0.99);
        self
    }

    /// Set additive error bound.
    pub fn with_gamma(mut self, gamma: f32) -> Self {
        self.gamma = gamma.max(0.0);
        self
    }

    /// Set disturbance scaling.
    pub fn with_alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha.max(0.0);
        self
    }

    /// Set prediction horizon.
    pub fn with_horizon(mut self, horizon: usize) -> Self {
        self.horizon = horizon.max(1);
        self
    }
}

/// Result of a single tube propagation step.
#[derive(Debug, Clone)]
pub struct TubeStepResult {
    /// Step index k.
    pub step: usize,
    /// Tube zonotope at step k.
    pub tube: Zonotope,
    /// Error bound ||e_k||_inf.
    pub error_bound: f32,
    /// Previous error bound ||e_{k-1}||_inf (0 for first step).
    pub prev_error_bound: f32,
    /// Contraction satisfied: `e_k <= rho * e_{k-1} + gamma`.
    pub contraction_satisfied: bool,
    /// Volume proxy (sum of generator norms).
    pub volume_proxy: f32,
}

impl fmt::Display for TubeStepResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TubeStep(k={}, e_k={:.6}, e_{{k-1}}={:.6}, contracted={}, volume={:.6})",
            self.step,
            self.error_bound,
            self.prev_error_bound,
            self.contraction_satisfied,
            self.volume_proxy
        )
    }
}

/// Result of recursive tube MPC propagation.
#[derive(Debug, Clone)]
pub struct TubeMPCResult {
    /// All step results.
    pub steps: Vec<TubeStepResult>,
    /// Final tube zonotope.
    pub final_tube: Zonotope,
    /// Final error bound.
    pub final_error_bound: f32,
    /// Initial error bound.
    pub initial_error_bound: f32,
    /// Overall contraction ratio: `e_final / e_initial`.
    pub contraction_ratio: f32,
    /// All steps satisfied contraction constraint.
    pub all_contracted: bool,
    /// Number of steps.
    pub num_steps: usize,
    /// Configuration used.
    pub config: TubeMPCConfig,
}

impl fmt::Display for TubeMPCResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TubeMPC(steps={}, e0={:.6}, e_final={:.6}, ratio={:.4}, all_contracted={})",
            self.num_steps,
            self.initial_error_bound,
            self.final_error_bound,
            self.contraction_ratio,
            self.all_contracted
        )
    }
}

/// Propagate a tube recursively using true zonotope arithmetic.
///
/// **Algorithm:**
/// ```text
/// T_0 = point(x_nominal)
/// for k = 0..horizon-1:
///     T_{k+1} = A_{cl} · T_k ⊕ R_k ⊕ α·W
///     e_{k+1} = max_error_bound(T_{k+1})
///     verify: e_{k+1} <= ρ·e_k + γ
/// ```
///
/// # Arguments
/// - `a_cl`: Closed-loop Koopman operator `A_{cl} = K + B·K_gain`, shape `[dim, dim]`.
/// - `x_nominal`: Nominal state (center of initial tube), shape `[1, dim]`.
/// - `residual_generators`: PINN-bound residual generators, shape `[num_res_gens, dim]`.
/// - `disturbance_generators`: Disturbance set generators, shape `[num_dist_gens, dim]`.
/// - `config`: Tube MPC configuration.
///
/// # Returns
/// `TubeMPCResult` with all step results and final tube.
pub fn propagate_tube_recursive(
    a_cl: &Tensor,
    x_nominal: &Tensor,
    residual_generators: &Tensor,
    disturbance_generators: &Tensor,
    config: &TubeMPCConfig,
) -> Result<TubeMPCResult> {
    let device = x_nominal.device();
    let horizon = config.horizon;

    // Create initial point zonotope
    let initial_tube = Zonotope::point(x_nominal)?;
    let initial_error_bound = initial_tube.max_error_bound()?;

    let mut steps = Vec::with_capacity(horizon);
    let mut current_tube = initial_tube;
    let mut prev_error_bound = initial_error_bound;
    let mut all_contracted = true;

    // Pre-scale disturbance generators by α
    let alpha_tensor = Tensor::full(config.alpha, (), device)?;
    let scaled_disturbance = disturbance_generators.broadcast_mul(&alpha_tensor)?;

    for k in 0..horizon {
        // Step 1: Linear map — A_{cl} · T_k
        let mapped = current_tube.linear_map(a_cl)?;

        // Step 2: Minkowski sum with residual zonotope R_k
        // Create residual zonotope from generators (center at origin)
        let residual_center = Tensor::zeros((1, residual_generators.dim(1)?), DType::F32, device)?;
        let residual_zonotope = Zonotope::new(
            residual_center,
            residual_generators.clone(),
            crate::zonotope::ZonotopeConfig {
                max_gens: config.max_gens,
                ..Default::default()
            },
        )?;
        let with_residual = mapped.minkowski_sum(&residual_zonotope)?;

        // Step 3: Minkowski sum with scaled disturbance α·W
        let dist_center = Tensor::zeros((1, scaled_disturbance.dim(1)?), DType::F32, device)?;
        let dist_zonotope = Zonotope::new(
            dist_center,
            scaled_disturbance.clone(),
            crate::zonotope::ZonotopeConfig {
                max_gens: config.max_gens,
                ..Default::default()
            },
        )?;
        let new_tube = with_residual.minkowski_sum(&dist_zonotope)?;

        // Compute error bound
        let error_bound = new_tube.max_error_bound()?;
        let volume_proxy = new_tube.volume_proxy()?;

        // Verify contraction
        let contraction_satisfied = if config.verify_contraction && k > 0 {
            let allowed = config.rho * prev_error_bound + config.gamma;
            let satisfied = error_bound <= allowed + config.contraction_tolerance;
            if !satisfied {
                all_contracted = false;
            }
            satisfied
        } else {
            true
        };

        let step_result = TubeStepResult {
            step: k,
            tube: new_tube.clone(),
            error_bound,
            prev_error_bound,
            contraction_satisfied,
            volume_proxy,
        };

        steps.push(step_result);
        prev_error_bound = error_bound;
        current_tube = new_tube;
    }

    let final_error_bound = steps.last().map(|s| s.error_bound).unwrap_or(0.0);
    let contraction_ratio = if initial_error_bound > 1e-8 {
        final_error_bound / initial_error_bound
    } else {
        0.0
    };

    Ok(TubeMPCResult {
        steps,
        final_tube: current_tube,
        final_error_bound,
        initial_error_bound,
        contraction_ratio,
        all_contracted,
        num_steps: horizon,
        config: config.clone(),
    })
}

/// Compute the closed-loop Koopman operator: `A_{cl} = K + B·K_gain`.
///
/// # Arguments
/// - `k_operator`: Koopman operator K, shape `[dim, dim]`.
/// - `b_matrix`: Control input matrix B, shape `[dim, u_dim]`.
/// - `k_gain`: Feedback gain K_gain, shape `[u_dim, dim]`.
///
/// # Returns
/// `A_{cl}` with shape `[dim, dim]`.
pub fn compute_closed_loop_koopman(
    k_operator: &Tensor,
    b_matrix: &Tensor,
    k_gain: &Tensor,
) -> Result<Tensor> {
    // A_cl = K + B @ K_gain
    let bk = b_matrix.matmul(k_gain)?;
    k_operator.broadcast_add(&bk)
}

/// Compute PINN-bound residual zonotope from neural network residual bounds.
///
/// Given a residual bound `r_max` per dimension, creates a zonotope with
/// diagonal generators representing the worst-case residual.
///
/// # Arguments
/// - `r_max`: Maximum residual per dimension, shape `[1, dim]` or `[dim]`.
/// - `device`: Device for tensor creation.
///
/// # Returns
/// Residual generators tensor with shape `[dim, dim]` (diagonal).
pub fn compute_pinn_residual_zonotope(
    r_max: &Tensor,
    device: &candle_core::Device,
) -> Result<Tensor> {
    // Extract to vec first (Candle tensors don't support direct indexing)
    let r_max_vec = r_max.flatten_all()?.to_vec1::<f32>()?;
    let dim = r_max_vec.len();

    // Create diagonal generator matrix
    let mut gen_data = vec![0.0f32; dim * dim];
    for i in 0..dim {
        gen_data[i * dim + i] = r_max_vec[i];
    }
    Tensor::from_vec(gen_data, (dim, dim), device)
}

/// Compute PINN (Physics-Informed Neural Network) residual loss.
///
/// Combines data fidelity (MSE) with physics consistency (PDE residual):
/// ```text
/// L = ||r_obs - r_pred||² + λ * ||dr/dt - N[r, ψ]||²
/// ```
///
/// # Arguments
/// - `r_obs`: Observed residual tensor.
/// - `r_pred`: Predicted residual from neural network.
/// - `dr_dt`: Temporal derivative of residual.
/// - `n_physics`: Known physical dynamics (e.g., manifold flow).
/// - `lambda`: Weight for physics loss term.
///
/// # Returns
/// Total loss: `data_loss + lambda * physics_loss`.
pub fn compute_pinn_residual_loss(
    r_obs: &Tensor,
    r_pred: &Tensor,
    dr_dt: &Tensor,
    n_physics: &Tensor,
    lambda: f32,
) -> Result<f32> {
    // 1. Data Loss (MSE)
    let data_diff = r_obs.broadcast_sub(r_pred)?;
    let data_loss = data_diff.sqr()?.mean_all()?.to_scalar::<f32>()?;

    // 2. Physics Loss (PDE Residual)
    let physics_diff = dr_dt.broadcast_sub(n_physics)?;
    let physics_loss = physics_diff.sqr()?.mean_all()?.to_scalar::<f32>()?;

    // 3. Total Loss
    Ok(data_loss + lambda * physics_loss)
}

/// Verify contractive constraint for a sequence of error bounds.
///
/// Checks that `e_{k+1} <= ρ·e_k + γ` for all k.
///
/// # Arguments
/// - `error_bounds`: Sequence of error bounds `[e_0, e_1, ..., e_n]`.
/// - `rho`: Contraction factor (must be < 1).
/// - `gamma`: Additive error bound.
/// - `tolerance`: Numerical tolerance.
///
/// # Returns
/// `(all_satisfied, violations)` where violations is a list of step indices
/// where the constraint was violated.
pub fn verify_contractive_sequence(
    error_bounds: &[f32],
    rho: f32,
    gamma: f32,
    tolerance: f32,
) -> (bool, Vec<usize>) {
    let mut violations = Vec::new();

    for k in 1..error_bounds.len() {
        let allowed = rho * error_bounds[k - 1] + gamma + tolerance;
        if error_bounds[k] > allowed {
            violations.push(k);
        }
    }

    (violations.is_empty(), violations)
}

/// Compute the tube radius sequence for radius-based Tube MPC.
///
/// Uses the recursive formula: `r_{k+1} = ||A_{cl}|| * r_k + ε_res + α·ε_dist`
/// where ε are the disturbance bounds.
///
/// # Arguments
/// - `a_cl_norm`: Infinity norm of A_{cl}.
/// - `initial_radius`: Initial tube radius r_0.
/// - `residual_bound`: Residual bound ε_res.
/// - `disturbance_bound`: Disturbance bound ε_dist.
/// - `alpha`: Disturbance scaling factor.
/// - `horizon`: Number of steps.
///
/// # Returns
/// Vector of radii `[r_0, r_1, ..., r_horizon]`.
pub fn compute_tube_radius_sequence(
    a_cl_norm: f32,
    initial_radius: f32,
    residual_bound: f32,
    disturbance_bound: f32,
    alpha: f32,
    horizon: usize,
) -> Vec<f32> {
    let mut radii = vec![initial_radius];
    let mut r = initial_radius;

    for _ in 0..horizon {
        r = a_cl_norm * r + residual_bound + alpha * disturbance_bound;
        radii.push(r);
    }

    radii
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::Device;

    fn make_tensor(rows: usize, cols: usize, seed: f32, device: &Device) -> Result<Tensor> {
        let mut data = vec![0.0f32; rows * cols];
        for (i, val) in data.iter_mut().enumerate() {
            *val = seed * (i as f32 + 1.0);
        }
        Tensor::from_vec(data, (rows, cols), device)
    }

    #[test]
    fn test_tube_mpc_config_default() {
        let config = TubeMPCConfig::default();
        assert!(config.rho < 1.0);
        assert!(config.gamma >= 0.0);
        assert!(config.alpha >= 0.0);
        assert_eq!(config.horizon, 10);
        assert!(config.verify_contraction);
    }

    #[test]
    fn test_tube_mpc_config_edge_fast() {
        let config = TubeMPCConfig::edge_fast();
        assert!(config.rho < 1.0);
        assert_eq!(config.horizon, 5);
        assert_eq!(config.max_gens, 32);
    }

    #[test]
    fn test_tube_mpc_config_high_precision() {
        let config = TubeMPCConfig::high_precision();
        assert!(config.rho < 1.0);
        assert_eq!(config.horizon, 20);
        assert_eq!(config.max_gens, 128);
    }

    #[test]
    fn test_tube_mpc_config_builder() {
        let config = TubeMPCConfig::default()
            .with_rho(0.85)
            .with_gamma(0.005)
            .with_alpha(0.3)
            .with_horizon(15);
        assert!((config.rho - 0.85).abs() < 1e-5);
        assert!((config.gamma - 0.005).abs() < 1e-5);
        assert!((config.alpha - 0.3).abs() < 1e-5);
        assert_eq!(config.horizon, 15);
    }

    #[test]
    fn test_tube_mpc_config_rho_clamped() {
        let config = TubeMPCConfig::default().with_rho(1.5);
        assert!(config.rho <= 0.99);
    }

    #[test]
    fn test_compute_closed_loop_koopman() -> Result<()> {
        let device = Device::Cpu;
        // K = I (identity)
        let k = Tensor::eye(3, DType::F32, &device)?;
        // B = I
        let b = Tensor::eye(3, DType::F32, &device)?;
        // K_gain = 0.1 * I
        let k_gain = Tensor::eye(3, DType::F32, &device)?.broadcast_mul(&Tensor::full(
            0.1f32,
            (),
            &device,
        )?)?;

        let a_cl = compute_closed_loop_koopman(&k, &b, &k_gain)?;

        // A_cl = I + 0.1*I = 1.1*I
        let expected = Tensor::eye(3, DType::F32, &device)?.broadcast_mul(&Tensor::full(
            1.1f32,
            (),
            &device,
        )?)?;

        let diff = a_cl
            .broadcast_sub(&expected)?
            .abs()?
            .sum_all()?
            .to_scalar::<f32>()?;
        assert!(diff < 1e-5);
        Ok(())
    }

    #[test]
    fn test_compute_pinn_residual_zonotope() -> Result<()> {
        let device = Device::Cpu;
        let r_max = Tensor::new(&[0.1f32, 0.2, 0.3], &device)?.unsqueeze(0)?;
        let gens = compute_pinn_residual_zonotope(&r_max, &device)?;

        assert_eq!(gens.shape().dims(), &[3, 3]);

        // Check diagonal values
        let gens_vec: Vec<f32> = gens.flatten_all()?.to_vec1()?;
        assert!((gens_vec[0] - 0.1).abs() < 1e-5); // [0,0]
        assert!((gens_vec[4] - 0.2).abs() < 1e-5); // [1,1]
        assert!((gens_vec[8] - 0.3).abs() < 1e-5); // [2,2]
                                                   // Off-diagonal should be 0
        assert!(gens_vec[1].abs() < 1e-8);
        Ok(())
    }

    #[test]
    fn test_compute_pinn_residual_loss_zero() -> Result<()> {
        let device = Device::Cpu;
        let r_obs = Tensor::new(&[1.0f32, 2.0, 3.0], &device)?;
        let r_pred = r_obs.clone();
        let dr_dt = Tensor::new(&[0.1f32, 0.2, 0.3], &device)?;
        let n_physics = dr_dt.clone();

        // Perfect prediction: both losses should be 0
        let loss = compute_pinn_residual_loss(&r_obs, &r_pred, &dr_dt, &n_physics, 1.0)?;
        assert!(loss.abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn test_compute_pinn_residual_loss_data_only() -> Result<()> {
        let device = Device::Cpu;
        let r_obs = Tensor::new(&[1.0f32, 2.0], &device)?;
        let r_pred = Tensor::new(&[0.0f32, 0.0], &device)?;
        let dr_dt = Tensor::new(&[0.0f32, 0.0], &device)?;
        let n_physics = dr_dt.clone();

        // Data loss = MSE([1,2], [0,0]) = (1+4)/2 = 2.5
        // Physics loss = 0
        let loss = compute_pinn_residual_loss(&r_obs, &r_pred, &dr_dt, &n_physics, 1.0)?;
        assert!((loss - 2.5).abs() < 1e-5);
        Ok(())
    }

    #[test]
    fn test_compute_pinn_residual_loss_physics_only() -> Result<()> {
        let device = Device::Cpu;
        let r_obs = Tensor::new(&[1.0f32, 2.0], &device)?;
        let r_pred = r_obs.clone();
        let dr_dt = Tensor::new(&[1.0f32, 1.0], &device)?;
        let n_physics = Tensor::new(&[0.0f32, 0.0], &device)?;

        // Data loss = 0
        // Physics loss = MSE([1,1], [0,0]) = (1+1)/2 = 1.0
        // Total = 0 + 2.0 * 1.0 = 2.0
        let loss = compute_pinn_residual_loss(&r_obs, &r_pred, &dr_dt, &n_physics, 2.0)?;
        assert!((loss - 2.0).abs() < 1e-5);
        Ok(())
    }

    #[test]
    fn test_compute_pinn_residual_loss_lambda_weight() -> Result<()> {
        let device = Device::Cpu;
        let r_obs = Tensor::new(&[2.0f32], &device)?;
        let r_pred = Tensor::new(&[0.0f32], &device)?;
        let dr_dt = Tensor::new(&[1.0f32], &device)?;
        let n_physics = Tensor::new(&[0.0f32], &device)?;

        // Data loss = 4.0, Physics loss = 1.0
        // With lambda=0: loss = 4.0
        let loss_zero = compute_pinn_residual_loss(&r_obs, &r_pred, &dr_dt, &n_physics, 0.0)?;
        assert!((loss_zero - 4.0).abs() < 1e-5);

        // With lambda=1: loss = 4.0 + 1.0 = 5.0
        let loss_one = compute_pinn_residual_loss(&r_obs, &r_pred, &dr_dt, &n_physics, 1.0)?;
        assert!((loss_one - 5.0).abs() < 1e-5);

        // With lambda=3: loss = 4.0 + 3*1.0 = 7.0
        let loss_three = compute_pinn_residual_loss(&r_obs, &r_pred, &dr_dt, &n_physics, 3.0)?;
        assert!((loss_three - 7.0).abs() < 1e-5);
        Ok(())
    }

    #[test]
    fn test_verify_contractive_sequence_satisfied() {
        // e_k = 0.9^k (exponential decay)
        let bounds: Vec<f32> = (0..10).map(|k| 0.9f32.powi(k as i32)).collect();
        let (satisfied, violations) = verify_contractive_sequence(&bounds, 0.92, 0.01, 1e-4);
        assert!(satisfied);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_verify_contractive_sequence_violated() {
        // e_k = 1.1^k (exponential growth)
        let bounds: Vec<f32> = (0..10).map(|k| 1.1f32.powi(k as i32)).collect();
        let (satisfied, violations) = verify_contractive_sequence(&bounds, 0.92, 0.01, 1e-4);
        assert!(!satisfied);
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_compute_tube_radius_sequence() {
        let radii = compute_tube_radius_sequence(0.9, 1.0, 0.01, 0.02, 0.5, 5);
        assert_eq!(radii.len(), 6); // r_0 through r_5
        assert!((radii[0] - 1.0).abs() < 1e-5);
        // r_1 = 0.9*1.0 + 0.01 + 0.5*0.02 = 0.92
        assert!((radii[1] - 0.92).abs() < 1e-5);
    }

    #[test]
    fn test_compute_tube_radius_stable() {
        // Stable: ||A_cl|| < 1
        let radii = compute_tube_radius_sequence(0.9, 1.0, 0.01, 0.01, 0.5, 20);
        // Should converge to steady state
        assert!(radii[radii.len() - 1] < radii[radii.len() - 2] + 0.02);
    }

    #[test]
    fn test_compute_tube_radius_unstable() {
        // Unstable: ||A_cl|| > 1
        let radii = compute_tube_radius_sequence(1.1, 1.0, 0.01, 0.01, 0.5, 10);
        // Should grow
        assert!(radii[radii.len() - 1] > radii[0]);
    }

    #[test]
    fn test_propagate_tube_recursive_basic() -> Result<()> {
        let device = Device::Cpu;
        let dim = 3;

        // A_cl = 0.5 * I (contractive)
        let a_cl = Tensor::eye(dim, DType::F32, &device)?.broadcast_mul(&Tensor::full(
            0.5f32,
            (),
            &device,
        )?)?;

        // x_nominal = [1, 2, 3]
        let x_nominal = Tensor::new(&[1.0f32, 2.0, 3.0], &device)?.unsqueeze(0)?;

        // Small residual generators
        let residual_gens = Tensor::zeros((dim, dim), DType::F32, &device)?;

        // Small disturbance generators
        let dist_gens = Tensor::zeros((dim, dim), DType::F32, &device)?;

        let config = TubeMPCConfig {
            horizon: 5,
            rho: 0.92,
            gamma: 0.01,
            alpha: 0.5,
            max_gens: 32,
            verify_contraction: true,
            contraction_tolerance: 1e-4,
        };

        let result =
            propagate_tube_recursive(&a_cl, &x_nominal, &residual_gens, &dist_gens, &config)?;

        assert_eq!(result.num_steps, 5);
        assert_eq!(result.steps.len(), 5);
        // With contractive A_cl and zero disturbances, error should decrease
        assert!(result.final_error_bound < result.initial_error_bound + 0.1);
        Ok(())
    }

    #[test]
    fn test_propagate_tube_recursive_display() -> Result<()> {
        let device = Device::Cpu;
        let dim = 2;

        let a_cl = Tensor::eye(dim, DType::F32, &device)?.broadcast_mul(&Tensor::full(
            0.8f32,
            (),
            &device,
        )?)?;
        let x_nominal = Tensor::ones((1, dim), DType::F32, &device)?;
        let residual_gens = Tensor::zeros((dim, dim), DType::F32, &device)?;
        let dist_gens = Tensor::zeros((dim, dim), DType::F32, &device)?;

        let config = TubeMPCConfig {
            horizon: 3,
            ..Default::default()
        };

        let result =
            propagate_tube_recursive(&a_cl, &x_nominal, &residual_gens, &dist_gens, &config)?;

        let display = format!("{}", result);
        assert!(display.contains("TubeMPC"));
        assert!(display.contains("steps="));
        Ok(())
    }

    #[test]
    fn test_propagate_tube_recursive_contracts() -> Result<()> {
        let device = Device::Cpu;
        let dim = 4;

        // Strongly contractive: A_cl = 0.5 * I
        let a_cl = Tensor::eye(dim, DType::F32, &device)?.broadcast_mul(&Tensor::full(
            0.5f32,
            (),
            &device,
        )?)?;

        // Start with non-zero state
        let x_nominal = Tensor::new(&[1.0f32, 2.0, 3.0, 4.0], &device)?.unsqueeze(0)?;

        // Zero disturbances for clean contraction test
        let residual_gens = Tensor::zeros((dim, dim), DType::F32, &device)?;
        let dist_gens = Tensor::zeros((dim, dim), DType::F32, &device)?;

        let config = TubeMPCConfig {
            horizon: 10,
            rho: 0.92,
            gamma: 0.001,
            alpha: 0.0,
            max_gens: 32,
            verify_contraction: true,
            contraction_tolerance: 1e-4,
        };

        let result =
            propagate_tube_recursive(&a_cl, &x_nominal, &residual_gens, &dist_gens, &config)?;

        // With A_cl = 0.5*I and zero disturbances, error should halve each step
        // e_final < e0 * 0.5^10 ≈ e0 * 0.001
        assert!(result.contraction_ratio < 0.1);
        assert!(result.all_contracted);
        Ok(())
    }

    #[test]
    fn test_propagate_tube_recursive_with_disturbance() -> Result<()> {
        let device = Device::Cpu;
        let dim = 3;

        // Contractive A_cl
        let a_cl = Tensor::eye(dim, DType::F32, &device)?.broadcast_mul(&Tensor::full(
            0.7f32,
            (),
            &device,
        )?)?;

        let x_nominal = Tensor::ones((1, dim), DType::F32, &device)?;

        // Non-zero residual generators (small noise)
        let residual_gens = make_tensor(dim, dim, 0.01, &device)?;

        // Non-zero disturbance generators
        let dist_gens = make_tensor(dim, dim, 0.02, &device)?;

        let config = TubeMPCConfig {
            horizon: 5,
            rho: 0.95,
            gamma: 0.05,
            alpha: 0.5,
            max_gens: 32,
            verify_contraction: true,
            contraction_tolerance: 1e-3,
        };

        let result =
            propagate_tube_recursive(&a_cl, &x_nominal, &residual_gens, &dist_gens, &config)?;

        assert_eq!(result.num_steps, 5);
        // With disturbances, tube should grow but remain bounded
        assert!(result.final_error_bound > 0.0);
        Ok(())
    }

    #[test]
    fn test_tube_step_result_display() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::new(&[1.0f32, 2.0], &device)?.unsqueeze(0)?;
        let tube = Zonotope::point(&center)?;

        let step = TubeStepResult {
            step: 0,
            tube,
            error_bound: 3.0,
            prev_error_bound: 0.0,
            contraction_satisfied: true,
            volume_proxy: 0.0,
        };

        let display = format!("{}", step);
        assert!(display.contains("TubeStep"));
        assert!(display.contains("k=0"));
        Ok(())
    }
}
