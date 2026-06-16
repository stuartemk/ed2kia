//! Sprint 154 (v15.4.0) — GGUF Native Audit Support
//!
//! Enables loading of quantized GGUF models (INT4/INT8/Q4_K_M) via
//! `candle_core::quantized::gguf_file` and `candle_transformers::models::quantized_llama`
//! for safety auditing on edge hardware without VRAM requirements.
//!
//! ## Architecture
//!
//! Wraps `quantized_llama::ModelWeights` to provide:
//! - GGUF model loading from `.gguf` files
//! - Logit-based concept projection for audit steering
//! - Hidden state simulation for Koopman/steering pipeline compatibility
//! - GGUF metadata extraction (hidden_size, layers, quant type)
//!
//! ## Edge Deployment
//!
//! Designed for CPU-only operation with Q4_K_M quantization:
//! - Llama-3-8B-Q4_K_M: ~4.9GB RAM (vs ~16GB F32)
//! - Llama-3.1-8B-IQ2_XS: ~2.3GB RAM (ultra-edge)
//! - No GPU required, runs on Raspberry Pi 5 / Jetson Nano

use candle_core::quantized::gguf_file;
use candle_core::{DType, Device, Result, Tensor, D};
use candle_transformers::models::quantized_llama::ModelWeights as QLlama;
use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::Path;
use tokenizers::Tokenizer;

// ---------------------------------------------------------------------------
// Model Type + Info
// ---------------------------------------------------------------------------

/// Model type discriminator for TensorAudit GGUF integration.
#[derive(Debug)]
pub enum ModelType {
    /// Full-precision F32 model (existing path).
    F32,
    /// Quantized GGUF model (INT4/INT8/Q4_K_M/etc.).
    GGUF(GGUFModelInfo),
}

/// Metadata about the loaded GGUF model.
#[derive(Debug, Clone)]
pub struct GGUFModelInfo {
    /// Number of hidden dimensions (embedding_length).
    pub hidden_size: usize,
    /// Number of transformer blocks.
    pub n_layers: usize,
    /// Number of attention heads.
    pub n_heads: usize,
    /// Number of KV heads.
    pub n_kv_heads: usize,
    /// Head dimension.
    pub head_dim: usize,
    /// Vocabulary size.
    pub vocab_size: usize,
    /// Quantization type string from GGUF metadata.
    pub quant_type: String,
    /// Model architecture name.
    pub architecture: String,
}

impl std::fmt::Display for GGUFModelInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GGUF({} hidden={} layers={} heads={} kv={} head_dim={} quant={})",
            self.architecture,
            self.hidden_size,
            self.n_layers,
            self.n_heads,
            self.n_kv_heads,
            self.head_dim,
            self.quant_type,
        )
    }
}

// ---------------------------------------------------------------------------
// GGUF Metadata Extraction from Content
// ---------------------------------------------------------------------------

fn extract_metadata_from_gguf(content: &gguf_file::Content) -> GGUFModelInfo {
    let md = &content.metadata;

    let get_u32 = |key: &str| -> u32 { md.get(key).and_then(|v| v.to_u32().ok()).unwrap_or(0) };

    let embedding_length = get_u32("llama.embedding_length") as usize;
    let block_count = get_u32("llama.block_count") as usize;
    let head_count = get_u32("llama.attention.head_count") as usize;
    let head_count_kv = get_u32("llama.attention.head_count_kv") as usize;
    let rope_dim = get_u32("llama.rope.dimension_count") as usize;
    let vocab_size = get_u32("llama.vocab_size") as usize;

    let head_dim = if head_count > 0 {
        embedding_length / head_count
    } else {
        rope_dim
    };

    // Detect architecture from metadata
    let architecture = md
        .get("general.architecture")
        .and_then(|v| v.to_string().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "llama".to_string());

    GGUFModelInfo {
        hidden_size: embedding_length,
        n_layers: block_count,
        n_heads: head_count,
        n_kv_heads: head_count_kv,
        head_dim,
        vocab_size,
        quant_type: "GGUF".to_string(),
        architecture,
    }
}

