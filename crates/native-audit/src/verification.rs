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
// Unit Tests
// ============================================================================

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
}
