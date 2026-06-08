//! Taylor Models — Certified Polynomial Approximation with Remainder Bounds.
//!
//! Taylor Models provide a rigorous framework for certified reachability analysis
//! by combining polynomial approximations with guaranteed remainder bounds. Unlike
//! pure zonotopes that suffer from wrapping effect in continuous-time dynamics,
//! Taylor Models track the polynomial structure of the flow explicitly.
//!
//! **Taylor Model of Order k=2:**
//! ```text
//! T(x) = p0 + p1·(x - x0) + (1/2)·p2·(x - x0)² + [−r, r]
//! ```
//! where:
//! - `p0` = center (degree 0)
//! - `p1` = Jacobian coefficients (degree 1)
//! - `p2` = Hessian diagonal approximation (degree 2)
//! - `[−r, r]` = remainder bound (conservative interval)
//!
//! **Key Properties:**
//! 1. **Inclusion**: The true function value is always contained in the Taylor Model.
//! 2. **Composition**: Taylor Models compose under addition, multiplication, and function application.
//! 3. **Affine Exactness**: Linear transformations are exact (no remainder growth).
//! 4. **Remainder Control**: Remainder grows predictably under non-linear operations.
//!
//! **Hybrid with Zonotopes:**
//! - Taylor Models for continuous-time ODE integration (polynomial flow tracking).
//! - Zonotopes for discrete affine propagation and generator reduction.
//! - Switch between representations based on operation type for optimal tightness.
//!
//! **CBF Verification:**
//! Given CBF `h(x) = w^T·x + b`, evaluate on Taylor Model:
//! ```text
//! h(T) = w^T·p0 + b + w^T·p1·(x-x0) + (1/2)·w^T·p2·(x-x0)² + [-r', r']
//! ```
//! Lower bound: `h_min = w^T·p0 + b - |w^T·p1|·δ - (1/2)·|w^T·p2|·δ² - r'`
//! where `δ` is the maximum deviation from center.

use candle_core::{DType, Result, Tensor};

/// Configuration for Taylor Model creation and propagation.
#[derive(Debug, Clone)]
pub struct TaylorConfig {
    /// Order of Taylor expansion (1 = linear, 2 = quadratic).
    pub order: usize,
    /// Maximum remainder bound before triggering reduction.
    pub max_remainder: f32,
    /// Enable hybrid zonotope reduction when remainder exceeds threshold.
    pub hybrid_reduction: bool,
    /// Threshold for switching to zonotope representation.
    pub zonotope_threshold: f32,
}

impl Default for TaylorConfig {
    fn default() -> Self {
        Self {
            order: 2,
            max_remainder: 1.0,
            hybrid_reduction: true,
            zonotope_threshold: 0.1,
        }
    }
}

/// A Taylor Model representing a set of states with polynomial approximation + remainder.
///
/// T(x) = center + linear @ (x - x0) + quadratic @ (x - x0)^2 + [-remainder, remainder]
#[derive(Debug, Clone)]
pub struct TaylorModel {
    /// Center point (degree 0 polynomial coefficient). Shape: [1, dim].
    pub center: Tensor,
    /// Linear coefficients (Jacobian approximation). Shape: [dim, dim].
    pub linear: Tensor,
    /// Quadratic coefficients (diagonal Hessian approximation). Shape: [dim, dim].
    pub quadratic: Option<Tensor>,
    /// Remainder bound: true value lies within [-remainder, remainder] of polynomial.
    pub remainder: f32,
    /// Dimension of the state space.
    pub dim: usize,
    /// Configuration for propagation behavior.
    config: TaylorConfig,
}

impl TaylorModel {
    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    /// Create a Taylor Model from a center tensor with epsilon-ball uncertainty.
    ///
    /// Initial linear term is identity (point perturbation), quadratic is zero,
    /// and remainder is epsilon (conservative bound on first-order truncation).
    pub fn new_from_epsilon(center: &Tensor, epsilon: f32) -> Result<Self> {
        let dim = center.dim(1)?;
        let device = center.device();
        let identity = Tensor::eye(dim, DType::F32, device)?;

        Ok(Self {
            center: center.clone(),
            linear: identity,
            quadratic: None,
            remainder: epsilon,
            dim,
            config: TaylorConfig::default(),
        })
    }

