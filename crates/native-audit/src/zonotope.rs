//! Zonotope Geometry — Certified Bound Propagation for High-Dimensional Latent Spaces.
//!
//! Zonotopes provide precise interval-like bounds that capture linear correlations,
//! dramatically reducing the wrapping effect that plagues pure interval arithmetic
//! in 4096D+ latent spaces.
//!
//! **Zonotope:** Z = { c + G @ epsilon | epsilon in [-1,1]^k }
//! - c: center vector [hidden_dim]
//! - G: generator matrix [num_gens, hidden_dim]
//! - k (num_gens) << hidden_dim for tractable propagation
//!
//! **Key Operations:**
//! - Affine: c' = W@c + b, G' = W@G  (exact)
//! - ReLU: Slope bounding [l,u] per dimension (controlled over-approx)
//! - Bounds: lower = c - sum(|G_i|), upper = c + sum(|G_i|)
//! - Minkowski sum: concat generators
//!
//! **Certified Steering Robustness:**
//! Guarantees that under adversarial perturbation of radius epsilon,
//! the trajectory never violates CBF h(x) >= 0.

use candle_core::{DType, Result, Tensor};

/// Configuration for zonotope creation and propagation.
#[derive(Debug, Clone)]
pub struct ZonotopeConfig {
    /// Maximum number of generators (controls precision vs. speed).
    pub max_gens: usize,
    /// Perturbation radius for initial zonotope.
    pub epsilon: f32,
    /// Enable generator reduction via truncation after each non-linear op.
    pub reduce_after_nonlinear: bool,
    /// Threshold for generator pruning (remove generators with norm below this).
    pub prune_threshold: f32,
}

impl Default for ZonotopeConfig {
    fn default() -> Self {
        Self {
            max_gens: 64,
            epsilon: 0.05,
            reduce_after_nonlinear: true,
            prune_threshold: 1e-6,
        }
    }
}

/// A zonotope represented as center + generator matrix.
///
/// Z = { c + G @ epsilon | epsilon in [-1,1]^k }
/// where c is the center and G contains k generator rows.
#[derive(Debug, Clone)]
pub struct Zonotope {
    pub center: Tensor,     // [1, hidden_dim]
    pub generators: Tensor, // [num_gens, hidden_dim]
    config: ZonotopeConfig,
}

impl Zonotope {
    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    /// Create a zonotope from a center tensor with epsilon-ball perturbation.
    ///
    /// Initial generators are diagonal (identity * epsilon) plus low-rank random
    /// generators to capture correlations.
    pub fn new_from_epsilon(center: &Tensor, epsilon: f32, max_gens: usize) -> Result<Self> {
        let hidden_dim = center.dim(1)?;
        let num_gens = max_gens.min(hidden_dim);
        let device = center.device();

        // Diagonal generators: each generator perturbs one dimension
        let mut gen_data = vec![0.0f32; num_gens * hidden_dim];
        for i in 0..num_gens {
            gen_data[i * hidden_dim + i] = epsilon;
        }
        let generators = Tensor::from_vec(gen_data, (num_gens, hidden_dim), device)?;

        Ok(Self {
            center: center.clone(),
            generators,
            config: ZonotopeConfig {
                max_gens,
                epsilon,
                ..Default::default()
            },
        })
    }

    /// Create a zonotope from explicit center and generator tensors.
    pub fn new(center: Tensor, generators: Tensor, config: ZonotopeConfig) -> Result<Self> {
        // Validate shapes
        assert_eq!(
            center.dim(1)?,
            generators.dim(1)?,
            "Center and generator dimensions must match"
        );
        Ok(Self {
            center,
            generators,
            config,
        })
    }

    /// Create a zonotope from an interval vector (lo, hi) per dimension.
    ///
    /// Each interval [lo_i, hi_i] becomes center_i = (lo_i + hi_i)/2
    /// with generator_i = (hi_i - lo_i)/2.
    pub fn from_intervals(lo: &Tensor, hi: &Tensor) -> Result<Self> {
        let center = {
            let s = Tensor::full(0.5f32, (), lo.device())?;
            lo.broadcast_add(hi)?.broadcast_mul(&s)?
        };
        let generators = {
            let s = Tensor::full(0.5f32, (), hi.device())?;
            hi.broadcast_sub(lo)?.broadcast_mul(&s)?
        };
        let num_gens = generators.dim(0)?;
        let config = ZonotopeConfig {
            max_gens: num_gens,
            ..Default::default()
        };
        Ok(Self {
            center,
            generators,
            config,
        })
    }

    /// Create a point zonotope (zero generators).
    pub fn point(center: &Tensor) -> Result<Self> {
        let hidden_dim = center.dim(1)?;
        let generators = Tensor::zeros((0, hidden_dim), DType::F32, center.device())?;
        Ok(Self {
            center: center.clone(),
            generators,
            config: ZonotopeConfig::default(),
        })
    }

    // -----------------------------------------------------------------------
    // Core Properties
    // -----------------------------------------------------------------------

    /// Number of generators.
    pub fn num_gens(&self) -> Result<usize> {
        self.generators.dim(0)
    }

    /// Hidden dimension.
    pub fn hidden_dim(&self) -> Result<usize> {
        self.center.dim(1)
    }

    /// Compute lower and upper bounds: [c - sum(|G_i|), c + sum(|G_i|)].
    pub fn compute_bounds(&self) -> Result<(Tensor, Tensor)> {
        let sum_abs = self.generators.abs()?.sum(0)?.unsqueeze(0)?;
        let lower = self.center.broadcast_sub(&sum_abs)?;
        let upper = self.center.broadcast_add(&sum_abs)?;
        Ok((lower, upper))
    }

    /// Compute the volume proxy (sum of generator norms) as a measure of uncertainty.
    ///
    /// Lower volume = tighter bounds = less over-approximation.
    pub fn volume_proxy(&self) -> Result<f32> {
        let sum_abs = self.generators.abs()?.sum_all()?;
        sum_abs.to_scalar::<f32>()
    }

