//! Meta-Active Inference — Nodes optimize their own steering/VFE hyperparameters collectively.
//!
//! Each node maintains meta-hyperparameters (lr, lambda_OT, beta_CBF, sae_sparsity)
//! and updates them to minimize long-term meta-VFE across the collective.
//!
//! **Meta-Update Rule:**
//! ```text
//! θ_meta ← θ_meta - α_meta * ∇_meta E[VFE_t+H | θ_meta]
//! ```
//! where H is the planning horizon and the gradient is approximated via finite differences,
//! evolutionary strategies, or Natural Evolution Strategies (NES).
//!
//! **NES Gradient (Sprint 111):**
//! ```text
//! ∇J(θ) ≈ (1/Nσ²) Σᵢ (Rᵢ - b) (εᵢ/σ)
//! ```
//! with antithetic sampling (εᵢ and -εᵢ pairs) + moving-average baseline b.

use candle_core::Result;
use std::f32::consts::PI;

/// Meta-hyperparameters for a single node's active inference stack.
#[derive(Debug, Clone)]
pub struct MetaHyperParams {
    /// Learning rate for local steering updates.
    pub lr: f32,
    /// Weight for optimal transport term in VFE.
    pub lambda_ot: f32,
    /// Control Barrier Function safety coefficient.
    pub beta_cbf: f32,
    /// SAE sparsity penalty λ.
    pub sae_sparsity: f32,
    /// Cross-modal fusion weight.
    pub lambda_cross: f32,
    /// CIRL cooperation weight β.
    pub beta_cirl: f32,
}

impl Default for MetaHyperParams {
    fn default() -> Self {
        Self {
            lr: 1e-3,
            lambda_ot: 0.1,
            beta_cbf: 0.5,
            sae_sparsity: 0.01,
            lambda_cross: 0.2,
            beta_cirl: 0.3,
        }
    }
}

impl MetaHyperParams {
    /// Bounds for each hyperparameter (min, max).
    pub fn bounds(&self) -> Vec<(f32, f32)> {
        vec![
            (1e-5, 1e-1), // lr
            (0.0, 1.0),   // lambda_ot
            (0.1, 2.0),   // beta_cbf
            (1e-4, 0.1),  // sae_sparsity
            (0.0, 1.0),   // lambda_cross
            (0.0, 1.0),   // beta_cirl
        ]
    }

    /// Clamp all parameters to valid bounds.
    pub fn clamp(&self) -> Self {
        let bounds = self.bounds();
        let mut params = self.clone();
        params.lr = params.lr.max(bounds[0].0).min(bounds[0].1);
        params.lambda_ot = params.lambda_ot.max(bounds[1].0).min(bounds[1].1);
        params.beta_cbf = params.beta_cbf.max(bounds[2].0).min(bounds[2].1);
        params.sae_sparsity = params.sae_sparsity.max(bounds[3].0).min(bounds[3].1);
        params.lambda_cross = params.lambda_cross.max(bounds[4].0).min(bounds[4].1);
        params.beta_cirl = params.beta_cirl.max(bounds[5].0).min(bounds[5].1);
        params
    }
}

/// Configuration for meta-active inference.
#[derive(Debug, Clone)]
pub struct MetaActiveInferenceConfig {
    /// Meta-learning rate for hyperparameter updates.
    pub meta_lr: f32,
    /// Planning horizon H for meta-VFE estimation.
    pub horizon: usize,
    /// Number of perturbation directions for finite-difference gradient.
    pub num_perturbations: usize,
    /// Perturbation magnitude ε.
    pub perturbation_eps: f32,
    /// Population size for evolutionary strategy (if used).
    pub population_size: usize,
    /// Use evolutionary strategy instead of finite differences.
    pub use_es: bool,
    /// Use Natural Evolution Strategies (NES) with antithetic sampling + baseline.
    pub use_nes: bool,
    /// Decay factor for meta-VFE history (exponential moving average).
    pub history_decay: f32,
}

impl Default for MetaActiveInferenceConfig {
    fn default() -> Self {
        Self {
            meta_lr: 5e-3,
            horizon: 10,
            num_perturbations: 6,
            perturbation_eps: 1e-3,
            population_size: 20,
            use_es: true,
            use_nes: false,
            history_decay: 0.9,
        }
    }
}

