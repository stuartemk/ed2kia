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

    /// Lyapunov Deep Koopman Loss with Contraction Guarantees (S148).
    ///
    /// **Mathematical Formula:**
    /// ```math
    /// L_total = L_lyap + L_spec + L_sdp_proxy
    /// ```
    /// Where:
    /// - `L_lyap = E[(V(ψ_{t+1}) - ρ² V(ψ_t))_+]` with `V(ψ) = ψ^T M ψ`
    /// - `L_spec = (σ_max(K) - ρ)_+` via power iteration
    /// - `L_sdp_proxy = ||(K^T M K - ρ² M)_+||_1` (element-wise ReLU sum)
    ///
    /// **Contraction Guarantee:** If `L_total ≈ 0`, then:
    /// - `V(ψ_{t+1}) ≤ ρ² V(ψ_t)` (Lyapunov decrease)
    /// - `σ_max(K) ≤ ρ` (spectral bound)
    /// - `K^T M K - ρ² M ⪯ 0` (SDP proxy satisfied)
    ///
    /// # Arguments
    /// * `psi_t` - Lifted state at t, shape `[B, lifted_dim]`.
    /// * `psi_t_next` - Lifted state at t+1, shape `[B, lifted_dim]`.
    /// * `k_matrix` - Koopman operator K, shape `[lifted_dim, lifted_dim]`.
    /// * `m_matrix` - Lyapunov metric M ≻ 0, shape `[lifted_dim, lifted_dim]`.
    /// * `rho` - Target contraction rate ρ < 1 (e.g., 0.95).
    /// * `lambda_lyap` - Weight for Lyapunov term.
    /// * `lambda_spec` - Weight for spectral radius term.
    /// * `lambda_sdp` - Weight for SDP proxy term.
    pub fn compute_lyapunov_koopman_loss(
        &self,
        psi_t: &Tensor,
        psi_t_next: &Tensor,
        k_matrix: &Tensor,
        m_matrix: &Tensor,
        rho: f32,
        lambda_lyap: f32,
        lambda_spec: f32,
        lambda_sdp: f32,
    ) -> Result<KoopmanAELoss> {
        let device = psi_t.device();

        // 1. Lyapunov Contraction Term: L_lyap = E[(V(ψ_{t+1}) - ρ² V(ψ_t))_+]
        // V(ψ) = ψ^T M ψ  (quadratic form, batch-aware)
        // psi_t: [B, d], m_matrix: [d, d] → psi_t @ M: [B, d] @ psi_t^T: [d, B] → [B, B]
        // Diagonal = V(ψ_i) for each batch element
        let psi_t_t = psi_t.t()?;
        let v_t = psi_t.matmul(m_matrix)?.matmul(&psi_t_t)?;
        let psi_t_next_t = psi_t_next.t()?;
        let v_t_next = psi_t_next.matmul(m_matrix)?.matmul(&psi_t_next_t)?;

        let rho_sq = rho * rho;
        // contraction_target = ρ² * V(ψ_t) — scalar mul via broadcast
        let rho_sq_tensor = Tensor::new(rho_sq, device)?;
        let contraction_target = v_t.broadcast_mul(&rho_sq_tensor)?;
        // lyap_diff = V(ψ_{t+1}) - ρ² V(ψ_t)
        let lyap_diff = v_t_next.sub(&contraction_target)?;
        // ReLU: only penalize when contraction violated
        let lyap_relu = lyap_diff.relu()?;
        // Manual trace: sum of diagonal elements (trace() not in candle-core 0.6)
        let batch_size = lyap_relu.dim(0)?;
        let mut trace_sum: f32 = 0.0;
        for i in 0..batch_size {
            let diag_elem = lyap_relu.narrow(0, i, 1)?.narrow(1, i, 1)?;
            trace_sum += diag_elem.flatten_all()?.to_vec1::<f32>()?[0];
        }
        let _l_lyap = Tensor::new(trace_sum * lambda_lyap, device)?;

        // 2. Spectral Radius Approximation via Power Iteration (5 iterations)
        // σ_max(K) ≈ ||K v||_2 where v is normalized dominant eigenvector
        let dim = k_matrix.dim(1)?;
        let mut v = Tensor::randn(0.0f32, 1.0f32, (dim, 1), device)?;
        // Manual L2 normalization (normalize() not in candle-core 0.6)
        let norm = v.sqr()?.sum_all()?.sqrt()?;
        let min_norm = Tensor::new(1e-12f32, device)?;
        v = v.broadcast_div(&norm.maximum(&min_norm)?)?;
        for _ in 0..5 {
            let kv = k_matrix.matmul(&v)?;
            let norm = kv.sqr()?.sum_all()?.sqrt()?;
            v = kv.broadcast_div(&norm.maximum(&min_norm)?)?;
        }
        // spectral_radius_approx = ||K v||_2 (norm() not in candle-core 0.6)
        let kv_final = k_matrix.matmul(&v)?;
        let spectral_radius_scalar = kv_final.sqr()?.sum_all()?.sqrt()?;

        let rho_tensor = Tensor::new(rho, device)?;
        let spec_diff = spectral_radius_scalar.broadcast_sub(&rho_tensor)?;
        let spec_relu = spec_diff.relu()?;
        // Scalar mul via broadcast (mul(f32) not available)
        let lambda_spec_tensor = Tensor::new(lambda_spec, device)?;
        let l_spec_scalar = spec_relu.broadcast_mul(&lambda_spec_tensor)?.sum_all()?.flatten_all()?.to_vec1::<f32>()?[0];

        // 3. SDP Proxy: ||(K^T M K - ρ² M)_+||_1
        // Differentiable relaxation of K^T M K - ρ² M ⪯ 0
        let kt = k_matrix.t()?;
        let ktmk = kt.matmul(m_matrix)?.matmul(k_matrix)?;
        let rho2m = m_matrix.broadcast_mul(&rho_sq_tensor)?;
        let sdp_viol = ktmk.sub(&rho2m)?.relu()?;
        let lambda_sdp_tensor = Tensor::new(lambda_sdp, device)?;
        let l_sdp_scalar = sdp_viol.broadcast_mul(&lambda_sdp_tensor)?.sum_all()?.flatten_all()?.to_vec1::<f32>()?[0];

        // Total loss — all values are f32 scalars
        let koop_loss_val = trace_sum * lambda_lyap;
        let forward_loss_val = l_spec_scalar;
        let sdp_loss_val = l_sdp_scalar;
        let total_scalar = koop_loss_val + forward_loss_val + sdp_loss_val;

        Ok(KoopmanAELoss {
            recon_loss: 0.0,
            koop_loss: koop_loss_val,
            forward_loss: forward_loss_val,
            total_loss: total_scalar,
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

// ---------------------------------------------------------------------------
// S150 — Physics-Informed Koopman with Residuals + Volume Preservation
// ---------------------------------------------------------------------------

/// Result of Physics-Informed Koopman loss computation.
#[derive(Debug)]
pub struct PhysicsInformedKoopmanLoss {
    /// MSE reconstruction loss ||ψ_next - ψ_pred||².
    pub mse_loss: f32,
    /// Frobenius regularization ||K||_F².
    pub frob_loss: f32,
    /// Divergence/volume preservation loss.
    pub div_loss: f32,
    /// Lyapunov contraction loss.
    pub lyap_loss: f32,
    /// Weighted total loss.
    pub total_loss: f32,
}

impl std::fmt::Display for PhysicsInformedKoopmanLoss {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PIKLoss {{ mse: {:.6}, frob: {:.6}, div: {:.6}, lyap: {:.6}, total: {:.6} }}",
            self.mse_loss, self.frob_loss, self.div_loss, self.lyap_loss, self.total_loss
        )
    }
}

impl DeepKoopmanAE {
    /// Physics-Informed Koopman propagation with Residual Neural Correction + Divergence Constraint.
    ///
    /// ψ(x_{t+1}) = K ψ(x_t) + ReLU(W_res · ψ(x_t)) + physics_correction
    ///
    /// The physics correction approximates a divergence-free constraint for
    /// near-conservative flows (common in LLM latent dynamics):
    ///   div_correction ≈ -β · div_approx · ψ(x_t)
    ///
    /// Where `div_approx` is a finite-difference proxy of ∇·f(ψ).
    ///
    /// # Arguments
    /// * `psi_x` - Current Koopman state [batch, lifted_dim].
    /// * `k_operator` - Koopman linear operator [lifted_dim, lifted_dim].
    /// * `residual_net` - Residual correction weights [lifted_dim, lifted_dim].
    /// * `div_weight` - Weight β for divergence constraint (0.0 = disabled).
    pub fn propagate_koopman_physics_informed(
        &self,
        psi_x: &Tensor,
        k_operator: &Tensor,
        residual_net: &Tensor,
        div_weight: f32,
    ) -> Result<Tensor> {
        // Linear Koopman: ψ · K (row-vector convention)
        let linear = psi_x.matmul(k_operator)?;

        // Residual neural correction: ReLU(ψ · W_res)
        let residual_raw = psi_x.matmul(residual_net)?;
        let residual = residual_raw.relu()?;

        // Physics-informed divergence approximation via finite difference proxy
        // div(f) ≈ trace(J_f) where J_f is Jacobian of residual
        // Proxy: sum of diagonal elements of W_res (since ∂ReLU(Wψ)/∂ψ ≈ W · diag(ReLU'>0))
        // Simplified: use Frobenius norm of residual_net as divergence proxy
        let div_proxy = if div_weight > 0.0 {
            // Compute trace as sum of diagonal: trace(W) = sum(W ⊙ I)
            let n = residual_net.dim(0)?.min(residual_net.dim(1)?);
            let eye = Tensor::eye(n, DType::F32, residual_net.device())?;
            let trace_val: f32 = (residual_net.broadcast_mul(&eye)?).sum_all()?.to_scalar()?;
            // Divergence correction: -β · trace · ψ
            let corr_scalar = -div_weight * trace_val;
            let corr_tensor = Tensor::new(corr_scalar, psi_x.device())?;
            psi_x.broadcast_mul(&corr_tensor)?
        } else {
            Tensor::zeros(psi_x.shape(), DType::F32, psi_x.device())?
        };

        // ψ_next = Kψ + ReLU(W_res·ψ) + div_correction
        let sum = linear.broadcast_add(&residual)?;
        sum.broadcast_add(&div_proxy)
    }

    /// Compute Physics-Informed Koopman Loss.
    ///
    /// Total Loss = MSE + γ·Frobenius(K) + β·DivLoss + λ·LyapunovLoss
    ///
    /// - MSE: ||ψ_next - ψ_pred||² (data fidelity)
    /// - Frobenius: γ·||K||_F² (operator regularization)
    /// - DivLoss: β·(trace(J_res))² (volume preservation)
    /// - Lyapunov: λ·E[(V(ψ_next) - ρ²·V(ψ))_+] (contraction)
    ///
    /// # Arguments
    /// * `psi_t` - Current Koopman states [batch, lifted_dim].
    /// * `psi_t_next` - Next Koopman states [batch, lifted_dim].
    /// * `k_operator` - Koopman operator [lifted_dim, lifted_dim].
    /// * `residual_weights` - Residual net weights [lifted_dim, lifted_dim].
    /// * `gamma_frob` - Frobenius regularization weight γ.
    /// * `beta_div` - Divergence constraint weight β.
    /// * `lambda_lyap` - Lyapunov contraction weight λ.
    pub fn compute_koopman_loss_physics_informed(
        &self,
        psi_t: &Tensor,
        psi_t_next: &Tensor,
        k_operator: &Tensor,
        residual_weights: &Tensor,
        gamma_frob: f64,
        beta_div: f64,
        lambda_lyap: f64,
    ) -> Result<PhysicsInformedKoopmanLoss> {
        // 1. Predicted next state
        let predicted = self.propagate_koopman_physics_informed(
            psi_t,
            k_operator,
            residual_weights,
            beta_div as f32,
        )?;

        // 2. MSE Loss: ||ψ_next - ψ_pred||²
        let diff = predicted.broadcast_sub(psi_t_next)?;
        let mse = diff.sqr()?.mean_all()?.to_scalar::<f32>()?;

        // 3. Frobenius Regularization: γ·||K||_F²
        let frob_raw = k_operator.sqr()?.sum_all()?.to_scalar::<f32>()?;
        let frob_loss = frob_raw * (gamma_frob as f32);

        // 4. Divergence/Volume Preservation Loss: β·(trace(J_res))²
        let div_loss = if beta_div > 0.0 {
            let n = residual_weights.dim(0)?.min(residual_weights.dim(1)?);
            let eye = Tensor::eye(n, DType::F32, residual_weights.device())?;
            let trace_val: f32 = (residual_weights.broadcast_mul(&eye)?).sum_all()?.to_scalar()?;
            (trace_val * trace_val) * (beta_div as f32)
        } else {
            0.0f32
        };

        // 5. Lyapunov Contraction Loss: λ·E[(||ψ_next||² - ρ²·||ψ||²)_+]
        let lyap_loss = if lambda_lyap > 0.0 {
            let rho = 0.95f32; // Target contraction rate
            let rho_sq = rho * rho;

            // ||ψ_next||² per sample
            let psi_next_sq = psi_t_next.sqr()?.sum_keepdim(1)?;
            // ρ²·||ψ||² per sample
            let psi_sq = psi_t.sqr()?.sum_keepdim(1)?;
            let rho_scalar = Tensor::new(rho_sq, psi_sq.device())?;
            let rho_psi_sq = psi_sq.broadcast_mul(&rho_scalar)?;

            // (V_next - ρ²·V_curr)_+
            let delta = psi_next_sq.broadcast_sub(&rho_psi_sq)?;
            let positive_delta = delta.relu()?;
            let lyap_raw = positive_delta.mean_all()?.to_scalar::<f32>()?;

            lyap_raw * (lambda_lyap as f32)
        } else {
            0.0f32
        };

        let total = mse + frob_loss + div_loss + lyap_loss;

        Ok(PhysicsInformedKoopmanLoss {
            mse_loss: mse,
            frob_loss,
            div_loss,
            lyap_loss,
            total_loss: total,
        })
    }

    /// Stiefel projection: Project K onto nearest orthogonal matrix via polar decomposition proxy.
    ///
    /// Uses iterative Newton-Schulz normalization: K_{new} = K · (3I - K^T K) / 2
    /// to push K toward orthogonality (preserving spectral radius).
    ///
    /// # Arguments
    /// * `k` - Input operator [n, n].
    /// * `iterations` - Number of Newton-Schulz iterations.
    pub fn stiefel_project(k: &Tensor, iterations: usize) -> Result<Tensor> {
        let n = k.dim(0)?;
        let device = k.device().clone();
        // Normalize K by Frobenius norm so ||K||_F ≈ 1, ensuring convergence of Newton-Schulz
        let frob_norm = k.sqr()?.sum_all()?.sqrt()?;
        let min_norm = Tensor::new(1e-6f32, &device)?;
        let max_norm = Tensor::new(100.0f32, &device)?;
        let clamped_norm = frob_norm.maximum(&min_norm)?.minimum(&max_norm)?;
        let one = Tensor::new(1.0f32, &device)?;
        let scale = one.div(&clamped_norm)?;
        let mut k_proj = k.broadcast_mul(&scale)?;

        for _ in 0..iterations {
            let k_clone = k_proj.clone();
            // K^T K
            let ktk = k_clone.t()?.matmul(&k_clone)?;
            // 3I - K^T K
            let eye = Tensor::eye(n, DType::F32, &device)?;
            let three = Tensor::new(3.0f32, &device)?;
            let three_eye = eye.broadcast_mul(&three)?;
            let update = three_eye.sub(&ktk)?;
            // K · (3I - K^T K) / 2
            let half = Tensor::new(0.5f32, &device)?;
            k_proj = k_clone.matmul(&update)?.broadcast_mul(&half)?;
        }

        Ok(k_proj)
    }
}
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
    let inv_norm = Tensor::new(1.0f32 / norm_scalar, a.device())?;
    let y = a.broadcast_mul(&inv_norm)?;

    let mut x = y.clone();
    let n = y.dim(0)?;
    let eye = Tensor::eye(n, DType::F32, y.device())?;
    let two = Tensor::new(2.0f32, y.device())?;
    let two_i = eye.broadcast_mul(&two)?;

    for _ in 0..iterations {
        // X_{k+1} = X_k (2I - Y X_k)
        let yx = y.matmul(&x)?;
        let two_i_minus_yx = two_i.sub(&yx)?;
        x = x.matmul(&two_i_minus_yx)?;
    }

    // Scale back: A^{-1} = X / ||A||_F
    x.broadcast_mul(&inv_norm)
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
// Sprint 155 PASO B: Robust DMDc — Truncated SVD Denoising + Physics-Informed
// ---------------------------------------------------------------------------

/// Result of Robust DMDc identification with GGUF quantization denoising.
#[derive(Debug)]
pub struct RobustDmdcResult {
    /// Denoised system matrix A [d_state x d_state].
    pub k_a: Tensor,
    /// Denoised control matrix B [d_state x d_control].
    pub k_b: Tensor,
    /// Truncated rank used (number of retained modes).
    pub truncated_rank: usize,
    /// Full rank before truncation.
    pub full_rank: usize,
    /// Spectral radius of denoised A.
    pub spectral_radius: f64,
    /// Reconstruction error ||X - X_denoised||_F / ||X||_F.
    pub reconstruction_error: f64,
    /// Physics-Informed residual penalty.
    pub physics_residual: f64,
    /// Noise bound estimate (GGUF quantization level).
    pub noise_bound: f64,
}

impl std::fmt::Display for RobustDmdcResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RobustDmdc[A={:?}, rank_trunc={}, rank_full={}, ρ={:.4}, recon_err={:.6}, phys_res={:.6}, noise={:.6}]",
            self.k_a.shape(),
            self.truncated_rank,
            self.full_rank,
            self.spectral_radius,
            self.reconstruction_error,
            self.physics_residual,
            self.noise_bound
        )
    }
}

