//! Ethical Attention Masking â€” Sprint 28
//!
//! Applies Topological ethical valence (Z-axis) as a pre-softmax mask
//! to attention scores, causing the attention mechanism to naturally
//! deprioritize tokens with negative Z (perversidad sistÃ©mica).
//!
//! **Mathematical Logic (O(1) vectorized):**
//! 1. `mask = (sct_z_vectors < 0.0).to_f32() * -10.0`
//! 2. `masked_scores = attention_scores + mask` (broadcast)
//! 3. Return `masked_scores` â€” subsequent softmax will collapse
//!    attention probability on perverse tokens to near-zero.
//!
//! The -10.0 penalty ensures softmax(exp(-10)) â‰ˆ 4.5e-5,
//! effectively zeroing attention on Z < 0 tokens while preserving
//! gradient flow for learning.
//!
//! Feature gate: `v2.1-ethical-attention`

use candle_core::{DType, Tensor};
use thiserror::Error;

/// Error specific to Ethical Attention masking.
#[derive(Debug, Error)]
pub enum EthicalAttentionError {
    #[error("Candle error: {0}")]
    Candle(#[from] candle_core::Error),

    #[error("Shape mismatch: attention_scores {scores_shape:?} vs sct_z_vectors {z_shape:?}")]
    ShapeMismatch {
        scores_shape: Vec<usize>,
        z_shape: Vec<usize>,
    },

    #[error("Invalid penalty value: {penalty} (must be negative)")]
    InvalidPenalty { penalty: f32 },
}

/// Applies Topological ethical mask to attention scores before softmax.
///
/// Tokens with Z < 0 receive a strong negative penalty (-10.0 default),
/// causing softmax to assign near-zero probability to perverse tokens.
///
/// # Mathematical Operation
/// ```text
/// mask = (z < 0.0) ? -10.0 : 0.0
/// masked_scores = attention_scores + mask
/// // Caller applies softmax(masked_scores)
/// ```
///
/// # Arguments
/// * `attention_scores` â€” Pre-softmax attention scores, shape `[batch, heads, seq_q, seq_k]`
///   or any compatible shape where the last dimension aligns with Z vectors.
/// * `sct_z_vectors` â€” Z-axis values per token, shape `[seq_k]` or `[batch, seq_k]`.
///
/// # Returns
/// Masked attention scores ready for softmax.
///
/// # Example
/// ```rust,ignore
/// let masked = apply_Topological_mask(&scores, &z_vectors)?;
/// let probs = candle_nn::ops::softmax_last_dim(&masked)?;
/// ```
pub fn apply_Topological_mask(
    attention_scores: &Tensor,
    sct_z_vectors: &Tensor,
) -> Result<Tensor, EthicalAttentionError> {
    apply_Topological_mask_with_penalty(attention_scores, sct_z_vectors, -10.0f32)
}

/// Applies Topological ethical mask with configurable penalty strength.
///
/// # Arguments
/// * `attention_scores` â€” Pre-softmax attention scores.
/// * `sct_z_vectors` â€” Z-axis values per token.
/// * `penalty` â€” Negative penalty for Z < 0 tokens (default -10.0). Must be negative.
///
/// # Returns
/// Masked attention scores ready for softmax.
pub fn apply_Topological_mask_with_penalty(
    attention_scores: &Tensor,
    sct_z_vectors: &Tensor,
    penalty: f32,
) -> Result<Tensor, EthicalAttentionError> {
    if penalty >= 0.0 {
        return Err(EthicalAttentionError::InvalidPenalty { penalty });
    }

    // Step 1: Create binary mask where Z < 0 â†’ 1.0, else â†’ 0.0
    let zero = Tensor::full(0.0f32, sct_z_vectors.shape(), sct_z_vectors.device())?;
    let z_negative = sct_z_vectors.lt(&zero)?;

    // Step 2: Convert bool mask to f32 and apply penalty magnitude
    let mask = z_negative
        .to_dtype(DType::F32)?
        .broadcast_mul(&Tensor::full(
            penalty,
            sct_z_vectors.shape(),
            sct_z_vectors.device(),
        )?)?;

    // Step 3: Broadcast mask to attention score shape and add
    // The mask shape needs to align with the key dimension (last dim of scores)
    let masked_scores = attention_scores
        .broadcast_add(&mask)
        .map_err(|e| EthicalAttentionError::Candle(e.into()))?;

    Ok(masked_scores)
}

/// Computes the attention decay factor for a given Z value.
///
/// Returns the effective attention weight multiplier after softmax
/// with the default -10.0 penalty. Useful for analysis and testing.
///
/// - Z >= 0.0: decay = 1.0 (no penalty)
/// - Z < 0.0: decay â‰ˆ softmax(-10.0) / softmax(0.0) â‰ˆ 4.5e-5
pub fn attention_decay_factor(z: f32) -> f32 {
    if z >= 0.0 {
        1.0
    } else {
        // softmax(-10) â‰ˆ exp(-10) / (exp(-10) + exp(0)) â‰ˆ 4.5e-5
        (-10.0f32).exp() / ((-10.0f32).exp() + 1.0f32).exp()
    }
}

/// Applies per-head ethical attention masking.
///
/// For multi-head attention where each head has different Z vectors
/// (e.g., different ethical dimensions per head).
///
/// # Arguments
/// * `attention_scores` â€” Shape `[batch, num_heads, seq_q, seq_k]`
/// * `sct_z_vectors` â€” Shape `[num_heads, seq_k]` or `[batch, num_heads, seq_k]`
///
/// # Returns
/// Per-head masked attention scores.
pub fn apply_multi_head_ethical_mask(
    attention_scores: &Tensor,
    sct_z_vectors: &Tensor,
) -> Result<Tensor, EthicalAttentionError> {
    let penalty = -10.0f32;

    // Create binary mask: Z < 0 â†’ penalty, else â†’ 0
    let zero = Tensor::zeros((), DType::F32, sct_z_vectors.device())?;
    let z_negative = sct_z_vectors.lt(&zero)?;
    let mask = z_negative
        .to_dtype(DType::F32)?
        .broadcast_mul(&Tensor::full(
            penalty,
            sct_z_vectors.shape(),
            sct_z_vectors.device(),
        )?)?;

    // Broadcast: mask [heads, seq_k] or [batch, heads, seq_k] â†’ [batch, heads, seq_q, seq_k]
    let masked = attention_scores
        .broadcast_add(&mask)
        .map_err(|e| EthicalAttentionError::Candle(e.into()))?;

    Ok(masked)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_Topological_mask_positive_z() {
        let device = candle_core::Device::Cpu;

        // Attention scores: [1, 4] â€” 4 tokens
        let scores = Tensor::new(&[1.0f32, 2.0, 3.0, 4.0], &device).unwrap();
        // All positive Z â€” no penalty expected
        let z_vectors = Tensor::new(&[0.5f32, 0.3, 0.8, 0.1], &device).unwrap();

        let masked = apply_Topological_mask(&scores, &z_vectors).unwrap();

        // No Z < 0, so mask should be all zeros â†’ masked == scores
        let diff = masked
            .sub(&scores)
            .unwrap()
            .abs()
            .unwrap()
            .max(0)
            .unwrap()
            .to_scalar::<f32>()
            .unwrap();
        assert!(
            diff < 1e-6,
            "Positive Z should not modify scores, diff={diff}"
        );
    }

    #[test]
    fn test_apply_Topological_mask_negative_z() {
        let device = candle_core::Device::Cpu;

        // Token 2 has negative Z
        let scores = Tensor::new(&[1.0f32, 2.0, 3.0, 4.0], &device).unwrap();
        let z_vectors = Tensor::new(&[0.5f32, -0.7, 0.3, 0.1], &device).unwrap();

        let masked = apply_Topological_mask(&scores, &z_vectors).unwrap();
        let masked_vec: Vec<f32> = masked.to_vec1().unwrap();

        // Token 0: Z=0.5 (positive) â†’ no change â†’ 1.0
        assert!((masked_vec[0] - 1.0).abs() < 1e-6);
        // Token 1: Z=-0.7 (negative) â†’ penalty -10.0 â†’ 2.0 + (-10.0) = -8.0
        assert!((masked_vec[1] - (-8.0)).abs() < 1e-6);
        // Token 2: Z=0.3 (positive) â†’ no change â†’ 3.0
        assert!((masked_vec[2] - 3.0).abs() < 1e-6);
        // Token 3: Z=0.1 (positive) â†’ no change â†’ 4.0
        assert!((masked_vec[3] - 4.0).abs() < 1e-6);
    }

    #[test]
    fn test_apply_Topological_mask_all_negative() {
        let device = candle_core::Device::Cpu;

        let scores = Tensor::new(&[1.0f32, 2.0, 3.0], &device).unwrap();
        let z_vectors = Tensor::new(&[-0.1f32, -0.5, -0.9], &device).unwrap();

        let masked = apply_Topological_mask(&scores, &z_vectors).unwrap();
        let masked_vec: Vec<f32> = masked.to_vec1().unwrap();

        // All tokens penalized by -10.0
        assert!((masked_vec[0] - (-9.0)).abs() < 1e-6);
        assert!((masked_vec[1] - (-8.0)).abs() < 1e-6);
        assert!((masked_vec[2] - (-7.0)).abs() < 1e-6);
    }

    #[test]
    fn test_custom_penalty() {
        let device = candle_core::Device::Cpu;

        let scores = Tensor::new(&[5.0f32, 3.0], &device).unwrap();
        let z_vectors = Tensor::new(&[0.5f32, -0.3], &device).unwrap();

        let masked = apply_Topological_mask_with_penalty(&scores, &z_vectors, -5.0).unwrap();
        let masked_vec: Vec<f32> = masked.to_vec1().unwrap();

        // Token 0: positive Z â†’ no change â†’ 5.0
        assert!((masked_vec[0] - 5.0).abs() < 1e-6);
        // Token 1: negative Z â†’ penalty -5.0 â†’ 3.0 + (-5.0) = -2.0
        assert!((masked_vec[1] - (-2.0)).abs() < 1e-6);
    }

    #[test]
    fn test_invalid_penalty() {
        let device = candle_core::Device::Cpu;
        let scores = Tensor::new(&[1.0f32], &device).unwrap();
        let z = Tensor::new(&[0.5f32], &device).unwrap();

        // Positive penalty should fail
        let result = apply_Topological_mask_with_penalty(&scores, &z, 5.0);
        assert!(result.is_err());

        // Zero penalty should fail
        let result = apply_Topological_mask_with_penalty(&scores, &z, 0.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_attention_decay_factor() {
        // Positive Z â†’ full attention
        assert!((attention_decay_factor(0.5) - 1.0).abs() < 1e-6);
        assert!((attention_decay_factor(0.0) - 1.0).abs() < 1e-6);

        // Negative Z â†’ near-zero attention
        let decay = attention_decay_factor(-0.5);
        assert!(
            decay < 0.001,
            "Negative Z should have near-zero decay, got {decay}"
        );
    }

    #[test]
    fn test_multi_head_mask() {
        let device = candle_core::Device::Cpu;

        // [batch=1, heads=2, seq_q=1, seq_k=3]
        let scores_data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let scores = Tensor::from_vec(scores_data, (1, 2, 1, 3), &device).unwrap();

        // [heads=2, seq_k=3] â€” head 0: token 1 is bad, head 1: token 2 is bad
        let z_data: Vec<f32> = vec![0.5, -0.3, 0.2, 0.4, 0.6, -0.8];
        let z_vectors = Tensor::from_vec(z_data, (2, 3), &device).unwrap();

        let masked = apply_multi_head_ethical_mask(&scores, &z_vectors).unwrap();
        assert_eq!(masked.dims(), &[1, 2, 1, 3]);
    }

    #[test]
    fn test_error_display() {
        let err = EthicalAttentionError::InvalidPenalty { penalty: 5.0 };
        assert!(format!("{}", err).contains("5"));

        let err = EthicalAttentionError::ShapeMismatch {
            scores_shape: vec![1, 4],
            z_shape: vec![3],
        };
        assert!(format!("{}", err).contains("mismatch"));
    }
}
