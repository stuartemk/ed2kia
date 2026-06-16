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
    /// Wasserstein ambiguity radius ε_W for robust RLS updates.
    /// When innovation ||e|| > ε_W, the update is attenuated by
    /// weight w = min(1, ε_W / ||e||) to reject adversarial distribution shifts.
    /// Set to f64::MAX to disable Wasserstein robustness (standard RLS).
    /// Default: 0.1 (moderate robustness against outliers).
    pub wasserstein_radius: f64,
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
            wasserstein_radius: 0.1,
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
            wasserstein_radius: 0.2,
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
            wasserstein_radius: 0.05,
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

    /// Set custom Wasserstein ambiguity radius.
    pub fn with_wasserstein_radius(mut self, radius: f64) -> Self {
        self.wasserstein_radius = radius.max(0.0);
        self
    }
}

// ---------------------------------------------------------------------------
// Sprint 168 (v16.8.0) — Dimensional Collapse Config
// ---------------------------------------------------------------------------

/// Core selection method for dimensional collapse.
///
/// Determines how to rank SAE features when extracting the top-k
/// topological core for Koopman control in reduced dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreSelectionMethod {
    /// Select by L2 norm of SAE activations (energy-based).
    Norm,
    /// Select by SAE sparsity weights (importance-based).
    Sparsity,
    /// Weighted combination of norm + sparsity (default: 0.7 norm + 0.3 sparsity).
    Mixed,
}

impl Default for CoreSelectionMethod {
    fn default() -> Self {
        Self::Mixed
    }
}

/// Configuration for dimensional collapse to Matryoshka SAE Core.
///
/// Sprint 168 pivot: Run Koopman control, Tube MPC, and certification
/// **only in low-dim core** (top 16-32 dims by topological/moral importance).
///
/// This makes edge propagation viable by collapsing from 4096D+ → core_dim=32,
/// reducing complexity from O(4096^3) to O(32^3) — a 512Kx speedup.
#[derive(Debug, Clone)]
pub struct DimensionalCollapseConfig {
    /// Core dimensions for control (16-32 recommended for edge).
    /// Default: 32.
    pub core_dim: usize,
    /// Method for core selection.
    /// Default: Mixed (weighted norm + sparsity).
    pub selection_method: CoreSelectionMethod,
    /// Enable adaptive core_dim based on activation energy.
    /// When true, core_dim varies between min_core_dim and max_core_dim.
    /// Default: false (fixed core_dim).
    pub adaptive_core: bool,
    /// Minimum core_dim when adaptive is enabled.
    /// Default: 16.
    pub min_core_dim: usize,
    /// Maximum core_dim when adaptive is enabled.
    /// Default: 64.
    pub max_core_dim: usize,
    /// Weight for norm component in Mixed selection (sparsity weight = 1 - alpha).
    /// Default: 0.7 (70% norm + 30% sparsity).
    pub mixed_norm_weight: f32,
}

impl Default for DimensionalCollapseConfig {
    fn default() -> Self {
        Self {
            core_dim: 32,
            selection_method: CoreSelectionMethod::Mixed,
            adaptive_core: false,
            min_core_dim: 16,
            max_core_dim: 64,
            mixed_norm_weight: 0.7,
        }
    }
}

impl DimensionalCollapseConfig {
    /// Preset for edge deployment (minimal core, aggressive collapse).
    pub fn edge_fast() -> Self {
        Self {
            core_dim: 16,
            selection_method: CoreSelectionMethod::Norm,
            adaptive_core: true,
            min_core_dim: 8,
            max_core_dim: 32,
            mixed_norm_weight: 0.8,
        }
    }

    /// Preset for server-class nodes (larger core, higher precision).
    pub fn high_precision() -> Self {
        Self {
            core_dim: 64,
            selection_method: CoreSelectionMethod::Mixed,
            adaptive_core: false,
            min_core_dim: 32,
            max_core_dim: 128,
            mixed_norm_weight: 0.6,
        }
    }

    /// Set fixed core dimension.
    pub fn with_core_dim(mut self, dim: usize) -> Self {
        self.core_dim = dim.max(1).min(256);
        self.adaptive_core = false;
        self
    }

    /// Enable adaptive core with bounds.
    pub fn with_adaptive_core(mut self, min_dim: usize, max_dim: usize) -> Self {
        self.adaptive_core = true;
        self.min_core_dim = min_dim.max(1);
        self.max_core_dim = max_dim.min(256).max(self.min_core_dim);
        self
    }

    /// Validate configuration constraints.
    pub fn validate(&self) -> std::result::Result<(), String> {
        if self.core_dim < 1 || self.core_dim > 256 {
            return Err(format!(
                "core_dim must be in [1, 256], got {}",
                self.core_dim
            ));
        }
        if self.min_core_dim < 1 {
            return Err("min_core_dim must be >= 1".to_string());
        }
        if self.max_core_dim < self.min_core_dim {
            return Err(format!(
                "max_core_dim ({}) must be >= min_core_dim ({})",
                self.max_core_dim, self.min_core_dim
            ));
        }
        if self.mixed_norm_weight < 0.0 || self.mixed_norm_weight > 1.0 {
            return Err(format!(
                "mixed_norm_weight must be in [0, 1], got {}",
                self.mixed_norm_weight
            ));
        }
        Ok(())
    }
}

/// Result of extracting topological core from high-dimensional SAE features.
#[derive(Debug, Clone)]
pub struct CoreExtractionResult {
    /// Core features [batch, core_dim].
    pub core_features: Tensor,
    /// Indices of selected core dimensions (in original SAE space).
    pub core_indices: Vec<usize>,
    /// Importance scores for selected dimensions.
    pub importance_scores: Vec<f32>,
    /// Total dimensions in original SAE features.
    pub original_dim: usize,
    /// Effective core dimension used.
    pub effective_core_dim: usize,
}

impl std::fmt::Display for CoreExtractionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CoreExtraction[{}→{} dims | top importance: {:.4}]",
            self.original_dim,
            self.effective_core_dim,
            self.importance_scores.first().copied().unwrap_or(0.0)
        )
    }
}