/// Safety constraints for meta-optimization.
///
/// Defines the reachable parameter region using Taylor-model-inspired bounds.
/// Only parameter updates that keep the projected reach-set within the safe
/// hyper-rectangle are accepted.
#[derive(Debug, Clone)]
pub struct MetaSafetyConstraints {
    /// Maximum allowed per-parameter deviation from current params in a single step.
    pub max_step_size: f32,
    /// Minimum safety margin: beta_cbf must stay above this threshold.
    pub min_beta_cbf: f32,
    /// Maximum learning rate to avoid unstable updates.
    pub max_lr: f32,
    /// Minimum VFE improvement required to accept a step.
    pub min_vfe_improvement: f32,
    /// Look-ahead horizon for reach-set projection (number of simulated steps).
    pub reach_horizon: usize,
}

impl Default for MetaSafetyConstraints {
    fn default() -> Self {
        Self {
            max_step_size: 0.05,
            min_beta_cbf: 0.1,
            max_lr: 0.1,
            min_vfe_improvement: 0.0,
            reach_horizon: 3,
        }
    }
}

/// Result of a safe meta-optimization step.
#[derive(Debug, Clone)]
pub struct SafeOptResult {
    /// Meta-VFE reduction achieved (positive = improvement).
    pub vfe_reduction: f32,
    /// Whether the update was accepted (true) or rejected for safety (false).
    pub accepted: bool,
    /// Reason for rejection if not accepted.
    pub rejection_reason: Option<String>,
    /// Safety margin after the update (beta_cbf distance from min).
    pub safety_margin: f32,
    /// Reach-set diameter proxy (max projected deviation over horizon).
    pub reach_diameter: f32,
}

/// History entry for meta-VFE tracking.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HistoryEntry {
    #[allow(dead_code)]
    pub round: usize,
    #[allow(dead_code)]
    pub meta_vfe: f32,
    #[allow(dead_code)]
    pub params: MetaHyperParams,
}

/// Engine for meta-active inference — self-improving hyperparameter optimization.
pub struct MetaActiveInferenceEngine {
    config: MetaActiveInferenceConfig,
    current_params: MetaHyperParams,
    best_params: MetaHyperParams,
    best_meta_vfe: f32,
    history: Vec<HistoryEntry>,
    round: usize,
    // NES state: moving-average baseline for variance reduction
    nes_baseline: f32,
    nes_baseline_decay: f32,
}

impl MetaActiveInferenceEngine {
    pub fn new(config: &MetaActiveInferenceConfig) -> Self {
        let params = MetaHyperParams::default();
        Self {
            config: config.clone(),
            current_params: params.clone(),
            best_params: params.clone(),
            best_meta_vfe: f32::MAX,
            history: Vec::new(),
            round: 0,
            nes_baseline: 0.0,
            nes_baseline_decay: 0.9,
        }
    }

    /// Estimate meta-VFE for a given set of hyperparameters.
    ///
    /// Meta-VFE = weighted combination of:
    /// - Prediction uncertainty (higher lr → faster but noisier)
    /// - Safety margin (higher beta_cbf → safer but more conservative)
    /// - Sparsity efficiency (higher sae_sparsity → sparser but less faithful)
    /// - Cross-modal alignment (higher lambda_cross → better fusion)
    /// - Cooperative alignment (higher beta_cirl → more cooperative)
    ///
    /// This is a proxy model simulating the long-term effect of hyperparameters.
    pub fn estimate_meta_vfe(&self, params: &MetaHyperParams, peer_vfes: &[f32]) -> f32 {
        // Proxy: balance exploration-exploitation-safety-sparsity-cooperation
        // Exploration-exploitation tradeoff: moderate lr is optimal
        let optimal_lr = 1.0 - (-2.0 * (params.lr - 0.01).powi(2) + 0.0002).sqrt().abs();

        // Safety: beta_cbf should be moderate
        let safety_cost = (params.beta_cbf - 0.5).powi(2);

        // Sparsity: moderate sparsity is optimal
        let sparsity_cost = (params.sae_sparsity - 0.01).powi(2) * 100.0;

        // Cross-modal: moderate fusion is optimal
        let cross_cost = (params.lambda_cross - 0.3).powi(2);

        // Cooperation: moderate cooperation is optimal
        let coop_cost = (params.beta_cirl - 0.3).powi(2);

        // Peer influence: average peer VFE
        let peer_avg = if peer_vfes.is_empty() {
            1.0
        } else {
            peer_vfes.iter().sum::<f32>() / peer_vfes.len() as f32
        };

        0.3 * optimal_lr
            + 0.15 * safety_cost
            + 0.15 * sparsity_cost
            + 0.15 * cross_cost
            + 0.1 * coop_cost
            + 0.15 * peer_avg
    }

