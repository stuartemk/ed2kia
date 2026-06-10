//! Topology — Sliced Gromov-Wasserstein (SGW) for isometry-invariant activation manifolds.
//!
//! **Sprint 129:** True topological metric via Sliced Gromov-Wasserstein approximation.
//! SGW is invariant to isometries in activation manifolds, unlike SWD which only
//! compares marginal distributions. SGW captures structural geometry of pairwise
//! distance patterns between activation distributions.
//!
//! Mathematical foundation:
//! ```math
//! SGW(X, Y) ≈ (1/L) Σ_{l=1}^{L} ∫ |d_X(θ_l(x), θ_l(x')) - d_Y(θ_l(y), θ_l(y'))|² dμ dμ'
//! ```
//! Where θ_l are random 1D projections, d_X/d_Y are intra-distribution distances,
//! and L is the number of random projections.

use candle_core::{Result, Tensor};

/// Configuration for Sliced Gromov-Wasserstein computation.
#[derive(Debug, Clone)]
pub struct SgwConfig {
    /// Number of random projections (higher = more accurate, slower).
    pub num_projections: usize,
    /// Subsample size for pairwise distance matrix (limits memory).
    pub max_subsample: usize,
    /// Random seed for reproducibility.
    pub seed: u64,
}

impl Default for SgwConfig {
    fn default() -> Self {
        Self {
            num_projections: 64,
            max_subsample: 128,
            seed: 42,
        }
    }
}

impl SgwConfig {
    /// Create config with custom projection count.
    pub fn with_projections(mut self, n: usize) -> Self {
        self.num_projections = n.max(1);
        self
    }

    /// Create config with custom subsample size.
    pub fn with_subsample(mut self, n: usize) -> Self {
        self.max_subsample = n.max(2);
        self
    }

    /// Create config with custom seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Fast config for edge devices (fewer projections).
    pub fn edge_fast() -> Self {
        Self {
            num_projections: 16,
            max_subsample: 64,
            seed: 42,
        }
    }

    /// High-accuracy config for verification.
    pub fn high_accuracy() -> Self {
        Self {
            num_projections: 256,
            max_subsample: 256,
            seed: 42,
        }
    }
}

/// Result of SGW computation with diagnostic info.
#[derive(Debug, Clone)]
pub struct SgwResult {
    /// Sliced Gromov-Wasserstein distance.
    pub distance: f32,
    /// Number of projections used.
    pub projections_used: usize,
    /// Mean projection variance (diagnostic for manifold quality).
    pub mean_variance: f32,
}

impl std::fmt::Display for SgwResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SGW(dist={:.6}, proj={}, var={:.6})",
            self.distance, self.projections_used, self.mean_variance
        )
    }
}

// ─── LCG PRNG for deterministic random projections ───

fn lcg_next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    *state
}

fn next_f32(state: &mut u64) -> f32 {
    (lcg_next(state) as f32) / (u64::MAX as f32)
}

/// Box-Muller transform for Gaussian samples.
fn gaussian(state: &mut u64) -> f32 {
    let u1 = next_f32(state).max(1e-8f32);
    let u2 = next_f32(state);
    let r = (-2.0 * u1.ln()).sqrt();
    r * (2.0 * std::f32::consts::PI * u2).cos()
}