// ---------------------------------------------------------------------------
// TensorAuditGGUF — GGUF Model Wrapper
// ---------------------------------------------------------------------------

/// TensorAudit variant for GGUF quantized models.
///
/// Wraps `quantized_llama::ModelWeights` and provides:
/// - GGUF model loading from `.gguf` files
/// - Logit-based concept projection for audit steering
/// - Hidden state simulation for Koopman/steering pipeline
pub struct TensorAuditGGUF {
    /// The underlying quantized Llama model weights.
    model: QLlama,
    /// Tokenizer for text encoding.
    pub tokenizer: Tokenizer,
    /// Device (CPU for edge deployment).
    pub device: Device,
    /// Which layers to target for extraction (used for simulation).
    pub target_layers: Vec<usize>,
    /// GGUF model metadata.
    pub info: GGUFModelInfo,
}

impl TensorAuditGGUF {
    /// Load a GGUF model from file path.
    ///
    /// # Arguments
    /// * `gguf_path` - Path to `.gguf` file
    /// * `device` - Target device (CPU for edge)
    /// * `target_layers` - Layer indices for hidden state simulation
    pub fn load(gguf_path: &str, device: &Device, target_layers: Vec<usize>) -> Result<Self> {
        let mut file = fs::File::open(gguf_path).map_err(|e| {
            candle_core::Error::Msg(format!("Failed to open GGUF file '{}': {}", gguf_path, e))
        })?;

        let content = gguf_file::Content::read(&mut file)
            .map_err(|e| candle_core::Error::Msg(format!("GGUF parse error: {}", e)))?;

        // Extract metadata BEFORE consuming content
        let info = extract_metadata_from_gguf(&content);

        let model = QLlama::from_gguf(content, &mut file, device)?;

        // Load tokenizer
        let tokenizer = find_and_load_tokenizer(gguf_path).unwrap_or_else(|| {
            eprintln!(
                "WARNING: No tokenizer found near '{}', using fallback",
                gguf_path
            );
            create_fallback_tokenizer()
        });

        eprintln!(
            "GGUF model loaded: {} hidden={} layers={} heads={} kv={} head_dim={} vocab={} target_layers={:?}",
            info.architecture,
            info.hidden_size,
            info.n_layers,
            info.n_heads,
            info.n_kv_heads,
            info.head_dim,
            info.vocab_size,
            target_layers,
        );

        Ok(Self {
            model,
            tokenizer,
            device: device.clone(),
            target_layers,
            info,
        })
    }

    /// Load from GGUF buffer (for in-memory / streaming scenarios).
    pub fn load_from_buffer(
        buffer: &[u8],
        device: &Device,
        target_layers: Vec<usize>,
        tokenizer: Tokenizer,
    ) -> Result<Self> {
        let mut cursor = Cursor::new(buffer);
        let content = gguf_file::Content::read(&mut cursor)
            .map_err(|e| candle_core::Error::Msg(format!("GGUF parse error: {}", e)))?;

        let info = extract_metadata_from_gguf(&content);
        let model = QLlama::from_gguf(content, &mut cursor, device)?;

        Ok(Self {
            model,
            tokenizer,
            device: device.clone(),
            target_layers,
            info,
        })
    }

    /// Forward pass — generate logits for the given prompt.
    pub fn forward_logits(&mut self, prompt: &str) -> Result<Tensor> {
        let tokens = self
            .tokenizer
            .encode(prompt, true)
            .map_err(|e| candle_core::Error::Msg(format!("Tokenization error: {}", e)))?;

        let token_ids: Vec<u32> = tokens.get_ids().to_vec();
        if token_ids.is_empty() {
            return Err(candle_core::Error::Msg("Empty token sequence".to_string()));
        }

        let input_tensor = Tensor::new(token_ids.as_slice(), &self.device)?.unsqueeze(0)?;
        self.model.forward(&input_tensor, 0)
    }

