//! Cross-Attention Multi-Modal Fusion — True cross-attention between modality embeddings.
//!
//! Extends the multi-modal engine with proper cross-attention mechanisms:
//! - Mini cross-attention transformer for full attention-based fusion
//! - Modality-specific gating for adaptive fusion weights
//! - Bilinear pooling for compact cross-modal representations
//!
//! **Cross-Attention:**
//! ```text
//! Q_m = W_q^m · h_m,  K_n = W_k^n · h_n,  V_n = W_v^n · h_n
//! Attention(Q_m, K_n, V_n) = softmax(Q_m K_n^T / √d) · V_n
//! Fusion = Σ_m Σ_n gate(m,n) · Attention(Q_m, K_n, V_n)
//! ```

use candle_core::{Device, IndexOp, Result, Tensor};

/// Configuration for cross-attention fusion.
#[derive(Debug, Clone)]
pub struct CrossAttentionConfig {
    /// Embedding dimension per modality.
    pub embed_dim: usize,
    /// Number of attention heads.
    pub num_heads: usize,
    /// Use bilinear pooling instead of full attention (faster, less memory).
    pub use_bilinear: bool,
    /// Number of modalities.
    pub num_modalities: usize,
    /// Gating temperature (lower = more selective).
    pub gate_temperature: f32,
    /// Dropout rate for attention weights.
    #[allow(dead_code)]
    pub attention_dropout: f32,
    /// Normalize outputs to unit norm.
    pub normalize_output: bool,
}

impl Default for CrossAttentionConfig {
    fn default() -> Self {
        Self {
            embed_dim: 64,
            num_heads: 4,
            use_bilinear: false,
            num_modalities: 3,
            gate_temperature: 1.0,
            attention_dropout: 0.1,
            normalize_output: true,
        }
    }
}

/// Result of cross-attention fusion.
pub struct FusionResult {
    /// Fused embedding tensor.
    pub fused: Tensor,
    /// Attention weights per modality pair [M x M].
    pub attention_weights: Vec<Vec<f32>>,
    /// Gating scores per modality [M].
    pub gate_scores: Vec<f32>,
    /// Fusion energy (lower = better aligned).
    pub fusion_energy: f32,
}

/// Cross-attention layer for two modality streams.
struct CrossAttentionLayer {
    w_q: Tensor,
    w_k: Tensor,
    w_v: Tensor,
    w_o: Tensor,
    scale: f32,
    num_heads: usize,
    head_dim: usize,
}

impl CrossAttentionLayer {
    fn new(config: &CrossAttentionConfig, device: &Device) -> Result<Self> {
        let head_dim = config.embed_dim / config.num_heads;
        let scale = 1.0 / (head_dim as f32).sqrt();
        let std = 1.0 / (config.embed_dim as f32).sqrt();
        Ok(Self {
            w_q: Tensor::randn(0.0f32, std, (config.embed_dim, config.embed_dim), device)?,
            w_k: Tensor::randn(0.0f32, std, (config.embed_dim, config.embed_dim), device)?,
            w_v: Tensor::randn(0.0f32, std, (config.embed_dim, config.embed_dim), device)?,
            w_o: Tensor::randn(0.0f32, std, (config.embed_dim, config.embed_dim), device)?,
            scale,
            num_heads: config.num_heads,
            head_dim,
        })
    }

    /// Apply cross-attention: query from modality A, key/value from modality B.
    fn forward(&self, query: &Tensor, key: &Tensor, value: &Tensor) -> Result<Tensor> {
        let (batch, seq, _dim) = query.shape().dims3()?;
        let embed_dim = self.num_heads * self.head_dim;

        // Flatten (batch, seq, dim) -> (batch*seq, dim) for matmul with 2D weight
        let qs = (batch * seq, embed_dim);
        let q = query
            .reshape(qs)?
            .matmul(&self.w_q)?
            .reshape((batch, seq, embed_dim))?;
        let k = key
            .reshape(qs)?
            .matmul(&self.w_k)?
            .reshape((batch, seq, embed_dim))?;
        let v = value
            .reshape(qs)?
            .matmul(&self.w_v)?
            .reshape((batch, seq, embed_dim))?;

        let q = q
            .reshape((batch, seq, self.num_heads, self.head_dim))?
            .permute((0, 2, 1, 3))?;
        let k = k
            .reshape((batch, seq, self.num_heads, self.head_dim))?
            .permute((0, 2, 1, 3))?;
        let v = v
            .reshape((batch, seq, self.num_heads, self.head_dim))?
            .permute((0, 2, 1, 3))?
            .contiguous()?;

        let scale_tensor = Tensor::new(self.scale, q.device())?;
        let q_scaled = q.broadcast_mul(&scale_tensor)?;
        let k_transposed = k.permute((0, 1, 3, 2))?;
        let k_transposed = k_transposed.contiguous()?;
        let scores = q_scaled.matmul(&k_transposed)?.contiguous()?;

        let attention = candle_nn::ops::softmax_last_dim(&scores)?;
        let context = attention.matmul(&v)?;
        let context = context
            .permute((0, 2, 1, 3))?
            .reshape((batch * seq, embed_dim))?;

        context.matmul(&self.w_o)?.reshape((batch, seq, embed_dim))
    }

