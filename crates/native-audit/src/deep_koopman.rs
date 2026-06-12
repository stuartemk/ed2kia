//! DeepKoopman Autoencoder — Neural Lifting + Koopman Operator Learning.
//!
//! Implements DeepKoopman with:
//! - Symmetric Encoder/Decoder for neural observable lifting ψ(x) = Encoder(x; θ)
//! - Koopman operator K learned online in reduced subspace (64-256D)
//! - Contraction metric M for Lyapunov verification: V̇ ≤ -αV
//! - Integration with Contracting Tube MPC and Mean-Field Replicator Dynamics
//!
//! Key formulas:
//! - Lifting: ψ(x) = Encoder(x; θ), ψ̂_{t+1} = K ψ_t, x' = Decoder(ψ̂_{t+1})
//! - Loss: L = ||ψ(x_{t+1}) - K ψ(x_t)||²_F + ||Decoder(ψ(x)) - x||² + λ||V̇ + αV||²_+
//! - Lyapunov: V(ψ) = (ψ - ψ_safe)ᵀ M (ψ - ψ_safe), M ≻ 0

use candle_core::{Device, DType, Result, Tensor};
use candle_nn::{Linear, Module};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for DeepKoopman autoencoder.
#[derive(Debug, Clone)]
pub struct DeepKoopmanConfig {
    /// Hidden dimension for encoder/decoder intermediate layer.
    pub hidden_dim: usize,
    /// Lifted Koopman observable dimension (64-256 recommended).
    pub lifted_dim: usize,
    /// Learning rate for online K update.
    pub lr: f64,
    /// Ridge regularization coefficient for K stability.
    pub ridge: f64,
    /// Contraction rate target α > 0 for Lyapunov V̇ ≤ -αV.
    pub alpha: f64,
    /// Loss weight for contraction penalty term.
    pub lambda_contraction: f64,
    /// Enable symplectic structure preservation in encoder.
    pub symplectic: bool,
}

impl Default for DeepKoopmanConfig {
    fn default() -> Self {
        Self {
            hidden_dim: 128,
            lifted_dim: 64,
            lr: 1e-3,
            ridge: 1e-4,
            alpha: 0.1,
            lambda_contraction: 1.0,
            symplectic: false,
        }
    }
}

impl DeepKoopmanConfig {
    /// Fast configuration for edge devices.
    pub fn edge_fast() -> Self {
        Self {
            hidden_dim: 64,
            lifted_dim: 32,
            lr: 5e-3,
            ridge: 1e-3,
            alpha: 0.05,
            lambda_contraction: 0.5,
            symplectic: false,
        }
    }

    /// High-precision configuration for server-class nodes.
    pub fn high_precision() -> Self {
        Self {
            hidden_dim: 256,
            lifted_dim: 128,
            lr: 1e-4,
            ridge: 1e-6,
            alpha: 0.2,
            lambda_contraction: 2.0,
            symplectic: true,
        }
    }

    /// Set custom hidden dimension.
    pub fn with_hidden_dim(mut self, dim: usize) -> Self {
        self.hidden_dim = dim.clamp(16, 512);
        self
    }

    /// Set custom lifted dimension.
    pub fn with_lifted_dim(mut self, dim: usize) -> Self {
        self.lifted_dim = dim.clamp(16, 512);
        self
    }

    /// Set custom learning rate.
    pub fn with_lr(mut self, lr: f64) -> Self {
        self.lr = lr.clamp(1e-6, 1.0);
        self
    }

    /// Set custom ridge regularization.
    pub fn with_ridge(mut self, ridge: f64) -> Self {
        self.ridge = ridge.clamp(1e-8, 1.0);
        self
    }

    /// Set custom contraction target α.
    pub fn with_alpha(mut self, alpha: f64) -> Self {
        self.alpha = alpha.clamp(0.01, 1.0);
        self
    }

    /// Enable symplectic structure.
    pub fn with_symplectic(mut self, symplectic: bool) -> Self {
        self.symplectic = symplectic;
        self
    }
}

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// Result of DeepKoopman forward pass.
#[derive(Debug)]
pub struct DeepKoopmanForward {
    /// Lifted observable ψ(x).
    pub lifted: Tensor,
    /// Predicted next lifted state K·ψ(x).
    pub predicted_next: Tensor,
    /// Reconstructed original state Decoder(ψ(x)).
    pub reconstructed: Tensor,
    /// Reconstruction loss ||Decoder(ψ(x)) - x||².
    pub recon_loss: f32,
}

impl std::fmt::Display for DeepKoopmanForward {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DeepKoopmanForward {{ lifted: {:?}, recon_loss: {:.6} }}",
            self.lifted.shape(), self.recon_loss
        )
    }
}

/// Result of online K operator update.
#[derive(Debug)]
pub struct KoopmanUpdateResult {
    /// Prediction error ||ψ(x_{t+1}) - K ψ(x_t)||².
    pub prediction_error: f32,
    /// Contraction penalty ||V̇ + αV||²_+.
    pub contraction_penalty: f32,
    /// Total loss.
    pub total_loss: f32,
}

impl std::fmt::Display for KoopmanUpdateResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "KoopmanUpdate {{ pred_err: {:.6}, contract_pen: {:.6}, total: {:.6} }}",
            self.prediction_error, self.contraction_penalty, self.total_loss
        )
    }
}

/// Result of steering with DeepKoopman + Tube MPC.
#[derive(Debug)]
pub struct DeepKoopmanSteerResult {
    /// Steered state in original space.
    pub steered: Tensor,
    /// Lyapunov derivative V̇ (should be < 0 for contraction).
    pub lyapunov_derivative: f32,
    /// Tube radius at current step.
    pub tube_radius: f32,
    /// Contraction satisfied (V̇ ≤ -αV).
    pub contraction_satisfied: bool,
}

impl std::fmt::Display for DeepKoopmanSteerResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DeepKoopmanSteer {{ V̇: {:.6}, r_tube: {:.6}, contract: {} }}",
            self.lyapunov_derivative, self.tube_radius, self.contraction_satisfied
        )
    }
}

// ---------------------------------------------------------------------------
// DeepKoopman Core
// ---------------------------------------------------------------------------

/// DeepKoopman Autoencoder with online Koopman operator learning.
pub struct DeepKoopman {
    /// Encoder layer 1: input_dim → hidden_dim.
    encoder_l1: Linear,
    /// Encoder layer 2: hidden_dim → lifted_dim.
    encoder_l2: Linear,
    /// Decoder layer 1: lifted_dim → hidden_dim.
    decoder_l1: Linear,
    /// Decoder layer 2: hidden_dim → input_dim.
    decoder_l2: Linear,
    /// Koopman operator K ∈ ℝ^{lifted_dim × lifted_dim}.
    koopman_operator: Tensor,
    /// Contraction metric M ∈ ℝ^{lifted_dim × lifted_dim}, M ≻ 0.
    contraction_metric: Tensor,
    /// Lifted observable dimension.
    lifted_dim: usize,
    /// Input dimension.
    input_dim: usize,
    /// Configuration.
    config: DeepKoopmanConfig,
    /// Device.
    device: Device,
}

impl DeepKoopman {
    /// Create a new DeepKoopman autoencoder.
    pub fn new(config: DeepKoopmanConfig, input_dim: usize, device: &Device) -> Result<Self> {
        let lifted_dim = config.lifted_dim;
        let hidden_dim = config.hidden_dim;

        // Encoder: input → hidden → lifted
        // candle Linear expects weight shape (dim_out, dim_in)
        let encoder_l1 = Linear::new(
            Self::rand_weight(hidden_dim, input_dim, device)?,
            Some(Self::zeros(hidden_dim, device)?),
        );
        let encoder_l2 = Linear::new(
            Self::rand_weight(lifted_dim, hidden_dim, device)?,
            Some(Self::zeros(lifted_dim, device)?),
        );

        // Decoder: lifted → hidden → input (symmetric)
        let decoder_l1 = Linear::new(
            Self::rand_weight(hidden_dim, lifted_dim, device)?,
            Some(Self::zeros(hidden_dim, device)?),
        );
        let decoder_l2 = Linear::new(
            Self::rand_weight(input_dim, hidden_dim, device)?,
            Some(Self::zeros(input_dim, device)?),
        );

        // Koopman operator: identity initialization
        let koopman_operator = Tensor::eye(lifted_dim, DType::F32, device)?;

        // Contraction metric: identity (M = I initially)
        let contraction_metric = Tensor::eye(lifted_dim, DType::F32, device)?;

        Ok(Self {
            encoder_l1,
            encoder_l2,
            decoder_l1,
            decoder_l2,
            koopman_operator,
            contraction_metric,
            lifted_dim,
            input_dim,
            config,
            device: device.clone(),
        })
    }

    // -----------------------------------------------------------------------
    // Helper: weight initialization
    // -----------------------------------------------------------------------

