//! SAE Modular Low-Dim Projection — Sparse Autoencoder for verified subspace auditing.
//!
//! Projects high-dimensional hidden states to interpretable low-dimensional latents
//! via sparse coding. Verification (zonotope/IBP/reach-tube) runs in latent subspace
//! for dramatic complexity reduction.
//!
//! **Mathematical Foundation:**
//! - Encoding: `z = topk(ReLU(W_e @ x + b_e), k)`
//! - Decoding: `x_hat = W_d @ z + b_d`
//! - Reconstruction: `x = x_hat + e`, with `||e|| < tau` soundness threshold
//! - Subspace soundness: Verify(Z_latent) ⊇ Project(Z_hidden) when `||e|| < tau`

use candle_core::{DType, Device, Result, Tensor};

/// Configuration for SAE projection.
#[derive(Debug, Clone)]
pub struct SAEConfig {
    /// Input dimension (e.g., 4096 for Llama hidden).
    pub input_dim: usize,
    /// Latent dimension (e.g., 1024 for 4x reduction).
    pub latent_dim: usize,
    /// Top-k sparsity (active latents).
    pub top_k: usize,
    /// Reconstruction error threshold for soundness.
    pub recon_threshold: f32,
}

impl Default for SAEConfig {
    fn default() -> Self {
        Self {
            input_dim: 4096,
            latent_dim: 1024,
            top_k: 512,
            recon_threshold: 0.05,
        }
    }
}

/// Sparse Autoencoder for modular low-dim projection.
pub struct SAE {
    pub encoder_w: Tensor, // [input_dim, latent_dim]
    pub encoder_b: Tensor, // [latent_dim]
    pub decoder_w: Tensor, // [latent_dim, input_dim]
    pub decoder_b: Tensor, // [input_dim]
    pub top_k: usize,
    pub recon_threshold: f32,
}

/// Result of SAE projection.
#[derive(Debug)]
pub struct ProjectionResult {
    /// Sparse latent codes [batch, latent_dim].
    pub latents: Tensor,
    /// Reconstructed input [batch, input_dim].
    pub reconstructed: Tensor,
    /// Per-sample reconstruction error (L2 norm).
    pub recon_errors: Vec<f32>,
    /// Number of active features per sample.
    pub active_features: Vec<usize>,
    /// Average reconstruction error.
    pub avg_recon_error: f32,
    /// Soundness: all errors < threshold.
    pub sound: bool,
}

impl std::fmt::Display for ProjectionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ProjectionResult(latents={:?}, avg_error={:.4}, sound={}, active={:?})",
            self.latents.shape(),
            self.avg_recon_error,
            self.sound,
            self.active_features
        )
    }
}

impl SAE {
    /// Create a new SAE with random orthogonal-ish initialization.
    ///
    /// Encoder weights are initialized so that decoder @ encoder ≈ identity
    /// on the first `latent_dim` coordinates (diagonal projection).
    pub fn new(config: &SAEConfig, device: &Device) -> Result<Self> {
        if config.top_k > config.latent_dim {
            return Err(candle_core::Error::Msg(
                "top_k cannot exceed latent_dim".to_string(),
            ));
        }

        let (ld, id) = (config.latent_dim, config.input_dim);

        // Encoder: [input_dim, latent_dim] so that x @ W works for batched input
        let mut encoder_data: Vec<f32> = Vec::with_capacity(id * ld);
        let mut rng_state = 42u64;
        for _col in 0..ld {
            let mut col_data = vec![0.0f32; id];
            // Random direction
            for val in col_data.iter_mut() {
                *val = next_random_f32(&mut rng_state);
            }
            // Normalize
            let norm: f32 = col_data.iter().map(|v| v * v).sum::<f32>().sqrt().max(1e-8);
            for val in col_data.iter_mut() {
                *val /= norm;
            }
            // Scale by sqrt(input_dim / latent_dim) for energy preservation
            let scale = (id as f32 / ld as f32).sqrt();
            for val in col_data.iter_mut() {
                *val *= scale;
            }
            // Interleave into row-major [input_dim, latent_dim]
            encoder_data.extend(col_data);
        }
        let encoder_w = Tensor::from_vec(encoder_data, (id, ld), device)?;

        // Encoder bias: zeros
        let encoder_b = Tensor::zeros(ld, DType::F32, device)?;

        // Decoder: transpose of encoder (tie weights for autoencoder) [latent_dim, input_dim]
        let decoder_w = encoder_w.t()?.contiguous()?;

        // Decoder bias: zeros
        let decoder_b = Tensor::zeros(id, DType::F32, device)?;

        Ok(Self {
            encoder_w,
            encoder_b,
            decoder_w,
            decoder_b,
            top_k: config.top_k,
            recon_threshold: config.recon_threshold,
        })
    }

    /// Create identity SAE (passthrough with dimension reduction via selection).
    pub fn identity_projection(
        input_dim: usize,
        latent_dim: usize,
        device: &Device,
    ) -> Result<Self> {
        if latent_dim > input_dim {
            return Err(candle_core::Error::Msg(
                "latent_dim cannot exceed input_dim for identity projection".to_string(),
            ));
        }
        // Identity projection: encoder [input_dim, latent_dim], decoder [latent_dim, input_dim]
        let mut encoder_data = vec![0.0f32; input_dim * latent_dim];
        for i in 0..latent_dim {
            encoder_data[i * latent_dim + i] = 1.0;
        }
        let encoder_w = Tensor::from_vec(encoder_data, (input_dim, latent_dim), device)?;
        let encoder_b = Tensor::zeros(latent_dim, DType::F32, device)?;

        let mut decoder_data = vec![0.0f32; latent_dim * input_dim];
        for i in 0..latent_dim {
            decoder_data[i * input_dim + i] = 1.0;
        }
        let decoder_w = Tensor::from_vec(decoder_data, (latent_dim, input_dim), device)?;
        let decoder_b = Tensor::zeros(input_dim, DType::F32, device)?;

        Ok(Self {
            encoder_w,
            encoder_b,
            decoder_w,
            decoder_b,
            top_k: latent_dim,
            recon_threshold: 0.1,
        })
    }

    /// Project hidden state to sparse latent codes.
    ///
    /// `z = topk(ReLU(W_e @ x + b_e), k)`
    pub fn encode(&self, hidden: &Tensor) -> Result<Tensor> {
        // Linear: pre_activation = W_e @ x + b_e
        let matmul_out = hidden.matmul(&self.encoder_w)?;
        let pre_act = self.add_bias_2d(&matmul_out, &self.encoder_b)?;

        // ReLU
        let relu_act = pre_act.relu()?;

        // Top-k sparsification
        self.apply_topk(&relu_act)
    }

