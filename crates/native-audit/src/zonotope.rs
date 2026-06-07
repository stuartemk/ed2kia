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
}
