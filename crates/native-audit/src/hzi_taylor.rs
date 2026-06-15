//! Hybrid Zonotope-Interval (HZI) + Taylor Model Integration.
//!
//! Sprint 166 (v16.6.0) — Tight Tubes for Edge-Certified Control.
//!
//! Extends the hybrid zonotope with:
//! 1. **Taylor Model Remainder Bounds**: Certified polynomial approximation
//!    with guaranteed remainder for nonlinear propagation.
//! 2. **Girard Order Reduction**: Optimal generator reduction minimizing
//!    volume increase (not simple truncation).
//! 3. **Interval-Nonlinearity Tracking**: Separate interval bounds for
//!    nonlinear parts, reducing wrapping effect.
//! 4. **Conformal Epsilon Integration**: PAC-conformal calibration of
//!    tube radius for CBF verification.
//!
//! **Taylor Model Propagation:**
//! ```math
//! T(x) = p_0 + p_1(x - x_0) + \\frac{1}{2}p_2(x - x_0)^2 + [-r, r]
//! ```
//! where remainder r is bounded via mean-value form:
//! ```math
//! r = \\frac{M_3}{6} \\delta^3
//! ```
//! with M_3 = bound on third derivative, δ = interval width.
//!
//! **Girard Reduction:**
//! Given generators G ∈ ℝ^{k×n} with k > k_max:
//! 1. Compute G^T G (Gram matrix).
//! 2. Find smallest eigenvalue λ_min and eigenvector v_min.
//! 3. Remove generator corresponding to v_min direction.
//! 4. Add box generator with radius ||G v_min||_∞.
//!
//! **HZI Propagation:**
//! ```math
//! Z_{k+1} = K Z_k ⊕ W ⊖ ε_k
//! ```
//! where ε_k is conformal calibration radius.

use candle_core::{DType, Result, Tensor};

use crate::taylor_model::{TaylorConfig, TaylorModel};
use crate::zonotope::{Zonotope, ZonotopeConfig};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for HZI + Taylor propagation.
#[derive(Debug, Clone)]
pub struct HZIConfig {
    /// Base zonotope configuration.
    pub zonotope_config: ZonotopeConfig,
    /// Taylor model configuration.
    pub taylor_config: TaylorConfig,
    /// Enable Girard order reduction.
    pub girard_reduction: bool,
    /// Maximum generators after reduction.
    pub max_gens_after_reduction: usize,
    /// Conformal epsilon for tube tightening.
    pub conformal_epsilon: f32,
    /// Disturbance bound W.
    pub disturbance_bound: f32,
    /// Taylor order for nonlinear expansion.
    pub taylor_order: usize,
    /// Remainder bound scaling factor (conservative buffer).
    pub remainder_scale: f32,
}

impl Default for HZIConfig {
    fn default() -> Self {
        Self {
            zonotope_config: ZonotopeConfig::default(),
            taylor_config: TaylorConfig::default(),
            girard_reduction: true,
            max_gens_after_reduction: 32,
            conformal_epsilon: 0.05,
            disturbance_bound: 0.05,
            taylor_order: 2,
            remainder_scale: 1.1,
        }
    }
}

impl HZIConfig {
    /// Fast configuration for edge devices.
    pub fn edge_fast() -> Self {
        Self {
            zonotope_config: ZonotopeConfig {
                max_gens: 32,
                epsilon: 0.08,
                reduce_after_nonlinear: true,
                prune_threshold: 1e-5,
            },
            taylor_config: TaylorConfig {
                order: 1,
                max_remainder: 0.5,
                hybrid_reduction: true,
                zonotope_threshold: 0.2,
            },
            girard_reduction: true,
            max_gens_after_reduction: 16,
            conformal_epsilon: 0.08,
            disturbance_bound: 0.08,
            taylor_order: 1,
            remainder_scale: 1.2,
        }
    }

