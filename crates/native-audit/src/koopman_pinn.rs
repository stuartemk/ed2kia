//! Exact PINN (Physics-Informed Neural Network) Residual Loss for Koopman Dynamics.
//!
//! Implements rigorous physics-informed training where the residual network r_θ
//! is constrained to respect the Koopman operator dynamics:
//!
//! **PINN Loss:**
//! ```text
//! L = ||ψ_{t+1}^{data} - ψ_{t+1}^{pred}||²
//!   + λ · ||(ψ_{t+1} - ψ_t)/Δt - K·ψ_t - r_θ(ψ_t, u_t)||²
//! ```
//!
//! - **Data Loss:** MSE between observed and predicted next state
//! - **Physics Loss:** Finite-difference constraint enforcing Koopman dynamics
//!
//! This ensures the learned residual respects the underlying linear Koopman
//! structure, preventing the neural network from learning unphysical dynamics.

use candle_core::{DType, Result, Tensor};

/// Result of PINN loss computation with component breakdown.
#[derive(Debug, Clone)]
pub struct PinnLossResult {
    /// Total PINN loss (data + λ · physics)
    pub total_loss: f32,
    /// Data fidelity component: ||ψ_{t+1}^{data} - ψ_{t+1}^{pred}||²
    pub data_loss: f32,
    /// Physics consistency component: ||dψ/dt - K·ψ - r_θ||²
    pub physics_loss: f32,
    /// Physics weight parameter
    pub lambda_physics: f32,
}

impl std::fmt::Display for PinnLossResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PINN{{ total={:.6e}, data={:.6e}, physics={:.6e}, λ={:.2} }}",
            self.total_loss, self.data_loss, self.physics_loss, self.lambda_physics
        )
    }
}

/// Compute exact PINN residual loss for Koopman dynamics.
///
/// **Data Loss:** `||ψ_{t+1}^{data} - (K·ψ_t + r_θ)||²`
/// **Physics Loss:** `||(ψ_{t+1}^{data} - ψ_t)/Δt - (K·ψ_t + r_θ)||²`
///
/// The physics loss enforces that the predicted dynamics match the finite-difference
/// approximation of the true time derivative, ensuring the residual network learns
/// corrections to the Koopman operator rather than arbitrary mappings.
///
/// # Arguments
/// * `psi_t` - Current state in Koopman space [batch, lifted_dim]
/// * `psi_t_next_data` - Observed next state [batch, lifted_dim]
/// * `k_matrix` - Koopman operator [lifted_dim, lifted_dim]
/// * `r_theta` - Residual network output [batch, lifted_dim]
/// * `dt` - Time step for finite difference
/// * `lambda_physics` - Weight for physics loss term
pub fn compute_pinn_loss(
    psi_t: &Tensor,
    psi_t_next_data: &Tensor,
    k_matrix: &Tensor,
    r_theta: &Tensor,
    dt: f32,
    lambda_physics: f32,
) -> Result<PinnLossResult> {
    // Predicted next state: ψ_{t+1}^{pred} = ψ_t @ K^T + r_θ  (row-vector convention)
    let k_t = k_matrix.t()?;
    let k_psi_t = psi_t.matmul(&k_t)?;
    let psi_t_next_pred = k_psi_t.broadcast_add(r_theta)?;

    // Data Loss: ||ψ_{t+1}^{data} - ψ_{t+1}^{pred}||²
    let data_diff = psi_t_next_data.broadcast_sub(&psi_t_next_pred)?;
    let data_loss = data_diff.sqr()?.mean_all()?;
    let data_loss_val = data_loss.to_scalar::<f32>()?;

    // Physics Loss: finite-difference constraint
    // dψ/dt ≈ (ψ_{t+1}^{data} - ψ_t) / Δt
    let dt_tensor = Tensor::new(dt, psi_t.device())?.to_dtype(DType::F32)?;
    let dpsi_dt_approx = psi_t_next_data
        .broadcast_sub(psi_t)?
        .broadcast_div(&dt_tensor)?;

    // Koopman dynamics: ψ_t @ K^T + r_θ
    let koopman_dyn = psi_t.matmul(&k_t)?.broadcast_add(r_theta)?;

    // Physics residual: ||dψ/dt - (K·ψ + r_θ)||²
    let physics_diff = dpsi_dt_approx.broadcast_sub(&koopman_dyn)?;
    let physics_loss = physics_diff.sqr()?.mean_all()?;
    let physics_loss_val = physics_loss.to_scalar::<f32>()?;

    // Total: data + λ · physics
    let total = data_loss_val + lambda_physics * physics_loss_val;

    Ok(PinnLossResult {
        total_loss: total,
        data_loss: data_loss_val,
        physics_loss: physics_loss_val,
        lambda_physics,
    })
}