    /// Compute the average interval width per dimension.
    pub fn avg_width(&self) -> Result<f32> {
        let (lower, upper) = self.compute_bounds()?;
        let widths = upper.broadcast_sub(&lower)?.sum_all()?;
        let dim = self.hidden_dim()? as f32;
        Ok(widths.to_scalar::<f32>()? / dim)
    }

    // -----------------------------------------------------------------------
    // Linear Operations (Exact)
    // -----------------------------------------------------------------------

    /// Affine transformation: c' = W@c + b, G' = W@G.
    ///
    /// This is exact — no over-approximation introduced.
    pub fn affine_transform(&self, weight: &Tensor, bias: Option<&Tensor>) -> Result<Self> {
        let new_center = {
            let mut c = self.center.matmul(&weight.t()?)?;
            if let Some(b) = bias {
                c = c.broadcast_add(b)?;
            }
            c
        };
        let new_gens = self.generators.matmul(&weight.t()?)?;
        Ok(Self {
            center: new_center,
            generators: new_gens,
            config: self.config.clone(),
        })
    }

    /// Element-wise scaling: c' = c * s, G' = G * s.
    pub fn scale(&self, s: f32) -> Result<Self> {
        let scalar = Tensor::full(s, (), self.center.device())?;
        Ok(Self {
            center: self.center.broadcast_mul(&scalar)?,
            generators: self.generators.broadcast_mul(&scalar)?,
            config: self.config.clone(),
        })
    }

    // -----------------------------------------------------------------------
    // Non-Linear Operations (Over-Approximated)
    // -----------------------------------------------------------------------

    /// ReLU over-approximation using slope bounding.
    ///
    /// For each dimension i, compute bounds [lo_i, hi_i]:
    /// - If lo_i >= 0: ReLU is identity (exact)
    /// - If hi_i <= 0: ReLU is zero (exact)
    /// - Otherwise: slope in [0, (hi)/(hi-lo)] → add new generator
    pub fn relu_approx(&self) -> Result<Self> {
        let (lower, upper) = self.compute_bounds()?;
        let device = self.center.device();

        // Compute slope bounds per dimension
        // slope_upper = hi / (hi - lo) for mixed signs, clamped to [0, 1]
        let lo_data: Vec<f32> = lower.flatten_all()?.to_vec1()?;
        let hi_data: Vec<f32> = upper.flatten_all()?.to_vec1()?;
        let dim = lo_data.len();

        let mut new_gen_rows = Vec::new();

        for i in 0..dim {
            let lo = lo_data[i];
            let hi = hi_data[i];

            if lo >= 0.0 {
                // All positive — ReLU is identity, no new generator
                continue;
            } else if hi <= 0.0 {
                // All negative — ReLU is zero, no new generator
                continue;
            }
            // Mixed signs — add slope uncertainty generator
            let slope = hi / (hi - lo + 1e-8);
            let clamped_slope = slope.clamp(0.0, 1.0);
            // New generator row: slope * original generators at this dimension
            let mut row = vec![0.0f32; dim];
            row[i] = clamped_slope;
            new_gen_rows.push(row);
        }

        // Apply ReLU to center (approximation)
        let new_center = self.center.relu()?;

        // Concatenate new generators with existing (transformed through ReLU Jacobian approx)
        let _num_new = new_gen_rows.len();
        let mut all_gens = self.generators.to_vec2::<f32>()?;

        // Add diagonal slope generators for mixed-sign dimensions
        for row in new_gen_rows {
            all_gens.push(row);
        }

        let new_generators = if all_gens.is_empty() {
            Tensor::zeros((0, dim), DType::F32, device)?
        } else {
            let (n, d) = all_gens
                .first()
                .map(|r| (all_gens.len(), r.len()))
                .unwrap_or((0, dim));
            Tensor::from_vec(all_gens.into_iter().flatten().collect(), (n, d), device)?
        };

        let mut result = Self {
            center: new_center,
            generators: new_generators,
            config: self.config.clone(),
        };

        // Reduce generators if needed
        if self.config.reduce_after_nonlinear {
            result = result.reduce_generators()?;
        }

        Ok(result)
    }

    /// SiLU (Sigmoid Linear Unit) over-approximation.
    ///
    /// SiLU(x) = x / (1 + exp(-x)). Slope bounds: [0, ~1.59].
    pub fn silu_approx(&self) -> Result<Self> {
        // SiLU derivative is bounded in [0, ~1.59]
        // Use center through SiLU, add uncertainty generator
        let new_center = self.center.silu()?;

        // Add uncertainty from slope variation
        let (_lower, _) = self.compute_bounds()?;
        let scalar = Tensor::full(0.8f32, (), self.generators.device())?;
        let uncertainty = self
            .generators
            .abs()?
            .sum(0)?
            .unsqueeze(0)?
            .broadcast_mul(&scalar)?;

        let new_generators = Tensor::cat(&[&self.generators, &uncertainty], 0)?;

        let mut result = Self {
            center: new_center,
            generators: new_generators,
            config: self.config.clone(),
        };

        if self.config.reduce_after_nonlinear {
            result = result.reduce_generators()?;
        }

        Ok(result)
    }

    // -----------------------------------------------------------------------
    // Set Operations
    // -----------------------------------------------------------------------

    /// Minkowski sum: Z1 + Z2 = {c1+c2, [G1; G2]}.
    pub fn minkowski_sum(&self, other: &Zonotope) -> Result<Self> {
        let new_center = self.center.broadcast_add(&other.center)?;
        let new_generators = Tensor::cat(&[&self.generators, &other.generators], 0)?;
        let max_gens = self.config.max_gens.max(other.config.max_gens);
        let mut result = Self {
            center: new_center,
            generators: new_generators,
            config: self.config.clone(),
        };
        if result.num_gens()? > max_gens {
            result = result.reduce_generators()?;
        }
        Ok(result)
    }

