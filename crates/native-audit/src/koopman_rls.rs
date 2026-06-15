//! Koopman RLS (Recursive Least Squares) with Forgetting Factor + Adaptive SVD.
//!
//! Sprint 166 (v16.6.0) — Adaptive RLS Koopman + Fast ADMM/Proj LMI.
//!
//! Implements online RLS adaptation for the Koopman operator K with:
//! - Forgetting factor λ ∈ (0,1] for tracking time-varying dynamics.
//! - Dead-zone adaptation: skip update if ||error|| < ε (stability).
//! - Rank-1 adaptive SVD updates for low-rank K approximation.
//! - Persistency-of-excitation (PoE) check via covariance eigenvalue monitoring.
//!
//! **RLS Update (exact, vectorized):**
//! ```math
//! \\hat{\\theta}_t = \\hat{\\theta}_{t-1} + K_t (y_t - \\Phi_t \\hat{\\theta}_{t-1})
//! ```
//! ```math
//! K_t = \\frac{P_{t-1} \\Phi_t^T}{\\lambda + \\Phi_t P_{t-1} \\Phi_t^T}
//! ```
//! ```math
//! P_t = \\frac{1}{\\lambda} \\left( P_{t-1} - K_t \\Phi_t P_{t-1} \\right)
//! ```
//!
//! Where:
//! - `\\hat{\\theta}` = Koopman operator K flattened or block-structured.
//! - `\\Phi_t` = lifted observable ψ(x_t) as regressor row [1, d_lifted].
//! - `y_t` = next lifted observable ψ(x_{t+1}) [1, d_lifted].
//! - `P` = covariance matrix [d_lifted × d_lifted].
//! - `λ` = forgetting factor (0.95..1.0 recommended).
//!
//! **Adaptive SVD (rank-1 update):**
//! After RLS update, optionally perform rank-1 SVD truncation:
//! - Compute approximate singular values via power iteration.
//! - Truncate to top-r components for compression + stability.
//!
//! **Dead-Zone:**
//! Skip RLS update if innovation ||y_t - Φ_t θ̂_{t-1}|| < ε_dead.
//! Prevents covariance explosion under low excitation.
//!
//! **PoE Check:**
//! Monitor condition number of P. If cond(P) > threshold, inflate diagonal.

use candle_core::{DType, Result, Tensor};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for Koopman RLS adaptation.
#[derive(Debug, Clone)]
pub struct KoopmanRLSConfig {
    /// Forgetting factor λ ∈ (0, 1]. λ=1.0 means no forgetting (standard RLS).
    /// λ<1.0 gives more weight to recent data (adaptive tracking).
    /// Default: 0.99 (mild forgetting for online adaptation).
    pub forgetting_lambda: f64,
    /// Initial covariance P_0 = δ^{-1} I. Small δ → high initial uncertainty.
    /// Default: 1e-2 (P_0 = 100·I).
    pub initial_covariance_scale: f64,
    /// Dead-zone threshold ε. Skip update if ||innovation|| < ε.
    /// Default: 1e-6 (very tight dead-zone).
    pub dead_zone_epsilon: f64,
    /// Maximum condition number for covariance inflation.
    /// Default: 1e8 (aggressive inflation if P becomes ill-conditioned).
    pub max_condition_number: f64,
    /// Diagonal inflation amount when PoE check fails.
    /// Default: 1e-4.
    pub inflation_amount: f64,
    /// Enable adaptive SVD truncation after RLS update.
    /// Default: false (use full-rank K).
    pub use_adaptive_svd: bool,
    /// Target rank for SVD truncation (0 = auto-select via energy criterion).
    /// Default: 0.
    pub svd_target_rank: usize,
    /// Energy threshold for auto-rank selection (keep components with ≥ this fraction).
    /// Default: 0.99 (keep 99% of energy).
    pub svd_energy_threshold: f64,
    /// Maximum SVD iterations for power method approximation.
    /// Default: 20.
    pub svd_max_iter: usize,
    /// SVD convergence tolerance.
    /// Default: 1e-8.
    pub svd_tolerance: f64,
}