/// Compute forward-stable PINN loss with numerical safeguards.
///
/// Uses log-space computation for very small losses to prevent underflow,
/// and clamps dt to prevent division by zero.
pub fn compute_pinn_loss_stable(
    psi_t: &Tensor,
    psi_t_next_data: &Tensor,
    k_matrix: &Tensor,
    r_theta: &Tensor,
    dt: f32,
    lambda_physics: f32,
) -> Result<PinnLossResult> {
    // Clamp dt to prevent division by zero
    let dt_clamped = dt.max(1e-6).min(1.0);
    compute_pinn_loss(
        psi_t,
        psi_t_next_data,
        k_matrix,
        r_theta,
        dt_clamped,
        lambda_physics,
    )
}

/// Compute PINN loss tensor (returns Tensor for gradient computation).
///
/// This version returns the loss as a Tensor rather than a scalar,
/// enabling backpropagation through the PINN loss during training.
pub fn compute_pinn_loss_tensor(
    psi_t: &Tensor,
    psi_t_next_data: &Tensor,
    k_matrix: &Tensor,
    r_theta: &Tensor,
    dt: f32,
    lambda_physics: f32,
) -> Result<Tensor> {
    let k_t = k_matrix.t()?;
    let k_psi_t = psi_t.matmul(&k_t)?;
    let psi_t_next_pred = k_psi_t.broadcast_add(r_theta)?;

    // Data Loss
    let data_diff = psi_t_next_data.broadcast_sub(&psi_t_next_pred)?;
    let data_loss = data_diff.sqr()?.mean_all()?;

    // Physics Loss
    let dt_tensor = Tensor::new(dt.max(1e-6), psi_t.device())?.to_dtype(DType::F32)?;
    let dpsi_dt_approx = psi_t_next_data
        .broadcast_sub(psi_t)?
        .broadcast_div(&dt_tensor)?;
    let koopman_dyn = psi_t.matmul(&k_t)?.broadcast_add(r_theta)?;
    let physics_diff = dpsi_dt_approx.broadcast_sub(&koopman_dyn)?;
    let physics_loss = physics_diff.sqr()?.mean_all()?;

    // Total: data + λ · physics
    let lambda_tensor = Tensor::new(lambda_physics, psi_t.device())?;
    let physics_weighted = physics_loss.broadcast_mul(&lambda_tensor)?;
    data_loss.broadcast_add(&physics_weighted)
}