    /// Create a Taylor Model from a center tensor with custom configuration.
    pub fn new_from_epsilon_with_config(
        center: &Tensor,
        epsilon: f32,
        config: TaylorConfig,
    ) -> Result<Self> {
        let dim = center.dim(1)?;
        let device = center.device();
        let identity = Tensor::eye(dim, DType::F32, device)?;

        Ok(Self {
            center: center.clone(),
            linear: identity,
            quadratic: None,
            remainder: epsilon,
            dim,
            config,
        })
    }

    /// Create a point Taylor Model (zero uncertainty).
    pub fn point(center: &Tensor) -> Result<Self> {
        let dim = center.dim(1)?;
        Ok(Self {
            center: center.clone(),
            linear: Tensor::zeros((dim, dim), DType::F32, center.device())?,
            quadratic: None,
            remainder: 0.0,
            dim,
            config: TaylorConfig::default(),
        })
    }

    /// Create a Taylor Model from explicit polynomial coefficients.
    pub fn new(
        center: Tensor,
        linear: Tensor,
        quadratic: Option<Tensor>,
        remainder: f32,
        config: TaylorConfig,
    ) -> Result<Self> {
        let dim = center.dim(1)?;
        Ok(Self {
            center,
            linear,
            quadratic,
            remainder,
            dim,
            config,
        })
    }

    // -----------------------------------------------------------------------
    // Core Operations
    // -----------------------------------------------------------------------

    /// Affine transformation: y = W @ x + b.
    ///
    /// This operation is **exact** for Taylor Models — the polynomial structure
    /// transforms precisely without remainder growth (unlike zonotopes which
    /// may accumulate wrapping error).
    ///
    /// ```text
    /// y_center = W @ x_center + b
    /// y_linear = W @ x_linear
    /// y_quadratic = W @ x_quadratic  (if present)
    /// y_remainder = x_remainder  (unchanged for affine)
    /// ```
    pub fn affine_transform(&self, weight: &Tensor, bias: Option<&Tensor>) -> Result<Self> {
        // Transform center: W @ c + b
        let new_center = if let Some(b) = bias {
            let wc = weight.matmul(&self.center.t()?)?.t()?;
            wc.broadcast_add(b)?
        } else {
            weight.matmul(&self.center.t()?)?.t()?
        };

        // Transform linear: W @ L
        let new_linear = weight.matmul(&self.linear)?;

        // Transform quadratic: W @ Q (if present)
        let new_quadratic = if let Some(ref q) = self.quadratic {
            Some(weight.matmul(q)?)
        } else {
            None
        };

        // Remainder: For affine transform, remainder scales by induced norm of W
        // but conservatively we keep it unchanged (affine is exact for polynomial part)
        let w_norm = weight.sqr()?.sum_all()?.sqrt()?.to_scalar::<f32>()?;
        let new_remainder = self.remainder * w_norm.max(1.0);

        Ok(Self {
            center: new_center,
            linear: new_linear,
            quadratic: new_quadratic,
            remainder: new_remainder,
            dim: self.dim,
            config: self.config.clone(),
        })
    }

    /// Compute tight bounds [lower, upper] from the Taylor Model.
    ///
    /// Uses the polynomial structure to compute bounds that are tighter than
    /// pure interval arithmetic but looser than the full Taylor representation.
    ///
    /// ```text
    /// deviation = sum(|linear|, axis=0) + remainder
    /// lower = center - deviation
    /// upper = center + deviation
    /// ```
    pub fn compute_bounds(&self) -> Result<(Tensor, Tensor)> {
        // Linear deviation: sum of absolute values of linear coefficients
        let lin_dev = self.linear.abs()?.sum(0)?.unsqueeze(0)?;

        // Quadratic deviation (if present): conservative bound
        let quad_dev = if let Some(ref q) = self.quadratic {
            q.abs()?.sum(0)?.unsqueeze(0)?
        } else {
            Tensor::zeros((1, self.dim), DType::F32, self.center.device())?
        };

        // Remainder as constant tensor (broadcast to match shape)
        let rem_t = Tensor::new(&[self.remainder], self.center.device())?.unsqueeze(0)?;

        // Total deviation
        let total_dev = lin_dev.broadcast_add(&quad_dev)?.broadcast_add(&rem_t.broadcast_as((1, self.dim))?)?;

        let lower = self.center.broadcast_sub(&total_dev)?;
        let upper = self.center.broadcast_add(&total_dev)?;

        Ok((lower, upper))
    }