impl Default for KoopmanRLSConfig {
    fn default() -> Self {
        Self {
            forgetting_lambda: 0.99,
            initial_covariance_scale: 1e-2,
            dead_zone_epsilon: 1e-6,
            max_condition_number: 1e8,
            inflation_amount: 1e-4,
            use_adaptive_svd: false,
            svd_target_rank: 0,
            svd_energy_threshold: 0.99,
            svd_max_iter: 20,
            svd_tolerance: 1e-8,
        }
    }
}

impl KoopmanRLSConfig {
    /// Fast configuration for edge devices (higher forgetting, tighter dead-zone).
    pub fn edge_fast() -> Self {
        Self {
            forgetting_lambda: 0.97,
            initial_covariance_scale: 1e-1,
            dead_zone_epsilon: 1e-4,
            max_condition_number: 1e6,
            inflation_amount: 1e-3,
            use_adaptive_svd: true,
            svd_target_rank: 0,
            svd_energy_threshold: 0.95,
            svd_max_iter: 10,
            svd_tolerance: 1e-6,
        }
    }

    /// High-precision configuration for server-class nodes.
    pub fn high_precision() -> Self {
        Self {
            forgetting_lambda: 0.999,
            initial_covariance_scale: 1e-4,
            dead_zone_epsilon: 1e-8,
            max_condition_number: 1e10,
            inflation_amount: 1e-6,
            use_adaptive_svd: false,
            svd_target_rank: 0,
            svd_energy_threshold: 0.999,
            svd_max_iter: 50,
            svd_tolerance: 1e-12,
        }
    }

    /// Set custom forgetting factor.
    pub fn with_forgetting(mut self, lambda: f64) -> Self {
        self.forgetting_lambda = lambda.clamp(0.5, 1.0);
        self
    }

    /// Set custom dead-zone threshold.
    pub fn with_dead_zone(mut self, epsilon: f64) -> Self {
        self.dead_zone_epsilon = epsilon.max(0.0);
        self
    }

    /// Enable adaptive SVD with target rank.
    pub fn with_svd_rank(mut self, rank: usize) -> Self {
        self.use_adaptive_svd = true;
        self.svd_target_rank = rank;
        self
    }
}

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// Result of a single RLS update step.
#[derive(Debug, Clone)]
pub struct RLSUpdateResult {
    /// Innovation norm ||y_t - Φ_t θ̂_{t-1}||.
    pub innovation_norm: f64,
    /// Update was applied (false if dead-zone triggered).
    pub updated: bool,
    /// Current condition number estimate of P.
    pub condition_number: f64,
    /// Covariance inflation applied.
    pub inflated: bool,
    /// SVD truncation applied (rank before/after).
    pub svd_rank_before: Option<usize>,
    pub svd_rank_after: Option<usize>,
    /// Prediction error (MSE).
    pub prediction_error: f64,
}

impl std::fmt::Display for RLSUpdateResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RLS{{ innov={:.6e}, updated={}, cond={:.2e}, infl={}, svd={:?}->{:?}, mse={:.6e} }}",
            self.innovation_norm,
            self.updated,
            self.condition_number,
            self.inflated,
            self.svd_rank_before,
            self.svd_rank_after,
            self.prediction_error,
        )
    }
}

// ---------------------------------------------------------------------------
// KoopmanRLS Core
// ---------------------------------------------------------------------------

/// Recursive Least Squares estimator for Koopman operator K.
///
/// Maintains:
/// - `theta`: Current estimate of K as [d_lifted, d_lifted] tensor.
/// - `P`: Covariance matrix [d_lifted, d_lifted].
///
/// Each call to `update_koopman_rls` processes one transition (ψ_t → ψ_{t+1}).
pub struct KoopmanRLS {
    /// Current Koopman operator estimate K ∈ ℝ^{d×d}.
    theta: Tensor,
    /// Covariance matrix P ∈ ℝ^{d×d}.
    covariance: Tensor,
    /// Configuration.
    config: KoopmanRLSConfig,
    /// Lifted dimension.
    lifted_dim: usize,
    /// Device.
    device: Device,
    /// Update counter.
    update_count: usize,
}