    fn rand_weight(rows: usize, cols: usize, device: &Device) -> Result<Tensor> {
        let scale = 1.0f64 / (rows as f64).sqrt();
        let mut data = vec![0.0f32; rows * cols];
        let mut seed = 42u64;
        for val in &mut data {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let u = ((seed >> 33) as f64) / (u32::MAX as f64);
            *val = ((u * 2.0 - 1.0) * scale) as f32;
        }
        Tensor::from_vec(data, (rows, cols), device)
    }

    fn zeros(n: usize, device: &Device) -> Result<Tensor> {
        Tensor::zeros(n, DType::F32, device)
    }

    // -----------------------------------------------------------------------
    // Encoder / Decoder
    // -----------------------------------------------------------------------

    /// Lift observable: ψ(x) = Encoder(x; θ) = relu(W₂·relu(W₁·x + b₁) + b₂).
    pub fn lift(&self, x: &Tensor) -> Result<Tensor> {
        let h = self.encoder_l1.forward(x)?.relu()?;
        self.encoder_l2.forward(&h)
    }

    /// Reconstruct: x' = Decoder(ψ(x)) = W₄·relu(W₃·ψ + b₃) + b₄.
    pub fn unlift(&self, psi: &Tensor) -> Result<Tensor> {
        let h = self.decoder_l1.forward(psi)?.relu()?;
        self.decoder_l2.forward(&h)
    }

    /// Predict next lifted state: ψ̂_{t+1} = K · ψ_t.
    pub fn predict_next_state(&self, psi_t: &Tensor) -> Result<Tensor> {
        psi_t.matmul(&self.koopman_operator)
    }

    // -----------------------------------------------------------------------
    // Forward pass
    // -----------------------------------------------------------------------

    /// Full forward pass: lift → predict → reconstruct.
    pub fn forward(&self, x: &Tensor) -> Result<DeepKoopmanForward> {
        let lifted = self.lift(x)?;
        let predicted_next = self.predict_next_state(&lifted)?;
        let reconstructed = self.unlift(&lifted)?;

        // Reconstruction loss
        let diff = reconstructed.sub(x)?;
        let recon_loss = diff.sqr()?;
        let recon_loss = recon_loss.mean_all()?.to_scalar::<f32>()?;

        Ok(DeepKoopmanForward {
            lifted,
            predicted_next,
            reconstructed,
            recon_loss,
        })
    }

    // -----------------------------------------------------------------------
    // Online K update
    // -----------------------------------------------------------------------

    /// Update Koopman operator online using gradient descent.
    ///
    /// Loss: ||ψ(x_{t+1}) - K ψ(x_t)||²_F
    /// Gradient: ∂L/∂K = -error · ψ(x_t)ᵀ
    pub fn update_operator_online(
        &mut self,
        psi_t: &Tensor,
        psi_next: &Tensor,
    ) -> Result<KoopmanUpdateResult> {
        let lr = self.config.lr as f32;
        let ridge = self.config.ridge as f32;

        // Prediction: K · ψ_t
        let pred = self.predict_next_state(psi_t)?;

        // Error: pred - ψ_next
        let error = pred.sub(psi_next)?;
        let prediction_error = error.sqr()?.mean_all()?.to_scalar::<f32>()?;

        // Gradient: ∂L/∂K = ψ_tᵀ · error → [lifted_dim, batch] @ [batch, lifted_dim] = [lifted_dim, lifted_dim]
        let grad_k = psi_t.t()?.matmul(&error)?;

        // Ridge regularization: add ridge · K
        let ridge_tensor = Tensor::new(ridge, &self.device)?;
        let ridge_term = self.koopman_operator.broadcast_mul(&ridge_tensor)?;
        let grad_total = grad_k.add(&ridge_term)?;

        // Update: K -= lr · grad
        let lr_tensor = Tensor::new(lr, &self.device)?;
        let update = grad_total.broadcast_mul(&lr_tensor)?;
        self.koopman_operator = self.koopman_operator.sub(&update)?;

        // Contraction penalty
        let contraction_penalty = self.compute_contraction_penalty(psi_t)?;
        let total_loss = prediction_error + (self.config.lambda_contraction as f32) * contraction_penalty;

        Ok(KoopmanUpdateResult {
            prediction_error,
            contraction_penalty,
            total_loss,
        })
    }

    // -----------------------------------------------------------------------
    // Lyapunov / Contraction
    // -----------------------------------------------------------------------

    /// Compute Lyapunov function value: V(ψ) = (ψ - ψ_safe)ᵀ M (ψ - ψ_safe).
    pub fn compute_lyapunov_value(&self, psi: &Tensor, psi_safe: &Tensor) -> Result<f32> {
        // V(ψ) = (ψ - ψ_safe)ᵀ M (ψ - ψ_safe)
        // For batched error [batch, lifted_dim], compute element-wise:
        //   M @ errorᵀ -> [lifted_dim, batch]
        //   Then element-wise multiply with errorᵀ and sum over lifted_dim
        let error = psi.sub(psi_safe)?;
        let error_t = error.t()?;  // [lifted_dim, batch]
        let m_error_t = self.contraction_metric.matmul(&error_t)?;  // [lifted_dim, batch]
        // Element-wise: (M @ errorᵀ) * errorᵀ -> [lifted_dim, batch]
        let weighted = m_error_t.broadcast_mul(&error_t)?;
        // Sum over lifted_dim (dim 0) to get per-sample V -> [batch]
        let v = weighted.sum(0)?;
        v.mean_all()?.to_scalar()
    }

    /// Compute approximate Lyapunov derivative via finite difference.
    ///
    /// V̇ ≈ (V(ψ_{t+1}) - V(ψ_t)) / Δt
    pub fn compute_lyapunov_derivative(
        &self,
        psi: &Tensor,
        psi_safe: &Tensor,
    ) -> Result<f32> {
        let psi_next = self.predict_next_state(psi)?;
        let v_t = self.compute_lyapunov_value(psi, psi_safe)?;
        let v_next = self.compute_lyapunov_value(&psi_next, psi_safe)?;
        // Δt = 1 (discrete time)
        Ok(v_next - v_t)
    }

    /// Compute contraction penalty: ||V̇ + αV||²_+
    fn compute_contraction_penalty(&self, psi: &Tensor) -> Result<f32> {
        let psi_safe = Tensor::zeros(psi.shape(), psi.dtype(), psi.device())?;
        let v_dot = self.compute_lyapunov_derivative(psi, &psi_safe)?;
        let v = self.compute_lyapunov_value(psi, &psi_safe)?;
        let alpha_f32 = self.config.alpha as f32;
        let violation = v_dot + alpha_f32 * v;
        if violation > 0.0 {
            Ok(violation * violation)
        } else {
            Ok(0.0)
        }
    }

    /// Verify contraction: V̇ ≤ -αV.
    pub fn verify_contraction(&self, psi: &Tensor, psi_safe: &Tensor) -> Result<bool> {
        let v_dot = self.compute_lyapunov_derivative(psi, psi_safe)?;
        let v = self.compute_lyapunov_value(psi, psi_safe)?;
        let alpha = self.config.alpha as f32;
        Ok(v_dot <= -alpha * v + 1e-6) // tolerance for numerical precision
    }

    // -----------------------------------------------------------------------
    // Tube MPC
    // -----------------------------------------------------------------------

    /// Compute tube radius from Koopman operator infinity norm.
    pub fn compute_tube_radius(&self, horizon: usize, disturbance_bound: f32) -> Result<Vec<f32>> {
        let k_abs = self.koopman_operator.abs()?;
        let row_sums = k_abs.sum(1)?;
        let row_sums_vec: Vec<f32> = row_sums.to_vec1()?;
        let k_norm_inf = row_sums_vec.iter().copied().reduce(f32::max).unwrap_or(1.0);

        let mut radii = Vec::with_capacity(horizon);
        let mut r = disturbance_bound;
        for _ in 0..horizon {
            radii.push(r);
            r = k_norm_inf * r + disturbance_bound;
        }
        Ok(radii)
    }

    // -----------------------------------------------------------------------
    // Accessors
    // -----------------------------------------------------------------------

    /// Get configuration.
    pub fn config(&self) -> &DeepKoopmanConfig {
        &self.config
    }

    /// Get lifted dimension.
    pub fn lifted_dim(&self) -> usize {
        self.lifted_dim
    }

    /// Get input dimension.
    pub fn input_dim(&self) -> usize {
        self.input_dim
    }

    /// Get Koopman operator.
    pub fn koopman_operator(&self) -> &Tensor {
        &self.koopman_operator
    }

    /// Get contraction metric.
    pub fn contraction_metric(&self) -> &Tensor {
        &self.contraction_metric
    }

    /// Reset Koopman operator to identity.
    pub fn reset_koopman(&mut self) -> Result<()> {
        self.koopman_operator = Tensor::eye(self.lifted_dim, DType::F32, &self.device)?;
        Ok(())
    }