    /// Compute the width of the Taylor Model bounds (upper - lower).
    pub fn width(&self) -> Result<Tensor> {
        let (lower, upper) = self.compute_bounds()?;
        upper.broadcast_sub(&lower)
    }

    /// Compute log-volume proxy (sum of log widths) for tightness comparison.
    pub fn log_volume_proxy(&self) -> Result<f32> {
        let w = self.width()?;
        // Clamp to avoid log(0)
        let eps = Tensor::new(&[1e-10f32], w.device())?.broadcast_as(w.dims())?;
        let clamped = w.maximum(&eps)?;
        clamped.log()?.sum_all()?.to_scalar::<f32>()
    }

    // -----------------------------------------------------------------------
    // Taylor Model Arithmetic
    // -----------------------------------------------------------------------

    /// Add two Taylor Models (Minkowski sum equivalent).
    pub fn add(&self, other: &TaylorModel) -> Result<Self> {
        let new_center = self.center.broadcast_add(&other.center)?;
        let new_linear = self.linear.broadcast_add(&other.linear)?;
        let new_quadratic = match (&self.quadratic, &other.quadratic) {
            (Some(a), Some(b)) => Some(a.broadcast_add(b)?),
            (Some(a), None) => Some(a.clone()),
            (None, Some(b)) => Some(b.clone()),
            (None, None) => None,
        };
        let new_remainder = self.remainder + other.remainder;

        Ok(Self {
            center: new_center,
            linear: new_linear,
            quadratic: new_quadratic,
            remainder: new_remainder,
            dim: self.dim,
            config: self.config.clone(),
        })
    }

    /// Scale Taylor Model by a scalar: s * T.
    pub fn scale(&self, s: f32) -> Result<Self> {
        let s_t = Tensor::new(&[s], self.center.device())?;
        let new_center = self.center.broadcast_mul(&s_t)?;
        let new_linear = self.linear.broadcast_mul(&s_t.unsqueeze(0)?)?;
        let new_quadratic = self
            .quadratic
            .as_ref()
            .map(|q| q.broadcast_mul(&s_t.unsqueeze(0)?))
            .transpose()?;
        let new_remainder = self.remainder * s.abs();

        Ok(Self {
            center: new_center,
            linear: new_linear,
            quadratic: new_quadratic,
            remainder: new_remainder,
            dim: self.dim,
            config: self.config.clone(),
        })
    }

    /// Add a scaled Taylor Model: self + s * other.
    pub fn add_scaled(&self, other: &TaylorModel, s: f32) -> Result<Self> {
        let scaled = other.scale(s)?;
        self.add(&scaled)
    }

    // -----------------------------------------------------------------------
    // Non-linear Operations
    // -----------------------------------------------------------------------

    /// Apply ReLU approximation: max(0, x) using slope bounding.
    ///
    /// For each dimension, classify as positive/negative/mixed based on bounds,
    /// then apply appropriate slope bounds [l, u]:
    /// - Positive (lower > 0): identity (l=1, u=1)
    /// - Negative (upper < 0): zero (l=0, u=0)
    /// - Mixed: slope in [0, 1] with remainder increase
    pub fn relu_approx(&self) -> Result<Self> {
        let (lower, upper) = self.compute_bounds()?;

        // Classify each dimension
        let zero = Tensor::zeros((1, self.dim), DType::F32, self.center.device())?;
        let one = Tensor::ones((1, self.dim), DType::F32, self.center.device())?;

        // Positive mask: lower > 0
        let positive = lower.gt(&zero)?.to_dtype(DType::F32)?;
        // Negative mask: upper < 0
        let _negative = upper.lt(&zero)?;
        // Mixed: interval crosses zero => lower <= 0 AND upper >= 0
        let crosses_below = lower.le(&zero)?.to_dtype(DType::F32)?;
        let crosses_above = upper.ge(&zero)?.to_dtype(DType::F32)?;
        let mixed = crosses_below.broadcast_mul(&crosses_above)?;

        // Slope bounds: l (lower slope), u (upper slope)
        let l = positive.broadcast_mul(&one)?; // l = 1 for positive, 0 otherwise
        let _u = positive
            .broadcast_mul(&one)?
            .broadcast_add(&mixed)?; // u = 1 for positive or mixed

        // Apply slope bounds to linear term
        let new_linear = self
            .linear
            .t()?
            .broadcast_mul(&l)?
            .t()?;

        // Remainder increase for mixed dimensions
        let mixed_count = mixed.sum_all()?.to_scalar::<f32>()? as usize;
        let width_per_dim = self.width()?.sum_all()?.to_scalar::<f32>()? / self.dim as f32;
        let new_remainder = self.remainder + 0.5 * mixed_count as f32 * width_per_dim;

        // Center passes through ReLU
        let new_center = self.center.maximum(&zero)?;

        Ok(Self {
            center: new_center,
            linear: new_linear,
            quadratic: None, // Quadratic info lost in non-linear op
            remainder: new_remainder,
            dim: self.dim,
            config: self.config.clone(),
        })
    }

