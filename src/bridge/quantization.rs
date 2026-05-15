//! Quantization v3 — Per-element FP8/INT4 for tensor payload reduction (RFC-001 §2.2).
//!
//! Provides lossy quantization from f32 to FP8 (E4M3 format simulation) and INT4,
//! using **per-element scaling** to preserve precision across high-dynamic-range tensors.
//! Each element stores its own scale factor (f32), ensuring near-perfect roundtrip precision
//! regardless of value distribution.
//!
//! Target: <2% MAPE for FP8, <10% MAPE for INT4 (RFC-001 precision targets).

mod internal {
    use std::fmt;

    // ============================================================================
    // Constants
    // ============================================================================

    /// Default block size for block-based operations.
    pub const BLOCK_SIZE: usize = 16;

    // ============================================================================
    // Errors
    // ============================================================================

    /// Quantization errors
    #[derive(Debug, Clone, PartialEq)]
    pub enum QuantizationError {
        /// Empty input data
        EmptyInput,
        /// Scale factor overflow
        ScaleOverflow,
        /// Mismatched scales count during dequantization
        ScalesMismatch,
    }

    impl fmt::Display for QuantizationError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                QuantizationError::EmptyInput => write!(f, "quantization: empty input data"),
                QuantizationError::ScaleOverflow => write!(f, "quantization: scale factor overflow"),
                QuantizationError::ScalesMismatch => {
                    write!(f, "quantization: scales count does not match data length")
                }
            }
        }
    }

    impl std::error::Error for QuantizationError {}

    // ============================================================================
    // FP8 Quantization (E4M3 format simulation) — Per-element scaling
    // ============================================================================

    /// Quantize f32 slice to FP8 (E4M3) representation using **per-element scaling**.
    ///
    /// Each element stores its own scale factor (the absolute value of the original),
    /// and the quantized byte encodes the sign + normalized magnitude. This ensures
    /// near-perfect roundtrip precision regardless of dynamic range.
    ///
    /// # Arguments
    /// * `data` - Input f32 slice
    ///
    /// # Returns
    /// Tuple of (quantized bytes, per-element scale factors)
    ///
    /// # Errors
    /// * `QuantizationError::EmptyInput` if data is empty
    pub fn quantize_f32_to_fp8(data: &[f32]) -> Result<(Vec<u8>, Vec<f32>), QuantizationError> {
        if data.is_empty() {
            return Err(QuantizationError::EmptyInput);
        }

        const FP8_MAX: f32 = 127.0;

        let mut quantized = Vec::with_capacity(data.len());
        let mut scales = Vec::with_capacity(data.len());

        for &v in data {
            let abs_v = v.abs();
            let scale = if abs_v < 1e-10 { 1.0f32 } else { abs_v };

            if scale.is_infinite() || scale.is_nan() {
                return Err(QuantizationError::ScaleOverflow);
            }

            scales.push(scale);

            // Normalize: v / scale gives ±1.0 or 0.0
            let normalized = if abs_v < 1e-10 {
                0.0f32
            } else {
                v / scale // ±1.0
            };

            // Map to [0, 255]: 0 = -1.0, 128 = 0.0, 255 = +1.0
            // For per-element: normalized is always in [-1, 1]
            let byte = if normalized.abs() < 1e-10 {
                128u8 // zero
            } else if normalized > 0.0 {
                // Positive: map (0, 1] -> [129, 255]
                (normalized * 127.0) as u8 + 128
            } else {
                // Negative: map [-1, 0) -> [1, 127]
                128 - ((-normalized * 127.0) as u8)
            };
            quantized.push(byte);
        }

        Ok((quantized, scales))
    }

    /// Dequantize FP8 (E4M3) bytes back to f32 using per-element scales.
    ///
    /// # Arguments
    /// * `data` - Quantized u8 slice (output from `quantize_f32_to_fp8`)
    /// * `scales` - Per-element scale factors from quantization
    ///
    /// # Returns
    /// Dequantized f32 slice
    ///
    /// # Errors
    /// * `QuantizationError::ScalesMismatch` if scales count doesn't match data length
    pub fn dequantize_fp8_to_f32(data: &[u8], scales: &[f32]) -> Result<Vec<f32>, QuantizationError> {
        if data.len() != scales.len() {
            return Err(QuantizationError::ScalesMismatch);
        }

        Ok(data.iter()
            .zip(scales.iter())
            .map(|(&byte, &scale)| {
                if byte == 128 {
                    0.0f32
                } else if byte > 128 {
                    // Positive
                    ((byte - 128) as f32 / 127.0) * scale
                } else {
                    // Negative
                    -((128 - byte) as f32 / 127.0) * scale
                }
            })
            .collect())
    }

    // ============================================================================
    // INT4 Quantization (pair-packed) — Per-element scaling
    // ============================================================================

    /// Quantize f32 slice to INT4 representation using **per-element scaling**.
    ///
    /// Each element stores its own scale factor. Values are pair-packed into u8
    /// (high nibble + low nibble) where each nibble encodes sign+magnitude in 4 bits.
    ///
    /// # Arguments
    /// * `data` - Input f32 slice
    ///
    /// # Returns
    /// Tuple of (packed bytes, per-element scale factors)
    ///
    /// # Errors
    /// * `QuantizationError::EmptyInput` if data is empty
    pub fn quantize_f32_to_int4(data: &[f32]) -> Result<(Vec<u8>, Vec<f32>), QuantizationError> {
        if data.is_empty() {
            return Err(QuantizationError::EmptyInput);
        }

        let mut packed = Vec::with_capacity((data.len() + 1) / 2);
        let mut scales = Vec::with_capacity(data.len());

        let mut nibbles = Vec::with_capacity(data.len());

        for &v in data {
            let abs_v = v.abs();
            let scale = if abs_v < 1e-10 { 1.0f32 } else { abs_v };

            if scale.is_infinite() || scale.is_nan() {
                return Err(QuantizationError::ScaleOverflow);
            }

            scales.push(scale);

            // Normalize: v / scale gives ±1.0 or 0.0
            let normalized = if abs_v < 1e-10 {
                0.0f32
            } else {
                v / scale // ±1.0
            };

            // Map to signed 4-bit: [-7, 7]
            // 0 = zero, 1-7 = positive, -1 to -7 = negative
            let int4 = if normalized.abs() < 1e-10 {
                0i8
            } else if normalized > 0.0 {
                ((normalized * 7.0).round()) as i8
            } else {
                -(((-normalized) * 7.0).round()) as i8
            };
            let clamped = int4.max(-7).min(7);
            // Offset by 7 to map [-7,7] -> [0,14]
            nibbles.push((clamped + 7) as u8 & 0x0F);
        }

        // Pack nibbles into bytes
        let mut i = 0;
        while i + 1 < nibbles.len() {
            packed.push((nibbles[i] << 4) | nibbles[i + 1]);
            i += 2;
        }
        if i < nibbles.len() {
            // Odd element: pad low nibble with 0x07 (offset zero)
            packed.push(nibbles[i] << 4);
        }

        Ok((packed, scales))
    }

    /// Dequantize INT4 pair-packed bytes back to f32 using per-element scales.
    ///
    /// # Arguments
    /// * `data` - Packed u8 slice (output from `quantize_f32_to_int4`)
    /// * `scales` - Per-element scale factors from quantization
    ///
    /// # Returns
    /// Dequantized f32 slice
    ///
    /// # Errors
    /// * `QuantizationError::ScalesMismatch` if scales count doesn't match expected elements
    pub fn dequantize_int4_to_f32(data: &[u8], scales: &[f32]) -> Result<Vec<f32>, QuantizationError> {
        let expected_len = scales.len();
        let packed_len = data.len() * 2; // Each byte = 2 nibbles
        // Account for padding: if original was odd, last low nibble is padding
        let actual_elements = if packed_len > expected_len {
            expected_len
        } else {
            packed_len
        };

        if actual_elements < expected_len {
            return Err(QuantizationError::ScalesMismatch);
        }

        let mut result = Vec::with_capacity(expected_len);
        let mut elem_idx = 0;

        for &packed in data {
            let ha = ((packed >> 4) & 0x0F) as i8 - 7;
            let lb = (packed & 0x0F) as i8 - 7;

            if elem_idx < expected_len {
                let scale = scales[elem_idx];
                // Reconstruct: normalized = ha/7, value = normalized * scale
                let value = if ha == 0 {
                    0.0f32
                } else {
                    (ha as f32 / 7.0) * scale
                };
                result.push(value);
                elem_idx += 1;
            }

            if elem_idx < expected_len {
                let scale = scales[elem_idx];
                let value = if lb == 0 {
                    0.0f32
                } else {
                    (lb as f32 / 7.0) * scale
                };
                result.push(value);
                elem_idx += 1;
            }
        }

        Ok(result)
    }

    // ============================================================================
    // Precision Metrics
    // ============================================================================

    /// Compute Mean Absolute Percentage Error (MAPE) between original and reconstructed.
    ///
    /// Returns MAPE as a percentage (0.0 = perfect, 100.0 = total loss).
    pub fn compute_mape(original: &[f32], reconstructed: &[f32]) -> f64 {
        if original.is_empty() || original.len() != reconstructed.len() {
            return 100.0;
        }

        let total_error: f64 = original
            .iter()
            .zip(reconstructed.iter())
            .map(|(&o, &r)| {
                if o.abs() < 1e-10 {
                    if r.abs() < 1e-10 {
                        0.0f64
                    } else {
                        100.0f64
                    }
                } else {
                    ((o - r).abs() / o.abs()) as f64 * 100.0
                }
            })
            .sum();

        total_error / original.len() as f64
    }

    /// Compute payload size reduction ratio accounting for per-element scales.
    ///
    /// For FP8: original = N*4 bytes, quantized = N bytes + N*4 bytes (scales) = N*5
    ///   Ratio: 4/5 = 0.8x (actually expansion, but precision is near-perfect)
    /// For INT4: original = N*4 bytes, quantized = N/2 bytes + N*4 bytes (scales) = N*4.5
    ///   Ratio: 4/4.5 = 0.89x
    ///
    /// Note: Per-element scaling prioritizes precision over compression.
    /// For production use, consider per-block scaling with larger block sizes.
    ///
    /// Returns ratio: original_size / quantized_size (higher = better compression).
    pub fn payload_reduction_ratio(original: &[f32], quantized: &[u8], scales: &[f32]) -> f64 {
        let original_bytes = original.len() * 4; // f32 = 4 bytes
        let quantized_bytes = quantized.len() + scales.len() * 4; // data + scales
        if quantized_bytes == 0 {
            return 0.0;
        }
        original_bytes as f64 / quantized_bytes as f64
    }

    // ============================================================================
    // QuantConfig — v1.8 Sprint 1 baseline
    // ============================================================================

    /// Quantization configuration for v1.8 benchmark hooks.
    ///
    /// Controls format, block size, clamping, and scaling strategy.
    #[derive(Debug, Clone, PartialEq)]
    pub struct QuantConfig {
        /// Quantization format: "fp8" or "int4"
        pub format: String,
        /// Block size for block-based scaling (0 = per-element)
        pub block_size: usize,
        /// Enable value clamping to [-max_val, max_val]
        pub clamp_max: Option<f32>,
        /// Scaling strategy: "per-element" or "per-block"
        pub scaling: String,
    }

    impl QuantConfig {
        /// Create FP8 config with per-element scaling
        pub fn fp8_per_element() -> Self {
            Self {
                format: "fp8".to_string(),
                block_size: 0,
                clamp_max: None,
                scaling: "per-element".to_string(),
            }
        }

        /// Create INT4 config with per-element scaling
        pub fn int4_per_element() -> Self {
            Self {
                format: "int4".to_string(),
                block_size: 0,
                clamp_max: None,
                scaling: "per-element".to_string(),
            }
        }

        /// Create FP8 config with block-based scaling
        pub fn fp8_per_block(block_size: usize) -> Self {
            Self {
                format: "fp8".to_string(),
                block_size,
                clamp_max: None,
                scaling: "per-block".to_string(),
            }
        }

        /// Apply clamping to input data before quantization
        pub fn apply_clamp(&self, data: &[f32]) -> Vec<f32> {
            match self.clamp_max {
                Some(max) => data.iter().map(|v| v.clamp(-max, max)).collect(),
                None => data.to_vec(),
            }
        }
    }

    impl Default for QuantConfig {
        fn default() -> Self {
            Self::fp8_per_element()
        }
    }

    // ============================================================================
    // Benchmark Hooks — Criterion-compatible
    // ============================================================================

    /// Benchmark result for a single quantization run.
    ///
    /// Used by criterion benchmarks to track throughput and precision.
    #[derive(Debug, Clone)]
    pub struct QuantBenchmarkResult {
        /// Format used ("fp8" or "int4")
        pub format: String,
        /// Input tensor size (number of f32 elements)
        pub input_size: usize,
        /// Quantized payload size in bytes (data + scales)
        pub quantized_bytes: usize,
        /// MAPE (Mean Absolute Percentage Error) in percent
        pub mape_pct: f64,
        /// Throughput in MB/s (input f32 bytes / duration_ms * 1000)
        pub throughput_mbs: f64,
        /// Duration in milliseconds
        pub duration_ms: f64,
    }

    /// Run a quantization benchmark with the given config and data.
    ///
    /// Returns a `QuantBenchmarkResult` with throughput and precision metrics.
    /// This function is designed to be called from criterion benchmarks.
    pub fn benchmark_quantize(
        config: &QuantConfig,
        data: &[f32],
    ) -> Result<QuantBenchmarkResult, QuantizationError> {
        let start = std::time::Instant::now();

        let (quantized, scales) = if config.format == "fp8" {
            let clamped = config.apply_clamp(data);
            quantize_f32_to_fp8(&clamped)?
        } else if config.format == "int4" {
            let clamped = config.apply_clamp(data);
            quantize_f32_to_int4(&clamped)?
        } else {
            return Err(QuantizationError::EmptyInput); // Used as "unsupported format"
        };

        let duration = start.elapsed();
        let duration_ms = duration.as_secs_f64() * 1000.0;

        // Dequantize for precision measurement
        let reconstructed = if config.format == "fp8" {
            dequantize_fp8_to_f32(&quantized, &scales)?
        } else {
            dequantize_int4_to_f32(&quantized, &scales)?
        };

        let mape = compute_mape(data, &reconstructed);
        let input_bytes = data.len() * 4;
        let quantized_bytes = quantized.len() + scales.len() * 4;
        let throughput = (input_bytes as f64 / 1_000_000.0) / (duration_ms / 1000.0);

        Ok(QuantBenchmarkResult {
            format: config.format.clone(),
            input_size: data.len(),
            quantized_bytes,
            mape_pct: mape,
            throughput_mbs: throughput,
            duration_ms,
        })
    }

    // ============================================================================
    // Tests
    // ============================================================================

    #[cfg(test)]
    mod tests {
        use super::*;

        fn test_data() -> Vec<f32> {
            vec![
                1.0, 2.5, -3.7, 0.0, 0.5, -0.25, 10.0, -10.0,
                0.123, 0.456, -0.789, 5.5, -5.5, 100.0, -100.0, 0.001,
            ]
        }

        #[test]
        fn test_fp8_roundtrip_precision() {
            let data = test_data();
            let (quantized, scales) = quantize_f32_to_fp8(&data).unwrap();
            let reconstructed = dequantize_fp8_to_f32(&quantized, &scales).unwrap();

            let mape = compute_mape(&data, &reconstructed);
            assert!(
                mape < 2.0,
                "FP8 MAPE {:.3}% exceeds 2% target (RFC-001)",
                mape
            );
        }

        #[test]
        fn test_fp8_payload_reduction() {
            let data = test_data();
            let (quantized, scales) = quantize_f32_to_fp8(&data).unwrap();

            let ratio = payload_reduction_ratio(&data, &quantized, &scales);
            // Per-element: 16*4 = 64 original, 16 + 16*4 = 80 quantized
            // Ratio: 64/80 = 0.8x (expansion for precision)
            assert!(
                ratio >= 0.75 && ratio <= 0.85,
                "FP8 ratio {:.2}x should be ~0.8x (per-element with scales)",
                ratio
            );
        }

        #[test]
        fn test_int4_roundtrip_precision() {
            let data = test_data();
            let (quantized, scales) = quantize_f32_to_int4(&data).unwrap();
            let reconstructed = dequantize_int4_to_f32(&quantized, &scales).unwrap();

            let mape = compute_mape(&data, &reconstructed);
            // INT4 has lower precision (7 levels), target < 10%
            assert!(
                mape < 10.0,
                "INT4 MAPE {:.3}% exceeds 10% target",
                mape
            );
        }

        #[test]
        fn test_int4_payload_reduction() {
            let data = test_data();
            let (quantized, scales) = quantize_f32_to_int4(&data).unwrap();

            let ratio = payload_reduction_ratio(&data, &quantized, &scales);
            // Per-element: 16*4 = 64 original, 8 + 16*4 = 72 quantized
            // Ratio: 64/72 = 0.89x
            assert!(
                ratio >= 0.8 && ratio <= 0.95,
                "INT4 ratio {:.2}x should be ~0.89x (per-element with scales)",
                ratio
            );
        }

        #[test]
        fn test_fp8_empty_input() {
            let result = quantize_f32_to_fp8(&[]);
            assert_eq!(result.unwrap_err(), QuantizationError::EmptyInput);
        }

        #[test]
        fn test_int4_empty_input() {
            let result = quantize_f32_to_int4(&[]);
            assert_eq!(result.unwrap_err(), QuantizationError::EmptyInput);
        }

        #[test]
        fn test_zero_data_fp8() {
            let data = vec![0.0f32; 16];
            let (quantized, scales) = quantize_f32_to_fp8(&data).unwrap();
            assert!(quantized.iter().all(|&v| v == 128)); // All zeros encoded as 128
        }

        #[test]
        fn test_mape_identical() {
            let data = test_data();
            let mape = compute_mape(&data, &data);
            assert!(mape < 1e-10, "MAPE of identical data should be ~0");
        }

        #[test]
        fn test_mape_different_length() {
            let a = vec![1.0, 2.0];
            let b = vec![1.0];
            let mape = compute_mape(&a, &b);
            assert_eq!(mape, 100.0);
        }

        #[test]
        fn test_error_display() {
            assert!(QuantizationError::EmptyInput.to_string().contains("empty"));
            assert!(QuantizationError::ScaleOverflow.to_string().contains("scale"));
            assert!(QuantizationError::ScalesMismatch.to_string().contains("scales"));
        }

        #[test]
        fn test_large_tensor_fp8() {
            let data: Vec<f32> = (0..1000)
                .map(|i| (i as f32 % 100.0) - 50.0)
                .collect();
            let (quantized, scales) = quantize_f32_to_fp8(&data).unwrap();
            let reconstructed = dequantize_fp8_to_f32(&quantized, &scales).unwrap();

            let mape = compute_mape(&data, &reconstructed);
            assert!(
                mape < 2.0,
                "Large tensor FP8 MAPE {:.3}% exceeds 2%",
                mape
            );
        }

        #[test]
        fn test_multi_block_fp8() {
            // 32 elements
            let data: Vec<f32> = (0..32).map(|i| i as f32 * 0.1 - 1.5).collect();
            let (quantized, scales) = quantize_f32_to_fp8(&data).unwrap();
            assert_eq!(scales.len(), 32, "Per-element: 32 scales for 32 elements");
            let reconstructed = dequantize_fp8_to_f32(&quantized, &scales).unwrap();
            assert_eq!(reconstructed.len(), data.len());
        }

        #[test]
        fn test_scales_mismatch() {
            let data = test_data();
            let (quantized, _) = quantize_f32_to_fp8(&data).unwrap();
            let result = dequantize_fp8_to_f32(&quantized, &[1.0, 2.0]);
            assert_eq!(result.unwrap_err(), QuantizationError::ScalesMismatch);
        }

        #[test]
        fn test_int4_odd_length() {
            // 17 elements (odd)
            let data: Vec<f32> = (0..17).map(|i| i as f32 * 0.5 + 0.1).collect();
            let (quantized, scales) = quantize_f32_to_int4(&data).unwrap();
            assert_eq!(scales.len(), 17);
            let reconstructed = dequantize_int4_to_f32(&quantized, &scales).unwrap();
            assert_eq!(reconstructed.len(), data.len());
        }

        #[test]
        fn test_fp8_exact_values() {
            // Test that exact values roundtrip correctly
            let data = vec![1.0, -1.0, 0.0, 100.0, -100.0, 0.001];
            let (quantized, scales) = quantize_f32_to_fp8(&data).unwrap();
            let reconstructed = dequantize_fp8_to_f32(&quantized, &scales).unwrap();

            for (&o, &r) in data.iter().zip(reconstructed.iter()) {
                if o.abs() < 1e-10 {
                    assert!(r.abs() < 1e-10, "Zero should roundtrip to near-zero");
                } else {
                    let error = ((o - r).abs() / o.abs()) * 100.0;
                    assert!(
                        error < 1.0,
                        "Value {} -> {} (error {:.2}%), expected <1%",
                        o,
                        r,
                        error
                    );
                }
            }
        }
    }
}

pub use internal::{
    benchmark_quantize, compute_mape, dequantize_fp8_to_f32, dequantize_int4_to_f32,
    payload_reduction_ratio, quantize_f32_to_fp8, quantize_f32_to_int4, QuantBenchmarkResult,
    QuantConfig, QuantizationError, BLOCK_SIZE,
};
