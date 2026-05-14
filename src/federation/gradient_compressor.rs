//! Gradient compression for federated learning.
//!
//! Provides two compression strategies to reduce bandwidth in federated
//! gradient aggregation:
//!
//! 1. **Top-K Sparsity** – Keep only the K largest-magnitude gradient
//!    components and zero out the rest. Stores indices + values separately.
//!
//! 2. **int8 Quantization** – Map f32 gradients to 8-bit integers using a
//!    global scale factor, reducing payload size by 4x.
//!
//! Both strategies can be combined in a single pipeline
//! (`compress_and_quantize`) for maximum compression.
//!
//! # Feature Flag
//!
//! This module is gated behind `#[cfg(feature = "v1.1-sprint1")]`.

#[cfg(feature = "v1.1-sprint1")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "v1.1-sprint1")]
use tracing::debug;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Compressed gradient representation supporting Top-K sparsity + int8 quantization.
#[cfg(feature = "v1.1-sprint1")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedGradient {
    /// Quantized gradient values (int8 after full pipeline, f32 for Top-K only).
    pub data: Vec<i8>,
    /// Indices of the top-K elements in the original vector.
    pub indices: Vec<usize>,
    /// Scale factor for dequantization (max_abs / 127).
    pub scale: f32,
    /// Original dimension before compression.
    pub original_dim: usize,
    /// Ratio of compressed elements to original dimension (lower = better).
    pub compression_ratio: f32,
}

#[cfg(feature = "v1.1-sprint1")]
impl CompressedGradient {
    /// Create a new compressed gradient.
    pub fn new(
        data: Vec<i8>,
        indices: Vec<usize>,
        scale: f32,
        original_dim: usize,
        compression_ratio: f32,
    ) -> Self {
        Self {
            data,
            indices,
            scale,
            original_dim,
            compression_ratio,
        }
    }

    /// Estimate the serialized size in bytes.
    pub fn estimated_size_bytes(&self) -> usize {
        self.data.len() + self.indices.len() * std::mem::size_of::<usize>()
            + std::mem::size_of::<f32>() * 2
            + std::mem::size_of::<usize>()
    }
}

// ---------------------------------------------------------------------------
// GradientCompressor
// ---------------------------------------------------------------------------

/// Gradient compressor supporting Top-K sparsity and int8 quantization.
///
/// # Example
///
/// ```ignore
/// use ed2kia::federation_v2::gradient_compressor::GradientCompressor;
///
/// let compressor = GradientCompressor;
/// let deltas = vec![0.1, -0.5, 0.3, 0.0, 0.9];
///
/// // Top-K sparsity (keep top 3)
/// let (values, indices) = compressor.compress_top_k(&deltas, 3);
/// let reconstructed = compressor.decompress_top_k(&values, &indices, deltas.len());
///
/// // int8 quantization
/// let (quantized, scale) = compressor.quantize_int8(&deltas);
/// let dequantized = compressor.dequantize_int8(&quantized, scale);
///
/// // Combined pipeline
/// let compressed = compressor.compress_and_quantize(&deltas, 3);
/// ```
#[cfg(feature = "v1.1-sprint1")]
pub struct GradientCompressor;

#[cfg(feature = "v1.1-sprint1")]
impl GradientCompressor {
    /// Create a new compressor instance.
    pub const fn new() -> Self {
        GradientCompressor
    }

    // ------------------------------------------------------------------
    // Top-K Sparsity
    // ------------------------------------------------------------------