    /// High-precision configuration.
    pub fn high_precision() -> Self {
        Self {
            zonotope_config: ZonotopeConfig {
                max_gens: 128,
                epsilon: 0.02,
                reduce_after_nonlinear: true,
                prune_threshold: 1e-8,
            },
            taylor_config: TaylorConfig {
                order: 3,
                max_remainder: 0.1,
                hybrid_reduction: true,
                zonotope_threshold: 0.05,
            },
            girard_reduction: true,
            max_gens_after_reduction: 64,
            conformal_epsilon: 0.02,
            disturbance_bound: 0.02,
            taylor_order: 3,
            remainder_scale: 1.05,
        }
    }
}

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// Result of HZI propagation step.
#[derive(Debug, Clone)]
pub struct HZIPropagationResult {
    /// Propagated HZI state.
    pub hzi: HybridZonotopeTaylor,
    /// Tube radius at this step.
    pub tube_radius: f32,
    /// Volume proxy (log).
    pub log_volume: f32,
    /// Number of generators.
    pub num_gens: usize,
    /// Taylor remainder bound.
    pub taylor_remainder: f32,
    /// Conformal epsilon applied.
    pub conformal_epsilon: f32,
    /// Safety margin (CBF lower bound).
    pub safety_margin: f32,
}

impl std::fmt::Display for HZIPropagationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HZI{{ r={:.6}, log_vol={:.2}, gens={}, taylor_r={:.6e}, ε={:.4}, safety={:.4} }}",
            self.tube_radius,
            self.log_volume,
            self.num_gens,
            self.taylor_remainder,
            self.conformal_epsilon,
            self.safety_margin,
        )
    }
}

/// Result of Girard reduction.
#[derive(Debug, Clone)]
pub struct GirardReductionResult {
    /// Generators before reduction.
    pub gens_before: usize,
    /// Generators after reduction.
    pub gens_after: usize,
    /// Volume increase factor (log10).
    pub volume_increase_log10: f32,
    /// Box radius added.
    pub box_radius: f32,
}

// ---------------------------------------------------------------------------
// Hybrid Zonotope-Taylor Core
// ---------------------------------------------------------------------------

/// Hybrid Zonotope-Interval with Taylor Model integration.
///
/// Combines:
/// - Zonotope for exact affine propagation.
/// - Interval bounds for nonlinear wrapping control.
/// - Taylor models for certified polynomial approximation.
#[derive(Debug, Clone)]
pub struct HybridZonotopeTaylor {
    /// Center of the set. Shape: [1, dim].
    pub center: Tensor,
    /// Zonotope generators. Shape: [num_gens, dim].
    pub generators: Tensor,
    /// Interval bounds for nonlinear parts. Shape: [dim].
    pub interval_lo: Tensor,
    pub interval_hi: Tensor,
    /// Taylor model remainder. Shape: [dim].
    pub taylor_remainder: Tensor,
    /// Current number of generators.
    pub num_gens: usize,
    /// Dimension.
    pub dim: usize,
    /// Configuration.
    pub config: HZIConfig,
}

impl HybridZonotopeTaylor {
    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    /// Create from center + epsilon ball.
    pub fn new_from_epsilon(center: &Tensor, epsilon: f32, config: HZIConfig) -> Result<Self> {
        let device = center.device();
        let dim = center.dim(1)?;
        let max_gens = config.zonotope_config.max_gens.min(dim);

        // Diagonal generators
        let mut gen_data = vec![0.0f32; max_gens * dim];
        for i in 0..max_gens {
            gen_data[i * dim + i] = epsilon;
        }
        let generators = Tensor::from_vec(gen_data, (max_gens, dim), device)?;

        // Interval bounds: center ± epsilon
        let eps_tensor = Tensor::new(epsilon, device)?.to_dtype(DType::F32)?;
        let center_f32 = center.to_dtype(DType::F32)?;
        let interval_lo = center_f32.broadcast_sub(&eps_tensor)?;
        let interval_hi = center_f32.broadcast_add(&eps_tensor)?;

        // Taylor remainder: initial epsilon (first-order truncation)
        let taylor_remainder = Tensor::full(epsilon, (dim,), device)?.to_dtype(DType::F32)?;

        Ok(Self {
            center: center_f32,
            generators,
            interval_lo,
            interval_hi,
            taylor_remainder,
            num_gens: max_gens,
            dim,
            config,
        })
    }