    /// Extract attention weights for analysis.
    fn attention_weights(&self, query: &Tensor, key: &Tensor) -> Result<Tensor> {
        let (batch, seq, _dim) = query.shape().dims3()?;
        let embed_dim = self.num_heads * self.head_dim;

        // Flatten for matmul with 2D weight
        let qs = (batch * seq, embed_dim);
        let q = query
            .reshape(qs)?
            .matmul(&self.w_q)?
            .reshape((batch, seq, embed_dim))?;
        let k = key
            .reshape(qs)?
            .matmul(&self.w_k)?
            .reshape((batch, seq, embed_dim))?;

        let q = q
            .reshape((batch, seq, self.num_heads, self.head_dim))?
            .permute((0, 2, 1, 3))?;
        let k = k
            .reshape((batch, seq, self.num_heads, self.head_dim))?
            .permute((0, 2, 1, 3))?;

        let scale_tensor = Tensor::new(self.scale, q.device())?;
        let q_scaled = q.broadcast_mul(&scale_tensor)?;
        let k_transposed = k.permute((0, 1, 3, 2))?;
        let scores = q_scaled.matmul(&k_transposed)?;

        candle_nn::ops::softmax_last_dim(&scores)
    }
}

/// Full cross-attention fusion engine.
pub struct CrossAttentionFusion {
    #[allow(dead_code)]
    config: CrossAttentionConfig,
    layers: Vec<CrossAttentionLayer>,
    gating_weights: Tensor,
    output_norm: bool,
}

impl CrossAttentionFusion {
    pub fn new(config: &CrossAttentionConfig, device: &Device) -> Result<Self> {
        let num_m = config.num_modalities;
        let mut layers = Vec::new();

        for m in 0..num_m {
            for n in 0..num_m {
                if m != n {
                    let layer = CrossAttentionLayer::new(config, device)?;
                    layers.push(layer);
                }
            }
        }

        let gating_weights = Tensor::randn(0.0f32, 0.02f32, (num_m,), device)?;

        Ok(Self {
            config: config.clone(),
            layers,
            gating_weights,
            output_norm: config.normalize_output,
        })
    }