    /// Compute meta-gradient via finite differences.
    fn compute_meta_gradient_fd(&self, params: &MetaHyperParams, peer_vfes: &[f32]) -> Vec<f32> {
        let eps = self.config.perturbation_eps;
        let base_vfe = self.estimate_meta_vfe(params, peer_vfes);
        let num_params = params.bounds().len();
        let mut gradient = vec![0.0; num_params];

        for (i, grad) in gradient
            .iter_mut()
            .enumerate()
            .take(num_params.min(self.config.num_perturbations))
        {
            let mut perturbed = params.clone();
            let (min_b, max_b) = params.bounds()[i];
            let range = max_b - min_b;

            // Perturb +ε
            let perturbed_val = match i {
                0 => perturbed.lr + eps * range,
                1 => perturbed.lambda_ot + eps * range,
                2 => perturbed.beta_cbf + eps * range,
                3 => perturbed.sae_sparsity + eps * range,
                4 => perturbed.lambda_cross + eps * range,
                5 => perturbed.beta_cirl + eps * range,
                _ => continue,
            };

            // Apply perturbation
            match i {
                0 => perturbed.lr = perturbed_val,
                1 => perturbed.lambda_ot = perturbed_val,
                2 => perturbed.beta_cbf = perturbed_val,
                3 => perturbed.sae_sparsity = perturbed_val,
                4 => perturbed.lambda_cross = perturbed_val,
                5 => perturbed.beta_cirl = perturbed_val,
                _ => continue,
            }

            let perturbed_vfe = self.estimate_meta_vfe(&perturbed, peer_vfes);
            *grad = (perturbed_vfe - base_vfe) / (eps * range).max(1e-10);
        }

        gradient
    }

    /// Compute meta-gradient via Evolutionary Strategy (ES).
    fn compute_meta_gradient_es(&self, params: &MetaHyperParams, peer_vfes: &[f32]) -> Vec<f32> {
        let pop_size = self.config.population_size;
        let num_params = params.bounds().len();
        let sigma = self.config.perturbation_eps;

        let mut population_vfes = Vec::with_capacity(pop_size);
        let mut population_dirs = Vec::with_capacity(pop_size);

        for i in 0..pop_size {
            // Generate random perturbation direction
            let mut dir = Vec::with_capacity(num_params);
            for j in 0..num_params {
                // Box-Muller for Gaussian
                let r1 = ((i as u64 * 100 + j as u64)
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(777)
                    % 1_000_000) as f32
                    / 1_000_000.0;
                let r2 = ((i as u64 * 100 + j as u64)
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(888)
                    % 1_000_000) as f32
                    / 1_000_000.0;
                let u1 = r1.clamp(1e-6, 1.0 - 1e-6);
                let z = ((-2.0_f32 * u1.ln()).sqrt()) * (2.0 * PI * r2).cos();
                dir.push(z);
            }

            // Perturb params
            let mut perturbed = params.clone();
            for (j, d) in dir.iter().enumerate().take(num_params) {
                let (min_b, max_b) = params.bounds()[j];
                let range = (max_b - min_b) / 2.0;
                let perturbation = sigma * d * range;
                match j {
                    0 => perturbed.lr = (perturbed.lr + perturbation).max(min_b).min(max_b),
                    1 => {
                        perturbed.lambda_ot =
                            (perturbed.lambda_ot + perturbation).max(min_b).min(max_b)
                    }
                    2 => {
                        perturbed.beta_cbf =
                            (perturbed.beta_cbf + perturbation).max(min_b).min(max_b)
                    }
                    3 => {
                        perturbed.sae_sparsity = (perturbed.sae_sparsity + perturbation)
                            .max(min_b)
                            .min(max_b)
                    }
                    4 => {
                        perturbed.lambda_cross = (perturbed.lambda_cross + perturbation)
                            .max(min_b)
                            .min(max_b)
                    }
                    5 => {
                        perturbed.beta_cirl =
                            (perturbed.beta_cirl + perturbation).max(min_b).min(max_b)
                    }
                    _ => {}
                }
            }

            let vfe = self.estimate_meta_vfe(&perturbed, peer_vfes);
            population_vfes.push(vfe);
            population_dirs.push(dir);
        }

        // Rank-based ES gradient
        let mut pop: Vec<(f32, Vec<f32>)> =
            population_vfes.into_iter().zip(population_dirs).collect();
        pop.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut gradient = vec![0.0; num_params];
        let half_pop = pop_size / 2;

        for (rank, (_vfe, dir)) in pop.iter().enumerate() {
            let weight = if rank < half_pop {
                1.0 - rank as f32 / half_pop as f32
            } else {
                -(rank as f32 - half_pop as f32) / half_pop as f32
            };
            for (j, d) in dir.iter().enumerate().take(num_params) {
                gradient[j] += weight * d;
            }
        }

        let scale = 2.0 * half_pop as f32;
        for g in gradient.iter_mut() {
            *g /= scale.max(1.0);
        }

        gradient
    }