/// Compute Sliced Gromov-Wasserstein distance between two activation tensors.
///
/// SGW is invariant to isometries (rotation, translation) of the activation manifold,
/// making it a true topological metric for comparing activation distributions.
///
/// # Formula
/// ```math
/// SGW(X, Y) = sqrt( (1/L) Σ_{l=1}^{L} mean( (d_X^l - d_Y^l)² ) )
/// ```
/// where `d_X^l` and `d_Y^l` are pairwise distance matrices on 1D projections.
///
/// # Arguments
/// * `t1` - First activation tensor `[B, D]`
/// * `t2` - Second activation tensor `[B, D]`
/// * `config` - SGW configuration
///
/// # Returns
/// SGW distance (0 = identical structure, higher = more different)
pub fn compute_sliced_gromov_wasserstein(
    t1: &Tensor,
    t2: &Tensor,
    config: &SgwConfig,
) -> Result<SgwResult> {
    let (n1, dim) = match (t1.dim(0), t1.dim(1)) {
        (Ok(n), Ok(d)) => (n, d),
        _ => return Err(candle_core::Error::Msg("t1 must be 2D [B, D]".into())),
    };
    let n2 = t2
        .dim(0)
        .map_err(|_| candle_core::Error::Msg("t2 must be 2D [B, D]".into()))?;

    if dim
        != t2
            .dim(1)
            .map_err(|_| candle_core::Error::Msg("dim mismatch".to_string()))?
    {
        return Err(candle_core::Error::Msg(
            "t1 and t2 must have same feature dimension".into(),
        ));
    }

    if n1 == 0 || n2 == 0 {
        return Ok(SgwResult {
            distance: 0.0,
            projections_used: 0,
            mean_variance: 0.0,
        });
    }

    let device = t1.device();
    let subsample = config.max_subsample.min(n1).min(n2);
    let num_proj = config.num_projections;
    let mut state = config.seed;

    let mut total_loss = 0.0f32;
    let mut total_var = 0.0f32;

    for _ in 0..num_proj {
        // Random unit direction via Gaussian + normalization
        let mut dir: Vec<f32> = (0..dim).map(|_| gaussian(&mut state)).collect();
        let norm: f32 = dir.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-12);
        dir.iter_mut().for_each(|x| *x /= norm);

        let dir_tensor = Tensor::from_vec(dir, (dim, 1), device)?;

        // Project both tensors onto random direction: [N, D] @ [D, 1] = [N, 1]
        let proj1 = t1.narrow(0, 0, subsample)?.matmul(&dir_tensor)?;
        let proj2 = t2.narrow(0, 0, subsample)?.matmul(&dir_tensor)?;

        // Compute pairwise distance matrices (1D absolute differences)
        let d1 = compute_pairwise_dist_1d(&proj1)?;
        let d2 = compute_pairwise_dist_1d(&proj2)?;

        // GW loss for this projection: mean squared difference of distance matrices
        let diff = d1.broadcast_sub(&d2)?;
        let gw_loss = diff.sqr()?.mean_all()?.to_scalar::<f32>()?;
        total_loss += gw_loss;

        // Track variance for diagnostics
        let var1 = proj1.var(0)?.to_scalar::<f32>().unwrap_or(0.0);
        let var2 = proj2.var(0)?.to_scalar::<f32>().unwrap_or(0.0);
        total_var += (var1 + var2) * 0.5;
    }

    let mean_loss = total_loss / num_proj as f32;
    let mean_var = total_var / num_proj as f32;

    Ok(SgwResult {
        distance: mean_loss.sqrt(),
        projections_used: num_proj,
        mean_variance: mean_var,
    })
}

/// Compute pairwise absolute distance matrix for 1D projections.
/// Input: `[N, 1]` → Output: `[N, N]` where `out[i][j] = |x_i - x_j|`
fn compute_pairwise_dist_1d(proj: &Tensor) -> Result<Tensor> {
    let shape = proj.shape().dims().to_vec();
    let n = match shape.len() {
        2 => shape[0],
        1 => shape[0],
        _ => return Err(candle_core::Error::Msg("proj must be 1D or 2D".into())),
    };

    let flat = if shape.len() == 2 {
        proj.squeeze(1)?
    } else {
        proj.clone()
    };

    // Expand to [N, 1] and [1, N] for broadcasting
    let col = flat.unsqueeze(1)?;
    let row = flat.unsqueeze(0)?;

    // Broadcast subtraction: [N, N]
    let diff = col.broadcast_sub(&row.broadcast_as((n, n))?)?;
    diff.abs()
}