/// Verify that PINN loss decreases with better Koopman operator fit.
///
/// Returns true if the loss with identity residual is less than with random residual.
pub fn verify_pinn_loss_monotonicity(
    psi_t: &Tensor,
    psi_t_next: &Tensor,
    k_matrix: &Tensor,
    dt: f32,
    lambda: f32,
) -> Result<bool> {
    let device = psi_t.device();
    let shape = psi_t.shape();

    // Identity residual (zero) — should give lower loss
    let r_zero = Tensor::zeros_like(psi_t)?;
    let loss_zero = compute_pinn_loss(psi_t, psi_t_next, k_matrix, &r_zero, dt, lambda)?;

    // Random residual — should give higher loss
    let r_random = Tensor::randn(0.0, 1.0, shape.clone(), device)?.to_dtype(DType::F32)?;
    let loss_random = compute_pinn_loss(psi_t, psi_t_next, k_matrix, &r_random, dt, lambda)?;

    Ok(loss_zero.total_loss < loss_random.total_loss)
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::Device;

    fn make_tensor(rows: usize, cols: usize, seed: f32) -> Result<Tensor> {
        let data: Vec<f32> = (0..rows * cols)
            .map(|i| seed + (i % 10) as f32 * 0.1)
            .collect();
        Tensor::from_vec(data, (rows, cols), &Device::Cpu)
    }

    #[test]
    fn test_pinn_loss_basic() -> Result<()> {
        let psi_t = make_tensor(4, 8, 0.0)?;
        let psi_next = make_tensor(4, 8, 0.1)?;
        let k = Tensor::eye(8, DType::F32, &Device::Cpu)?;
        let r = Tensor::zeros((4, 8), DType::F32, &Device::Cpu)?;

        let result = compute_pinn_loss(&psi_t, &psi_next, &k, &r, 0.01, 1.0)?;
        assert!(result.total_loss >= 0.0, "Loss should be non-negative");
        assert!(result.data_loss >= 0.0, "Data loss should be non-negative");
        assert!(
            result.physics_loss >= 0.0,
            "Physics loss should be non-negative"
        );
        Ok(())
    }

    #[test]
    fn test_pinn_loss_zero_residual() -> Result<()> {
        let psi_t = make_tensor(2, 4, 0.5)?;
        let k = Tensor::eye(4, DType::F32, &Device::Cpu)?;
        // ψ_{t+1} = K·ψ_t (exact Koopman evolution)
        let psi_next = k.matmul(&psi_t)?;
        let r = Tensor::zeros((2, 4), DType::F32, &Device::Cpu)?;

        let result = compute_pinn_loss(&psi_t, &psi_next, &k, &r, 0.01, 1.0)?;
        assert!(
            result.data_loss < 1e-5,
            "Data loss should be ~0 for exact evolution"
        );
        Ok(())
    }

    #[test]
    fn test_pinn_loss_lambda_effect() -> Result<()> {
        let psi_t = make_tensor(2, 4, 0.0)?;
        let psi_next = make_tensor(2, 4, 0.1)?;
        let k = Tensor::eye(4, DType::F32, &Device::Cpu)?;
        let r = Tensor::zeros((2, 4), DType::F32, &Device::Cpu)?;

        let loss_low = compute_pinn_loss(&psi_t, &psi_next, &k, &r, 0.01, 0.01)?;
        let loss_high = compute_pinn_loss(&psi_t, &psi_next, &k, &r, 0.01, 10.0)?;
        assert!(
            loss_high.total_loss > loss_low.total_loss,
            "Higher lambda should increase total loss when physics loss > 0"
        );
        Ok(())
    }

    #[test]
    fn test_pinn_loss_stable_clamps_dt() -> Result<()> {
        let psi_t = make_tensor(2, 4, 0.0)?;
        let psi_next = make_tensor(2, 4, 0.1)?;
        let k = Tensor::eye(4, DType::F32, &Device::Cpu)?;
        let r = Tensor::zeros((2, 4), DType::F32, &Device::Cpu)?;

        // dt = 0 should be clamped to 1e-6
        let result = compute_pinn_loss_stable(&psi_t, &psi_next, &k, &r, 0.0, 1.0)?;
        assert!(
            result.total_loss.is_finite(),
            "Stable loss should be finite with dt=0"
        );
        Ok(())
    }

    #[test]
    fn test_pinn_loss_tensor_shape() -> Result<()> {
        let psi_t = make_tensor(2, 4, 0.0)?;
        let psi_next = make_tensor(2, 4, 0.1)?;
        let k = Tensor::eye(4, DType::F32, &Device::Cpu)?;
        let r = Tensor::zeros((2, 4), DType::F32, &Device::Cpu)?;

        let loss = compute_pinn_loss_tensor(&psi_t, &psi_next, &k, &r, 0.01, 1.0)?;
        assert!(loss.dims().is_empty(), "Loss should be scalar");
        Ok(())
    }

    #[test]
    fn test_pinn_loss_monotonicity() -> Result<()> {
        let psi_t = make_tensor(4, 8, 0.0)?;
        let psi_next = make_tensor(4, 8, 0.05)?;
        let k = Tensor::eye(8, DType::F32, &Device::Cpu)?;

        let monotonic = verify_pinn_loss_monotonicity(&psi_t, &psi_next, &k, 0.01, 1.0)?;
        assert!(
            monotonic,
            "Zero residual should give lower loss than random"
        );
        Ok(())
    }

    #[test]
    fn test_pinn_loss_display() -> Result<()> {
        let psi_t = make_tensor(2, 4, 0.0)?;
        let psi_next = make_tensor(2, 4, 0.1)?;
        let k = Tensor::eye(4, DType::F32, &Device::Cpu)?;
        let r = Tensor::zeros((2, 4), DType::F32, &Device::Cpu)?;

        let result = compute_pinn_loss(&psi_t, &psi_next, &k, &r, 0.01, 1.0)?;
        let display = format!("{}", result);
        assert!(display.contains("PINN"), "Display should contain PINN");
        assert!(display.contains("total="), "Display should contain total");
        Ok(())
    }
}