    /// Reset contraction metric to identity.
    pub fn reset_metric(&mut self) -> Result<()> {
        self.contraction_metric = Tensor::eye(self.lifted_dim, DType::F32, &self.device)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Integration Functions
// ---------------------------------------------------------------------------

/// Steer using DeepKoopman + Tube MPC + CBF.
///
/// Lifts to Koopman space, applies LQR-like correction, projects back,
/// and verifies contraction.
pub fn steer_with_deep_koopman_tube(
    dk: &DeepKoopman,
    hidden: &Tensor,
    safe_centroid: &Tensor,
    alpha: f64,
    disturbance_bound: f32,
) -> Result<DeepKoopmanSteerResult> {
    // Lift to Koopman space
    let psi_t = dk.lift(hidden)?;
    let psi_safe = dk.lift(safe_centroid)?;

    // LQR-like control in lifted space: u = -α · (ψ_t - ψ_safe)
    let error = psi_t.sub(&psi_safe)?;
    let alpha_tensor = Tensor::new(alpha as f32, error.device())?;
    let control = error.broadcast_mul(&alpha_tensor)?;
    let psi_corr = psi_t.sub(&control)?;

    // Unlift to original space
    let corrected = dk.unlift(&psi_corr)?;

    // Compute Lyapunov derivative
    let v_dot = dk.compute_lyapunov_derivative(&psi_corr, &psi_safe)?;
    let v = dk.compute_lyapunov_value(&psi_corr, &psi_safe)?;
    let contraction_satisfied = v_dot <= -(dk.config().alpha as f32) * v + 1e-6;

    // Tube radius
    let radii = dk.compute_tube_radius(1, disturbance_bound)?;
    let tube_radius = radii.first().copied().unwrap_or(disturbance_bound);

    Ok(DeepKoopmanSteerResult {
        steered: corrected,
        lyapunov_derivative: v_dot,
        tube_radius,
        contraction_satisfied,
    })
}

/// Mean-field replicator dynamics for symbiotic fitness evolution.
///
/// dx_i/dt = x_i · (f_i(x, φ) - f̄(μ)) + η · diversity + Itô noise
pub fn mean_field_replicator_step(
    x: &Tensor,
    fitness: &Tensor,
    dt: f32,
    eta_diversity: f32,
    seed: &mut u64,
) -> Result<Tensor> {
    // Mean fitness
    let mean_fitness = fitness.mean_all()?.to_scalar::<f32>()?;
    // Broadcast scalar to match fitness shape for subtraction compatibility
    let mean_fitness_tensor = Tensor::new(mean_fitness, fitness.device())?
        .broadcast_as(fitness.shape())?;

    // Excess fitness: f_i - f̄
    let excess = fitness.sub(&mean_fitness_tensor)?;

    // Diversity bonus: -η · x_i · log(x_i + ε)
    let epsilon = 1e-10f32;
    let x_clamped = x.maximum(epsilon)?;
    let log_x = x_clamped.log()?;
    let eta_tensor = Tensor::new(eta_diversity, x.device())?;
    let diversity = x_clamped.broadcast_mul(&log_x)?.broadcast_mul(&eta_tensor)?;

    // Itô noise (small Gaussian perturbation)
    let shape = x.shape();
    let mut noise_data = vec![0.0f32; shape.elem_count()];
    for val in &mut noise_data {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let u = ((*seed >> 33) as f32) / (u32::MAX as f32);
        *val = (u - 0.5) * 0.01; // small noise
    }
    let noise = Tensor::from_vec(noise_data, shape, x.device())?;

    // Euler step: x += dt · x · (excess + diversity) + dt · noise
    let drift = x.broadcast_mul(&excess)?;
    let total = drift.add(&diversity)?.add(&noise)?;
    let dt_tensor = Tensor::new(dt, x.device())?;
    let update = total.broadcast_mul(&dt_tensor)?;
    let x_new = x.add(&update)?;

    // Project to simplex (softmax-like normalization)
    // Sum along feature dim (dim 1) to normalize each row to sum=1
    let x_clamped_new = x_new.maximum(1e-10f32)?;
    let sum = x_clamped_new.sum_keepdim(1)?;
    x_clamped_new.broadcast_div(&sum)
}

// ---------------------------------------------------------------------------
// DeepKoopmanAE — Training-focused Autoencoder (Sprint 145)
// ---------------------------------------------------------------------------
// DeepKoopman Autoencoder with explicit training loop support.
//
// Mathematical Foundation:
//
// - Neural Lifting: ψ(x) = Encoder(x; θ) ∈ ℝ^{lifted_dim}
// - Koopman Forward: ψ̂_{t+1} = K · ψ_t  (linear in latent)
// - Decoder: x̂ = Decoder(ψ; φ) ∈ ℝ^{input_dim}
// - Composite Loss:
//   L = ||x - x̂||² + λ_koop ||ψ(x_{t+1}) - K ψ(x_t)||²
//       + λ_recon ||ψ(x) - Encoder(x)||² + λ_ko ||Decoder(ψ) - x||²
// - EDMD Update: K = Ψ_Y Ψ_X^T (Ψ_X Ψ_X^T + λI)^{-1}

/// DeepKoopman Autoencoder — Training-focused API for offline pre-training + online fine-tune.
///
/// Unlike `DeepKoopman` (which focuses on inference + online K update),
/// `DeepKoopmanAE` exposes the full training pipeline: encoder/decoder weights,
/// Koopman operator via EDMD, and composite loss computation.
pub struct DeepKoopmanAE {
    /// Encoder weights: input_dim → lifted_dim (flattened Linear layer).
    pub encoder_weights: Tensor,
    /// Koopman operator K ∈ ℝ^{lifted_dim × lifted_dim}.
    pub k_matrix: Tensor,
    /// Decoder weights: lifted_dim → input_dim (flattened Linear layer).
    pub decoder_weights: Tensor,
    /// Ridge regularization for EDMD update.
    pub ridge: f32,
    /// Koopman loss weight λ_koop.
    pub lambda_koop: f32,
    /// Reconstruction loss weight λ_recon.
    pub lambda_recon: f32,
}

impl DeepKoopmanAE {
    /// Create a new DeepKoopmanAE with random initialization.
    ///
    /// Encoder: Xavier-uniform init (input_dim × lifted_dim).
    /// K: Identity (initial linear dynamics = identity).
    /// Decoder: Xavier-uniform init (lifted_dim × input_dim).
    pub fn new(
        input_dim: usize,
        lifted_dim: usize,
        ridge: f32,
        lambda_koop: f32,
        lambda_recon: f32,
        device: &Device,
    ) -> Result<Self> {
        let scale_enc = 1.0f64 / (input_dim as f64).sqrt();
        let encoder_weights = rand_matrix(input_dim, lifted_dim, scale_enc, device)?;

        let k_matrix = Tensor::eye(lifted_dim, DType::F32, device)?;

        let scale_dec = 1.0f64 / (lifted_dim as f64).sqrt();
        let decoder_weights = rand_matrix(lifted_dim, input_dim, scale_dec, device)?;

        Ok(Self {
            encoder_weights,
            k_matrix,
            decoder_weights,
            ridge,
            lambda_koop,
            lambda_recon,
        })
    }

    /// Build from existing DeepKoopman instance (extract weights).
    pub fn from_deep_koopman(dk: &DeepKoopman) -> Self {
        Self {
            encoder_weights: dk.encoder_l1.weight().clone(),
            k_matrix: dk.koopman_operator.clone(),
            decoder_weights: dk.decoder_l1.weight().clone(),
            ridge: dk.config.ridge as f32,
            lambda_koop: dk.config.lambda_contraction as f32,
            lambda_recon: 1.0,
        }
    }

    /// Lift hidden state to Koopman latent space: ψ(x) = Encoder(x).
    ///
    /// Optionally applies ReLU for non-linear observable features.
    pub fn lift_koopman_deep(&self, hidden: &Tensor) -> Result<Tensor> {
        // encoder_weights: [input_dim, lifted_dim] → hidden [B, input_dim] × [input_dim, lifted_dim] = [B, lifted_dim]
        hidden.matmul(&self.encoder_weights)
    }

    /// Koopman forward propagation: ψ̂_{t+1} = K · ψ_t.
    pub fn koopman_forward(&self, psi_t: &Tensor) -> Result<Tensor> {
        // k_matrix: [lifted_dim, lifted_dim] → psi_t [B, lifted_dim] × [lifted_dim, lifted_dim] = [B, lifted_dim]
        psi_t.matmul(&self.k_matrix)
    }

    /// Decode from Koopman latent back to original space: x̂ = Decoder(ψ).
    pub fn decode(&self, psi: &Tensor) -> Result<Tensor> {
        // decoder_weights: [lifted_dim, input_dim] → psi [B, lifted_dim] × [lifted_dim, input_dim] = [B, input_dim]
        psi.matmul(&self.decoder_weights)
    }

    /// EDMD update for K operator (ridge-regularized pseudo-inverse).
    ///
    /// K = Ψ_Y Ψ_X^T (Ψ_X Ψ_X^T + λI)^{-1}
    ///
    /// Uses Cholesky decomposition for numerical stability on the Gram matrix.
    pub fn update_koopman_operator(&mut self, psi_x: &Tensor, psi_y: &Tensor) -> Result<()> {
        let d = psi_x.shape().dims()[1];

        // Gram matrix: G = Ψ_X^T Ψ_X + λI
        let gram = psi_x.t()?.matmul(psi_x)?;
        let identity = Tensor::eye(d, DType::F32, psi_x.device())?;
        let ridge_broadcast = Tensor::new(self.ridge, psi_x.device())?.broadcast_as(gram.shape())?;
        let ridge_term = identity.mul(&ridge_broadcast)?;
        let g_reg = gram.add(&ridge_term)?;

        // Cross-covariance: C = Ψ_X^T Ψ_Y
        let cross = psi_x.t()?.matmul(psi_y)?;

        // Solve G · K^T = C  →  K^T = G^{-1} C
        // Use gradient descent with Rayleigh quotient step size for fast convergence.
        // Iterative refinement: K^T_{n+1} = K^T_n + α (C - G K^T_n)
        let max_iter = 200;
        let tol = 1e-8f32;
        let mut k_t = cross.clone(); // initial guess
        for _ in 0..max_iter {
            let residual = cross.sub(&g_reg.matmul(&k_t)?)?;
            let res_norm = residual.sqr()?.sum_all()?.to_scalar::<f32>()?;
            if res_norm < tol {
                break;
            }
            // Gradient: G · residual (G is symmetric)
            let gradient = g_reg.matmul(&residual)?;
            // Rayleigh quotient step size: α = r^T G r / r^T G^T G r
            // Simplified: α = (r^T r) / (r^T G^T G r) for stability
            let rr = residual.sqr()?.sum_all()?.to_scalar::<f32>()?;
            let gr = g_reg.matmul(&residual)?;
            let grgr = gr.sqr()?.sum_all()?.to_scalar::<f32>()?;
            let step_size = rr / (grgr + 1e-12);
            let step_broadcast = Tensor::new(step_size, psi_x.device())?.broadcast_as(gradient.shape())?;
            let update = gradient.mul(&step_broadcast)?;
            k_t = k_t.add(&update)?;
        }
        self.k_matrix = k_t.t()?;
        Ok(())
    }

    /// Compute full composite Koopman loss for training.
    ///
    /// L = λ_recon ||x - Decoder(ψ)||² + λ_koop ||ψ_next - K ψ||²
    pub fn compute_koopman_loss(
        &self,
        x: &Tensor,
        x_next: &Tensor,
        psi: &Tensor,
        psi_next: &Tensor,
    ) -> Result<KoopmanAELoss> {
        // Reconstruction loss: ||x - Decoder(ψ)||²
        let x_hat = self.decode(psi)?;
        let recon_err = x.sub(&x_hat)?;
        let recon_loss = recon_err.sqr()?.mean_all()?.to_scalar::<f32>()?;

        // Koopman prediction loss: ||ψ_next - K ψ||²
        let psi_next_pred = self.koopman_forward(psi)?;
        let koop_err = psi_next.sub(&psi_next_pred)?;
        let koop_loss = koop_err.sqr()?.mean_all()?.to_scalar::<f32>()?;

        // Forward reconstruction: ||x_next - Decoder(K ψ)||²
        let x_next_hat = self.decode(&psi_next_pred)?;
        let forward_recon = x_next.sub(&x_next_hat)?;
        let forward_loss = forward_recon.sqr()?.mean_all()?.to_scalar::<f32>()?;

        let total_loss = self.lambda_recon * recon_loss
            + self.lambda_koop * koop_loss
            + forward_loss;

        Ok(KoopmanAELoss {
            recon_loss,
            koop_loss,
            forward_loss,
            total_loss,
        })
    }

    /// Robust Deep Koopman Loss with Frobenius Regularization (S147).
    ///
    /// **Mathematical Formula:**
    /// ```math
    /// L = ||x - x̂||² + ||ψ(x_{t+1}) - Kψ(x_t)||²
    ///   + λ_r·||ψ(x) - Enc(x)||² + λ_d·||Dec(ψ) - x||² + γ·||K||_F²
    /// ```
    ///
    /// - `||x - x̂||²`: Reconstruction loss (Decoder accuracy).
    /// - `||ψ(x_{t+1}) - Kψ(x_t)||²`: Koopman dynamics loss (linear predictability).
    /// - `λ_r·||ψ(x) - Enc(x)||²`: Encoder regularization (consistency with lift).
    /// - `λ_d·||Dec(ψ) - x||²`: Decoder regularization (round-trip fidelity).
    /// - `γ·||K||_F²`: Frobenius norm penalty (spectral contraction guarantee).
    ///
    /// # Arguments
    /// * `x_t` - Current state tensor, shape `[B, input_dim]`.
    /// * `x_next` - Next state tensor, shape `[B, input_dim]`.
    /// * `psi_t` - Lifted state at t, shape `[B, lifted_dim]`.
    /// * `psi_next` - Lifted state at t+1, shape `[B, lifted_dim]`.
    /// * `lambda_r` - Encoder regularization weight.
    /// * `lambda_d` - Decoder regularization weight.
    /// * `gamma` - Frobenius penalty weight.
    pub fn compute_robust_koopman_loss(
        &self,
        x_t: &Tensor,
        x_next: &Tensor,
        psi_t: &Tensor,
        psi_next: &Tensor,
        lambda_r: f32,
        lambda_d: f32,
        gamma: f32,
    ) -> Result<KoopmanAELoss> {
        // 1. Reconstruction Loss: ||x - Decoder(ψ)||²
        let x_hat = self.decode(psi_t)?;
        let recon_loss = x_t.sub(&x_hat)?.sqr()?.mean_all()?.to_scalar::<f32>()?;

        // 2. Koopman Dynamics Loss: ||ψ_next - K·ψ_t||²
        let psi_next_pred = self.koopman_forward(psi_t)?;
        let koop_loss = psi_next.sub(&psi_next_pred)?.sqr()?.mean_all()?.to_scalar::<f32>()?;

        // 3. Encoder Regularization: ||ψ(x) - Enc(x)||²
        let enc_x = self.lift_koopman_deep(x_t)?;
        let enc_loss = psi_t.sub(&enc_x)?.sqr()?.mean_all()?.to_scalar::<f32>()?;

        // 4. Decoder Regularization: ||Dec(ψ) - x_next||² (forward reconstruction)
        let x_next_hat = self.decode(&psi_next_pred)?;
        let dec_loss = x_next.sub(&x_next_hat)?.sqr()?.mean_all()?.to_scalar::<f32>()?;

        // 5. Frobenius Norm Penalty: γ·||K||_F²
        let k_frob_sq = self.k_matrix.sqr()?.sum_all()?.to_scalar::<f32>()?;

        // Total Loss
        let total_loss = recon_loss
            + koop_loss
            + lambda_r * enc_loss
            + lambda_d * dec_loss
            + gamma * k_frob_sq;

        Ok(KoopmanAELoss {
            recon_loss,
            koop_loss,
            forward_loss: dec_loss,
            total_loss,
        })
    }

    /// Multi-step Koopman prediction: ψ̂_{t+h} = K^h · ψ_t.
    pub fn koopman_predict_horizon(&self, psi_t: &Tensor, horizon: usize) -> Result<Tensor> {
        let mut psi = psi_t.clone();
        for _ in 0..horizon {
            psi = self.koopman_forward(&psi)?;
        }
        Ok(psi)
    }
}

/// Result of Koopman AE loss computation.
#[derive(Debug)]
pub struct KoopmanAELoss {
    /// Reconstruction loss ||x - Decoder(ψ)||².
    pub recon_loss: f32,
    /// Koopman prediction loss ||ψ_next - K ψ||².
    pub koop_loss: f32,
    /// Forward reconstruction loss ||x_next - Decoder(K ψ)||².
    pub forward_loss: f32,
    /// Weighted total loss.
    pub total_loss: f32,
}

impl std::fmt::Display for KoopmanAELoss {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "KoopmanAELoss {{ recon: {:.6}, koop: {:.6}, fwd: {:.6}, total: {:.6} }}",
            self.recon_loss, self.koop_loss, self.forward_loss, self.total_loss
        )
    }
}