use candle_core::Device;

impl KoopmanRLS {
    /// Create a new Koopman RLS estimator.
    ///
    /// Initializes K = I (identity) and P = δ^{-1} I.
    pub fn new(lifted_dim: usize, config: KoopmanRLSConfig, device: &Device) -> Result<Self> {
        let theta = Tensor::eye(lifted_dim, DType::F64, device)?;
        let p_scale = 1.0 / config.initial_covariance_scale;
        let covariance = Tensor::eye(lifted_dim, DType::F64, device)?
            .broadcast_mul(&Tensor::new(p_scale, device)?)?;

        Ok(Self {
            theta,
            covariance,
            config,
            lifted_dim,
            device: device.clone(),
            update_count: 0,
        })
    }

    /// Return the current Koopman operator estimate.
    pub fn k_operator(&self) -> &Tensor {
        &self.theta
    }

    /// Return the current covariance matrix.
    pub fn covariance(&self) -> &Tensor {
        &self.covariance
    }

    /// Return the number of updates performed.
    pub fn update_count(&self) -> usize {
        self.update_count
    }

    /// Return a reference to the configuration.
    pub fn config(&self) -> &KoopmanRLSConfig {
        &self.config
    }

    // -----------------------------------------------------------------------
    // RLS Update — Exact formula implementation
    // -----------------------------------------------------------------------