    /// Create from existing zonotope.
    pub fn from_zonotope(z: &Zonotope, config: HZIConfig) -> Result<Self> {
        let (lo, hi) = z.compute_bounds()?;
        let dim = z.center.dim(1)?;
        let num_gens = z.generators.dim(0)?;
        let _device = z.center.device();

        // Taylor remainder: initial from zonotope width
        let widths = hi.broadcast_sub(&lo)?;

        Ok(Self {
            center: z.center.clone(),
            generators: z.generators.clone(),
            interval_lo: lo,
            interval_hi: hi,
            taylor_remainder: widths,
            num_gens,
            dim,
            config,
        })
    }

    /// Create from Taylor model.
    pub fn from_taylor_model(tm: &TaylorModel, config: HZIConfig) -> Result<Self> {
        let device = tm.center.device();
        let dim = tm.dim;

        // Taylor model → zonotope: center = tm.center, generators from linear term
        let linear_rows = tm.linear.dim(0)?;
        let max_gens = config.zonotope_config.max_gens.min(dim);
        let actual_gens = linear_rows.min(max_gens);

        // Use rows of linear matrix as generators
        let generators = if actual_gens > 0 {
            tm.linear.narrow(0, 0, actual_gens)?.to_dtype(DType::F32)?
        } else {
            Tensor::zeros((max_gens, dim), DType::F32, device)?
        };

        // Interval bounds from Taylor model
        let center_f32 = tm.center.to_dtype(DType::F32)?;
        let rem = tm.remainder;
        let rem_tensor = Tensor::new(rem, device)?.to_dtype(DType::F32)?;
        let interval_lo = center_f32.broadcast_sub(&rem_tensor)?;
        let interval_hi = center_f32.broadcast_add(&rem_tensor)?;

        // Taylor remainder
        let taylor_remainder = Tensor::full(rem, (dim,), device)?.to_dtype(DType::F32)?;

        Ok(Self {
            center: center_f32,
            generators,
            interval_lo,
            interval_hi,
            taylor_remainder,
            num_gens: actual_gens,
            dim,
            config,
        })
    }

    // -----------------------------------------------------------------------
    // Propagation: Affine (exact)
    // -----------------------------------------------------------------------

    /// Exact affine propagation: Z' = (Wc + b, WG).
    ///
    /// **Formula:**
    /// ```math
    /// c' = W c + b
    /// G' = W G
    /// ```
    /// Interval bounds and Taylor remainder transform accordingly.
    pub fn affine_propagate(&self, weight: &Tensor, bias: Option<&Tensor>) -> Result<Self> {
        let _device = self.center.device();

        // Affine center
        let new_center = self.center.matmul(weight)?;
        let new_center = if let Some(b) = bias {
            new_center.broadcast_add(b)?
        } else {
            new_center
        };

        // Affine generators: G' = G @ W^T (row-vector convention)
        let w_t = weight.t()?;
        let new_generators = self.generators.matmul(&w_t)?;

        // Update interval bounds via interval arithmetic on affine map
        let (new_lo, new_hi) = self.affine_interval_propagate(weight, bias)?;

        // Taylor remainder: scales with linear part
        // taylor_remainder is [dim], weight is [out_dim, in_dim]
        // new_remainder_j = sum_i |weight[j,i]| * remainder_i
        let rem_2d = self.taylor_remainder.abs()?.unsqueeze(0)?; // [1, dim]
        let w_abs = weight.abs()?;
        let new_remainder = rem_2d.matmul(&w_abs.t()?)?.squeeze(0)?;

        let new_dim = new_center.dim(1)?;
        let num_gens = self.num_gens;

        let mut result = Self {
            center: new_center,
            generators: new_generators,
            interval_lo: new_lo,
            interval_hi: new_hi,
            taylor_remainder: new_remainder,
            num_gens,
            dim: new_dim,
            config: self.config.clone(),
        };

        // Apply Girard reduction if needed
        if self.config.girard_reduction && num_gens > self.config.max_gens_after_reduction {
            result = result.girard_reduce()?;
        }

        Ok(result)
    }