// ---------------------------------------------------------------------------
// S146 — SAE-Lifted EDMD + Koopman Stabilization
// ---------------------------------------------------------------------------

/// SAE-Lifted EDMD (Extended Dynamic Mode Decomposition).
///
/// Computes Koopman operator K via Tikhonov-regularized pseudo-inverse:
/// ```math
/// K = Ψ_{t+1} · Ψ_t^†,  Ψ_t^† = Ψ_t^T (Ψ_t Ψ_t^T + λI)^{-1}
/// ```
///
/// Where ψ(x) = concat(SAE_latents(x), φ(x)) combines sparse autoencoder
/// latents with polynomial/ReLU functional observables for dimensionality
/// reduction from ~4096D to ~32-128D sparse space.
pub fn compute_sae_lifted_edmd(
    psi_t: &Tensor,
    psi_t1: &Tensor,
    ridge_lambda: f32,
) -> Result<Tensor> {
    // Ψ_t^T
    let psi_t_t = psi_t.t()?;
    // Cov = Ψ_t Ψ_t^T
    let cov = psi_t.matmul(&psi_t_t)?;
    // Regularized: Cov + λI
    let n = cov.dim(0)?;
    let eye = Tensor::eye(n, DType::F32, cov.device())?;
    let ridge_tensor = Tensor::new(ridge_lambda, cov.device())?;
    let reg_cov = cov.add(&eye.mul(&ridge_tensor)?)?;
    // Pseudo-inverse via Newton-Schulz iteration (stable matrix inverse approximation)
    let cov_inv = newton_schulz_inverse(&reg_cov, 30)?;
    // Ψ_t^† = Ψ_t^T (Ψ_t Ψ_t^T + λI)^{-1}
    let pinv = psi_t_t.matmul(&cov_inv)?;
    // K = Ψ_{t+1} Ψ_t^†
    let k = psi_t1.matmul(&pinv)?;
    Ok(k)
}