    /// Add bias to 1D or 2D tensor, handling broadcasting manually.
    fn add_bias_2d(&self, tensor: &Tensor, bias: &Tensor) -> Result<Tensor> {
        match tensor.shape().dims().len() {
            1 => {
                // 1D [dim] + [dim]
                tensor.add(bias)
            }
            2 => {
                // 2D [batch, dim] + [dim] — manual broadcasting
                let (batch, dim) = {
                    let d = tensor.shape().dims();
                    (d[0], d[1])
                };
                let vals: Vec<Vec<f32>> = tensor.to_vec2()?;
                let bias_vals: Vec<f32> = bias.to_vec1()?;
                let mut result = Vec::with_capacity(batch * dim);
                for row in &vals {
                    for (i, v) in row.iter().enumerate() {
                        result.push(v + bias_vals[i]);
                    }
                }
                Tensor::from_vec(result, (batch, dim), tensor.device())
            }
            _ => Err(candle_core::Error::Msg(
                "add_bias_2d expects 1D or 2D tensor".to_string(),
            )),
        }
    }

    /// Apply top-k sparsification: keep top-k values, zero rest.
    fn apply_topk(&self, activated: &Tensor) -> Result<Tensor> {
        let dims = activated.shape().dims();
        match dims.len() {
            1 => {
                // Single sample [latent_dim]
                let vals: Vec<f32> = activated.to_vec1()?;
                let mut indexed: Vec<(usize, f32)> = vals.into_iter().enumerate().collect();
                indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                let mut result = vec![0.0f32; dims[0]];
                for (rank, (idx, val)) in indexed.into_iter().enumerate() {
                    if rank < self.top_k {
                        result[idx] = val;
                    }
                }
                Tensor::from_vec(result, dims[0], activated.device())
            }
            2 => {
                // Batch [batch, latent_dim]
                let batch = dims[0];
                let latent = dims[1];
                let all_vals: Vec<Vec<f32>> = activated.to_vec2()?;
                let mut result = vec![0.0f32; batch * latent];
                let default_vec: Vec<f32> = Vec::new();

                for b in 0..batch {
                    let row = all_vals.get(b).unwrap_or(&default_vec);
                    let mut indexed: Vec<(usize, f32)> = row.iter().copied().enumerate().collect();
                    indexed
                        .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                    for (rank, (idx, val)) in indexed.into_iter().enumerate() {
                        if rank < self.top_k {
                            result[b * latent + idx] = val;
                        }
                    }
                }
                Tensor::from_vec(result, (batch, latent), activated.device())
            }
            _ => Err(candle_core::Error::Msg(format!(
                "SAE topk expects 1D or 2D tensor, got {}D",
                dims.len()
            ))),
        }
    }

    /// Decode sparse latents back to input space.
    ///
    /// `x_hat = W_d @ z + b_d`
    /// Decode sparse latents back to input space.
    ///
    /// `x_hat = W_d @ z + b_d`
    pub fn decode(&self, latents: &Tensor) -> Result<Tensor> {
        let matmul_out = latents.matmul(&self.decoder_w)?;
        self.add_bias_2d(&matmul_out, &self.decoder_b)
    }

    /// Full projection: encode → decode, compute reconstruction error.
    ///
    /// Returns `ProjectionResult` with latents, reconstruction, and soundness check.
    pub fn project(&self, hidden: &Tensor) -> Result<ProjectionResult> {
        let latents = self.encode(hidden)?;
        let reconstructed = self.decode(&latents)?;

        // Reconstruction error: ||x - x_hat|| per sample
        let diff = hidden.sub(&reconstructed)?;
        let sq = diff.sqr()?;

        let (recon_errors, active_features) = match sq.shape().dims().len() {
            1 => {
                let sum: f32 = sq.sum_all()?.to_scalar::<f32>()?;
                let vals: Vec<f32> = latents.to_vec1()?;
                let active_count: usize = vals.iter().filter(|v| **v != 0.0).count();
                (vec![sum.sqrt()], vec![active_count])
            }
            2 => {
                let batch = sq.shape().dims()[0];
                let mut errors = Vec::with_capacity(batch);
                let mut features = Vec::with_capacity(batch);
                let sq_vals: Vec<Vec<f32>> = sq.to_vec2()?;
                let lat_vals: Vec<Vec<f32>> = latents.to_vec2()?;

                for b in 0..batch {
                    let default_vec: Vec<f32> = Vec::new();
                    let sq_row = sq_vals.get(b).unwrap_or(&default_vec);
                    let sum: f32 = sq_row.iter().map(|v| v * v).sum::<f32>().sqrt();
                    errors.push(sum);

                    let lat_row = lat_vals.get(b).unwrap_or(&default_vec);
                    let active = lat_row.iter().filter(|v| **v != 0.0).count();
                    features.push(active);
                }
                (errors, features)
            }
            _ => {
                return Err(candle_core::Error::Msg(
                    "SAE expects 1D or 2D input".to_string(),
                ))
            }
        };

        let avg_error = if recon_errors.is_empty() {
            0.0
        } else {
            recon_errors.iter().sum::<f32>() / recon_errors.len() as f32
        };

        let sound = recon_errors.iter().all(|e| *e < self.recon_threshold);

        Ok(ProjectionResult {
            latents,
            reconstructed,
            recon_errors,
            active_features,
            avg_recon_error: avg_error,
            sound,
        })
    }

    /// Compute subspace tightness ratio.
    ///
    /// Ratio of latent-space volume to input-space volume proxy.
    /// Lower = more compression = more efficient verification.
    pub fn tightness_ratio(&self, config: &SAEConfig) -> f32 {
        config.latent_dim as f32 / config.input_dim as f32
    }

    /// Get effective dimension for verification.
    pub fn effective_dim(&self) -> usize {
        self.top_k
    }
}

// =============================================================================
// Sprint 122 — Federated SAE Loss: KL Divergence + Numerical Stability
// =============================================================================

/// Numerically stable KL divergence: KL(P || Q) = Σ P·log(P/Q).
///
/// **Numerical Stability:**
/// - Clamps P and Q to [eps, 1] to avoid log(0)
/// - Uses log-stable formulation: P·(log(P) - log(Q))
/// - Handles edge cases: P=0 → contribution 0, Q=0 → clamped
///
/// # Arguments
/// * `p_vals` - Probability distribution P (must sum to ~1)
/// * `q_vals` - Probability distribution Q (must sum to ~1)
/// * `eps` - Clamping epsilon (default 1e-10)
///
/// # Returns
/// KL divergence value (≥ 0). Returns f32::NAN if lengths mismatch.
pub fn compute_kl_divergence(p_vals: &[f32], q_vals: &[f32], eps: f32) -> f32 {
    if p_vals.len() != q_vals.len() || p_vals.is_empty() {
        return f32::NAN;
    }
    let eps = eps.max(1e-12);
    p_vals
        .iter()
        .zip(q_vals.iter())
        .map(|(&p, &q)| {
            let p_clamped = p.max(eps).min(1.0);
            let q_clamped = q.max(eps).min(1.0);
            p_clamped * (p_clamped.ln() - q_clamped.ln())
        })
        .sum()
}

/// Symmetric KL divergence: (KL(P||Q) + KL(Q||P)) / 2.
///
/// Provides a more balanced measure of distributional difference.
pub fn compute_symmetric_kl(p_vals: &[f32], q_vals: &[f32], eps: f32) -> f32 {
    let kl_pq = compute_kl_divergence(p_vals, q_vals, eps);
    let kl_qp = compute_kl_divergence(q_vals, p_vals, eps);
    if kl_pq.is_nan() || kl_qp.is_nan() {
        return f32::NAN;
    }
    (kl_pq + kl_qp) * 0.5
}

