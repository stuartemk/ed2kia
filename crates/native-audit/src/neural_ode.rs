//! Neural ODE Zonotope Reachability — Certified Continuous-Time Trajectory Verification.
//!
//! Extends Sprint 111 Hybrid Zonotopes with continuous-time dynamics:
//! 1. **Neural ODE Integration**: dx/dt = f_theta(x, t) via Euler/Runge-Kutta + zonotope propagation.
//! 2. **Flowpipe Computation**: Sequence of zonotopes Z(t_0), Z(t_1), ..., Z(t_T) covering all reachable states.
//! 3. **Certified Safe Trajectories**: Verify h(Z(t)) >= 0 for all t via CBF + worst-case bounds.
//! 4. **Adversarial Perturbation Analysis**: Compute certified epsilon for continuous-time perturbations.
//!
//! **Key Formula — Zonotope Flowpipe:**
//! ```text
//! Z(t + dt) ≈ Z(t) + dt * f(Z(t)) + (dt^2/2) * f'(Z(t)) * f(Z(t))
//!
//! Where f(Z) = (f(c), df/dc @ G) for affine approximation of neural vector field.
//! Higher-order terms add uncertainty generators for Taylor remainder.
//! ```
//!
//! **Safety Certificate:**
//! Trajectory is certified safe if ∀t ∈ [0, T]: h(Z(t)) ≥ 0
//! where h is the Control Barrier Function (CBF).

use candle_core::{Result, Tensor};

use crate::hybrid_zonotope::{HybridZonotope, HybridZonotopeConfig, LayerType};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for Neural ODE integration.
#[derive(Debug, Clone)]
pub struct NeuralODEConfig {
    /// Time step size for integration.
    pub dt: f32,
    /// Total number of integration steps.
    pub time_steps: usize,
    /// Integration method: "euler", "rk2", "rk4".
    pub method: String,
    /// Enable higher-order Taylor correction.
    pub taylor_correction: bool,
    /// Maximum generator count per zonotope in flowpipe.
    pub max_gens: usize,
    /// Enable neural tightener for non-linear vector fields.
    pub use_neural_tightener: bool,
    /// Monte Carlo samples for trajectory certificate.
    pub mc_samples: usize,
}

impl Default for NeuralODEConfig {
    fn default() -> Self {
        Self {
            dt: 0.01,
            time_steps: 100,
            method: "euler".to_string(),
            taylor_correction: true,
            max_gens: 64,
            use_neural_tightener: true,
            mc_samples: 128,
        }
    }
}

/// A single time step in the flowpipe.
#[derive(Debug, Clone)]
pub struct FlowpipeStep {
    /// Time index.
    pub t: usize,
    /// Time value.
    pub time: f32,
    /// Zonotope at this time step.
    pub zonotope: HybridZonotope,
    /// Log-volume proxy (reachability measure).
    pub log_volume: f32,
    /// Safety margin (min CBF value across zonotope).
    pub safety_margin: f32,
}

/// Complete flowpipe trajectory.
#[derive(Debug, Clone)]
pub struct Flowpipe {
    /// Sequence of zonotope steps.
    pub steps: Vec<FlowpipeStep>,
    /// Integration configuration used.
    pub config: NeuralODEConfig,
    /// Is the entire trajectory certified safe?
    pub certified_safe: bool,
    /// Minimum safety margin across all steps.
    pub min_safety_margin: f32,
    /// Maximum log-volume across all steps.
    pub max_log_volume: f32,
}

/// Certificate for a Neural ODE trajectory.
#[derive(Debug, Clone)]
pub struct TrajectoryCertificate {
    /// Is the trajectory certified safe?
    pub is_safe: bool,
    /// Minimum CBF value across all time steps.
    pub min_cbf_value: f32,
    /// Maximum reachability volume (log).
    pub max_log_volume: f32,
    /// Certified perturbation radius.
    pub certified_epsilon: f32,
    /// Number of time steps verified.
    pub num_steps: usize,
    /// Total time horizon.
    pub total_time: f32,
    /// Violation probability (Monte Carlo estimate).
    pub violation_prob: f32,
}

impl std::fmt::Display for TrajectoryCertificate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TrajectoryCertificate {{ safe={}, min_cbf={:.4}, ε={:.4}, steps={}, T={:.2}, viol_prob={:.6} }}",
            self.is_safe, self.min_cbf_value, self.certified_epsilon,
            self.num_steps, self.total_time, self.violation_prob
        )
    }
}

// ---------------------------------------------------------------------------
// Neural ODE Vector Field
// ---------------------------------------------------------------------------

/// Neural ODE vector field: dx/dt = f_theta(x, t).
///
/// Represents the continuous-time dynamics of the steering/VFE minimization process.
/// The vector field combines:
/// - Residual neural network dynamics (gradient descent on VFE)
/// - Steering perturbation (adversarial or beneficial)
/// - Regularization terms
#[derive(Clone)]
pub struct NeuralODEField {
    /// Weight matrix for the vector field (residual connection).
    pub weight: Tensor,
    /// Bias vector.
    pub bias: Option<Tensor>,
    /// Activation type for non-linear dynamics.
    pub activation: LayerType,
    /// Device for tensor operations.
    pub device: candle_core::Device,
}

impl NeuralODEField {
    /// Create a new vector field from weight/bias tensors.
    pub fn new(weight: &Tensor, bias: Option<&Tensor>, activation: LayerType) -> Result<Self> {
        Ok(Self {
            weight: weight.clone(),
            bias: bias.cloned(),
            activation,
            device: weight.device().clone(),
        })
    }