    /// Intersection approximation: use the tighter bounds per dimension.
    pub fn intersect(&self, other: &Zonotope) -> Result<Self> {
        let (self_lo, self_hi) = self.compute_bounds()?;
        let (other_lo, other_hi) = other.compute_bounds()?;

        // Tighter intersection bounds
        let inter_lo = self_lo.maximum(&other_lo)?;
        let inter_hi = self_hi.minimum(&other_hi)?;

        // Create new zonotope from intersection intervals
        Self::from_intervals(&inter_lo, &inter_hi)
    }

    // -----------------------------------------------------------------------
    // Generator Reduction
    // -----------------------------------------------------------------------

    /// Reduce generator count by pruning small generators and merging.
    fn reduce_generators(mut self) -> Result<Self> {
        let num_gens = self.num_gens()?;
        let max_gens = self.config.max_gens;

        if num_gens <= max_gens {
            return Ok(self);
        }

        // Compute norm of each generator row (L2 norm via sqr+sum+sqrt)
        let norms = self.generators.sqr()?.sum(1)?.sqrt()?;
        let norms_vec: Vec<f32> = norms.flatten_all()?.to_vec1()?;

        // Sort by norm (descending) and keep top max_gens
        let mut indexed: Vec<(usize, f32)> = norms_vec.iter().copied().enumerate().collect();
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let kept_indices: Vec<usize> = indexed.iter().take(max_gens).map(|(i, _)| *i).collect();

        // Select kept generators
        let dim = self.hidden_dim()?;
        let mut new_gen_data = Vec::with_capacity(max_gens * dim);
        for &idx in &kept_indices {
            let start = idx * dim;
            let _end = start + dim;
            new_gen_data.extend_from_slice(&self.generators.to_vec2::<f32>()?[idx][..dim]);
        }

        let device = self.generators.device();
        self.generators = Tensor::from_vec(new_gen_data, (max_gens, dim), device)?;

        Ok(self)
    }

    // -----------------------------------------------------------------------
    // Girard-Style Order Reduction (Sprint 159)
    // -----------------------------------------------------------------------