    /// Compress gradient deltas using Top-K sparsity.
    ///
    /// Selects the `k` elements with the largest absolute values and returns
    /// their values alongside their original indices. If `k >= deltas.len()`,
    /// all elements are returned unchanged.
    ///
    /// # Arguments
    ///
    /// * `deltas` – The full gradient delta vector.
    /// * `k` – Number of top elements to retain.
    ///
    /// # Returns
    ///
    /// A tuple of `(compressed_values, original_indices)`.
    pub fn compress_top_k(deltas: &[f32], k: usize) -> (Vec<f32>, Vec<usize>) {
        if k >= deltas.len() || k == 0 {
            return (deltas.to_vec(), (0..deltas.len()).collect());
        }

        let mut indexed: Vec<(usize, f32)> = deltas
            .iter()
            .enumerate()
            .map(|(i, &v)| (i, v.abs()))
            .collect();

        // Sort by absolute value descending to find top-K.
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let top_k: Vec<(usize, f32)> = indexed[..k].to_vec();
        let mut top_k_sorted = top_k;
        top_k_sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut values = Vec::with_capacity(k);
        let mut indices = Vec::with_capacity(k);

        for &(idx, _) in &top_k_sorted {
            values.push(deltas[idx]);
            indices.push(idx);
        }

        debug!(
            "Top-K compression: {} -> {} elements (ratio={:.2})",
            deltas.len(),
            k,
            k as f32 / deltas.len() as f32
        );

        (values, indices)
    }

    /// Decompress a Top-K compressed gradient back to the original dimension.
    ///
    /// Reconstructs the full vector by placing compressed values at their
    /// original indices and filling the rest with zeros.
    ///
    /// # Arguments
    ///
    /// * `compressed` – The compressed gradient values.
    /// * `indices` – The original indices of each compressed value.
    /// * `original_dim` – The dimension of the original gradient vector.
    ///
    /// # Returns
    ///
    /// The reconstructed gradient vector of length `original_dim`.
    pub fn decompress_top_k(compressed: &[f32], indices: &[usize], original_dim: usize) -> Vec<f32> {
        let mut reconstructed = vec![0.0f32; original_dim];

        for (&idx, &value) in indices.iter().zip(compressed.iter()) {
            if idx < original_dim {
                reconstructed[idx] = value;
            }
        }

        debug!(
            "Top-K decompression: {} elements -> {} dim",
            compressed.len(),
            original_dim
        );

        reconstructed
    }

    // ------------------------------------------------------------------
    // int8 Quantization
    // ------------------------------------------------------------------

    /// Quantize f32 gradient deltas to int8 using uniform quantization.
    ///
    /// The scale factor is computed as `max(abs(values)) / 127`, and each
    /// value is quantized as `round(value / scale)`. If all values are zero,
    /// a default scale of 1.0 is used.
    ///
    /// # Arguments
    ///
    /// * `deltas` – The gradient delta vector to quantize.
    ///
    /// # Returns
    ///
    /// A tuple of `(quantized_values, scale_factor)`.
    pub fn quantize_int8(deltas: &[f32]) -> (Vec<i8>, f32) {
        let max_abs = deltas
            .iter()
            .fold(0.0f32, |max, &v| v.abs().max(max));

        let scale = if max_abs > 0.0 { max_abs / 127.0 } else { 1.0 };

        let quantized: Vec<i8> = deltas
            .iter()
            .map(|&v| (v / scale).round().clamp(-127.0, 127.0) as i8)
            .collect();

        debug!(
            "int8 quantization: {} elements, scale={:.6}",
            deltas.len(),
            scale
        );

        (quantized, scale)
    }

    /// Dequantize int8 values back to f32 using the original scale factor.
    ///
    /// # Arguments
    ///
    /// * `quantized` – The int8 quantized values.
    /// * `scale` – The scale factor used during quantization.
    ///
    /// # Returns
    ///
    /// The reconstructed f32 gradient vector.
    pub fn dequantize_int8(quantized: &[i8], scale: f32) -> Vec<f32> {
        quantized
            .iter()
            .map(|&v| v as f32 * scale)
            .collect()
    }

    // ------------------------------------------------------------------
    // Combined Pipeline
    // ------------------------------------------------------------------