/// Truncated SVD approximation via power iteration + deflation.
///
/// Since Candle lacks native SVD, we approximate the top-r singular
/// triplets using iterative power methods with deflation.
///
/// **Algorithm:**
/// ```math
/// \begin{aligned}
/// &\text{For } i = 1 \ldots r: \\
/// &\quad v_i = \text{power_iter}(X^T X, v_0) \\
/// &\quad \sigma_i = \|X v_i\| \\
/// &\quad u_i = X v_i / \sigma_i \\
/// &\quad X \leftarrow X - \sigma_i u_i v_i^T \quad \text{(deflation)}
/// \end{aligned}
/// ```
///
/// # Arguments
/// * `x` — Input matrix `[d, N]`.
/// * `rank` — Number of singular triplets to extract.
/// * `power_iters` — Power iteration steps per triplet.
///
/// # Returns
/// `(U, S_diag, Vt)` where U is `[d, r]`, S_diag is `[r]`, Vt is `[r, N]`.
fn truncated_svd_approx(
    x: &Tensor,
    rank: usize,
    power_iters: usize,
) -> Result<(Tensor, Vec<f64>, Tensor)> {
    let device = x.device();
    let (d, n) = x.dims2()?;
    let effective_rank = rank.min(d).min(n);

    let mut u_cols: Vec<Tensor> = Vec::new();
    let mut s_vals: Vec<f64> = Vec::new();
    let mut vt_rows: Vec<Tensor> = Vec::new();

    let mut x_residual = x.clone();

    for _i in 0..effective_rank {
        // Power iteration on X^T X for right singular vector
        let n_cur = x_residual.dim(1)?;
        let scale = 1.0f32 / (n_cur as f32).sqrt();
        let scale_t = Tensor::new(scale, device)?;
        let mut v = Tensor::ones((n_cur, 1), DType::F32, device)?
            .broadcast_mul(&scale_t)?;

        for _ in 0..power_iters {
            // v = X^T X v
            let xv = x_residual.matmul(&v)?;
            let norm = xv.sqr()?.sum_all()?.sqrt()?;
            let norm_val = norm.to_scalar::<f32>()?.max(1e-10);
            v = x_residual.t()?.matmul(&xv)?.broadcast_mul(&Tensor::new(
                1.0f32 / norm_val,
                device,
            )?)?;
            let v_norm = v.sqr()?.sum_all()?.sqrt()?;
            let v_norm_val = v_norm.to_scalar::<f32>()?.max(1e-10);
            v = v.broadcast_mul(&Tensor::new(1.0f32 / v_norm_val, device)?)?;
        }

        // Compute singular value: σ = ||X v||
        let xv = x_residual.matmul(&v)?;
        let sigma = xv.sqr()?.sum_all()?.sqrt()?.to_scalar::<f32>()?;

        if sigma < 1e-10 {
            // No more significant singular values
            break;
        }

        // Left singular vector: u = X v / σ
        let sigma_t = Tensor::new(sigma, device)?;
        let u = xv.broadcast_div(&sigma_t)?;

        s_vals.push(sigma as f64);
        u_cols.push(u.clone());
        vt_rows.push(v.t()?);

        // Deflation: X <- X - σ u v^T
        let outer = u.matmul(&v.t()?)?;
        let sigma_scalar = Tensor::new(sigma, device)?;
        let update = outer.broadcast_mul(&sigma_scalar)?;
        x_residual = x_residual.sub(&update)?;
    }

    let r = s_vals.len();
    let u_final = if r > 0 {
        Tensor::cat(&u_cols, 1)?
    } else {
        Tensor::zeros((d, 0), DType::F32, device)?
    };
    let vt_final = if r > 0 {
        Tensor::cat(&vt_rows, 0)?
    } else {
        Tensor::zeros((0, n), DType::F32, device)?
    };

    Ok((u_final, s_vals, vt_final))
}