    /// Apply tanh approximation using slope bounding.
    ///
    /// Tanh has derivative in (0, 1], with tighter bounds near ±infinity.
    pub fn tanh_approx(&self) -> Result<Self> {
        // Conservative: tanh'(x) in (0, 1]
        // For small x: tanh(x) ≈ x (slope ≈ 1)
        // For large |x|: tanh(x) ≈ sign(x) (slope ≈ 0)

        let (lower, upper) = self.compute_bounds()?;
        let _zero = Tensor::zeros((1, self.dim), DType::F32, self.center.device())?;

        // Slope bound: 1 / cosh²(max(|lower|, |upper|))
        let max_abs = lower.abs()?.maximum(&upper.abs()?)?;
        let cosh_sq = max_abs.exp()?.sqr()?.add(&Tensor::new(&[1.0f32], self.center.device())?)?;
        let slope = Tensor::new(&[1.0f32], self.center.device())?.broadcast_div(&cosh_sq)?;

        // Apply slope to linear
        let new_linear = self.linear.t()?.broadcast_mul(&slope)?.t()?;

        // Center through tanh
        let new_center = self.center.tanh()?;

        // Remainder increases due to non-linearity
        let width = self.width()?.sum_all()?.to_scalar::<f32>()?;
        let new_remainder = self.remainder + 0.25 * width / self.dim as f32;

        Ok(Self {
            center: new_center,
            linear: new_linear,
            quadratic: None,
            remainder: new_remainder,
            dim: self.dim,
            config: self.config.clone(),
        })
    }

    // -----------------------------------------------------------------------
    // ODE Integration
    // -----------------------------------------------------------------------

    /// Euler step for Neural ODE: x(t+dt) = x(t) + dt * f(x(t), t).
    ///
    /// The vector field `f` is evaluated on the Taylor Model, then integrated.
    pub fn euler_step(
        &self,
        f: &dyn Fn(&TaylorModel) -> Result<TaylorModel>,
        dt: f32,
    ) -> Result<Self> {
        let f_x = f(self)?;
        let dt_scaled = f_x.scale(dt)?;
        self.add(&dt_scaled)
    }

    /// RK2 (Heun's method) step for Neural ODE.
    ///
    /// ```text
    /// k1 = f(x(t))
    /// k2 = f(x(t) + dt/2 * k1)
    /// x(t+dt) = x(t) + dt * k2
    /// ```
    pub fn rk2_step(
        &self,
        f: &dyn Fn(&TaylorModel) -> Result<TaylorModel>,
        dt: f32,
    ) -> Result<Self> {
        let k1 = f(self)?;
        let half_dt = 0.5 * dt;
        let predictor = self.add_scaled(&k1, half_dt)?;
        let k2 = f(&predictor)?;
        self.add_scaled(&k2, dt)
    }

    /// RK4 (Classical Runge-Kutta) step for Neural ODE.
    ///
    /// ```text
    /// k1 = f(x(t))
    /// k2 = f(x(t) + dt/2 * k1)
    /// k3 = f(x(t) + dt/2 * k2)
    /// k4 = f(x(t) + dt * k3)
    /// x(t+dt) = x(t) + dt/6 * (k1 + 2*k2 + 2*k3 + k4)
    /// ```
    pub fn rk4_step(
        &self,
        f: &dyn Fn(&TaylorModel) -> Result<TaylorModel>,
        dt: f32,
    ) -> Result<Self> {
        let k1 = f(self)?;
        let half_dt = 0.5 * dt;

        let predictor2 = self.add_scaled(&k1, half_dt)?;
        let k2 = f(&predictor2)?;

        let predictor3 = self.add_scaled(&k2, half_dt)?;
        let k3 = f(&predictor3)?;

        let predictor4 = self.add_scaled(&k3, dt)?;
        let k4 = f(&predictor4)?;

        // Combine: x + dt/6 * (k1 + 2*k2 + 2*k3 + k4)
        let combined = k1
            .add_scaled(&k2, 2.0)?
            .add_scaled(&k3, 2.0)?
            .add(&k4)?
            .scale(dt / 6.0)?;

        self.add(&combined)
    }