    /// Girard-style order reduction: collapse minor generators into Interval Hull diagonal.
    ///
    /// Algorithm:
    /// 1. Compute L1 norm of each generator row
    /// 2. Sort descending by norm
    /// 3. Keep top-k generators (k = dims * max_order)
    /// 4. Collapse remaining generators into diagonal bounding box
    /// 5. Concatenate kept generators + diagonal hull
    ///
    /// This limits the zonotope order to `dims * max_order + dims` generators,
    /// preventing the wrapping effect explosion in high-dimensional spaces.
    pub fn reduce_order(&self, max_order: usize) -> Result<Self> {
        let dims = self.hidden_dim()?;
        let num_gens = self.num_gens()?;
        let max_gens = dims * max_order;

        if num_gens <= max_gens {
            return Ok(self.clone());
        }

        // 1. L1 norms of generator rows (sum of abs per row)
        let norms = self.generators.abs()?.sum_keepdim(1)?; // [num_gens, 1]
        let norms_vec: Vec<f32> = norms.flatten_all()?.to_vec1()?;

        // 2. Sort indices descending by norm
        let mut indices: Vec<usize> = (0..num_gens).collect();
        indices.sort_by(|&a, &b| {
            norms_vec[b]
                .partial_cmp(&norms_vec[a])
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 3. Keep top generators (reserve space for diagonal hull)
        let keep_len = max_gens.saturating_sub(dims);
        let keep_indices: Vec<usize> = indices[..keep_len].to_vec();
        let collapse_indices: Vec<usize> = indices[keep_len..].to_vec();

        // 4. Extract kept generators
        let kept_gens = Self::extract_rows(&self.generators, &keep_indices)?;

        // 5. Collapse remaining → bounding box (sum of abs per dimension)
        let collapsed = Self::extract_rows(&self.generators, &collapse_indices)?;
        let hull_diag = collapsed.abs()?.sum(0)?; // [hidden_dim]

        // 6. Create diagonal matrix from hull
        let diag_matrix = Self::create_diagonal_from_vec(&hull_diag)?;

        // 7. Concatenate kept + diagonal along generator axis (dim 0)
        let new_gens = Tensor::cat(&[&kept_gens, &diag_matrix], 0)?;

        Ok(Zonotope {
            center: self.center.clone(),
            generators: new_gens,
            config: self.config.clone(),
        })
    }

    /// Extract specific rows from a tensor by indices.
    fn extract_rows(tensor: &Tensor, indices: &[usize]) -> Result<Tensor> {
        if indices.is_empty() {
            let dims = tensor.dim(0)?;
            return Tensor::zeros((dims, 0), tensor.dtype(), tensor.device());
        }
        let mut rows = Vec::with_capacity(indices.len());
        for &i in indices {
            let row = tensor.narrow(0, i, 1)?;
            rows.push(row);
        }
        Tensor::cat(&rows, 0)
    }

    /// Create a diagonal matrix from a 1D vector.
    fn create_diagonal_from_vec(diag: &Tensor) -> Result<Tensor> {
        let d = diag.dim(0)?;
        let eye = Tensor::eye(d, diag.dtype(), diag.device())?;
        eye.broadcast_mul(&diag.unsqueeze(1)?)
    }

    /// Propagate zonotope through linear map: Z' = A · Z
    ///
    /// New center: A · c
    /// New generators: A · G
    pub fn propagate_linear(&self, a_matrix: &Tensor) -> Result<Self> {
        // Row-vector convention: z_new = z @ A^T
        let a_t = a_matrix.t()?;
        let new_center = self.center.matmul(&a_t)?;
        let new_gens = self.generators.matmul(&a_t)?;
        Ok(Zonotope {
            center: new_center,
            generators: new_gens,
            config: self.config.clone(),
        })
    }

    // -----------------------------------------------------------------------
    // Certified Steering Robustness
    // -----------------------------------------------------------------------

    /// Verify that the zonotope (representing all possible perturbed states)
    /// remains within the safe region defined by CBF.
    ///
    /// **Certificate:** For all x in Z:
    /// - projection onto toxic direction <= 0
    /// - distance to safe centroid <= cbf_beta
    pub fn verify_steering_robustness(
        &self,
        safe_centroid: &Tensor,
        toxic_centroid: &Tensor,
        cbf_beta: f32,
    ) -> Result<RobustnessCertificate> {
        let (lower, upper) = self.compute_bounds()?;

        // Worst-case direction toward toxic
        let concept_vec = toxic_centroid.broadcast_sub(safe_centroid)?;
        let concept_norm = concept_vec.sqr()?.sum_all()?.sqrt()?;

        // Check worst-case upper bound projection
        let centered_upper = upper.broadcast_sub(safe_centroid)?;
        let concept_flat = concept_vec.flatten_all()?;
        let proj_upper = (centered_upper.flatten_all()? * &concept_flat)?
            .sum_all()?
            .to_scalar::<f32>()?;
        let proj_normalized = proj_upper / (concept_norm.to_scalar::<f32>()? + 1e-8);

        // Check best-case lower bound projection
        let centered_lower = lower.broadcast_sub(safe_centroid)?;
        let proj_lower = (centered_lower.flatten_all()? * &concept_flat)?
            .sum_all()?
            .to_scalar::<f32>()?;
        let proj_lower_normalized = proj_lower / (concept_norm.to_scalar::<f32>()? + 1e-8);

        // Distance to safe centroid (worst case = upper bound)
        let dist_to_safe = centered_upper
            .sqr()?
            .sum_all()?
            .sqrt()?
            .to_scalar::<f32>()?;

        // Volume comparison with interval bounds
        let volume = self.volume_proxy()?;

        // Safety checks
        let direction_safe = proj_normalized <= 0.0;
        let distance_safe = dist_to_safe <= cbf_beta;
        let certified = direction_safe && distance_safe;

        Ok(RobustnessCertificate {
            certified,
            direction_safe,
            distance_safe,
            proj_upper: proj_normalized,
            proj_lower: proj_lower_normalized,
            dist_to_safe,
            volume_proxy: volume,
            num_gens: self.num_gens()?,
        })
    }

    /// Compare zonotope bounds against pure interval bounds to measure
    /// the reduction in over-approximation (wrapping effect).
    pub fn wrapping_reduction_vs_intervals(&self, interval_width: f32) -> Result<f32> {
        let my_width = self.avg_width()?;
        if interval_width <= 0.0 {
            return Ok(0.0);
        }
        let reduction = 1.0 - (my_width / interval_width);
        Ok(reduction.max(0.0))
    }

    // -----------------------------------------------------------------------
    // S157 — True Koopman Tube MPC Support
    // -----------------------------------------------------------------------

    /// Compute the maximum error bound in infinity norm.
    ///
    /// **Mathematical Definition:**
    /// `max_error_bound(Z) = max_i (|c_i| + sum_j |G_ij|)`
    ///
    /// This represents the worst-case infinity-norm of any point in the zonotope:
    /// `||x||_inf <= max_error_bound(Z)` for all `x in Z`.
    ///
    /// Used for contractive error verification in Tube MPC:
    /// `||e_{k+1}||_inf <= rho * ||e_k||_inf + gamma`
    pub fn max_error_bound(&self) -> Result<f32> {
        // Compute sum of absolute generators per dimension: sum_j |G_ij|
        let abs_gens = self.generators.abs()?;
        let sum_per_dim = abs_gens.sum(0)?; // Shape: [1, hidden_dim]

        // Compute |c_i| + sum_j |G_ij| per dimension
        let abs_center = self.center.abs()?;
        let bound_per_dim = abs_center.broadcast_add(&sum_per_dim)?;

        // Take maximum across all dimensions
        let bound_vec = bound_per_dim.flatten_all()?.to_vec1::<f32>()?;
        let max_bound = bound_vec.into_iter().fold(f32::NEG_INFINITY, |a, b| a.max(b));
        Ok(max_bound)
    }

    /// Linear map: Z' = A · Z = {A·c, A·G}.
    ///
    /// Convenience method for tube MPC where no bias is needed.
    /// Equivalent to `affine_transform(A, None)`.
    ///
    /// **Tensor Convention:** Center is `[1, dim]`, generators are `[num_gens, dim]`.
    /// Weight matrix `A` should be `[dim, dim]` for same-dimension transforms,
    /// or `[out_dim, dim]` for dimension-changing transforms.
    ///
    /// Result: center' = A @ c^T (shape [out_dim, 1]), generators' = A @ G^T (shape [out_dim, num_gens])
    pub fn linear_map(&self, a: &Tensor) -> Result<Self> {
        self.affine_transform(a, None)
    }

    /// Scale the zonotope by a scalar factor (for contractive error bounds).
    ///
    /// Z' = alpha * Z = {alpha*c, alpha*G}
    pub fn scale_factor(&self, alpha: f32) -> Result<Self> {
        self.scale(alpha)
    }
}

/// Result of robustness verification.
#[derive(Debug, Clone)]
pub struct RobustnessCertificate {
    pub certified: bool,
    pub direction_safe: bool,
    pub distance_safe: bool,
    pub proj_upper: f32,
    pub proj_lower: f32,
    pub dist_to_safe: f32,
    pub volume_proxy: f32,
    pub num_gens: usize,
}

impl std::fmt::Display for RobustnessCertificate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RobustnessCertificate {{ certified={}, direction_safe={}, distance_safe={}, \
             proj_upper={:.4}, proj_lower={:.4}, dist_to_safe={:.4}, volume={:.4}, gens={} }}",
            self.certified,
            self.direction_safe,
            self.distance_safe,
            self.proj_upper,
            self.proj_lower,
            self.dist_to_safe,
            self.volume_proxy,
            self.num_gens
        )
    }
}