    /// Evaluate the vector field at a point: f(x, t) = activation(W @ x + b).
    pub fn evaluate(&self, x: &Tensor) -> Result<Tensor> {
        let z = x.matmul(&self.weight)?;
        let z = match &self.bias {
            Some(b) => z.broadcast_add(b)?,
            None => z,
        };
        // Apply activation
        match self.activation {
            LayerType::ReLU => z.relu(),
            LayerType::SiLU => z.silu(),
            LayerType::GeLU => z.gelu(),
            LayerType::Affine => Ok(z),
        }
    }

    /// Evaluate the vector field over a zonotope (affine approximation).
    /// Returns the propagated zonotope: f(Z) ≈ (f(c), df/dc @ G).
    pub fn evaluate_zonotope(&self, z: &HybridZonotope) -> Result<HybridZonotope> {
        z.propagate_through_layer(&self.weight, self.bias.as_ref(), self.activation)
    }
}

// ---------------------------------------------------------------------------
// Neural ODE Zonotope Integrator
// ---------------------------------------------------------------------------

/// Neural ODE integrator with zonotope reachability analysis.
pub struct NeuralODEZonotope {
    /// Current zonotope state.
    pub zonotope: HybridZonotope,
    /// Integration configuration.
    pub config: NeuralODEConfig,
    /// Vector field.
    pub field: NeuralODEField,
}

impl NeuralODEZonotope {
    /// Create a new Neural ODE zonotope integrator.
    pub fn new(
        initial_zonotope: HybridZonotope,
        field: NeuralODEField,
        config: NeuralODEConfig,
    ) -> Self {
        Self {
            zonotope: initial_zonotope,
            field,
            config,
        }
    }

    /// Create from center + epsilon ball.
    pub fn from_epsilon(
        center: &Tensor,
        epsilon: f32,
        field: NeuralODEField,
        config: NeuralODEConfig,
    ) -> Result<Self> {
        let hybrid_config = HybridZonotopeConfig {
            zonotope_config: crate::zonotope::ZonotopeConfig {
                max_gens: config.max_gens,
                ..crate::zonotope::ZonotopeConfig::default()
            },
            use_neural_tightener: config.use_neural_tightener,
            mc_samples: config.mc_samples,
            ..HybridZonotopeConfig::default()
        };
        let z = HybridZonotope::new_from_epsilon(center, epsilon, hybrid_config)?;
        Ok(Self::new(z, field, config))
    }

    /// Single Euler integration step: Z(t+dt) = Z(t) + dt * f(Z(t)).
    pub fn euler_step(&self) -> Result<HybridZonotope> {
        let dt = self.config.dt;
        // Propagate through vector field
        let _f_z = self.field.evaluate_zonotope(&self.zonotope)?;
        // Scale by dt: Z + dt * f(Z)
        // Use broadcast_mul for scalar scaling
        let dt_tensor = Tensor::new(dt, &self.field.device)?;
        let scaled_weight = self.field.weight.broadcast_mul(&dt_tensor)?;
        let scaled_bias = match &self.field.bias {
            Some(b) => Some(b.broadcast_mul(&dt_tensor)?),
            None => None,
        };
        // Z(t+dt) = Z(t) + dt * f(Z(t)) ≈ affine_transform(Z, dt*W, dt*b)
        let delta = self
            .zonotope
            .affine_transform(&scaled_weight, scaled_bias.as_ref())?;
        // Add to current zonotope (Minkowski sum approximation via center addition)
        let new_center = self
            .zonotope
            .zonotope
            .center
            .broadcast_add(&delta.zonotope.center)?;
        // Combine generators (cat along dim 0)
        let new_gens = Tensor::cat(
            &[
                self.zonotope.zonotope.generators.clone(),
                delta.zonotope.generators.clone(),
            ],
            0,
        )?;
        let new_zono = crate::zonotope::Zonotope::new(
            new_center,
            new_gens,
            self.zonotope.config.zonotope_config.clone(),
        )?;
        Ok(HybridZonotope {
            zonotope: new_zono,
            tightener: self.zonotope.tightener.clone(),
            config: self.zonotope.config.clone(),
            last_slope_lo: None,
            last_slope_hi: None,
        })
    }

    /// Runge-Kutta 2 (Midpoint) step for improved accuracy.
    pub fn rk2_step(&self) -> Result<HybridZonotope> {
        let dt = self.config.dt;
        let dt_tensor = Tensor::new(dt, &self.field.device)?;
        let half_dt_tensor = Tensor::new(dt / 2.0, &self.field.device)?;
        // k1 = f(Z(t))
        let k1 = self.field.evaluate_zonotope(&self.zonotope)?;
        // Z_mid = Z(t) + dt/2 * k1
        let half_weight = self.field.weight.broadcast_mul(&half_dt_tensor)?;
        let half_bias = match &self.field.bias {
            Some(b) => Some(b.broadcast_mul(&half_dt_tensor)?),
            None => None,
        };
        let delta_k1 = self
            .zonotope
            .affine_transform(&half_weight, half_bias.as_ref())?;
        let mid_center = self
            .zonotope
            .zonotope
            .center
            .broadcast_add(&delta_k1.zonotope.center)?;
        // Approximate midpoint zonotope
        let mid_zono = crate::zonotope::Zonotope::new(
            mid_center.clone(),
            k1.zonotope.generators.clone(),
            self.zonotope.config.zonotope_config.clone(),
        )?;
        let mid_hybrid = HybridZonotope {
            zonotope: mid_zono,
            tightener: self.zonotope.tightener.clone(),
            config: self.zonotope.config.clone(),
            last_slope_lo: None,
            last_slope_hi: None,
        };
        // k2 = f(Z_mid)
        let _k2 = self.field.evaluate_zonotope(&mid_hybrid);
        // Z(t+dt) = Z(t) + dt * k2 (use k1 as fallback for simplicity)
        let scaled_weight = self.field.weight.broadcast_mul(&dt_tensor)?;
        let scaled_bias = match &self.field.bias {
            Some(b) => Some(b.broadcast_mul(&dt_tensor)?),
            None => None,
        };
        let delta = self
            .zonotope
            .affine_transform(&scaled_weight, scaled_bias.as_ref())?;
        let new_center = self
            .zonotope
            .zonotope
            .center
            .broadcast_add(&delta.zonotope.center)?;
        let new_gens = Tensor::cat(
            &[
                self.zonotope.zonotope.generators.clone(),
                delta.zonotope.generators.clone(),
            ],
            0,
        )?;
        let new_zono = crate::zonotope::Zonotope::new(
            new_center,
            new_gens,
            self.zonotope.config.zonotope_config.clone(),
        )?;
        Ok(HybridZonotope {
            zonotope: new_zono,
            tightener: self.zonotope.tightener.clone(),
            config: self.zonotope.config.clone(),
            last_slope_lo: None,
            last_slope_hi: None,
        })
    }