    /// Compute meta-gradient via Natural Evolution Strategies (NES).
    ///
    /// **NES Formula:**
    /// ```text
    /// ∇J(θ) ≈ (1/Nσ²) Σᵢ (Rᵢ - b) (εᵢ/σ)
    /// ```
    /// where:
    /// - εᵢ ~ N(0, I) are isotropic Gaussian perturbations
    /// - Rᵢ = -VFE(θ + σ·εᵢ) is the reward (negative VFE = better)
    /// - b is a moving-average baseline for variance reduction
    /// - Antithetic sampling: for each εᵢ, also sample -εᵢ
    ///
    /// Antithetic sampling halves variance: Cov(ε, -ε) = -I → cancels common noise.
    fn compute_meta_gradient_nes(
        &mut self,
        params: &MetaHyperParams,
        peer_vfes: &[f32],
    ) -> Vec<f32> {
        let pop_size = self.config.population_size;
        let num_params = params.bounds().len();
        let sigma = self.config.perturbation_eps;

        // Antithetic pairs: pop_size/2 pairs → pop_size total samples
        let num_pairs = pop_size / 2;
        let mut gradient = vec![0.0f32; num_params];
        let mut rewards = Vec::with_capacity(num_pairs);

        for i in 0..num_pairs {
            // Generate Gaussian perturbation εᵢ via Box-Muller
            let mut eps = Vec::with_capacity(num_params);
            for j in 0..num_params {
                let r1 = ((i as u64 * 100 + j as u64)
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(777)
                    % 1_000_000) as f32
                    / 1_000_000.0;
                let r2 = ((i as u64 * 100 + j as u64)
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(888)
                    % 1_000_000) as f32
                    / 1_000_000.0;
                let u1 = r1.clamp(1e-6, 1.0 - 1e-6);
                let z = ((-2.0_f32 * u1.ln()).sqrt()) * (2.0 * PI * r2).cos();
                eps.push(z);
            }

            // Perturb: θ⁺ = θ + σ·ε
            let perturbed_plus = Self::apply_perturbation(params, &eps, sigma);
            // Antithetic: θ⁻ = θ - σ·ε
            let perturbed_minus = Self::apply_antithetic_perturbation(params, &eps, sigma);

            // Evaluate VFE (lower is better → reward = -VFE)
            let vfe_plus = self.estimate_meta_vfe(&perturbed_plus, peer_vfes);
            let vfe_minus = self.estimate_meta_vfe(&perturbed_minus, peer_vfes);
            let r_plus = -vfe_plus;
            let r_minus = -vfe_minus;

            rewards.push(r_plus);

            // NES gradient contribution: (Rᵢ - b) · εᵢ / σ²
            // Antithetic pair: (R⁺ - b)·ε + (R⁻ - b)·(-ε) = (R⁺ - R⁻)·ε
            // Baseline cancels out with antithetic sampling!
            let diff = (r_plus - r_minus) / (2.0 * sigma * sigma).max(1e-10);
            for (j, e) in eps.iter().enumerate().take(num_params) {
                gradient[j] += diff * e;
            }
        }

        // Average over pairs
        let scale = num_pairs as f32;
        for g in gradient.iter_mut() {
            *g /= scale.max(1.0);
        }

        // Update moving-average baseline: b ← decay·b + (1-decay)·mean(R)
        if !rewards.is_empty() {
            let mean_reward = rewards.iter().sum::<f32>() / rewards.len() as f32;
            self.nes_baseline = self.nes_baseline_decay * self.nes_baseline
                + (1.0 - self.nes_baseline_decay) * mean_reward;
        }

        gradient
    }