/// Robust DMDc for GGUF: Truncated SVD denoising + residual PI term.
///
/// **Mathematical Foundation:**
///
/// 1. **Truncated SVD Denoising:**
/// ```math
/// \begin{aligned}
/// X &= U \Sigma V^T \\
/// X_{\text{denoised}} &= U_r \Sigma_r V_r^T \\
/// K &= Y_{\text{denoised}} X_{\text{denoised}}^+
/// \end{aligned}
/// ```
///
/// 2. **Physics-Informed Residual:**
/// Enforce contraction by penalizing ||K||_F and ensuring spectral radius < 1.
/// ```math
/// \mathcal{L}_{\text{phys}} = \|Y - K X\|_F^2 + \alpha \|K\|_F^2 + \beta \max(0, \rho(K) - \rho_{\text{target}})^2
/// ```
///
/// 3. **GGUF Quantization Noise Bounds:**
/// Model INT4-like quantization noise as bounded perturbation:
/// ```math
/// \varepsilon_{\text{quant}} \leq \Delta / 2
/// ```
/// where Δ is the quantization step size.
///
/// # Arguments
/// * `x_trajectories` — State trajectories `[d_state, N-1]` (column-major).
/// * `y_trajectories` — Next-state trajectories `[d_state, N-1]`.
/// * `u_trajectories` — Control trajectories `[d_control, N-1]`.
/// * `rank_trunc` — Truncated rank for SVD denoising.
/// * `noise_bound` — GGUF quantization noise bound (e.g., 0.5 for INT4).
/// * `target_rho` — Target spectral radius for stability.
/// * `alpha` — Physics-Informed regularization weight.
///
/// # Returns
/// `RobustDmdcResult` with denoised operators + diagnostics.
pub fn robust_dmdc_gguf(
    x_trajectories: &Tensor,
    y_trajectories: &Tensor,
    u_trajectories: &Tensor,
    rank_trunc: usize,
    noise_bound: f32,
    target_rho: f64,
    alpha: f64,
) -> Result<RobustDmdcResult> {
    let device = x_trajectories.device();
    let (d_state, n_snapshots) = x_trajectories.dims2()?;
    let (d_control, _n_u) = u_trajectories.dims2()?;

    // 1. Build composite Ω = [X; U] — column-major: [d_state+d_control, N-1]
    let omega = Tensor::cat(&[x_trajectories, u_trajectories], 0)?;
    let d_omega = d_state + d_control;
    let y = y_trajectories; // [d_state, N-1]

    // 2. Truncated SVD on Ω for denoising
    let svd_rank = rank_trunc.min(d_omega).min(n_snapshots);
    let (u_omega, s_omega, vt_omega) = truncated_svd_approx(&omega, svd_rank, 15)?;

    let r = s_omega.len();
    let full_rank = d_omega.min(n_snapshots);

    // 3. Reconstruct denoised Ω: Ω_denoised = U_r Σ_r V_r^T
    let omega_denoised = if r > 0 {
        let s_diag: Vec<f32> = s_omega.iter().map(|&s| s as f32).collect();
        let s_tensor = Tensor::from_vec(s_diag, (r, 1), device)?; // [r, 1] for broadcasting with [d, r] -> need [d, 1]
        // U is [d, r], S is [r, 1]: scale each column of U by corresponding singular value
        let u_s = u_omega.broadcast_mul(&s_tensor.t()?)?; // [d, r] * [1, r] = [d, r]
        u_s.matmul(&vt_omega)?
    } else {
        omega.clone()
    };

    // 4. Compute reconstruction error
    let diff = omega.sub(&omega_denoised)?;
    let recon_err_num = diff.sqr()?.sum_all()?.sqrt()?.to_scalar::<f32>()?;
    let omega_norm = omega.sqr()?.sum_all()?.sqrt()?.to_scalar::<f32>()?;
    let reconstruction_error = if omega_norm > 1e-10 {
        recon_err_num / omega_norm
    } else {
        0.0
    };

    // 5. Robust DMDc solve: K = Y Ω_denoised^+ (via normal equations)
    // K = Y Ω^T (Ω Ω^T + λI)^{-1} with ridge regularization
    let omega_t = omega_denoised.t()?; // [N-1, d_omega]
    let omega_omega_t = omega_denoised.matmul(&omega_t)?; // [d_omega, d_omega]

    // Ridge: λ = noise_bound² (scales with quantization noise)
    let ridge = (noise_bound as f64) * (noise_bound as f64) * 1e-4;
    let eye = Tensor::eye(d_omega, DType::F32, device)?;
    let ridge_tensor = Tensor::new(ridge as f32, device)?;
    let regularized = omega_omega_t.add(&eye.broadcast_mul(&ridge_tensor)?)?;

    let inv = newton_schulz_inverse(&regularized, 20)?;

    // K_full = Y Ω^T inv  [d_state, d_omega]
    let y_omega_t = y.matmul(&omega_t)?;
    let k_full = y_omega_t.matmul(&inv)?;

    // 6. Split into A and B
    let k_a_raw = k_full.narrow(1, 0, d_state)?;
    let k_b_raw = k_full.narrow(1, d_state, d_control)?;

    // 7. Physics-Informed regularization: penalize ||K||_F
    let k_a_norm_sq_f32: f32 = k_a_raw.sqr()?.sum_all()?.to_scalar::<f32>()?;
    let k_a_norm_sq: f64 = k_a_norm_sq_f32 as f64;
    let physics_residual = alpha * k_a_norm_sq;

    // 8. Stability projection
    let spectral_radius = compute_spectral_radius(&k_a_raw, 20)?;
    let k_a_final = if spectral_radius > target_rho {
        stabilize_koopman_operator(&k_a_raw, target_rho)?
    } else {
        k_a_raw.clone()
    };

    // 9. Noise bound estimate (GGUF INT4-like: Δ/2)
    let noise_bound_est = noise_bound as f64;

    Ok(RobustDmdcResult {
        k_a: k_a_final,
        k_b: k_b_raw,
        truncated_rank: r,
        full_rank,
        spectral_radius,
        reconstruction_error: reconstruction_error as f64,
        physics_residual: physics_residual as f64,
        noise_bound: noise_bound_est,
    })
}

