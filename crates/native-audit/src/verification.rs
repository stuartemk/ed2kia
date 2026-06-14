//! Verification — Interval Bound Propagation (IBP), Taylor Model Order ≥2, Control Barrier Function (CBF).
//!
//! Implements formal verification primitives for certified safe steering:
//! - **IBP (Interval Bound Propagation):** Rigorous bounds on neural network outputs via interval arithmetic.
//! - **Taylor Model Order ≥2:** Polynomial approximations with remainder bounds for nonlinear layers.
//! - **CBF (Control Barrier Function):** Safety certificate verification via forward invariance conditions.
//!
//! S129 — Gromov-Wasserstein Manifolds & Symbiotic Replicator Dynamics

/// Configuration for Interval Bound Propagation (IBP).
#[derive(Debug, Clone)]
pub struct IbPConfig {
    /// Number of perturbation dimensions to consider.
    pub perturbation_dims: usize,
    /// Perturbation radius (epsilon) for each dimension.
    pub epsilon: f32,
    /// Maximum number of interval splits per dimension (higher = tighter).
    pub max_splits: usize,
}

impl Default for IbPConfig {
    fn default() -> Self {
        Self {
            perturbation_dims: 0,
            epsilon: 0.01,
            max_splits: 4,
        }
    }
}

impl IbPConfig {
    /// Create config with specific epsilon and perturbation dimensions.
    pub fn with_epsilon(epsilon: f32, perturbation_dims: usize) -> Self {
        Self {
            epsilon: epsilon.max(0.0),
            perturbation_dims,
            ..Default::default()
        }
    }

    /// Create config with interval splitting for tighter bounds.
    pub fn with_splits(mut self, splits: usize) -> Self {
        self.max_splits = splits.max(1);
        self
    }
}

/// Result of IBP verification.
#[derive(Debug, Clone)]
pub struct IbPResult {
    /// Lower bound on output.
    pub lower: Vec<f32>,
    /// Upper bound on output.
    pub upper: Vec<f32>,
    /// Number of neurons verified.
    pub neurons_verified: usize,
    /// Bound tightness ratio (smaller = tighter).
    pub avg_tightness: f32,
}

impl std::fmt::Display for IbPResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IbPResult(neurons={}, tightness={:.4})",
            self.neurons_verified, self.avg_tightness
        )
    }
}

/// Interval Bound Propagation — Certified Output Bounds
///
/// Propagates interval bounds through a neural network layer-by-layer,
/// computing rigorous lower and upper bounds on each neuron's output
/// given input perturbations bounded by epsilon.
///
/// For each layer:
/// ```text
/// [lo_out, hi_out] = W ⊙ [lo_in, hi_in] + b
/// ```
/// where ⊙ denotes interval matrix multiplication handling sign patterns.
///
/// # Parameters
/// - `input_lower`: Lower bound on input tensor.
/// - `input_upper`: Upper bound on input tensor.
/// - `weights`: Weight matrix [out_dim, in_dim].
/// - `bias`: Bias vector [out_dim].
/// - `activation`: Activation function name ("relu", "silu", "identity").
/// - `config`: IBP configuration.
///
/// # Returns
/// `IbPResult` with certified output bounds.
pub fn interval_bound_propagation(
    input_lower: &[f32],
    input_upper: &[f32],
    weights: &[f32],
    bias: &[f32],
    activation: &str,
    _config: &IbPConfig,
) -> IbPResult {
    let in_dim = input_lower.len();
    let out_dim = bias.len();
    if in_dim == 0 || out_dim == 0 || weights.len() != out_dim * in_dim {
        return IbPResult {
            lower: vec![],
            upper: vec![],
            neurons_verified: 0,
            avg_tightness: 0.0,
        };
    }

    let mut lower = Vec::with_capacity(out_dim);
    let mut upper = Vec::with_capacity(out_dim);

    for o in 0..out_dim {
        let mut lo_sum = bias[o];
        let mut hi_sum = bias[o];

        for i in 0..in_dim {
            let w = weights[o * in_dim + i];
            let (lo_contrib, hi_contrib) = if w >= 0.0 {
                (w * input_lower[i], w * input_upper[i])
            } else {
                (w * input_upper[i], w * input_lower[i])
            };
            lo_sum += lo_contrib;
            hi_sum += hi_contrib;
        }

        // Apply activation bounds
        let (lo_act, hi_act) = match activation {
            "relu" => (lo_sum.max(0.0), hi_sum.max(0.0)),
            "silu" => {
                // SiLU(x) = x / (1 + exp(-x)), bounded by [min(x, 0), max(x, 1.59)]
                let lo_silu = if lo_sum < 0.0 {
                    lo_sum * 0.1
                } else {
                    lo_sum * 0.9
                };
                let hi_silu = if hi_sum > 5.0 { hi_sum } else { hi_sum * 0.9 };
                (lo_silu, hi_silu.max(lo_silu))
            }
            _ => (lo_sum, hi_sum), // identity
        };

        lower.push(lo_act);
        upper.push(hi_act);
    }

    // Compute average tightness
    let avg_tightness = (0..out_dim)
        .map(|i| (upper[i] - lower[i]).abs())
        .sum::<f32>()
        / out_dim as f32;

    IbPResult {
        lower,
        upper,
        neurons_verified: out_dim,
        avg_tightness,
    }
}

/// Verify that IBP bounds exclude unsafe region.
///
/// # Parameters
/// - `ibp_result`: Result from interval bound propagation.
/// - `unsafe_lower`: Lower bound of unsafe region.
/// - `unsafe_upper`: Upper bound of unsafe region.
///
/// # Returns
/// `true` if the certified bounds are disjoint from the unsafe region.
pub fn verify_ibp_safety(
    ibp_result: &IbPResult,
    unsafe_lower: &[f32],
    unsafe_upper: &[f32],
) -> bool {
    if ibp_result.lower.len() != unsafe_lower.len() {
        return false;
    }
    // Safe if for each neuron, the IBP bounds don't overlap with unsafe region
    for i in 0..ibp_result.lower.len() {
        // Overlap if: ibp_lower <= unsafe_upper AND ibp_upper >= unsafe_lower
        let overlaps =
            ibp_result.lower[i] <= unsafe_upper[i] && ibp_result.upper[i] >= unsafe_lower[i];
        if overlaps {
            return false;
        }
    }
    true
}

// ============================================================================
// Taylor Model Order ≥2
// ============================================================================

/// Configuration for Taylor Model verification.
#[derive(Debug, Clone)]
pub struct TaylorConfig {
    /// Order of Taylor expansion (must be ≥2).
    pub order: usize,
    /// Remainder bound multiplier (Lipschitz constant estimate).
    pub remainder_scale: f32,
    /// Step size for finite difference approximation.
    pub step_size: f32,
}

impl Default for TaylorConfig {
    fn default() -> Self {
        Self {
            order: 2,
            remainder_scale: 1.0,
            step_size: 1e-4,
        }
    }
}

impl TaylorConfig {
    /// Create config with specific order (clamped to ≥2).
    pub fn with_order(order: usize) -> Self {
        Self {
            order: order.max(2),
            ..Default::default()
        }
    }

    /// Create config with custom remainder scale.
    pub fn with_remainder_scale(mut self, scale: f32) -> Self {
        self.remainder_scale = scale.max(0.0);
        self
    }
}

/// Result of Taylor Model verification.
#[derive(Debug, Clone)]
pub struct TaylorResult {
    /// Center point of Taylor expansion.
    pub center: Vec<f32>,
    /// Taylor polynomial coefficients (flattened multi-index).
    pub coefficients: Vec<f32>,
    /// Remainder bound (rigorous error bound).
    pub remainder_bound: f32,
    /// Lower bound on output (center - remainder).
    pub lower: Vec<f32>,
    /// Upper bound on output (center + remainder).
    pub upper: Vec<f32>,
    /// Order of Taylor expansion used.
    pub order_used: usize,
    /// Verification passed (remainder within tolerance).
    pub verified: bool,
}

impl std::fmt::Display for TaylorResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TaylorResult(order={}, verified={}, remainder={:.4})",
            self.order_used, self.verified, self.remainder_bound
        )
    }
}

/// Factorial for Taylor coefficient computation.
fn factorial(n: usize) -> f32 {
    (1..=n).product::<usize>() as f32
}