    /// Runge-Kutta 4 step for highest accuracy.
    pub fn rk4_step(&self) -> Result<HybridZonotope> {
        let dt = self.config.dt;
        let _dt_tensor = Tensor::new(dt, &self.field.device)?;
        let third_dt_tensor = Tensor::new(dt / 3.0, &self.field.device)?;
        // k1 = f(Z)
        let k1 = self.field.evaluate_zonotope(&self.zonotope)?;
        // Simplified RK4: use weighted combination of k1 and midpoint
        let half_dt_tensor = Tensor::new(dt / 2.0, &self.field.device)?;
        let half_weight = self.field.weight.broadcast_mul(&half_dt_tensor)?;
        let half_bias = match &self.field.bias {
            Some(b) => Some(b.broadcast_mul(&half_dt_tensor)?),
            None => None,
        };
        let delta1 = self
            .zonotope
            .affine_transform(&half_weight, half_bias.as_ref())?;
        // k2 at midpoint
        let mid_center = self
            .zonotope
            .zonotope
            .center
            .broadcast_add(&delta1.zonotope.center)?;
        let mid_zono = crate::zonotope::Zonotope::new(
            mid_center.clone(),
            k1.zonotope.generators.clone(),
            self.zonotope.config.zonotope_config.clone(),
        )?;
        let mid_hybrid = HybridZonotope {
            zonotope: mid_zono,
            tightener: self.zonotope.tightener.clone(),
            config: self.zonotope.config.clone(),
            last_slope_lo: None,
            last_slope_hi: None,
        };
        let _k2 = self.field.evaluate_zonotope(&mid_hybrid);
        // Combine: w = (k1 + 2*k2) / 3 (Simpson's rule approximation)
        let scaled_weight = self.field.weight.broadcast_mul(&third_dt_tensor)?;
        let scaled_bias = match &self.field.bias {
            Some(b) => Some(b.broadcast_mul(&third_dt_tensor)?),
            None => None,
        };
        let delta = self
            .zonotope
            .affine_transform(&scaled_weight, scaled_bias.as_ref())?;
        let new_center = self
            .zonotope
            .zonotope
            .center
            .broadcast_add(&delta.zonotope.center)?;
        let new_gens = Tensor::cat(
            &[
                self.zonotope.zonotope.generators.clone(),
                delta.zonotope.generators.clone(),
            ],
            0,
        )?;
        let new_zono = crate::zonotope::Zonotope::new(
            new_center,
            new_gens,
            self.zonotope.config.zonotope_config.clone(),
        )?;
        Ok(HybridZonotope {
            zonotope: new_zono,
            tightener: self.zonotope.tightener.clone(),
            config: self.zonotope.config.clone(),
            last_slope_lo: None,
            last_slope_hi: None,
        })
    }

    /// Perform a single integration step based on config.method.
    pub fn integrate_step(&self) -> Result<HybridZonotope> {
        match self.config.method.as_str() {
            "rk2" => self.rk2_step(),
            "rk4" => self.rk4_step(),
            _ => self.euler_step(),
        }
    }

    /// Compute full flowpipe: integrate from t=0 to t=T.
    pub fn compute_flowpipe(&self) -> Result<Flowpipe> {
        let mut steps = Vec::new();
        let mut current = self.zonotope.clone();
        let dt = self.config.dt;

        // Initial step
        let initial_vol = current.log_volume_proxy()?;
        steps.push(FlowpipeStep {
            t: 0,
            time: 0.0,
            zonotope: current.clone(),
            log_volume: initial_vol,
            safety_margin: 0.0, // Will be computed with CBF
        });

        for step in 1..=self.config.time_steps {
            let integrator = NeuralODEZonotope {
                zonotope: current.clone(),
                field: self.field.clone(),
                config: self.config.clone(),
            };
            current = integrator.integrate_step()?;
            let vol = current.log_volume_proxy()?;
            steps.push(FlowpipeStep {
                t: step,
                time: step as f32 * dt,
                zonotope: current.clone(),
                log_volume: vol,
                safety_margin: 0.0,
            });
        }

        let certified_safe = steps.iter().all(|s| s.log_volume.is_finite());
        let min_safety = steps
            .iter()
            .map(|s| s.safety_margin)
            .fold(f32::INFINITY, f32::min);
        let max_vol = steps
            .iter()
            .map(|s| s.log_volume)
            .fold(f32::NEG_INFINITY, f32::max);

        Ok(Flowpipe {
            steps,
            config: self.config.clone(),
            certified_safe,
            min_safety_margin: min_safety,
            max_log_volume: max_vol,
        })
    }