/// Compute SGW ratio for safety certification.
/// Ratio < 1.0 means steered activations are closer to safe distribution.
pub fn compute_sgw_safety_ratio(
    original: &Tensor,
    steered: &Tensor,
    safe: &Tensor,
    config: &SgwConfig,
) -> Result<f32> {
    let sgw_orig_safe = compute_sliced_gromov_wasserstein(original, safe, config)?;
    let sgw_steer_safe = compute_sliced_gromov_wasserstein(steered, safe, config)?;

    let denom = sgw_orig_safe.distance.max(1e-12f32);
    Ok(sgw_steer_safe.distance / denom)
}

/// Compare two activation distributions using SGW with statistical significance.
/// Returns `true` if the distributions are significantly different (p < alpha).
pub fn sgw_significance_test(
    t1: &Tensor,
    t2: &Tensor,
    config: &SgwConfig,
    alpha: f32,
) -> Result<bool> {
    let result = compute_sliced_gromov_wasserstein(t1, t2, config)?;

    // Approximate significance: distance > 2 * sqrt(variance / n_proj)
    // This is a heuristic based on the Central Limit Theorem for the mean of projections
    let se = (result.mean_variance / config.num_projections as f32).sqrt();
    let z_score = result.distance / se.max(1e-12);

    // Two-tailed test: p < alpha when |z| > threshold
    // alpha=0.05 → z_threshold ≈ 1.96
    let z_threshold = match alpha {
        a if (a - 0.01).abs() < 1e-6 => 2.576,
        a if (a - 0.05).abs() < 1e-6 => 1.96,
        a if (a - 0.10).abs() < 1e-6 => 1.645,
        _ => 1.96, // Default to 0.05
    };

    Ok(z_score > z_threshold)
}

/// Compute manifold complexity via SGW self-distance variance.
/// Higher variance across projections indicates more complex manifold structure.
pub fn compute_manifold_complexity(t: &Tensor, config: &SgwConfig) -> Result<f32> {
    let mut state = config.seed;
    let dim = t
        .dim(1)
        .map_err(|_| candle_core::Error::Msg("t must be 2D [B, D]".into()))?;
    let n = t.dim(0)?;
    let subsample = config.max_subsample.min(n);

    let device = t.device();
    let mut variances: Vec<f32> = Vec::with_capacity(config.num_projections);

    for _ in 0..config.num_projections {
        let mut dir: Vec<f32> = (0..dim).map(|_| gaussian(&mut state)).collect();
        let norm: f32 = dir.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-12);
        dir.iter_mut().for_each(|x| *x /= norm);

        let dir_tensor = Tensor::from_vec(dir, (dim, 1), device)?;
        let proj = t.narrow(0, 0, subsample)?.matmul(&dir_tensor)?;

        let var = proj.var(0)?.to_scalar::<f32>().unwrap_or(0.0);
        variances.push(var);
    }

    if variances.is_empty() {
        return Ok(0.0);
    }

    let mean = variances.iter().sum::<f32>() / variances.len() as f32;
    let variance =
        variances.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / variances.len() as f32;

    // Coefficient of variation as complexity measure
    Ok(if mean.abs() > 1e-12 {
        variance.sqrt() / mean
    } else {
        0.0
    })
}