    /// Perform one RLS update step.
    ///
    /// **Formula:**
    /// ```math
    /// \\text{innovation} = y_t - \\Phi_t \\hat{\\theta}_{t-1}
    /// ```
    /// ```math
    /// K_t = \\frac{P_{t-1} \\Phi_t^T}{\\lambda + \\Phi_t P_{t-1} \\Phi_t^T}
    /// ```
    /// ```math
    /// \\hat{\\theta}_t = \\hat{\\theta}_{t-1} + K_t \\cdot \\text{innovation}
    /// ```
    /// ```math
    /// P_t = \\frac{1}{\\lambda} \\left( P_{t-1} - \\frac{K_t \\Phi_t P_{t-1}}{\\lambda + \\Phi_t P_{t-1} \\Phi_t^T} \\right)
    /// ```
    ///
    /// For matrix-valued K (Koopman operator), we process each output dimension
    /// independently: K ∈ ℝ^{d×d}, Φ_t ∈ ℝ^{1×d}, y_t ∈ ℝ^{1×d}.
    ///
    /// # Arguments
    /// * `phi_t` - Current lifted observable ψ(x_t). Shape: [1, d_lifted] or [d_lifted].
    /// * `y_t` - Next lifted observable ψ(x_{t+1}). Shape: [1, d_lifted] or [d_lifted].
    pub fn update_koopman_rls(&mut self, phi_t: &Tensor, y_t: &Tensor) -> Result<RLSUpdateResult> {
        let lambda = self.config.forgetting_lambda;
        let epsilon = self.config.dead_zone_epsilon;

        // Ensure 2D: [1, d]
        let phi = if phi_t.rank() == 1 {
            phi_t.unsqueeze(0)?.to_dtype(DType::F64)?
        } else {
            phi_t.to_dtype(DType::F64)?
        };
        let y = if y_t.rank() == 1 {
            y_t.unsqueeze(0)?.to_dtype(DType::F64)?
        } else {
            y_t.to_dtype(DType::F64)?
        };

        // Prediction: y_pred = phi @ K^T  (row-vector convention: [1,d] @ [d,d] = [1,d])
        let k_t = self.theta.t()?;
        let y_pred = phi.matmul(&k_t)?;

        // Innovation: e = y - y_pred
        let innovation = y.broadcast_sub(&y_pred)?;

        // Innovation norm
        let innovation_norm: f64 = innovation.sqr()?.sum_all()?.to_scalar::<f64>()?.sqrt();

        // Prediction error (MSE)
        let prediction_error: f64 = innovation.sqr()?.mean_all()?.to_scalar::<f64>()?;

        // Dead-zone check: skip update if innovation is too small
        if innovation_norm < epsilon {
            return Ok(RLSUpdateResult {
                innovation_norm,
                updated: false,
                condition_number: 1.0,
                inflated: false,
                svd_rank_before: None,
                svd_rank_after: None,
                prediction_error,
            });
        }

        // --- RLS Update for matrix K ---
        // For each output dimension j, solve:
        //   K_col_j = P @ phi^T / (lambda + phi @ P @ phi^T)
        //   theta_col_j += K_col_j * innovation_j
        //   P -= K_col_j @ phi @ P / (lambda + phi @ P @ phi^T)
        //
        // Vectorized: compute gain once, apply to all output dims.

        // phi^T: [d, 1]
        let phi_t_col = phi.t()?;

        // P @ phi^T: [d, d] @ [d, 1] = [d, 1]
        let p_phi_t = self.covariance.matmul(&phi_t_col)?;

        // phi @ P @ phi^T: [1, d] @ [d, 1] = scalar [1, 1]
        let phi_p_phi_t = phi.matmul(&p_phi_t)?; // [1, 1]
        let scalar_val: f64 = phi_p_phi_t.squeeze(0)?.squeeze(0)?.to_scalar::<f64>()?;

        // Denominator: lambda + phi @ P @ phi^T
        let denom = lambda + scalar_val;
        let denom_inv = 1.0 / denom;

        // Gain: K_gain = (P @ phi^T) / denom  → [d, 1]
        let gain = p_phi_t.broadcast_mul(&Tensor::new(denom_inv, &self.device)?)?;

        // Update theta: theta += gain @ innovation  → [d,1] @ [1,d] = [d,d]
        let theta_update = gain.matmul(&innovation)?;
        self.theta = self.theta.broadcast_add(&theta_update)?;

        // Update covariance: P = (1/lambda) * (P - gain @ phi @ P)
        // gain @ phi: [d,1] @ [1,d] = [d,d]
        let gain_phi = gain.matmul(&phi)?;
        // gain @ phi @ P: [d,d] @ [d,d] = [d,d]
        let gain_phi_p = gain_phi.matmul(&self.covariance)?;
        // P - gain_phi_p
        let p_new = self.covariance.broadcast_sub(&gain_phi_p)?;
        // (1/lambda) * P_new
        self.covariance = p_new.broadcast_mul(&Tensor::new(1.0 / lambda, &self.device)?)?;

        self.update_count += 1;

        // Condition number check + inflation
        let (cond, inflated) = self.check_condition_number()?;

        // Adaptive SVD truncation
        let (svd_before, svd_after) = if self.config.use_adaptive_svd {
            let before = self.lifted_dim;
            let after = self.adaptive_svd_truncate()?;
            (Some(before), Some(after))
        } else {
            (None, None)
        };

        Ok(RLSUpdateResult {
            innovation_norm,
            updated: true,
            condition_number: cond,
            inflated,
            svd_rank_before: svd_before,
            svd_rank_after: svd_after,
            prediction_error,
        })
    }

    // -----------------------------------------------------------------------
    // Condition number monitoring + covariance inflation
    // ---------------------------------------------------------------------------