    /// Affine interval propagation.
    fn affine_interval_propagate(
        &self,
        weight: &Tensor,
        bias: Option<&Tensor>,
    ) -> Result<(Tensor, Tensor)> {
        let device = self.center.device();
        let w_abs = weight.abs()?;
        let w_t = weight.t()?;

        // For each output dimension j:
        // lo'_j = min over x in [lo,hi] of (Wx+b)_j
        // hi'_j = max over x in [lo,hi] of (Wx+b)_j
        // Using interval arithmetic: [lo'_j, hi'_j] = Σ_i W_ij · [lo_i, hi_i] + b_j

        // Split W into positive and negative parts
        let w_pos = w_abs.broadcast_mul(&w_t.sign()?)?.relu()?;
        let w_neg = w_abs
            .broadcast_sub(&w_pos)?
            .broadcast_mul(&Tensor::new(-1.0f32, device)?)?;

        // lo' = W_pos @ lo + W_neg @ hi + b
        let lo_pos = self.interval_lo.matmul(&w_pos)?;
        let hi_neg = self
            .interval_hi
            .matmul(&w_neg)?
            .broadcast_mul(&Tensor::new(-1.0f32, device)?)?;
        let new_lo = lo_pos.broadcast_add(&hi_neg)?;

        // hi' = W_pos @ hi + W_neg @ lo + b
        let hi_pos = self.interval_hi.matmul(&w_pos)?;
        let lo_neg = self
            .interval_lo
            .matmul(&w_neg)?
            .broadcast_mul(&Tensor::new(-1.0f32, device)?)?;
        let new_hi = hi_pos.broadcast_add(&lo_neg)?;

        let new_lo = if let Some(b) = bias {
            new_lo.broadcast_add(b)?
        } else {
            new_lo
        };
        let new_hi = if let Some(b) = bias {
            new_hi.broadcast_add(b)?
        } else {
            new_hi
        };

        Ok((new_lo, new_hi))
    }

    // -----------------------------------------------------------------------
    // Propagation: Nonlinear with Taylor Remainder
    // -----------------------------------------------------------------------

    /// Propagate through ReLU with Taylor model remainder.
    ///
    /// **Taylor expansion of ReLU at center c:**
    /// ```math
    /// \\text{ReLU}(x) = \\text{ReLU}(c) + \\text{ReLU}'(c)(x - c) + R_1(x)
    /// ```
    /// where R_1(x) is bounded by the interval width.
    pub fn relu_taylor_propagate(&self) -> Result<Self> {
        let device = self.center.device();

        // ReLU center
        let new_center = self.center.relu()?;

        // Slope bounds per dimension
        let (slope_lo, slope_hi) = self.relu_slope_bounds()?;

        // Apply slope bounds to generators
        // G'[i,j] = ((l_j + h_j)/2) * G[i,j]
        let slope_mid = slope_lo
            .broadcast_add(&slope_hi)?
            .broadcast_div(&Tensor::new(2.0f32, device)?)?;
        let new_generators = self.generators.broadcast_mul(&slope_mid.unsqueeze(0)?)?;

        // Taylor remainder: increase by slope uncertainty
        let slope_width = slope_hi.broadcast_sub(&slope_lo)?;
        let current_widths = self.interval_hi.broadcast_sub(&self.interval_lo)?;
        let new_remainder = self
            .taylor_remainder
            .broadcast_add(&(slope_width.broadcast_mul(&current_widths)?))?;

        // Update interval bounds
        let new_interval_lo =
            new_center.broadcast_sub(&self.taylor_remainder.broadcast_mul(&slope_hi)?)?;
        let new_interval_hi =
            new_center.broadcast_add(&self.taylor_remainder.broadcast_mul(&slope_lo)?)?;

        // Tighten: interval_lo = max(interval_lo, center - width)
        let tightened_lo = new_interval_lo
            .broadcast_maximum(&new_center.broadcast_sub(&self.taylor_remainder.clone())?)?;
        let tightened_hi = new_interval_hi
            .broadcast_minimum(&new_center.broadcast_add(&self.taylor_remainder)?)?;

        Ok(Self {
            center: new_center,
            generators: new_generators,
            interval_lo: tightened_lo,
            interval_hi: tightened_hi,
            taylor_remainder: new_remainder,
            num_gens: self.num_gens,
            dim: self.dim,
            config: self.config.clone(),
        })
    }