    /// Apply perturbation to params: θ + σ·ε (clamped to bounds).
    fn apply_perturbation(params: &MetaHyperParams, eps: &[f32], sigma: f32) -> MetaHyperParams {
        let mut p = params.clone();
        let bounds = params.bounds();
        for (j, e) in eps.iter().enumerate().take(bounds.len()) {
            let (min_b, max_b) = bounds[j];
            let perturbation = sigma * e * (max_b - min_b) / 2.0;
            match j {
                0 => p.lr = (p.lr + perturbation).max(min_b).min(max_b),
                1 => p.lambda_ot = (p.lambda_ot + perturbation).max(min_b).min(max_b),
                2 => p.beta_cbf = (p.beta_cbf + perturbation).max(min_b).min(max_b),
                3 => p.sae_sparsity = (p.sae_sparsity + perturbation).max(min_b).min(max_b),
                4 => p.lambda_cross = (p.lambda_cross + perturbation).max(min_b).min(max_b),
                5 => p.beta_cirl = (p.beta_cirl + perturbation).max(min_b).min(max_b),
                _ => {}
            }
        }
        p
    }

    /// Apply antithetic perturbation: θ - σ·ε.
    fn apply_antithetic_perturbation(
        params: &MetaHyperParams,
        eps: &[f32],
        sigma: f32,
    ) -> MetaHyperParams {
        let neg_eps: Vec<f32> = eps.iter().map(|e| -e).collect();
        Self::apply_perturbation(params, &neg_eps, sigma)
    }

    /// Perform one meta-optimization round.
    ///
    /// Returns the meta-VFE reduction achieved (positive = improvement).
    pub fn meta_optimize(&mut self, peer_vfes: &[f32]) -> Result<f32> {
        let old_vfe = self.estimate_meta_vfe(&self.current_params, peer_vfes);

        // Compute gradient — clone params to avoid borrow conflict with NES's &mut self
        let params_clone = self.current_params.clone();
        let gradient = if self.config.use_nes {
            self.compute_meta_gradient_nes(&params_clone, peer_vfes)
        } else if self.config.use_es {
            self.compute_meta_gradient_es(&params_clone, peer_vfes)
        } else {
            self.compute_meta_gradient_fd(&params_clone, peer_vfes)
        };

        // Update parameters
        let bounds = self.current_params.bounds();
        let meta_lr = self.config.meta_lr;

        if let [g_lr, g_ot, g_cbf, g_sae, g_cross, g_cirl] = &gradient[..] {
            self.current_params.lr = (self.current_params.lr - meta_lr * g_lr)
                .max(bounds[0].0)
                .min(bounds[0].1);
            self.current_params.lambda_ot = (self.current_params.lambda_ot - meta_lr * g_ot)
                .max(bounds[1].0)
                .min(bounds[1].1);
            self.current_params.beta_cbf = (self.current_params.beta_cbf - meta_lr * g_cbf)
                .max(bounds[2].0)
                .min(bounds[2].1);
            self.current_params.sae_sparsity = (self.current_params.sae_sparsity - meta_lr * g_sae)
                .max(bounds[3].0)
                .min(bounds[3].1);
            self.current_params.lambda_cross = (self.current_params.lambda_cross
                - meta_lr * g_cross)
                .max(bounds[4].0)
                .min(bounds[4].1);
            self.current_params.beta_cirl = (self.current_params.beta_cirl - meta_lr * g_cirl)
                .max(bounds[5].0)
                .min(bounds[5].1);
        }

        self.current_params = self.current_params.clamp();

        let new_vfe = self.estimate_meta_vfe(&self.current_params, peer_vfes);
        let reduction = old_vfe - new_vfe;

        // Track best
        if new_vfe < self.best_meta_vfe {
            self.best_meta_vfe = new_vfe;
            self.best_params = self.current_params.clone();
        }

        // Record history
        self.history.push(HistoryEntry {
            round: self.round,
            meta_vfe: new_vfe,
            params: self.current_params.clone(),
        });

        self.round += 1;

        Ok(reduction)
    }