    /// Apply Top-K sparsity followed by int8 quantization.
    ///
    /// This two-stage pipeline first selects the top-K largest-magnitude
    /// elements, then quantizes them to int8. The resulting
    /// `CompressedGradient` can be decompressed using the indices, data,
    /// and scale fields.
    ///
    /// # Arguments
    ///
    /// * `deltas` – The full gradient delta vector.
    /// * `k` – Number of top elements to retain before quantization.
    ///
    /// # Returns
    ///
    /// A `CompressedGradient` containing the quantized data, indices,
    /// scale factor, original dimension, and compression ratio.
    pub fn compress_and_quantize(deltas: &[f32], k: usize) -> CompressedGradient {
        let original_dim = deltas.len();

        // Step 1: Top-K sparsity
        let (top_k_values, indices) = Self::compress_top_k(deltas, k);

        // Step 2: int8 quantization on selected values
        let (quantized_data, scale) = Self::quantize_int8(&top_k_values);

        let compression_ratio = quantized_data.len() as f32 / original_dim.max(1) as f32;

        debug!(
            "Combined compression: {} -> {} int8 elements (ratio={:.2}, scale={:.6})",
            original_dim,
            quantized_data.len(),
            compression_ratio,
            scale
        );

        CompressedGradient::new(quantized_data, indices, scale, original_dim, compression_ratio)
    }

    /// Fully decompress a `CompressedGradient` back to f32.
    ///
    /// Reverses the combined pipeline: dequantizes int8 to f32, then
    /// reconstructs the full vector using stored indices.
    ///
    /// # Arguments
    ///
    /// * `compressed` – The compressed gradient to decompress.
    ///
    /// # Returns
    ///
    /// The reconstructed f32 gradient vector of the original dimension.
    pub fn decompress_full(compressed: &CompressedGradient) -> Vec<f32> {
        // Step 1: Dequantize int8 -> f32
        let values = Self::dequantize_int8(&compressed.data, compressed.scale);

        // Step 2: Reconstruct full vector from indices
        Self::decompress_top_k(&values, &compressed.indices, compressed.original_dim)
    }
}

#[cfg(feature = "v1.1-sprint1")]
impl Default for GradientCompressor {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_deltas(seed: u32, dim: usize) -> Vec<f32> {
        (0..dim)
            .map(|i| ((i + seed as usize) % 100) as f32 / 50.0 - 1.0)
            .collect()
    }

    // ------------------------------------------------------------------
    // Top-K Sparsity Tests
    // ------------------------------------------------------------------

    #[test]
    fn test_compress_top_k_selects_largest() {
        let deltas = vec![0.1, -0.9, 0.3, 0.0, 0.7];
        let (values, indices) = GradientCompressor::compress_top_k(&deltas, 3);

        assert_eq!(values.len(), 3);
        assert_eq!(indices.len(), 3);

        // Top 3 by abs: -0.9 (idx 1), 0.7 (idx 4), 0.3 (idx 2)
        let abs_values: Vec<f32> = values.iter().map(|v| v.abs()).collect();
        assert!((abs_values[0] - 0.3).abs() < 1e-5);
        assert!((abs_values[1] - 0.7).abs() < 1e-5);
        assert!((abs_values[2] - 0.9).abs() < 1e-5);
    }

    #[test]
    fn test_compress_top_k_full_k() {
        let deltas = vec![0.1, -0.5, 0.3];
        let (values, indices) = GradientCompressor::compress_top_k(&deltas, 10);

        assert_eq!(values.len(), 3);
        assert_eq!(indices.len(), 3);
    }

    #[test]
    fn test_compress_top_k_zero_k() {
        let deltas = vec![0.1, -0.5, 0.3];
        let (values, indices) = GradientCompressor::compress_top_k(&deltas, 0);

        assert_eq!(values.len(), 3);
        assert_eq!(indices.len(), 3);
    }

    #[test]
    fn test_decompress_top_k_round_trip() {
        let deltas = make_deltas(42, 100);
        let k = 20;

        let (values, indices) = GradientCompressor::compress_top_k(&deltas, k);
        let reconstructed =
            GradientCompressor::decompress_top_k(&values, &indices, deltas.len());

        assert_eq!(reconstructed.len(), deltas.len());

        // Check that the selected indices match
        let mut error_count = 0;
        for (&idx, _) in indices.iter().zip(values.iter()) {
            if (reconstructed[idx] - deltas[idx]).abs() > 1e-5 {
                error_count += 1;
            }
        }
        assert_eq!(error_count, 0, "Selected indices should match original values");
    }

    // ------------------------------------------------------------------
    // int8 Quantization Tests
    // ------------------------------------------------------------------

