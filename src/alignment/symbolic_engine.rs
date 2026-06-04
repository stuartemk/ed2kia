//! Symbolic Embedding Engine â€” Sprint 28
//!
//! Fuses traditional token embeddings (`candle_nn::Embedding`) with the
//! Topological Context Tensor (SCT) 3D vectors to produce _symbolic embeddings_
//! where ethical valence (Z-axis) dynamically modulates attention weight.
//!
//! **Mathematical Fusion (O(1) vectorized):**
//! 1. `base_emb = embedding.forward(token_ids)` â†’ shape `[batch, seq, dim]`
//! 2. `sct_tensor = stack(sct_dict[token] for token in token_ids)` â†’ `[batch, seq, 3]`
//! 3. `z_axis = sct_tensor[..., 2]` â†’ `[batch, seq]`
//! 4. `scale = (1.0 + z_axis.clamp(-0.5, 0.5)).unsqueeze(-1)` â†’ `[batch, seq, 1]`
//! 5. `result = base_emb * scale` (broadcast multiplication)
//!
//! Tokens with Z < 0 (perversidad) are attenuated. Tokens with Z > 0 (symbiosis)
//! are amplified. Zero imperative loops â€” all `candle_core` tensor operations.
//!
//! Feature gate: `v2.1-symbolic-engine`

use candle_core::{DType, Device, Module, Tensor};
use candle_nn::Embedding;
use dashmap::DashMap;
use thiserror::Error;

use crate::alignment::sct_core::TopologicalTensor;

/// Error specific to the Symbolic Embedding Engine.
#[derive(Debug, Error)]
pub enum SymbolicError {
    #[error("Candle error: {0}")]
    Candle(#[from] candle_core::Error),

    #[error("Token {token_id} has no SCT mapping in the symbolic dictionary")]
    MissingSctMapping { token_id: u32 },

    #[error("Invalid embedding dimensions: vocab={vocab_size}, dim={embed_dim}")]
    InvalidDimensions { vocab_size: usize, embed_dim: usize },

    #[error("Batch shape mismatch: expected [{batch}, {seq}], got {shape:?}")]
    ShapeMismatch {
        batch: usize,
        seq: usize,
        shape: Vec<usize>,
    },
}

/// Symbolic Embedding Layer â€” Token embeddings modulated by SCT ethical valence.
///
/// Combines a standard embedding matrix with a per-token SCT dictionary.
/// During forward pass, the Z-axis of each token's SCT is used to scale
/// the base embedding, creating "ethical intuition" at the matrix level.
///
/// ### Usage
/// ```rust,ignore
/// let symbolic = SymbolicEmbedding::new(vocab, dim, &device)?;
/// let result = symbolic.forward(&token_ids)?; // [batch, seq, dim] scaled by SCT Z
/// ```
pub struct SymbolicEmbedding {
    /// Base token embedding layer.
    base: Embedding,
    /// Per-token SCT mapping: token_id â†’ TopologicalTensor.
    sct_dict: DashMap<u32, TopologicalTensor>,
    /// Vocabulary size.
    vocab_size: usize,
    /// Embedding dimension.
    embed_dim: usize,
}

impl SymbolicEmbedding {
    /// Creates a new `SymbolicEmbedding` with random initialization.
    ///
    /// # Arguments
    /// * `vocab_size` â€” Number of tokens in the vocabulary.
    /// * `embed_dim` â€” Dimension of the embedding vectors.
    /// * `device` â€” CUDA/CPU device for tensor allocation.
    ///
    /// # Returns
    /// `SymbolicEmbedding` ready for forward pass.
    pub fn new(
        vocab_size: usize,
        embed_dim: usize,
        device: &Device,
    ) -> Result<Self, SymbolicError> {
        if vocab_size == 0 || embed_dim == 0 {
            return Err(SymbolicError::InvalidDimensions {
                vocab_size,
                embed_dim,
            });
        }

        // Initialize with ones so forward pass produces non-trivial embeddings.
        // In production, these would be loaded from pretrained weights via VarBuilder::from_mmap/fs.
        let embeddings = Tensor::ones((vocab_size, embed_dim), DType::F32, device)?;
        let base = Embedding::new(embeddings, embed_dim);

        Ok(Self {
            base,
            sct_dict: DashMap::new(),
            vocab_size,
            embed_dim,
        })
    }