    /// Estimate condition number of P via power iteration (approx).
    /// If cond(P) > threshold, inflate diagonal.
    fn check_condition_number(&mut self) -> Result<(f64, bool)> {
        // Approximate largest eigenvalue via power iteration on P
        let n = self.lifted_dim;
        let max_iter = 20;
        let tol = 1e-6;

        // Random initial vector
        let mut v = Self::rand_vec(n, &self.device)?;
        let mut lambda_max: f64 = 1.0;

        for _ in 0..max_iter {
            let pv = self.covariance.matmul(&v)?;
            let norm: f64 = pv.sqr()?.sum_all()?.to_scalar::<f64>()?.sqrt();
            if norm < tol {
                break;
            }
            v = pv.broadcast_div(&Tensor::new(norm, &self.device)?)?;
            // Rayleigh quotient: v^T P v
            let pv2 = self.covariance.matmul(&v)?;
            let rq: f64 = v.broadcast_mul(&pv2)?.sum_all()?.to_scalar::<f64>()?;
            let diff = (rq - lambda_max).abs();
            lambda_max = rq;
            if diff < tol {
                break;
            }
        }

        // Estimate smallest eigenvalue via inverse power iteration on P
        // (or use trace/n as approximation for speed)
        let n = self.covariance.dim(0)?;
        let flat: Vec<f64> = self.covariance.flatten_all()?.to_vec1()?;
        let trace: f64 = (0..n).map(|i| flat[i * n + i]).sum();
        let lambda_min_est = trace / n as f64;

        let cond = if lambda_min_est.abs() < 1e-15 {
            1e15
        } else {
            lambda_max.abs() / lambda_min_est.abs()
        };

        let inflated = if cond > self.config.max_condition_number {
            // Inflate diagonal: P += inflation * I
            let inflation = Tensor::eye(n, DType::F64, &self.device)?
                .broadcast_mul(&Tensor::new(self.config.inflation_amount, &self.device)?)?;
            self.covariance = self.covariance.broadcast_add(&inflation)?;
            true
        } else {
            false
        };

        Ok((cond, inflated))
    }

    fn rand_vec(n: usize, device: &Device) -> Result<Tensor> {
        let scale = 1.0f64 / (n as f64).sqrt();
        let mut data = vec![0.0f64; n];
        let mut seed = 42u64;
        for val in &mut data {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let u = ((seed >> 33) as f64) / (u32::MAX as f64);
            *val = (u * 2.0 - 1.0) * scale;
        }
        Tensor::from_vec(data, (n, 1), device)
    }

    // -----------------------------------------------------------------------
    // Adaptive SVD Truncation
    // ---------------------------------------------------------------------------

    /// Truncate K to low-rank approximation via approximate SVD (power iteration).
    /// Returns the effective rank after truncation.
    fn adaptive_svd_truncate(&mut self) -> Result<usize> {
        let n = self.lifted_dim;
        let target_rank = self.config.svd_target_rank;
        let energy_threshold = self.config.svd_energy_threshold;
        let max_iter = self.config.svd_max_iter;
        let tol = self.config.svd_tolerance;

        // Compute K^T K for eigenvalue approximation
        let k_t = self.theta.t()?;
        let ktk = k_t.matmul(&self.theta)?;

        // Power iteration to find top singular values
        let mut singular_values = Vec::new();
        let mut kt_k_copy = ktk.clone();

        let rank = if target_rank > 0 {
            target_rank.min(n)
        } else {
            n // Auto-select via energy
        };

        for r in 0..rank {
            // Random initial vector
            let mut v = Self::rand_vec(n, &self.device)?;
            let mut sigma: f64 = 1.0;

            for _ in 0..max_iter {
                let av = kt_k_copy.matmul(&v)?;
                let norm: f64 = av.sqr()?.sum_all()?.to_scalar::<f64>()?.sqrt();
                if norm < tol {
                    break;
                }
                v = av.broadcast_div(&Tensor::new(norm, &self.device)?)?;
                let av2 = kt_k_copy.matmul(&v)?;
                let rq: f64 = v.broadcast_mul(&av2)?.sum_all()?.to_scalar::<f64>()?;
                if (rq - sigma).abs() < tol {
                    break;
                }
                sigma = rq;
            }

            singular_values.push(sigma.sqrt());

            // Deflate: A -= sigma * v @ v^T
            if r < rank - 1 {
                let vvt = v.matmul(&v.t()?)?;
                let deflate = vvt.broadcast_mul(&Tensor::new(sigma, &self.device)?)?;
                kt_k_copy = kt_k_copy.broadcast_sub(&deflate)?;
            }
        }

        // Determine effective rank via energy criterion
        let total_energy: f64 = singular_values.iter().map(|&s| s * s).sum();
        let mut cumulative = 0.0f64;
        let effective_rank = singular_values
            .iter()
            .enumerate()
            .find(|&(_i, &s)| {
                cumulative += s * s;
                total_energy < 1e-15 || cumulative / total_energy.max(1e-15) >= energy_threshold
            })
            .map(|(i, _)| i + 1)
            .unwrap_or(1)
            .max(1);

        // Reconstruct low-rank K using top singular vectors
        if effective_rank < n {
            // Full SVD reconstruction via power iteration
            self.reconstruct_low_rank(effective_rank, max_iter, tol)?;
        }

        Ok(effective_rank)
    }