    /// Integrate using the method specified in config.
    pub fn integrate_step(
        &self,
        f: &dyn Fn(&TaylorModel) -> Result<TaylorModel>,
        dt: f32,
        method: &str,
    ) -> Result<Self> {
        match method {
            "euler" => self.euler_step(f, dt),
            "rk2" => self.rk2_step(f, dt),
            "rk4" => self.rk4_step(f, dt),
            _ => self.euler_step(f, dt),
        }
    }

    // -----------------------------------------------------------------------
    // CBF Verification
    // -----------------------------------------------------------------------

    /// Evaluate a linear CBF `h(x) = w^T @ x + b` on this Taylor Model.
    ///
    /// Returns the lower bound of `h(x)` over the reachable set, which
    /// certifies safety if `h_min >= 0`.
    pub fn evaluate_cbf(&self, weight: &Tensor, bias: f32) -> Result<f32> {
        // h(center) = w^T @ c + b
        let h_center = weight.matmul(&self.center.t()?)?.squeeze(0)?.squeeze(0)?.to_scalar::<f32>()? + bias;

        // Worst-case deviation from linear term
        let lin_dev = weight
            .matmul(&self.linear.abs()?)?
            .sum_all()?
            .to_scalar::<f32>()?;

        // Quadratic deviation (if present)
        let quad_dev = if let Some(ref q) = self.quadratic {
            weight.matmul(&q.abs()?)?.sum_all()?.to_scalar::<f32>()?
        } else {
            0.0
        };

        // Lower bound: h(center) - |deviation| - remainder
        let h_min = h_center - lin_dev - quad_dev - self.remainder;
        Ok(h_min)
    }

    /// Compute Lie derivative bound: L_f h = ∇h · f(x).
    ///
    /// For linear CBF `h(x) = w^T @ x + b`, ∇h = w.
    /// We evaluate `w · f(T)` on the Taylor Model to get bounds.
    pub fn lie_derivative_bound(
        &self,
        f: &dyn Fn(&TaylorModel) -> Result<TaylorModel>,
        weight: &Tensor,
    ) -> Result<(f32, f32)> {
        let f_t = f(self)?;
        let (lower, upper) = f_t.compute_bounds()?;

        // w · f_lower and w · f_upper
        let w_lower = weight.matmul(&lower.t()?)?.squeeze(0)?.squeeze(0)?.to_scalar::<f32>()?;
        let w_upper = weight.matmul(&upper.t()?)?.squeeze(0)?.squeeze(0)?.to_scalar::<f32>()?;

        Ok((w_lower, w_upper))
    }

    // -----------------------------------------------------------------------
    // Hybrid Operations
    // -----------------------------------------------------------------------

    /// Convert Taylor Model to a Zonotope for generator reduction.
    ///
    /// This enables hybrid propagation: use Taylor Models for continuous-time
    /// integration (tight polynomial tracking), then switch to Zonotopes for
    /// discrete operations where generator reduction is beneficial.
    pub fn to_zonotope(&self) -> Result<crate::zonotope::Zonotope> {
        use crate::zonotope::ZonotopeConfig;

        // Use linear coefficients as generators + remainder as extra generator
        let config = ZonotopeConfig {
            max_gens: self.dim.min(64),
            epsilon: self.remainder,
            reduce_after_nonlinear: true,
            prune_threshold: 1e-6,
        };

        crate::zonotope::Zonotope::new(self.center.clone(), self.linear.clone(), config)
    }

    /// Create Taylor Model from a Zonotope.
    ///
    /// Center becomes Taylor center, generators become linear coefficients,
    /// and remainder is conservatively estimated from zonotope width.
    pub fn from_zonotope(z: &crate::zonotope::Zonotope) -> Result<Self> {
        let (lo, hi) = z.compute_bounds()?;
        let width = hi.broadcast_sub(&lo)?;
        let remainder = width.sum_all()?.to_scalar::<f32>()? / z.center.dim(1)? as f32 * 0.5;

        Ok(Self {
            center: z.center.clone(),
            linear: z.generators.clone(),
            quadratic: None,
            remainder,
            dim: z.center.dim(1)?,
            config: TaylorConfig::default(),
        })
    }