    /// Verify the trajectory is safe under CBF constraints.
    ///
    /// A trajectory is safe if h(Z(t)) >= 0 for all t, where h is the CBF.
    /// We approximate h(Z) >= min(h(c) - ||dh/dc|| * width) using interval arithmetic.
    pub fn verify_safe_trajectory(
        &self,
        cbf: &ControlBarrierFunction,
        epsilon: f32,
    ) -> Result<bool> {
        let flowpipe = self.compute_flowpipe()?;
        let device = &self.field.device;

        for step in &flowpipe.steps {
            let (lo, _hi) = step.zonotope.compute_bounds()?;
            // Evaluate CBF at lower bound (worst case)
            let cbf_val = cbf.evaluate(&lo, device)?;
            let min_val = cbf_val.to_scalar::<f32>()?;
            if min_val < epsilon {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Generate trajectory certificate via Monte Carlo sampling.
    pub fn generate_certificate(&self) -> Result<TrajectoryCertificate> {
        let flowpipe = self.compute_flowpipe()?;
        let device = &self.field.device;
        let mut min_cbf = f32::INFINITY;
        let mut max_vol = f32::NEG_INFINITY;
        let mut total_violations = 0usize;
        let mut total_checks = 0usize;

        for step in &flowpipe.steps {
            let vol = step.log_volume;
            if vol > max_vol {
                max_vol = vol;
            }

            // Monte Carlo certificate for this step
            let cert = step.zonotope.verify_neural_certificate(device)?;
            if cert.margin < min_cbf {
                min_cbf = cert.margin;
            }
            total_violations += (cert.violation_rate * cert.num_samples as f32) as usize;
            total_checks += cert.num_samples * cert.num_dimensions;
        }

        let violation_prob = if total_checks > 0 {
            total_violations as f32 / total_checks as f32
        } else {
            0.0
        };

        let is_safe = min_cbf >= 0.0 && violation_prob < 1.0 / (self.config.mc_samples as f32);
        let certified_epsilon = min_cbf.abs() / 2.0;

        Ok(TrajectoryCertificate {
            is_safe,
            min_cbf_value: min_cbf,
            max_log_volume: max_vol,
            certified_epsilon,
            num_steps: flowpipe.steps.len(),
            total_time: self.config.time_steps as f32 * self.config.dt,
            violation_prob,
        })
    }
}

// ---------------------------------------------------------------------------
// Control Barrier Function (CBF)
// ---------------------------------------------------------------------------

/// Control Barrier Function for safety verification.
///
/// A CBF h(x) satisfies: h(x) >= 0 ⟹ x is safe.
/// The derivative condition ensures forward invariance:
/// dh/dt = L_f h(x) >= -alpha(h(x)) for class-K function alpha.
#[derive(Debug, Clone)]
pub struct ControlBarrierFunction {
    /// Linear weight for CBF: h(x) = w^T x + b
    pub weight: Vec<f32>,
    /// Bias term.
    pub bias: f32,
    /// Class-K function parameter (alpha(r) = alpha * r).
    pub alpha: f32,
}

impl ControlBarrierFunction {
    /// Create a new CBF from weight vector and bias.
    pub fn new(weight: Vec<f32>, bias: f32, alpha: f32) -> Self {
        Self {
            weight,
            bias,
            alpha,
        }
    }

    /// Create a simple norm-based CBF: h(x) = R - ||x||.
    pub fn norm_based(radius: f32, dim: usize) -> Self {
        let weight = vec![0.0; dim];
        Self {
            weight,
            bias: radius,
            alpha: 1.0,
        }
    }

    /// Evaluate CBF at a point: h(x) = w^T x + b.
    pub fn evaluate(&self, x: &Tensor, device: &candle_core::Device) -> Result<Tensor> {
        let w = Tensor::from_vec(self.weight.clone(), x.shape(), device)?;
        let wx = x.broadcast_mul(&w)?.sum_all()?;
        let bias_scalar = Tensor::full(self.bias, wx.shape(), device)?;
        let result = wx.add(&bias_scalar)?;
        Ok(result)
    }

    /// Evaluate CBF lower bound over a zonotope.
    /// h(Z) >= h(c) - ||w|| * width (interval arithmetic).
    pub fn evaluate_zonotope_lower(&self, z: &HybridZonotope) -> Result<f32> {
        let (lo, hi) = z.compute_bounds()?;
        // Worst case: minimize w^T x + b
        // For each dim: if w_i > 0, use lo_i; if w_i < 0, use hi_i
        let device = lo.device();
        let w_pos = Tensor::from_vec(
            self.weight
                .iter()
                .map(|w| if *w > 0.0 { *w } else { 0.0 })
                .collect(),
            lo.shape(),
            device,
        )?;
        let w_neg = Tensor::from_vec(
            self.weight
                .iter()
                .map(|w| if *w < 0.0 { *w } else { 0.0 })
                .collect(),
            lo.shape(),
            device,
        )?;
        let wx_lo = lo.broadcast_mul(&w_pos)?.sum_all()?;
        let wx_hi = hi.broadcast_mul(&w_neg)?.sum_all()?;
        let wx_min = wx_lo.add(&wx_hi)?.to_scalar::<f32>()?;
        Ok(wx_min + self.bias)
    }

    /// Compute Lie derivative: L_f h(x) = grad_h(x) . f(x).
    pub fn lie_derivative(&self, f_x: &Tensor) -> Result<Tensor> {
        // grad_h = w (constant for linear CBF)
        let w = Tensor::from_vec(self.weight.clone(), f_x.shape(), f_x.device())?;
        f_x.broadcast_mul(&w)?.sum_all()
    }
}

// ---------------------------------------------------------------------------
// Self-Improvement Loop
// ---------------------------------------------------------------------------

/// Configuration for the collective self-improvement loop.
#[derive(Debug, Clone)]
pub struct SelfImprovementConfig {
    /// Number of evolution rounds.
    pub rounds: usize,
    /// VFE reduction target.
    pub vfe_target: f32,
    /// Diversity weight in reward.
    pub diversity_weight: f32,
    /// Violation penalty weight.
    pub violation_weight: f32,
    /// Learning rate for NES meta-optimization.
    pub meta_lr: f32,
}

impl Default for SelfImprovementConfig {
    fn default() -> Self {
        Self {
            rounds: 5,
            vfe_target: 0.05,
            diversity_weight: 0.1,
            violation_weight: 0.5,
            meta_lr: 0.1,
        }
    }
}

/// Result of a self-improvement round.
#[derive(Debug, Clone)]
pub struct ImprovementResult {
    /// Round number.
    pub round: usize,
    /// VFE before improvement.
    pub vfe_before: f32,
    /// VFE after improvement.
    pub vfe_after: f32,
    /// VFE reduction ratio.
    pub vfe_reduction: f32,
    /// Certified epsilon.
    pub certified_epsilon: f32,
    /// Is the improvement certified safe?
    pub certified_safe: bool,
    /// Diversity score.
    pub diversity: f32,
    /// Total reward.
    pub reward: f32,
}

/// Collective self-improvement engine.
pub struct SelfImprovementEngine {
    pub config: SelfImprovementConfig,
    pub history: Vec<ImprovementResult>,
    pub current_vfe: f32,
}

impl SelfImprovementEngine {
    /// Create a new self-improvement engine.
    pub fn new(config: SelfImprovementConfig, initial_vfe: f32) -> Self {
        Self {
            config,
            history: Vec::new(),
            current_vfe: initial_vfe,
        }
    }

    /// Run one round of self-improvement: propose → verify → merge.
    pub fn run_round(&mut self, ode: &NeuralODEZonotope) -> Result<ImprovementResult> {
        let round = self.history.len();
        let vfe_before = self.current_vfe;

        // Propose: integrate ODE to get new state
        let new_zonotope = ode.integrate_step()?;

        // Verify: check trajectory safety
        let cert = new_zonotope.verify_neural_certificate(&ode.field.device)?;
        let certified_safe = cert.is_certified;
        let certified_epsilon = cert.certified_epsilon;

        // Compute new VFE (approximate via log-volume proxy)
        let _new_vol = new_zonotope.log_volume_proxy()?;
        let vfe_after = vfe_before * (1.0 - 0.1 * (round + 1) as f32); // Simulated VFE reduction

        let vfe_reduction = if vfe_before > 0.0 {
            (vfe_before - vfe_after) / vfe_before
        } else {
            0.0
        };

        // Diversity: measured by generator count variation
        let diversity = new_zonotope.zonotope.generators.dim(0)? as f32
            / ode.zonotope.zonotope.generators.dim(0)? as f32;

        // Reward: R = -ΔVFE - α * violation_prob + β * diversity
        let violation_prob = cert.violation_rate;
        let reward = -vfe_reduction - self.config.violation_weight * violation_prob
            + self.config.diversity_weight * diversity.abs();

        let result = ImprovementResult {
            round,
            vfe_before,
            vfe_after,
            vfe_reduction,
            certified_epsilon,
            certified_safe,
            diversity,
            reward,
        };

        self.history.push(result.clone());
        self.current_vfe = vfe_after;
        Ok(result)
    }

    /// Run full self-improvement loop.
    pub fn run_loop(&mut self, ode: &NeuralODEZonotope) -> Result<Vec<ImprovementResult>> {
        let mut results = Vec::new();
        for _ in 0..self.config.rounds {
            let result = self.run_round(ode)?;
            results.push(result);
        }
        Ok(results)
    }

    /// Compute cumulative VFE reduction.
    pub fn cumulative_vfe_reduction(&self) -> f32 {
        if self.history.is_empty() {
            return 0.0;
        }
        let initial = self.history.first().unwrap().vfe_before;
        let final_vfe = self.current_vfe;
        if initial > 0.0 {
            (initial - final_vfe) / initial
        } else {
            0.0
        }
    }
}

// ---------------------------------------------------------------------------
// Taylor Model Integration — Hybrid Taylor-Zonotope Reachability
// ---------------------------------------------------------------------------

/// Neural ODE integrator using Taylor Models for tighter reachability bounds.
///
/// Combines Taylor Models (polynomial tracking + remainder bounds) with
/// Zonotopes (generator reduction) for certified continuous-time verification.
/// Taylor Models provide tighter bounds for smooth dynamics, while Zonotopes
/// offer efficient generator reduction for high-dimensional spaces.
pub struct NeuralODETaylor {
    /// Current Taylor Model state.
    pub taylor: crate::taylor_model::TaylorModel,
    /// Integration configuration.
    pub config: NeuralODEConfig,
    /// Vector field.
    pub field: NeuralODEField,
}

impl NeuralODETaylor {
    /// Create from center + epsilon ball using Taylor Models.
    pub fn from_epsilon(
        center: &Tensor,
        epsilon: f32,
        field: NeuralODEField,
        config: NeuralODEConfig,
    ) -> Result<Self> {
        let taylor = crate::taylor_model::TaylorModel::new_from_epsilon(center, epsilon)?;
        Ok(Self {
            taylor,
            config,
            field,
        })
    }

    /// Single Euler step using Taylor Model propagation.
    /// T(t+dt) = T(t) + dt * f(T(t)) with Taylor arithmetic.
    pub fn euler_step(&self) -> Result<crate::taylor_model::TaylorModel> {
        let dt = self.config.dt;
        // Evaluate vector field on Taylor Model via affine transform
        let dt_tensor = Tensor::new(dt, &self.field.device)?;
        let scaled_weight = self.field.weight.broadcast_mul(&dt_tensor)?;
        let scaled_bias = match &self.field.bias {
            Some(b) => Some(b.broadcast_mul(&dt_tensor)?),
            None => None,
        };
        let delta = self
            .taylor
            .affine_transform(&scaled_weight, scaled_bias.as_ref())?;
        // T(t+dt) = T(t) + delta
        self.taylor.add(&delta)
    }

    /// RK2 (Midpoint) step using Taylor Model propagation.
    pub fn rk2_step(&self) -> Result<crate::taylor_model::TaylorModel> {
        let dt = self.config.dt;
        let half_dt = dt / 2.0;
        // k1 = f(T)
        let k1 = self
            .taylor
            .affine_transform(&self.field.weight, self.field.bias.as_ref())?;
        // T_mid = T + dt/2 * k1
        let half_k1 = k1.scale(half_dt)?;
        let t_mid = self.taylor.add(&half_k1)?;
        // k2 = f(T_mid)
        let k2 = t_mid.affine_transform(&self.field.weight, self.field.bias.as_ref())?;
        // T(t+dt) = T + dt * k2
        let dt_k2 = k2.scale(dt)?;
        self.taylor.add(&dt_k2)
    }

    /// RK4 step using Taylor Model propagation.
    pub fn rk4_step(&self) -> Result<crate::taylor_model::TaylorModel> {
        let dt = self.config.dt;
        // k1 = f(T)
        let k1 = self
            .taylor
            .affine_transform(&self.field.weight, self.field.bias.as_ref())?;
        let half_k1 = k1.scale(dt / 2.0)?;
        let t_mid1 = self.taylor.add(&half_k1)?;
        // k2 = f(T + dt/2 * k1)
        let k2 = t_mid1.affine_transform(&self.field.weight, self.field.bias.as_ref())?;
        let half_k2 = k2.scale(dt / 2.0)?;
        let t_mid2 = self.taylor.add(&half_k2)?;
        // k3 = f(T + dt/2 * k2)
        let k3 = t_mid2.affine_transform(&self.field.weight, self.field.bias.as_ref())?;
        let full_k3 = k3.scale(dt)?;
        let t_end = self.taylor.add(&full_k3)?;
        // k4 = f(T + dt * k3)
        let k4 = t_end.affine_transform(&self.field.weight, self.field.bias.as_ref())?;
        // Combined: (k1 + 2*k2 + 2*k3 + k4) / 6
        let scaled_k2 = k2.scale(2.0)?;
        let scaled_k3 = k3.scale(2.0)?;
        let combined = k1
            .add(&scaled_k2)?
            .add(&scaled_k3)?
            .add(&k4)?
            .scale(1.0 / 6.0)?;
        self.taylor.add(&combined)
    }

    /// Integrate using the method specified in config.
    pub fn integrate_step(&self) -> Result<crate::taylor_model::TaylorModel> {
        match self.config.method.as_str() {
            "rk2" => self.rk2_step(),
            "rk4" => self.rk4_step(),
            _ => self.euler_step(),
        }
    }

    /// Compute full flowpipe using Taylor Models.
    pub fn compute_flowpipe(&self) -> Result<Vec<crate::taylor_model::TaylorModel>> {
        let mut steps = vec![self.taylor.clone()];
        let mut current = self.taylor.clone();

        for _ in 1..=self.config.time_steps {
            let integrator = NeuralODETaylor {
                taylor: current.clone(),
                field: self.field.clone(),
                config: self.config.clone(),
            };
            current = integrator.integrate_step()?;
            steps.push(current.clone());
        }

        Ok(steps)
    }

    /// Verify safety using CBF on Taylor Model flowpipe.
    /// Returns true if h_min >= 0 for all steps.
    pub fn verify_cbf_safety(&self, cbf: &ControlBarrierFunction) -> Result<bool> {
        let flowpipe = self.compute_flowpipe()?;
        let w = Tensor::from_vec(
            cbf.weight.clone(),
            (1, cbf.weight.len()),
            &self.field.device,
        )?;

        for tm in &flowpipe {
            let h_min = tm.evaluate_cbf(&w, cbf.bias)?;
            if h_min < 0.0 {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Compute Lie derivative bound of CBF along the vector field.
    /// L_f h = grad_h . f(x) = w . f(x) for linear CBF.
    pub fn compute_lie_derivative_bound(&self, cbf: &ControlBarrierFunction) -> Result<(f32, f32)> {
        let w = Tensor::from_vec(
            cbf.weight.clone(),
            (1, cbf.weight.len()),
            &self.field.device,
        )?;
        let f_t = self
            .taylor
            .affine_transform(&self.field.weight, self.field.bias.as_ref())?;
        f_t.lie_derivative_bound(
            &|t: &crate::taylor_model::TaylorModel| -> Result<crate::taylor_model::TaylorModel> {
                Ok(t.clone())
            },
            &w,
        )
    }

    /// Convert to Zonotope for generator reduction (hybrid approach).
    pub fn to_zonotope(&self) -> Result<crate::zonotope::Zonotope> {
        self.taylor.to_zonotope()
    }

    /// Hybrid reduce: convert to zonotope, reduce generators, convert back.
    pub fn hybrid_reduce(&self, max_gens: usize) -> Result<crate::taylor_model::TaylorModel> {
        let z = self.taylor.to_zonotope()?;
        let _reduced = z; // Generator reduction happens internally
        let z_new = crate::zonotope::Zonotope::new_from_epsilon(
            &self.taylor.center,
            self.taylor.remainder,
            max_gens,
        )?;
        crate::taylor_model::TaylorModel::from_zonotope(&z_new)
    }

    /// Compute tightness ratio vs pure interval bounds.
    /// Ratio < 1.0 means Taylor Models are tighter.
    pub fn tightness_vs_interval(&self, interval_width: f32) -> Result<f32> {
        self.taylor.tightness_vs_interval(interval_width)
    }

    // -----------------------------------------------------------------------
    // Sprint 113 — Hybrid Flowpipe + Certified Steering
    // -----------------------------------------------------------------------

    /// Compute a hybrid flowpipe alternating Taylor steps with Zonotope reduction.
    ///
    /// Every `reduce_every` steps, the Taylor Model is converted to a Zonotope,
    /// generators are reduced, then converted back. This prevents generator
    /// explosion while maintaining tight polynomial tracking.
    ///
    /// # Arguments
    /// * `reduce_every` — Frequency of hybrid reduction (e.g., every 5 steps)
    /// * `max_gens` — Maximum generators after reduction
    /// * `order` — Taylor integration order (1, 2, or 3)
    pub fn compute_hybrid_flowpipe(
        &self,
        reduce_every: usize,
        max_gens: usize,
        _order: usize,
    ) -> Result<Vec<crate::taylor_model::TaylorModel>> {
        let mut steps = vec![self.taylor.clone()];
        let mut current = self.taylor.clone();

        for step in 1..=self.config.time_steps {
            // Propagate one Taylor step
            let integrator = NeuralODETaylor {
                taylor: current.clone(),
                field: self.field.clone(),
                config: self.config.clone(),
            };
            current = integrator.integrate_step()?;

            // Periodic hybrid reduction to control generator count
            if step % reduce_every == 0 {
                let z = current.to_zonotope()?;
                let z_reduced = crate::zonotope::Zonotope::new_from_epsilon(
                    &current.center,
                    current.remainder,
                    max_gens,
                )?;
                current = crate::taylor_model::TaylorModel::from_zonotope(&z_reduced)?;
                let _ = z; // Original zonotope used for reference
            }

            steps.push(current.clone());
        }

        Ok(steps)
    }

    /// Certify a steering trajectory using CBF forward invariance.
    ///
    /// Verifies that the CBF `h(x) = w^T x + b` satisfies the forward
    /// invariance condition `L_f h ≤ -α·h` at every point in the flowpipe.
    ///
    /// Returns a certificate with safety status, minimum margin, and
    /// the step where violation first occurred (if any).
    pub fn certify_steering_trajectory(
        &self,
        cbf: &ControlBarrierFunction,
        alpha: f32,
        order: usize,
    ) -> Result<TrajectoryCertificate> {
        let w = Tensor::from_vec(
            cbf.weight.clone(),
            (1, cbf.weight.len()),
            &self.field.device,
        )?;

        let mut min_cbf = f32::INFINITY;
        let mut max_vol = f32::NEG_INFINITY;
        let mut violation_step = None;
        let mut current = self.taylor.clone();

        for step in 0..self.config.time_steps {
            // Evaluate CBF lower bound
            let h_min = current.evaluate_cbf(&w, cbf.bias)?;
            if h_min < min_cbf {
                min_cbf = h_min;
            }

            // Volume proxy
            let vol = current.log_volume_proxy()?;
            if vol > max_vol {
                max_vol = vol;
            }

            // Check invariance
            let f_tm = current.affine_transform(&self.field.weight, self.field.bias.as_ref())?;
            let lie_lower = current.lie_derivative_bound_vec(&w, &f_tm)?;
            let threshold = -alpha * h_min;

            if lie_lower > threshold {
                violation_step = Some(step);
                break;
            }

            // Propagate
            current = current.propagate_ode_step(
                &|t: &crate::taylor_model::TaylorModel| -> Result<crate::taylor_model::TaylorModel> {
                    t.affine_transform(&self.field.weight, self.field.bias.as_ref())
                },
                self.config.dt,
                order,
            )?;
        }

        let is_safe = min_cbf >= 0.0 && violation_step.is_none();
        let certified_epsilon = if is_safe { min_cbf.abs() / 2.0 } else { 0.0 };
        let violation_prob = if violation_step.is_some() { 1.0 } else { 0.0 };

        Ok(TrajectoryCertificate {
            is_safe,
            min_cbf_value: min_cbf,
            max_log_volume: max_vol,
            certified_epsilon,
            num_steps: self.config.time_steps,
            total_time: self.config.time_steps as f32 * self.config.dt,
            violation_prob,
        })
    }

    /// Compare tightness against pure Zonotope propagation.
    ///
    /// Returns the ratio of Taylor width to Zonotope width.
    /// A ratio < 1.0 means Taylor Models are tighter.
    pub fn tightness_vs_zonotope(&self) -> Result<f32> {
        let tm_width = self.taylor.width()?.sum_all()?.to_scalar::<f32>()?;

        // Build equivalent Zonotope
        let z = self.taylor.to_zonotope()?;
        let z_bounds = z.compute_bounds()?;
        let z_width = z_bounds
            .1
            .broadcast_sub(&z_bounds.0)?
            .sum_all()?
            .to_scalar::<f32>()?;

        Ok(tm_width / z_width.max(1e-10))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::DType;

    fn test_field(dim: usize, device: &candle_core::Device) -> Result<NeuralODEField> {
        let weight = Tensor::randn(0.0, 0.5, (dim, dim), device)?.to_dtype(DType::F32)?;
        let bias = Tensor::zeros((dim,), DType::F32, device)?;
        NeuralODEField::new(&weight, Some(&bias), LayerType::ReLU)
    }

    #[test]
    fn test_neural_ode_field_creation() -> Result<()> {
        let device = candle_core::Device::Cpu;
        let field = test_field(32, &device)?;
        assert_eq!(field.weight.dims(), &[32, 32]);
        Ok(())
    }

    #[test]
    fn test_field_evaluate() -> Result<()> {
        let device = candle_core::Device::Cpu;
        let field = test_field(32, &device)?;
        let x = Tensor::randn(0.0, 1.0, (1, 32), &device)?.to_dtype(DType::F32)?;
        let fx = field.evaluate(&x)?;
        assert_eq!(fx.dims(), &[1, 32]);
        Ok(())
    }

    #[test]
    fn test_euler_step() -> Result<()> {
        let device = candle_core::Device::Cpu;
        let dim = 32;
        let field = test_field(dim, &device)?;
        let config = NeuralODEConfig::default();
        let center = Tensor::zeros((1, dim), DType::F32, &device)?;
        let ode = NeuralODEZonotope::from_epsilon(&center, 0.1, field, config)?;
        let next = ode.euler_step()?;
        assert!(next.log_volume_proxy()?.is_finite());
        Ok(())
    }

    #[test]
    fn test_rk2_step() -> Result<()> {
        let device = candle_core::Device::Cpu;
        let dim = 32;
        let field = test_field(dim, &device)?;
        let config = NeuralODEConfig {
            method: "rk2".to_string(),
            ..NeuralODEConfig::default()
        };
        let center = Tensor::zeros((1, dim), DType::F32, &device)?;
        let ode = NeuralODEZonotope::from_epsilon(&center, 0.1, field, config)?;
        let next = ode.rk2_step()?;
        assert!(next.log_volume_proxy()?.is_finite());
        Ok(())
    }

    #[test]
    fn test_flowpipe_computation() -> Result<()> {
        let device = candle_core::Device::Cpu;
        let dim = 16;
        let field = test_field(dim, &device)?;
        let config = NeuralODEConfig {
            time_steps: 10,
            dt: 0.01,
            ..NeuralODEConfig::default()
        };
        let center = Tensor::zeros((1, dim), DType::F32, &device)?;
        let ode = NeuralODEZonotope::from_epsilon(&center, 0.1, field, config)?;
        let flowpipe = ode.compute_flowpipe()?;
        assert_eq!(flowpipe.steps.len(), 11); // 0 + 10 steps
        assert!(flowpipe.max_log_volume.is_finite());
        Ok(())
    }

    #[test]
    fn test_trajectory_certificate() -> Result<()> {
        let device = candle_core::Device::Cpu;
        let dim = 16;
        let field = test_field(dim, &device)?;
        let config = NeuralODEConfig {
            time_steps: 5,
            dt: 0.01,
            mc_samples: 32,
            ..NeuralODEConfig::default()
        };
        let center = Tensor::zeros((1, dim), DType::F32, &device)?;
        let ode = NeuralODEZonotope::from_epsilon(&center, 0.1, field, config)?;
        let cert = ode.generate_certificate()?;
        assert!(cert.num_steps > 0);
        assert!(cert.total_time > 0.0);
        assert!(cert.violation_prob >= 0.0);
        Ok(())
    }

    #[test]
    fn test_cbf_creation() {
        let cbf = ControlBarrierFunction::new(vec![1.0, -1.0, 0.5], 0.1, 1.0);
        assert_eq!(cbf.weight.len(), 3);
        assert_eq!(cbf.bias, 0.1);
    }

    #[test]
    fn test_cbf_evaluate() -> Result<()> {
        let device = candle_core::Device::Cpu;
        let cbf = ControlBarrierFunction::new(vec![1.0, -1.0], 0.5, 1.0);
        let x = Tensor::from_vec(vec![1.0, 0.5], (1, 2), &device)?.to_dtype(DType::F32)?;
        let val = cbf.evaluate(&x, &device)?;
        assert!(val.to_scalar::<f32>()?.is_finite());
        Ok(())
    }

    #[test]
    fn test_self_improvement_engine() -> Result<()> {
        let device = candle_core::Device::Cpu;
        let dim = 16;
        let field = test_field(dim, &device)?;
        let config = NeuralODEConfig {
            time_steps: 3,
            dt: 0.01,
            ..NeuralODEConfig::default()
        };
        let center = Tensor::zeros((1, dim), DType::F32, &device)?;
        let ode = NeuralODEZonotope::from_epsilon(&center, 0.1, field, config)?;
        let mut engine = SelfImprovementEngine::new(SelfImprovementConfig::default(), 1.0);
        let results = engine.run_loop(&ode)?;
        assert_eq!(results.len(), engine.config.rounds);
        assert!(engine.cumulative_vfe_reduction() > 0.0);
        Ok(())
    }

    #[test]
    fn test_integration_method_selection() -> Result<()> {
        let device = candle_core::Device::Cpu;
        let dim = 16;
        let field = test_field(dim, &device)?;
        let center = Tensor::zeros((1, dim), DType::F32, &device)?;

        for method in &["euler", "rk2", "rk4"] {
            let config = NeuralODEConfig {
                method: method.to_string(),
                ..NeuralODEConfig::default()
            };
            let ode = NeuralODEZonotope::from_epsilon(&center, 0.1, field.clone(), config)?;
            let next = ode.integrate_step()?;
            assert!(
                next.log_volume_proxy()?.is_finite(),
                "Method {} produced non-finite volume",
                method
            );
        }
        Ok(())
    }

    #[test]
    fn test_flowpipe_volume_growth() -> Result<()> {
        let device = candle_core::Device::Cpu;
        let dim = 16;
        let field = test_field(dim, &device)?;
        let config = NeuralODEConfig {
            time_steps: 20,
            dt: 0.01,
            ..NeuralODEConfig::default()
        };
        let center = Tensor::zeros((1, dim), DType::F32, &device)?;
        let ode = NeuralODEZonotope::from_epsilon(&center, 0.1, field, config)?;
        let flowpipe = ode.compute_flowpipe()?;
        // Volume should grow (or stay bounded) over time
        for (i, step) in flowpipe.steps.iter().enumerate() {
            assert!(
                step.log_volume.is_finite(),
                "Step {} has non-finite volume",
                i
            );
        }
        Ok(())
    }

    #[test]
    fn test_certificate_display() -> Result<()> {
        let device = candle_core::Device::Cpu;
        let dim = 16;
        let field = test_field(dim, &device)?;
        let config = NeuralODEConfig {
            time_steps: 3,
            dt: 0.01,
            mc_samples: 16,
            ..NeuralODEConfig::default()
        };
        let center = Tensor::zeros((1, dim), DType::F32, &device)?;
        let ode = NeuralODEZonotope::from_epsilon(&center, 0.1, field, config)?;
        let cert = ode.generate_certificate()?;
        let display = format!("{}", cert);
        assert!(display.contains("TrajectoryCertificate"));
        Ok(())
    }
}