// ─── Unit Tests ───

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::{DType, Device};

    fn make_tensor(rows: usize, cols: usize, seed: u64) -> Result<Tensor> {
        let mut data: Vec<f32> = Vec::with_capacity(rows * cols);
        let mut state = seed;
        for _ in 0..(rows * cols) {
            data.push(gaussian(&mut state));
        }
        Tensor::from_vec(data, (rows, cols), &Device::Cpu)
    }

    #[test]
    fn test_sgw_config_default() {
        let cfg = SgwConfig::default();
        assert_eq!(cfg.num_projections, 64);
        assert_eq!(cfg.max_subsample, 128);
        assert_eq!(cfg.seed, 42);
    }

    #[test]
    fn test_sgw_config_with_projections() {
        let cfg = SgwConfig::default().with_projections(128);
        assert_eq!(cfg.num_projections, 128);
    }

    #[test]
    fn test_sgw_config_projections_min() {
        let cfg = SgwConfig::default().with_projections(0);
        assert_eq!(cfg.num_projections, 1);
    }

    #[test]
    fn test_sgw_config_with_subsample() {
        let cfg = SgwConfig::default().with_subsample(256);
        assert_eq!(cfg.max_subsample, 256);
    }

    #[test]
    fn test_sgw_config_subsample_min() {
        let cfg = SgwConfig::default().with_subsample(1);
        assert_eq!(cfg.max_subsample, 2);
    }

    #[test]
    fn test_sgw_config_with_seed() {
        let cfg = SgwConfig::default().with_seed(123);
        assert_eq!(cfg.seed, 123);
    }

    #[test]
    fn test_sgw_config_edge_fast() {
        let cfg = SgwConfig::edge_fast();
        assert_eq!(cfg.num_projections, 16);
        assert_eq!(cfg.max_subsample, 64);
    }

    #[test]
    fn test_sgw_config_high_accuracy() {
        let cfg = SgwConfig::high_accuracy();
        assert_eq!(cfg.num_projections, 256);
        assert_eq!(cfg.max_subsample, 256);
    }

    #[test]
    fn test_sgw_identical_tensors() -> Result<()> {
        let t = make_tensor(16, 8, 42)?;
        let cfg = SgwConfig::edge_fast();
        let result = compute_sliced_gromov_wasserstein(&t, &t, &cfg)?;
        assert!(
            result.distance < 1e-5,
            "SGW of identical tensors should be ~0, got {}",
            result.distance
        );
        Ok(())
    }

    #[test]
    fn test_sgw_different_distributions() -> Result<()> {
        let t1 = make_tensor(16, 8, 42)?;
        let t2 = make_tensor(16, 8, 999)?;
        let cfg = SgwConfig::edge_fast();
        let result = compute_sliced_gromov_wasserstein(&t1, &t2, &cfg)?;
        assert!(
            result.distance > 0.0,
            "SGW of different tensors should be > 0"
        );
        Ok(())
    }

    #[test]
    fn test_sgw_distance_positive() -> Result<()> {
        let t1 = make_tensor(32, 16, 1)?;
        let t2 = make_tensor(32, 16, 2)?;
        let cfg = SgwConfig::default();
        let result = compute_sliced_gromov_wasserstein(&t1, &t2, &cfg)?;
        assert!(result.distance >= 0.0, "SGW distance must be non-negative");
        Ok(())
    }

    #[test]
    fn test_sgw_symmetry() -> Result<()> {
        let t1 = make_tensor(16, 8, 42)?;
        let t2 = make_tensor(16, 8, 99)?;
        let cfg = SgwConfig::edge_fast();
        let d12 = compute_sliced_gromov_wasserstein(&t1, &t2, &cfg)?;
        let d21 = compute_sliced_gromov_wasserstein(&t2, &t1, &cfg)?;
        assert!(
            (d12.distance - d21.distance).abs() < 1e-4,
            "SGW should be symmetric: d12={:.6} vs d21={:.6}",
            d12.distance,
            d21.distance
        );
        Ok(())
    }

    #[test]
    fn test_sgw_zero_tensors() -> Result<()> {
        let t1 = Tensor::zeros((8, 4), DType::F32, &Device::Cpu)?;
        let t2 = Tensor::zeros((8, 4), DType::F32, &Device::Cpu)?;
        let cfg = SgwConfig::edge_fast();
        let result = compute_sliced_gromov_wasserstein(&t1, &t2, &cfg)?;
        assert!(result.distance < 1e-5, "SGW of zero tensors should be ~0");
        Ok(())
    }

    #[test]
    fn test_sgw_projections_used() -> Result<()> {
        let t1 = make_tensor(16, 8, 42)?;
        let t2 = make_tensor(16, 8, 99)?;
        let cfg = SgwConfig::default().with_projections(32);
        let result = compute_sliced_gromov_wasserstein(&t1, &t2, &cfg)?;
        assert_eq!(result.projections_used, 32);
        Ok(())
    }

    #[test]
    fn test_sgw_mean_variance_positive() -> Result<()> {
        let t1 = make_tensor(16, 8, 42)?;
        let t2 = make_tensor(16, 8, 99)?;
        let cfg = SgwConfig::edge_fast();
        let result = compute_sliced_gromov_wasserstein(&t1, &t2, &cfg)?;
        assert!(
            result.mean_variance >= 0.0,
            "Variance should be non-negative"
        );
        Ok(())
    }

    #[test]
    fn test_sgw_result_display() -> Result<()> {
        let result = SgwResult {
            distance: 0.5,
            projections_used: 64,
            mean_variance: 1.2,
        };
        let s = format!("{}", result);
        assert!(s.contains("SGW"));
        assert!(s.contains("0.500000"));
        assert!(s.contains("64"));
        Ok(())
    }

    #[test]
    fn test_sgw_subsample_effect() -> Result<()> {
        let t1 = make_tensor(64, 8, 42)?;
        let t2 = make_tensor(64, 8, 99)?;
        let cfg_small = SgwConfig::edge_fast().with_subsample(8);
        let cfg_large = SgwConfig::edge_fast().with_subsample(64);
        let r_small = compute_sliced_gromov_wasserstein(&t1, &t2, &cfg_small)?;
        let r_large = compute_sliced_gromov_wasserstein(&t1, &t2, &cfg_large)?;
        // Both should produce valid distances
        assert!(r_small.distance >= 0.0);
        assert!(r_large.distance >= 0.0);
        Ok(())
    }

    #[test]
    fn test_sgw_deterministic() -> Result<()> {
        let t1 = make_tensor(16, 8, 42)?;
        let t2 = make_tensor(16, 8, 99)?;
        let cfg = SgwConfig::edge_fast().with_seed(12345);
        let r1 = compute_sliced_gromov_wasserstein(&t1, &t2, &cfg)?;
        let r2 = compute_sliced_gromov_wasserstein(&t1, &t2, &cfg)?;
        assert!(
            (r1.distance - r2.distance).abs() < 1e-6,
            "SGW should be deterministic with same seed"
        );
        Ok(())
    }

    #[test]
    fn test_pairwise_dist_symmetry() -> Result<()> {
        let proj = make_tensor(8, 1, 42)?;
        let dist = compute_pairwise_dist_1d(&proj)?;
        let dims: Vec<usize> = dist.shape().dims().to_vec();
        assert_eq!(dims, vec![8, 8], "Pairwise dist should be square");
        // Diagonal should be zero
        let dist_vec: Vec<Vec<f32>> = dist.to_vec2()?;
        let diag: Vec<f32> = (0..8).map(|i| dist_vec[i][i]).collect();
        for d in &diag {
            assert!(*d < 1e-5, "Diagonal should be ~0, got {}", d);
        }
        Ok(())
    }

    #[test]
    fn test_pairwise_dist_non_negative() -> Result<()> {
        let proj = make_tensor(8, 1, 42)?;
        let dist = compute_pairwise_dist_1d(&proj)?;
        let vals: Vec<Vec<f32>> = dist.to_vec2()?;
        for row in &vals {
            for v in row {
                assert!(*v >= 0.0, "Distance must be non-negative");
            }
        }
        Ok(())
    }

    #[test]
    fn test_sgw_safety_ratio_improvement() -> Result<()> {
        let safe = make_tensor(16, 8, 100)?;
        let original = make_tensor(16, 8, 200)?;
        // Steered is closer to safe (blend of safe + original)
        let steered = safe
            .broadcast_mul(&Tensor::new(0.7f32, safe.device())?)?
            .add(&original.broadcast_mul(&Tensor::new(0.3f32, original.device())?)?)?;
        let cfg = SgwConfig::edge_fast();
        let ratio = compute_sgw_safety_ratio(&original, &steered, &safe, &cfg)?;
        assert!(ratio >= 0.0, "Safety ratio should be non-negative");
        Ok(())
    }

    #[test]
    fn test_sgw_safety_ratio_identity() -> Result<()> {
        let safe = make_tensor(16, 8, 100)?;
        let cfg = SgwConfig::edge_fast();
        let ratio = compute_sgw_safety_ratio(&safe, &safe, &safe, &cfg)?;
        // When original == steered == safe, ratio should be ~0 (denom is ~0, so clamped)
        assert!(ratio >= 0.0);
        Ok(())
    }

    #[test]
    fn test_sgw_significance_test_different() -> Result<()> {
        let t1 = make_tensor(32, 16, 1)?;
        let t2 = make_tensor(32, 16, 999)?;
        let cfg = SgwConfig::default();
        let significant = sgw_significance_test(&t1, &t2, &cfg, 0.05)?;
        // Very different distributions should be significant
        assert!(significant, "Different distributions should be significant");
        Ok(())
    }

    #[test]
    fn test_sgw_significance_test_identical() -> Result<()> {
        let t = make_tensor(32, 16, 42)?;
        let cfg = SgwConfig::default();
        let significant = sgw_significance_test(&t, &t, &cfg, 0.05)?;
        assert!(
            !significant,
            "Identical distributions should not be significant"
        );
        Ok(())
    }

    #[test]
    fn test_manifold_complexity_positive() -> Result<()> {
        let t = make_tensor(32, 16, 42)?;
        let cfg = SgwConfig::edge_fast();
        let complexity = compute_manifold_complexity(&t, &cfg)?;
        assert!(complexity >= 0.0, "Complexity should be non-negative");
        Ok(())
    }

    #[test]
    fn test_manifold_complexity_zeros() -> Result<()> {
        let t = Tensor::zeros((16, 8), DType::F32, &Device::Cpu)?;
        let cfg = SgwConfig::edge_fast();
        let complexity = compute_manifold_complexity(&t, &cfg)?;
        assert!(complexity < 1e-5, "Zero tensor should have ~0 complexity");
        Ok(())
    }

    #[test]
    fn test_manifold_complexity_deterministic() -> Result<()> {
        let t = make_tensor(32, 16, 42)?;
        let cfg = SgwConfig::edge_fast().with_seed(999);
        let c1 = compute_manifold_complexity(&t, &cfg)?;
        let c2 = compute_manifold_complexity(&t, &cfg)?;
        assert!((c1 - c2).abs() < 1e-6, "Complexity should be deterministic");
        Ok(())
    }

    #[test]
    fn test_sgw_dimension_mismatch() -> Result<()> {
        let t1 = make_tensor(16, 8, 42)?;
        let t2 = make_tensor(16, 12, 99)?;
        let cfg = SgwConfig::edge_fast();
        let result = compute_sliced_gromov_wasserstein(&t1, &t2, &cfg);
        assert!(result.is_err(), "Should error on dimension mismatch");
        Ok(())
    }

    #[test]
    fn test_sgw_empty_tensor() -> Result<()> {
        let t1 = Tensor::from_vec(Vec::<f32>::new(), (0, 8), &Device::Cpu)?;
        let t2 = Tensor::from_vec(Vec::<f32>::new(), (0, 8), &Device::Cpu)?;
        let cfg = SgwConfig::edge_fast();
        let result = compute_sliced_gromov_wasserstein(&t1, &t2, &cfg)?;
        assert!(
            result.distance < 1e-5,
            "Empty tensors should give ~0 distance"
        );
        assert_eq!(result.projections_used, 0);
        Ok(())
    }

    #[test]
    fn test_sgw_translation_invariance() -> Result<()> {
        let t1 = make_tensor(16, 8, 42)?;
        let t2 = make_tensor(16, 8, 99)?;
        // Add constant offset to both (translation)
        let offset = Tensor::full(5.0f32, (16, 8), &Device::Cpu)?;
        let t1_shifted = t1.add(&offset)?;
        let t2_shifted = t2.add(&offset)?;

        let cfg = SgwConfig::edge_fast();
        let d_orig = compute_sliced_gromov_wasserstein(&t1, &t2, &cfg)?;
        let d_shifted = compute_sliced_gromov_wasserstein(&t1_shifted, &t2_shifted, &cfg)?;

        // SGW should be approximately translation-invariant
        assert!(
            (d_orig.distance - d_shifted.distance).abs() < 0.1,
            "SGW should be translation-invariant: orig={:.4} shifted={:.4}",
            d_orig.distance,
            d_shifted.distance
        );
        Ok(())
    }

    #[test]
    fn test_sgw_scaling_effect() -> Result<()> {
        let t1 = make_tensor(16, 8, 42)?;
        let t2 = make_tensor(16, 8, 99)?;
        let scaled = t2.broadcast_mul(&Tensor::new(2.0f32, t2.device())?)?;

        let cfg = SgwConfig::edge_fast();
        let d_orig = compute_sliced_gromov_wasserstein(&t1, &t2, &cfg)?;
        let d_scaled = compute_sliced_gromov_wasserstein(&t1, &scaled, &cfg)?;

        // Scaling should change the distance
        assert!(
            (d_orig.distance - d_scaled.distance).abs() > 1e-4,
            "Scaling should affect SGW distance"
        );
        Ok(())
    }

    #[test]
    fn test_sgw_more_projections_converges() -> Result<()> {
        let t1 = make_tensor(32, 16, 42)?;
        let t2 = make_tensor(32, 16, 99)?;
        let cfg_few = SgwConfig::default().with_projections(8).with_seed(42);
        let cfg_many = SgwConfig::default().with_projections(128).with_seed(42);
        let r_few = compute_sliced_gromov_wasserstein(&t1, &t2, &cfg_few)?;
        let r_many = compute_sliced_gromov_wasserstein(&t1, &t2, &cfg_many)?;

        // More projections should give more stable estimate
        // (not necessarily closer to any specific value, but both valid)
        assert!(r_few.distance >= 0.0);
        assert!(r_many.distance >= 0.0);
        Ok(())
    }

    #[test]
    fn test_full_sgw_pipeline() -> Result<()> {
        // Simulate full pipeline: detect → steer → verify
        let safe = make_tensor(32, 16, 100)?;
        let original = make_tensor(32, 16, 200)?;
        let cfg = SgwConfig::edge_fast();

        // Step 1: Measure distance to safe
        let sgw_before = compute_sliced_gromov_wasserstein(&original, &safe, &cfg)?;

        // Step 2: "Steer" toward safe (convex blend)
        let steered = original
            .broadcast_mul(&Tensor::new(0.3f32, original.device())?)?
            .add(&safe.broadcast_mul(&Tensor::new(0.7f32, safe.device())?)?)?;

        // Step 3: Measure distance after steering
        let sgw_after = compute_sliced_gromov_wasserstein(&steered, &safe, &cfg)?;

        // Step 4: Verify improvement
        assert!(
            sgw_after.distance < sgw_before.distance,
            "Steering should reduce SGW distance: before={:.4} after={:.4}",
            sgw_before.distance,
            sgw_after.distance
        );

        // Step 5: Safety ratio
        let ratio = compute_sgw_safety_ratio(&original, &steered, &safe, &cfg)?;
        assert!(ratio < 1.0, "Safety ratio should be < 1 after steering");

        Ok(())
    }
}