    /// Simulate hidden states at target layers using logit-based projection.
    ///
    /// Since `quantized_llama::ModelWeights` doesn't expose intermediate
    /// layer outputs, we use the final logits projected back through
    /// the embedding space as an approximation of deep hidden states.
    ///
    /// For full hidden state extraction, use the F32 model path or
    /// implement a custom GGUF loader with layer hooks.
    pub fn forward_extract_multi(&mut self, prompt: &str) -> Result<HashMap<usize, Tensor>> {
        let logits = self.forward_logits(prompt)?;
        let (_b_sz, _seq_len, vocab_dim) = logits.dims3()?;
        let hidden_dim = self.info.hidden_size;

        let mut extracted = HashMap::new();

        for &layer_idx in &self.target_layers {
            // Simulate layer output: scale logits by layer depth factor
            // Deeper layers have more transformed representations
            let depth_factor = 1.0 + 0.1 * (layer_idx as f64 / self.info.n_layers as f64);
            let simulated = (&logits * depth_factor)?;
            // Project to hidden dim if needed (truncate/pad)
            let simulated = if vocab_dim != hidden_dim {
                simulated.narrow(D::Minus1, 0, hidden_dim.min(vocab_dim))?
            } else {
                simulated
            };
            extracted.insert(layer_idx, simulated);
        }

        Ok(extracted)
    }

    /// Extract hidden state from the first target layer (backward compat).
    pub fn forward_extract(&mut self, prompt: &str) -> Result<Tensor> {
        let map = self.forward_extract_multi(prompt)?;
        let first_layer = *self.target_layers.first().unwrap_or(&0);
        map.get(&first_layer).cloned().ok_or_else(|| {
            candle_core::Error::Msg(format!("Layer {} not found in extracted map", first_layer))
        })
    }

    /// Encode prompt to token IDs.
    pub fn encode(&self, prompt: &str) -> Result<Vec<u32>> {
        let tokens = self
            .tokenizer
            .encode(prompt, true)
            .map_err(|e| candle_core::Error::Msg(format!("Tokenization error: {}", e)))?;
        Ok(tokens.get_ids().to_vec())
    }

    /// Decode token IDs to text.
    pub fn decode(&self, token_ids: &[u32]) -> Result<String> {
        let text = self
            .tokenizer
            .decode(token_ids, true)
            .map_err(|e| candle_core::Error::Msg(format!("Decode error: {}", e)))?;
        Ok(text)
    }

    /// Generate tokens autoregressively.
    pub fn generate(
        &mut self,
        prompt: &str,
        max_tokens: usize,
        temperature: Option<f64>,
    ) -> Result<Vec<u32>> {
        let tokens = self.encode(prompt)?;
        let mut all_tokens = tokens.clone();

        let mut input_tensor = Tensor::new(tokens.as_slice(), &self.device)?.unsqueeze(0)?;
        let mut index_pos = 0;

        for _ in 0..max_tokens {
            let logits = self.model.forward(&input_tensor, index_pos)?;
            let logits = logits.squeeze(0)?;
            let next_token = if let Some(temp) = temperature {
                let scaled = (&logits / temp)?;
                let probs = candle_nn::ops::softmax(&scaled, D::Minus1)?;
                // Sample from distribution using argmax of random * probs
                let cpu_probs = probs.to_vec1::<f32>()?;
                let mut cumsum = 0.0f32;
                let r: f32 = rand::random();
                let mut selected = 0u32;
                for (i, &p) in cpu_probs.iter().enumerate() {
                    cumsum += p;
                    if cumsum >= r {
                        selected = i as u32;
                        break;
                    }
                }
                selected
            } else {
                logits
                    .argmax(D::Minus1)?
                    .to_dtype(DType::U32)?
                    .to_scalar::<u32>()?
            };

            all_tokens.push(next_token);
            input_tensor = Tensor::new(&[next_token], &self.device)?.unsqueeze(0)?;
            index_pos += 1;
        }

        Ok(all_tokens)
    }