    #[test]
    fn test_quantize_int8_scale() {
        let deltas = vec![0.0, 0.5, -1.0, 0.25];
        let (quantized, scale) = GradientCompressor::quantize_int8(&deltas);

        assert_eq!(quantized.len(), deltas.len());
        // max_abs = 1.0, scale = 1.0 / 127
        assert!((scale - 1.0 / 127.0).abs() < 1e-5);
    }

    #[test]
    fn test_quantize_int8_all_zeros() {
        let deltas = vec![0.0; 10];
        let (quantized, scale) = GradientCompressor::quantize_int8(&deltas);

        assert_eq!(quantized.len(), 10);
        assert_eq!(scale, 1.0);
        assert!(quantized.iter().all(|&v| v == 0));
    }

    #[test]
    fn test_dequantize_int8_round_trip() {
        let deltas = make_deltas(42, 100);
        let (quantized, scale) = GradientCompressor::quantize_int8(&deltas);
        let reconstructed = GradientCompressor::dequantize_int8(&quantized, scale);

        assert_eq!(reconstructed.len(), deltas.len());

        // int8 quantization has inherent error for small values relative to max.
        // Check mean absolute relative error < 2% across significant values.
        let mut total_rel_err = 0.0f32;
        let mut count = 0;
        for (orig, recon) in deltas.iter().zip(reconstructed.iter()) {
            if orig.abs() > 0.01 {
                // Only check values that are significant enough for int8 to represent well
                let rel_err = (orig - recon).abs() / orig.abs();
                total_rel_err += rel_err;
                count += 1;
            }
        }
        let mean_rel_err = if count > 0 { total_rel_err / count as f32 } else { 0.0 };
        assert!(
            mean_rel_err < 0.02,
            "Mean relative error {:.4} should be < 2%",
            mean_rel_err
        );
    }

    // ------------------------------------------------------------------
    // Combined Pipeline Tests
    // ------------------------------------------------------------------

    #[test]
    fn test_compress_and_quantize_pipeline() {
        let deltas = make_deltas(42, 200);
        let k = 50;

        let compressed = GradientCompressor::compress_and_quantize(&deltas, k);

        assert_eq!(compressed.original_dim, 200);
        assert_eq!(compressed.data.len(), k);
        assert_eq!(compressed.indices.len(), k);
        assert!((compressed.compression_ratio - k as f32 / 200.0).abs() < 1e-5);
    }

    #[test]
    fn test_decompress_full_round_trip_accuracy() {
        let deltas = make_deltas(42, 100);
        let k = 30;

        let compressed = GradientCompressor::compress_and_quantize(&deltas, k);
        let reconstructed = GradientCompressor::decompress_full(&compressed);

        assert_eq!(reconstructed.len(), deltas.len());

        // Check that selected indices have low relative error (< 1%)
        let mut max_relative_error = 0.0f32;
        for &idx in &compressed.indices {
            let orig = deltas[idx];
            let recon = reconstructed[idx];
            if orig.abs() > 1e-6 {
                let rel_err = (orig - recon).abs() / orig.abs();
                max_relative_error = max_relative_error.max(rel_err);
            }
        }
        assert!(
            max_relative_error < 0.01,
            "Max relative error {:.4} should be < 1%",
            max_relative_error
        );
    }

    #[test]
    fn test_compression_ratio() {
        let deltas = make_deltas(1, 1000);
        let k = 100;

        let compressed = GradientCompressor::compress_and_quantize(&deltas, k);
        assert!((compressed.compression_ratio - 0.1).abs() < 1e-5);
    }

    #[test]
    fn test_estimated_size_bytes() {
        let deltas = make_deltas(1, 100);
        let k = 25;

        let compressed = GradientCompressor::compress_and_quantize(&deltas, k);
        let size = compressed.estimated_size_bytes();

        // data: 25 bytes (i8), indices: 25 * 8 bytes (usize), scale + ratio + dim: 3 * 4 bytes
        let expected_min = 25 + 25 * std::mem::size_of::<usize>() + 12;
        assert!(size >= expected_min);
    }
}