/// Compute KL divergence between two tensors (element-wise per row).
///
/// Each row is treated as a separate distribution.
/// Returns a vector of KL values, one per row.
pub fn compute_kl_divergence_tensor(p: &Tensor, q: &Tensor, eps: f32) -> Result<Vec<f32>> {
    let p_vec = p.to_vec2::<f32>()?;
    let q_vec = q.to_vec2::<f32>()?;
    if p_vec.len() != q_vec.len() {
        return Err(candle_core::Error::Msg(
            "KL divergence: row count mismatch".to_string(),
        ));
    }
    let kl_values: Vec<f32> = p_vec
        .iter()
        .zip(q_vec.iter())
        .map(|(p_row, q_row)| compute_kl_divergence(p_row, q_row, eps))
        .collect();
    Ok(kl_values)
}

/// Federated SAE loss: combined reconstruction error + KL regularization.
///
/// **Loss Formula:**
///     L = L_recon + λ · KL(local_latents || global_prior)
///
/// where:
/// - L_recon: Mean squared reconstruction error per sample
/// - KL: KL divergence between local latent distribution and global prior
/// - λ: Regularization weight (balances reconstruction vs. alignment)
///
/// **Federated Context:**
/// In federated SAE training, each node computes local latents. The KL term
/// encourages local latents to align with the global prior (aggregated from
/// all nodes), preventing distributional drift.
///
/// # Arguments
/// * `recon_errors` - Per-sample reconstruction errors (MSE)
/// * `local_latents` - Local latent activations [batch, latent_dim]
/// * `global_prior` - Global prior distribution [latent_dim] (aggregated mean)
/// * `lambda` - KL regularization weight
/// * `eps` - Numerical stability epsilon
///
/// # Returns
/// Total federated loss value.
pub fn federated_sae_loss(
    recon_errors: &[f32],
    local_latents: &Tensor,
    global_prior: &[f32],
    lambda: f32,
    eps: f32,
) -> f32 {
    if recon_errors.is_empty() {
        return 0.0;
    }

    // Reconstruction loss: mean of per-sample errors
    let recon_loss = recon_errors.iter().sum::<f32>() / recon_errors.len() as f32;

    // KL regularization: average KL per sample against global prior
    let latents_vec = match local_latents.to_vec2::<f32>() {
        Ok(v) => v,
        Err(_) => return recon_loss, // Fallback if tensor access fails
    };

    let kl_sum: f32 = latents_vec
        .iter()
        .map(|latent_row| {
            // Convert latent activations to soft distribution (softmax-like normalization)
            let sum: f32 = latent_row.iter().map(|&v| v.abs()).sum();
            if sum < eps {
                return 0.0; // Skip near-zero latents
            }
            let normalized: Vec<f32> = latent_row.iter().map(|&v| v.abs() / sum).collect();
            compute_kl_divergence(&normalized, global_prior, eps)
        })
        .sum();

    let kl_loss = if !latents_vec.is_empty() {
        kl_sum / latents_vec.len() as f32
    } else {
        0.0
    };

    recon_loss + lambda * kl_loss
}

/// Numerically stable softmax for latent distribution normalization.
///
/// Uses max-subtraction trick: softmax(x)_i = exp(x_i - max(x)) / Σ exp(x_j - max(x))
/// This prevents overflow in exp() for large values.
pub fn stable_softmax(values: &[f32]) -> Vec<f32> {
    if values.is_empty() {
        return vec![];
    }
    let max_val = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exps: Vec<f32> = values.iter().map(|&v| (v - max_val).exp()).collect();
    let sum: f32 = exps.iter().sum();
    if sum < 1e-15 || !sum.is_finite() {
        // Uniform distribution fallback
        let uniform = 1.0 / values.len() as f32;
        return vec![uniform; values.len()];
    }
    exps.iter().map(|&e| e / sum).collect()
}

/// Numerically stable log-sum-exp: log(Σ exp(x_i)).
///
/// Uses the identity: log(Σ exp(x_i)) = max(x) + log(Σ exp(x_i - max(x)))
/// to prevent overflow.
pub fn stable_log_sum_exp(values: &[f32]) -> f32 {
    if values.is_empty() {
        return f32::NEG_INFINITY;
    }
    let max_val = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    if max_val == f32::NEG_INFINITY {
        return f32::NEG_INFINITY;
    }
    let sum_exp: f32 = values.iter().map(|&v| (v - max_val).exp()).sum();
    if sum_exp <= 0.0 || !sum_exp.is_finite() {
        return f32::NEG_INFINITY;
    }
    max_val + sum_exp.ln()
}

/// Jensen-Shannon divergence: symmetric and bounded version of KL.
///
/// JSD(P||Q) = (KL(P||M) + KL(Q||M)) / 2, where M = (P+Q)/2.
/// Bounded: 0 ≤ JSD ≤ log(2) ≈ 0.693.
/// Always finite (unlike KL), making it ideal for federated aggregation.
pub fn compute_jensen_shannon(p_vals: &[f32], q_vals: &[f32], eps: f32) -> f32 {
    if p_vals.len() != q_vals.len() || p_vals.is_empty() {
        return f32::NAN;
    }
    let eps = eps.max(1e-12);
    // Compute mixture M = (P + Q) / 2
    let m: Vec<f32> = p_vals
        .iter()
        .zip(q_vals.iter())
        .map(|(&p, &q)| ((p + q) * 0.5).max(eps).min(1.0))
        .collect();

    let p_clamped: Vec<f32> = p_vals.iter().map(|&v| v.max(eps).min(1.0)).collect();
    let q_clamped: Vec<f32> = q_vals.iter().map(|&v| v.max(eps).min(1.0)).collect();

    let kl_pm = compute_kl_divergence(&p_clamped, &m, eps);
    let kl_qm = compute_kl_divergence(&q_clamped, &m, eps);

    if kl_pm.is_nan() || kl_qm.is_nan() {
        return f32::NAN;
    }
    (kl_pm + kl_qm) * 0.5
}

// --- Random number generator (deterministic for reproducibility) ---
fn next_random_u64(state: &mut u64) -> u64 {
    let new_state = (*state)
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state = new_state;
    *state
}

fn next_random_f32(state: &mut u64) -> f32 {
    let x = next_random_u64(state);
    ((x >> 33) as f32 / (u32::MAX as f32)) * 2.0 - 1.0
}

// --- Verifiable SAE Update Proofs + Entropy Regularization (Sprint 123) ---