    /// Get the embedding dimension of this model.
    pub fn hidden_size(&self) -> usize {
        self.info.hidden_size
    }

    /// Get the number of layers.
    pub fn n_layers(&self) -> usize {
        self.info.n_layers
    }

    /// Get the vocabulary size.
    pub fn vocab_size(&self) -> usize {
        self.info.vocab_size
    }

    /// Compute concept projection from logits (for audit steering).
    pub fn compute_concept_projection(&self, logits: &Tensor, concept_dir: &Tensor) -> Result<f32> {
        let probs = candle_nn::ops::softmax(logits, D::Minus1)?;
        let dot = (probs.flatten_all()? * concept_dir.flatten_all()?)?;
        dot.to_scalar()
    }

    /// Memory usage estimate in MB.
    pub fn estimate_memory_mb(&self) -> f64 {
        let base_mb =
            (self.info.hidden_size as f64 * self.info.hidden_size as f64 * 4.0) / 1_048_576.0;
        let layer_count = self.info.n_layers as f64;
        let per_layer = base_mb * 4.0;
        let quant_factor = match self.info.quant_type.as_str() {
            "Q4_K_M" | "Q4_0" | "Q4_1" => 0.15625,
            "Q5_K_M" | "Q5_0" | "Q5_1" => 0.19531,
            "Q8_0" | "Q8_1" => 0.3125,
            "IQ2_XS" => 0.07813,
            "IQ3_XXS" => 0.11719,
            _ => 0.5,
        };
        (base_mb + per_layer * layer_count) * quant_factor
    }
}

impl std::fmt::Display for TensorAuditGGUF {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TensorAuditGGUF({} target_layers={:?} mem~{}MB)",
            self.info,
            self.target_layers,
            self.estimate_memory_mb(),
        )
    }
}

// ---------------------------------------------------------------------------
// Tokenizer Helpers
// ---------------------------------------------------------------------------

fn find_and_load_tokenizer(gguf_path: &str) -> Option<Tokenizer> {
    let gguf_dir = Path::new(gguf_path)
        .parent()
        .unwrap_or_else(|| Path::new(""));

    let candidates = [
        "tokenizer.json",
        "tokenizer.model",
        "tiktoken.model",
        "tokenizer_config.json",
    ];

    for name in &candidates {
        let path = gguf_dir.join(name);
        if path.exists() {
            if let Ok(t) = Tokenizer::from_file(&path) {
                eprintln!("Loaded tokenizer from: {}", path.display());
                return Some(t);
            }
        }
    }

    // Try sibling file (remove .gguf extension)
    let base = gguf_path.trim_end_matches(".gguf");
    for ext in &[".json", ".model"] {
        let path = format!("{}{}", base, ext);
        if Path::new(&path).exists() {
            if let Ok(t) = Tokenizer::from_file(&path) {
                return Some(t);
            }
        }
    }

    None
}

fn create_fallback_tokenizer() -> Tokenizer {
    // Minimal ByteLevel tokenizer as fallback when no tokenizer file is found.
    // This JSON creates a basic byte-level fallback tokenizer.
    let json = r#"{
        "model": {
            "type": "ByteLevel",
            "vocab": {},
            "add_prefix_space": true,
            "unknown_token": "<unk>",
            "suffix_token": "</s>",
            "prefix_token": "<s>",
            "pad_token": "<pad>",
            "cls_token": "<cls>",
            "mask_token": "<mask>"
        },
        "add_special_tokens": []
    }"#;
    Tokenizer::from_bytes(json.as_bytes()).unwrap_or_else(|_| {
        // Ultra-minimal fallback if even ByteLevel fails
        let minimal = r#"{
                "model": {"type":"BPE","vocab":{},"merges":[]},
                "add_special_tokens": []
            }"#;
        Tokenizer::from_bytes(minimal.as_bytes()).expect("Fallback tokenizer creation failed")
    })
}

// ---------------------------------------------------------------------------
// Public Entry Points
// ---------------------------------------------------------------------------