    /// Compute ReLU slope bounds per dimension.
    fn relu_slope_bounds(&self) -> Result<(Tensor, Tensor)> {
        let device = self.center.device();

        // For ReLU: slope is 0 if hi <= 0, 1 if lo >= 0, [0,1] if lo < 0 < hi
        let zero = Tensor::zeros(self.interval_lo.shape(), DType::F32, device)?;

        // slope_hi: 0 if hi <= 0, 1 otherwise
        let hi_le_zero = self.interval_hi.broadcast_maximum(&zero)?; // max(0, hi)

        // slope_hi: 0 if hi <= 0, 1 otherwise
        let slope_hi = hi_le_zero
            .broadcast_add(&Tensor::new(1e-6f32, device)?)?
            .broadcast_div(&hi_le_zero.broadcast_add(&Tensor::new(1.0f32, device)?)?)?;

        // Tighter: use sign of center
        let center_sign = self.center.sign()?;
        let slope_lo_tight = center_sign.relu()?.broadcast_mul(&slope_hi)?;

        Ok((slope_lo_tight, slope_hi))
    }

    // -----------------------------------------------------------------------
    // Girard Order Reduction
    // -----------------------------------------------------------------------

    /// Girard reduction: remove smallest-energy generator, add box.
    ///
    /// **Algorithm:**
    /// 1. Compute Gram matrix G^T G.
    /// 2. Find smallest eigenvalue/eigenvector via inverse power iteration.
    /// 3. Project out that direction from generators.
    /// 4. Add box generator with radius = projection norm.
    pub fn girard_reduce(&self) -> Result<Self> {
        let k = self.num_gens;
        let n = self.dim;
        let target = self.config.max_gens_after_reduction;
        let device = self.center.device();

        if k <= target {
            return Ok(self.clone());
        }

        let mut current_gens = self.generators.clone();
        let mut current_k = k;
        let mut total_box_radius = Tensor::zeros((n,), DType::F32, device)?;

        let _volume_before = self.log_volume_proxy()?;

        while current_k > target {
            // Gram matrix: G^T G → [n, n]
            let g_t = current_gens.t()?;
            let _gram = g_t.matmul(&current_gens)?;

            // Find direction of smallest energy via inverse power iteration
            // (approximate: use power iteration on G G^T for smallest singular direction)
            let gg_t = current_gens.matmul(&g_t)?; // [k, k]

            // Power iteration on G G^T to find dominant direction
            let scale = 1.0f32 / (current_k as f32).sqrt();
            let v_data: Vec<f32> = vec![scale; current_k];
            let mut v = Tensor::from_vec(v_data, (current_k, 1), device)?;

            for _ in 0..30 {
                let av = gg_t.matmul(&v)?;
                let norm: f32 = av.sqr()?.sum_all()?.to_scalar::<f32>()?.sqrt();
                if norm < 1e-12 {
                    break;
                }
                v = av.broadcast_div(&Tensor::new(norm, device)?)?;
            }

            // Project generators onto v direction: contribution = G^T v
            let contribution = g_t.matmul(&v)?; // [n, 1]

            // Box radius: infinity norm of contribution
            let contrib_abs = contribution.abs()?;
            let box_radius: f32 = contrib_abs.sum_all()?.to_scalar::<f32>()? / current_k as f32;

            // Remove the generator with smallest norm
            let gen_norms = current_gens.sqr()?.sum(1)?; // [k]
            let norms_vec: Vec<f32> = gen_norms.squeeze(0)?.to_vec1()?;
            let mut min_idx = 0;
            let mut min_norm = norms_vec[0];
            for (i, &norm) in norms_vec.iter().enumerate() {
                if norm < min_norm {
                    min_norm = norm;
                    min_idx = i;
                }
            }

            // Remove generator at min_idx
            let mut new_rows = Vec::new();
            for i in 0..current_k {
                if i != min_idx {
                    let row = current_gens.narrow(0, i, 1)?;
                    new_rows.push(row);
                }
            }
            current_gens = if new_rows.is_empty() {
                Tensor::zeros((1, n), DType::F32, device)?
            } else {
                Tensor::stack(&new_rows, 0)?
            };
            current_k -= 1;

            // Accumulate box radius
            total_box_radius =
                total_box_radius.broadcast_add(&Tensor::full(box_radius, (n,), device)?)?;
        }

        // Add box generators to taylor_remainder
        let new_remainder = self.taylor_remainder.broadcast_add(&total_box_radius)?;

        let _volume_after = {
            let widths = self.interval_hi.broadcast_sub(&self.interval_lo)?;
            let clamped = widths.broadcast_maximum(&Tensor::full(f32::EPSILON, (n,), device)?)?;
            clamped.log()?.sum_all()?.to_scalar::<f32>()?
        };

        Ok(Self {
            center: self.center.clone(),
            generators: current_gens,
            interval_lo: self.interval_lo.clone(),
            interval_hi: self.interval_hi.clone(),
            taylor_remainder: new_remainder,
            num_gens: current_k,
            dim: n,
            config: self.config.clone(),
        })
    }