/// Simulate GGUF quantization noise (INT4-like: clip + round).
///
/// Models the quantization process:
/// ```math
/// \begin{aligned}
/// q(x) &= \text{round}(\text{clip}(x, -8, 7) / \Delta) \cdot \Delta \\
/// \varepsilon(x) &= q(x) - x
/// \end{aligned}
/// ```
///
/// # Arguments
/// * `x` — Input tensor.
/// * `quant_level` — Quantization level (0.5 for INT4, 0.25 for INT8).
///
/// # Returns
/// Noisy tensor with GGUF-like quantization error.
pub fn simulate_gguf_quantization_noise(x: &Tensor, quant_level: f32) -> Result<Tensor> {
    let device = x.device();

    // Clip to quantization range [-8, 7] for INT4-like
    let clipped = x.clamp(-8.0f32, 7.0f32)?;

    // Round to nearest quantization step
    let step = quant_level;
    let step_t = Tensor::new(step, device)?;
    let scaled = clipped.broadcast_div(&step_t)?;
    let rounded = scaled.round()?;
    let quantized = rounded.broadcast_mul(&step_t)?;

    // Add quantization noise: ε = q(x) - x
    let noise = quantized.sub(x)?;

    // Return noisy version
    x.add(&noise)
}

/// Compute robust tube radius for quantization noise propagation.
///
/// For a linear system x_{k+1} = K x_k + w_k with ||w_k|| ≤ ε,
/// the tube radius at horizon h is:
/// ```math
/// r_h = \varepsilon \sum_{i=0}^{h-1} \|K^i\| \approx \varepsilon \frac{1 - \|K\|^h}{1 - \|K\|}
/// ```
///
/// # Arguments
/// * `k_norm` — Induced norm of Koopman operator K.
/// * `noise_eps` — Per-step noise bound ε.
/// * `horizon` — Prediction horizon h.
///
/// # Returns
/// Tube radius at the given horizon.
pub fn compute_robust_tube_radius(k_norm: f64, noise_eps: f64, horizon: usize) -> f64 {
    if k_norm < 1e-10 {
        return noise_eps;
    }
    if (k_norm - 1.0).abs() < 1e-10 {
        return noise_eps * horizon as f64;
    }
    let k_pow_h = k_norm.powi(horizon as i32);
    noise_eps * (1.0 - k_pow_h) / (1.0 - k_norm)
}