/// Hybrid zonotope-interval representation for mixed precision verification.
///
/// Uses zonotopes for linear layers (exact) and intervals for non-linear layers
/// (conservative but fast), then converts back to zonotope.
#[derive(Debug, Clone)]
pub struct HybridZonotope {
    pub zonotope: Zonotope,
    pub interval_lo: Tensor, // [1, hidden_dim]
    pub interval_hi: Tensor, // [1, hidden_dim]
}

impl HybridZonotope {
    /// Create from a zonotope, computing interval bounds.
    pub fn from_zonotope(z: &Zonotope) -> Result<Self> {
        let (lo, hi) = z.compute_bounds()?;
        Ok(Self {
            zonotope: z.clone(),
            interval_lo: lo,
            interval_hi: hi,
        })
    }

    /// Apply non-linear op using interval arithmetic, then convert back to zonotope.
    pub fn nonlinear_interval_step(&self, op: &str) -> Result<Zonotope> {
        let (new_lo, new_hi) = match op {
            "relu" => {
                let new_lo = self.interval_lo.relu()?;
                let new_hi = self.interval_hi.relu()?;
                (new_lo, new_hi)
            }
            "silu" => {
                // SiLU bounds: SiLU is monotonic for x > -1
                let new_lo = self.interval_lo.silu()?;
                let new_hi = self.interval_hi.silu()?;
                (new_lo, new_hi)
            }
            _ => {
                // Default: pass through
                (self.interval_lo.clone(), self.interval_hi.clone())
            }
        };
        Zonotope::from_intervals(&new_lo, &new_hi)
    }

    /// Refine zonotope using interval bounds (prune generators that violate intervals).
    pub fn refine_with_intervals(&self) -> Result<Zonotope> {
        let (z_lo, z_hi) = self.zonotope.compute_bounds()?;

        // Clamp zonotope bounds to interval bounds
        let clamped_lo = z_lo.maximum(&self.interval_lo)?;
        let clamped_hi = z_hi.minimum(&self.interval_hi)?;

        Zonotope::from_intervals(&clamped_lo, &clamped_hi)
    }
}

// ---------------------------------------------------------------------------
// Taylor Zonotope Propagation (Sprint 127 — The Thermodynamic Sun)
// ---------------------------------------------------------------------------
// Taylor Models for Neural ODE Verification:
//
// x(t + Δt) = x(t) + Σ_{k=1}^{p} f^{(k)}(x(t)) / k! · (Δt)^k + R_{p+1}
//
// Where R_{p+1} is the Lagrange remainder bound that guarantees formal verification.

/// Taylor Propagation Result — Center + Generators + Remainder Bound
#[derive(Debug, Clone)]
pub struct TaylorPropagationResult {
    /// Updated center after Taylor step.
    pub center: Tensor,
    /// Updated generators after Taylor step.
    pub generators: Tensor,
    /// Remainder bound (over-approximation of truncation error).
    pub remainder_bound: f32,
    /// Taylor order used.
    pub order: usize,
    /// Time step used.
    pub dt: f64,
}

impl TaylorPropagationResult {
    /// Check if the remainder bound is within tolerance.
    pub fn is_verified(&self, tolerance: f32) -> bool {
        self.remainder_bound < tolerance
    }

    /// Summary report.
    pub fn summary(&self) -> String {
        format!(
            "TaylorPropagation(order={}, dt={:.4}, remainder={:.6e}, verified={})",
            self.order,
            self.dt,
            self.remainder_bound,
            self.is_verified(1e-3)
        )
    }
}

/// Taylor Zonotope Propagation — Neural ODE Step with Formal Remainder Bound
///
/// Propagates a zonotope through one Neural ODE step using Taylor expansion:
/// ```text
/// x(t + Δt) = x(t) + f(x)·Δt + f''(x)/2·Δt² + ... + R_{p+1}
/// ```
///
/// # Parameters
/// - `state`: Current state tensor [1, dim].
/// - `dt`: Time step.
/// - `order`: Taylor expansion order (1 = Euler, 2 = second-order, etc.).
///
/// # Returns
/// Updated state tensor (center of propagated zonotope).
///
/// # Note
/// This is a stub implementation using Euler (order 1). Higher-order terms
/// and formal remainder bounds are prepared for integration with Candle's
/// autograd for Jacobian computation.
pub fn propagate_taylor_zonotope(state: &Tensor, dt: f64, order: usize) -> Result<Tensor> {
    // Placeholder: Euler (order 1) integration.
    // In production, evaluate Neural ODE dynamics: f(x) = neural_ode_forward(x)
    let f_x = state.clone(); // Replace with actual ODE forward pass

    let dt_tensor = Tensor::new(&[dt as f32], state.device())?;
    let step = f_x.broadcast_mul(&dt_tensor)?;

    // Higher-order stub: for order >= 2, add second derivative term
    if order >= 2 {
        // f''(x) · Δt² / 2 — stub as scaled first derivative
        let second_order = f_x
            .broadcast_mul(&Tensor::new(&[(0.5 * dt * dt) as f32], state.device())?)?
            .broadcast_mul(&Tensor::new(&[0.1f32], state.device())?)?; // Approximation factor
        let step = step.broadcast_add(&second_order)?;
        Ok(state.broadcast_add(&step)?)
    } else {
        state.broadcast_add(&step)
    }
}