/// Extract topological core from SAE features using configured selection method.
///
/// This is the key function for dimensional collapse: it reduces high-dimensional
/// SAE features (e.g., 2048D) to a compact core (e.g., 32D) that captures the
/// most dynamically relevant dimensions for Koopman control.
///
/// # Arguments
/// * `sae_features` - SAE activations [batch, d_sae] or [1, d_sae].
/// * `config` - Dimensional collapse configuration.
/// * `sae_sparsity_weights` - Optional sparsity weights from SAE [d_sae]. None uses norm-only.
///
/// # Returns
/// `CoreExtractionResult` with core features, indices, and importance scores.
pub fn extract_topological_core(
    sae_features: &Tensor,
    config: &DimensionalCollapseConfig,
    sae_sparsity_weights: Option<&Tensor>,
) -> Result<CoreExtractionResult> {
    let dims = sae_features.dims();
    let (batch, d_sae) = match dims.len() {
        2 => (dims[0], dims[1]),
        1 => (1, dims[0]),
        _ => candle_core::bail!("sae_features must be 1D or 2D, got {}D", dims.len()),
    };

    let effective_core_dim = if config.adaptive_core {
        // Adaptive: estimate core_dim from activation energy
        let energy = sae_features.sqr()?.to_dtype(DType::F32)?.sum_all()?;
        let energy_val: f32 = energy.to_scalar()?;
        // Log-scale mapping: higher energy → larger core
        let log_energy = (energy_val.max(1e-8) as f64).log2().clamp(0.0, 16.0);
        let scale = log_energy / 16.0;
        config.min_core_dim
            + ((scale * (config.max_core_dim as f64 - config.min_core_dim as f64)) as usize)
    } else {
        config.core_dim
    };

    let core_dim = effective_core_dim.min(d_sae);

    // Compute importance scores per dimension
    let importance: Tensor = match config.selection_method {
        CoreSelectionMethod::Norm => {
            // L2 norm per dimension across batch
            let sq = sae_features.sqr()?;
            if batch > 1 {
                sq.mean(0)?
            } else {
                sq.squeeze(0)?
            }
        }
        CoreSelectionMethod::Sparsity => {
            // Use SAE sparsity weights directly
            match sae_sparsity_weights {
                Some(w) => w.to_dtype(DType::F32)?,
                None => {
                    // Fallback to norm if no sparsity weights
                    let sq = sae_features.sqr()?;
                    if batch > 1 {
                        sq.mean(0)?
                    } else {
                        sq.squeeze(0)?
                    }
                }
            }
        }
        CoreSelectionMethod::Mixed => {
            // Weighted combination: alpha * norm + (1-alpha) * sparsity
            let norm_score = {
                let sq = sae_features.sqr()?;
                if batch > 1 {
                    sq.mean(0)?
                } else {
                    sq.squeeze(0)?
                }
            };
            match sae_sparsity_weights {
                Some(w) => {
                    let sparsity_score = w.to_dtype(DType::F32)?;
                    // Normalize both to [0,1] range using Vec max (Candle lacks max_all)
                    let norm_vec: Vec<f32> = if norm_score.dims().len() == 1 {
                        norm_score.to_vec1()?
                    } else {
                        norm_score.flatten_all()?.to_vec1()?
                    };
                    let sparsity_vec: Vec<f32> = if sparsity_score.dims().len() == 1 {
                        sparsity_score.to_vec1()?
                    } else {
                        sparsity_score.flatten_all()?.to_vec1()?
                    };
                    let norm_max = norm_vec.iter().copied().fold(0.0f32, f32::max).max(1e-12);
                    let sparsity_max = sparsity_vec.iter().copied().fold(0.0f32, f32::max).max(1e-12);
                    let norm_max_tensor = Tensor::new(norm_max, sae_features.device())?;
                    let sparsity_max_tensor = Tensor::new(sparsity_max, sae_features.device())?;
                    let norm_normed = norm_score.broadcast_div(&norm_max_tensor)?;
                    let sparsity_normed = sparsity_score.broadcast_div(&sparsity_max_tensor)?;
                    let alpha = config.mixed_norm_weight;
                    let one_minus_alpha = 1.0 - alpha;
                    let alpha_tensor = Tensor::new(alpha, sae_features.device())?;
                    let beta_tensor = Tensor::new(one_minus_alpha, sae_features.device())?;
                    let norm_weighted = norm_normed.broadcast_mul(&alpha_tensor)?;
                    let sparsity_weighted = sparsity_normed.broadcast_mul(&beta_tensor)?;
                    norm_weighted.broadcast_add(&sparsity_weighted)?
                }
                None => {
                    // No sparsity weights, use norm only
                    norm_score
                }
            }
        }
    };

    // Extract importance vector and rank dimensions
    let importance_vec: Vec<f32> = if importance.dims().len() == 1 {
        importance.to_vec1()?
    } else {
        importance.flatten_all()?.to_vec1()?
    };

    // Create indexed scores and sort descending
    let mut indexed: Vec<(usize, f32)> = importance_vec
        .into_iter()
        .enumerate()
        .collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Select top-k
    let core_indices: Vec<usize> = indexed[..core_dim].iter().map(|(i, _)| *i).collect();
    let importance_scores: Vec<f32> = indexed[..core_dim].iter().map(|(_, s)| *s).collect();

    // Extract core features by gathering selected columns
    let core_features = if core_indices.is_empty() {
        Tensor::zeros((batch, 1), sae_features.dtype(), sae_features.device())?
    } else {
        // Gather columns by indices using narrow slices
        let mut parts: Vec<Tensor> = Vec::with_capacity(core_dim);
        for &idx in &core_indices {
            let col = sae_features.narrow(1, idx, 1)?;
            parts.push(col);
        }
        Tensor::cat(&parts, 1)?
    };

    Ok(CoreExtractionResult {
        core_features,
        core_indices,
        importance_scores,
        original_dim: d_sae,
        effective_core_dim: core_dim,
    })
}

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
    /// Wasserstein robust weight w = min(1, ε_W / ||e||).
    /// w=1.0 means full update (innovation within ambiguity set).
    /// w<1.0 means attenuated update (innovation exceeds Wasserstein radius).
    pub wasserstein_weight: f64,
}

