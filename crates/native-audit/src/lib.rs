//! native-audit — Native tensor audit core for ed2kIA
//!
//! Candle-based hidden state extraction + TCM Z-axis for SmolLM2-135M.
//!
//! Fully self-contained Llama implementation with public block access
//! for intermediate hidden state extraction.

use candle_core::{DType, Device, Result, Tensor, D};
use candle_nn::{embedding, Embedding, Module, VarBuilder};
use candle_transformers::models::llama::{Config, LlamaConfig};
use std::fs;
use std::path::PathBuf;

#[allow(dead_code)]
const HF_ENDPOINT: &str = "https://huggingface.co";
const MODEL_REPO: &str = "HuggingFaceTB/SmolLM2-135M";

fn download_file(repo: &str, filename: &str) -> Result<PathBuf> {
    let cache_dir = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
        .join(".cache")
        .join("ed2kIA")
        .join(repo);
    fs::create_dir_all(&cache_dir)?;
    let dest = cache_dir.join(filename);
    if !dest.exists() {
        // Use direct download URL (avoids redirect timeout)
        let url = format!("https://huggingface.co/{}/resolve/main/{}", repo, filename);
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(600))
            .build()
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?;
        let resp = client
            .get(&url)
            .send()
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?;
        let bytes = resp
            .bytes()
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?;
        fs::write(&dest, bytes.as_ref())?;
    }
    Ok(dest)
}
use std::collections::HashMap;
use tokenizers::Tokenizer;

pub const MAX_SEQ_LEN: usize = 4096;

// ---------------------------------------------------------------------------
// Custom Cache with public fields for block access
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Cache {
    #[allow(dead_code)]
    masks: HashMap<usize, Tensor>,
    pub use_kv_cache: bool,
    pub kvs: Vec<Option<(Tensor, Tensor)>>,
    pub cos: Tensor,
    pub sin: Tensor,
    #[allow(dead_code)]
    device: Device,
}

impl Cache {
    fn new(use_kv_cache: bool, dtype: DType, config: &Config, device: &Device) -> Result<Self> {
        let n_elem = config.hidden_size / config.num_attention_heads;
        let theta: Vec<_> = (0..n_elem)
            .step_by(2)
            .map(|i| 1f32 / config.rope_theta.powf(i as f32 / n_elem as f32))
            .collect();
        let theta = Tensor::new(theta.as_slice(), device)?;
        let idx_theta = Tensor::arange(0, MAX_SEQ_LEN as u32, device)?
            .to_dtype(DType::F32)?
            .reshape((MAX_SEQ_LEN, 1))?
            .matmul(&theta.reshape((1, theta.elem_count()))?)?;
        let cos = idx_theta.cos()?.to_dtype(dtype)?;
        let sin = idx_theta.sin()?.to_dtype(dtype)?;
        Ok(Self {
            masks: HashMap::new(),
            use_kv_cache,
            kvs: vec![None; config.num_hidden_layers],
            device: device.clone(),
            cos,
            sin,
        })
    }

    #[allow(dead_code)]
    fn mask(&mut self, t: usize) -> Result<Tensor> {
        if let Some(mask) = self.masks.get(&t) {
            return Ok(mask.clone());
        }
        let mask: Vec<_> = (0..t)
            .flat_map(|i| (0..t).map(move |j| u8::from(j > i)))
            .collect();
        let mask = Tensor::from_slice(&mask, (t, t), &self.device)?;
        self.masks.insert(t, mask.clone());
        Ok(mask)
    }
}

// ---------------------------------------------------------------------------
// RmsNorm wrapper (loads tensor from VarBuilder)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct RmsNorm {
    weight: Tensor,
    eps: f64,
}

impl RmsNorm {
    fn load(size: usize, eps: f64, vb: VarBuilder) -> Result<Self> {
        let weight = vb.get(size, "weight")?;
        Ok(Self { weight, eps })
    }

    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let dtype = x.dtype();
        let internal_dtype = match dtype {
            DType::F16 | DType::BF16 => DType::F32,
            d => d,
        };
        let new_size = x.shape().clone();
        let x_norm = x.to_dtype(internal_dtype)?.sqr()?.sum_keepdim(D::Minus1)?;
        let hidden_size = self.weight.elem_count() as f64;
        let rms = (x_norm / hidden_size)?.sqrt()?;
        let eps_tensor = Tensor::full(self.eps as f32, rms.shape(), rms.device())?;
        let normed = x.broadcast_div(&(rms.add(&eps_tensor)?))?;
        normed
            .to_dtype(dtype)?
            .broadcast_mul(&self.weight)?
            .reshape(new_size.dims())
    }
}