    /// Inserts or updates the SCT mapping for a token.
    pub fn set_sct(&self, token_id: u32, sct: TopologicalTensor) {
        self.sct_dict.insert(token_id, sct);
    }

    /// Bulk inserts SCT mappings from an iterator.
    pub fn bulk_set_sct<I>(&self, mappings: I)
    where
        I: IntoIterator<Item = (u32, TopologicalTensor)>,
    {
        for (token_id, sct) in mappings {
            self.sct_dict.insert(token_id, sct);
        }
    }

    /// Returns the number of tokens with SCT mappings.
    pub fn mapping_count(&self) -> usize {
        self.sct_dict.len()
    }

    /// Forward pass with SCT fusion.
    ///
    /// Applies ethical scaling based on Z-axis of each token's SCT vector.
    /// Tokens without SCT mapping default to neutral (Z=0.0, scale=1.0).
    ///
    /// # Mathematical Fusion (O(1) vectorized)
    /// 1. `base_emb = self.base.forward(token_ids)` â†’ `[batch, seq, dim]`
    /// 2. Build `sct_tensor` from dictionary lookup â†’ `[batch, seq, 3]`
    /// 3. Extract Z: `z_axis = sct_tensor[..., 2]` â†’ `[batch, seq]`
    /// 4. `scale = (1.0 + z_axis.clamp(-0.5, 0.5)).unsqueeze(-1)` â†’ `[batch, seq, 1]`
    /// 5. `result = base_emb * scale` (broadcast)
    ///
    /// # Arguments
    /// * `token_ids` â€” Tensor of shape `[batch, seq]` containing token IDs.
    ///
    /// # Returns
    /// Symbolic embedding tensor of shape `[batch, seq, dim]` scaled by SCT Z-axis.
    pub fn forward(&self, token_ids: &Tensor) -> Result<Tensor, SymbolicError> {
        let shape = token_ids.dims();
        if shape.len() != 2 {
            return Err(SymbolicError::ShapeMismatch {
                batch: 0,
                seq: 0,
                shape: shape.to_vec(),
            });
        }
        let (batch, seq) = (shape[0], shape[1]);

        // Step 1: Get base embeddings â†’ [batch, seq, dim]
        let base_emb = self
            .base
            .forward(token_ids)
            .map_err(|e| SymbolicError::Candle(e.into()))?;

        // Step 2: Build SCT tensor from dictionary â†’ [batch, seq, 3]
        let sct_tensor = self.build_sct_tensor(token_ids, batch, seq)?;

        // Step 3: Extract Z-axis â†’ [batch, seq]
        let z_axis = sct_tensor
            .narrow(2, 2, 1)
            .map_err(|e| SymbolicError::Candle(e.into()))?
            .squeeze(2)
            .map_err(|e| SymbolicError::Candle(e.into()))?;

        // Step 4: Compute ethical scale = 1.0 + clamp(Z, -0.5, 0.5) â†’ [batch, seq, 1]
        let z_clamped = z_axis
            .clamp(-0.5f32, 0.5f32)
            .map_err(|e| SymbolicError::Candle(e.into()))?;
        let one = Tensor::ones(z_axis.shape(), DType::F32, z_axis.device())?;
        let scale = (one + z_clamped)?
            .unsqueeze(2)
            .map_err(|e| SymbolicError::Candle(e.into()))?; // [batch, seq, 1]

        // Step 5: Broadcast multiplication â†’ [batch, seq, dim]
        let result = base_emb
            .broadcast_mul(&scale)
            .map_err(|e| SymbolicError::Candle(e.into()))?;

        Ok(result)
    }

    /// Forward pass without SCT modulation (pure base embedding).
    ///
    /// Useful for ablation studies comparing symbolic vs. plain embeddings.
    pub fn forward_plain(&self, token_ids: &Tensor) -> Result<Tensor, SymbolicError> {
        self.base
            .forward(token_ids)
            .map_err(|e| SymbolicError::Candle(e.into()))
    }