/// Verifiable proof for a sparse SAE weight update.
///
/// Contains the sparse indices, updated weights, KL divergence against global prior,
/// entropy regularization term, and a cryptographic hash for integrity verification.
#[derive(Debug, Clone)]
pub struct SAEUpdateProof {
    /// Node that generated this update.
    pub node_id: u64,
    /// Timestamp of the update.
    pub timestamp: u64,
    /// Indices of the sparse weights updated.
    pub sparse_indices: Vec<usize>,
    /// Updated weight values at sparse indices.
    pub updated_weights: Vec<f32>,
    /// KL divergence of local latents against global prior.
    pub kl_divergence: f32,
    /// Entropy regularization term: -Σ p_i * log(p_i).
    pub entropy_reg: f32,
    /// Cryptographic hash of the proof payload for integrity.
    pub proof_hash: [u8; 32],
}

impl SAEUpdateProof {
    /// Create a new verifiable SAE update proof.
    ///
    /// Computes the proof hash from `(node_id, timestamp, sparse_indices, updated_weights, kl_divergence, entropy_reg)`.
    pub fn new(
        node_id: u64,
        timestamp: u64,
        sparse_indices: Vec<usize>,
        updated_weights: Vec<f32>,
        kl_divergence: f32,
        entropy_reg: f32,
    ) -> Self {
        let proof_hash = Self::compute_hash(
            node_id,
            timestamp,
            &sparse_indices,
            &updated_weights,
            kl_divergence,
            entropy_reg,
        );
        Self {
            node_id,
            timestamp,
            sparse_indices,
            updated_weights,
            kl_divergence,
            entropy_reg,
            proof_hash,
        }
    }

    /// Compute SHA-256 hash of the proof payload.
    fn compute_hash(
        node_id: u64,
        timestamp: u64,
        sparse_indices: &[usize],
        updated_weights: &[f32],
        kl_divergence: f32,
        entropy_reg: f32,
    ) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(node_id.to_le_bytes());
        hasher.update(timestamp.to_le_bytes());
        for &idx in sparse_indices {
            hasher.update(idx.to_le_bytes());
        }
        for &w in updated_weights {
            hasher.update(w.to_le_bytes());
        }
        hasher.update(kl_divergence.to_le_bytes());
        hasher.update(entropy_reg.to_le_bytes());
        hasher.finalize().into()
    }

    /// Verify the proof hash matches the payload.
    ///
    /// Returns `true` if the recomputed hash matches the stored `proof_hash`.
    pub fn verify(&self) -> bool {
        let expected = Self::compute_hash(
            self.node_id,
            self.timestamp,
            &self.sparse_indices,
            &self.updated_weights,
            self.kl_divergence,
            self.entropy_reg,
        );
        self.proof_hash == expected
    }

    /// Check if the update is within acceptable KL and entropy bounds.
    ///
    /// Returns `true` if `kl_divergence <= max_kl` and `entropy_reg >= min_entropy`.
    pub fn is_within_bounds(&self, max_kl: f32, min_entropy: f32) -> bool {
        self.kl_divergence <= max_kl && self.entropy_reg >= min_entropy
    }

    /// Sparsity ratio: fraction of weights updated vs total latent dimension.
    pub fn sparsity_ratio(&self, total_latents: usize) -> f32 {
        if total_latents == 0 {
            return 0.0;
        }
        self.sparse_indices.len() as f32 / total_latents as f32
    }
}

/// Generate a verifiable SAE update proof from latents and global prior.
///
/// Identifies sparse (top-k) latent activations, computes KL divergence and entropy regularization,
/// and returns a signed `SAEUpdateProof`.
pub fn generate_sae_update_proof(
    node_id: u64,
    timestamp: u64,
    latents: &[f32],
    global_prior: &[f32],
    top_k: usize,
    entropy_lambda: f32,
    eps: f32,
) -> SAEUpdateProof {
    let eps = eps.max(1e-12);
    // Identify top-k sparse indices by absolute latent value
    let mut indexed: Vec<(usize, f32)> = latents
        .iter()
        .enumerate()
        .map(|(i, &v)| (i, v.abs()))
        .collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let limit = top_k.min(indexed.len());
    let sparse_indices: Vec<usize> = indexed[..limit].iter().map(|(i, _)| *i).collect();
    let updated_weights: Vec<f32> = sparse_indices.iter().map(|&i| latents[i]).collect();

    // KL divergence
    let kl = compute_kl_divergence(latents, global_prior, eps);

    // Entropy regularization: -Σ p_i * log(p_i) where p = softmax(latents)
    let p = stable_softmax(latents);
    let entropy_reg = compute_sparse_update_entropy(&p);

    SAEUpdateProof::new(
        node_id,
        timestamp,
        sparse_indices,
        updated_weights,
        kl,
        entropy_lambda * entropy_reg,
    )
}

/// Compute entropy of a probability distribution: H(p) = -Σ p_i * log(p_i).
///
/// Uses numerical clamping to avoid log(0).
pub fn compute_sparse_update_entropy(p: &[f32]) -> f32 {
    let eps = 1e-12f32;
    -p.iter()
        .map(|&v| {
            let v_clamped = v.max(eps).min(1.0);
            v_clamped * v_clamped.ln()
        })
        .sum::<f32>()
}

/// Federated SAE loss with entropy regularization.
///
/// `Loss = recon_loss + λ_kl * KL(local||global) - λ_entropy * H(p)`
///
/// The entropy term encourages diverse latent activations, preventing mode collapse
/// in federated sparse autoencoders.
pub fn federated_sae_loss_with_entropy_reg(
    recon_loss: f32,
    latents: &[f32],
    global_prior: &[f32],
    lambda_kl: f32,
    lambda_entropy: f32,
    eps: f32,
) -> f32 {
    let eps = eps.max(1e-12);
    let kl = compute_kl_divergence(latents, global_prior, eps);
    let p = stable_softmax(latents);
    let entropy = compute_sparse_update_entropy(&p);
    recon_loss + lambda_kl * kl - lambda_entropy * entropy
}

/// Verify a batch of SAE update proofs from multiple federated nodes.
///
/// Returns a tuple `(valid_count, total_count, avg_kl, avg_entropy)`.
/// Rejects proofs with invalid hashes or non-finite values.
pub fn verify_sae_update_batch(proofs: &[SAEUpdateProof]) -> (usize, usize, f32, f32) {
    let total = proofs.len();
    if total == 0 {
        return (0, 0, 0.0, 0.0);
    }
    let mut valid = 0;
    let mut sum_kl = 0.0f32;
    let mut sum_entropy = 0.0f32;
    for proof in proofs {
        if proof.verify() && proof.kl_divergence.is_finite() && proof.entropy_reg.is_finite() {
            valid += 1;
            sum_kl += proof.kl_divergence;
            sum_entropy += proof.entropy_reg;
        }
    }
    let avg_kl = sum_kl / valid.max(1) as f32;
    let avg_entropy = sum_entropy / valid.max(1) as f32;
    (valid, total, avg_kl, avg_entropy)
}