// ---------------------------------------------------------------------------
// Attention & MLP blocks
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct CausalSelfAttention {
    q_proj: candle_nn::Linear,
    k_proj: candle_nn::Linear,
    v_proj: candle_nn::Linear,
    o_proj: candle_nn::Linear,
    num_attention_heads: usize,
    num_key_value_heads: usize,
    head_dim: usize,
}

impl CausalSelfAttention {
    fn forward(
        &self,
        x: &Tensor,
        index_pos: usize,
        block_idx: usize,
        cache: &mut Cache,
    ) -> Result<Tensor> {
        let (b_sz, seq_len, hidden_size) = x.dims3()?;
        let q = self.q_proj.forward(x)?;
        let k = self.k_proj.forward(x)?;
        let v = self.v_proj.forward(x)?;

        let q = q
            .reshape((b_sz, seq_len, self.num_attention_heads, self.head_dim))?
            .transpose(1, 2)?
            .contiguous()?;
        let k = k
            .reshape((b_sz, seq_len, self.num_key_value_heads, self.head_dim))?
            .transpose(1, 2)?
            .contiguous()?;
        let mut v = v
            .reshape((b_sz, seq_len, self.num_key_value_heads, self.head_dim))?
            .transpose(1, 2)?;

        let cos = cache.cos.narrow(0, index_pos, seq_len)?;
        let sin = cache.sin.narrow(0, index_pos, seq_len)?;
        let q = candle_nn::rotary_emb::rope(&q, &cos, &sin)?;
        let mut k = candle_nn::rotary_emb::rope(&k, &cos, &sin)?;

        if cache.use_kv_cache {
            if let Some((cache_k, cache_v)) = &cache.kvs[block_idx] {
                k = Tensor::cat(&[cache_k, &k], 2)?.contiguous()?;
                v = Tensor::cat(&[cache_v, &v], 2)?.contiguous()?;
            }
            cache.kvs[block_idx] = Some((k.clone(), v.clone()));
        }

        let k = self.repeat_kv(k)?;
        let v = self.repeat_kv(v)?;

        let q = q.to_dtype(DType::F32)?;
        let k = k.to_dtype(DType::F32)?;
        let v = v.to_dtype(DType::F32)?;
        let att = (q.matmul(&k.t()?)? / (self.head_dim as f64).sqrt())?;
        let att = candle_nn::ops::softmax(&att, D::Minus1)?;
        let y = att.matmul(&v.contiguous()?)?.to_dtype(q.dtype())?;
        let y = y.transpose(1, 2)?.reshape(&[b_sz, seq_len, hidden_size])?;
        self.o_proj.forward(&y)
    }

    fn repeat_kv(&self, x: Tensor) -> Result<Tensor> {
        let repeat = self.num_attention_heads / self.num_key_value_heads;
        if repeat <= 1 {
            return Ok(x);
        }
        let (b_sz, nheads, seq_len, head_dim) = x.dims4()?;
        x.unsqueeze(2)?
            .expand((b_sz, nheads, repeat, seq_len, head_dim))?
            .reshape((b_sz, nheads * repeat, seq_len, head_dim))
    }

    fn load(vb: VarBuilder, cfg: &Config) -> Result<Self> {
        let size_in = cfg.hidden_size;
        let size_q = (cfg.hidden_size / cfg.num_attention_heads) * cfg.num_attention_heads;
        let size_kv = (cfg.hidden_size / cfg.num_attention_heads) * cfg.num_key_value_heads;
        let q_proj = candle_nn::linear_no_bias(size_in, size_q, vb.pp("q_proj"))?;
        let k_proj = candle_nn::linear_no_bias(size_in, size_kv, vb.pp("k_proj"))?;
        let v_proj = candle_nn::linear_no_bias(size_in, size_kv, vb.pp("v_proj"))?;
        let o_proj = candle_nn::linear_no_bias(size_q, size_in, vb.pp("o_proj"))?;
        Ok(Self {
            q_proj,
            k_proj,
            v_proj,
            o_proj,
            num_attention_heads: cfg.num_attention_heads,
            num_key_value_heads: cfg.num_key_value_heads,
            head_dim: cfg.hidden_size / cfg.num_attention_heads,
        })
    }
}

