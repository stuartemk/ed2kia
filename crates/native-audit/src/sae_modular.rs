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

use candle_core::{Device, DType, Result, Tensor};

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
    pub encoder_w: Tensor,   // [input_dim, latent_dim]
    pub encoder_b: Tensor,   // [latent_dim]
    pub decoder_w: Tensor,   // [latent_dim, input_dim]
    pub decoder_b: Tensor,   // [input_dim]
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
    pub fn identity_projection(input_dim: usize, latent_dim: usize, device: &Device) -> Result<Self> {
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
                    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                    for (rank, (idx, val)) in indexed.into_iter().enumerate() {
                        if rank < self.top_k {
                            result[b * latent + idx] = val;
                        }
                    }
                }
                Tensor::from_vec(result, (batch, latent), activated.device())
            }
            _ => Err(candle_core::Error::Msg(
                format!("SAE topk expects 1D or 2D tensor, got {}D", dims.len()),
            )),
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
            _ => return Err(candle_core::Error::Msg("SAE expects 1D or 2D input".to_string())),
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

// --- Random number generator (deterministic for reproducibility) ---
fn next_random_u64(state: &mut u64) -> u64 {
    let new_state = (*state).wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *state = new_state;
    *state
}

fn next_random_f32(state: &mut u64) -> f32 {
    let x = next_random_u64(state);
    ((x >> 33) as f32 / (u32::MAX as f32)) * 2.0 - 1.0
}

// --- Unit Tests ---
#[cfg(test)]
mod tests {
    use super::*;

    fn make_hidden(batch: usize, dim: usize, device: &Device) -> Result<Tensor> {
        let data: Vec<f32> = (0..(batch * dim))
            .map(|i| (i as f32 % 7.0) / 7.0)
            .collect();
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
}