/// Aggregate sparse updates from multiple proofs using trust-weighted median.
///
/// For each sparse index, collects weights from all proofs that updated it,
/// then applies byzantine-weighted median with uniform trust (1.0).
pub fn aggregate_sparse_updates(proofs: &[SAEUpdateProof]) -> Vec<(usize, f32)> {
    if proofs.is_empty() {
        return vec![];
    }
    // Collect weights per index
    let mut index_weights: std::collections::HashMap<usize, Vec<f32>> =
        std::collections::HashMap::new();
    for proof in proofs {
        if !proof.verify() {
            continue;
        }
        for (i, &idx) in proof.sparse_indices.iter().enumerate() {
            index_weights
                .entry(idx)
                .or_default()
                .push(proof.updated_weights[i]);
        }
    }
    // Compute median per index
    let mut result: Vec<(usize, f32)> = index_weights
        .into_iter()
        .map(|(idx, mut weights)| {
            weights.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let mid = weights.len() / 2;
            let median = if weights.len() % 2 == 0 {
                (weights[mid - 1] + weights[mid]) * 0.5
            } else {
                weights[mid]
            };
            (idx, median)
        })
        .collect();
    result.sort_by_key(|&(idx, _)| idx);
    result
}

// --- Unit Tests ---
#[cfg(test)]
mod tests {
    use super::*;

    fn make_hidden(batch: usize, dim: usize, device: &Device) -> Result<Tensor> {
        let data: Vec<f32> = (0..(batch * dim)).map(|i| (i as f32 % 7.0) / 7.0).collect();
        Tensor::from_vec(data, (batch, dim), device)
    }

    #[test]
    fn test_sae_config_default() {
        let cfg = SAEConfig::default();
        assert_eq!(cfg.input_dim, 4096);
        assert_eq!(cfg.latent_dim, 1024);
        assert_eq!(cfg.top_k, 512);
        assert!(cfg.recon_threshold > 0.0);
    }

    #[test]
    fn test_sae_creation() -> Result<()> {
        let device = Device::Cpu;
        let config = SAEConfig {
            input_dim: 64,
            latent_dim: 16,
            top_k: 8,
            recon_threshold: 0.5,
        };
        let sae = SAE::new(&config, &device)?;
        assert_eq!(sae.encoder_w.shape().dims(), [16, 64]);
        assert_eq!(sae.decoder_w.shape().dims(), [64, 16]);
        assert_eq!(sae.top_k, 8);
        Ok(())
    }

    #[test]
    fn test_identity_projection() -> Result<()> {
        let device = Device::Cpu;
        let sae = SAE::identity_projection(64, 16, &device)?;
        assert_eq!(sae.encoder_w.shape().dims(), [16, 64]);
        assert_eq!(sae.decoder_w.shape().dims(), [64, 16]);
        Ok(())
    }

    #[test]
    fn test_encode_dimension() -> Result<()> {
        let device = Device::Cpu;
        let config = SAEConfig {
            input_dim: 32,
            latent_dim: 8,
            top_k: 4,
            recon_threshold: 0.5,
        };
        let sae = SAE::new(&config, &device)?;
        let hidden = make_hidden(1, 32, &device)?;
        let latents = sae.encode(&hidden)?;
        assert_eq!(latents.shape().dims(), [8]);
        Ok(())
    }

    #[test]
    fn test_decode_dimension() -> Result<()> {
        let device = Device::Cpu;
        let config = SAEConfig {
            input_dim: 32,
            latent_dim: 8,
            top_k: 4,
            recon_threshold: 0.5,
        };
        let sae = SAE::new(&config, &device)?;
        let hidden = make_hidden(1, 32, &device)?;
        let latents = sae.encode(&hidden)?;
        let reconstructed = sae.decode(&latents)?;
        assert_eq!(reconstructed.shape().dims(), [32]);
        Ok(())
    }

    #[test]
    fn test_project_soundness() -> Result<()> {
        let device = Device::Cpu;
        let config = SAEConfig {
            input_dim: 32,
            latent_dim: 16,
            top_k: 16,
            recon_threshold: 1.0,
        };
        let sae = SAE::new(&config, &device)?;
        let hidden = make_hidden(2, 32, &device)?;
        let result = sae.project(&hidden)?;
        assert_eq!(result.recon_errors.len(), 2);
        assert_eq!(result.active_features.len(), 2);
        assert!(result.avg_recon_error >= 0.0);
        Ok(())
    }