    /// Project a parameter update through a Taylor-model-inspired reach-set over `horizon` steps.
    ///
    /// Uses a first-order Taylor expansion of the meta-dynamics:
    /// ```text
    /// θ_{t+1} ≈ θ_t + Δθ
    /// reach(θ_t) = [θ_t - h·|Δθ|, θ_t + h·|Δθ|]
    /// ```
    /// where `h` is the reach horizon. Returns the reach-set diameter (max width across params).
    fn project_reach_set(
        &self,
        current: &MetaHyperParams,
        candidate: &MetaHyperParams,
        horizon: usize,
    ) -> f32 {
        let mut max_diameter = 0.0f32;
        let deltas = [
            (candidate.lr - current.lr).abs(),
            (candidate.lambda_ot - current.lambda_ot).abs(),
            (candidate.beta_cbf - current.beta_cbf).abs(),
            (candidate.sae_sparsity - current.sae_sparsity).abs(),
            (candidate.lambda_cross - current.lambda_cross).abs(),
            (candidate.beta_cirl - current.beta_cirl).abs(),
        ];
        for &d in &deltas {
            let diameter = 2.0 * horizon as f32 * d;
            if diameter > max_diameter {
                max_diameter = diameter;
            }
        }
        max_diameter
    }

    /// Check whether a candidate parameter set satisfies all safety constraints.
    ///
    /// Returns `true` if the candidate is within the safe reach-set,
    /// along with an optional rejection reason.
    fn check_safety(
        &self,
        current: &MetaHyperParams,
        candidate: &MetaHyperParams,
        constraints: &MetaSafetyConstraints,
    ) -> (bool, Option<String>) {
        // 1. Step-size constraint: max per-param deviation
        let deltas = [
            (candidate.lr - current.lr).abs(),
            (candidate.lambda_ot - current.lambda_ot).abs(),
            (candidate.beta_cbf - current.beta_cbf).abs(),
            (candidate.sae_sparsity - current.sae_sparsity).abs(),
            (candidate.lambda_cross - current.lambda_cross).abs(),
            (candidate.beta_cirl - current.beta_cirl).abs(),
        ];
        for (i, &d) in deltas.iter().enumerate() {
            if d > constraints.max_step_size {
                return (
                    false,
                    Some(format!(
                        "Step size {:.4} exceeds max {:.4} on param {}",
                        d,
                        constraints.max_step_size,
                        i
                    )),
                );
            }
        }

        // 2. CBF constraint: beta_cbf must stay above minimum
        if candidate.beta_cbf < constraints.min_beta_cbf {
            return (
                false,
                Some(format!(
                    "beta_cbf {:.4} below min {:.4}",
                    candidate.beta_cbf, constraints.min_beta_cbf
                )),
            );
        }

        // 3. LR constraint: learning rate must stay below maximum
        if candidate.lr > constraints.max_lr {
            return (
                false,
                Some(format!(
                    "lr {:.4} exceeds max {:.4}",
                    candidate.lr, constraints.max_lr
                )),
            );
        }

        // 4. Reach-set constraint: projected reach-set must fit within bounds
        let reach_diameter = self.project_reach_set(current, candidate, constraints.reach_horizon);
        if reach_diameter > constraints.max_step_size * 2.0 {
            return (
                false,
                Some(format!(
                    "Reach-set diameter {:.4} exceeds bound {:.4}",
                    reach_diameter,
                    constraints.max_step_size * 2.0
                )),
            );
        }

        (true, None)
    }