impl std::fmt::Display for RLSUpdateResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RLS{{ innov={:.6e}, updated={}, cond={:.2e}, infl={}, svd={:?}->{:?}, mse={:.6e}, w_W={:.3} }}",
            self.innovation_norm,
            self.updated,
            self.condition_number,
            self.inflated,
            self.svd_rank_before,
            self.svd_rank_after,
            self.prediction_error,
            self.wasserstein_weight
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

        // --- Wasserstein Robust Weight ---
        // w = min(1, ε_W / ||e||) — attenuate update if innovation exceeds ambiguity radius
        let wasserstein_weight = if self.config.wasserstein_radius.is_infinite() {
            1.0 // Disabled
        } else {
            (1.0_f64).min(self.config.wasserstein_radius / innovation_norm.max(1e-15))
        };

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
                wasserstein_weight,
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

        // Update theta: theta += w_W · (gain @ innovation)  → [d,1] @ [1,d] = [d,d]
        // Wasserstein weight attenuates the update for adversarial innovations
        let theta_update = gain.matmul(&innovation)?;
        let theta_update_weighted = theta_update.broadcast_mul(
            &Tensor::new(wasserstein_weight, &self.device)?,
        )?;
        self.theta = self.theta.broadcast_add(&theta_update_weighted)?;

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
            wasserstein_weight,
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

    // -----------------------------------------------------------------------
    // Dimensional Collapse RLS — Update in core space (Sprint 168)
    // -----------------------------------------------------------------------

    /// Perform RLS update using dimensional collapse.
    ///
    /// This method accepts full-dimensional SAE features, extracts the
    /// topological core (top-k dimensions by importance), and performs
    /// the RLS update entirely in the reduced core space.
    ///
    /// **Pipeline:**
    /// 1. Extract core from `phi_t_full` → `phi_core` [1, core_dim]
    /// 2. Extract core from `y_t_full` → `y_core` [1, core_dim]
    /// 3. Run standard RLS update on core features
    /// 4. Return `CoreRLSResult` with core indices and metrics
    ///
    /// # Arguments
    /// * `phi_t_full` - Current full-dim SAE features. Shape: [1, d_sae] or [d_sae].
    /// * `y_t_full` - Next full-dim SAE features. Shape: [1, d_sae] or [d_sae].
    /// * `collapse_config` - Configuration for dimensional collapse.
    /// * `sae_sparsity_weights` - Optional SAE sparsity weights for Mixed selection.
    ///
    /// # Returns
    /// `CoreRLSResult` containing the RLS result, core indices, and extraction metrics.
    pub fn update_koopman_rls_core(
        &mut self,
        phi_t_full: &Tensor,
        y_t_full: &Tensor,
        collapse_config: &DimensionalCollapseConfig,
        sae_sparsity_weights: Option<&Tensor>,
    ) -> Result<CoreRLSResult> {
        // Extract core from phi_t
        let phi_result = extract_topological_core(
            phi_t_full,
            collapse_config,
            sae_sparsity_weights,
        )?;
        let phi_core = &phi_result.core_features;
        let core_indices = phi_result.core_indices.clone();
        let original_dim = phi_result.original_dim;
        let effective_core_dim = phi_result.effective_core_dim;

        // Extract core from y_t using the same indices
        // (ensure consistent dimensionality between phi and y)
        let y_core = extract_core_by_indices(y_t_full, &core_indices)?;

        // Perform standard RLS update in core space
        let rls_result = self.update_koopman_rls(phi_core, &y_core)?;

        Ok(CoreRLSResult {
            rls_result,
            core_indices,
            original_dim,
            effective_core_dim,
            importance_scores: phi_result.importance_scores,
        })
    }
}

/// Result of a dimensional collapse RLS update.
#[derive(Debug, Clone)]
pub struct CoreRLSResult {
    /// Underlying RLS update result.
    pub rls_result: RLSUpdateResult,
    /// Indices of selected core dimensions.
    pub core_indices: Vec<usize>,
    /// Original SAE dimension before collapse.
    pub original_dim: usize,
    /// Effective core dimension used.
    pub effective_core_dim: usize,
    /// Importance scores for all dimensions.
    pub importance_scores: Vec<f32>,
}

impl std::fmt::Display for CoreRLSResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CoreRLS[orig={} → core={} | updated={} | err={:.6}]",
            self.original_dim,
            self.effective_core_dim,
            self.rls_result.updated,
            self.rls_result.prediction_error
        )
    }
}

/// Extract core features from a tensor using pre-computed indices.
///
/// This helper ensures that both `phi_t` and `y_t` use the exact same
/// core indices, maintaining dimensional consistency.
///
/// # Arguments
/// * `features` - Full-dim features. Shape: [1, d_sae] or [d_sae].
/// * `core_indices` - Indices of selected core dimensions.
///
/// # Returns
/// Core features tensor. Shape: [1, core_dim].
fn extract_core_by_indices(features: &Tensor, core_indices: &[usize]) -> Result<Tensor> {
    let dims = features.dims();
    let is_1d = dims.len() == 1;
    let (_batch, d_full) = if is_1d {
        (1, dims[0])
    } else {
        (dims[0], dims[1])
    };

    if core_indices.is_empty() {
        return Err(candle_core::Error::Msg(
            "extract_core_by_indices: core_indices is empty".to_string()
        ));
    }

    if core_indices.len() >= d_full {
        // No reduction needed — return as 2D [1, d_full]
        if is_1d {
            return features.unsqueeze(0);
        }
        return Ok(features.clone());
    }

    // For 1D input, extract scalars and stack into [core_dim], then unsqueeze to [1, core_dim]
    if is_1d {
        let mut scalars: Vec<Tensor> = Vec::with_capacity(core_indices.len());
        for &idx in core_indices {
            if idx >= d_full {
                return Err(candle_core::Error::Msg(format!(
                    "extract_core_by_indices: index {} out of bounds (dim={})",
                    idx,
                    d_full
                )));
            }
            let scalar = features.narrow(0, idx, 1)?; // [1]
            scalars.push(scalar);
        }
        let result = Tensor::cat(&scalars, 0)?; // [core_dim]
        return result.unsqueeze(0); // [1, core_dim]
    }

    // 2D input: extract columns
    let mut core_parts: Vec<Tensor> = Vec::with_capacity(core_indices.len());
    for &idx in core_indices {
        if idx >= d_full {
            return Err(candle_core::Error::Msg(format!(
                "extract_core_by_indices: index {} out of bounds (dim={})",
                idx,
                d_full
            )));
        }
        let col = features.narrow(1, idx, 1)?;
        core_parts.push(col);
    }

    // Concatenate along dimension 1
    Tensor::cat(&core_parts, 1)
}


// ---------------------------------------------------------------------------
// Sprint 167 — SAE Observable Lifting (Koopman lifting with SAE features)
// ---------------------------------------------------------------------------

/// Koopman lifting via SAE observables: ψ(x) = [x; φ_SAE(x)].
///
/// Composes the raw state `x` with SAE-encoded features `sae_features`
/// to form the lifted observable vector used in Koopman RLS.
///
/// **Formula:** ψ(x) = concat(x, φ_SAE(x)) ∈ ℝ^{d_model + d_sae}
///
/// # Arguments
/// * `x` — Raw activation/state tensor. Shape: `[batch, d_model]` or `[d_model]`.
/// * `sae_features` — SAE-encoded features. Shape: `[batch, d_sae]` or `[d_sae]`.
///
/// # Returns
/// Lifted observable ψ(x). Shape: `[batch, d_model + d_sae]` or `[d_model + d_sae]`.
pub fn koopman_lifting_sae(x: &Tensor, sae_features: &Tensor) -> Result<Tensor> {
    // Ensure both tensors have same rank
    let (x_2d, sae_2d, was_1d) = if x.rank() == 1 {
        (x.unsqueeze(0)?, sae_features.unsqueeze(0)?, true)
    } else {
        (x.clone(), sae_features.clone(), false)
    };

    // Concatenate along feature dimension (dim=1)
    let lifted = Tensor::cat(&[x_2d, sae_2d], 1)?;

    if was_1d {
        lifted.squeeze(0)
    } else {
        Ok(lifted)
    }
}