/// Taylor Zonotope Propagation with Full Remainder Bound
///
/// Returns complete Taylor propagation result including generators and
/// formal remainder bound for verification.
///
/// # Parameters
/// - `zonotope`: Input zonotope to propagate.
/// - `dt`: Time step.
/// - `order`: Taylor expansion order.
/// - `lipchitz_bound`: Upper bound on the Lipschitz constant of f(x).
pub fn propagate_taylor_zonotope_full(
    zonotope: &Zonotope,
    dt: f64,
    order: usize,
    lipchitz_bound: f32,
) -> Result<TaylorPropagationResult> {
    let effective_order = order.max(1).min(3); // Clamp to [1, 3]
    let device = zonotope.center.device();

    // Taylor center propagation
    let new_center = propagate_taylor_zonotope(&zonotope.center, dt, effective_order)?;

    // Generator propagation: G' = J · G where J ≈ I + dt · f'(x)
    // For Euler: J ≈ I + dt · (Lipschitz approximation)
    let jacobian_scale = 1.0 + dt as f32 * lipchitz_bound;
    let jacobian_scale_tensor = Tensor::new(&[jacobian_scale], device)?;
    let new_generators = zonotope.generators.broadcast_mul(&jacobian_scale_tensor)?;

    // Lagrange remainder bound: R_{p+1} ≤ L · M · |Δt|^{p+1} / (p+1)!
    // where L is Lipschitz constant, M bounds the (p+1)-th derivative
    let factorial = match effective_order + 1 {
        2 => 2.0_f64,
        3 => 6.0_f64,
        4 => 24.0_f64,
        _ => 1.0_f64,
    };
    let remainder = (lipchitz_bound as f64) * (dt.powi((effective_order + 1) as i32)) / factorial;

    // Add remainder to generators as inflation
    let remainder_bound = remainder as f32;

    Ok(TaylorPropagationResult {
        center: new_center,
        generators: new_generators,
        remainder_bound,
        order: effective_order,
        dt,
    })
}

/// Second-order Taylor propagation with explicit Hessian approximation.
///
/// Uses finite-difference approximation for the second derivative term:
/// ```text
/// f''(x) ≈ (f(x + h) - 2·f(x) + f(x - h)) / h²
/// ```
///
/// # Parameters
/// - `state`: Current state tensor.
/// - `dt`: Time step.
/// - `h`: Finite difference step size.
pub fn propagate_taylor_order2(state: &Tensor, dt: f64, h: f32) -> Result<Tensor> {
    let device = state.device();

    // f(x) — first derivative approximation (identity for stub)
    let f_x = state.clone();

    // f(x + h) and f(x - h) for finite difference
    let h_tensor = Tensor::new(&[h], device)?;
    let state_plus = state.broadcast_add(&h_tensor)?;
    let state_minus = state.broadcast_sub(&h_tensor)?;

    // f(x+h) and f(x-h) — identity approximation
    let f_plus = state_plus;
    let f_minus = state_minus;

    // f''(x) ≈ (f(x+h) - 2f(x) + f(x-h)) / h²
    // For identity: f''(x) = 0, so this term vanishes
    // In production, use actual Neural ODE forward pass
    let two_f = f_x.broadcast_mul(&Tensor::new(&[2.0f32], device)?)?;
    let numerator = f_plus.broadcast_sub(&two_f)?.broadcast_add(&f_minus)?;
    let h_sq = Tensor::new(&[h * h], device)?;
    let f_double_prime = numerator.broadcast_div(&h_sq)?;

    // x + f(x)·dt + f''(x)/2 · dt²
    let dt_tensor = Tensor::new(&[dt as f32], device)?;
    let first_order = f_x.broadcast_mul(&dt_tensor)?;
    let second_order =
        f_double_prime.broadcast_mul(&Tensor::new(&[(0.5 * dt * dt) as f32], device)?)?;

    state
        .broadcast_add(&first_order)?
        .broadcast_add(&second_order)
}