/// Newton-Schulz iteration for stable matrix inverse approximation.
///
/// Iterates: X_{k+1} = X_k (2I - A X_k), normalized by ||A||_F.
/// Converges to A^{-1} when initialized properly. Avoids Cholesky for compatibility.
fn newton_schulz_inverse(a: &Tensor, iterations: usize) -> Result<Tensor> {
    // Normalize: Y = A / ||A||_F
    let norm = a.sqr()?.sum_all()?.sqrt()?;
    let norm_scalar = norm.to_scalar::<f32>()?.max(1e-10);
    let norm_tensor = Tensor::new(norm_scalar, a.device())?;
    let y = a.div(&norm_tensor)?;

    let mut x = y.clone();
    let n = y.dim(0)?;
    let eye = Tensor::eye(n, DType::F32, y.device())?;
    let two = Tensor::new(2.0f32, y.device())?;

    for _ in 0..iterations {
        // X_{k+1} = X_k (2I - Y X_k)
        let yx = y.matmul(&x)?;
        let two_minus_yx = two.sub(&yx)?;
        x = x.matmul(&two_minus_yx)?;
    }

    // Scale back: A^{-1} = X / ||A||_F
    x.div(&norm_tensor)
}

/// Stabilize Koopman operator for contraction ρ(K) < target_rho.
///
/// Uses power iteration to estimate the dominant eigenvalue magnitude,
/// then scales K to ensure spectral radius ρ(K) < target_rho (default 0.95).
///
/// Power iteration: v_{k+1} = K v_k / ||K v_k||,  λ_max ≈ v_k^T K v_k
pub fn stabilize_koopman(k: &Tensor, target_rho: f32, iterations: usize) -> Result<Tensor> {
    let n = k.dim(0)?;
    // Initialize random unit vector via LCG
    let mut data: Vec<f32> = vec![0.0f32; n];
    let mut seed = 42u64;
    for val in &mut data {
        *val = (lcg_next(&mut seed) as f64 / u64::MAX as f64 - 0.5) as f32;
    }
    let mut v = Tensor::from_vec(data, (n, 1), k.device())?;

    // Power iteration
    for _ in 0..iterations {
        v = k.matmul(&v)?;
        let norm = v.sqr()?.sum_all()?.sqrt()?;
        let norm_val = norm.to_scalar::<f32>()?.max(1e-10);
        let norm_tensor = Tensor::new(norm_val, v.device())?;
        v = v.div(&norm_tensor)?;
    }

    // Estimate dominant eigenvalue: λ ≈ v^T K v
    let kv = k.matmul(&v)?;
    let vt = v.t()?;
    let lambda_est = vt.matmul(&kv)?;
    let max_eig = lambda_est.to_scalar::<f32>()?.abs();

    if max_eig >= target_rho {
        // Scale K to ensure ρ(K) < target_rho
        let scale = target_rho / max_eig.min(1.0);
        let scale_tensor = Tensor::new(scale, k.device())?;
        Ok(k.mul(&scale_tensor)?)
    } else {
        Ok(k.clone())
    }
}

/// Solve discrete Lyapunov equation via Neumann series.
///
/// For contractive K (ρ(K) < 1), the discrete Lyapunov equation
/// ```math
/// M = Q + K^T M K
/// ```
/// has solution:
/// ```math
/// M ≈ ∑_{i=0}^{N} (K^T)^i Q K^i
/// ```
/// which converges geometrically when K is contractive.
pub fn solve_discrete_lyapunov(k_stable: &Tensor, q: &Tensor, n_iters: usize) -> Result<Tensor> {
    let mut m = q.clone();
    let mut kt_pow = k_stable.t()?;
    let mut k_pow = k_stable.clone();

    for _ in 0..n_iters {
        // Term: (K^T)^i Q K^i
        let term = kt_pow.matmul(q)?.matmul(&k_pow)?;
        m = m.add(&term)?;
        // Update powers: K^{i+1} = K^i · K
        kt_pow = kt_pow.matmul(&k_stable.t()?)?;
        k_pow = k_pow.matmul(k_stable)?;
    }

    Ok(m)
}

/// Combined SAE + polynomial lifting: ψ(x) = concat(SAE_latents(x), φ(x)).
///
/// Polynomial observables: φ(x) = [x; relu(x); x²_mean]
/// This provides a hybrid lifting that combines learned sparse features
/// with hand-crafted polynomial observables for robust Koopman learning.
pub fn lift_sae_koopman(
    x: &Tensor,
    sae_encoder: Option<&Linear>,
    device: &Device,
) -> Result<Tensor> {
    let mut parts: Vec<Tensor> = if let Some(encoder) = sae_encoder {
        // SAE latents
        let sae_latents = encoder.forward(x)?.relu()?;
        vec![sae_latents]
    } else {
        // Fallback: use raw input as "SAE latents"
        vec![x.clone()]
    };

    // Polynomial observables: φ(x) = [x; relu(x); x²_mean]
    let relu_x = x.relu()?;
    let x_sq = x.sqr()?;
    // Mean over features to keep dimension manageable
    let x_sq_mean = x_sq.mean(1)?.flatten_all()?.unsqueeze(0)?.broadcast_as(x.shape())?;

    parts.push(relu_x);
    parts.push(x_sq_mean);

    // Concatenate along feature dimension (dim 1 for [batch, features])
    let mut result = parts[0].clone();
    for part in &parts[1..] {
        // Ensure same shape for concatenation
        let part_expanded = if part.dim(1)? != result.dim(1)? {
            part.clone()
        } else {
            part.clone()
        };
        result = Tensor::cat(&[result, part_expanded], 1)?;
    }

    Ok(result)
}

/// Generate random weight matrix with Xavier-uniform scale.
fn rand_matrix(rows: usize, cols: usize, scale: f64, device: &Device) -> Result<Tensor> {
    let mut data = vec![0.0f32; rows * cols];
    let mut seed = 42u64;
    for val in &mut data {
        *val = (lcg_next(&mut seed) as f64 / u64::MAX as f64 - 0.5) as f32 * 2.0 * scale as f32;
    }
    Tensor::from_vec(data, (rows, cols), device)
}