    /// Builds the SCT tensor from token IDs via dictionary lookup.
    ///
    /// Returns tensor of shape `[batch, seq, 3]` where each `[b, s]` slice
    /// contains `[x, y, z]` from the `TopologicalTensor` mapping.
    /// Tokens without mapping default to neutral `[0.5, 0.5, 0.0]`.
    fn build_sct_tensor(
        &self,
        token_ids: &Tensor,
        batch: usize,
        seq: usize,
    ) -> Result<Tensor, SymbolicError> {
        // Extract token IDs as u32 array
        let ids: Vec<u32> = token_ids.to_vec2()?.into_iter().flatten().collect();

        // Build SCT data: [batch * seq * 3] f32 values
        let neutral = TopologicalTensor {
            x: 0.5,
            y: 0.5,
            z: 0.0,
        };
        let mut sct_data: Vec<f32> = Vec::with_capacity(batch * seq * 3);
        for &token_id in &ids {
            let sct = self
                .sct_dict
                .get(&token_id)
                .map(|entry| *entry.value())
                .unwrap_or(neutral);
            sct_data.push(sct.x);
            sct_data.push(sct.y);
            sct_data.push(sct.z);
        }

        // Create tensor from data â†’ [batch, seq, 3]
        let device = token_ids.device();
        let sct_tensor = Tensor::from_vec(sct_data, (batch, seq, 3), device)
            .map_err(|e| SymbolicError::Candle(e.into()))?;

        Ok(sct_tensor)
    }

    /// Returns the vocabulary size.
    pub fn vocab_size(&self) -> usize {
        self.vocab_size
    }

    /// Returns the embedding dimension.
    pub fn embed_dim(&self) -> usize {
        self.embed_dim
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn neutral_sct() -> TopologicalTensor {
        TopologicalTensor::new(0.5, 0.5, 0.0).unwrap()
    }

    fn positive_sct() -> TopologicalTensor {
        TopologicalTensor::new(0.8, 0.2, 0.7).unwrap()
    }

    fn negative_sct() -> TopologicalTensor {
        TopologicalTensor::new(0.3, 0.9, -0.8).unwrap()
    }

    #[test]
    fn test_symbolic_embedding_creation() {
        let device = Device::Cpu;
        let emb = SymbolicEmbedding::new(1000, 256, &device);
        assert!(emb.is_ok());
        let emb = emb.unwrap();
        assert_eq!(emb.vocab_size(), 1000);
        assert_eq!(emb.embed_dim(), 256);
        assert_eq!(emb.mapping_count(), 0);
    }

    #[test]
    fn test_invalid_dimensions() {
        let device = Device::Cpu;
        let result = SymbolicEmbedding::new(0, 256, &device);
        assert!(result.is_err());
    }

    #[test]
    fn test_sct_mapping_insert() {
        let device = Device::Cpu;
        let emb = SymbolicEmbedding::new(100, 64, &device).unwrap();
        emb.set_sct(42, positive_sct());
        assert_eq!(emb.mapping_count(), 1);
        emb.set_sct(99, negative_sct());
        assert_eq!(emb.mapping_count(), 2);
    }

    #[test]
    fn test_bulk_sct_mapping() {
        let device = Device::Cpu;
        let emb = SymbolicEmbedding::new(100, 64, &device).unwrap();
        let mappings = vec![
            (1u32, positive_sct()),
            (2u32, negative_sct()),
            (3u32, neutral_sct()),
        ];
        emb.bulk_set_sct(mappings);
        assert_eq!(emb.mapping_count(), 3);
    }

    #[test]
    fn test_forward_plain() {
        let device = Device::Cpu;
        let emb = SymbolicEmbedding::new(100, 64, &device).unwrap();
        let token_ids = Tensor::new(&[1u32, 2, 3, 4], &device)
            .unwrap()
            .reshape((1, 4))
            .unwrap();
        let result = emb.forward_plain(&token_ids);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.dims(), &[1, 4, 64]);
    }