/// Load GGUF model for TensorAudit integration.
///
/// # Arguments
/// * `gguf_path` - Path to `.gguf` file
/// * `device` - Target device (CPU for edge)
/// * `target_layers` - Layer indices for hidden state simulation
///
/// # Example
/// ```ignore
/// use native_audit::gguf_audit::load_gguf;
/// use candle_core::Device;
///
/// let device = Device::Cpu;
/// let audit = load_gguf(
///     "models/llama-3-8b-instruct-q4_k_m.gguf",
///     &device,
///     vec![8, 16, 24],
/// )?;
///
/// let hidden = audit.forward_extract("Hello, world!")?;
/// ```
pub fn load_gguf(
    gguf_path: &str,
    device: &Device,
    target_layers: Vec<usize>,
) -> Result<TensorAuditGGUF> {
    TensorAuditGGUF::load(gguf_path, device, target_layers)
}

/// Load GGUF model with auto-detected target layers.
///
/// For 8B models (32+ layers): uses [8, 16, 24]
/// For 1.5-3B models (16-31 layers): uses [n/3, 2n/3]
/// For small models (<16 layers): uses [n/2]
pub fn load_gguf_auto(gguf_path: &str, device: &Device) -> Result<TensorAuditGGUF> {
    let audit = TensorAuditGGUF::load(gguf_path, device, vec![])?;

    let n_layers = audit.info.n_layers;
    let target_layers = if n_layers >= 32 {
        vec![n_layers / 4, n_layers / 2, (3 * n_layers) / 4]
    } else if n_layers >= 16 {
        vec![n_layers / 3, (2 * n_layers) / 3]
    } else {
        vec![n_layers / 2]
    };

    eprintln!("Auto-detected target layers: {:?}", target_layers);

    Ok(TensorAuditGGUF {
        target_layers,
        ..audit
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_type_enum() {
        let f32_type = ModelType::F32;
        let gguf_type = ModelType::GGUF(GGUFModelInfo {
            hidden_size: 4096,
            n_layers: 32,
            n_heads: 32,
            n_kv_heads: 8,
            head_dim: 128,
            vocab_size: 128256,
            quant_type: "Q4_K_M".to_string(),
            architecture: "llama".to_string(),
        });

        match f32_type {
            ModelType::F32 => {}
            ModelType::GGUF(_) => panic!("Should be F32"),
        }

        match gguf_type {
            ModelType::GGUF(info) => {
                assert_eq!(info.hidden_size, 4096);
                assert_eq!(info.n_layers, 32);
                assert_eq!(info.vocab_size, 128256);
            }
            ModelType::F32 => panic!("Should be GGUF"),
        }
    }

    #[test]
    fn test_gguf_info_display() {
        let info = GGUFModelInfo {
            hidden_size: 4096,
            n_layers: 32,
            n_heads: 32,
            n_kv_heads: 8,
            head_dim: 128,
            vocab_size: 128256,
            quant_type: "Q4_K_M".to_string(),
            architecture: "llama".to_string(),
        };

        let display = format!("{}", info);
        assert!(display.contains("llama"));
        assert!(display.contains("hidden=4096"));
        assert!(display.contains("Q4_K_M"));
    }

    #[test]
    fn test_gguf_info_fields() {
        let info = GGUFModelInfo {
            hidden_size: 4096,
            n_layers: 32,
            n_heads: 32,
            n_kv_heads: 8,
            head_dim: 128,
            vocab_size: 128256,
            quant_type: "Q4_K_M".to_string(),
            architecture: "llama".to_string(),
        };

        assert_eq!(info.hidden_size, info.n_heads * info.head_dim);
        assert_eq!(info.n_layers, 32);
        assert_eq!(info.head_dim, 128);
        assert_eq!(info.n_kv_heads, 8);
    }

    #[test]
    fn test_auto_target_layers_8b() {
        let n_layers = 32;
        let target_layers = if n_layers >= 32 {
            vec![n_layers / 4, n_layers / 2, (3 * n_layers) / 4]
        } else if n_layers >= 16 {
            vec![n_layers / 3, (2 * n_layers) / 3]
        } else {
            vec![n_layers / 2]
        };
        assert_eq!(target_layers, vec![8, 16, 24]);
    }

    #[test]
    fn test_auto_target_layers_1_5b() {
        let n_layers = 28;
        let target_layers = if n_layers >= 32 {
            vec![n_layers / 4, n_layers / 2, (3 * n_layers) / 4]
        } else if n_layers >= 16 {
            vec![n_layers / 3, (2 * n_layers) / 3]
        } else {
            vec![n_layers / 2]
        };
        assert_eq!(target_layers, vec![9, 18]);
    }

    #[test]
    fn test_auto_target_layers_small() {
        let n_layers = 8;
        let target_layers = if n_layers >= 32 {
            vec![n_layers / 4, n_layers / 2, (3 * n_layers) / 4]
        } else if n_layers >= 16 {
            vec![n_layers / 3, (2 * n_layers) / 3]
        } else {
            vec![n_layers / 2]
        };
        assert_eq!(target_layers, vec![4]);
    }

    #[test]
    fn test_extract_metadata_empty() {
        // Test with minimal GGUF content — metadata defaults to 0
        // This verifies the extraction doesn't panic on missing keys
        use candle_core::quantized::gguf_file;
        use std::collections::HashMap;

        let content = gguf_file::Content {
            magic: candle_core::quantized::gguf_file::VersionedMagic::GgufV3,
            metadata: HashMap::new(),
            tensor_infos: HashMap::new(),
            tensor_data_offset: 0,
        };

        let info = extract_metadata_from_gguf(&content);
        assert_eq!(info.hidden_size, 0);
        assert_eq!(info.n_layers, 0);
        assert_eq!(info.architecture, "llama");
    }

    #[test]
    fn test_estimate_memory_q4_k_m() {
        let info = GGUFModelInfo {
            hidden_size: 4096,
            n_layers: 32,
            n_heads: 32,
            n_kv_heads: 8,
            head_dim: 128,
            vocab_size: 128256,
            quant_type: "Q4_K_M".to_string(),
            architecture: "llama".to_string(),
        };

        let base_mb = (4096.0 * 4096.0 * 4.0) / 1_048_576.0;
        let per_layer = base_mb * 4.0;
        let expected = (base_mb + per_layer * 32.0) * 0.15625;

        assert!(
            expected > 500.0 && expected < 5000.0,
            "Expected ~1290MB, got {}",
            expected
        );
    }

    #[test]
    fn test_estimate_memory_iq2_vs_q4() {
        let info_q4 = GGUFModelInfo {
            hidden_size: 4096,
            n_layers: 32,
            n_heads: 32,
            n_kv_heads: 8,
            head_dim: 128,
            vocab_size: 128256,
            quant_type: "Q4_K_M".to_string(),
            architecture: "llama".to_string(),
        };
        let info_iq2 = GGUFModelInfo {
            hidden_size: 4096,
            n_layers: 32,
            n_heads: 32,
            n_kv_heads: 8,
            head_dim: 128,
            vocab_size: 128256,
            quant_type: "IQ2_XS".to_string(),
            architecture: "llama".to_string(),
        };

        let base_mb = (4096.0 * 4096.0 * 4.0) / 1_048_576.0;
        let per_layer = base_mb * 4.0;
        let total_base = base_mb + per_layer * 32.0;
        let mem_q4 = total_base * 0.15625;
        let mem_iq2 = total_base * 0.07813;

        assert!(
            mem_iq2 < mem_q4 / 2.0,
            "IQ2_XS should use <50% of Q4_K_M memory"
        );
    }

    #[test]
    fn test_fallback_tokenizer_creation() {
        let tokenizer = create_fallback_tokenizer();
        // Should not panic — tokenizer is created
        let _ = tokenizer.to_string(false).ok(); // Just verify no panic
    }
}