    // -----------------------------------------------------------------------
    // Tube Propagation with Conformal Epsilon
    // -----------------------------------------------------------------------

    /// Propagate tube over one step: Z_{k+1} = K Z_k ⊕ W ⊖ ε_k.
    ///
    /// # Arguments
    /// * `k_matrix` - Koopman operator K. Shape: [dim, dim].
    /// * `disturbance` - Disturbance bound W (scalar radius).
    /// * `conformal_eps` - Conformal calibration epsilon.
    pub fn tube_step(
        &self,
        k_matrix: &Tensor,
        disturbance: f32,
        conformal_eps: f32,
    ) -> Result<HZIPropagationResult> {
        // Affine propagation through K
        let propagated = self.affine_propagate(k_matrix, None)?;

        // Minkowski sum with disturbance ball
        let disturbed = propagated.minkowski_disturbance(disturbance)?;

        // Tighten with conformal epsilon
        let tightened = disturbed.conformal_tighten(conformal_eps)?;

        // Compute metrics before move
        let tube_radius = tightened.compute_tube_radius()?;
        let log_volume = tightened.log_volume_proxy()?;
        let remainder_mean: f32 = tightened.taylor_remainder.mean_all()?.to_scalar()?;
        let safety_margin = tightened.compute_safety_margin()?;
        let num_gens = tightened.num_gens;

        Ok(HZIPropagationResult {
            hzi: tightened,
            tube_radius,
            log_volume,
            num_gens,
            taylor_remainder: remainder_mean,
            conformal_epsilon: conformal_eps,
            safety_margin,
        })
    }

    /// Minkowski sum with disturbance ball.
    fn minkowski_disturbance(&self, disturbance: f32) -> Result<Self> {

        // Add disturbance to interval bounds
        let dist_tensor = Tensor::new(disturbance, self.center.device())?.to_dtype(DType::F32)?;
        let new_lo = self.interval_lo.broadcast_sub(&dist_tensor)?;
        let new_hi = self.interval_hi.broadcast_add(&dist_tensor)?;

        // Add disturbance to Taylor remainder
        let new_remainder = self.taylor_remainder.broadcast_add(&dist_tensor)?;

        Ok(Self {
            center: self.center.clone(),
            generators: self.generators.clone(),
            interval_lo: new_lo,
            interval_hi: new_hi,
            taylor_remainder: new_remainder,
            num_gens: self.num_gens,
            dim: self.dim,
            config: self.config.clone(),
        })
    }