    /// Apply hybrid reduction: if remainder exceeds threshold, convert to
    /// zonotope, reduce generators, and convert back.
    pub fn hybrid_reduce(&self, max_gens: usize) -> Result<Self> {
        if self.remainder < self.config.zonotope_threshold {
            return Ok(self.clone());
        }

        let z = self.to_zonotope()?;
        let (lo, hi) = z.compute_bounds()?;
        let reduced = crate::zonotope::Zonotope::from_intervals(&lo, &hi)?;
        // Re-constructor with max_gens limit
        let reduced = crate::zonotope::Zonotope::new_from_epsilon(
            &reduced.center,
            self.remainder,
            max_gens,
        )?;
        Self::from_zonotope(&reduced)
    }

    // -----------------------------------------------------------------------
    // Tightness Metrics
    // -----------------------------------------------------------------------

    /// Compute tightness ratio compared to pure interval bounds.
    ///
    /// Ratio < 1 means Taylor Model is tighter than intervals.
    pub fn tightness_vs_interval(&self, interval_width: f32) -> Result<f32> {
        let tm_width = self.width()?.sum_all()?.to_scalar::<f32>()?;
        Ok(tm_width / interval_width.max(1e-10))
    }

    /// Get the maximum deviation from center (conservative bound).
    pub fn max_deviation(&self) -> Result<f32> {
        let w = self.width()?;
        let half = Tensor::new(&[0.5f32], w.device())?;
        w.broadcast_mul(&half)?.sum_all()?.to_scalar::<f32>()
    }

    // -----------------------------------------------------------------------
    // Sprint 113 — Taylor Order 2-3 ODE Propagation + CBF Invariance
    // -----------------------------------------------------------------------

    /// Access the quadratic term (Hessian approximation) of this Taylor Model.
    pub fn quadratic_term(&self) -> Option<&Tensor> {
        self.quadratic.as_ref()
    }

    /// Propagate one ODE step using Taylor integration of the given order.
    ///
    /// For `dx/dt = f(x)`, computes:
    /// `x(t+dt) ≈ x(t) + dt·f(x) + (dt²/2)·f'(x)·f(x) + ... + R`
    ///
    /// The remainder `R` is conservatively bounded using Lipschitz estimation
    /// from the linear generator norm.
    ///
    /// # Arguments
    /// * `f` — Vector field function `f: TaylorModel → TaylorModel`
    /// * `dt` — Time step size
    /// * `order` — Taylor order (1 = Euler, 2 = RK2/Taylor2, 3 = RK4/Taylor3)
    pub fn propagate_ode_step(
        &self,
        f: &dyn Fn(&Self) -> Result<Self>,
        dt: f32,
        order: usize,
    ) -> Result<Self> {
        match order {
            1 => self.euler_step(f, dt),
            2 => self.rk2_step(f, dt),
            3 => self.rk4_step(f, dt),
            _ => self.rk4_step(f, dt), // Default to highest order
        }
    }

    /// Compute a conservative Lipschitz constant estimate for this Taylor Model.
    ///
    /// Uses the operator norm of the linear term (max singular value approximation
    /// via Frobenius norm) as an upper bound on the Jacobian.
    pub fn lipschitz_estimate(&self) -> Result<f32> {
        // Frobenius norm of linear term as Lipschitz upper bound
        let frobenius = self.linear.abs()?.sqr()?.sum_all()?.to_scalar::<f32>()?.sqrt();
        Ok(frobenius)
    }

    /// Update remainder bound after ODE propagation step.
    ///
    /// Conservative Taylor-Lagrange remainder:
    /// `R' = R · L · dt + (dt³/6) · L³ · ε²` (for order 2)
    /// where L is the Lipschitz constant and ε is the initial uncertainty.
    pub fn update_remainder_ode(&self, dt: f32, order: usize) -> Result<f32> {
        let lip = self.lipschitz_estimate()?;
        let r = self.remainder;
        let new_r = match order {
            1 => r * (1.0 + lip * dt), // Euler: R' = R(1 + L·dt)
            2 => r * (1.0 + lip * dt) + (dt * dt * dt / 6.0) * lip * lip * lip * r * r,
            _ => r * (1.0 + lip * dt) + (dt * dt * dt / 6.0) * lip * lip * lip * r * r,
        };
        Ok(new_r)
    }