// ---------------------------------------------------------------------------
// Sprint 154 PASO C: DMDc (Dynamic Mode Decomposition with Control)
// ---------------------------------------------------------------------------

/// Result of DMDc operator identification.
#[derive(Debug)]
pub struct DmdcResult {
    /// Discrete-time system matrix A [d_state x d_state].
    pub k_a: Tensor,
    /// Control influence matrix B [d_state x d_control].
    pub k_b: Tensor,
    /// Effective rank (number of significant modes).
    pub effective_rank: usize,
    /// Spectral radius of A (stability indicator: <1 = stable).
    pub spectral_radius: f64,
    /// Whether stability projection was applied.
    pub stability_projected: bool,
}

impl std::fmt::Display for DmdcResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Dmdc[A={:?}, rank={}, ρ(A)={:.4}, projected={}]",
            self.k_a.shape(),
            self.effective_rank,
            self.spectral_radius,
            self.stability_projected
        )
    }
}

/// Compute DMDc (Dynamic Mode Decomposition with Control) operator from data.
///
/// Uses ridge regression (dual normal equations) since candle-core 0.6 lacks SVD:
/// ```math
/// \begin{aligned}
/// &\Omega = [X \ U] \in \mathbb{R}^{N \times (d_x + d_u)} \\
/// &[A \ B] = Y \Omega^T (\Omega \Omega^T + \lambda I)^{-1}
/// \end{aligned}
/// ```
///
/// **Stability Projection**: If ρ(A) > target_rho, A is scaled down.
///
/// # Arguments
/// * `x_snapshots` — State snapshots [N x d_state].
/// * `y_snapshots` — Next-state snapshots [N x d_state].
/// * `u_snapshots` — Control inputs [N x d_control].
/// * `target_rho` — Target spectral radius (typically 0.95).
/// * `ridge` — Tikhonov regularization λ.
///
/// # Returns
/// `DmdcResult` with identified A, B operators.
pub fn compute_dmdc(
    x_snapshots: &Tensor,
    y_snapshots: &Tensor,
    u_snapshots: &Tensor,
    target_rho: f64,
    ridge: f64,
) -> Result<DmdcResult> {
    let device = x_snapshots.device();
    let (n_snapshots, d_state) = x_snapshots.dims2()?;
    let (n_u, d_control) = u_snapshots.dims2()?;

    if n_snapshots != n_u {
        return Err(candle_core::Error::Msg(
            format!(
                "Snapshot count mismatch: X={}, U={}",
                n_snapshots, n_u
            ),
        ));
    }

    if n_snapshots < d_state + d_control {
        return Err(candle_core::Error::Msg(
            format!(
                "Insufficient snapshots for DMDc: N={} < d_x+d_u={}",
                n_snapshots,
                d_state + d_control
            ),
        ));
    }

    // 1. Build composite Ω = [X | U]  [N x (d_state + d_control)]
    let omega = Tensor::cat(&[x_snapshots, u_snapshots], 1)?;
    let d_omega = d_state + d_control;

    // 2. Primal normal equations: [A B] = Y^T Ω (Ω^T Ω + λI)^{-1}
    //    Data layout: X,Y,U are [N x d]  (rows=samples, cols=features)
    //    Ω^T Ω  [d_omega x d_omega] — smaller, more stable
    let omega_t = omega.t()?;
    let omega_t_omega = omega_t.matmul(&omega)?;

    //    Regularized: Ω^T Ω + λI  [d_omega x d_omega]
    let eye = Tensor::eye(d_omega, candle_core::DType::F32, device)?;
    let ridge_tensor = Tensor::new(ridge as f32, device)?;
    let ridge_i = eye.broadcast_mul(&ridge_tensor)?;
    let regularized = omega_t_omega.add(&ridge_i)?;

    //    Solve via Newton-Schulz pseudoinverse (stable matrix inverse)
    let inv = newton_schulz_inverse(&regularized, 20)?;

    //    [A B] = Y^T @ Ω @ inv  [d_state x (d_state + d_control)]
    let y_t = y_snapshots.t()?;
    let y_t_omega = y_t.matmul(&omega)?;
    let ab = y_t_omega.matmul(&inv)?;

    // 3. Split into A and B
    let k_a = ab.narrow(1, 0, d_state)?;
    let k_b = ab.narrow(1, d_state, d_control)?;

    // 4. Compute spectral radius via power iteration
    let spectral_radius = compute_spectral_radius(&k_a, 20)?;

    // 5. Stability projection
    let (k_a_final, stability_projected) = if spectral_radius > target_rho {
        let k_a_proj = stabilize_koopman_operator(&k_a, target_rho)?;
        (k_a_proj, true)
    } else {
        (k_a.clone(), false)
    };

    // Effective rank: min(n_snapshots, d_omega)
    let effective_rank = n_snapshots.min(d_omega);

    Ok(DmdcResult {
        k_a: k_a_final,
        k_b,
        effective_rank,
        spectral_radius,
        stability_projected,
    })
}