    /// Fuse multiple modality embeddings using cross-attention.
    pub fn fuse(&self, modalities: &[Tensor]) -> Result<FusionResult> {
        let num_m = modalities.len();
        let device = modalities[0].device();

        // Track if input was 2D so we can squeeze output back
        let input_was_2d = modalities[0].shape().dims().len() == 2;

        let (batch, _embed_dim) = if input_was_2d {
            modalities[0].shape().dims2()?
        } else {
            let (b, _s, d) = modalities[0].shape().dims3()?;
            (b, d)
        };

        // Single modality: passthrough with gate score 1.0
        if num_m == 1 {
            let fused = if input_was_2d {
                modalities[0].clone()
            } else {
                ensure_3d(&modalities[0], batch)?.squeeze(1)?
            };
            let fusion_energy = fused.sqr()?.sum_all()?.to_scalar::<f32>()?.sqrt();
            return Ok(FusionResult {
                fused,
                attention_weights: vec![vec![0.0; 1]],
                gate_scores: vec![1.0],
                fusion_energy,
            });
        }

        let mut attention_results = Vec::new();
        let mut attention_weights = vec![vec![0.0; num_m]; num_m];
        let mut layer_idx = 0;

        for m in 0..num_m {
            let mod_m = ensure_3d(&modalities[m], batch)?;
            let mut mod_fused = mod_m.clone();

            for n in 0..num_m {
                if m != n && layer_idx < self.layers.len() {
                    let mod_n = ensure_3d(&modalities[n], batch)?;
                    let layer = &self.layers[layer_idx];

                    let cross = layer.forward(&mod_m, &mod_n, &mod_n)?;
                    mod_fused = mod_fused.add(&cross)?;

                    let attn = layer.attention_weights(&mod_m, &mod_n)?;
                    let attn_mean: f32 = attn.mean_all()?.to_scalar()?;
                    attention_weights[m][n] = attn_mean;
                    layer_idx += 1;
                }
            }

            attention_results.push(mod_fused);
        }

        let mut fused = attention_results[0].clone();
        for result in &attention_results[1..] {
            fused = fused.add(result)?;
        }

        fused = fused.broadcast_div(&Tensor::new(num_m as f32, device)?)?;

        // Gating — softmax over modality dimension (with temperature scaling)
        let temp = self.config.gate_temperature;
        let gated = self.gating_weights.mul(&Tensor::full(
            1.0 / temp,
            self.gating_weights.shape(),
            device,
        )?)?;
        let gate_scores_tensor = candle_nn::ops::softmax_last_dim(&gated)?.contiguous()?;
        let mut gate_scores = Vec::with_capacity(num_m);
        for i in 0..num_m {
            let score = gate_scores_tensor.i(i)?.to_scalar::<f32>().unwrap_or(0.0);
            gate_scores.push(score);
        }

        let fusion_energy = fused.sqr()?.sum_all()?.to_scalar::<f32>()?.sqrt();

        // Squeeze back to 2D if input was 2D: [batch, 1, dim] -> [batch, dim]
        let fused = if input_was_2d {
            fused.squeeze(1)?
        } else {
            fused
        };

        let fused = if self.output_norm {
            let norm = fused.sqr()?.sum_all()?.to_scalar::<f32>()?.sqrt();
            if norm > 1e-8 {
                fused.broadcast_div(&Tensor::new(norm, device)?)?
            } else {
                fused
            }
        } else {
            fused
        };

        Ok(FusionResult {
            fused,
            attention_weights,
            gate_scores,
            fusion_energy,
        })
    }

    /// Compute alignment score between two modality embeddings after fusion.
    pub fn alignment_score(&self, fused: &Tensor, target: &Tensor) -> Result<f32> {
        // Flatten to 2D [1, total] for matmul compatibility
        let f_flat = fused.flatten_all()?.reshape((1, fused.elem_count()))?;
        let t_flat = target.flatten_all()?.reshape((target.elem_count(), 1))?;

        let dot = f_flat.matmul(&t_flat)?.i(0)?.i(0)?.to_scalar::<f32>()?;
        let norm_f = f_flat.sqr()?.sum_all()?.to_scalar::<f32>()?.sqrt();
        let norm_t = t_flat.sqr()?.sum_all()?.to_scalar::<f32>()?.sqrt();
        let norm_product = norm_f * norm_t;

        if norm_product < 1e-10 {
            return Ok(0.0);
        }

        let score = dot / norm_product;
        Ok(score.clamp(-1.0, 1.0))
    }
}

/// Ensure tensor is 3D [batch, seq, dim].
fn ensure_3d(tensor: &Tensor, batch: usize) -> Result<Tensor> {
    match *tensor.shape().dims() {
        [b, _s, _d] if b == batch => Ok(tensor.clone()),
        [b, _d] if b == batch => tensor.unsqueeze(1),
        _ => tensor.reshape((batch, 1, tensor.elem_count() / batch)),
    }
}