/// Advanced observable lifting: ψ(x) = [x; x²; sin(x); φ_SAE(x)].
///
/// Extends basic SAE lifting with polynomial and trigonometric observables
/// for richer Koopman linearization of non-linear dynamics.
///
/// **Formula:** ψ(x) = concat(x, x⊙x, sin(x), φ_SAE(x))
///
/// # Arguments
/// * `x` — Raw activation/state tensor. Shape: `[batch, d_model]` or `[d_model]`.
/// * `sae_features` — SAE-encoded features. Shape: `[batch, d_sae]` or `[d_sae]`.
///
/// # Returns
/// Advanced lifted observable. Shape: `[batch, 3*d_model + d_sae]` or `[3*d_model + d_sae]`.
pub fn koopman_lifting_advanced(x: &Tensor, sae_features: &Tensor) -> Result<Tensor> {
    let (x_2d, sae_2d, was_1d) = if x.rank() == 1 {
        (x.unsqueeze(0)?, sae_features.unsqueeze(0)?, true)
    } else {
        (x.clone(), sae_features.clone(), false)
    };

    // Quadratic observables: x⊙x
    let x_sq = x_2d.sqr()?;

    // Trigonometric observables: sin(x)
    let x_sin = x_2d.sin()?;

    // Concatenate: [x, x², sin(x), φ_SAE(x)]
    let lifted = Tensor::cat(&[x_2d, x_sq, x_sin, sae_2d], 1)?;

    if was_1d {
        lifted.squeeze(0)
    } else {
        Ok(lifted)
    }
}

#[cfg(test)]
mod tests_s167 {
    use super::*;

    /// Test basic SAE lifting: ψ(x) = [x; φ_SAE(x)]
    #[test]
    fn test_koopman_lifting_sae_single() -> Result<()> {
        let device = Device::Cpu;
        let d_model = 8;
        let d_sae = 16;

        let x_data: Vec<f64> = (0..d_model).map(|i| i as f64 * 0.1).collect();
        let x = Tensor::from_vec(x_data, d_model, &device)?;

        let sae_data: Vec<f64> = (0..d_sae).map(|i| (i % 5) as f64 * 0.05).collect();
        let sae = Tensor::from_vec(sae_data, d_sae, &device)?;

        let lifted = koopman_lifting_sae(&x, &sae)?;
        assert_eq!(lifted.dim(0)?, d_model + d_sae);

        // Verify first d_model elements match x
        let flat: Vec<f64> = lifted.to_vec1()?;
        for i in 0..d_model {
            assert!((flat[i] - (i as f64 * 0.1)).abs() < 1e-10);
        }

        // Verify last d_sae elements match sae_features
        for i in 0..d_sae {
            assert!((flat[d_model + i] - ((i % 5) as f64 * 0.05)).abs() < 1e-10);
        }
        Ok(())
    }

    /// Test batch SAE lifting
    #[test]
    fn test_koopman_lifting_sae_batch() -> Result<()> {
        let device = Device::Cpu;
        let batch = 4;
        let d_model = 8;
        let d_sae = 16;

        let x_data: Vec<f64> = (0..(batch * d_model)).map(|i| (i % 7) as f64 * 0.1).collect();
        let x = Tensor::from_vec(x_data, (batch, d_model), &device)?;

        let sae_data: Vec<f64> = (0..(batch * d_sae)).map(|i| (i % 11) as f64 * 0.05).collect();
        let sae = Tensor::from_vec(sae_data, (batch, d_sae), &device)?;

        let lifted = koopman_lifting_sae(&x, &sae)?;
        assert_eq!(lifted.shape().dims(), [batch, d_model + d_sae]);
        Ok(())
    }

    /// Test advanced lifting: ψ(x) = [x; x²; sin(x); φ_SAE(x)]
    #[test]
    fn test_koopman_lifting_advanced() -> Result<()> {
        let device = Device::Cpu;
        let d_model = 4;
        let d_sae = 8;

        let x_data: Vec<f64> = vec![0.1, 0.2, 0.3, 0.4];
        let x = Tensor::from_vec(x_data, d_model, &device)?;

        let sae_data: Vec<f64> = (0..d_sae).map(|i| i as f64 * 0.01).collect();
        let sae = Tensor::from_vec(sae_data, d_sae, &device)?;

        let lifted = koopman_lifting_advanced(&x, &sae)?;
        // Expected: 3*d_model + d_sae = 12 + 8 = 20
        assert_eq!(lifted.dim(0)?, 3 * d_model + d_sae);

        let flat: Vec<f64> = lifted.to_vec1()?;

        // First d_model: x
        assert!((flat[0] - 0.1).abs() < 1e-10);
        // Next d_model: x²
        assert!((flat[d_model] - 0.01).abs() < 1e-10);
        // Next d_model: sin(x)
        assert!((flat[2 * d_model] - (0.1f64).sin()).abs() < 1e-10);
        // Last d_sae: sae features
        assert!((flat[3 * d_model] - 0.0).abs() < 1e-10);
        Ok(())
    }