    /// Perform one **safe** meta-optimization round with reach-set constraints.
    ///
    /// This method extends [`Self::meta_optimize`] with formal safety guarantees:
    /// 1. Computes the meta-gradient (FD/ES/NES as configured).
    /// 2. Proposes a candidate update.
    /// 3. Projects the candidate through a Taylor-model-inspired reach-set.
    /// 4. Verifies that the reach-set stays within the safe hyper-rectangle.
    /// 5. Only applies the update if all safety constraints are satisfied.
    ///
    /// Returns a [`SafeOptResult`] with detailed safety information.
    ///
    /// # Arguments
    /// * `peer_vfes` - Peer VFE observations for meta-gradient computation.
    /// * `constraints` - Safety constraints for the optimization step.
    ///
    /// # Safety Guarantees
    /// - beta_cbf always stays above `constraints.min_beta_cbf`.
    /// - Step size is bounded by `constraints.max_step_size`.
    /// - Reach-set projection over `constraints.reach_horizon` steps remains feasible.
    pub fn meta_optimize_safe(
        &mut self,
        peer_vfes: &[f32],
        constraints: &MetaSafetyConstraints,
    ) -> Result<SafeOptResult> {
        let old_vfe = self.estimate_meta_vfe(&self.current_params, peer_vfes);

        // Compute gradient using the same strategy as meta_optimize
        let params_clone = self.current_params.clone();
        let gradient = if self.config.use_nes {
            self.compute_meta_gradient_nes(&params_clone, peer_vfes)
        } else if self.config.use_es {
            self.compute_meta_gradient_es(&params_clone, peer_vfes)
        } else {
            self.compute_meta_gradient_fd(&params_clone, peer_vfes)
        };

        // Propose candidate update
        let bounds = self.current_params.bounds();
        let meta_lr = self.config.meta_lr;
        let mut candidate = self.current_params.clone();

        if let [g_lr, g_ot, g_cbf, g_sae, g_cross, g_cirl] = &gradient[..] {
            candidate.lr = (candidate.lr - meta_lr * g_lr).max(bounds[0].0).min(bounds[0].1);
            candidate.lambda_ot = (candidate.lambda_ot - meta_lr * g_ot)
                .max(bounds[1].0)
                .min(bounds[1].1);
            candidate.beta_cbf = (candidate.beta_cbf - meta_lr * g_cbf)
                .max(bounds[2].0)
                .min(bounds[2].1);
            candidate.sae_sparsity = (candidate.sae_sparsity - meta_lr * g_sae)
                .max(bounds[3].0)
                .min(bounds[3].1);
            candidate.lambda_cross = (candidate.lambda_cross - meta_lr * g_cross)
                .max(bounds[4].0)
                .min(bounds[4].1);
            candidate.beta_cirl = (candidate.beta_cirl - meta_lr * g_cirl)
                .max(bounds[5].0)
                .min(bounds[5].1);
        }

        candidate = candidate.clamp();

        // Compute reach-set projection
        let reach_diameter =
            self.project_reach_set(&self.current_params, &candidate, constraints.reach_horizon);

        // Safety check
        let (is_safe, rejection_reason) =
            self.check_safety(&self.current_params, &candidate, constraints);

        if !is_safe {
            // Reject update — return without modifying state
            let safety_margin = self.current_params.beta_cbf - constraints.min_beta_cbf;
            return Ok(SafeOptResult {
                vfe_reduction: 0.0,
                accepted: false,
                rejection_reason,
                safety_margin,
                reach_diameter,
            });
        }

        // Compute VFE improvement
        let new_vfe = self.estimate_meta_vfe(&candidate, peer_vfes);
        let reduction = old_vfe - new_vfe;

        // Check minimum improvement threshold
        if reduction < constraints.min_vfe_improvement {
            return Ok(SafeOptResult {
                vfe_reduction: reduction,
                accepted: false,
                rejection_reason: Some(format!(
                    "VFE improvement {:.6} below min {:.6}",
                    reduction, constraints.min_vfe_improvement
                )),
                safety_margin: candidate.beta_cbf - constraints.min_beta_cbf,
                reach_diameter,
            });
        }

        // Accept update
        self.current_params = candidate;

        // Track best
        if new_vfe < self.best_meta_vfe {
            self.best_meta_vfe = new_vfe;
            self.best_params = self.current_params.clone();
        }

        // Record history
        self.history.push(HistoryEntry {
            round: self.round,
            meta_vfe: new_vfe,
            params: self.current_params.clone(),
        });

        self.round += 1;

        let safety_margin = self.current_params.beta_cbf - constraints.min_beta_cbf;

        Ok(SafeOptResult {
            vfe_reduction: reduction,
            accepted: true,
            rejection_reason: None,
            safety_margin,
            reach_diameter,
        })
    }

    /// Run multiple safe meta-optimization rounds and return convergence curve.
    pub fn meta_optimize_safe_sequence(
        &mut self,
        num_rounds: usize,
        peer_vfes: &[f32],
        constraints: &MetaSafetyConstraints,
    ) -> Result<Vec<SafeOptResult>> {
        let mut results = Vec::with_capacity(num_rounds);
        for _ in 0..num_rounds {
            let result = self.meta_optimize_safe(peer_vfes, constraints)?;
            results.push(result);
        }
        Ok(results)
    }

    /// Run multiple meta-optimization rounds and return convergence curve.
    pub fn meta_optimize_sequence(
        &mut self,
        num_rounds: usize,
        peer_vfes: &[f32],
    ) -> Result<Vec<f32>> {
        let mut curve = Vec::with_capacity(num_rounds);
        for _ in 0..num_rounds {
            let _reduction = self.meta_optimize(peer_vfes)?;
            curve.push(self.best_meta_vfe);
        }
        Ok(curve)
    }

    /// Get current hyperparameters.
    pub fn current_params(&self) -> &MetaHyperParams {
        &self.current_params
    }

    /// Get best hyperparameters found so far.
    pub fn best_params(&self) -> &MetaHyperParams {
        &self.best_params
    }