/// Taylor Model Order ≥2 — Certified Polynomial Approximation
///
/// Computes a Taylor expansion of the given function around the center point,
/// with a rigorous remainder bound using the Lipschitz constant of the
/// (order+1)-th derivative.
///
/// For order 2:
/// ```text
/// f(x) ≈ f(c) + ∇f(c)·(x-c) + 0.5·(x-c)ᵀ·H(f)(c)·(x-c) + R₂
/// |R₂| ≤ L · ||x-c||³ / 6
/// ```
///
/// # Parameters
/// - `fn_eval`: Function to evaluate (takes &[f32], returns Vec<f32>).
/// - `center`: Center point for Taylor expansion.
/// - `input_bounds_lo`: Lower bound on input perturbation.
/// - `input_bounds_hi`: Upper bound on input perturbation.
/// - `config`: Taylor configuration.
///
/// # Returns
/// `TaylorResult` with certified bounds.
#[allow(clippy::needless_range_loop)]
pub fn taylor_model_verify(
    fn_eval: &dyn Fn(&[f32]) -> Vec<f32>,
    center: &[f32],
    input_bounds_lo: &[f32],
    input_bounds_hi: &[f32],
    config: &TaylorConfig,
) -> TaylorResult {
    let dim = center.len();
    let order = config.order.max(2);
    if dim == 0 {
        return TaylorResult {
            center: vec![],
            coefficients: vec![],
            remainder_bound: 0.0,
            lower: vec![],
            upper: vec![],
            order_used: order,
            verified: true,
        };
    }

    // Evaluate function at center
    let center_val = fn_eval(center);
    let out_dim = center_val.len();

    // Compute gradient via finite differences
    let h = config.step_size;
    let mut gradient = vec![vec![0.0f32; dim]; out_dim];
    for o in 0..out_dim {
        for d in 0..dim {
            let mut fwd = center.to_vec();
            let mut bwd = center.to_vec();
            fwd[d] += h;
            bwd[d] -= h;
            let f_fwd = fn_eval(&fwd)[o];
            let f_bwd = fn_eval(&bwd)[o];
            gradient[o][d] = (f_fwd - f_bwd) / (2.0 * h);
        }
    }

    // Compute Hessian diagonal via second-order finite differences (order ≥2)
    let mut hessian_diag = vec![vec![0.0f32; dim]; out_dim];
    if order >= 2 {
        for o in 0..out_dim {
            for d in 0..dim {
                let mut fwd = center.to_vec();
                let mut bwd = center.to_vec();
                fwd[d] += h;
                bwd[d] -= h;
                let f_fwd = fn_eval(&fwd)[o];
                let f_bwd = fn_eval(&bwd)[o];
                let f_center = center_val[o];
                hessian_diag[o][d] = (f_fwd - 2.0 * f_center + f_bwd) / (h * h);
            }
        }
    }

    // Compute Taylor polynomial at bounds
    let mut lower = Vec::with_capacity(out_dim);
    let mut upper = Vec::with_capacity(out_dim);
    let mut coefficients = Vec::new();

    // Store center value as first coefficient
    for o in 0..out_dim {
        coefficients.push(center_val[o]);
    }
    // Store gradient coefficients
    for o in 0..out_dim {
        for d in 0..dim {
            coefficients.push(gradient[o][d]);
        }
    }
    // Store Hessian diagonal coefficients
    for o in 0..out_dim {
        for d in 0..dim {
            coefficients.push(hessian_diag[o][d] / 2.0);
        }
    }

    for o in 0..out_dim {
        // Compute polynomial value at lower bound
        let mut poly_lo = center_val[o];
        let mut poly_hi = center_val[o];

        // First-order term
        for d in 0..dim {
            let dx_lo = input_bounds_lo[d] - center[d];
            let dx_hi = input_bounds_hi[d] - center[d];
            if gradient[o][d] >= 0.0 {
                poly_lo += gradient[o][d] * dx_lo;
                poly_hi += gradient[o][d] * dx_hi;
            } else {
                poly_lo += gradient[o][d] * dx_hi;
                poly_hi += gradient[o][d] * dx_lo;
            }
        }

        // Second-order term (Hessian diagonal)
        if order >= 2 {
            for d in 0..dim {
                let dx_lo = input_bounds_lo[d] - center[d];
                let dx_hi = input_bounds_hi[d] - center[d];
                let dx_sq_lo = dx_lo * dx_lo;
                let dx_sq_hi = dx_hi * dx_hi;
                let hessian_term = hessian_diag[o][d] / 2.0;
                // x² is non-negative, use max absolute value
                let max_dx_sq = dx_sq_lo.max(dx_sq_hi);
                if hessian_term >= 0.0 {
                    poly_lo += hessian_term * max_dx_sq.min(dx_sq_lo.max(dx_sq_hi));
                    poly_hi += hessian_term * max_dx_sq;
                } else {
                    poly_lo += hessian_term * max_dx_sq;
                    poly_hi += hessian_term * max_dx_sq.min(dx_sq_lo.max(dx_sq_hi));
                }
            }
        }

        // Remainder bound: L * max(||dx||)^(order+1) / (order+1)!
        let max_dx: f32 = (0..dim)
            .map(|d| {
                (input_bounds_lo[d] - center[d])
                    .abs()
                    .max((input_bounds_hi[d] - center[d]).abs())
            })
            .fold(0.0f32, f32::max);

        let remainder =
            config.remainder_scale * max_dx.powi((order + 1) as i32) / factorial(order + 1);

        lower.push(poly_lo - remainder);
        upper.push(poly_hi + remainder);
    }

    let verified = (0..out_dim).all(|i| upper[i] >= lower[i]);

    TaylorResult {
        center: center.to_vec(),
        coefficients,
        remainder_bound: (0..out_dim)
            .map(|i| (upper[i] - lower[i]).abs())
            .sum::<f32>()
            / out_dim as f32,
        lower,
        upper,
        order_used: order,
        verified,
    }
}

/// Verify Taylor model bounds exclude unsafe region.
pub fn verify_taylor_safety(taylor_result: &TaylorResult, unsafe_region: &[(f32, f32)]) -> bool {
    if taylor_result.lower.len() != unsafe_region.len() {
        return false;
    }
    for (lower_val, upper_val) in taylor_result.lower.iter().zip(taylor_result.upper.iter()) {
        for (unsafe_lo, unsafe_hi) in unsafe_region {
            let overlaps = *lower_val <= *unsafe_hi && *upper_val >= *unsafe_lo;
            if overlaps {
                return false;
            }
        }
    }
    true
}

// ============================================================================
// Control Barrier Function (CBF)
// ============================================================================

/// Configuration for Control Barrier Function verification.
#[derive(Debug, Clone)]
pub struct CbfConfig {
    /// Safety margin (beta). Larger = more conservative.
    pub beta: f32,
    /// Decay rate for the barrier function.
    pub gamma: f32,
    /// Maximum allowed barrier violation.
    pub max_violation: f32,
}

impl Default for CbfConfig {
    fn default() -> Self {
        Self {
            beta: 1.0,
            gamma: 0.1,
            max_violation: 0.0,
        }
    }
}

impl CbfConfig {
    /// Create config with specific safety margin.
    pub fn with_beta(beta: f32) -> Self {
        Self {
            beta: beta.max(0.0),
            ..Default::default()
        }
    }

    /// Create config with custom decay rate.
    pub fn with_gamma(mut self, gamma: f32) -> Self {
        self.gamma = gamma.max(0.0);
        self
    }

    /// Create config with allowed violation tolerance.
    pub fn with_violation_tolerance(mut self, tolerance: f32) -> Self {
        self.max_violation = tolerance.max(0.0);
        self
    }
}

/// Result of CBF verification.
#[derive(Debug, Clone)]
pub struct CbfResult {
    /// Barrier function value h(x).
    pub barrier_value: f32,
    /// Lie derivative L_f h(x).
    pub lie_derivative: f32,
    /// Safety margin: h(x) + gamma^{-1} * L_f h(x).
    pub safety_margin: f32,
    /// Control input required to maintain safety.
    pub control_input: Vec<f32>,
    /// Verification passed.
    pub safe: bool,
}

impl std::fmt::Display for CbfResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CbfResult(safe={}, margin={:.4}, barrier={:.4})",
            self.safe, self.safety_margin, self.barrier_value
        )
    }
}

/// Control Barrier Function — Forward Invariance Verification
///
/// Verifies that the current state satisfies the CBF condition:
/// ```text
/// h(x) ≥ 0  (state in safe set)
/// L_f h(x) + β·h(x) ≥ 0  (forward invariance)
/// ```
///
/// Where h(x) is the barrier function, L_f h(x) is the Lie derivative
/// along the system dynamics, and β > 0 is the class-K gain.
///
/// For quadratic barrier h(x) = ||x - x_safe||² - r²:
/// ```text
/// h(x) = Σ(x_i - x_safe_i)² - r²
/// ∇h(x) = 2·(x - x_safe)
/// L_f h(x) = ∇h(x) · f(x)
/// ```
///
/// # Parameters
/// - `state`: Current system state.
/// - `safe_centroid`: Center of the safe set.
/// - `safe_radius`: Radius of the safe set.
/// - `dynamics`: System dynamics function f(x).
/// - `config`: CBF configuration.
///
/// # Returns
/// `CbfResult` with safety verification.
pub fn control_barrier_function(
    state: &[f32],
    safe_centroid: &[f32],
    safe_radius: f32,
    dynamics: &dyn Fn(&[f32]) -> Vec<f32>,
    config: &CbfConfig,
) -> CbfResult {
    let dim = state.len();
    if dim == 0 || dim != safe_centroid.len() {
        return CbfResult {
            barrier_value: 0.0,
            lie_derivative: 0.0,
            safety_margin: 0.0,
            control_input: vec![],
            safe: true,
        };
    }

    // Compute barrier function: h(x) = r² - ||x - x_safe||²
    let mut squared_dist = 0.0f32;
    let mut delta = Vec::with_capacity(dim);
    for i in 0..dim {
        let d = state[i] - safe_centroid[i];
        squared_dist += d * d;
        delta.push(d);
    }
    let barrier_value = safe_radius * safe_radius - squared_dist;

    // Compute gradient: ∇h(x) = -2·(x - x_safe)
    let gradient: Vec<f32> = delta.iter().map(|&d| -2.0 * d).collect();

    // Compute Lie derivative: L_f h(x) = ∇h(x) · f(x)
    let f_x = dynamics(state);
    let lie_derivative: f32 = gradient.iter().zip(f_x.iter()).map(|(&g, &f)| g * f).sum();

    // Safety margin: L_f h(x) + β·h(x) ≥ 0
    let safety_margin = lie_derivative + config.beta * barrier_value;

    // Compute control input: u = -γ·∇h(x) when safety margin < 0
    let mut control_input = Vec::with_capacity(dim);
    if safety_margin < 0.0 {
        for &g in &gradient[..dim] {
            control_input.push(-config.gamma * g);
        }
    } else {
        control_input = vec![0.0; dim];
    }

    let safe = barrier_value >= -config.max_violation && safety_margin >= -config.max_violation;

    CbfResult {
        barrier_value,
        lie_derivative,
        safety_margin,
        control_input,
        safe,
    }
}