#[derive(Debug, Clone)]
struct Mlp {
    c_fc1: candle_nn::Linear,
    c_fc2: candle_nn::Linear,
    c_proj: candle_nn::Linear,
}

impl Mlp {
    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let x = (candle_nn::ops::silu(&self.c_fc1.forward(x)?)? * self.c_fc2.forward(x)?)?;
        self.c_proj.forward(&x)
    }

    fn load(vb: VarBuilder, cfg: &Config) -> Result<Self> {
        let h_size = cfg.hidden_size;
        let i_size = cfg.intermediate_size;
        let c_fc1 = candle_nn::linear_no_bias(h_size, i_size, vb.pp("gate_proj"))?;
        let c_fc2 = candle_nn::linear_no_bias(h_size, i_size, vb.pp("up_proj"))?;
        let c_proj = candle_nn::linear_no_bias(i_size, h_size, vb.pp("down_proj"))?;
        Ok(Self {
            c_fc1,
            c_fc2,
            c_proj,
        })
    }
}

#[derive(Debug, Clone)]
struct Block {
    rms_1: RmsNorm,
    attn: CausalSelfAttention,
    rms_2: RmsNorm,
    mlp: Mlp,
}

impl Block {
    fn forward(
        &self,
        x: &Tensor,
        index_pos: usize,
        block_idx: usize,
        cache: &mut Cache,
    ) -> Result<Tensor> {
        let residual = x;
        let x = self.rms_1.forward(x)?;
        let x = (self.attn.forward(&x, index_pos, block_idx, cache)? + residual)?;
        let residual = &x;
        let x = (self.mlp.forward(&self.rms_2.forward(&x)?)? + residual)?;
        Ok(x)
    }

    fn load(vb: VarBuilder, cfg: &Config) -> Result<Self> {
        let attn = CausalSelfAttention::load(vb.pp("self_attn"), cfg)?;
        let mlp = Mlp::load(vb.pp("mlp"), cfg)?;
        let rms_1 = RmsNorm::load(cfg.hidden_size, cfg.rms_norm_eps, vb.pp("input_layernorm"))?;
        let rms_2 = RmsNorm::load(
            cfg.hidden_size,
            cfg.rms_norm_eps,
            vb.pp("post_attention_layernorm"),
        )?;
        Ok(Self {
            rms_1,
            attn,
            rms_2,
            mlp,
        })
    }
}

// ---------------------------------------------------------------------------
// Custom Llama loader with public blocks
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct LlamaForAudit {
    wte: Embedding,
    pub blocks: Vec<Block>,
}

impl LlamaForAudit {
    fn load(vb: VarBuilder, cfg: &Config) -> Result<Self> {
        let wte = embedding(cfg.vocab_size, cfg.hidden_size, vb.pp("model.embed_tokens"))?;
        let blocks: Vec<_> = (0..cfg.num_hidden_layers)
            .map(|i| Block::load(vb.pp(format!("model.layers.{i}")), cfg).unwrap())
            .collect();
        Ok(Self { wte, blocks })
    }

    fn embed(&self, x: &Tensor) -> Result<Tensor> {
        self.wte.forward(x)
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct TensorAudit {
    model: LlamaForAudit,
    tokenizer: Tokenizer,
    device: Device,
    target_layer: usize,
    config: Config,
}

impl TensorAudit {
    /// Downloads (or uses cache) SmolLM2-135M via hf-hub and loads the model.
    pub fn load_smollm2(device: &Device, target_layer: usize) -> Result<Self> {
        let tokenizer_filename = download_file(MODEL_REPO, "tokenizer.json")?;
        let config_filename = download_file(MODEL_REPO, "config.json")?;
        let weights_filename = download_file(MODEL_REPO, "model.safetensors")?;

        let tokenizer = Tokenizer::from_file(tokenizer_filename)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?;
        let llama_config: LlamaConfig = serde_json::from_slice(&std::fs::read(config_filename)?)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?;
        let config = llama_config.into_config(false);

        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_filename], DType::F32, device)?
        };
        let model = LlamaForAudit::load(vb, &config)?;