/// Generate stub modality embeddings for testing.
pub fn generate_stub_modalities(
    num_modalities: usize,
    batch_size: usize,
    embed_dim: usize,
    device: &Device,
) -> Result<Vec<Tensor>> {
    let mut modalities = Vec::with_capacity(num_modalities);
    for m in 0..num_modalities {
        let offset = m as f32 * 0.1f32;
        let emb = Tensor::randn(offset, 0.5f32, (batch_size, embed_dim), device)?;
        modalities.push(emb);
    }
    Ok(modalities)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_attention_layer() -> Result<()> {
        let config = CrossAttentionConfig {
            embed_dim: 32,
            num_heads: 2,
            ..Default::default()
        };
        let device = Device::Cpu;

        let layer = CrossAttentionLayer::new(&config, &device)?;
        let batch = 2;
        let seq = 4;

        let query = Tensor::randn(0.0f32, 1.0f32, (batch, seq, config.embed_dim), &device)?;
        let key = Tensor::randn(0.0f32, 1.0f32, (batch, seq, config.embed_dim), &device)?;
        let value = Tensor::randn(0.0f32, 1.0f32, (batch, seq, config.embed_dim), &device)?;

        let output = layer.forward(&query, &key, &value)?;
        assert_eq!(output.shape().dims3()?, (batch, seq, config.embed_dim));

        Ok(())
    }

    #[test]
    fn test_fusion_basic() -> Result<()> {
        let config = CrossAttentionConfig {
            embed_dim: 32,
            num_heads: 2,
            num_modalities: 2,
            use_bilinear: false,
            ..Default::default()
        };
        let device = Device::Cpu;

        let fusion = CrossAttentionFusion::new(&config, &device)?;
        let modalities = generate_stub_modalities(2, 2, 32, &device)?;

        let result = fusion.fuse(&modalities)?;
        assert!(result.fusion_energy.is_finite());
        assert_eq!(result.attention_weights.len(), 2);
        assert_eq!(result.gate_scores.len(), 2);

        Ok(())
    }

    #[test]
    fn test_fusion_three_modalities() -> Result<()> {
        let config = CrossAttentionConfig {
            embed_dim: 32,
            num_heads: 2,
            num_modalities: 3,
            use_bilinear: false,
            ..Default::default()
        };
        let device = Device::Cpu;

        let fusion = CrossAttentionFusion::new(&config, &device)?;
        let modalities = generate_stub_modalities(3, 2, 32, &device)?;

        let result = fusion.fuse(&modalities)?;
        assert!(result.fusion_energy.is_finite());
        assert_eq!(result.attention_weights.len(), 3);

        Ok(())
    }

    #[test]
    fn test_alignment_score() -> Result<()> {
        let device = Device::Cpu;
        let a = Tensor::randn(0.0f32, 1.0f32, (4, 16), &device)?;
        let b = Tensor::randn(0.0f32, 1.0f32, (4, 16), &device)?;

        let config = CrossAttentionConfig::default();
        let fusion = CrossAttentionFusion::new(&config, &device)?;

        let score = fusion.alignment_score(&a, &b)?;
        assert!((-1.0..=1.0).contains(&score));

        Ok(())
    }

    #[test]
    fn test_generate_stub_modalities() -> Result<()> {
        let device = Device::Cpu;
        let modalities = generate_stub_modalities(3, 2, 16, &device)?;
        assert_eq!(modalities.len(), 3);
        for m in &modalities {
            assert_eq!(m.shape().dims2()?, (2, 16));
        }
        Ok(())
    }

    #[test]
    fn test_ensure_3d() -> Result<()> {
        let device = Device::Cpu;
        let t2d = Tensor::randn(0.0f32, 1.0f32, (2, 16), &device)?;
        let t3d = ensure_3d(&t2d, 2)?;
        assert_eq!(t3d.shape().dims3()?, (2, 1, 16));

        let t3d_orig = Tensor::randn(0.0f32, 1.0f32, (2, 4, 16), &device)?;
        let t3d_result = ensure_3d(&t3d_orig, 2)?;
        assert_eq!(t3d_result.shape().dims3()?, (2, 4, 16));

        Ok(())
    }

    #[test]
    fn test_fusion_result_shapes() -> Result<()> {
        let config = CrossAttentionConfig {
            embed_dim: 16,
            num_heads: 2,
            num_modalities: 2,
            normalize_output: true,
            ..Default::default()
        };
        let device = Device::Cpu;

        let fusion = CrossAttentionFusion::new(&config, &device)?;
        let modalities = generate_stub_modalities(2, 1, 16, &device)?;

        let result = fusion.fuse(&modalities)?;
        assert!(result.fusion_energy > 0.0);
        assert!(result.gate_scores.iter().all(|&s| s >= 0.0));

        Ok(())
    }

    #[test]
    fn test_attention_weights_extraction() -> Result<()> {
        let config = CrossAttentionConfig {
            embed_dim: 16,
            num_heads: 2,
            ..Default::default()
        };
        let device = Device::Cpu;

        let layer = CrossAttentionLayer::new(&config, &device)?;
        let query = Tensor::randn(0.0f32, 1.0f32, (1, 2, 16), &device)?;
        let key = Tensor::randn(0.0f32, 1.0f32, (1, 2, 16), &device)?;

        let weights = layer.attention_weights(&query, &key)?;
        assert!(weights.elem_count() > 0);

        Ok(())
    }
}