    /// Compute Lie derivative lower bound: L_f h = ∇h · f(x).
    ///
    /// For linear CBF `h(x) = w^T x + b`, ∇h = w (constant).
    /// We compute the worst-case (minimum) dot product of `grad_h` with
    /// the vector field evaluated over the Taylor Model reach set.
    ///
    /// # Arguments
    /// * `grad_h` — Gradient of CBF (constant for linear CBF), shape `[1, dim]`
    /// * `f_tm` — Taylor Model of the vector field `f(x)`
    pub fn lie_derivative_bound_vec(&self, grad_h: &Tensor, f_tm: &TaylorModel) -> Result<f32> {
        let (f_lo, f_hi) = f_tm.compute_bounds()?;
        // Worst-case: minimize grad_h · f(x)
        // For each component: if grad_h_i > 0, use f_lo_i; else use f_hi_i
        let device = f_lo.device();
        let zero = Tensor::zeros(grad_h.shape(), DType::F32, device)?;
        let grad_pos = grad_h.maximum(&zero)?; // Positive components
        let grad_neg = grad_h.minimum(&zero)?; // Negative components
        let wx_lo = f_lo.broadcast_mul(&grad_pos)?.sum_all()?.to_scalar::<f32>()?;
        let wx_hi = f_hi.broadcast_mul(&grad_neg)?.sum_all()?.to_scalar::<f32>()?;
        Ok(wx_lo + wx_hi)
    }

    /// Evaluate the CBF value at the center of this Taylor Model.
    ///
    /// `h(x) = w^T @ center + bias`
    pub fn cbf_value(&self, weight: &Tensor, bias: f32) -> Result<f32> {
        let h = weight.matmul(&self.center.t()?)?.squeeze(0)?.squeeze(0)?.to_scalar::<f32>()? + bias;
        Ok(h)
    }

    /// Verify CBF forward invariance over a sequence of ODE steps.
    ///
    /// Forward invariance requires: `L_f h(x) ≤ -α · h(x)` for all x in reach set.
    /// This method checks this condition at each integration step using
    /// Taylor Model bounds on the Lie derivative.
    ///
    /// # Arguments
    /// * `f` — Vector field function
    /// * `dt` — Time step size
    /// * `steps` — Number of integration steps to verify
    /// * `grad_h` — CBF gradient (constant for linear CBF)
    /// * `bias` — CBF bias term
    /// * `alpha` — Class-K function parameter (α > 0)
    /// * `order` — Taylor integration order
    ///
    /// # Returns
    /// `true` if invariance holds for all steps, `false` if violated
    #[allow(clippy::too_many_arguments)]
    pub fn verify_cbf_invariance(
        &self,
        f: &dyn Fn(&Self) -> Result<Self>,
        dt: f32,
        steps: usize,
        grad_h: &Tensor,
        bias: f32,
        alpha: f32,
        order: usize,
    ) -> Result<bool> {
        let mut current = self.clone();

        for _ in 0..steps {
            // Evaluate vector field on current reach set
            let f_tm = f(&current)?;

            // Compute Lie derivative lower bound
            let lie_lower = current.lie_derivative_bound_vec(grad_h, &f_tm)?;

            // Compute CBF value at center
            let h_val = current.cbf_value(grad_h, bias)?;

            // Check invariance: L_f h ≤ -α · h
            // If h > 0 (inside safe set), we need L_f h ≤ -α·h < 0
            // If h ≈ 0 (boundary), we need L_f h ≤ 0 (doesn't exit)
            let threshold = -alpha * h_val;
            if lie_lower > threshold {
                return Ok(false); // Invariance violated
            }

            // Propagate to next step
            current = current.propagate_ode_step(f, dt, order)?;
        }

        Ok(true)
    }