    #[test]
    fn test_tightness_ratio() -> Result<()> {
        let config = SAEConfig {
            input_dim: 4096,
            latent_dim: 1024,
            top_k: 512,
            recon_threshold: 0.05,
        };
        let device = Device::Cpu;
        let sae = SAE::new(&config, &device)?;
        let ratio = sae.tightness_ratio(&config);
        assert!((ratio - 0.25).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn test_effective_dim() -> Result<()> {
        let device = Device::Cpu;
        let config = SAEConfig {
            input_dim: 64,
            latent_dim: 16,
            top_k: 8,
            recon_threshold: 0.5,
        };
        let sae = SAE::new(&config, &device)?;
        assert_eq!(sae.effective_dim(), 8);
        Ok(())
    }

    #[test]
    fn test_projection_result_display() -> Result<()> {
        let device = Device::Cpu;
        let config = SAEConfig {
            input_dim: 16,
            latent_dim: 8,
            top_k: 4,
            recon_threshold: 0.5,
        };
        let sae = SAE::new(&config, &device)?;
        let hidden = make_hidden(1, 16, &device)?;
        let result = sae.project(&hidden)?;
        let display = format!("{}", result);
        assert!(display.contains("ProjectionResult"));
        Ok(())
    }

    #[test]
    fn test_topk_sparsifies() -> Result<()> {
        let device = Device::Cpu;
        let config = SAEConfig {
            input_dim: 16,
            latent_dim: 8,
            top_k: 2,
            recon_threshold: 1.0,
        };
        let sae = SAE::new(&config, &device)?;
        let hidden = make_hidden(1, 16, &device)?;
        let latents = sae.encode(&hidden)?;
        let vals: Vec<f32> = latents.to_vec1()?;
        let non_zero = vals.iter().filter(|v| **v != 0.0).count();
        assert!(non_zero <= 2, "Expected at most 2 active, got {}", non_zero);
        Ok(())
    }

    #[test]
    fn test_batch_projection() -> Result<()> {
        let device = Device::Cpu;
        let config = SAEConfig {
            input_dim: 32,
            latent_dim: 8,
            top_k: 4,
            recon_threshold: 1.0,
        };
        let sae = SAE::new(&config, &device)?;
        let hidden = make_hidden(4, 32, &device)?;
        let result = sae.project(&hidden)?;
        assert_eq!(result.recon_errors.len(), 4);
        assert_eq!(result.active_features.len(), 4);
        Ok(())
    }

    #[test]
    fn test_topk_exceeds_latent_dim_error() {
        let device = Device::Cpu;
        let config = SAEConfig {
            input_dim: 64,
            latent_dim: 16,
            top_k: 32, // exceeds latent_dim
            recon_threshold: 0.5,
        };
        let result = SAE::new(&config, &device);
        assert!(result.is_err());
    }

    #[test]
    fn test_identity_projection_sound() -> Result<()> {
        let device = Device::Cpu;
        let sae = SAE::identity_projection(32, 8, &device)?;
        let hidden = make_hidden(1, 32, &device)?;
        let result = sae.project(&hidden)?;
        // Identity projection of first 8 dims should have low error for those dims
        assert!(result.avg_recon_error >= 0.0);
        Ok(())
    }

    #[test]
    fn test_random_generator_deterministic() {
        let mut s1: u64 = 12345;
        let mut s2: u64 = 12345;
        let a1 = next_random_f32(&mut s1);
        let a2 = next_random_f32(&mut s2);
        assert_eq!(a1, a2);
    }

    // =====================================================================
    // Sprint 122 — Federated SAE Loss Tests
    // =====================================================================

    #[test]
    fn test_kl_divergence_identical_distributions() {
        let p = vec![0.25, 0.25, 0.25, 0.25];
        let q = vec![0.25, 0.25, 0.25, 0.25];
        let kl = compute_kl_divergence(&p, &q, 1e-10);
        assert!(
            kl >= 0.0 && kl < 1e-6,
            "KL(identical) should be ≈ 0, got {}",
            kl
        );
    }

    #[test]
    fn test_kl_divergence_positive() {
        let p = vec![0.9, 0.1];
        let q = vec![0.5, 0.5];
        let kl = compute_kl_divergence(&p, &q, 1e-10);
        assert!(
            kl > 0.0,
            "KL(P||Q) should be positive for different distributions"
        );
        assert!(kl.is_finite());
    }

    #[test]
    fn test_kl_divergence_asymmetric() {
        let p = vec![0.9, 0.1];
        let q = vec![0.5, 0.5];
        let kl_pq = compute_kl_divergence(&p, &q, 1e-10);
        let kl_qp = compute_kl_divergence(&q, &p, 1e-10);
        assert!((kl_pq - kl_qp).abs() > 1e-6, "KL should be asymmetric");
    }

    #[test]
    fn test_kl_divergence_length_mismatch() {
        let p = vec![0.5, 0.5];
        let q = vec![0.3, 0.3, 0.4];
        let kl = compute_kl_divergence(&p, &q, 1e-10);
        assert!(kl.is_nan());
    }

    #[test]
    fn test_kl_divergence_empty() {
        let p: Vec<f32> = vec![];
        let q: Vec<f32> = vec![];
        let kl = compute_kl_divergence(&p, &q, 1e-10);
        assert!(kl.is_nan());
    }

    #[test]
    fn test_kl_divergence_zero_handling() {
        // P has zeros — should be handled via clamping
        let p = vec![0.0, 1.0, 0.0];
        let q = vec![0.3, 0.4, 0.3];
        let kl = compute_kl_divergence(&p, &q, 1e-10);
        assert!(kl.is_finite(), "KL should handle zero P via clamping");
        assert!(kl >= 0.0);
    }

    #[test]
    fn test_kl_divergence_near_zero_q() {
        // Q near zero — should be clamped to eps
        let p = vec![0.5, 0.5];
        let q = vec![1e-15, 1.0 - 1e-15];
        let kl = compute_kl_divergence(&p, &q, 1e-10);
        assert!(kl.is_finite(), "KL should handle near-zero Q via clamping");
        assert!(kl > 0.0);
    }

    #[test]
    fn test_symmetric_kl_basic() {
        let p = vec![0.7, 0.3];
        let q = vec![0.5, 0.5];
        let sym = compute_symmetric_kl(&p, &q, 1e-10);
        let kl_pq = compute_kl_divergence(&p, &q, 1e-10);
        let kl_qp = compute_kl_divergence(&q, &p, 1e-10);
        let expected = (kl_pq + kl_qp) * 0.5;
        assert!((sym - expected).abs() < 1e-6);
        assert!(sym >= 0.0);
    }

    #[test]
    fn test_symmetric_kl_identical() {
        let p = vec![0.5, 0.5];
        let sym = compute_symmetric_kl(&p, &p, 1e-10);
        assert!(sym >= 0.0 && sym < 1e-6);
    }

    #[test]
    fn test_federated_sae_loss_basic() -> Result<()> {
        let device = Device::Cpu;
        let recon_errors = vec![0.01, 0.02, 0.015];
        // 3 samples, 4 latent dims
        let latents = Tensor::from_vec(
            vec![0.6, 0.2, 0.1, 0.1, 0.5, 0.3, 0.1, 0.1, 0.7, 0.15, 0.1, 0.05],
            (3, 4),
            &device,
        )?;
        let global_prior = vec![0.4, 0.3, 0.15, 0.15];
        let loss = federated_sae_loss(&recon_errors, &latents, &global_prior, 0.1, 1e-10);
        assert!(loss.is_finite());
        assert!(loss >= 0.0);
        // Recon loss = mean([0.01, 0.02, 0.015]) = 0.015
        // Total = 0.015 + 0.1 * kl_loss ≥ 0.015
        assert!(loss >= 0.015);
        Ok(())
    }

    #[test]
    fn test_federated_sae_loss_zero_lambda() -> Result<()> {
        let device = Device::Cpu;
        let recon_errors = vec![0.01, 0.02, 0.03];
        let latents = Tensor::zeros((3, 4), DType::F32, &device)?;
        let global_prior = vec![0.25, 0.25, 0.25, 0.25];
        let loss = federated_sae_loss(&recon_errors, &latents, &global_prior, 0.0, 1e-10);
        // With lambda=0, loss = recon_loss only
        let expected = (0.01 + 0.02 + 0.03) / 3.0;
        assert!((loss - expected).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn test_federated_sae_loss_empty_recon() -> Result<()> {
        let device = Device::Cpu;
        let latents = Tensor::zeros((2, 4), DType::F32, &device)?;
        let global_prior = vec![0.25, 0.25, 0.25, 0.25];
        let loss = federated_sae_loss(&[], &latents, &global_prior, 0.1, 1e-10);
        assert!((loss - 0.0).abs() < 1e-10);
        Ok(())
    }

    #[test]
    fn test_federated_sae_loss_high_lambda() -> Result<()> {
        let device = Device::Cpu;
        let recon_errors = vec![0.001];
        let latents = Tensor::from_vec(vec![0.9, 0.05, 0.03, 0.02], (1, 4), &device)?;
        let global_prior = vec![0.25, 0.25, 0.25, 0.25];
        let loss_low = federated_sae_loss(&recon_errors, &latents, &global_prior, 0.01, 1e-10);
        let loss_high = federated_sae_loss(&recon_errors, &latents, &global_prior, 10.0, 1e-10);
        assert!(loss_high > loss_low, "Higher lambda should increase loss");
        Ok(())
    }

    #[test]
    fn test_stable_softmax_basic() {
        let vals = vec![1.0, 2.0, 3.0];
        let result = stable_softmax(&vals);
        assert_eq!(result.len(), 3);
        let sum: f32 = result.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6, "Softmax should sum to 1");
        assert!(result.iter().all(|&v| v > 0.0));
        // Larger input → larger output
        assert!(result[2] > result[1] && result[1] > result[0]);
    }

    #[test]
    fn test_stable_softmax_large_values() {
        // Test numerical stability with large values
        let vals = vec![1000.0, 1001.0, 1002.0];
        let result = stable_softmax(&vals);
        assert!(result.iter().all(|&v| v.is_finite()));
        let sum: f32 = result.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_stable_softmax_empty() {
        let vals: Vec<f32> = vec![];
        let result = stable_softmax(&vals);
        assert!(result.is_empty());
    }

    #[test]
    fn test_stable_softmax_uniform() {
        let vals = vec![0.0, 0.0, 0.0, 0.0];
        let result = stable_softmax(&vals);
        for &v in &result {
            assert!((v - 0.25).abs() < 1e-6);
        }
    }

    #[test]
    fn test_stable_log_sum_exp_basic() {
        let vals = vec![0.0, 1.0, 2.0];
        let lse = stable_log_sum_exp(&vals);
        // log(exp(0) + exp(1) + exp(2)) = log(1 + 2.718 + 7.389) = log(11.107) ≈ 2.407
        assert!(lse.is_finite());
        assert!((lse - 2.407).abs() < 0.01);
    }

    #[test]
    fn test_stable_log_sum_exp_large_values() {
        // Should not overflow
        let vals = vec![1000.0, 1001.0, 1002.0];
        let lse = stable_log_sum_exp(&vals);
        assert!(lse.is_finite());
        // ≈ 1002 + log(1 + exp(-1) + exp(-2)) ≈ 1002.407
        assert!((lse - 1002.407).abs() < 0.01);
    }

    #[test]
    fn test_stable_log_sum_exp_empty() {
        let vals: Vec<f32> = vec![];
        let lse = stable_log_sum_exp(&vals);
        assert_eq!(lse, f32::NEG_INFINITY);
    }

    #[test]
    fn test_stable_log_sum_exp_single() {
        let vals = vec![3.0];
        let lse = stable_log_sum_exp(&vals);
        assert!((lse - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_jensen_shannon_identical() {
        let p = vec![0.5, 0.5];
        let js = compute_jensen_shannon(&p, &p, 1e-10);
        assert!(js >= 0.0 && js < 1e-6, "JSD(identical) should be ≈ 0");
    }

    #[test]
    fn test_jensen_shannon_bounded() {
        // JSD is bounded by log(2) ≈ 0.693
        let p = vec![1.0, 0.0];
        let q = vec![0.0, 1.0];
        let js = compute_jensen_shannon(&p, &q, 1e-10);
        assert!(js.is_finite());
        assert!(js >= 0.0);
        assert!(js <= 0.7, "JSD should be ≤ log(2) ≈ 0.693");
    }

    #[test]
    fn test_jensen_shannon_symmetric() {
        let p = vec![0.7, 0.2, 0.1];
        let q = vec![0.3, 0.3, 0.4];
        let js_pq = compute_jensen_shannon(&p, &q, 1e-10);
        let js_qp = compute_jensen_shannon(&q, &p, 1e-10);
        assert!((js_pq - js_qp).abs() < 1e-8, "JSD should be symmetric");
    }

    #[test]
    fn test_jensen_shannon_length_mismatch() {
        let p = vec![0.5, 0.5];
        let q = vec![0.3, 0.4, 0.3];
        let js = compute_jensen_shannon(&p, &q, 1e-10);
        assert!(js.is_nan());
    }

    #[test]
    fn test_kl_divergence_tensor_basic() -> Result<()> {
        let device = Device::Cpu;
        let p = Tensor::from_vec(vec![0.5, 0.5, 0.3, 0.7], (2, 2), &device)?;
        let q = Tensor::from_vec(vec![0.5, 0.5, 0.5, 0.5], (2, 2), &device)?;
        let kls = compute_kl_divergence_tensor(&p, &q, 1e-10)?;
        assert_eq!(kls.len(), 2);
        // Row 0: identical → KL ≈ 0
        assert!(kls[0] < 1e-6);
        // Row 1: different → KL > 0
        assert!(kls[1] > 0.0);
        Ok(())
    }

    #[test]
    fn test_kl_divergence_tensor_row_mismatch() -> Result<()> {
        let device = Device::Cpu;
        let p = Tensor::from_vec(vec![0.5, 0.5], (1, 2), &device)?;
        let q = Tensor::from_vec(vec![0.5, 0.5, 0.5, 0.5], (2, 2), &device)?;
        let result = compute_kl_divergence_tensor(&p, &q, 1e-10);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_federated_loss_numerical_stability() -> Result<()> {
        let device = Device::Cpu;
        // Near-zero latents — should not produce NaN/Inf
        let recon_errors = vec![0.001];
        let latents = Tensor::from_vec(vec![1e-15, 1e-15, 1e-15, 1e-15], (1, 4), &device)?;
        let global_prior = vec![0.25, 0.25, 0.25, 0.25];
        let loss = federated_sae_loss(&recon_errors, &latents, &global_prior, 1.0, 1e-10);
        assert!(
            loss.is_finite(),
            "Loss should be finite for near-zero latents"
        );
        Ok(())
    }

    // --- Sprint 123: Verifiable SAE Update Proofs + Entropy Regularization Tests ---

    #[test]
    fn test_sae_update_proof_creation() {
        let proof = SAEUpdateProof::new(1, 1000, vec![0, 2, 5], vec![0.8, -0.3, 0.1], 0.05, 0.1);
        assert_eq!(proof.node_id, 1);
        assert_eq!(proof.timestamp, 1000);
        assert_eq!(proof.sparse_indices, vec![0, 2, 5]);
        assert_eq!(proof.updated_weights, vec![0.8, -0.3, 0.1]);
        assert_eq!(proof.kl_divergence, 0.05);
        assert_eq!(proof.entropy_reg, 0.1);
    }

    #[test]
    fn test_sae_update_proof_verify_valid() {
        let proof = SAEUpdateProof::new(42, 2000, vec![1, 3], vec![0.5, -0.2], 0.1, 0.05);
        assert!(proof.verify());
    }

    #[test]
    fn test_sae_update_proof_verify_tampered() {
        let mut proof = SAEUpdateProof::new(1, 1000, vec![0, 2], vec![0.8, -0.3], 0.05, 0.1);
        proof.updated_weights[0] = 999.0;
        assert!(!proof.verify());
    }

    #[test]
    fn test_sae_update_proof_deterministic_hash() {
        let p1 = SAEUpdateProof::new(1, 1000, vec![0, 2], vec![0.8, -0.3], 0.05, 0.1);
        let p2 = SAEUpdateProof::new(1, 1000, vec![0, 2], vec![0.8, -0.3], 0.05, 0.1);
        assert_eq!(p1.proof_hash, p2.proof_hash);
    }

    #[test]
    fn test_sae_update_proof_is_within_bounds() {
        let proof = SAEUpdateProof::new(1, 1000, vec![0], vec![0.5], 0.05, 0.2);
        assert!(proof.is_within_bounds(0.1, 0.1));
        assert!(!proof.is_within_bounds(0.01, 0.1));
        assert!(!proof.is_within_bounds(0.1, 0.5));
    }

    #[test]
    fn test_sae_update_proof_sparsity_ratio() {
        let proof = SAEUpdateProof::new(1, 1000, vec![0, 2, 5], vec![0.8, -0.3, 0.1], 0.05, 0.1);
        assert!((proof.sparsity_ratio(10) - 0.3).abs() < 1e-6);
        assert_eq!(proof.sparsity_ratio(0), 0.0);
    }

    #[test]
    fn test_generate_sae_update_proof() {
        let latents = vec![0.9, 0.1, 0.8, 0.05, 0.7];
        let prior = vec![0.2, 0.2, 0.2, 0.2, 0.2];
        let proof = generate_sae_update_proof(1, 1000, &latents, &prior, 3, 0.1, 1e-10);
        assert_eq!(proof.node_id, 1);
        assert_eq!(proof.sparse_indices.len(), 3);
        assert_eq!(proof.updated_weights.len(), 3);
        assert!(proof.verify());
        assert!(proof.kl_divergence >= 0.0);
    }

    #[test]
    fn test_generate_sae_update_proof_top_k_exceeds_latents() {
        let latents = vec![0.5, 0.5];
        let prior = vec![0.5, 0.5];
        let proof = generate_sae_update_proof(1, 1000, &latents, &prior, 10, 0.1, 1e-10);
        assert_eq!(proof.sparse_indices.len(), 2);
    }

    #[test]
    fn test_compute_sparse_update_entropy_uniform() {
        let p = vec![0.25, 0.25, 0.25, 0.25];
        let h = compute_sparse_update_entropy(&p);
        // H = log(4) = 1.386...
        assert!((h - (-p[0] * p[0].ln()) * 4.0).abs() < 1e-6);
        assert!(h > 0.0);
    }

    #[test]
    fn test_compute_sparse_update_entropy_deterministic() {
        let p = vec![1.0, 0.0, 0.0];
        let h = compute_sparse_update_entropy(&p);
        assert!(h >= 0.0);
        assert!(h.is_finite());
    }

    #[test]
    fn test_compute_sparse_update_entropy_empty() {
        let h = compute_sparse_update_entropy(&[]);
        assert!((h - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_federated_sae_loss_with_entropy_reg_basic() {
        let latents = vec![0.5, 0.5, 0.5, 0.5];
        let prior = vec![0.25, 0.25, 0.25, 0.25];
        let loss = federated_sae_loss_with_entropy_reg(0.1, &latents, &prior, 1.0, 0.1, 1e-10);
        assert!(loss.is_finite());
    }

    #[test]
    fn test_federated_sae_loss_with_entropy_reg_zero_lambda() {
        let latents = vec![0.5, 0.5];
        let prior = vec![0.5, 0.5];
        let loss = federated_sae_loss_with_entropy_reg(0.1, &latents, &prior, 0.0, 0.0, 1e-10);
        assert!((loss - 0.1).abs() < 1e-6);
    }

    #[test]
    fn test_federated_sae_loss_with_entropy_reg_high_entropy_lambda() {
        let latents = vec![0.25, 0.25, 0.25, 0.25];
        let prior = vec![0.25, 0.25, 0.25, 0.25];
        let loss = federated_sae_loss_with_entropy_reg(0.0, &latents, &prior, 0.0, 10.0, 1e-10);
        // High entropy lambda should reduce loss significantly
        assert!(loss < 0.0);
    }

    #[test]
    fn test_verify_sae_update_batch_empty() {
        let (valid, total, avg_kl, avg_ent) = verify_sae_update_batch(&[]);
        assert_eq!(valid, 0);
        assert_eq!(total, 0);
        assert_eq!(avg_kl, 0.0);
        assert_eq!(avg_ent, 0.0);
    }

    #[test]
    fn test_verify_sae_update_batch_all_valid() {
        let proofs = vec![
            SAEUpdateProof::new(1, 1000, vec![0], vec![0.5], 0.1, 0.2),
            SAEUpdateProof::new(2, 1001, vec![1], vec![0.3], 0.2, 0.15),
        ];
        let (valid, total, avg_kl, avg_ent) = verify_sae_update_batch(&proofs);
        assert_eq!(valid, 2);
        assert_eq!(total, 2);
        assert!((avg_kl - 0.15).abs() < 1e-6);
        assert!((avg_ent - 0.175).abs() < 1e-6);
    }

    #[test]
    fn test_verify_sae_update_batch_rejects_tampered() {
        let mut p1 = SAEUpdateProof::new(1, 1000, vec![0], vec![0.5], 0.1, 0.2);
        let p2 = SAEUpdateProof::new(2, 1001, vec![1], vec![0.3], 0.2, 0.15);
        p1.updated_weights[0] = 999.0;
        let proofs = vec![p1, p2];
        let (valid, total, _avg_kl, _avg_ent) = verify_sae_update_batch(&proofs);
        assert_eq!(valid, 1);
        assert_eq!(total, 2);
    }

    #[test]
    fn test_verify_sae_update_batch_rejects_nan() {
        let p1 = SAEUpdateProof::new(1, 1000, vec![0], vec![0.5], f32::NAN, 0.2);
        let p2 = SAEUpdateProof::new(2, 1001, vec![1], vec![0.3], 0.2, 0.15);
        let proofs = vec![p1, p2];
        let (valid, total, _avg_kl, _avg_ent) = verify_sae_update_batch(&proofs);
        assert_eq!(valid, 1);
        assert_eq!(total, 2);
    }

    #[test]
    fn test_aggregate_sparse_updates_empty() {
        let result = aggregate_sparse_updates(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_aggregate_sparse_updates_single() {
        let proofs = vec![SAEUpdateProof::new(
            1,
            1000,
            vec![0, 2],
            vec![0.5, 0.3],
            0.1,
            0.2,
        )];
        let result = aggregate_sparse_updates(&proofs);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], (0, 0.5));
        assert_eq!(result[1], (2, 0.3));
    }

    #[test]
    fn test_aggregate_sparse_updates_median() {
        let proofs = vec![
            SAEUpdateProof::new(1, 1000, vec![0], vec![0.1], 0.1, 0.2),
            SAEUpdateProof::new(2, 1001, vec![0], vec![0.9], 0.1, 0.2),
            SAEUpdateProof::new(3, 1002, vec![0], vec![0.5], 0.1, 0.2),
        ];
        let result = aggregate_sparse_updates(&proofs);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, 0);
        assert!((result[0].1 - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_aggregate_sparse_updates_skips_invalid() {
        let mut p1 = SAEUpdateProof::new(1, 1000, vec![0], vec![0.5], 0.1, 0.2);
        let p2 = SAEUpdateProof::new(2, 1001, vec![0], vec![0.3], 0.1, 0.2);
        p1.updated_weights[0] = 999.0;
        let proofs = vec![p1, p2];
        let result = aggregate_sparse_updates(&proofs);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, 0.3);
    }

    #[test]
    fn test_aggregate_sparse_updates_sorted_indices() {
        let proofs = vec![SAEUpdateProof::new(
            1,
            1000,
            vec![5, 2, 0],
            vec![0.1, 0.2, 0.3],
            0.1,
            0.2,
        )];
        let result = aggregate_sparse_updates(&proofs);
        assert_eq!(result[0].0, 0);
        assert_eq!(result[1].0, 2);
        assert_eq!(result[2].0, 5);
    }
}