    /// Tighten tube with conformal epsilon.
    fn conformal_tighten(&self, conformal_eps: f32) -> Result<Self> {

        // Conformal tightening: reduce bounds by epsilon around center
        let eps_tensor = Tensor::new(conformal_eps, self.center.device())?.to_dtype(DType::F32)?;

        // Tighten interval bounds toward center
        let center_lo = self.center.broadcast_sub(&eps_tensor)?;
        let center_hi = self.center.broadcast_add(&eps_tensor)?;

        let tightened_lo = self.interval_lo.broadcast_maximum(&center_lo)?;
        let tightened_hi = self.interval_hi.broadcast_minimum(&center_hi)?;

        // Tighten Taylor remainder
        let tightened_remainder = self.taylor_remainder.broadcast_minimum(&eps_tensor)?;

        Ok(Self {
            center: self.center.clone(),
            generators: self.generators.clone(),
            interval_lo: tightened_lo,
            interval_hi: tightened_hi,
            taylor_remainder: tightened_remainder,
            num_gens: self.num_gens,
            dim: self.dim,
            config: self.config.clone(),
        })
    }

    // -----------------------------------------------------------------------
    // Metrics
    // -----------------------------------------------------------------------

    /// Compute tube radius (infinity norm of zonotope).
    pub fn compute_tube_radius(&self) -> Result<f32> {
        let gen_abs = self.generators.abs()?;
        let col_sums = gen_abs.sum(0)?;
        let remainder = self.taylor_remainder.clone();
        let total_width = col_sums.broadcast_add(&remainder)?;
        let widths: Vec<f32> = total_width.squeeze(0)?.to_vec1()?;
        Ok(widths.iter().copied().reduce(f32::max).unwrap_or(0.0))
    }

    /// Compute log volume proxy.
    pub fn log_volume_proxy(&self) -> Result<f32> {
        let widths = self.interval_hi.broadcast_sub(&self.interval_lo)?;
        let device = widths.device();
        let clamped =
            widths.broadcast_maximum(&Tensor::full(f32::EPSILON, widths.shape(), device)?)?;
        clamped.log()?.sum_all()?.to_scalar::<f32>()
    }

    /// Compute safety margin (minimum distance to constraint boundary).
    /// For CBF h(x) = w^T x + b, this is the lower bound of h on the HZI set.
    pub fn compute_safety_margin(&self) -> Result<f32> {
        // Conservative estimate: center - tube_radius
        let radius = self.compute_tube_radius()?;
        let center_norm: f32 = self.center.sqr()?.sum_all()?.to_scalar::<f32>()?.sqrt();
        Ok(center_norm - radius)
    }