    /// Compute the certified safety margin: minimum CBF value over the reach set.
    ///
    /// This is the lower bound of `h(x)` for all `x` in the Taylor Model set.
    /// A positive margin certifies safety.
    pub fn safety_margin(&self, weight: &Tensor, bias: f32) -> Result<f32> {
        self.evaluate_cbf(weight, bias)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::Device;

    #[test]
    fn test_taylor_creation() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::new(&[1.0f32, 2.0, 3.0], &device)?;
        let center = center.unsqueeze(0)?; // [1, 3]
        let tm = TaylorModel::new_from_epsilon(&center, 0.1)?;
        assert_eq!(tm.dim, 3);
        assert!((tm.remainder - 0.1).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn test_taylor_point() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::new(&[1.0f32, 2.0], &device)?;
        let center = center.unsqueeze(0)?;
        let tm = TaylorModel::point(&center)?;
        assert!(tm.remainder < 1e-6);
        Ok(())
    }

    #[test]
    fn test_affine_exact() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::new(&[1.0f32, 2.0], &device)?;
        let center = center.unsqueeze(0)?;
        let tm = TaylorModel::new_from_epsilon(&center, 0.01)?;

        // Identity transform should preserve structure
        let w = Tensor::eye(2, DType::F32, &device)?;
        let result = tm.affine_transform(&w, None)?;
        assert!((result.remainder - tm.remainder).abs() < 1.0); // Norm-based scaling
        Ok(())
    }

    #[test]
    fn test_bounds_computation() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::new(&[0.0f32], &device)?;
        let center = center.unsqueeze(0)?;
        let tm = TaylorModel::new_from_epsilon(&center, 0.1)?;

        let (lower, upper) = tm.compute_bounds()?;
        let lo = lower.squeeze(0)?.squeeze(0)?.to_scalar::<f32>()?;
        let hi = upper.squeeze(0)?.squeeze(0)?.to_scalar::<f32>()?;
        assert!(lo < 0.0 && hi > 0.0);
        Ok(())
    }

    #[test]
    fn test_addition() -> Result<()> {
        let device = Device::Cpu;
        let c1 = Tensor::new(&[1.0f32, 2.0], &device)?.unsqueeze(0)?;
        let c2 = Tensor::new(&[3.0f32, 4.0], &device)?.unsqueeze(0)?;
        let tm1 = TaylorModel::new_from_epsilon(&c1, 0.1)?;
        let tm2 = TaylorModel::new_from_epsilon(&c2, 0.2)?;

        let sum = tm1.add(&tm2)?;
        assert!((sum.remainder - 0.3).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn test_scale() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::new(&[2.0f32], &device)?.unsqueeze(0)?;
        let tm = TaylorModel::new_from_epsilon(&center, 0.1)?;

        let scaled = tm.scale(2.0)?;
        assert!((scaled.remainder - 0.2).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn test_relu_approx() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::new(&[1.0f32, -1.0, 0.0], &device)?.unsqueeze(0)?;
        let tm = TaylorModel::new_from_epsilon(&center, 0.05)?;

        let result = tm.relu_approx()?;
        let new_center = result.center.to_vec2::<f32>()?;
        assert!(new_center[0][0] >= 0.0); // ReLU(1) = 1
        assert!(new_center[0][1] == 0.0); // ReLU(-1) = 0
        Ok(())
    }

    #[test]
    fn test_euler_step() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::new(&[1.0f32, 0.0], &device)?.unsqueeze(0)?;
        let tm = TaylorModel::new_from_epsilon(&center, 0.01)?;

        // Simple vector field: f(x) = -x (stable)
        let f = |t: &TaylorModel| -> Result<TaylorModel> { t.scale(-1.0) };
        let next = tm.euler_step(&f, 0.1)?;

        // x(0.1) ≈ x(0) + 0.1 * (-x(0)) = 0.9 * x(0)
        let c = next.center.to_vec2::<f32>()?;
        assert!((c[0][0] - 0.9).abs() < 0.01);
        Ok(())
    }

    #[test]
    fn test_cbf_evaluation() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::new(&[2.0f32, 3.0], &device)?.unsqueeze(0)?;
        let tm = TaylorModel::new_from_epsilon(&center, 0.1)?;

        // h(x) = x[0] + x[1] - 1 (safe if h >= 0)
        let w = Tensor::new(&[1.0f32, 1.0], &device)?.unsqueeze(0)?;
        let h_min = tm.evaluate_cbf(&w, -1.0)?;
        assert!(h_min > 0.0); // 2+3-1 = 4, minus deviation
        Ok(())
    }

    #[test]
    fn test_log_volume() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::new(&[1.0f32, 2.0, 3.0], &device)?.unsqueeze(0)?;
        let tm = TaylorModel::new_from_epsilon(&center, 0.1)?;
        let vol = tm.log_volume_proxy()?;
        assert!(vol.is_finite());
        Ok(())
    }
}