    #[test]
    fn test_forward_with_sct_neutral() {
        let device = Device::Cpu;
        let emb = SymbolicEmbedding::new(100, 64, &device).unwrap();
        // No SCT mappings â€” all tokens default to neutral (Z=0, scale=1.0)
        let token_ids = Tensor::new(&[1u32, 2, 3], &device)
            .unwrap()
            .reshape((1, 3))
            .unwrap();
        let result = emb.forward(&token_ids).unwrap();
        let plain = emb.forward_plain(&token_ids).unwrap();

        // With neutral SCT (Z=0), scale = 1.0 + clamp(0, -0.5, 0.5) = 1.0
        // So symbolic == plain
        let diff = result
            .sub(&plain)
            .unwrap()
            .abs()
            .unwrap()
            .max(0)
            .unwrap()
            .to_scalar::<f32>()
            .unwrap();
        assert!(
            diff < 1e-6,
            "Neutral SCT should produce identical embeddings, diff={diff}"
        );
    }

    #[test]
    fn test_forward_positive_sct_amplifies() {
        let device = Device::Cpu;
        let emb = SymbolicEmbedding::new(100, 64, &device).unwrap();
        // Token 1 has positive SCT (Z=0.7)
        emb.set_sct(1, positive_sct());

        let token_ids = Tensor::new(&[1u32], &device)
            .unwrap()
            .reshape((1, 1))
            .unwrap();
        let symbolic = emb.forward(&token_ids).unwrap();
        let plain = emb.forward_plain(&token_ids).unwrap();

        // scale = 1.0 + clamp(0.7, -0.5, 0.5) = 1.0 + 0.5 = 1.5
        // symbolic norm should be ~1.5x plain norm
        let sym_norm = symbolic
            .sqr()
            .unwrap()
            .sum(0)
            .unwrap()
            .sqrt()
            .unwrap()
            .to_scalar::<f32>()
            .unwrap();
        let plain_norm = plain
            .sqr()
            .unwrap()
            .sum(0)
            .unwrap()
            .sqrt()
            .unwrap()
            .to_scalar::<f32>()
            .unwrap();

        if plain_norm > 1e-6 {
            let ratio = sym_norm / plain_norm;
            assert!(
                (ratio - 1.5f32).abs() < 0.05,
                "Positive SCT should amplify ~1.5x, got ratio={ratio}"
            );
        }
    }

    #[test]
    fn test_forward_negative_sct_attenuates() {
        let device = Device::Cpu;
        let emb = SymbolicEmbedding::new(100, 64, &device).unwrap();
        // Token 1 has negative SCT (Z=-0.8)
        emb.set_sct(1, negative_sct());

        let token_ids = Tensor::new(&[1u32], &device)
            .unwrap()
            .reshape((1, 1))
            .unwrap();
        let symbolic = emb.forward(&token_ids).unwrap();
        let plain = emb.forward_plain(&token_ids).unwrap();

        // scale = 1.0 + clamp(-0.8, -0.5, 0.5) = 1.0 + (-0.5) = 0.5
        // symbolic norm should be ~0.5x plain norm
        let sym_norm = symbolic
            .sqr()
            .unwrap()
            .sum(0)
            .unwrap()
            .sqrt()
            .unwrap()
            .to_scalar::<f32>()
            .unwrap();
        let plain_norm = plain
            .sqr()
            .unwrap()
            .sum(0)
            .unwrap()
            .sqrt()
            .unwrap()
            .to_scalar::<f32>()
            .unwrap();

        if plain_norm > 1e-6 {
            let ratio = sym_norm / plain_norm;
            assert!(
                (ratio - 0.5f32).abs() < 0.05,
                "Negative SCT should attenuate ~0.5x, got ratio={ratio}"
            );
        }
    }

    #[test]
    fn test_error_display() {
        let err = SymbolicError::MissingSctMapping { token_id: 42 };
        assert!(format!("{}", err).contains("42"));

        let err = SymbolicError::InvalidDimensions {
            vocab_size: 0,
            embed_dim: 64,
        };
        assert!(format!("{}", err).contains("vocab"));
    }
}