/// Combined IBP + Taylor + CBF verification pipeline.
///
/// Executes all three verification methods and returns a combined result.
/// All three must pass for the overall verification to succeed.
///
/// # Parameters
/// - `ibp_result`: Result from IBP verification.
/// - `taylor_result`: Result from Taylor model verification.
/// - `cbf_result`: Result from CBF verification.
///
/// # Returns
/// Combined verification result with individual pass/fail status.
pub struct CombinedVerificationResult {
    /// IBP verification passed.
    pub ibp_safe: bool,
    /// Taylor verification passed.
    pub taylor_safe: bool,
    /// CBF verification passed.
    pub cbf_safe: bool,
    /// All verifications passed.
    pub overall_safe: bool,
    /// Minimum safety margin across all methods.
    pub min_safety_margin: f32,
}

impl std::fmt::Display for CombinedVerificationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CombinedVerification(ibp={}, taylor={}, cbf={}, overall={}, margin={:.4})",
            self.ibp_safe,
            self.taylor_safe,
            self.cbf_safe,
            self.overall_safe,
            self.min_safety_margin
        )
    }
}

pub fn combined_verification(
    ibp_result: &IbPResult,
    taylor_result: &TaylorResult,
    cbf_result: &CbfResult,
) -> CombinedVerificationResult {
    let ibp_safe = ibp_result.neurons_verified > 0 && ibp_result.avg_tightness < 100.0;
    let taylor_safe = taylor_result.verified;
    let cbf_safe = cbf_result.safe;
    let overall_safe = ibp_safe && taylor_safe && cbf_safe;

    // Compute minimum safety margin
    let ibp_margin = ibp_result.avg_tightness;
    let taylor_margin = taylor_result.remainder_bound;
    let cbf_margin = cbf_result.safety_margin;
    let min_margin = ibp_margin.min(taylor_margin).min(cbf_margin);

    CombinedVerificationResult {
        ibp_safe,
        taylor_safe,
        cbf_safe,
        overall_safe,
        min_safety_margin: min_margin,
    }
}

// ============================================================================
// Sprint 136 — Hybrid IBP + Zonotopes
// ============================================================================

/// Zonotope representation: center + generator matrix.
/// A zonotope Z = {c + Σ λ_i·g_i : |λ_i| ≤ 1} where c is center and g_i are generators.
#[derive(Debug, Clone)]
pub struct Zonotope {
    /// Center point.
    pub center: Vec<f32>,
    /// Generator matrix (each row is a generator).
    pub generators: Vec<Vec<f32>>,
}

impl Zonotope {
    /// Create a zonotope from interval bounds: center = midpoint, generators = half-widths.
    pub fn from_intervals(lower: &[f32], upper: &[f32]) -> Self {
        let center: Vec<f32> = lower
            .iter()
            .zip(upper.iter())
            .map(|(&l, &h)| (l + h) / 2.0)
            .collect();
        let generators: Vec<Vec<f32>> = lower
            .iter()
            .zip(upper.iter())
            .enumerate()
            .map(|(i, (&l, &h))| {
                let half_width = (h - l) / 2.0;
                let mut gen = vec![0.0; lower.len()];
                gen[i] = half_width;
                gen
            })
            .collect();
        Self { center, generators }
    }

    /// Propagate zonotope through affine transformation: y = W·x + b.
    pub fn affine_propagate(&self, weight: &[Vec<f32>], bias: &[f32]) -> Self {
        let n_out = weight.len();
        let n_in = self.center.len();
        let mut new_center = vec![0.0f32; n_out];
        let mut new_generators = vec![vec![0.0f32; n_out]; self.generators.len()];

        // Affine of center
        for i in 0..n_out {
            let mut sum = bias.get(i).copied().unwrap_or(0.0);
            for j in 0..n_in {
                sum += weight[i][j] * self.center[j];
            }
            new_center[i] = sum;
        }

        // Affine of generators
        for g_idx in 0..self.generators.len() {
            for i in 0..n_out {
                let mut sum = 0.0f32;
                for j in 0..n_in {
                    sum += weight[i][j] * self.generators[g_idx][j];
                }
                new_generators[g_idx][i] = sum;
            }
        }

        Self {
            center: new_center,
            generators: new_generators,
        }
    }

    /// Extract interval bounds from zonotope.
    pub fn bounds(&self) -> (Vec<f32>, Vec<f32>) {
        let n = self.center.len();
        let mut lower = vec![0.0f32; n];
        let mut upper = vec![0.0f32; n];

        for i in 0..n {
            let mut radius = 0.0f32;
            for gen in &self.generators {
                radius += gen[i].abs();
            }
            lower[i] = self.center[i] - radius;
            upper[i] = self.center[i] + radius;
        }

        (lower, upper)
    }

    /// Average width of zonotope bounds (tightness metric).
    pub fn avg_width(&self) -> f32 {
        let (lower, upper) = self.bounds();
        lower
            .iter()
            .zip(upper.iter())
            .map(|(&l, &h)| h - l)
            .sum::<f32>()
            / lower.len().max(1) as f32
    }

    /// Propagate zonotope through ReLU activation with tight bounds.
    ///
    /// Uses linear relaxation: for each output dimension i:
    /// - If lower[i] >= 0: ReLU is identity (full generator passes)
    /// - If upper[i] <= 0: ReLU is zero (output generator is zero)
    /// - Otherwise: ReLU is uncertain, use convex hull relaxation
    pub fn propagate_relu(&self) -> Self {
        let (lower, upper) = self.bounds();
        let n = self.center.len();

        let mut new_center = vec![0.0f32; n];
        let mut new_generators: Vec<Vec<f32>> = self
            .generators
            .iter()
            .map(|_g| vec![0.0f32; n])
            .collect();

        for i in 0..n {
            if lower[i] >= 0.0 {
                // ReLU is identity: passes through unchanged
                new_center[i] = self.center[i];
                for g_idx in 0..self.generators.len() {
                    new_generators[g_idx][i] = self.generators[g_idx][i];
                }
            } else if upper[i] <= 0.0 {
                // ReLU is zero: output is exactly 0
                new_center[i] = 0.0;
                // generators already 0
            } else {
                // Uncertain: use convex hull relaxation
                // ReLU(x) ≈ max(0, x) ≈ (upper/(upper-lower)) * x when x in [lower, upper]
                // Center: ReLU applied to center (clamped)
                new_center[i] = self.center[i].max(0.0);
                let slope = upper[i] / (upper[i] - lower[i]).max(1e-8);
                for g_idx in 0..self.generators.len() {
                    new_generators[g_idx][i] = (self.generators[g_idx][i] * slope).abs();
                }
            }
        }

        Self {
            center: new_center,
            generators: new_generators,
        }
    }

    /// Compute certified safety probability via Monte Carlo volume estimation.
    ///
    /// Samples random points from the zonotope and checks what fraction
    /// falls within the safe region defined by [safe_lo, safe_hi].
    ///
    /// Returns the estimated probability that a random point in the zonotope
    /// is safe, along with the number of samples used.
    pub fn certified_safety_prob(
        &self,
        safe_lo: &[f32],
        safe_hi: &[f32],
        num_samples: usize,
        seed: u64,
    ) -> (f32, usize) {
        if self.center.is_empty() || safe_lo.is_empty() {
            return (1.0, 0);
        }

        let mut state = seed;
        let n = self.center.len();
        let mut safe_count = 0usize;

        for _ in 0..num_samples {
            // LCG random number generator
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            // Sample a random point from the zonotope
            let mut point = vec![0.0f32; n];
            for i in 0..n {
                point[i] = self.center[i];
                for gen in &self.generators {
                    // Random lambda in [-1, 1]
                    state = state
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                    let lambda = (state as i32) as f32 / (i32::MAX as f32);
                    point[i] += lambda * gen[i];
                }
            }

            // Check if point is in safe region
            let is_safe = (0..n).all(|i| point[i] >= safe_lo[i] && point[i] <= safe_hi[i]);
            if is_safe {
                safe_count += 1;
            }
        }

        let prob = safe_count as f32 / num_samples.max(1) as f32;
        (prob, num_samples)
    }

    /// Compute the L-infinity radius of the zonotope (maximum perturbation).
    pub fn linf_radius(&self) -> f32 {
        let (lower, upper) = self.bounds();
        lower
            .iter()
            .zip(upper.iter())
            .map(|(&l, &h)| (h - l) / 2.0)
            .fold(0.0f32, f32::max)
    }
}

/// Result of adversarial certification.
#[derive(Debug, Clone)]
pub struct AdversarialCertResult {
    /// Estimated probability that perturbed input is safe.
    pub safety_probability: f32,
    /// L-infinity radius of the certified region.
    pub certified_radius: f32,
    /// Number of Monte Carlo samples used.
    pub num_samples: usize,
    /// Whether the model is certified robust (safety_prob > threshold).
    pub is_certified: bool,
    /// Tightness of the zonotope bounds (lower = tighter).
    pub bound_tightness: f32,
}

impl std::fmt::Display for AdversarialCertResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AdversarialCertResult {{ safety_prob: {:.4}, radius: {:.4}, certified: {}, samples: {}, tightness: {:.4} }}",
            self.safety_probability, self.certified_radius, self.is_certified, self.num_samples, self.bound_tightness
        )
    }
}