    /// Full integration: RLS Koopman with SAE lifting
    #[test]
    fn test_rls_with_sae_lifting() -> Result<()> {
        let device = Device::Cpu;
        let d_model = 4;
        let d_sae = 4;
        let d_lifted = d_model + d_sae;
        let config = KoopmanRLSConfig::edge_fast();
        let mut rls = KoopmanRLS::new(d_lifted, config, &device)?;

        // True dynamics in raw space: y = A @ x (diagonal, stable)
        let a_data: Vec<f64> = vec![0.7, 0.0, 0.0, 0.0,
                                     0.0, 0.6, 0.0, 0.0,
                                     0.0, 0.0, 0.5, 0.0,
                                     0.0, 0.0, 0.0, 0.4];
        let a = Tensor::from_vec(a_data, (d_model, d_model), &device)?;

        // SAE "encoder": simple linear transform
        let sae_w: Vec<f64> = (0..(d_model * d_sae)).map(|i| ((i % 7) as f64 - 3.5) * 0.1).collect();
        let sae_encoder = Tensor::from_vec(sae_w, (d_model, d_sae), &device)?;

        // Generate transitions with SAE lifting
        let n_steps = 100;
        let mut seed: u64 = 123;
        for _ in 0..n_steps {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let x_data: Vec<f64> = (0..d_model)
                .map(|_| ((seed >> 33) as f64 / u32::MAX as f64 - 0.5) * 2.0)
                .collect();
            let x = Tensor::from_vec(x_data.clone(), (1, d_model), &device)?;

            // SAE features: φ(x) = x @ W_sae
            let phi_x = x.matmul(&sae_encoder)?;

            // Lifted: ψ(x) = [x; φ(x)]
            let psi_x = koopman_lifting_sae(&x, &phi_x)?;

            // Next state: y = A @ x
            let y_raw = x.matmul(&a.t()?)?;
            // SAE features of next state
            let phi_y = y_raw.matmul(&sae_encoder)?;
            let psi_y = koopman_lifting_sae(&y_raw, &phi_y)?;

            let result = rls.update_koopman_rls(&psi_x, &psi_y)?;
            if result.updated {
                // Innovation should decrease over time
                assert!(result.prediction_error < 10.0, "Error too large: {}", result.prediction_error);
            }
        }

        assert!(rls.update_count() > 50, "Should have many updates: {}", rls.update_count());
        Ok(())
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

    // -----------------------------------------------------------------------
    // Sprint 168 — Dimensional Collapse tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_dimensional_collapse_config_default() {
        let cfg = DimensionalCollapseConfig::default();
        assert_eq!(cfg.core_dim, 32);
        assert_eq!(cfg.selection_method, CoreSelectionMethod::Mixed);
        assert!(!cfg.adaptive_core);
        assert_eq!(cfg.min_core_dim, 16);
        assert_eq!(cfg.max_core_dim, 64);
        assert!((cfg.mixed_norm_weight - 0.7).abs() < 1e-6);
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_dimensional_collapse_config_edge_fast() {
        let cfg = DimensionalCollapseConfig::edge_fast();
        assert_eq!(cfg.core_dim, 16);
        assert_eq!(cfg.selection_method, CoreSelectionMethod::Norm);
        assert!(cfg.adaptive_core);
        assert_eq!(cfg.min_core_dim, 8);
        assert_eq!(cfg.max_core_dim, 32);
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_dimensional_collapse_config_high_precision() {
        let cfg = DimensionalCollapseConfig::high_precision();
        assert_eq!(cfg.core_dim, 64);
        assert_eq!(cfg.selection_method, CoreSelectionMethod::Mixed);
        assert!(!cfg.adaptive_core);
        assert_eq!(cfg.min_core_dim, 32);
        assert_eq!(cfg.max_core_dim, 128);
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_dimensional_collapse_config_with_core_dim() {
        let cfg = DimensionalCollapseConfig::default().with_core_dim(48);
        assert_eq!(cfg.core_dim, 48);
        assert!(!cfg.adaptive_core);
    }

    #[test]
    fn test_dimensional_collapse_config_with_adaptive_core() {
        let cfg = DimensionalCollapseConfig::default().with_adaptive_core(16, 48);
        assert!(cfg.adaptive_core);
        assert_eq!(cfg.min_core_dim, 16);
        assert_eq!(cfg.max_core_dim, 48);
    }

    #[test]
    fn test_dimensional_collapse_validate_invalid_core_dim() {
        let mut cfg = DimensionalCollapseConfig::default();
        cfg.core_dim = 0;
        assert!(cfg.validate().is_err());

        cfg.core_dim = 300;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_dimensional_collapse_validate_invalid_bounds() {
        let mut cfg = DimensionalCollapseConfig::default();
        cfg.min_core_dim = 0;
        assert!(cfg.validate().is_err());

        cfg.min_core_dim = 1;
        cfg.max_core_dim = 0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_extract_topological_core_norm() -> Result<()> {
        let device = Device::Cpu;
        let d_sae = 64;
        let core_dim = 16;

        // Create features where first 10 dims have high energy
        let data: Vec<f32> = (0..d_sae)
            .map(|i| if i < 10 { 1.0f32 } else { 0.01f32 })
            .collect();
        let sae_features = Tensor::from_vec(data, (1, d_sae), &device)?;

        let cfg = DimensionalCollapseConfig {
            core_dim,
            selection_method: CoreSelectionMethod::Norm,
            ..Default::default()
        };

        let result = extract_topological_core(&sae_features, &cfg, None)?;
        assert_eq!(result.effective_core_dim, core_dim);
        assert_eq!(result.original_dim, d_sae);
        assert_eq!(result.core_indices.len(), core_dim);

        // Top indices should be the high-energy ones (0-9)
        assert!(result.core_indices.contains(&0));
        assert!(result.core_indices.contains(&9));

        // Core features shape
        let core_dims = result.core_features.dims();
        assert_eq!(core_dims[0], 1);
        assert_eq!(core_dims[1], core_dim);
        Ok(())
    }

    #[test]
    fn test_extract_topological_core_mixed_no_sparsity() -> Result<()> {
        let device = Device::Cpu;
        let d_sae = 32;
        let core_dim = 8;

        let data: Vec<f32> = (0..d_sae)
            .map(|i| (d_sae - i) as f32 * 0.1)
            .collect();
        let sae_features = Tensor::from_vec(data, (1, d_sae), &device)?;

        let cfg = DimensionalCollapseConfig {
            core_dim,
            selection_method: CoreSelectionMethod::Mixed,
            ..Default::default()
        };

        let result = extract_topological_core(&sae_features, &cfg, None)?;
        assert_eq!(result.effective_core_dim, core_dim);
        // Should select dims with highest norm (first dims have highest values)
        assert!(result.core_indices.contains(&0));
        Ok(())
    }

    #[test]
    fn test_extract_topological_core_adaptive() -> Result<()> {
        let device = Device::Cpu;
        let d_sae = 128;

        // High energy signal
        let data: Vec<f32> = (0..d_sae).map(|_| 1.0f32).collect();
        let sae_features = Tensor::from_vec(data, (1, d_sae), &device)?;

        let cfg = DimensionalCollapseConfig {
            adaptive_core: true,
            min_core_dim: 16,
            max_core_dim: 48,
            ..Default::default()
        };

        let result = extract_topological_core(&sae_features, &cfg, None)?;
        assert!(result.effective_core_dim >= 16);
        assert!(result.effective_core_dim <= 48);
        assert!(result.effective_core_dim <= d_sae);
        Ok(())
    }

    #[test]
    fn test_extract_topological_core_clamps_to_d_sae() -> Result<()> {
        let device = Device::Cpu;
        let d_sae = 8;

        let data: Vec<f32> = (0..d_sae).map(|i| i as f32).collect();
        let sae_features = Tensor::from_vec(data, (1, d_sae), &device)?;

        // Request more core dims than available
        let cfg = DimensionalCollapseConfig {
            core_dim: 32,
            ..Default::default()
        };

        let result = extract_topological_core(&sae_features, &cfg, None)?;
        assert_eq!(result.effective_core_dim, d_sae);
        Ok(())
    }

    #[test]
    fn test_core_extraction_result_display() -> Result<()> {
        let device = Device::Cpu;
        let d_sae = 16;
        let data: Vec<f32> = (0..d_sae).map(|i| i as f32 * 0.1).collect();
        let sae_features = Tensor::from_vec(data, (1, d_sae), &device)?;

        let cfg = DimensionalCollapseConfig {
            core_dim: 4,
            ..Default::default()
        };

        let result = extract_topological_core(&sae_features, &cfg, None)?;
        let display = format!("{}", result);
        assert!(display.contains("CoreExtraction"));
        assert!(display.contains("dims"));
        Ok(())
    }

    #[test]
    fn test_core_selection_method_default() {
        let method = CoreSelectionMethod::default();
        assert_eq!(method, CoreSelectionMethod::Mixed);
    }

    // -----------------------------------------------------------------------
    // Sprint 168 — PASO C: Dimensional Collapse in Koopman RLS
    // -----------------------------------------------------------------------

    #[test]
    fn test_core_rls_update_basic() -> Result<()> {
        let device = Device::Cpu;
        let d_sae = 64;
        let core_dim = 16;

        // Create KoopmanRLS in core space
        let mut rls = KoopmanRLS::new(core_dim, KoopmanRLSConfig::default(), &device)?;

        // Create full-dim features
        let phi_data: Vec<f32> = (0..d_sae).map(|i| i as f32 * 0.1).collect();
        let y_data: Vec<f32> = (0..d_sae).map(|i| (i + 1) as f32 * 0.1).collect();
        let phi_full = Tensor::from_vec(phi_data, (1, d_sae), &device)?;
        let y_full = Tensor::from_vec(y_data, (1, d_sae), &device)?;

        let collapse_cfg = DimensionalCollapseConfig {
            core_dim,
            ..Default::default()
        };

        let result = rls.update_koopman_rls_core(&phi_full, &y_full, &collapse_cfg, None)?;
        assert_eq!(result.effective_core_dim, core_dim);
        assert_eq!(result.original_dim, d_sae);
        assert_eq!(result.core_indices.len(), core_dim);
        Ok(())
    }

    #[test]
    fn test_core_rls_update_reduces_dimension() -> Result<()> {
        let device = Device::Cpu;
        let d_sae = 128;
        let core_dim = 32;

        let mut rls = KoopmanRLS::new(core_dim, KoopmanRLSConfig::default(), &device)?;

        let phi_data: Vec<f32> = (0..d_sae).map(|_| 1.0f32).collect();
        let y_data: Vec<f32> = (0..d_sae).map(|_| 2.0f32).collect();
        let phi_full = Tensor::from_vec(phi_data, (1, d_sae), &device)?;
        let y_full = Tensor::from_vec(y_data, (1, d_sae), &device)?;

        let collapse_cfg = DimensionalCollapseConfig {
            core_dim,
            ..Default::default()
        };

        let result = rls.update_koopman_rls_core(&phi_full, &y_full, &collapse_cfg, None)?;
        // K operator should be [core_dim, core_dim]
        assert_eq!(rls.k_operator().shape().dims(), [core_dim, core_dim]);
        assert!(result.rls_result.updated);
        Ok(())
    }

    #[test]
    fn test_core_rls_result_display() -> Result<()> {
        let device = Device::Cpu;
        let d_sae = 32;
        let core_dim = 8;

        let mut rls = KoopmanRLS::new(core_dim, KoopmanRLSConfig::default(), &device)?;

        let phi_data: Vec<f32> = (0..d_sae).map(|i| i as f32 * 0.5).collect();
        let y_data: Vec<f32> = (0..d_sae).map(|i| (i + 1) as f32 * 0.5).collect();
        let phi_full = Tensor::from_vec(phi_data, (1, d_sae), &device)?;
        let y_full = Tensor::from_vec(y_data, (1, d_sae), &device)?;

        let collapse_cfg = DimensionalCollapseConfig {
            core_dim,
            ..Default::default()
        };

        let result = rls.update_koopman_rls_core(&phi_full, &y_full, &collapse_cfg, None)?;
        let display = format!("{}", result);
        assert!(display.contains("CoreRLS"));
        assert!(display.contains("orig=32"));
        assert!(display.contains("core=8"));
        Ok(())
    }

    #[test]
    fn test_extract_core_by_indices() -> Result<()> {
        let device = Device::Cpu;
        let d_full = 64;
        let core_indices = vec![0, 10, 20, 30, 40, 50];

        let data: Vec<f32> = (0..d_full).map(|i| i as f32).collect();
        let features = Tensor::from_vec(data, (1, d_full), &device)?;

        let core = extract_core_by_indices(&features, &core_indices)?;
        let dims = core.dims();
        assert_eq!(dims, [1, 6]);

        // Verify extracted values match indices
        let vec = core.flatten_all()?.to_vec1::<f32>()?;
        assert!((vec[0] - 0.0).abs() < 1e-5);
        assert!((vec[1] - 10.0).abs() < 1e-5);
        assert!((vec[2] - 20.0).abs() < 1e-5);
        assert!((vec[3] - 30.0).abs() < 1e-5);
        assert!((vec[4] - 40.0).abs() < 1e-5);
        assert!((vec[5] - 50.0).abs() < 1e-5);
        Ok(())
    }

    #[test]
    fn test_extract_core_by_indices_1d() -> Result<()> {
        let device = Device::Cpu;
        let d_full = 32;
        let core_indices = vec![5, 15, 25];

        let data: Vec<f32> = (0..d_full).map(|i| i as f32 * 2.0).collect();
        let features = Tensor::from_vec(data, d_full, &device)?;

        let core = extract_core_by_indices(&features, &core_indices)?;
        let dims = core.dims();
        assert_eq!(dims, [1, 3]);

        let vec = core.flatten_all()?.to_vec1::<f32>()?;
        assert!((vec[0] - 10.0).abs() < 1e-5); // index 5 * 2
        assert!((vec[1] - 30.0).abs() < 1e-5); // index 15 * 2
        assert!((vec[2] - 50.0).abs() < 1e-5); // index 25 * 2
        Ok(())
    }

    #[test]
    fn test_extract_core_by_indices_no_reduction() -> Result<()> {
        let device = Device::Cpu;
        let d_full = 8;
        // Indices cover all dimensions
        let core_indices: Vec<usize> = (0..d_full).collect();

        let data: Vec<f32> = (0..d_full).map(|i| i as f32).collect();
        let features = Tensor::from_vec(data, (1, d_full), &device)?;

        let core = extract_core_by_indices(&features, &core_indices)?;
        let dims = core.dims();
        assert_eq!(dims, [1, d_full]);
        Ok(())
    }

    #[test]
    fn test_core_rls_multiple_updates() -> Result<()> {
        let device = Device::Cpu;
        let d_sae = 64;
        let core_dim = 16;

        let mut rls = KoopmanRLS::new(core_dim, KoopmanRLSConfig::default(), &device)?;

        for step in 0..5u32 {
            let phi_data: Vec<f32> = (0..d_sae)
                .map(|i| (i as f32 + step as f32) * 0.1)
                .collect();
            let y_data: Vec<f32> = (0..d_sae)
                .map(|i| (i as f32 + (step + 1) as f32) * 0.1)
                .collect();
            let phi_full = Tensor::from_vec(phi_data, (1, d_sae), &device)?;
            let y_full = Tensor::from_vec(y_data, (1, d_sae), &device)?;

            let collapse_cfg = DimensionalCollapseConfig {
                core_dim,
                ..Default::default()
            };

            let result =
                rls.update_koopman_rls_core(&phi_full, &y_full, &collapse_cfg, None)?;
            assert_eq!(result.effective_core_dim, core_dim);
        }

        assert_eq!(rls.update_count(), 5);
        Ok(())
    }

    #[test]
    fn test_core_rls_with_edge_fast_config() -> Result<()> {
        let device = Device::Cpu;
        let d_sae = 128;
        let core_dim = 16;

        // Use fixed core_dim (no adaptive) to ensure RLS dim matches
        let collapse_cfg = DimensionalCollapseConfig {
            core_dim,
            selection_method: CoreSelectionMethod::Norm,
            adaptive_core: false,
            min_core_dim: 16,
            max_core_dim: 32,
            mixed_norm_weight: 0.5,
        };

        let mut rls = KoopmanRLS::new(core_dim, KoopmanRLSConfig::default(), &device)?;

        let phi_data: Vec<f32> = (0..d_sae).map(|i| i as f32 * 0.05).collect();
        let y_data: Vec<f32> = (0..d_sae).map(|i| (i + 2) as f32 * 0.05).collect();
        let phi_full = Tensor::from_vec(phi_data, (1, d_sae), &device)?;
        let y_full = Tensor::from_vec(y_data, (1, d_sae), &device)?;

        let result = rls.update_koopman_rls_core(&phi_full, &y_full, &collapse_cfg, None)?;
        assert_eq!(result.effective_core_dim, core_dim);
        assert_eq!(result.importance_scores.len(), core_dim);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Wasserstein Robust Koopman — PASO B (Sprint 169)
    // -----------------------------------------------------------------------

    /// Test: Normal innovation (||e|| < ε_W) → wasserstein_weight ≈ 1.0
    #[test]
    fn test_wasserstein_normal_innovation() -> Result<()> {
        let device = Device::Cpu;
        let dim = 8;
        let config = KoopmanRLSConfig::default()
            .with_wasserstein_radius(0.1)
            .with_dead_zone(0.0);
        let mut rls = KoopmanRLS::new(dim, config, &device)?;

        // Small perturbation → small innovation
        let phi_data: Vec<f64> = (0..dim).map(|i| i as f64 * 0.01).collect();
        let y_data: Vec<f64> = (0..dim).map(|i| i as f64 * 0.01 + 0.001).collect();
        let phi = Tensor::from_vec(phi_data, (1, dim), &device)?;
        let y = Tensor::from_vec(y_data, (1, dim), &device)?;

        let result = rls.update_koopman_rls(&phi, &y)?;
        // Innovation is small (< ε_W=0.1), so weight should be 1.0
        assert!((result.wasserstein_weight - 1.0).abs() < 1e-6);
        Ok(())
    }

    /// Test: Large innovation (||e|| > ε_W) → wasserstein_weight < 1.0
    #[test]
    fn test_wasserstein_large_innovation() -> Result<()> {
        let device = Device::Cpu;
        let dim = 8;
        let config = KoopmanRLSConfig::default()
            .with_wasserstein_radius(0.05)
            .with_dead_zone(0.0);
        let mut rls = KoopmanRLS::new(dim, config, &device)?;

        // Large perturbation → large innovation
        let phi_data: Vec<f64> = (0..dim).map(|_| 0.0).collect();
        let y_data: Vec<f64> = (0..dim).map(|_| 1.0).collect();
        let phi = Tensor::from_vec(phi_data, (1, dim), &device)?;
        let y = Tensor::from_vec(y_data, (1, dim), &device)?;

        let result = rls.update_koopman_rls(&phi, &y)?;
        // Innovation norm ≈ sqrt(8) ≈ 2.83 >> ε_W=0.05
        // w = min(1, 0.05 / 2.83) ≈ 0.0177
        assert!(result.wasserstein_weight < 1.0);
        assert!(result.wasserstein_weight > 0.0);
        assert!((result.wasserstein_weight - 0.0177).abs() < 0.005);
        Ok(())
    }

    /// Test: Disabled Wasserstein (ε_W = ∞) → wasserstein_weight always 1.0
    #[test]
    fn test_wasserstein_disabled() -> Result<()> {
        let device = Device::Cpu;
        let dim = 8;
        let config = KoopmanRLSConfig::default()
            .with_wasserstein_radius(f64::MAX)
            .with_dead_zone(0.0);
        let mut rls = KoopmanRLS::new(dim, config, &device)?;

        // Large innovation
        let phi_data: Vec<f64> = (0..dim).map(|_| 0.0).collect();
        let y_data: Vec<f64> = (0..dim).map(|_| 10.0).collect();
        let phi = Tensor::from_vec(phi_data, (1, dim), &device)?;
        let y = Tensor::from_vec(y_data, (1, dim), &device)?;

        let result = rls.update_koopman_rls(&phi, &y)?;
        // Disabled → always full weight
        assert!((result.wasserstein_weight - 1.0).abs() < 1e-6);
        Ok(())
    }

    /// Test: Wasserstein weight decreases monotonically with innovation norm
    #[test]
    fn test_wasserstein_weight_monotonicity() -> Result<()> {
        let device = Device::Cpu;
        let dim = 4;
        let epsilon_w = 0.1;
        let config = KoopmanRLSConfig::default()
            .with_wasserstein_radius(epsilon_w)
            .with_dead_zone(0.0);

        let innovations = vec![0.01, 0.05, 0.1, 0.2, 0.5, 1.0];
        let mut weights = Vec::new();

        for &innov_scale in &innovations {
            let mut rls = KoopmanRLS::new(dim, config.clone(), &device)?;
            let phi_data: Vec<f64> = (0..dim).map(|_| 0.0).collect();
            let y_data: Vec<f64> = (0..dim).map(|_| innov_scale).collect();
            let phi = Tensor::from_vec(phi_data, (1, dim), &device)?;
            let y = Tensor::from_vec(y_data, (1, dim), &device)?;

            let result = rls.update_koopman_rls(&phi, &y)?;
            weights.push(result.wasserstein_weight);
        }

        // Verify monotonic decrease
        for i in 1..weights.len() {
            assert!(weights[i] <= weights[i - 1] + 1e-6,
                "Weight should decrease with innovation: w[{}] = {:.6} > w[{}] = {:.6}",
                i, weights[i], i - 1, weights[i - 1]);
        }
        // First weight should be 1.0 (smallest innovation)
        assert!((weights[0] - 1.0).abs() < 1e-6);
        Ok(())
    }

    /// Test: Wasserstein weight in dead-zone path
    #[test]
    fn test_wasserstein_dead_zone() -> Result<()> {
        let device = Device::Cpu;
        let dim = 4;
        let config = KoopmanRLSConfig::default()
            .with_wasserstein_radius(0.1)
            .with_dead_zone(1.0); // Large dead zone
        let mut rls = KoopmanRLS::new(dim, config, &device)?;

        // Small innovation → falls into dead zone
        let phi_data: Vec<f64> = (0..dim).map(|i| i as f64 * 0.001).collect();
        let y_data: Vec<f64> = (0..dim).map(|i| i as f64 * 0.001 + 0.0001).collect();
        let phi = Tensor::from_vec(phi_data, (1, dim), &device)?;
        let y = Tensor::from_vec(y_data, (1, dim), &device)?;

        let result = rls.update_koopman_rls(&phi, &y)?;
        assert!(!result.updated);
        // Wasserstein weight should still be computed even in dead zone
        assert!(result.wasserstein_weight > 0.0);
        assert!(result.wasserstein_weight <= 1.0);
        Ok(())
    }

    /// Test: with_wasserstein_radius clamps negative values to 0
    #[test]
    fn test_wasserstein_radius_negative_clamp() {
        let config = KoopmanRLSConfig::default().with_wasserstein_radius(-0.5);
        assert_eq!(config.wasserstein_radius, 0.0);
    }

    /// Test: Display includes wasserstein_weight
    #[test]
    fn test_rls_update_result_display_wasserstein() -> Result<()> {
        let device = Device::Cpu;
        let dim = 4;
        let config = KoopmanRLSConfig::default()
            .with_wasserstein_radius(0.05)
            .with_dead_zone(0.0);
        let mut rls = KoopmanRLS::new(dim, config, &device)?;

        let phi_data: Vec<f64> = (0..dim).map(|_| 0.0).collect();
        let y_data: Vec<f64> = (0..dim).map(|_| 1.0).collect();
        let phi = Tensor::from_vec(phi_data, (1, dim), &device)?;
        let y = Tensor::from_vec(y_data, (1, dim), &device)?;

        let result = rls.update_koopman_rls(&phi, &y)?;
        let display = format!("{}", result);
        assert!(display.contains("w_W="));
        Ok(())
    }

    /// Integration: Wasserstein robustness attenuates adversarial updates
    #[test]
    fn test_wasserstein_robustness_integration() -> Result<()> {
        let device = Device::Cpu;
        let dim = 8;
        // Use large epsilon_W so first update (cold start) is not attenuated
        let epsilon_w = 5.0;

        // Standard RLS (no Wasserstein)
        let config_std = KoopmanRLSConfig::default()
            .with_wasserstein_radius(f64::MAX)
            .with_dead_zone(0.0);
        let mut rls_std = KoopmanRLS::new(dim, config_std, &device)?;

        // Wasserstein robust RLS
        let config_robust = KoopmanRLSConfig::default()
            .with_wasserstein_radius(epsilon_w)
            .with_dead_zone(0.0);
        let mut rls_robust = KoopmanRLS::new(dim, config_robust, &device)?;

        // Normal data first — both should learn similarly (innovation < ε_W)
        let phi_normal: Vec<f64> = (0..dim).map(|i| i as f64 * 0.1).collect();
        let y_normal: Vec<f64> = (0..dim).map(|i| (i + 1) as f64 * 0.1).collect();
        let phi_t = Tensor::from_vec(phi_normal.clone(), (1, dim), &device)?;
        let y_t = Tensor::from_vec(y_normal.clone(), (1, dim), &device)?;

        let r_std_normal = rls_std.update_koopman_rls(&phi_t, &y_t)?;
        let r_robust_normal = rls_robust.update_koopman_rls(&phi_t, &y_t)?;

        // Standard always w=1.0
        assert!((r_std_normal.wasserstein_weight - 1.0).abs() < 1e-6);
        // Robust: innovation norm ≈ 1.43 < ε_W=5.0 → w=1.0
        assert!((r_robust_normal.wasserstein_weight - 1.0).abs() < 1e-6);

        // Adversarial injection — very large innovation
        let phi_adv: Vec<f64> = (0..dim).map(|i| i as f64 * 0.1).collect();
        let y_adv: Vec<f64> = (0..dim).map(|_| 100.0).collect();
        let phi_adv_t = Tensor::from_vec(phi_adv, (1, dim), &device)?;
        let y_adv_t = Tensor::from_vec(y_adv, (1, dim), &device)?;

        let r_std_adv = rls_std.update_koopman_rls(&phi_adv_t, &y_adv_t)?;
        let r_robust_adv = rls_robust.update_koopman_rls(&phi_adv_t, &y_adv_t)?;

        // Standard: full weight (no protection)
        assert!((r_std_adv.wasserstein_weight - 1.0).abs() < 1e-6);
        // Robust: innovation ≈ sqrt(8)*100 >> ε_W=5.0 → heavily attenuated
        assert!(r_robust_adv.wasserstein_weight < 0.1);
        // Robust result should show much smaller effective update
        assert!(r_robust_adv.wasserstein_weight < r_std_adv.wasserstein_weight);
        Ok(())
    }
}