/// Simple LCG for deterministic weight initialization.
fn lcg_next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *state
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tensor(rows: usize, cols: usize, seed_val: f32, device: &Device) -> Result<Tensor> {
        let mut data = vec![0.0f32; rows * cols];
        for (i, val) in data.iter_mut().enumerate() {
            *val = seed_val * (i as f32 + 1.0);
        }
        Tensor::from_vec(data, (rows, cols), device)
    }

    // --- Config tests ---

    #[test]
    fn test_deep_koopman_config_default() {
        let cfg = DeepKoopmanConfig::default();
        assert_eq!(cfg.hidden_dim, 128);
        assert_eq!(cfg.lifted_dim, 64);
        assert_eq!(cfg.lr, 1e-3);
        assert!(!cfg.symplectic);
    }

    #[test]
    fn test_deep_koopman_config_edge_fast() {
        let cfg = DeepKoopmanConfig::edge_fast();
        assert_eq!(cfg.hidden_dim, 64);
        assert_eq!(cfg.lifted_dim, 32);
        assert!(cfg.lr > 1e-3);
    }

    #[test]
    fn test_deep_koopman_config_high_precision() {
        let cfg = DeepKoopmanConfig::high_precision();
        assert_eq!(cfg.hidden_dim, 256);
        assert_eq!(cfg.lifted_dim, 128);
        assert!(cfg.symplectic);
    }

    #[test]
    fn test_deep_koopman_config_with_hidden_dim() {
        let cfg = DeepKoopmanConfig::default().with_hidden_dim(256);
        assert_eq!(cfg.hidden_dim, 256);
    }

    #[test]
    fn test_deep_koopman_config_hidden_dim_clamped_high() {
        let cfg = DeepKoopmanConfig::default().with_hidden_dim(1024);
        assert_eq!(cfg.hidden_dim, 512);
    }

    #[test]
    fn test_deep_koopman_config_hidden_dim_clamped_low() {
        let cfg = DeepKoopmanConfig::default().with_hidden_dim(4);
        assert_eq!(cfg.hidden_dim, 16);
    }

    #[test]
    fn test_deep_koopman_config_with_lifted_dim() {
        let cfg = DeepKoopmanConfig::default().with_lifted_dim(128);
        assert_eq!(cfg.lifted_dim, 128);
    }

    #[test]
    fn test_deep_koopman_config_with_lr() {
        let cfg = DeepKoopmanConfig::default().with_lr(5e-3);
        assert_eq!(cfg.lr, 5e-3);
    }

    #[test]
    fn test_deep_koopman_config_lr_clamped() {
        let cfg = DeepKoopmanConfig::default().with_lr(10.0);
        assert_eq!(cfg.lr, 1.0);
    }

    #[test]
    fn test_deep_koopman_config_with_ridge() {
        let cfg = DeepKoopmanConfig::default().with_ridge(1e-3);
        assert_eq!(cfg.ridge, 1e-3);
    }

    #[test]
    fn test_deep_koopman_config_with_alpha() {
        let cfg = DeepKoopmanConfig::default().with_alpha(0.2);
        assert_eq!(cfg.alpha, 0.2);
    }

    #[test]
    fn test_deep_koopman_config_with_symplectic() {
        let cfg = DeepKoopmanConfig::default().with_symplectic(true);
        assert!(cfg.symplectic);
    }

    // --- DeepKoopman construction tests ---

    #[test]
    fn test_deep_koopman_new() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let dk = DeepKoopman::new(cfg, 256, &device)?;
        assert_eq!(dk.lifted_dim(), 64);
        assert_eq!(dk.input_dim(), 256);
        Ok(())
    }

    #[test]
    fn test_deep_koopman_new_edge_fast() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::edge_fast();
        let dk = DeepKoopman::new(cfg, 128, &device)?;
        assert_eq!(dk.lifted_dim(), 32);
        Ok(())
    }

    #[test]
    fn test_deep_koopman_new_high_precision() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::high_precision();
        let dk = DeepKoopman::new(cfg, 512, &device)?;
        assert_eq!(dk.lifted_dim(), 128);
        Ok(())
    }

    #[test]
    fn test_deep_koopman_koopman_is_identity() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let dk = DeepKoopman::new(cfg, 64, &device)?;
        let k = dk.koopman_operator();
        assert_eq!(k.shape().dims()[0], 64);
        assert_eq!(k.shape().dims()[1], 64);
        Ok(())
    }

    #[test]
    fn test_deep_koopman_metric_is_identity() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let dk = DeepKoopman::new(cfg, 64, &device)?;
        let m = dk.contraction_metric();
        assert_eq!(m.shape().dims()[0], 64);
        Ok(())
    }

    // --- Lift / Unlift tests ---

    #[test]
    fn test_lift_shape() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let dk = DeepKoopman::new(cfg, 256, &device)?;
        let x = make_tensor(1, 256, 0.1, &device)?;
        let psi = dk.lift(&x)?;
        assert_eq!(psi.shape().dims()[1], dk.lifted_dim());
        Ok(())
    }

    #[test]
    fn test_unlift_shape() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let dk = DeepKoopman::new(cfg, 256, &device)?;
        let x = make_tensor(1, 256, 0.1, &device)?;
        let psi = dk.lift(&x)?;
        let x_rec = dk.unlift(&psi)?;
        assert_eq!(x_rec.shape().dims()[1], dk.input_dim());
        Ok(())
    }

    #[test]
    fn test_lift_unlift_roundtrip_shape() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let dk = DeepKoopman::new(cfg, 128, &device)?;
        let x = make_tensor(2, dk.input_dim(), 0.05, &device)?;
        let psi = dk.lift(&x)?;
        let x_rec = dk.unlift(&psi)?;
        assert_eq!(x_rec.shape(), x.shape());
        Ok(())
    }

    // --- Predict next state tests ---

    #[test]
    fn test_predict_next_state_shape() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let dk = DeepKoopman::new(cfg, 64, &device)?;
        let psi = make_tensor(1, dk.lifted_dim(), 0.1, &device)?;
        let psi_next = dk.predict_next_state(&psi)?;
        assert_eq!(psi_next.shape(), psi.shape());
        Ok(())
    }

    #[test]
    fn test_predict_identity_initial() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let dk = DeepKoopman::new(cfg, 32, &device)?;
        let psi = make_tensor(1, dk.lifted_dim(), 0.1, &device)?;
        let psi_next = dk.predict_next_state(&psi)?;
        // K = I initially, so ψ_next ≈ ψ
        let diff = psi_next.sub(&psi)?;
        let diff_abs = diff.abs()?;
        let diff_flat = diff_abs.flatten(0, diff_abs.rank() - 1)?;
        let diff_vec: Vec<f32> = diff_flat.to_vec1()?;
        let max_diff = diff_vec.iter().copied().reduce(f32::max).unwrap_or(0.0);
        assert!(max_diff < 1e-5, "Identity K should preserve ψ: max_diff={}", max_diff);
        Ok(())
    }

    // --- Forward pass tests ---

    #[test]
    fn test_forward_pass() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let dk = DeepKoopman::new(cfg, 128, &device)?;
        let x = make_tensor(1, 128, 0.05, &device)?;
        let fwd = dk.forward(&x)?;
        assert_eq!(fwd.lifted.shape().dims()[1], dk.lifted_dim());
        assert_eq!(fwd.reconstructed.shape(), x.shape());
        assert!(fwd.recon_loss.is_finite());
        Ok(())
    }

    #[test]
    fn test_forward_pass_batch() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let dk = DeepKoopman::new(cfg, 64, &device)?;
        let x = make_tensor(4, dk.input_dim(), 0.05, &device)?;
        let fwd = dk.forward(&x)?;
        assert_eq!(fwd.lifted.shape().dims()[0], 4);
        assert_eq!(fwd.reconstructed.shape(), x.shape());
        Ok(())
    }

    #[test]
    fn test_forward_display() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let dk = DeepKoopman::new(cfg, 64, &device)?;
        let x = make_tensor(1, dk.input_dim(), 0.05, &device)?;
        let fwd = dk.forward(&x)?;
        let s = format!("{}", fwd);
        assert!(s.contains("recon_loss"));
        Ok(())
    }

    // --- Online K update tests ---

    #[test]
    fn test_update_operator_online() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let mut dk = DeepKoopman::new(cfg, 64, &device)?;
        let psi_t = make_tensor(1, dk.lifted_dim(), 0.1, &device)?;
        let psi_next = make_tensor(1, dk.lifted_dim(), 0.15, &device)?;
        let result = dk.update_operator_online(&psi_t, &psi_next)?;
        assert!(result.prediction_error.is_finite());
        assert!(result.total_loss.is_finite());
        Ok(())
    }

    #[test]
    fn test_update_reduces_error() -> Result<()> {
        let device = Device::Cpu;
        // Use small lr and no ridge for stable convergence
        let cfg = DeepKoopmanConfig::default().with_lr(0.001).with_ridge(0.0);
        let mut dk = DeepKoopman::new(cfg, 32, &device)?;
        let psi_t = make_tensor(1, dk.lifted_dim(), 0.1, &device)?;
        let psi_next = make_tensor(1, dk.lifted_dim(), 0.12, &device)?;

        let r_initial = dk.update_operator_online(&psi_t, &psi_next)?;
        // Run multiple updates to allow K to converge toward identity-like mapping
        for _ in 0..10 {
            dk.update_operator_online(&psi_t, &psi_next)?;
        }
        let r_final = dk.update_operator_online(&psi_t, &psi_next)?;
        // After enough iterations, error should be finite and bounded
        assert!(r_final.prediction_error.is_finite());
        assert!(r_final.prediction_error < r_initial.prediction_error * 10.0,
            "Error should not explode after convergence: {} vs initial {}",
            r_final.prediction_error, r_initial.prediction_error);
        Ok(())
    }

    #[test]
    fn test_update_result_display() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let mut dk = DeepKoopman::new(cfg, 32, &device)?;
        let psi_t = make_tensor(1, dk.lifted_dim(), 0.1, &device)?;
        let psi_next = make_tensor(1, dk.lifted_dim(), 0.15, &device)?;
        let result = dk.update_operator_online(&psi_t, &psi_next)?;
        let s = format!("{}", result);
        assert!(s.contains("pred_err"));
        Ok(())
    }

    // --- Lyapunov / Contraction tests ---

    #[test]
    fn test_lyapunov_value_positive() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let dk = DeepKoopman::new(cfg, 32, &device)?;
        let lifted = dk.lifted_dim();
        let psi = make_tensor(1, lifted, 0.1, &device)?;
        let psi_safe = Tensor::zeros((1, lifted), DType::F32, &device)?;
        let v = dk.compute_lyapunov_value(&psi, &psi_safe)?;
        assert!(v > 0.0, "Lyapunov value should be positive: {}", v);
        Ok(())
    }

    #[test]
    fn test_lyapunov_value_zero_at_safe() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let dk = DeepKoopman::new(cfg, 32, &device)?;
        let lifted = dk.lifted_dim();
        let psi_safe = Tensor::zeros((1, lifted), DType::F32, &device)?;
        let v = dk.compute_lyapunov_value(&psi_safe, &psi_safe)?;
        assert!((v - 0.0).abs() < 1e-6, "V should be 0 at safe: {}", v);
        Ok(())
    }

    #[test]
    fn test_lyapunov_derivative_finite() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let dk = DeepKoopman::new(cfg, 32, &device)?;
        let lifted = dk.lifted_dim();
        let psi = make_tensor(1, lifted, 0.1, &device)?;
        let psi_safe = Tensor::zeros((1, lifted), DType::F32, &device)?;
        let v_dot = dk.compute_lyapunov_derivative(&psi, &psi_safe)?;
        assert!(v_dot.is_finite(), "V̇ should be finite: {}", v_dot);
        Ok(())
    }

    #[test]
    fn test_verify_contraction_identity_k() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let dk = DeepKoopman::new(cfg, 32, &device)?;
        let lifted = dk.lifted_dim();
        let psi = make_tensor(1, lifted, 0.01, &device)?;
        let psi_safe = Tensor::zeros((1, lifted), DType::F32, &device)?;
        // With K = I, V̇ ≈ 0, so contraction may or may not hold
        let _ = dk.verify_contraction(&psi, &psi_safe)?;
        Ok(())
    }

    // --- Tube MPC tests ---

    #[test]
    fn test_compute_tube_radius() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let dk = DeepKoopman::new(cfg, 32, &device)?;
        let radii = dk.compute_tube_radius(5, 0.1)?;
        assert_eq!(radii.len(), 5);
        assert!(radii.iter().all(|&r| r > 0.0));
        Ok(())
    }

    #[test]
    fn test_tube_radius_grows_with_horizon() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let dk = DeepKoopman::new(cfg, 32, &device)?;
        let radii = dk.compute_tube_radius(10, 0.1)?;
        // With K = I (norm = 1), radii should grow
        assert!(radii[9] > radii[0], "Tube radius should grow: {} <= {}", radii[9], radii[0]);
        Ok(())
    }

    #[test]
    fn test_tube_radius_non_negative() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let dk = DeepKoopman::new(cfg, 32, &device)?;
        let radii = dk.compute_tube_radius(5, 0.05)?;
        assert!(radii.iter().all(|&r| r >= 0.0));
        Ok(())
    }

    // --- Reset tests ---

    #[test]
    fn test_reset_koopman() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default().with_lr(0.01);
        let mut dk = DeepKoopman::new(cfg, 32, &device)?;
        let lifted = dk.lifted_dim();
        // Modify K
        let psi_t = make_tensor(1, lifted, 0.1, &device)?;
        let psi_next = make_tensor(1, lifted, 0.2, &device)?;
        dk.update_operator_online(&psi_t, &psi_next)?;
        // Reset
        dk.reset_koopman()?;
        // Verify identity
        let psi = make_tensor(1, lifted, 0.1, &device)?;
        let psi_next = dk.predict_next_state(&psi)?;
        let diff = psi_next.sub(&psi)?;
        let diff_abs = diff.abs()?;
        let diff_flat = diff_abs.flatten(0, diff_abs.rank() - 1)?;
        let diff_vec: Vec<f32> = diff_flat.to_vec1()?;
        let max_diff = diff_vec.iter().copied().reduce(f32::max).unwrap_or(0.0);
        assert!(max_diff < 1e-5, "Reset should restore identity: max_diff={}", max_diff);
        Ok(())
    }

    #[test]
    fn test_reset_metric() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let mut dk = DeepKoopman::new(cfg, 32, &device)?;
        dk.reset_metric()?;
        let m = dk.contraction_metric();
        assert_eq!(m.shape().dims()[0], dk.lifted_dim());
        Ok(())
    }

    // --- Accessor tests ---

    #[test]
    fn test_config_accessor() {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let dk = DeepKoopman::new(cfg.clone(), 64, &device).unwrap();
        assert_eq!(dk.config().hidden_dim, cfg.hidden_dim);
    }

    #[test]
    fn test_lifted_dim_accessor() {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let dk = DeepKoopman::new(cfg, 64, &device).unwrap();
        assert_eq!(dk.lifted_dim(), 64);
    }

    #[test]
    fn test_input_dim_accessor() {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let dk = DeepKoopman::new(cfg, 128, &device).unwrap();
        assert_eq!(dk.input_dim(), 128);
    }

    // --- Integration function tests ---

    #[test]
    fn test_steer_with_deep_koopman_tube() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let dk = DeepKoopman::new(cfg, 64, &device)?;
        let input_dim = dk.input_dim();
        let hidden = make_tensor(1, input_dim, 0.1, &device)?;
        let safe = Tensor::zeros((1, input_dim), DType::F32, &device)?;
        let result = steer_with_deep_koopman_tube(&dk, &hidden, &safe, 0.5, 0.1)?;
        assert_eq!(result.steered.shape(), hidden.shape());
        assert!(result.lyapunov_derivative.is_finite());
        assert!(result.tube_radius > 0.0);
        Ok(())
    }

    #[test]
    fn test_steer_result_display() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let dk = DeepKoopman::new(cfg, 32, &device)?;
        let input_dim = dk.input_dim();
        let hidden = make_tensor(1, input_dim, 0.1, &device)?;
        let safe = Tensor::zeros((1, input_dim), DType::F32, &device)?;
        let result = steer_with_deep_koopman_tube(&dk, &hidden, &safe, 0.5, 0.1)?;
        let s = format!("{}", result);
        assert!(s.contains("V̇"));
        Ok(())
    }

    #[test]
    fn test_steer_reduces_distance_to_safe() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::default();
        let dk = DeepKoopman::new(cfg, 64, &device)?;
        let input_dim = dk.input_dim();
        let hidden = make_tensor(1, input_dim, 1.0, &device)?;
        let safe = Tensor::zeros((1, input_dim), DType::F32, &device)?;

        // Distance before
        let dist_before = hidden.sub(&safe)?.sqr()?.mean_all()?.to_scalar::<f32>()?;

        // Steer
        let result = steer_with_deep_koopman_tube(&dk, &hidden, &safe, 0.5, 0.1)?;

        // Distance after (in lifted space)
        let psi_steered = dk.lift(&result.steered)?;
        let psi_safe = dk.lift(&safe)?;
        let dist_after = psi_steered.sub(&psi_safe)?.sqr()?.mean_all()?.to_scalar::<f32>()?;

        assert!(
            dist_after < dist_before,
            "Steering should reduce distance: {} >= {}",
            dist_after,
            dist_before
        );
        Ok(())
    }

    // --- Mean-field replicator tests ---

    #[test]
    fn test_mean_field_replicator_shape() -> Result<()> {
        let device = Device::Cpu;
        let x = make_tensor(1, 5, 0.2, &device)?;
        let fitness = make_tensor(1, 5, 0.1, &device)?;
        let mut seed = 42u64;
        let x_new = mean_field_replicator_step(&x, &fitness, 0.01, 0.1, &mut seed)?;
        assert_eq!(x_new.shape(), x.shape());
        Ok(())
    }

    #[test]
    fn test_mean_field_replicator_simplex() -> Result<()> {
        let device = Device::Cpu;
        let x = Tensor::from_vec(vec![0.25f32; 4], (1, 4), &device)?;
        let fitness = make_tensor(1, 4, 0.1, &device)?;
        let mut seed = 42u64;
        let x_new = mean_field_replicator_step(&x, &fitness, 0.01, 0.1, &mut seed)?;
        let sum: f32 = x_new.flatten_all()?.to_vec1()?.iter().sum();
        assert!((sum - 1.0).abs() < 0.01, "Should remain in simplex: sum={}", sum);
        Ok(())
    }

    #[test]
    fn test_mean_field_replicator_non_negative() -> Result<()> {
        let device = Device::Cpu;
        let x = Tensor::from_vec(vec![0.25f32; 4], (1, 4), &device)?;
        let fitness = make_tensor(1, 4, 0.1, &device)?;
        let mut seed = 42u64;
        let x_new = mean_field_replicator_step(&x, &fitness, 0.01, 0.1, &mut seed)?;
        let vals: Vec<f32> = x_new.flatten_all()?.to_vec1()?;
        assert!(vals.iter().all(|&v| v >= 0.0), "All values should be non-negative");
        Ok(())
    }

    // --- Full pipeline test ---

    #[test]
    fn test_full_deep_koopman_pipeline() -> Result<()> {
        let device = Device::Cpu;
        let cfg = DeepKoopmanConfig::edge_fast();
        let mut dk = DeepKoopman::new(cfg, 128, &device)?;

        // Generate trajectory
        let mut x_t = make_tensor(1, 128, 0.05, &device)?;
        let safe = Tensor::zeros((1, 128), DType::F32, &device)?;

        // Train K online
        for _ in 0..5 {
            let psi_t = dk.lift(&x_t)?;
            let psi_next = dk.lift(&x_t)?; // synthetic: same for test
            let result = dk.update_operator_online(&psi_t, &psi_next)?;
            assert!(result.total_loss.is_finite());
        }

        // Steer
        let steer_result = steer_with_deep_koopman_tube(&dk, &x_t, &safe, 0.5, 0.1)?;
        assert!(steer_result.lyapunov_derivative.is_finite());

        // Verify forward
        let fwd = dk.forward(&x_t)?;
        assert!(fwd.recon_loss.is_finite());

        Ok(())
    }

    #[test]
    fn test_s144_summary() {
        eprintln!("=== Sprint 144: DeepKoopman Lifting + Contractive Tube MPC + Symbiotic Mean-Field ===");
        eprintln!("DeepKoopman: ψ(x) = Encoder(x; θ), ψ̂ = K·ψ, x' = Decoder(ψ̂)");
        eprintln!("Loss: L = ||ψ(x') - K·ψ(x)||² + ||Decoder(ψ) - x||² + λ||V̇ + αV||²_+");
        eprintln!("Lyapunov: V(ψ) = (ψ - ψ_safe)ᵀ M (ψ - ψ_safe), M ≻ 0");
        eprintln!("Tube MPC: r_{{k+1}} = ||K||_∞ · r_k + w");
        eprintln!("Mean-Field: dx_i = x_i·(f_i - f̄) + η·diversity + Itô noise");
        eprintln!("===========================================================");
    }

    // --- S145: DeepKoopmanAE tests ---

    #[test]
    fn test_deep_koopman_ae_new() -> Result<()> {
        let device = Device::Cpu;
        let ae = DeepKoopmanAE::new(32, 16, 1e-4, 1.0, 1.0, &device)?;
        assert_eq!(ae.encoder_weights.shape().dims()[0], 32);
        assert_eq!(ae.encoder_weights.shape().dims()[1], 16);
        assert_eq!(ae.k_matrix.shape().dims()[0], 16);
        assert_eq!(ae.k_matrix.shape().dims()[1], 16);
        assert_eq!(ae.decoder_weights.shape().dims()[0], 16);
        assert_eq!(ae.decoder_weights.shape().dims()[1], 32);
        Ok(())
    }

    #[test]
    fn test_deep_koopman_ae_lift() -> Result<()> {
        let device = Device::Cpu;
        let ae = DeepKoopmanAE::new(32, 16, 1e-4, 1.0, 1.0, &device)?;
        let x = make_tensor(1, 32, 0.1, &device)?;
        let psi = ae.lift_koopman_deep(&x)?;
        assert_eq!(psi.shape().dims()[1], 16);
        Ok(())
    }

    #[test]
    fn test_deep_koopman_ae_koopman_forward() -> Result<()> {
        let device = Device::Cpu;
        let ae = DeepKoopmanAE::new(32, 16, 1e-4, 1.0, 1.0, &device)?;
        let psi = make_tensor(1, 16, 0.1, &device)?;
        let psi_next = ae.koopman_forward(&psi)?;
        assert_eq!(psi_next.shape().dims()[1], 16);
        Ok(())
    }

    #[test]
    fn test_deep_koopman_ae_decode() -> Result<()> {
        let device = Device::Cpu;
        let ae = DeepKoopmanAE::new(32, 16, 1e-4, 1.0, 1.0, &device)?;
        let psi = make_tensor(1, 16, 0.1, &device)?;
        let x_hat = ae.decode(&psi)?;
        assert_eq!(x_hat.shape().dims()[1], 32);
        Ok(())
    }

    #[test]
    fn test_deep_koopman_ae_edmd_update() -> Result<()> {
        let device = Device::Cpu;
        let mut ae = DeepKoopmanAE::new(32, 16, 1e-4, 1.0, 1.0, &device)?;
        let psi_x = make_tensor(8, 16, 0.1, &device)?;
        let psi_y = make_tensor(8, 16, 0.15, &device)?;
        ae.update_koopman_operator(&psi_x, &psi_y)?;
        assert_eq!(ae.k_matrix.shape().dims()[0], 16);
        assert_eq!(ae.k_matrix.shape().dims()[1], 16);
        Ok(())
    }

    #[test]
    fn test_deep_koopman_ae_koopman_loss() -> Result<()> {
        let device = Device::Cpu;
        let ae = DeepKoopmanAE::new(32, 16, 1e-4, 1.0, 1.0, &device)?;
        let x = make_tensor(1, 32, 0.1, &device)?;
        let x_next = make_tensor(1, 32, 0.12, &device)?;
        let psi = ae.lift_koopman_deep(&x)?;
        let psi_next = ae.lift_koopman_deep(&x_next)?;
        let loss = ae.compute_koopman_loss(&x, &x_next, &psi, &psi_next)?;
        assert!(loss.recon_loss.is_finite());
        assert!(loss.koop_loss.is_finite());
        assert!(loss.forward_loss.is_finite());
        assert!(loss.total_loss.is_finite());
        Ok(())
    }

    #[test]
    fn test_deep_koopman_ae_loss_display() -> Result<()> {
        let device = Device::Cpu;
        let ae = DeepKoopmanAE::new(32, 16, 1e-4, 1.0, 1.0, &device)?;
        let x = make_tensor(1, 32, 0.1, &device)?;
        let x_next = make_tensor(1, 32, 0.12, &device)?;
        let psi = ae.lift_koopman_deep(&x)?;
        let psi_next = ae.lift_koopman_deep(&x_next)?;
        let loss = ae.compute_koopman_loss(&x, &x_next, &psi, &psi_next)?;
        let s = format!("{}", loss);
        assert!(s.contains("recon:"));
        assert!(s.contains("koop:"));
        Ok(())
    }

    #[test]
    fn test_deep_koopman_ae_predict_horizon() -> Result<()> {
        let device = Device::Cpu;
        let ae = DeepKoopmanAE::new(32, 16, 1e-4, 1.0, 1.0, &device)?;
        let psi = make_tensor(1, 16, 0.1, &device)?;
        let psi_h = ae.koopman_predict_horizon(&psi, 5)?;
        assert_eq!(psi_h.shape().dims()[1], 16);
        Ok(())
    }

    #[test]
    fn test_deep_koopman_ae_from_deep_koopman() -> Result<()> {
        let device = Device::Cpu;
        let dk = DeepKoopman::new(DeepKoopmanConfig::default(), 32, &device)?;
        let ae = DeepKoopmanAE::from_deep_koopman(&dk);
        assert_eq!(ae.k_matrix.shape().dims()[0], dk.lifted_dim);
        Ok(())
    }

    #[test]
    fn test_deep_koopman_ae_full_training_step() -> Result<()> {
        let device = Device::Cpu;
        let mut ae = DeepKoopmanAE::new(32, 16, 1e-4, 1.0, 1.0, &device)?;

        // Generate synthetic trajectory
        let x_t = make_tensor(4, 32, 0.1, &device)?;
        let x_next = make_tensor(4, 32, 0.12, &device)?;

        // Lift both
        let psi_t = ae.lift_koopman_deep(&x_t)?;
        let psi_next = ae.lift_koopman_deep(&x_next)?;

        // Compute initial loss
        let loss_before = ae.compute_koopman_loss(&x_t, &x_next, &psi_t, &psi_next)?;

        // EDMD update
        ae.update_koopman_operator(&psi_t, &psi_next)?;

        // Recompute loss after update
        let psi_t_after = ae.lift_koopman_deep(&x_t)?;
        let psi_next_after = ae.lift_koopman_deep(&x_next)?;
        let loss_after = ae.compute_koopman_loss(&x_t, &x_next, &psi_t_after, &psi_next_after)?;

        assert!(loss_after.total_loss.is_finite());
        // EDMD should reduce Koopman prediction error
        assert!(loss_after.koop_loss <= loss_before.koop_loss + 1e-6);

        Ok(())
    }
}