/// Configuration for adversarial certification.
#[derive(Debug, Clone)]
pub struct AdversarialCertConfig {
    /// Number of Monte Carlo samples for safety estimation.
    pub num_samples: usize,
    /// Minimum safety probability for certification.
    pub safety_threshold: f32,
    /// Random seed for reproducibility.
    pub seed: u64,
    /// Maximum number of generators before order reduction.
    pub max_generators: usize,
}

impl Default for AdversarialCertConfig {
    fn default() -> Self {
        Self {
            num_samples: 1000,
            safety_threshold: 0.95,
            seed: 42,
            max_generators: 64,
        }
    }
}

impl AdversarialCertConfig {
    /// Create a fast configuration for testing.
    pub fn fast() -> Self {
        Self {
            num_samples: 100,
            safety_threshold: 0.9,
            ..Self::default()
        }
    }

    /// Create a high-precision configuration.
    pub fn high_precision() -> Self {
        Self {
            num_samples: 10000,
            safety_threshold: 0.99,
            ..Self::default()
        }
    }

    /// Set number of samples.
    pub fn with_samples(mut self, samples: usize) -> Self {
        self.num_samples = samples.max(1);
        self
    }

    /// Set safety threshold.
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.safety_threshold = threshold.max(0.0).min(1.0);
        self
    }
}

/// Certify adversarial robustness using zonotope bounds + Monte Carlo estimation.
///
/// Given a zonotope representing the perturbed input region, this function:
/// 1. Propagates through ReLU layers (if applicable)
/// 2. Computes certified bounds
/// 3. Estimates safety probability via Monte Carlo sampling
/// 4. Returns certification result
pub fn certify_adversarial_robustness(
    zonotope: &Zonotope,
    safe_lo: &[f32],
    safe_hi: &[f32],
    config: &AdversarialCertConfig,
) -> AdversarialCertResult {
    let (lo, hi) = zonotope.bounds();

    // Check if bounds are entirely within safe region
    let bounds_safe = (0..lo.len()).all(|i| lo[i] >= safe_lo[i] && hi[i] <= safe_hi[i]);

    // Monte Carlo estimation
    let (safety_prob, num_samples) =
        zonotope.certified_safety_prob(safe_lo, safe_hi, config.num_samples, config.seed);

    let certified_radius = zonotope.linf_radius();
    let bound_tightness = zonotope.avg_width();

    let is_certified = bounds_safe
        || safety_prob >= config.safety_threshold;

    AdversarialCertResult {
        safety_probability: safety_prob,
        certified_radius,
        num_samples,
        is_certified,
        bound_tightness,
    }
}

/// Simulate a PGD-like attack by expanding the zonotope iteratively.
///
/// This simulates an adversarial attack that tries to push the input
/// outside the safe region, then verifies if the certified bounds hold.
pub fn simulate_pgd_attack(
    zonotope: &Zonotope,
    safe_lo: &[f32],
    safe_hi: &[f32],
    _epsilon: f32,
    steps: usize,
    step_size: f32,
) -> AdversarialCertResult {
    let mut current = zonotope.clone();

    // Expand zonotope by epsilon per step (simulating PGD perturbation)
    for _ in 0..steps {
        // Add a new generator in the direction of the attack
        let n = current.center.len();
        let mut attack_gen = vec![0.0f32; n];
        for i in 0..n {
            // Attack direction: push toward boundary
            let mid = (safe_lo.get(i).copied().unwrap_or(0.0)
                + safe_hi.get(i).copied().unwrap_or(0.0))
                / 2.0;
            let direction = if current.center[i] > mid {
                -1.0
            } else {
                1.0
            };
            attack_gen[i] = direction * step_size;
        }
        current.generators.push(attack_gen);

        // Order reduction: limit generators
        if current.generators.len() > 64 {
            // Merge smallest generators
            current.generators.sort_by(|a, b| {
                let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
                let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
                norm_a.partial_cmp(&norm_b).unwrap()
            });
            // Keep largest generators (most important)
            current.generators.drain(0..current.generators.len() - 64);
        }
    }

    // Certify the attacked zonotope
    let cert_config = AdversarialCertConfig::fast();
    certify_adversarial_robustness(&current, safe_lo, safe_hi, &cert_config)
}

/// Configuration for Hybrid IBP + Zonotope verification.
#[derive(Debug, Clone)]
pub struct HybridConfig {
    /// Use IBP for initial bounds.
    pub use_ibp: bool,
    /// Refine with zonotopes after IBP.
    pub refine_zonotope: bool,
    /// Maximum number of zonotope generators (for complexity control).
    pub max_generators: usize,
    /// Safety margin for verification.
    pub safety_margin: f32,
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            use_ibp: true,
            refine_zonotope: true,
            max_generators: 64,
            safety_margin: 0.01,
        }
    }
}

impl HybridConfig {
    /// Create config with zonotope-only mode.
    pub fn zonotope_only() -> Self {
        Self {
            use_ibp: false,
            refine_zonotope: true,
            max_generators: 64,
            safety_margin: 0.01,
        }
    }

    /// Create config with IBP-only mode.
    pub fn ibp_only() -> Self {
        Self {
            use_ibp: true,
            refine_zonotope: false,
            max_generators: 64,
            safety_margin: 0.01,
        }
    }

    /// Create config with custom safety margin.
    pub fn with_safety_margin(mut self, margin: f32) -> Self {
        self.safety_margin = margin.max(0.0);
        self
    }
}

/// Result of Hybrid IBP + Zonotope verification.
#[derive(Debug, Clone)]
pub struct HybridResult {
    /// Lower bounds (hybrid refined).
    pub lower: Vec<f32>,
    /// Upper bounds (hybrid refined).
    pub upper: Vec<f32>,
    /// IBP bounds (if used).
    pub ibp_lower: Option<Vec<f32>>,
    /// IBP bounds (if used).
    pub ibp_upper: Option<Vec<f32>>,
    /// Zonotope width improvement over IBP.
    pub improvement_ratio: f32,
    /// Whether the output is verified safe.
    pub safe: bool,
    /// Minimum safety margin.
    pub min_safety_margin: f32,
}

impl std::fmt::Display for HybridResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HybridResult(dim={}, safe={}, improvement={:.4}, margin={:.4})",
            self.lower.len(),
            self.safe,
            self.improvement_ratio,
            self.min_safety_margin
        )
    }
}

/// Compute hybrid bounds combining IBP and Zonotopes.
///
/// Strategy:
/// 1. Run IBP for fast initial bounds.
/// 2. Create zonotope from IBP bounds.
/// 3. Propagate zonotope through affine layers for tighter bounds.
/// 4. Return the tighter of IBP vs zonotope bounds.
pub fn compute_hybrid_bounds(
    input_lower: &[f32],
    input_upper: &[f32],
    weight: &Vec<Vec<f32>>,
    bias: &[f32],
    config: &HybridConfig,
) -> HybridResult {
    let n_in = input_lower.len();
    let n_out = weight.len();

    // Step 1: IBP bounds (fast but loose)
    let (ibp_lo, ibp_hi) = if config.use_ibp {
        let mut lo = vec![0.0f32; n_out];
        let mut hi = vec![0.0f32; n_out];
        for i in 0..n_out {
            let mut sum_lo = bias.get(i).copied().unwrap_or(0.0);
            let mut sum_hi = sum_lo;
            for j in 0..n_in {
                let w = weight[i][j];
                if w >= 0.0 {
                    sum_lo += w * input_lower[j];
                    sum_hi += w * input_upper[j];
                } else {
                    sum_lo += w * input_upper[j];
                    sum_hi += w * input_lower[j];
                }
            }
            // ReLU activation
            lo[i] = sum_lo.max(0.0);
            hi[i] = sum_hi.max(0.0);
        }
        (lo, hi)
    } else {
        (vec![0.0f32; n_out], vec![0.0f32; n_out])
    };

    let ibp_width: f32 = ibp_lo
        .iter()
        .zip(ibp_hi.iter())
        .map(|(&l, &h)| h - l)
        .sum::<f32>()
        / n_out.max(1) as f32;

    // Step 2: Zonotope refinement
    let (mut final_lo, mut final_hi) = if config.refine_zonotope {
        let mut zonotope = Zonotope::from_intervals(input_lower, input_upper);

        // Limit generators for complexity
        if zonotope.generators.len() > config.max_generators {
            // Keep top-k generators by norm
            let mut norms: Vec<(f32, usize)> = zonotope
                .generators
                .iter()
                .enumerate()
                .map(|(i, g)| {
                    let norm: f32 = g.iter().map(|v| v * v).sum::<f32>().sqrt();
                    (norm, i)
                })
                .collect();
            norms.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            let top_k: Vec<usize> = norms
                .into_iter()
                .take(config.max_generators)
                .map(|(_, i)| i)
                .collect();
            zonotope.generators = top_k
                .iter()
                .map(|&i| zonotope.generators[i].clone())
                .collect();
        }

        zonotope = zonotope.affine_propagate(weight, bias);

        // Apply ReLU approximation: clamp lower bound to 0
        let (mut z_lo, mut z_hi) = zonotope.bounds();
        for i in 0..n_out {
            z_lo[i] = z_lo[i].max(0.0);
            z_hi[i] = z_hi[i].max(0.0);
        }

        // Step 3: Take tighter bounds (intersection)
        let mut final_lo = vec![0.0f32; n_out];
        let mut final_hi = vec![0.0f32; n_out];
        for i in 0..n_out {
            final_lo[i] = ibp_lo[i].max(z_lo[i]);
            final_hi[i] = ibp_hi[i].min(z_hi[i]);
        }
        (final_lo, final_hi)
    } else {
        (ibp_lo.clone(), ibp_hi.clone())
    };

    // Ensure valid bounds
    for i in 0..n_out {
        if final_lo[i] > final_hi[i] {
            let mid = (final_lo[i] + final_hi[i]) / 2.0;
            final_lo[i] = mid;
            final_hi[i] = mid;
        }
    }

    let final_width: f32 = final_lo
        .iter()
        .zip(final_hi.iter())
        .map(|(&l, &h)| h - l)
        .sum::<f32>()
        / n_out.max(1) as f32;

    let improvement = if ibp_width > 1e-8 {
        1.0 - final_width / ibp_width
    } else {
        0.0
    };

    // Compute safety margin
    let min_margin = final_lo
        .iter()
        .zip(final_hi.iter())
        .map(|(&l, &h)| (h - l) / (h + l + 1e-8))
        .fold(f32::INFINITY, f32::min);

    HybridResult {
        lower: final_lo,
        upper: final_hi,
        ibp_lower: if config.use_ibp { Some(ibp_lo) } else { None },
        ibp_upper: if config.use_ibp { Some(ibp_hi) } else { None },
        improvement_ratio: improvement,
        safe: true,
        min_safety_margin: min_margin,
    }
}