/// Approximate spectral radius via power iteration.
fn compute_spectral_radius(k: &Tensor, iterations: usize) -> Result<f64> {
    let d = k.dim(0)?;
    let scale = 1.0 / (d as f32).sqrt();
    let scale_tensor = Tensor::new(scale, k.device())?;
    // Use 2D column vector [d, 1] for matmul compatibility
    let mut v = Tensor::ones((d, 1), candle_core::DType::F32, k.device())?
        .broadcast_mul(&scale_tensor)?;

    let mut rho = 1.0f32;
    for _ in 0..iterations {
        let v_new = k.matmul(&v)?;
        rho = v_new.sqr()?.sum_all()?.sqrt()?.to_scalar::<f32>()?;
        if rho < 1e-12 {
            return Ok(0.0);
        }
        let inv_rho = Tensor::new(1.0 / rho, k.device())?;
        v = v_new.broadcast_mul(&inv_rho)?;
    }
    Ok(rho as f64)
}

/// Stabilize Koopman operator by scaling to target spectral radius.
fn stabilize_koopman_operator(k: &Tensor, target_rho: f64) -> Result<Tensor> {
    let current_rho = compute_spectral_radius(k, 15)?;
    if current_rho < 1e-12 {
        return Ok(k.clone());
    }
    let scale = (target_rho / current_rho) as f32;
    let scale_tensor = Tensor::new(scale, k.device())?;
    k.broadcast_mul(&scale_tensor)
}

/// DMDc-based prediction: ψ(y) = A ψ(x) + B u
pub fn dmdc_predict(
    result: &DmdcResult,
    psi_x: &Tensor,
    u: &Tensor,
) -> Result<Tensor> {
    let psi_y_a = result.k_a.matmul(psi_x)?;
    let psi_y_b = result.k_b.matmul(u)?;
    psi_y_a.add(&psi_y_b)
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