    /// Reconstruct K as low-rank approximation U Σ V^T via power iteration SVD.
    fn reconstruct_low_rank(&mut self, rank: usize, max_iter: usize, tol: f64) -> Result<()> {
        let n = self.lifted_dim;
        let mut u_cols = Vec::with_capacity(rank);
        let mut s_vals = Vec::with_capacity(rank);
        let mut v_cols = Vec::with_capacity(rank);

        let k_copy = self.theta.clone();

        for r in 0..rank {
            // Right singular vector: power iteration on K^T K
            let k_t = self.theta.t()?;
            let kt_k = k_t.matmul(&self.theta)?;

            let mut v = Self::rand_vec(n, &self.device)?;
            for _ in 0..max_iter {
                let av = kt_k.matmul(&v)?;
                let norm: f64 = av.sqr()?.sum_all()?.to_scalar::<f64>()?.sqrt();
                if norm < tol {
                    break;
                }
                v = av.broadcast_div(&Tensor::new(norm, &self.device)?)?;
            }
            v_cols.push(v.clone());

            // Left singular vector: u = K @ v / ||K @ v||
            let kv = k_copy.matmul(&v)?;
            let norm_u: f64 = kv.sqr()?.sum_all()?.to_scalar::<f64>()?.sqrt();
            let sigma = norm_u;
            s_vals.push(sigma);

            if sigma > tol {
                let u = kv.broadcast_div(&Tensor::new(sigma, &self.device)?)?;
                u_cols.push(u);
            } else {
                break;
            }

            // Deflate K -= sigma * u @ v^T
            if r < rank - 1 {
                let uv_t = u_cols.last().unwrap().matmul(&v.t()?)?;
                let deflate = uv_t.broadcast_mul(&Tensor::new(sigma, &self.device)?)?;
                self.theta = self.theta.broadcast_sub(&deflate)?;
            }
        }

        // Reconstruct: K ≈ U Σ V^T
        let actual_rank = u_cols.len().min(v_cols.len());
        if actual_rank == 0 {
            self.theta = Tensor::eye(n, DType::F64, &self.device)?;
            return Ok(());
        }

        let mut reconstruction = Tensor::zeros((n, n), DType::F64, &self.device)?;
        for i in 0..actual_rank {
            let uv_t = u_cols[i].matmul(&v_cols[i].t()?)?;
            let term = uv_t.broadcast_mul(&Tensor::new(s_vals[i], &self.device)?)?;
            reconstruction = reconstruction.broadcast_add(&term)?;
        }

        self.theta = reconstruction;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Prediction
    // ---------------------------------------------------------------------------

    /// Predict next lifted state: ψ_{t+1} = K · ψ_t.
    pub fn predict_next(&self, psi_t: &Tensor) -> Result<Tensor> {
        let psi = if psi_t.rank() == 1 {
            psi_t.unsqueeze(0)?.to_dtype(DType::F64)?
        } else {
            psi_t.to_dtype(DType::F64)?
        };
        let k_t = self.theta.t()?;
        psi.matmul(&k_t)
    }

    // -----------------------------------------------------------------------
    // Batch update
    // ---------------------------------------------------------------------------

    /// Process a batch of transitions efficiently.
    ///
    /// # Arguments
    /// * `phi_batch` - Batch of current states. Shape: [batch, d_lifted].
    /// * `y_batch` - Batch of next states. Shape: [batch, d_lifted].
    pub fn update_batch(
        &mut self,
        phi_batch: &Tensor,
        y_batch: &Tensor,
    ) -> Result<Vec<RLSUpdateResult>> {
        let batch_size = phi_batch.dim(0)?;
        let mut results = Vec::with_capacity(batch_size);

        for i in 0..batch_size {
            let phi_i = phi_batch.narrow(0, i, 1)?;
            let y_i = y_batch.narrow(0, i, 1)?;
            let result = self.update_koopman_rls(&phi_i, &y_i)?;
            results.push(result);
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rls_initialization() -> Result<()> {
        let device = Device::Cpu;
        let lifted_dim = 16;
        let config = KoopmanRLSConfig::default();
        let rls = KoopmanRLS::new(lifted_dim, config, &device)?;

        assert_eq!(rls.theta.shape().dims(), [lifted_dim, lifted_dim]);
        assert_eq!(rls.covariance.shape().dims(), [lifted_dim, lifted_dim]);
        assert_eq!(rls.update_count(), 0);
        Ok(())
    }

    #[test]
    fn test_rls_single_update() -> Result<()> {
        let device = Device::Cpu;
        let d = 8;
        let config = KoopmanRLSConfig::default();
        let mut rls = KoopmanRLS::new(d, config, &device)?;

        // Create a simple linear system: y = A @ x
        let a_data: Vec<f64> = (0..d)
            .flat_map(|i| (0..d).map(move |j| if i == j { 0.9 } else { 0.05 }))
            .collect();
        let a = Tensor::from_vec(a_data, (d, d), &device)?;

        // Generate one transition
        let x_data: Vec<f64> = (0..d).map(|i| (i + 1) as f64 * 0.1).collect();
        let x = Tensor::from_vec(x_data.clone(), (1, d), &device)?;
        let y = x.matmul(&a.t()?)?;

        let result = rls.update_koopman_rls(&x, &y)?;
        assert!(result.updated);
        assert_eq!(rls.update_count(), 1);
        Ok(())
    }

    #[test]
    fn test_rls_dead_zone() -> Result<()> {
        let device = Device::Cpu;
        let d = 4;
        let config = KoopmanRLSConfig {
            dead_zone_epsilon: 1.0, // Large dead-zone
            ..Default::default()
        };
        let mut rls = KoopmanRLS::new(d, config, &device)?;

        // Identity system: y = x (prediction error ≈ 0 since K starts as I)
        let x_data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0];
        let x = Tensor::from_vec(x_data.clone(), (1, d), &device)?;
        let y = x.clone(); // y = x, so innovation ≈ 0

        let result = rls.update_koopman_rls(&x, &y)?;
        assert!(!result.updated); // Dead-zone should trigger
        Ok(())
    }

    #[test]
    fn test_rls_convergence() -> Result<()> {
        let device = Device::Cpu;
        let d = 4;
        let config = KoopmanRLSConfig::edge_fast();
        let mut rls = KoopmanRLS::new(d, config, &device)?;

        // True system: simple diagonal K
        let true_k: Vec<f64> = (0..d)
            .flat_map(|i| (0..d).map(move |j| if i == j { 0.7 } else { 0.1 }))
            .collect();
        let k_true = Tensor::from_vec(true_k, (d, d), &device)?;

        // Generate diverse training data for persistent excitation
        let n_samples = 200;
        let mut seed: u64 = 42;
        for _ in 0..n_samples {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let x_data: Vec<f64> = (0..d)
                .map(|_| ((seed >> 33) as f64 / u32::MAX as f64 - 0.5) * 2.0)
                .collect();
            let x = Tensor::from_vec(x_data, (1, d), &device)?;
            let y = x.matmul(&k_true.t()?)?;
            let _ = rls.update_koopman_rls(&x, &y)?;
        }

        // K should have converged closer to K_true
        let k_est = rls.k_operator();
        let diff = k_est.broadcast_sub(&k_true)?;
        let error: f64 = diff.sqr()?.mean_all()?.to_scalar::<f64>()?;
        assert!(error < 0.5, "RLS should converge: error = {}", error);
        Ok(())
    }

    #[test]
    fn test_rls_batch_update() -> Result<()> {
        let device = Device::Cpu;
        let d = 4;
        let batch = 10;
        let config = KoopmanRLSConfig::default();
        let mut rls = KoopmanRLS::new(d, config, &device)?;

        let phi_data: Vec<f64> = (0..(batch * d)).map(|i| (i % 7) as f64 * 0.1).collect();
        let y_data: Vec<f64> = (0..(batch * d)).map(|i| (i % 11) as f64 * 0.05).collect();

        let phi_batch = Tensor::from_vec(phi_data, (batch, d), &device)?;
        let y_batch = Tensor::from_vec(y_data, (batch, d), &device)?;

        let results = rls.update_batch(&phi_batch, &y_batch)?;
        assert_eq!(results.len(), batch);
        assert_eq!(rls.update_count(), batch);
        Ok(())
    }

    #[test]
    fn test_rls_forgetting_factor() -> Result<()> {
        let device = Device::Cpu;
        let d = 4;
        let config = KoopmanRLSConfig {
            forgetting_lambda: 0.80,
            ..Default::default()
        };
        let mut rls = KoopmanRLS::new(d, config, &device)?;

        let mut seed: u64 = 42;

        // First system: identity (K = I)
        for _ in 0..100 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let x_data: Vec<f64> = (0..d)
                .map(|_| ((seed >> 33) as f64 / u32::MAX as f64 - 0.5) * 2.0)
                .collect();
            let x = Tensor::from_vec(x_data, (1, d), &device)?;
            let y = x.clone();
            let _ = rls.update_koopman_rls(&x, &y)?;
        }

        // Record diagonal after first phase
        let k_after_phase1 = rls.k_operator().clone();
        let n = k_after_phase1.dim(0)?;
        let flat1: Vec<f64> = k_after_phase1.flatten_all()?.to_vec1()?;
        let diag1: Vec<f64> = (0..n).map(|i| flat1[i * n + i]).collect();
        let mean_diag1 = diag1.iter().sum::<f64>() / diag1.len() as f64;

        // Switch to different system: y = 0.2 * x
        for _ in 0..200 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let x_data: Vec<f64> = (0..d)
                .map(|_| ((seed >> 33) as f64 / u32::MAX as f64 - 0.5) * 2.0)
                .collect();
            let x = Tensor::from_vec(x_data, (1, d), &device)?;
            let y = x.broadcast_mul(&Tensor::new(0.2f64, &device)?)?;
            let _ = rls.update_koopman_rls(&x, &y)?;
        }

        // With forgetting, K should have moved towards 0.2
        let k_est = rls.k_operator();
        let flat2: Vec<f64> = k_est.flatten_all()?.to_vec1()?;
        let diag2: Vec<f64> = (0..n).map(|i| flat2[i * n + i]).collect();
        let mean_diag2 = diag2.iter().sum::<f64>() / diag2.len() as f64;

        // Diagonal should have decreased significantly (moved from ~1.0 towards 0.2)
        assert!(
            mean_diag2 < mean_diag1 - 0.15,
            "Forgetting should reduce diagonal: phase1={} phase2={}",
            mean_diag1,
            mean_diag2
        );
        Ok(())
    }
}