        Ok(Self {
            model,
            tokenizer,
            device: device.clone(),
            target_layer,
            config,
        })
    }

    /// Extracts the hidden state tensor from the target layer.
    pub fn forward_extract(&self, prompt: &str) -> Result<Tensor> {
        let tokens = self
            .tokenizer
            .encode(prompt, true)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?;
        let tokens = tokens.get_ids();
        let input_tensor = Tensor::new(tokens, &self.device)?.unsqueeze(0)?;

        let mut cache = Cache::new(true, DType::F32, &self.config, &self.device)?;
        let mut x = self.model.embed(&input_tensor)?;

        for (i, block) in self.model.blocks.iter().enumerate() {
            x = block.forward(&x, 0, i, &mut cache)?;
            if i == self.target_layer {
                return Ok(x);
            }
        }

        Err(candle_core::Error::Msg(format!(
            "Target layer {} not found in model (has {} blocks)",
            self.target_layer,
            self.model.blocks.len()
        )))
    }

    /// Computes the TCM Z-axis from activation tensor.
    /// Computes the TCM Z-axis as **Max Absolute Z-score** from activation tensor.
    ///
    /// Z-score: Z = (X - μ) / σ
    /// Anomaly detection requires finding max(|Z|), not mean(Z) which always yields ~0.
    pub fn compute_tcm_z_axis(&self, activations: &Tensor) -> Result<f32> {
        let flat = activations.flatten_all()?;
        let mean = flat.mean_all()?;

        // Manual std: sqrt(mean((x - mean)^2))
        let variance = flat.broadcast_sub(&mean)?.sqr()?.mean_all()?;
        let std_dev = variance.sqrt()?;

        let z = flat
            .broadcast_sub(&mean)?
            .broadcast_div(&(std_dev + 1e-8)?)?;
        let z_vec = z.to_vec1::<f32>()?;
        let max_abs_z = z_vec.iter().fold(0.0_f32, |acc, &x| acc.max(x.abs()));
        Ok(max_abs_z)
    }

    /// Extracts the last token tensor from [1, seq_len, hidden_dim] → [hidden_dim].
    ///
    /// The last token in a causal LLM concentrates the full contextual representation,
    /// making it superior to mean pooling for semantic discrimination.
    pub fn extract_last_token(&self, hidden_state: &Tensor) -> Result<Tensor> {
        let seq_len = hidden_state.dim(1)?;
        hidden_state.narrow(1, seq_len - 1, 1)?.squeeze(1)
    }

    /// Computes Cosine Distance between two 1D tensors (1.0 - Cosine Similarity).
    ///
    /// Cosine similarity isolates *intention* (vector direction) from *syntax* (vector magnitude),
    /// solving the "curse of dimensionality" that plagued MSE-based approaches.
    /// Returns: 0.0 = identical, 1.0 = orthogonal, 2.0 = opposite.
    fn compute_cosine_distance(&self, t1: &Tensor, t2: &Tensor) -> Result<f32> {
        // Dot product (A · B)
        let dot_product = (t1 * t2)?.sum_all()?.to_scalar::<f32>()?;

        // Magnitudes (||A|| and ||B||)
        let norm1 = t1.sqr()?.sum_all()?.sqrt()?.to_scalar::<f32>()?;
        let norm2 = t2.sqr()?.sum_all()?.sqrt()?.to_scalar::<f32>()?;

        // Avoid division by zero
        let denominator = norm1 * norm2;
        let cos_sim = if denominator > 1e-8 {
            dot_product / denominator
        } else {
            0.0
        };

        // Cosine Distance
        Ok(1.0 - cos_sim)
    }

    /// **Moral Triangulation with Cosine Compass**: Returns the corruption ratio (D_safe / D_toxic).
    ///
    /// Uses Cosine Distance instead of MSE to isolate directional intent from syntactic magnitude.
    /// Ratio > 1.0 means closer to toxic anchor in latent direction space.
    pub fn compute_triangulated_z_axis(
        &self,
        test_tensor: &Tensor,
        safe_anchor: &Tensor,
        toxic_anchor: &Tensor,
    ) -> Result<f32> {
        let test_last = self.extract_last_token(test_tensor)?;
        let safe_last = self.extract_last_token(safe_anchor)?;
        let toxic_last = self.extract_last_token(toxic_anchor)?;

        let dist_safe = self.compute_cosine_distance(&test_last, &safe_last)?;
        let dist_toxic = self.compute_cosine_distance(&test_last, &toxic_last)?;

        // Directional Corruption Ratio: higher means closer to toxic anchor
        let ratio = dist_safe / (dist_toxic + 1e-8);
        Ok(ratio)
    }
}