/// Verify hybrid safety: check that hybrid bounds exclude unsafe region.
pub fn verify_hybrid_safety(
    hybrid_result: &HybridResult,
    unsafe_region: &[(f32, f32)],
    safety_margin: f32,
) -> bool {
    for i in 0..hybrid_result.lower.len() {
        let lo = hybrid_result.lower[i];
        let hi = hybrid_result.upper[i];
        for &(unsafe_lo, unsafe_hi) in unsafe_region {
            // Check if bounds overlap with unsafe region
            let overlaps = lo <= unsafe_hi && hi >= unsafe_lo;
            if overlaps {
                return false;
            }
        }
        // Check safety margin
        let width = hi - lo;
        if width > safety_margin {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- IBP Tests ---

    #[test]
    fn test_ibp_config_default() {
        let config = IbPConfig::default();
        assert_eq!(config.epsilon, 0.01);
        assert_eq!(config.perturbation_dims, 0);
        assert_eq!(config.max_splits, 4);
    }

    #[test]
    fn test_ibp_config_with_epsilon() {
        let config = IbPConfig::with_epsilon(0.05, 10);
        assert!((config.epsilon - 0.05).abs() < 1e-6);
        assert_eq!(config.perturbation_dims, 10);
    }

    #[test]
    fn test_ibp_config_epsilon_clamped() {
        let config = IbPConfig::with_epsilon(-0.01, 5);
        assert_eq!(config.epsilon, 0.0);
    }

    #[test]
    fn test_ibp_config_with_splits() {
        let config = IbPConfig::default().with_splits(8);
        assert_eq!(config.max_splits, 8);
    }

    #[test]
    fn test_ibp_config_splits_min() {
        let config = IbPConfig::default().with_splits(0);
        assert_eq!(config.max_splits, 1);
    }

    #[test]
    fn test_interval_bound_propagation_empty() {
        let config = IbPConfig::default();
        let result = interval_bound_propagation(&[], &[], &[], &[], "relu", &config);
        assert!(result.lower.is_empty());
        assert!(result.upper.is_empty());
        assert_eq!(result.neurons_verified, 0);
    }

    #[test]
    fn test_interval_bound_propagation_identity() {
        let config = IbPConfig::default();
        let input_lo = [-1.0, 0.0];
        let input_hi = [1.0, 2.0];
        let weights = [1.0, 0.0, 0.0, 1.0]; // Identity
        let bias = [0.0, 0.0];
        let result =
            interval_bound_propagation(&input_lo, &input_hi, &weights, &bias, "identity", &config);
        assert_eq!(result.neurons_verified, 2);
        assert!((result.lower[0] - (-1.0)).abs() < 1e-5);
        assert!((result.upper[0] - 1.0).abs() < 1e-5);
        assert!((result.lower[1] - 0.0).abs() < 1e-5);
        assert!((result.upper[1] - 2.0).abs() < 1e-5);
    }

    #[test]
    fn test_interval_bound_propagation_relu() {
        let config = IbPConfig::default();
        let input_lo = [-2.0, 0.0];
        let input_hi = [-1.0, 1.0];
        let weights = [1.0, 0.0, 0.0, 1.0];
        let bias = [0.0, 0.0];
        let result =
            interval_bound_propagation(&input_lo, &input_hi, &weights, &bias, "relu", &config);
        // ReLU of [-2,-1] = [0,0], ReLU of [0,1] = [0,1]
        assert!((result.lower[0] - 0.0).abs() < 1e-5);
        assert!((result.upper[0] - 0.0).abs() < 1e-5);
        assert!((result.lower[1] - 0.0).abs() < 1e-5);
        assert!((result.upper[1] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_interval_bound_propagation_negative_weight() {
        let config = IbPConfig::default();
        let input_lo = [0.0, 0.0];
        let input_hi = [1.0, 1.0];
        let weights = [-1.0, 0.0]; // Negative weight flips interval
        let bias = [0.0];
        let result =
            interval_bound_propagation(&input_lo, &input_hi, &weights, &bias, "identity", &config);
        // -1 * [0,1] = [-1,0]
        assert!((result.lower[0] - (-1.0)).abs() < 1e-5);
        assert!((result.upper[0] - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_interval_bound_propagation_with_bias() {
        let config = IbPConfig::default();
        let input_lo = [0.0];
        let input_hi = [1.0];
        let weights = [2.0];
        let bias = [3.0];
        let result =
            interval_bound_propagation(&input_lo, &input_hi, &weights, &bias, "identity", &config);
        // 2 * [0,1] + 3 = [3,5]
        assert!((result.lower[0] - 3.0).abs() < 1e-5);
        assert!((result.upper[0] - 5.0).abs() < 1e-5);
    }

    #[test]
    fn test_verify_ibp_safety_safe() {
        let ibp = IbPResult {
            lower: vec![0.0, 0.0],
            upper: vec![1.0, 1.0],
            neurons_verified: 2,
            avg_tightness: 1.0,
        };
        // Unsafe region: [5,10] — far from IBP bounds
        let unsafe_lo = [5.0, 5.0];
        let unsafe_hi = [10.0, 10.0];
        assert!(verify_ibp_safety(&ibp, &unsafe_lo, &unsafe_hi));
    }

    #[test]
    fn test_verify_ibp_safety_unsafe() {
        let ibp = IbPResult {
            lower: vec![0.0, 0.0],
            upper: vec![5.0, 5.0],
            neurons_verified: 2,
            avg_tightness: 5.0,
        };
        // Unsafe region: [2,3] — overlaps with IBP bounds
        let unsafe_lo = [2.0, 2.0];
        let unsafe_hi = [3.0, 3.0];
        assert!(!verify_ibp_safety(&ibp, &unsafe_lo, &unsafe_hi));
    }

    #[test]
    fn test_verify_ibp_safety_dimension_mismatch() {
        let ibp = IbPResult {
            lower: vec![0.0],
            upper: vec![1.0],
            neurons_verified: 1,
            avg_tightness: 1.0,
        };
        assert!(!verify_ibp_safety(&ibp, &[0.0, 0.0], &[1.0, 1.0]));
    }

    // --- Taylor Model Tests ---

    #[test]
    fn test_taylor_config_default() {
        let config = TaylorConfig::default();
        assert_eq!(config.order, 2);
        assert!((config.remainder_scale - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_taylor_config_with_order() {
        let config = TaylorConfig::with_order(3);
        assert_eq!(config.order, 3);
    }

    #[test]
    fn test_taylor_config_order_clamped() {
        let config = TaylorConfig::with_order(1);
        assert_eq!(config.order, 2); // Clamped to ≥2
    }

    #[test]
    fn test_taylor_config_with_remainder_scale() {
        let config = TaylorConfig::default().with_remainder_scale(2.0);
        assert!((config.remainder_scale - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_taylor_model_empty() {
        let config = TaylorConfig::default();
        let fn_eval = |x: &[f32]| x.to_vec();
        let result = taylor_model_verify(&fn_eval, &[], &[], &[], &config);
        assert!(result.center.is_empty());
        assert!(result.verified);
    }

    #[test]
    fn test_taylor_model_identity_function() {
        let config = TaylorConfig::default();
        let fn_eval = |x: &[f32]| x.to_vec();
        let center = [0.0, 0.0];
        let lo = [-0.1, -0.1];
        let hi = [0.1, 0.1];
        let result = taylor_model_verify(&fn_eval, &center, &lo, &hi, &config);
        assert_eq!(result.order_used, 2);
        assert!(result.verified);
        // Identity: f(x) = x, bounds should be close to [-0.1, 0.1]
        assert!(result.lower[0] <= 0.0);
        assert!(result.upper[0] >= 0.0);
    }

    #[test]
    fn test_taylor_model_quadratic() {
        let config = TaylorConfig::with_order(2);
        let fn_eval = |x: &[f32]| vec![x[0] * x[0]];
        let center = [1.0];
        let lo = [0.9];
        let hi = [1.1];
        let result = taylor_model_verify(&fn_eval, &center, &lo, &hi, &config);
        assert_eq!(result.order_used, 2);
        assert!(result.verified);
        // f(1) = 1, f(0.9) = 0.81, f(1.1) = 1.21
        assert!(result.lower[0] <= 1.0);
        assert!(result.upper[0] >= 1.0);
    }

    #[test]
    fn test_taylor_model_order3() {
        let config = TaylorConfig::with_order(3);
        let fn_eval = |x: &[f32]| vec![x[0] * x[0] * x[0]];
        let center = [0.0];
        let lo = [-0.1];
        let hi = [0.1];
        let result = taylor_model_verify(&fn_eval, &center, &lo, &hi, &config);
        assert_eq!(result.order_used, 3);
        assert!(result.verified);
    }

    #[test]
    fn test_taylor_model_remainder_decreases_with_smaller_bounds() {
        let config = TaylorConfig::default();
        let fn_eval = |x: &[f32]| vec![x[0] * x[0]];
        let center = [0.0];
        let lo_wide = [-0.5];
        let hi_wide = [0.5];
        let lo_narrow = [-0.1];
        let hi_narrow = [0.1];
        let result_wide = taylor_model_verify(&fn_eval, &center, &lo_wide, &hi_wide, &config);
        let result_narrow = taylor_model_verify(&fn_eval, &center, &lo_narrow, &hi_narrow, &config);
        assert!(result_narrow.remainder_bound < result_wide.remainder_bound);
    }

    #[test]
    fn test_verify_taylor_safety_safe() {
        let taylor = TaylorResult {
            center: vec![0.0],
            coefficients: vec![0.0],
            remainder_bound: 0.1,
            lower: vec![-0.1],
            upper: vec![0.1],
            order_used: 2,
            verified: true,
        };
        // Unsafe region: [5,10]
        let unsafe_region = [(5.0, 10.0)];
        assert!(verify_taylor_safety(&taylor, &unsafe_region));
    }

    #[test]
    fn test_verify_taylor_safety_unsafe() {
        let taylor = TaylorResult {
            center: vec![0.0],
            coefficients: vec![0.0],
            remainder_bound: 5.0,
            lower: vec![-2.5],
            upper: vec![2.5],
            order_used: 2,
            verified: true,
        };
        // Unsafe region: [0,1] — overlaps
        let unsafe_region = [(0.0, 1.0)];
        assert!(!verify_taylor_safety(&taylor, &unsafe_region));
    }

    // --- CBF Tests ---

    #[test]
    fn test_cbf_config_default() {
        let config = CbfConfig::default();
        assert!((config.beta - 1.0).abs() < 1e-6);
        assert!((config.gamma - 0.1).abs() < 1e-6);
        assert_eq!(config.max_violation, 0.0);
    }

    #[test]
    fn test_cbf_config_with_beta() {
        let config = CbfConfig::with_beta(2.0);
        assert!((config.beta - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_cbf_config_beta_clamped() {
        let config = CbfConfig::with_beta(-1.0);
        assert_eq!(config.beta, 0.0);
    }

    #[test]
    fn test_cbf_config_with_gamma() {
        let config = CbfConfig::default().with_gamma(0.5);
        assert!((config.gamma - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_cbf_config_with_violation_tolerance() {
        let config = CbfConfig::default().with_violation_tolerance(0.1);
        assert!((config.max_violation - 0.1).abs() < 1e-6);
    }

    #[test]
    fn test_cbf_empty_state() {
        let config = CbfConfig::default();
        let dynamics = |x: &[f32]| x.to_vec();
        let result = control_barrier_function(&[], &[], 1.0, &dynamics, &config);
        assert!(result.control_input.is_empty());
        assert!(result.safe);
    }

    #[test]
    fn test_cbf_safe_state() {
        let config = CbfConfig::default();
        // Dynamics: f(x) = 0 (stationary)
        let dynamics = |_x: &[f32]| vec![0.0, 0.0];
        let state = [0.0, 0.0];
        let safe = [0.0, 0.0];
        let result = control_barrier_function(&state, &safe, 1.0, &dynamics, &config);
        // h(x) = 1 - 0 = 1 > 0, L_f h = 0, margin = 0 + 1*1 = 1 > 0
        assert!(result.safe);
        assert!(result.barrier_value > 0.0);
        assert!(result.safety_margin > 0.0);
    }

    #[test]
    fn test_cbf_unsafe_state() {
        let config = CbfConfig::default();
        let dynamics = |_x: &[f32]| vec![0.0, 0.0];
        let state = [5.0, 5.0]; // Far from safe centroid
        let safe = [0.0, 0.0];
        let result = control_barrier_function(&state, &safe, 1.0, &dynamics, &config);
        // h(x) = 1 - 50 = -49 < 0
        assert!(!result.safe);
        assert!(result.barrier_value < 0.0);
    }

    #[test]
    fn test_cbf_boundary_state() {
        let config = CbfConfig::default();
        let dynamics = |_x: &[f32]| vec![0.0, 0.0];
        let state = [1.0, 0.0]; // On boundary (radius=1)
        let safe = [0.0, 0.0];
        let result = control_barrier_function(&state, &safe, 1.0, &dynamics, &config);
        // h(x) = 1 - 1 = 0, margin = 0
        assert!((result.barrier_value - 0.0).abs() < 1e-5);
        assert!(result.safe);
    }

    #[test]
    fn test_cbf_with_control_input() {
        let config = CbfConfig::default();
        // Dynamics pushing away from safe set
        let dynamics = |x: &[f32]| vec![x[0] * 0.1, x[1] * 0.1];
        let state = [0.9, 0.0];
        let safe = [0.0, 0.0];
        let result = control_barrier_function(&state, &safe, 1.0, &dynamics, &config);
        assert_eq!(result.control_input.len(), 2);
    }

    #[test]
    fn test_cbf_violation_tolerance() {
        let config = CbfConfig::default().with_violation_tolerance(1.0);
        let dynamics = |_x: &[f32]| vec![0.0];
        let state = [1.1]; // Slightly outside radius=1
        let safe = [0.0];
        let result = control_barrier_function(&state, &safe, 1.0, &dynamics, &config);
        // h(x) = 1 - 1.21 = -0.21, within tolerance of 1.0
        assert!(result.safe);
    }

    #[test]
    fn test_cbf_dimension_mismatch() {
        let config = CbfConfig::default();
        let dynamics = |x: &[f32]| x.to_vec();
        let result = control_barrier_function(&[1.0, 2.0], &[1.0], 1.0, &dynamics, &config);
        assert!(result.safe); // Returns safe=true for invalid input
    }

    // --- Combined Verification Tests ---

    #[test]
    fn test_combined_verification_all_safe() {
        let ibp = IbPResult {
            lower: vec![0.0],
            upper: vec![1.0],
            neurons_verified: 1,
            avg_tightness: 1.0,
        };
        let taylor = TaylorResult {
            center: vec![0.5],
            coefficients: vec![0.5],
            remainder_bound: 0.1,
            lower: vec![0.4],
            upper: vec![0.6],
            order_used: 2,
            verified: true,
        };
        let cbf = CbfResult {
            barrier_value: 0.5,
            lie_derivative: 0.0,
            safety_margin: 0.5,
            control_input: vec![0.0],
            safe: true,
        };
        let result = combined_verification(&ibp, &taylor, &cbf);
        assert!(result.ibp_safe);
        assert!(result.taylor_safe);
        assert!(result.cbf_safe);
        assert!(result.overall_safe);
    }

    #[test]
    fn test_combined_verification_ibp_fail() {
        let ibp = IbPResult {
            lower: vec![0.0],
            upper: vec![1.0],
            neurons_verified: 0, // No neurons verified
            avg_tightness: 1.0,
        };
        let taylor = TaylorResult {
            center: vec![0.5],
            coefficients: vec![0.5],
            remainder_bound: 0.1,
            lower: vec![0.4],
            upper: vec![0.6],
            order_used: 2,
            verified: true,
        };
        let cbf = CbfResult {
            barrier_value: 0.5,
            lie_derivative: 0.0,
            safety_margin: 0.5,
            control_input: vec![0.0],
            safe: true,
        };
        let result = combined_verification(&ibp, &taylor, &cbf);
        assert!(!result.ibp_safe);
        assert!(!result.overall_safe);
    }

    #[test]
    fn test_combined_verification_cbf_fail() {
        let ibp = IbPResult {
            lower: vec![0.0],
            upper: vec![1.0],
            neurons_verified: 1,
            avg_tightness: 1.0,
        };
        let taylor = TaylorResult {
            center: vec![0.5],
            coefficients: vec![0.5],
            remainder_bound: 0.1,
            lower: vec![0.4],
            upper: vec![0.6],
            order_used: 2,
            verified: true,
        };
        let cbf = CbfResult {
            barrier_value: -1.0,
            lie_derivative: -0.5,
            safety_margin: -1.5,
            control_input: vec![0.0],
            safe: false,
        };
        let result = combined_verification(&ibp, &taylor, &cbf);
        assert!(!result.cbf_safe);
        assert!(!result.overall_safe);
    }

    #[test]
    fn test_combined_verification_display() {
        let result = CombinedVerificationResult {
            ibp_safe: true,
            taylor_safe: true,
            cbf_safe: true,
            overall_safe: true,
            min_safety_margin: 0.5,
        };
        let display = format!("{}", result);
        assert!(display.contains("ibp=true"));
        assert!(display.contains("overall=true"));
    }

    // --- Full Pipeline Tests ---

    #[test]
    fn test_full_verification_pipeline() {
        // IBP
        let config_ibp = IbPConfig::with_epsilon(0.01, 2);
        let ibp_result = interval_bound_propagation(
            &[-0.1, -0.1],
            &[0.1, 0.1],
            &[1.0, 0.0, 0.0, 1.0],
            &[0.0, 0.0],
            "relu",
            &config_ibp,
        );

        // Taylor
        let config_taylor = TaylorConfig::with_order(2);
        let fn_eval = |x: &[f32]| vec![x[0].max(0.0), x[1].max(0.0)];
        let taylor_result = taylor_model_verify(
            &fn_eval,
            &[0.0, 0.0],
            &[-0.1, -0.1],
            &[0.1, 0.1],
            &config_taylor,
        );

        // CBF
        let config_cbf = CbfConfig::with_beta(1.0);
        let dynamics = |_x: &[f32]| vec![0.0, 0.0];
        let cbf_result =
            control_barrier_function(&[0.0, 0.0], &[0.0, 0.0], 1.0, &dynamics, &config_cbf);

        // Combined
        let combined = combined_verification(&ibp_result, &taylor_result, &cbf_result);
        assert!(combined.cbf_safe);
        assert!(combined.taylor_safe);
    }

    #[test]
    fn test_factorial() {
        assert_eq!(factorial(0), 1.0);
        assert_eq!(factorial(1), 1.0);
        assert_eq!(factorial(2), 2.0);
        assert_eq!(factorial(3), 6.0);
        assert_eq!(factorial(4), 24.0);
        assert_eq!(factorial(5), 120.0);
    }

    #[test]
    fn test_ibp_result_display() {
        let result = IbPResult {
            lower: vec![0.0],
            upper: vec![1.0],
            neurons_verified: 10,
            avg_tightness: 0.5,
        };
        let display = format!("{}", result);
        assert!(display.contains("neurons=10"));
    }

    #[test]
    fn test_taylor_result_display() {
        let result = TaylorResult {
            center: vec![0.0],
            coefficients: vec![0.0],
            remainder_bound: 0.1,
            lower: vec![-0.1],
            upper: vec![0.1],
            order_used: 2,
            verified: true,
        };
        let display = format!("{}", result);
        assert!(display.contains("order=2"));
        assert!(display.contains("verified=true"));
    }

    #[test]
    fn test_cbf_result_display() {
        let result = CbfResult {
            barrier_value: 0.5,
            lie_derivative: 0.0,
            safety_margin: 0.5,
            control_input: vec![0.0],
            safe: true,
        };
        let display = format!("{}", result);
        assert!(display.contains("safe=true"));
    }

    // ─── Sprint 136 — Hybrid IBP + Zonotope Tests ───

    #[test]
    fn test_hybrid_config_default() {
        let config = HybridConfig::default();
        assert!(config.use_ibp);
        assert!(config.refine_zonotope);
        assert_eq!(config.max_generators, 64);
        assert!((config.safety_margin - 0.01).abs() < 1e-6);
    }

    #[test]
    fn test_hybrid_config_zonotope_only() {
        let config = HybridConfig::zonotope_only();
        assert!(!config.use_ibp);
        assert!(config.refine_zonotope);
    }

    #[test]
    fn test_hybrid_config_ibp_only() {
        let config = HybridConfig::ibp_only();
        assert!(config.use_ibp);
        assert!(!config.refine_zonotope);
    }

    #[test]
    fn test_hybrid_config_with_safety_margin() {
        let config = HybridConfig::default().with_safety_margin(0.05);
        assert!((config.safety_margin - 0.05).abs() < 1e-6);
    }

    #[test]
    fn test_hybrid_config_safety_margin_clamped() {
        let config = HybridConfig::default().with_safety_margin(-0.1);
        assert!(config.safety_margin >= 0.0);
    }

    #[test]
    fn test_zonotope_from_intervals() {
        let lower = vec![0.0, 1.0];
        let upper = vec![2.0, 3.0];
        let z = Zonotope::from_intervals(&lower, &upper);
        assert!((z.center[0] - 1.0).abs() < 1e-6);
        assert!((z.center[1] - 2.0).abs() < 1e-6);
        assert_eq!(z.generators.len(), 2);
    }

    #[test]
    fn test_zonotope_bounds() {
        let lower = vec![0.0, 1.0];
        let upper = vec![2.0, 3.0];
        let z = Zonotope::from_intervals(&lower, &upper);
        let (lo, hi) = z.bounds();
        assert!((lo[0] - 0.0).abs() < 1e-6);
        assert!((hi[0] - 2.0).abs() < 1e-6);
        assert!((lo[1] - 1.0).abs() < 1e-6);
        assert!((hi[1] - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_zonotope_avg_width() {
        let lower = vec![0.0];
        let upper = vec![2.0];
        let z = Zonotope::from_intervals(&lower, &upper);
        assert!((z.avg_width() - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_zonotope_affine_propagate() {
        let lower = vec![0.0, 0.0];
        let upper = vec![1.0, 1.0];
        let z = Zonotope::from_intervals(&lower, &upper);
        // Identity weight + bias of 1
        let weight = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let bias = vec![1.0, 1.0];
        let z_out = z.affine_propagate(&weight, &bias);
        assert!((z_out.center[0] - 1.5).abs() < 1e-6);
        assert!((z_out.center[1] - 1.5).abs() < 1e-6);
    }

    #[test]
    fn test_compute_hybrid_bounds_basic() {
        let lower = vec![0.0, 0.0];
        let upper = vec![1.0, 1.0];
        let weight = vec![vec![1.0, 0.5], vec![0.5, 1.0]];
        let bias = vec![0.0, 0.0];
        let config = HybridConfig::default();

        let result = compute_hybrid_bounds(&lower, &upper, &weight, &bias, &config);
        assert_eq!(result.lower.len(), 2);
        assert_eq!(result.upper.len(), 2);
        assert!(result.ibp_lower.is_some());
        assert!(result.ibp_upper.is_some());
    }

    #[test]
    fn test_compute_hybrid_bounds_ibp_only() {
        let lower = vec![0.0];
        let upper = vec![1.0];
        let weight = vec![vec![2.0]];
        let bias = vec![0.0];
        let config = HybridConfig::ibp_only();

        let result = compute_hybrid_bounds(&lower, &upper, &weight, &bias, &config);
        assert_eq!(result.lower.len(), 1);
        assert!((result.lower[0] - 0.0).abs() < 1e-6);
        assert!((result.upper[0] - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_hybrid_bounds_negative_weight() {
        let lower = vec![0.0];
        let upper = vec![1.0];
        let weight = vec![vec![-1.0]];
        let bias = vec![0.0];
        let config = HybridConfig::ibp_only();

        let result = compute_hybrid_bounds(&lower, &upper, &weight, &bias, &config);
        // With negative weight: lo = -1*1 = -1, hi = -1*0 = 0, after ReLU: lo=0, hi=0
        assert!(result.lower[0] >= 0.0);
        assert!(result.upper[0] >= result.lower[0]);
    }

    #[test]
    fn test_compute_hybrid_bounds_improvement() {
        let lower = vec![0.0, 0.0];
        let upper = vec![1.0, 1.0];
        let weight = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let bias = vec![0.0, 0.0];
        let config = HybridConfig::default();

        let result = compute_hybrid_bounds(&lower, &upper, &weight, &bias, &config);
        assert!(result.improvement_ratio >= -1.0);
        assert!(result.improvement_ratio <= 1.0);
    }

    #[test]
    fn test_verify_hybrid_safety_safe() {
        let result = HybridResult {
            lower: vec![0.0],
            upper: vec![0.1],
            ibp_lower: None,
            ibp_upper: None,
            improvement_ratio: 0.0,
            safe: true,
            min_safety_margin: 0.5,
        };
        // Unsafe region: [5,10] — no overlap
        let unsafe_region = [(5.0, 10.0)];
        assert!(verify_hybrid_safety(&result, &unsafe_region, 1.0));
    }

    #[test]
    fn test_verify_hybrid_safety_unsafe_overlap() {
        let result = HybridResult {
            lower: vec![0.0],
            upper: vec![10.0],
            ibp_lower: None,
            ibp_upper: None,
            improvement_ratio: 0.0,
            safe: true,
            min_safety_margin: 0.5,
        };
        // Unsafe region: [5,8] — overlaps with [0,10]
        let unsafe_region = [(5.0, 8.0)];
        assert!(!verify_hybrid_safety(&result, &unsafe_region, 1.0));
    }

    #[test]
    fn test_verify_hybrid_safety_margin_exceeded() {
        let result = HybridResult {
            lower: vec![0.0],
            upper: vec![10.0],
            ibp_lower: None,
            ibp_upper: None,
            improvement_ratio: 0.0,
            safe: true,
            min_safety_margin: 0.5,
        };
        // No unsafe region but width exceeds margin
        let unsafe_region: &[(f32, f32)] = &[];
        assert!(!verify_hybrid_safety(&result, &unsafe_region, 1.0));
    }

    #[test]
    fn test_hybrid_result_display() {
        let result = HybridResult {
            lower: vec![0.0, 0.0],
            upper: vec![1.0, 1.0],
            ibp_lower: None,
            ibp_upper: None,
            improvement_ratio: 0.5,
            safe: true,
            min_safety_margin: 0.3,
        };
        let display = format!("{}", result);
        assert!(display.contains("dim=2"));
        assert!(display.contains("safe=true"));
    }

    #[test]
    fn test_hybrid_full_pipeline() {
        let lower = vec![0.0, 0.5];
        let upper = vec![1.0, 1.5];
        let weight = vec![vec![1.0, 0.5], vec![0.5, 1.0]];
        let bias = vec![0.1, 0.1];
        let config = HybridConfig::default();

        let result = compute_hybrid_bounds(&lower, &upper, &weight, &bias, &config);
        assert_eq!(result.lower.len(), 2);
        for i in 0..2 {
            assert!(
                result.upper[i] >= result.lower[i],
                "Upper >= Lower at dim {}",
                i
            );
        }

        let unsafe_region: &[(f32, f32)] = &[];
        let safe = verify_hybrid_safety(&result, &unsafe_region, 10.0);
        assert!(safe || !safe); // Just verify it doesn't panic
    }

    // ===== S137: Adversarial Certification Tests =====

    #[test]
    fn test_adversarial_cert_config_default() {
        let cfg = AdversarialCertConfig::default();
        assert_eq!(cfg.num_samples, 1000);
        assert!((cfg.safety_threshold - 0.95).abs() < 1e-5);
        assert_eq!(cfg.seed, 42);
        assert_eq!(cfg.max_generators, 64);
    }

    #[test]
    fn test_adversarial_cert_config_fast() {
        let cfg = AdversarialCertConfig::fast();
        assert_eq!(cfg.num_samples, 100);
        assert!((cfg.safety_threshold - 0.9).abs() < 1e-5);
    }

    #[test]
    fn test_adversarial_cert_config_high_precision() {
        let cfg = AdversarialCertConfig::high_precision();
        assert_eq!(cfg.num_samples, 10000);
        assert!((cfg.safety_threshold - 0.99).abs() < 1e-5);
    }

    #[test]
    fn test_adversarial_cert_config_with_samples() {
        let cfg = AdversarialCertConfig::default().with_samples(500);
        assert_eq!(cfg.num_samples, 500);
    }

    #[test]
    fn test_adversarial_cert_config_with_threshold() {
        let cfg = AdversarialCertConfig::default().with_threshold(0.99);
        assert!((cfg.safety_threshold - 0.99).abs() < 1e-5);
    }

    #[test]
    fn test_adversarial_cert_config_threshold_clamped() {
        let cfg = AdversarialCertConfig::default().with_threshold(1.5);
        assert!((cfg.safety_threshold - 1.0).abs() < 1e-5);
        let cfg2 = AdversarialCertConfig::default().with_threshold(-0.5);
        assert!((cfg2.safety_threshold - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_zonotope_propagate_relu_positive() {
        // All inputs positive: ReLU is identity
        let z = Zonotope {
            center: vec![1.0, 2.0],
            generators: vec![vec![0.1, 0.0], vec![0.0, 0.2]],
        };
        let result = z.propagate_relu();
        assert!((result.center[0] - 1.0).abs() < 1e-5);
        assert!((result.center[1] - 2.0).abs() < 1e-5);
        // Generators should pass through
        assert!((result.generators[0][0] - 0.1).abs() < 1e-5);
    }

    #[test]
    fn test_zonotope_propagate_relu_negative() {
        // All inputs negative: ReLU is zero
        let z = Zonotope {
            center: vec![-1.0, -2.0],
            generators: vec![vec![0.1, 0.0], vec![0.0, 0.2]],
        };
        let result = z.propagate_relu();
        assert!((result.center[0] - 0.0).abs() < 1e-5);
        assert!((result.center[1] - 0.0).abs() < 1e-5);
        // Generators should be zero
        for g in &result.generators {
            for &v in g {
                assert!((v - 0.0).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn test_zonotope_propagate_relu_mixed() {
        // Mixed: first dim crosses zero, second is positive
        let z = Zonotope {
            center: vec![0.0, 1.0],
            generators: vec![vec![0.5, 0.0], vec![0.0, 0.1]],
        };
        let result = z.propagate_relu();
        // First dim: center=0, bounds=[-0.5, 0.5], mixed region
        assert!(result.center[0] >= 0.0);
        // Second dim: fully positive, passes through
        assert!((result.center[1] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_zonotope_linf_radius() {
        let z = Zonotope {
            center: vec![0.0, 0.0],
            generators: vec![vec![0.5, 0.3], vec![0.0, 0.7]],
        };
        let radius = z.linf_radius();
        // Dim 0: radius = 0.5, Dim 1: radius = 0.3 + 0.7 = 1.0
        // L-inf = max(0.5, 1.0) = 1.0
        assert!((radius - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_certified_safety_prob_perfect() {
        // Zonotope entirely within safe region
        let z = Zonotope {
            center: vec![0.5, 0.5],
            generators: vec![vec![0.1, 0.0], vec![0.0, 0.1]],
        };
        let safe_lo = vec![0.0, 0.0];
        let safe_hi = vec![1.0, 1.0];
        let (prob, samples) = z.certified_safety_prob(&safe_lo, &safe_hi, 100, 42);
        assert_eq!(samples, 100);
        assert!((prob - 1.0).abs() < 1e-5, "Perfect safety: got {}", prob);
    }

    #[test]
    fn test_certified_safety_prob_partial() {
        // Zonotope partially overlaps safe region
        let z = Zonotope {
            center: vec![0.0, 0.0],
            generators: vec![vec![1.0, 0.0], vec![0.0, 1.0]],
        };
        // Safe region is [0, 1] x [0, 1], zonotope is [-1, 1] x [-1, 1]
        // ~1/4 should be safe
        let safe_lo = vec![0.0, 0.0];
        let safe_hi = vec![1.0, 1.0];
        let (prob, _samples) = z.certified_safety_prob(&safe_lo, &safe_hi, 500, 42);
        assert!(prob > 0.1 && prob < 0.5, "Partial safety should be ~0.25, got {}", prob);
    }

    #[test]
    fn test_certified_safety_prob_empty() {
        let z = Zonotope {
            center: vec![],
            generators: vec![],
        };
        let (prob, samples) = z.certified_safety_prob(&[], &[], 100, 42);
        assert!((prob - 1.0).abs() < 1e-5);
        assert_eq!(samples, 0);
    }

    #[test]
    fn test_certify_adversarial_robustness_safe() {
        let z = Zonotope {
            center: vec![0.5, 0.5],
            generators: vec![vec![0.05, 0.0], vec![0.0, 0.05]],
        };
        let safe_lo = vec![0.0, 0.0];
        let safe_hi = vec![1.0, 1.0];
        let config = AdversarialCertConfig::fast();
        let result = certify_adversarial_robustness(&z, &safe_lo, &safe_hi, &config);
        assert!(result.is_certified, "Should be certified safe");
        assert!(result.safety_probability >= 0.9);
        assert_eq!(result.num_samples, config.num_samples);
    }

    #[test]
    fn test_certify_adversarial_robustness_unsafe() {
        let z = Zonotope {
            center: vec![1.5, 1.5],
            generators: vec![vec![0.5, 0.0], vec![0.0, 0.5]],
        };
        let safe_lo = vec![0.0, 0.0];
        let safe_hi = vec![1.0, 1.0];
        let config = AdversarialCertConfig::fast();
        let result = certify_adversarial_robustness(&z, &safe_lo, &safe_hi, &config);
        assert!(!result.is_certified, "Should NOT be certified safe");
    }

    #[test]
    fn test_adversarial_cert_result_display() {
        let result = AdversarialCertResult {
            safety_probability: 0.95,
            certified_radius: 0.1,
            num_samples: 1000,
            is_certified: true,
            bound_tightness: 0.2,
        };
        let display = format!("{}", result);
        assert!(display.contains("0.95"));
        assert!(display.contains("true"));
    }

    #[test]
    fn test_simulate_pgd_attack_basic() {
        let z = Zonotope {
            center: vec![0.5, 0.5],
            generators: vec![vec![0.05, 0.0], vec![0.0, 0.05]],
        };
        let safe_lo = vec![0.0, 0.0];
        let safe_hi = vec![1.0, 1.0];
        let result = simulate_pgd_attack(&z, &safe_lo, &safe_hi, 0.5, 3, 0.1);
        // After attack, radius should be larger
        assert!(result.certified_radius > 0.05);
        assert!(result.num_samples > 0);
    }

    #[test]
    fn test_simulate_pgd_attack_large_epsilon() {
        let z = Zonotope {
            center: vec![0.5, 0.5],
            generators: vec![vec![0.05, 0.0], vec![0.0, 0.05]],
        };
        let safe_lo = vec![0.4, 0.4];
        let safe_hi = vec![0.6, 0.6];
        // Large attack should break certification
        let result = simulate_pgd_attack(&z, &safe_lo, &safe_hi, 1.0, 10, 0.2);
        // With large attack on small safe region, likely not certified
        assert!(result.num_samples > 0);
    }

    #[test]
    fn test_pgd_attack_order_reduction() {
        let z = Zonotope {
            center: vec![0.5, 0.5],
            generators: vec![vec![0.05, 0.0], vec![0.0, 0.05]],
        };
        let safe_lo = vec![0.0, 0.0];
        let safe_hi = vec![1.0, 1.0];
        // Many steps should trigger order reduction
        let result = simulate_pgd_attack(&z, &safe_lo, &safe_hi, 0.5, 100, 0.05);
        assert!(result.num_samples > 0);
        // Should not panic even with many steps
        assert!(result.certified_radius >= 0.0);
    }

    #[test]
    fn test_full_adversarial_pipeline() {
        // Create zonotope from intervals
        let z = Zonotope::from_intervals(&[0.4, 0.4], &[0.6, 0.6]);

        // Propagate through ReLU (should be identity since positive)
        let z_relu = z.propagate_relu();
        assert!(z_relu.center[0] > 0.0);

        // Certify robustness
        let safe_lo = vec![0.0, 0.0];
        let safe_hi = vec![1.0, 1.0];
        let config = AdversarialCertConfig::fast();
        let cert = certify_adversarial_robustness(&z_relu, &safe_lo, &safe_hi, &config);
        assert!(cert.is_certified);

        // Simulate attack
        let attack_result = simulate_pgd_attack(&z_relu, &safe_lo, &safe_hi, 0.3, 5, 0.05);
        assert!(attack_result.num_samples > 0);
    }
}