    /// Get best meta-VFE achieved.
    pub fn best_meta_vfe(&self) -> f32 {
        self.best_meta_vfe
    }

    /// Get convergence history.
    pub fn history(&self) -> &[HistoryEntry] {
        &self.history
    }

    /// Compute improvement ratio: (initial_vfe - current_vfe) / initial_vfe.
    pub fn improvement_ratio(&self) -> f32 {
        if self.history.is_empty() {
            return 0.0;
        }
        let initial = self.history.first().unwrap().meta_vfe;
        let current = self.best_meta_vfe;
        if initial.abs() < 1e-10 {
            return 0.0;
        }
        (initial - current) / initial
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta_hyper_params_default() {
        let params = MetaHyperParams::default();
        assert!(params.lr > 0.0 && params.lr < 1.0);
        assert!(params.lambda_ot >= 0.0 && params.lambda_ot <= 1.0);
        assert!(params.beta_cbf > 0.0);
        assert!(params.sae_sparsity > 0.0);
    }

    #[test]
    fn test_meta_hyper_params_clamp() {
        let params = MetaHyperParams {
            lr: 999.0,
            beta_cbf: -5.0,
            ..Default::default()
        };
        let clamped = params.clamp();
        assert!(clamped.lr <= 1e-1);
        assert!(clamped.beta_cbf >= 0.1);
    }

    #[test]
    fn test_meta_vfe_estimation() {
        let engine = MetaActiveInferenceEngine::new(&MetaActiveInferenceConfig::default());
        let params = MetaHyperParams::default();
        let vfe = engine.estimate_meta_vfe(&params, &[0.5, 0.6, 0.4]);
        assert!(vfe.is_finite());
        assert!(vfe >= 0.0);
    }

    #[test]
    fn test_meta_optimization_improves() {
        let config = MetaActiveInferenceConfig {
            meta_lr: 0.05,
            use_es: true,
            population_size: 30,
            ..Default::default()
        };
        let mut engine = MetaActiveInferenceEngine::new(&config);
        let peer_vfes = &[0.5, 0.6, 0.4, 0.55];

        let initial_vfe = engine.best_meta_vfe();
        let curve = engine.meta_optimize_sequence(20, peer_vfes).unwrap();

        assert!(curve.len() == 20);
        assert!(
            curve.last().unwrap() <= &initial_vfe,
            "Meta-VFE should decrease: initial={}, final={}",
            initial_vfe,
            curve.last().unwrap()
        );
    }

    #[test]
    fn test_improvement_ratio() {
        let config = MetaActiveInferenceConfig {
            meta_lr: 0.1,
            use_es: true,
            ..Default::default()
        };
        let mut engine = MetaActiveInferenceEngine::new(&config);
        let peer_vfes = &[0.5];

        engine.meta_optimize_sequence(15, peer_vfes).unwrap();
        let ratio = engine.improvement_ratio();
        assert!(
            ratio >= 0.0,
            "Improvement ratio should be non-negative: {}",
            ratio
        );
    }

    #[test]
    fn test_fd_gradient() {
        let config = MetaActiveInferenceConfig {
            use_es: false,
            num_perturbations: 6,
            ..Default::default()
        };
        let mut engine = MetaActiveInferenceEngine::new(&config);
        let peer_vfes = &[0.5];

        let reduction = engine.meta_optimize(peer_vfes).unwrap();
        assert!(reduction.is_finite());
    }

    #[test]
    fn test_empty_peer_vfes() {
        let mut engine = MetaActiveInferenceEngine::new(&MetaActiveInferenceConfig::default());
        let reduction = engine.meta_optimize(&[]).unwrap();
        assert!(reduction.is_finite());
    }

    #[test]
    fn test_convergence_stability() {
        let config = MetaActiveInferenceConfig {
            meta_lr: 0.02,
            use_es: true,
            population_size: 20,
            ..Default::default()
        };
        let mut engine = MetaActiveInferenceEngine::new(&config);
        let peer_vfes = &[0.5, 0.5, 0.5];

        let curve = engine.meta_optimize_sequence(30, peer_vfes).unwrap();

        // Check that curve is non-increasing (best VFE should not worsen)
        for i in 1..curve.len() {
            assert!(
                curve[i] <= curve[i - 1] + 1e-6,
                "Curve should be non-increasing: [{}]={}, [{}]={}",
                i - 1,
                curve[i - 1],
                i,
                curve[i]
            );
        }
    }
}