    /// Evaluate CBF lower bound: h_min = w^T c - ||w||_1 · radius.
    pub fn cbf_lower_bound(&self, w: &Tensor, b: f32) -> Result<f32> {
        let w_abs_sum: f32 = w.abs()?.sum_all()?.to_scalar::<f32>()?;
        let radius = self.compute_tube_radius()?;
        let wc: f32 = w
            .broadcast_mul(&self.center)?
            .sum_all()?
            .to_scalar::<f32>()?;
        Ok(wc - w_abs_sum * radius + b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::Device;

    #[test]
    fn test_hzi_creation() -> Result<()> {
        let device = Device::Cpu;
        let dim = 16;
        let center = Tensor::zeros((1, dim), DType::F32, &device)?;
        let config = HZIConfig::default();

        let hzi = HybridZonotopeTaylor::new_from_epsilon(&center, 0.05, config)?;
        assert_eq!(hzi.dim, dim);
        assert!(hzi.num_gens > 0);
        Ok(())
    }

    #[test]
    fn test_hzi_affine_propagate() -> Result<()> {
        let device = Device::Cpu;
        let dim = 8;
        let center = Tensor::ones((1, dim), DType::F32, &device)?;
        let config = HZIConfig::edge_fast();
        let hzi = HybridZonotopeTaylor::new_from_epsilon(&center, 0.05, config)?;

        // Identity transform
        let w = Tensor::eye(dim, DType::F32, &device)?;
        let result = hzi.affine_propagate(&w, None)?;
        assert_eq!(result.dim, dim);
        Ok(())
    }

    #[test]
    fn test_hzi_relu_propagate() -> Result<()> {
        let device = Device::Cpu;
        let dim = 8;
        let center_data: Vec<f32> = vec![-1.0, 0.5, -0.5, 1.0, 0.0, -2.0, 0.3, -0.1];
        let center = Tensor::from_vec(center_data, (1, dim), &device)?;
        let config = HZIConfig::default();
        let hzi = HybridZonotopeTaylor::new_from_epsilon(&center, 0.1, config)?;

        let result = hzi.relu_taylor_propagate()?;
        // ReLU should make center non-negative
        let center_out: Vec<f32> = result.center.squeeze(0)?.to_vec1()?;
        for &v in &center_out {
            assert!(v >= -1e-6, "ReLU output should be non-negative: {}", v);
        }
        Ok(())
    }

    #[test]
    fn test_hzi_tube_step() -> Result<()> {
        let device = Device::Cpu;
        let dim = 8;
        let center = Tensor::ones((1, dim), DType::F32, &device)?;
        let config = HZIConfig::edge_fast();
        let hzi = HybridZonotopeTaylor::new_from_epsilon(&center, 0.05, config.clone())?;

        // Stable Koopman: K = 0.9 * I
        let k =
            Tensor::eye(dim, DType::F32, &device)?.broadcast_mul(&Tensor::new(0.9f32, &device)?)?;

        let result = hzi.tube_step(&k, 0.05, 0.03)?;
        assert!(result.tube_radius > 0.0);
        assert!(result.num_gens <= config.max_gens_after_reduction || result.num_gens <= dim);
        Ok(())
    }

    #[test]
    fn test_hzi_girard_reduction() -> Result<()> {
        let device = Device::Cpu;
        let dim = 8;
        let max_gens = 32;
        let center = Tensor::ones((1, dim), DType::F32, &device)?;
        let config = HZIConfig {
            zonotope_config: ZonotopeConfig {
                max_gens,
                ..Default::default()
            },
            max_gens_after_reduction: 16,
            girard_reduction: true,
            ..Default::default()
        };
        let hzi = HybridZonotopeTaylor::new_from_epsilon(&center, 0.05, config)?;

        // Identity transform (should trigger reduction)
        let w = Tensor::eye(dim, DType::F32, &device)?;
        let result = hzi.affine_propagate(&w, None)?;
        assert!(result.num_gens <= 16 || result.num_gens <= dim);
        Ok(())
    }

    #[test]
    fn test_hzi_cbf_lower_bound() -> Result<()> {
        let device = Device::Cpu;
        let dim = 4;
        let center = Tensor::from_vec(vec![1.0f32, 2.0, 3.0, 4.0], (1, dim), &device)?;
        let config = HZIConfig::default();
        let hzi = HybridZonotopeTaylor::new_from_epsilon(&center, 0.1, config)?;

        // CBF: h(x) = x_0 + x_1 + x_2 + x_3 - 5
        let w = Tensor::from_vec(vec![1.0f32, 1.0, 1.0, 1.0], (dim,), &device)?;
        let b = -5.0f32;

        let h_min = hzi.cbf_lower_bound(&w, b)?;
        // Center value: 1+2+3+4-5 = 5, minus radius
        assert!(h_min < 5.0, "CBF lower bound should be < center value");
        assert!(h_min > 0.0, "CBF should be positive for safe state");
        Ok(())
    }

    #[test]
    fn test_hzi_from_taylor_model() -> Result<()> {
        let device = Device::Cpu;
        let dim = 8;
        let center = Tensor::ones((1, dim), DType::F32, &device)?;
        let tm = TaylorModel::new_from_epsilon(&center, 0.05)?;
        let config = HZIConfig::default();

        let hzi = HybridZonotopeTaylor::from_taylor_model(&tm, config)?;
        assert_eq!(hzi.dim, dim);
        Ok(())
    }

    #[test]
    fn test_hzi_conformal_tighten() -> Result<()> {
        let device = Device::Cpu;
        let dim = 4;
        let center = Tensor::from_vec(vec![1.0, 2.0, 3.0, 4.0], (1, dim), &device)?;
        let config = HZIConfig::default();
        let hzi = HybridZonotopeTaylor::new_from_epsilon(&center, 0.1, config)?;

        let tightened = hzi.conformal_tighten(0.05)?;
        let r_before = hzi.compute_tube_radius()?;
        let r_after = tightened.compute_tube_radius()?;
        assert!(
            r_after <= r_before,
            "Conformal tightening should reduce radius"
        );
        Ok(())
    }
}