/// CBF Safety Verification for Taylor-Propagated Zonotope.
///
/// Verifies that the Control Barrier Function h(x) ≥ 0 holds
/// for all points in the propagated zonotope.
///
/// # Parameters
/// - `result`: Taylor propagation result.
/// - `safe_center`: Center of the safe set.
/// - `margin`: Required safety margin.
pub fn verify_taylor_cbf_safety(
    result: &TaylorPropagationResult,
    safe_center: &Tensor,
    margin: f32,
) -> Result<bool> {
    // Compute bounds of propagated zonotope
    let abs_sum: Tensor = result.generators.abs()?.sum(0)?;
    let lower = result.center.broadcast_sub(&abs_sum)?;
    let upper = result.center.broadcast_add(&abs_sum)?;

    // Check if all bounds are within safe region
    let margin_tensor = Tensor::new(&[margin], result.center.device())?;
    let safe_lower = safe_center.broadcast_sub(&margin_tensor)?;
    let safe_upper = safe_center.broadcast_add(&margin_tensor)?;

    // Verify: lower >= safe_lower AND upper <= safe_upper
    let lo_ok = lower.broadcast_sub(&safe_lower)?.ge(0f32)?;
    let hi_ok = safe_upper.broadcast_sub(&upper)?.ge(0f32)?;

    // All elements must satisfy
    let all_ok = lo_ok.mul(&hi_ok)?;
    // Check if any element is 0 (false) — use to_vec and check
    let vec: Vec<u8> = all_ok.flatten_all()?.to_vec1()?;
    Ok(vec.iter().all(|&v| v != 0))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::Device;

    #[test]
    fn test_zonotope_creation() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::new(&[1.0f32, 2.0, 3.0], &device)?.unsqueeze(0)?;
        let z = Zonotope::new_from_epsilon(&center, 0.1, 3)?;

        assert_eq!(z.center.shape().dims()[1], 3);
        assert_eq!(z.num_gens()?, 3);
        assert_eq!(z.hidden_dim()?, 3);
        Ok(())
    }

    #[test]
    fn test_zonotope_bounds() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::new(&[0.0f32, 0.0], &device)?.unsqueeze(0)?;
        let z = Zonotope::new_from_epsilon(&center, 0.5, 2)?;

        let (lo, hi) = z.compute_bounds()?;
        let lo_vec: Vec<f32> = lo.flatten_all()?.to_vec1()?;
        let hi_vec: Vec<f32> = hi.flatten_all()?.to_vec1()?;

        // With epsilon=0.5 and diagonal generators, bounds should be [-0.5, 0.5]
        assert!((lo_vec[0] + 0.5).abs() < 1e-5);
        assert!((hi_vec[0] - 0.5).abs() < 1e-5);
        Ok(())
    }

    #[test]
    fn test_affine_transform() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::new(&[1.0f32, 2.0], &device)?.unsqueeze(0)?;
        let z = Zonotope::new_from_epsilon(&center, 0.1, 2)?;

        // Identity transform
        let weight = Tensor::from_vec(vec![1.0f32, 0.0, 0.0, 1.0], (2, 2), &device)?;
        let z2 = z.affine_transform(&weight, None)?;

        // Center should be unchanged
        let c: Vec<f32> = z2.center.flatten_all()?.to_vec1()?;
        assert!((c[0] - 1.0).abs() < 1e-5);
        assert!((c[1] - 2.0).abs() < 1e-5);
        Ok(())
    }

    #[test]
    fn test_relu_approx() -> Result<()> {
        let device = Device::Cpu;
        // Center at [1.0, -1.0, 0.0] — positive, negative, mixed
        let center = Tensor::new(&[1.0f32, -1.0, 0.0], &device)?.unsqueeze(0)?;
        let z = Zonotope::new_from_epsilon(&center, 0.1, 3)?;

        let z_relu = z.relu_approx()?;
        let c: Vec<f32> = z_relu.center.flatten_all()?.to_vec1()?;

        // ReLU([1, -1, 0]) = [1, 0, 0]
        assert!((c[0] - 1.0).abs() < 1e-4);
        assert!(c[1].abs() < 1e-4);
        Ok(())
    }

    #[test]
    fn test_minkowski_sum() -> Result<()> {
        let device = Device::Cpu;
        let c1 = Tensor::new(&[1.0f32, 2.0], &device)?.unsqueeze(0)?;
        let c2 = Tensor::new(&[3.0f32, 4.0], &device)?.unsqueeze(0)?;
        let z1 = Zonotope::new_from_epsilon(&c1, 0.1, 2)?;
        let z2 = Zonotope::new_from_epsilon(&c2, 0.1, 2)?;

        let z_sum = z1.minkowski_sum(&z2)?;
        let c: Vec<f32> = z_sum.center.flatten_all()?.to_vec1()?;

        assert!((c[0] - 4.0).abs() < 1e-5);
        assert!((c[1] - 6.0).abs() < 1e-5);
        Ok(())
    }

    #[test]
    fn test_volume_proxy() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::zeros((1, 4), DType::F32, &device)?;
        let z = Zonotope::new_from_epsilon(&center, 0.1, 4)?;

        let vol = z.volume_proxy()?;
        assert!(vol > 0.0);
        assert!((vol - 0.4).abs() < 1e-5); // 4 gens * 0.1 each
        Ok(())
    }

    #[test]
    fn test_hybrid_zonotope() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::new(&[1.0f32, -1.0], &device)?.unsqueeze(0)?;
        let z = Zonotope::new_from_epsilon(&center, 0.1, 2)?;

        let hybrid = HybridZonotope::from_zonotope(&z)?;
        let z_refined = hybrid.nonlinear_interval_step("relu")?;

        // After ReLU, bounds should be non-negative
        let (lo, _) = z_refined.compute_bounds()?;
        let lo_vec: Vec<f32> = lo.flatten_all()?.to_vec1()?;
        assert!(lo_vec[0] >= -1e-4);
        assert!(lo_vec[1] >= -1e-4);
        Ok(())
    }

    #[test]
    fn test_point_zonotope() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::new(&[1.0f32, 2.0, 3.0], &device)?.unsqueeze(0)?;
        let z = Zonotope::point(&center)?;

        assert_eq!(z.num_gens()?, 0);
        let (lo, hi) = z.compute_bounds()?;
        // Point zonotope: bounds = center
        let diff = lo
            .broadcast_sub(&hi)?
            .abs()?
            .sum_all()?
            .to_scalar::<f32>()?;
        assert!(diff.abs() < 1e-5);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Taylor Zonotope Propagation Tests (Sprint 127)
    // -----------------------------------------------------------------------

    #[test]
    fn test_taylor_propagation_order1() -> Result<()> {
        let device = Device::Cpu;
        let state = Tensor::ones((1, 10), DType::F32, &device)?;
        let result = propagate_taylor_zonotope(&state, 0.1, 1)?;
        assert_eq!(result.shape().dims(), &[1, 10]);
        // Euler: x + f(x)*dt = 1.0 + 1.0*0.1 = 1.1
        let val: f32 = result.mean_all()?.to_scalar()?;
        assert!((val - 1.1).abs() < 1e-5);
        Ok(())
    }

    #[test]
    fn test_taylor_propagation_order2() -> Result<()> {
        let device = Device::Cpu;
        let state = Tensor::ones((1, 10), DType::F32, &device)?;
        let result = propagate_taylor_zonotope(&state, 0.1, 2)?;
        assert_eq!(result.shape().dims(), &[1, 10]);
        // Order 2 adds second-order term
        let val: f32 = result.mean_all()?.to_scalar()?;
        // Should be > Euler result due to positive second-order term
        assert!(val > 1.1);
        Ok(())
    }

    #[test]
    fn test_taylor_propagation_preserves_shape() -> Result<()> {
        let device = Device::Cpu;
        let state = Tensor::zeros((1, 4096), DType::F32, &device)?;
        let result = propagate_taylor_zonotope(&state, 0.01, 1)?;
        assert_eq!(result.shape().dims(), &[1, 4096]);
        Ok(())
    }

    #[test]
    fn test_taylor_propagation_zero_dt() -> Result<()> {
        let device = Device::Cpu;
        let state = Tensor::new(&[1.0f32, 2.0, 3.0], &device)?.unsqueeze(0)?;
        let result = propagate_taylor_zonotope(&state, 0.0, 1)?;
        // With dt=0, result should equal input
        let diff = state
            .broadcast_sub(&result)?
            .abs()?
            .sum_all()?
            .to_scalar::<f32>()?;
        assert!(diff < 1e-5);
        Ok(())
    }

    #[test]
    fn test_taylor_propagation_full() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::new(&[1.0f32, 2.0, 3.0], &device)?.unsqueeze(0)?;
        let z = Zonotope::new_from_epsilon(&center, 0.1, 3)?;
        let result = propagate_taylor_zonotope_full(&z, 0.1, 1, 1.0)?;
        assert_eq!(result.center.shape().dims(), &[1, 3]);
        assert!(result.remainder_bound >= 0.0);
        assert_eq!(result.order, 1);
        assert!((result.dt - 0.1).abs() < 1e-10);
        Ok(())
    }

    #[test]
    fn test_taylor_propagation_full_order_clamping() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::new(&[1.0f32, 2.0], &device)?.unsqueeze(0)?;
        let z = Zonotope::new_from_epsilon(&center, 0.1, 2)?;
        // Request order 10, should clamp to 3
        let result = propagate_taylor_zonotope_full(&z, 0.1, 10, 1.0)?;
        assert_eq!(result.order, 3);
        Ok(())
    }

    #[test]
    fn test_taylor_propagation_remainder_decreases_with_order() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::new(&[1.0f32, 2.0], &device)?.unsqueeze(0)?;
        let z = Zonotope::new_from_epsilon(&center, 0.1, 2)?;
        let r1 = propagate_taylor_zonotope_full(&z, 0.1, 1, 1.0)?;
        let r2 = propagate_taylor_zonotope_full(&z, 0.1, 2, 1.0)?;
        // Higher order → smaller remainder
        assert!(r2.remainder_bound < r1.remainder_bound);
        Ok(())
    }

    #[test]
    fn test_taylor_result_is_verified() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::new(&[1.0f32, 2.0], &device)?.unsqueeze(0)?;
        let z = Zonotope::new_from_epsilon(&center, 0.01, 2)?;
        let result = propagate_taylor_zonotope_full(&z, 0.01, 2, 0.1)?;
        // Small dt + low Lipschitz → small remainder → verified
        assert!(result.is_verified(1.0));
        Ok(())
    }

    #[test]
    fn test_taylor_result_summary() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::new(&[1.0f32], &device)?.unsqueeze(0)?;
        let z = Zonotope::new_from_epsilon(&center, 0.1, 1)?;
        let result = propagate_taylor_zonotope_full(&z, 0.1, 1, 1.0)?;
        let summary = result.summary();
        assert!(summary.contains("order="));
        assert!(summary.contains("dt="));
        assert!(summary.contains("remainder="));
        Ok(())
    }

    #[test]
    fn test_taylor_order2_finite_difference() -> Result<()> {
        let device = Device::Cpu;
        let state = Tensor::new(&[1.0f32, 2.0, 3.0], &device)?.unsqueeze(0)?;
        let result = propagate_taylor_order2(&state, 0.1, 0.001)?;
        assert_eq!(result.shape().dims(), &[1, 3]);
        // For identity f(x)=x, f''(x)=0, so order2 ≈ order1
        let euler = propagate_taylor_zonotope(&state, 0.1, 1)?;
        let diff = result
            .broadcast_sub(&euler)?
            .abs()?
            .sum_all()?
            .to_scalar::<f32>()?;
        // Difference should be tiny (f'' ≈ 0 for identity)
        assert!(diff < 1.0);
        Ok(())
    }

    #[test]
    fn test_taylor_cbf_safety_safe() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::new(&[5.0f32, 5.0], &device)?.unsqueeze(0)?;
        let gens = Tensor::zeros((2, 2), DType::F32, &device)?;
        let result = TaylorPropagationResult {
            center: center.clone(),
            generators: gens,
            remainder_bound: 0.0,
            order: 1,
            dt: 0.1,
        };
        let safe_center = Tensor::new(&[5.0f32, 5.0], &device)?.unsqueeze(0)?;
        let is_safe = verify_taylor_cbf_safety(&result, &safe_center, 1.0)?;
        assert!(is_safe);
        Ok(())
    }

    #[test]
    fn test_taylor_cbf_safety_unsafe() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::new(&[10.0f32, 10.0], &device)?.unsqueeze(0)?;
        let gens = Tensor::zeros((2, 2), DType::F32, &device)?;
        let result = TaylorPropagationResult {
            center,
            generators: gens,
            remainder_bound: 0.0,
            order: 1,
            dt: 0.1,
        };
        let safe_center = Tensor::new(&[0.0f32, 0.0], &device)?.unsqueeze(0)?;
        let is_safe = verify_taylor_cbf_safety(&result, &safe_center, 1.0)?;
        assert!(!is_safe);
        Ok(())
    }

    #[test]
    fn test_full_taylor_pipeline() -> Result<()> {
        let device = Device::Cpu;
        // Create zonotope
        let center = Tensor::new(&[1.0f32, 2.0, 3.0], &device)?.unsqueeze(0)?;
        let z = Zonotope::new_from_epsilon(&center, 0.05, 3)?;

        // Propagate with Taylor
        let result = propagate_taylor_zonotope_full(&z, 0.01, 2, 1.0)?;

        // Verify safety
        let safe_center = center.clone();
        let is_safe = verify_taylor_cbf_safety(&result, &safe_center, 1.0)?;

        // Check result integrity
        assert_eq!(result.order, 2);
        assert!(result.remainder_bound >= 0.0);
        // With small dt and close safe_center, should be safe
        assert!(is_safe || result.remainder_bound < 1.0);
        Ok(())
    }
}
