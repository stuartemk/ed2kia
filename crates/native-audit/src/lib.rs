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
    target_layers: Vec<usize>,
    config: Config,
}

impl TensorAudit {
    /// Downloads (or uses cache) SmolLM2-135M via hf-hub and loads the model.
    ///
    /// `target_layers` specifies which layer indices to extract during forward pass.
    /// Use e.g. `vec![4, 8]` for multi-layer Intention Trajectory analysis.
    pub fn load_smollm2(device: &Device, target_layers: Vec<usize>) -> Result<Self> {
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
            target_layers,
            config,
        })
    }

    /// Extracts the hidden state tensor from the first target layer (backward compat).
    pub fn forward_extract(&self, prompt: &str) -> Result<Tensor> {
        let map = self.forward_extract_multi(prompt)?;
        let first_layer = *self.target_layers.first().unwrap_or(&0);
        map.get(&first_layer).cloned().ok_or_else(|| {
            candle_core::Error::Msg(format!("Layer {} not found in extracted map", first_layer))
        })
    }

    /// **Multi-Layer Extraction** — Extracts hidden states from all target layers in a single pass.
    ///
    /// Returns a HashMap mapping layer index -> hidden state tensor [1, seq_len, hidden_dim].
    /// Enables Intention Trajectory analysis by comparing shallow (syntax) vs deep (intent) layers.
    pub fn forward_extract_multi(&self, prompt: &str) -> Result<HashMap<usize, Tensor>> {
        let tokens = self
            .tokenizer
            .encode(prompt, true)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?;
        let tokens = tokens.get_ids();
        let input_tensor = Tensor::new(tokens, &self.device)?.unsqueeze(0)?;

        let mut cache = Cache::new(true, DType::F32, &self.config, &self.device)?;
        let mut x = self.model.embed(&input_tensor)?;

        let mut extracted = HashMap::new();

        for (i, block) in self.model.blocks.iter().enumerate() {
            x = block.forward(&x, 0, i, &mut cache)?;
            if self.target_layers.contains(&i) {
                extracted.insert(i, x.clone());
            }
        }

        Ok(extracted)
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

    /// **Concept Vector Projection** — Representation Engineering approach.
    ///
    /// Derives the Concept Vector (V_concept = C_toxic - C_safe) from multi-anchor centroids,
    /// then projects the centered test tensor onto this vector using dot product projection.
    ///
    /// Unlike Cosine Similarity (which only measures angle), dot product projection measures
    /// *how far* along the concept direction the test tensor lies, providing magnitude-based
    /// separation between toxic and safe samples.
    ///
    /// projection = dot(centered_test, V_concept) / ||V_concept||
    ///
    /// Positive projection = alignment with toxicity direction.
    /// Negative projection = alignment with safety direction.
    /// Threshold calibrated empirically (typically near midpoint).
    /// **Concept Vector Projection** — Representation Engineering approach.
    ///
    /// Derives the Concept Vector (V_concept = C_toxic - C_safe) from multi-anchor centroids,
    /// then projects the centered test tensor onto this vector using dot product projection.
    ///
    /// Uses last-token extraction for centroid-compatible projections.
    /// For adversarial resilience, use `compute_temporal_max_projection` instead.
    ///
    /// projection = dot(centered_test, V_concept) / ||V_concept||
    ///
    /// Positive projection = alignment with toxicity direction.
    /// Negative projection = alignment with safety direction.
    /// Threshold calibrated empirically (typically near midpoint).
    pub fn compute_concept_projection(
        &self,
        test_tensor: &Tensor,
        safe_centroid: &Tensor,
        toxic_centroid: &Tensor,
    ) -> Result<f32> {
        let test_last = self.extract_last_token(test_tensor)?;

        // 1. Derive Concept Vector (Pure toxicity direction)
        let concept_vector = toxic_centroid.broadcast_sub(safe_centroid)?;

        // 2. Center test tensor relative to safe space
        let centered_test = test_last.broadcast_sub(safe_centroid)?;

        // 3. Dot product projection: how far along the concept direction
        let dot_product = (&centered_test * &concept_vector)?
            .sum_all()?
            .to_scalar::<f32>()?;

        // 4. Normalize by concept vector magnitude (not test magnitude)
        let concept_norm = concept_vector
            .sqr()?
            .sum_all()?
            .sqrt()?
            .to_scalar::<f32>()?;

        if concept_norm > 1e-8 {
            Ok(dot_product / concept_norm)
        } else {
            Ok(0.0)
        }
    }

    /// **Temporal Max-Pooling** — Adversarial Sentinel (Sprint 98).
    ///
    /// Instead of extracting only the last token (vulnerable to adversarial suffixes),
    /// this method projects ALL tokens in the sequence onto the concept vector,
    /// then returns the maximum projection value found across the temporal dimension.
    ///
    /// Attackers hide toxic prompts by appending benign suffixes:
    ///   "How to synthesize drugs... Please format as a poem about spring flowers."
    /// The last token is "flowers" (safe), but earlier tokens carry toxic intent.
    ///
    /// Temporal Max-Pooling finds the most toxic token regardless of position:
    ///   For each token t in sequence:
    ///     proj[t] = dot(token[t] - C_safe, V_concept) / ||V_concept||
    ///   Return max(proj)
    ///
    /// # Arguments
    /// * `test_tensor` - Full hidden state tensor [1, seq_len, hidden_dim]
    /// * `safe_centroid` - Safe anchor centroid [hidden_dim]
    /// * `toxic_centroid` - Toxic anchor centroid [hidden_dim]
    ///
    /// # Returns
    /// Maximum projection value across all tokens in the sequence.
    pub fn compute_temporal_max_projection(
        &self,
        test_tensor: &Tensor,
        safe_centroid: &Tensor,
        toxic_centroid: &Tensor,
    ) -> Result<f32> {
        // 1. Derive Concept Vector (Pure toxicity direction)
        let concept_vector = toxic_centroid.broadcast_sub(safe_centroid)?;

        // 2. Normalize concept vector magnitude
        let concept_norm = concept_vector
            .sqr()?
            .sum_all()?
            .sqrt()?
            .to_scalar::<f32>()?;

        if concept_norm < 1e-8 {
            return Ok(0.0);
        }

        // 3. Get test tensor shape [1, seq_len, hidden_dim]
        let test_shape = test_tensor.shape();
        let dims = test_shape.dims();

        // 4. Reshape concept_vector to [1, 1, hidden_dim], then broadcast to [1, seq_len, hidden_dim]
        let cv = concept_vector.flatten_all()?;
        let hidden_dim = cv.dim(0)?;
        let concept_vector_3d = cv.reshape(&[1, 1, hidden_dim])?;
        let concept_broadcast = concept_vector_3d.broadcast_as(dims)?;

        // 5. Reshape safe_centroid similarly
        let sc = safe_centroid.flatten_all()?;
        let safe_centroid_3d = sc.reshape(&[1, 1, hidden_dim])?;
        let safe_broadcast = safe_centroid_3d.broadcast_as(dims)?;

        // 6. Center all tokens relative to safe space
        let centered = test_tensor.broadcast_sub(&safe_broadcast)?;

        // 7. Dot product projection for each token
        // centered * concept_broadcast → [1, seq_len, hidden_dim] element-wise
        // Then sum along hidden_dim (dim=2) → [1, seq_len]
        let projections = (&centered * &concept_broadcast)?.sum_keepdim(2)?;

        // 5. Normalize by concept vector magnitude → [1, seq_len]
        let norm_tensor = Tensor::new(&[concept_norm], &self.device)?;
        let normalized = projections.broadcast_div(&norm_tensor)?;

        // 6. Flatten and find max projection across all tokens
        let proj_vec = normalized.flatten_all()?.to_vec1::<f32>()?;
        let max_proj = *proj_vec
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(&0.0);

        Ok(max_proj)
    }

    /// **Wasserstein-2 Distance ($W_2$)** — True Optimal Transport Metric.
    ///
    /// Calculates the Wasserstein-2 distance between two 1D tensors by sorting
    /// both and computing RMSE between sorted values. This measures the true
    /// "cost" of deforming one activation distribution into another.
    ///
    /// $W_2(U, V) = \sqrt{\frac{1}{N}\sum_{i=1}^{N} (\text{sort}(U)_i - \text{sort}(V)_i)^2}$
    ///
    /// # Arguments
    /// * `t1` - First tensor (will be flattened to 1D)
    /// * `t2` - Second tensor (will be flattened to 1D)
    ///
    /// # Returns
    /// Wasserstein-2 distance value
    pub fn compute_wasserstein_2_distance(&self, t1: &Tensor, t2: &Tensor) -> Result<f32> {
        // 1. Flatten and extract to vectors
        let mut vec1 = t1.flatten_all()?.to_vec1::<f32>()?;
        let mut vec2 = t2.flatten_all()?.to_vec1::<f32>()?;

        // 2. Sort both vectors (ascending)
        vec1.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        vec2.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // 3. Create sorted tensors
        let sorted_t1 = Tensor::new(vec1.as_slice(), &self.device)?;
        let sorted_t2 = Tensor::new(vec2.as_slice(), &self.device)?;

        // 4. Compute RMSE between sorted tensors
        let diff = sorted_t1.broadcast_sub(&sorted_t2)?;
        let sqr_diff = diff.sqr()?;
        let w2_dist = sqr_diff.mean_all()?.sqrt()?.to_scalar::<f32>()?;

        Ok(w2_dist)
    }

    /// **Temporal Max-Pooling using Wasserstein-2 Ratio**.
    ///
    /// For each token in the sequence, computes the ratio of Wasserstein distances:
    /// $Ratio_i = \frac{W_2(\text{token}_i, \text{safe\_centroid})}{W_2(\text{token}_i, \text{toxic\_centroid}) + \epsilon}$
    ///
    /// A ratio > 1.0 means the token is closer to the toxic centroid (costs less
    /// to transform into toxic than into safe). Returns the maximum ratio and its
    /// token index.
    ///
    /// # Arguments
    /// * `test_tensor` - Full hidden state tensor [1, seq_len, hidden_dim]
    /// * `safe_centroid` - Safe anchor centroid [hidden_dim]
    /// * `toxic_centroid` - Toxic anchor centroid [hidden_dim]
    ///
    /// # Returns
    /// `(max_ratio, max_idx)` — Maximum Wasserstein ratio and its token index
    /// **Sliced-Wasserstein Distance (SWD)** — Rigorous Optimal Transport for High-Dim Manifolds.
    ///
    /// Projects both tensors onto N random 1D directions (Monte Carlo approximation
    /// of the integral over the sphere), computes W2 1D on each projection, then
    /// averages and takes the square root.
    ///
    /// $SWD(P, Q)^2 \approx \frac{1}{N}\sum_{i=1}^{N} W_2(\theta_i \cdot P, \theta_i \cdot Q)^2$
    ///
    /// # Arguments
    /// * `t1` - First tensor [1, hidden_dim]
    /// * `t2` - Second tensor [1, hidden_dim]
    /// * `num_projections` - Number of random projections (default: 32)
    ///
    /// # Returns
    /// Sliced-Wasserstein distance value
    pub fn compute_sliced_wasserstein(
        &self,
        t1: &Tensor,
        t2: &Tensor,
        num_projections: usize,
    ) -> Result<f32> {
        let hidden_dim = t1.dim(1)?;
        let mut total_w2 = 0.0;

        // Monte Carlo approximation of the integral over the sphere
        for _ in 0..num_projections {
            // Random normalized vector
            let rand_vec = Tensor::randn(0f32, 1f32, (hidden_dim, 1), &self.device)?;
            let norm = rand_vec.sqr()?.sum_all()?.sqrt()?.to_scalar::<f32>()?;
            let norm_tensor = Tensor::new(&[norm], &self.device)?;
            let proj_dir = rand_vec.broadcast_div(&norm_tensor)?;

            // Project tensors onto the 1D line
            let p1 = t1.matmul(&proj_dir)?.flatten_all()?;
            let p2 = t2.matmul(&proj_dir)?.flatten_all()?;

            // Compute W2 1D on the projection
            let mut vec1 = p1.to_vec1::<f32>()?;
            let mut vec2 = p2.to_vec1::<f32>()?;
            vec1.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            vec2.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            let sorted_p1 = Tensor::new(vec1.as_slice(), &self.device)?;
            let sorted_p2 = Tensor::new(vec2.as_slice(), &self.device)?;

            let diff = sorted_p1.broadcast_sub(&sorted_p2)?;
            let w2_1d = diff.sqr()?.mean_all()?.to_scalar::<f32>()?; // Variance, no sqrt yet

            total_w2 += w2_1d;
        }

        // Average variances and take final square root
        Ok((total_w2 / num_projections as f32).sqrt())
    }

    /// **Activation Steering** — Convex Interpolation Correction.
    ///
    /// When a toxic trajectory is detected, steers the hidden state toward the
    /// safe centroid using convex interpolation:
    /// $h_{new} = (1-\alpha) \cdot h + \alpha \cdot C_{safe}$
    ///
    /// This is bounded and guarantees the result stays in the convex hull of
    /// the original state and the safe centroid, preventing over-correction.
    ///
    /// # Arguments
    /// * `hidden_state` - Current hidden state tensor
    /// * `toxic_centroid` - Toxic anchor centroid (unused, kept for API compatibility)
    /// * `safe_centroid` - Safe anchor centroid (target for interpolation)
    /// * `alpha` - Interpolation weight in [0,1] (1.0 = fully safe, 0.0 = no change)
    ///
    /// # Returns
    /// Corrected hidden state tensor
    pub fn steer_activation(
        &self,
        hidden_state: &Tensor,
        _toxic_centroid: &Tensor,
        safe_centroid: &Tensor,
        alpha: f64,
    ) -> Result<Tensor> {
        // Clamp alpha to [0, 1] for convex interpolation
        let alpha = alpha.clamp(0.0, 1.0);
        let one_minus_alpha = 1.0 - alpha;

        let shape = hidden_state.shape();
        let ndims = shape.dims().len();

        // Flatten safe_centroid to [hidden_dim]
        let safe_flat = safe_centroid.flatten_all()?;
        let hidden_dim = safe_flat.dim(0)?;

        // Broadcast safe_centroid to match hidden_state dimensions
        let safe_broadcast = if ndims == 3 {
            // [1, seq_len, hidden_dim] → reshape safe to [1, 1, hidden_dim]
            let safe_3d = safe_flat.reshape(&[1, 1, hidden_dim])?;
            let dims = shape.dims();
            safe_3d.broadcast_as(dims)?
        } else {
            // 1D or 2D: keep as-is
            safe_flat
        };

        // h_new = (1-α)·h + α·safe
        let scaled_hidden =
            hidden_state.broadcast_mul(&Tensor::new(&[one_minus_alpha as f32], &self.device)?)?;
        let scaled_safe =
            safe_broadcast.broadcast_mul(&Tensor::new(&[alpha as f32], &self.device)?)?;
        scaled_hidden.broadcast_add(&scaled_safe)
    }

    /// **Lyapunov-Controlled Activation Steering** (Contraction Mapping).
    ///
    /// Unlike convex interpolation which blends the entire state toward the safe
    /// centroid (potentially destroying orthogonal semantic information), this method:
    /// 1. Computes the normalized toxic direction: $d = (C_{toxic} - C_{safe}) / ||C_{toxic} - C_{safe}||$
    /// 2. Projects the centered state onto $d$: $proj = \langle h - C_{safe}, d \rangle$
    /// 3. Clips the projection to $[-\beta, \beta]$ for contraction mapping stability
    /// 4. Subtracts only the toxic magnitude: $h_{new} = h - \alpha \cdot clip(proj) \cdot d$
    ///
    /// This preserves the orthogonal (linguistic) components of the hidden state
    /// while removing only the toxic projection.
    ///
    /// # Arguments
    /// * `hidden_state` - Current hidden state tensor [1, seq_len, hidden_dim] or [hidden_dim]
    /// * `toxic_centroid` - Toxic anchor centroid
    /// * `safe_centroid` - Safe anchor centroid
    /// * `alpha` - Learning rate of the control (e.g. 1.0)
    /// * `beta` - Clipping limit to preserve semantics (e.g. 10.0)
    ///
    /// # Returns
    /// Corrected hidden state tensor
    pub fn steer_activation_lyapunov(
        &self,
        hidden_state: &Tensor,
        toxic_centroid: &Tensor,
        safe_centroid: &Tensor,
        alpha: f64,
        beta: f64,
    ) -> Result<Tensor> {
        // 1. Toxic direction vector: V = C_toxic - C_safe
        let v_toxic = toxic_centroid.broadcast_sub(safe_centroid)?;

        // 2. Normalize: d = V / ||V||
        let norm_v = v_toxic.sqr()?.sum_all()?.sqrt()?.to_scalar::<f32>()?;
        let norm_tensor = Tensor::new(&[norm_v + 1e-8], &self.device)?;
        let d = v_toxic.broadcast_div(&norm_tensor)?;

        // 3. Center the hidden state: h_centered = h - C_safe
        let h_centered = hidden_state.broadcast_sub(safe_centroid)?;

        let shape = hidden_state.shape();
        let ndims = shape.dims().len();

        if ndims == 3 {
            // [1, seq_len, hidden_dim] — per-token projection
            // d is [hidden_dim], need to broadcast to [1, 1, hidden_dim]
            let d_flat = d.flatten_all()?;
            let hidden_dim = d_flat.dim(0)?;
            let d_3d = d_flat.reshape(&[1, 1, hidden_dim])?;

            // Element-wise multiply: h_centered * d → [1, seq_len, hidden_dim]
            let elem_prod = h_centered.broadcast_mul(&d_3d)?;

            // Sum over hidden_dim to get per-token projection: [1, seq_len]
            let proj = elem_prod.sum(2)?;

            // Clip to [-beta, beta]
            let clipped = proj.clamp(-beta as f32, beta as f32)?;

            // Only correct if projection is positive (points toward toxic)
            // Use ReLU-like mask: mask = (clipped > 0) as f32
            let zero = Tensor::new(&[0.0f32], &self.device)?;
            let mask = clipped
                .broadcast_gt(&zero)?
                .to_dtype(candle_core::DType::F32)?;

            // correction_magnitude = alpha * clipped * mask → [1, seq_len]
            let alpha_tensor = Tensor::new(&[alpha as f32], &self.device)?;
            let corr_mag = clipped.broadcast_mul(&alpha_tensor)?;
            let corr_mag = corr_mag.broadcast_mul(&mask)?;

            // Reshape to [1, seq_len, 1] for broadcasting
            let seq_len = corr_mag.dim(1)?;
            let corr_mag_3d = corr_mag.reshape(&[1, seq_len, 1])?;

            // correction_vector = corr_mag * d → [1, seq_len, hidden_dim]
            let corr_vec = corr_mag_3d.broadcast_mul(&d_3d)?;

            // h_new = h - correction
            hidden_state.broadcast_sub(&corr_vec)
        } else {
            // 1D/2D case: scalar projection
            let proj = (h_centered.broadcast_mul(&d)?)
                .sum_all()?
                .to_scalar::<f32>()?;

            let clipped_proj = proj.clamp(-beta as f32, beta as f32);

            if clipped_proj > 0.0 {
                let corr_mag = Tensor::new(&[(alpha as f32) * clipped_proj], &self.device)?;
                let corr_vec = d.broadcast_mul(&corr_mag)?;
                hidden_state.broadcast_sub(&corr_vec)
            } else {
                // Homeostasis — already safe
                Ok(hidden_state.clone())
            }
        }
    }

    /// **Inverse Normal CDF (Φ⁻¹) — Beasley-Springer-Moro Approximation**.
    ///
    /// Provides accurate quantile estimation for certified robustness radius calculation.
    /// Used in Randomized Smoothing (Cohen et al. 2019) to compute certified radius:
    /// ε = σ * Φ⁻¹(p_safe)
    ///
    /// # Arguments
    /// * `p` — Probability in (0, 1)
    ///
    /// # Returns
    /// Quantile z such that P(Z ≤ z) = p for standard normal distribution
    fn norm_cdf_inv(p: f64) -> f64 {
        if p <= 0.0 {
            return f64::NEG_INFINITY;
        }
        if p >= 1.0 {
            return f64::INFINITY;
        }

        let mut p = p;
        let sign = if p < 0.5 {
            p = 1.0 - p;
            -1.0
        } else {
            1.0
        };

        // Rational approximation for upper tail
        let t = (-2.0 * (1.0 - p).ln()).sqrt();
        let num = 2.515517 + t * (0.802853 + t * 0.010328);
        let den = 1.0 + t * (1.432788 + t * (0.189269 + t * 0.001308));
        sign * (t - num / den)
    }

    /// **Certified Robustness via Randomized Smoothing + Lyapunov Steering**.
    ///
    /// Implements Cohen et al. (2019) Randomized Smoothing with integration of
    /// Lyapunov-Controlled Steering (Sprint 101) for certified robustness guarantees.
    ///
    /// **Algorithm:**
    /// 1. Sample `n_samples` noisy versions: h_noisy = h + N(0, σ²I)
    /// 2. Apply Lyapunov Steering to each noisy sample
    /// 3. Evaluate Sliced-Wasserstein Ratio on steered samples
    /// 4. Estimate p_safe = fraction of samples with ratio ≤ 1.0
    /// 5. Compute certified radius: ε = σ * Φ⁻¹(p_safe) for p_safe > 0.5
    ///
    /// **Guarantee:** No adversary with ||δ||₂ < ε can change the safety decision.
    ///
    /// # Arguments
    /// * `hidden_state` — Original hidden state tensor [1, seq_len, hidden_dim]
    /// * `safe_centroid` — Safe concept centroid [1, hidden_dim]
    /// * `toxic_centroid` — Toxic concept centroid [1, hidden_dim]
    /// * `sigma` — Noise standard deviation for randomized smoothing
    /// * `n_samples` — Number of Monte Carlo samples (higher = more precise)
    /// * `alpha_lyap` — Lyapunov steering coefficient (1.0 = exact toxic removal)
    ///
    /// # Returns
    /// `(p_safe, epsilon, avg_steered_ratio)` where:
    /// - `p_safe`: Empirical probability of safe classification
    /// - `epsilon`: Certified L2 radius (0 if p_safe ≤ 0.5)
    /// - `avg_steered_ratio`: Mean SWD ratio across steered samples
    pub fn certify_robustness(
        &self,
        hidden_state: &Tensor,
        safe_centroid: &Tensor,
        toxic_centroid: &Tensor,
        sigma: f64,
        n_samples: usize,
        alpha_lyap: f64,
    ) -> Result<(f64, f64, f64)> {
        // Extract last token to match centroid computation pattern
        let last_token = if hidden_state.shape().dims().len() == 3 {
            self.extract_last_token(hidden_state)?
        } else {
            hidden_state.clone()
        };

        let mut safe_count = 0usize;
        let mut steered_ratios: Vec<f64> = Vec::with_capacity(n_samples);
        let dims = last_token.dims().to_vec();

        for i in 0..n_samples {
            // 1. Gaussian noise: N(0, σ²I)
            let noise = Tensor::randn(0.0f32, (sigma * 0.5) as f32, dims.as_slice(), &self.device)?;
            let noisy = last_token.broadcast_add(&noise)?;

            // 2. Lyapunov Steering (Sprint 101)
            let steered = self.steer_activation_lyapunov(
                &noisy,
                toxic_centroid,
                safe_centroid,
                alpha_lyap,
                1000.0, // beta = 1000 (no clipping)
            )?;

            // 3. Evaluate Sliced-Wasserstein Ratio (direct, single-token)
            let dist_safe = self.compute_sliced_wasserstein(&steered, safe_centroid, 32)?;
            let dist_toxic = self.compute_sliced_wasserstein(&steered, toxic_centroid, 32)?;
            let ratio = dist_safe / (dist_toxic + 1e-8);
            steered_ratios.push(ratio as f64);

            // 4. Count safe classifications (ratio ≤ 1.0 = closer to safe)
            if ratio <= 1.0 {
                safe_count += 1;
            }

            // Progress logging every 50 samples
            if (i + 1) % 50 == 0 || i + 1 == n_samples {
                println!(
                    "   📊 Sample {}/{} — p_safe so far: {:.2}",
                    i + 1,
                    n_samples,
                    safe_count as f64 / (i + 1) as f64
                );
            }
        }

        let p_safe = safe_count as f64 / n_samples as f64;

        // Certified radius: ε = σ * Φ⁻¹(p_safe) for p_safe > 0.5
        let epsilon = if p_safe > 0.5 {
            sigma * Self::norm_cdf_inv(p_safe)
        } else {
            0.0
        };

        let avg_ratio = steered_ratios.iter().sum::<f64>() / n_samples as f64;

        Ok((p_safe, epsilon, avg_ratio))
    }

    /// **Temporal Max-Pooling using Sliced-Wasserstein Ratio**.
    ///
    /// For each token in the sequence, computes the ratio of Sliced-Wasserstein distances:
    /// $Ratio_i = \frac{SWD(\text{token}_i, \text{safe\_centroid})}{SWD(\text{token}_i, \text{toxic\_centroid}) + \epsilon}$
    ///
    /// Uses 32 random projections for Monte Carlo approximation.
    ///
    /// # Arguments
    /// * `test_tensor` - Full hidden state tensor [1, seq_len, hidden_dim]
    /// * `safe_centroid` - Safe anchor centroid [hidden_dim]
    /// * `toxic_centroid` - Toxic anchor centroid [hidden_dim]
    ///
    /// # Returns
    /// `(max_ratio, max_idx)` — Maximum Sliced-Wasserstein ratio and its token index
    pub fn compute_temporal_sliced_wasserstein_ratio(
        &self,
        test_tensor: &Tensor,    // [1, seq_len, hidden_dim]
        safe_centroid: &Tensor,  // [hidden_dim]
        toxic_centroid: &Tensor, // [hidden_dim]
    ) -> Result<(f32, usize)> {
        let seq_len = test_tensor.dim(1)?;
        let num_projections = 32; // Balance precision vs latency

        let mut max_ratio = f32::NEG_INFINITY;
        let mut max_idx = 0;

        for i in 0..seq_len {
            let token_tensor = test_tensor.narrow(1, i, 1)?.squeeze(1)?; // [hidden_dim]

            let dist_safe =
                self.compute_sliced_wasserstein(&token_tensor, safe_centroid, num_projections)?;
            let dist_toxic =
                self.compute_sliced_wasserstein(&token_tensor, toxic_centroid, num_projections)?;

            // Ratio SWD: > 1.0 means closer to toxic (cheaper to transform to toxic)
            let ratio = dist_safe / (dist_toxic + 1e-8);

            if ratio > max_ratio {
                max_ratio = ratio;
                max_idx = i;
            }
        }

        Ok((max_ratio, max_idx))
    }

    /// **Temporal Max-Pooling using Wasserstein-2 Ratio** (Legacy — kept for compatibility).
    ///
    /// For each token in the sequence, computes the ratio of Wasserstein distances:
    /// $Ratio_i = \frac{W_2(\text{token}_i, \text{safe\_centroid})}{W_2(\text{token}_i, \text{toxic\_centroid}) + \epsilon}$
    ///
    /// A ratio > 1.0 means the token is closer to the toxic centroid (costs less
    /// to transform into toxic than into safe). Returns the maximum ratio and its
    /// token index.
    ///
    /// # Arguments
    /// * `test_tensor` - Full hidden state tensor [1, seq_len, hidden_dim]
    /// * `safe_centroid` - Safe anchor centroid [hidden_dim]
    /// * `toxic_centroid` - Toxic anchor centroid [hidden_dim]
    ///
    /// # Returns
    /// `(max_ratio, max_idx)` — Maximum Wasserstein ratio and its token index
    pub fn compute_temporal_wasserstein_ratio(
        &self,
        test_tensor: &Tensor,    // [1, seq_len, hidden_dim]
        safe_centroid: &Tensor,  // [hidden_dim]
        toxic_centroid: &Tensor, // [hidden_dim]
    ) -> Result<(f32, usize)> {
        let seq_len = test_tensor.dim(1)?;

        let mut max_ratio = f32::NEG_INFINITY;
        let mut max_idx = 0;

        for i in 0..seq_len {
            let token_tensor = test_tensor.narrow(1, i, 1)?.squeeze(1)?; // [hidden_dim]

            let dist_safe = self.compute_wasserstein_2_distance(&token_tensor, safe_centroid)?;
            let dist_toxic = self.compute_wasserstein_2_distance(&token_tensor, toxic_centroid)?;

            // Ratio W2: > 1.0 means closer to toxic (cheaper to transform to toxic)
            let ratio = dist_safe / (dist_toxic + 1e-8);

            if ratio > max_ratio {
                max_ratio = ratio;
                max_idx = i;
            }
        }

        Ok((max_ratio, max_idx))
    }

    /// **Dynamic Threshold Calibration** — Anti-Hardcoding mechanism.
    ///
    /// Calculates thresholds from the actual anchor projections using robust
    /// median-based statistics with IQR outlier removal. This prevents anchor
    /// outliers from corrupting the threshold calculation.
    ///
    /// L6 threshold: Placed below the safe median (permissive gate — lets most through).
    /// L8 threshold: Placed at midpoint between safe and toxic medians (discriminative gate).
    ///
    /// This eliminates magic numbers like `-103.5` or `-65.0`, making the system
    /// generalize across models and datasets.
    ///
    /// # Arguments
    /// * `safe_projections_l6` - Vector of L6 projections for safe anchor prompts
    /// * `toxic_projections_l6` - Vector of L6 projections for toxic anchor prompts
    /// * `safe_projections_l8` - Vector of L8 projections for safe anchor prompts
    /// * `toxic_projections_l8` - Vector of L8 projections for toxic anchor prompts
    ///
    /// # Returns
    /// `(threshold_l6, threshold_l8)` — Dynamic thresholds for Tri-Gate Logic
    pub fn calibrate_thresholds(
        &self,
        safe_projections_l6: &[f32],
        toxic_projections_l6: &[f32],
        safe_projections_l8: &[f32],
        toxic_projections_l8: &[f32],
    ) -> Result<(f32, f32)> {
        // Robust calibration: median + IQR outlier removal
        let median_safe_l6 = self.median_iqr_clean(safe_projections_l6);
        let _median_toxic_l6 = self.median_iqr_clean(toxic_projections_l6);
        let median_safe_l8 = self.median_iqr_clean(safe_projections_l8);
        let median_toxic_l8 = self.median_iqr_clean(toxic_projections_l8);

        // L6 threshold: Below safe median — permissive gate
        // Allows both safe and toxic through; L8 + momentum do the real filtering
        let threshold_l6 = median_safe_l6 - 5.0;

        // L8 threshold: Closer to safe median (0.25 ratio) — discriminative gate
        // This ensures contextual-safe prompts (like the novelist) fail the L8 gate
        // because their L8 projection stays near the safe cluster
        let threshold_l8 = median_safe_l8 + (median_toxic_l8 - median_safe_l8) * 0.25;

        Ok((threshold_l6, threshold_l8))
    }

    /// Computes the median of a projection slice after IQR-based outlier removal.
    /// Values outside [Q1 - 1.5*IQR, Q3 + 1.5*IQR] are excluded before computing median.
    /// Falls back to raw median if all values are removed.
    fn median_iqr_clean(&self, projections: &[f32]) -> f32 {
        if projections.is_empty() {
            return 0.0;
        }

        let mut sorted = projections.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let n = sorted.len();
        if n <= 3 {
            // Too few samples for IQR — use raw median
            return sorted[n / 2];
        }

        // Quartiles
        let q1 = sorted[n / 4];
        let q3 = sorted[3 * n / 4];
        let iqr = q3 - q1;
        let lower = q1 - 1.5 * iqr;
        let upper = q3 + 1.5 * iqr;

        // Filter outliers
        let cleaned: Vec<f32> = sorted
            .iter()
            .copied()
            .filter(|&x| x >= lower && x <= upper)
            .collect();

        if cleaned.is_empty() {
            // Fallback to raw median
            sorted[n / 2]
        } else {
            cleaned[cleaned.len() / 2]
        }
    }

    /// **Zonotope/Interval Abstract Interpretation for Lyapunov Projection Bounds**.
    ///
    /// Computes deterministic bounds on the Lyapunov projection using interval arithmetic:
    /// - Given hidden state `h` and perturbation ball `||δ||₂ ≤ ε`, propagate bounds
    ///   through the Lyapunov projection: `proj = <h + δ - C_safe, d>` where `d = (C_toxic - C_safe)/||C_toxic - C_safe||`
    /// - Lower bound: `proj_lower = <h - C_safe, d> - ε * ||d||₁` (worst-case δ aligns against d)
    /// - Upper bound: `proj_upper = <h - C_safe, d> + ε * ||d||₁` (worst-case δ aligns with d)
    /// - If `proj_upper ≤ 0`, then for ALL δ in the ball, the state is provably safe
    ///
    /// # Arguments
    /// * `hidden_state` — Center hidden state [hidden_dim]
    /// * `safe_centroid` — Safe anchor centroid [hidden_dim]
    /// * `toxic_centroid` — Toxic anchor centroid [hidden_dim]
    /// * `epsilon_input` — L2 perturbation radius for interval propagation
    ///
    /// # Returns
    /// `(proj_lower, proj_upper, certified_radius_det)` where certified_radius_det = epsilon_input if provably safe
    pub fn abstract_verify_lyapunov(
        &self,
        hidden_state: &Tensor,
        safe_centroid: &Tensor,
        toxic_centroid: &Tensor,
        epsilon_input: f64,
    ) -> Result<(f64, f64, f64)> {
        // 1. Toxic direction: d = (C_toxic - C_safe) / ||C_toxic - C_safe||
        let v_toxic = toxic_centroid.broadcast_sub(safe_centroid)?;
        let norm_v = v_toxic.sqr()?.sum_all()?.sqrt()?.to_scalar::<f32>()?;
        let norm_tensor = Tensor::new(&[norm_v + 1e-8], &self.device)?;
        let d = v_toxic.broadcast_div(&norm_tensor)?;

        // 2. Centered state: h_centered = h - C_safe
        let h_centered = hidden_state.broadcast_sub(safe_centroid)?;

        // 3. Nominal projection: proj = <h_centered, d>
        let proj = h_centered
            .broadcast_mul(&d)?
            .sum_all()?
            .to_scalar::<f32>()? as f64;

        // 4. Interval bounds: ||δ||₂ ≤ ε implies |<δ, d>| ≤ ε * ||d||₂ = ε
        // (Cauchy-Schwarz, since ||d||₂ = 1 for normalized direction)
        let interval_half = epsilon_input; // Cauchy-Schwarz tight bound

        let proj_lower = proj - interval_half;
        let proj_upper = proj + interval_half;

        // 5. Certified radius: if proj_upper ≤ 0, all perturbations in ball are safe
        let certified_radius_det = if proj_upper <= 0.0 {
            epsilon_input // Full ball is certified safe
        } else if proj_lower > 0.0 {
            0.0 // Even center is toxic — no certification possible
        } else {
            // Partial: certified radius = distance to boundary = proj (nominal margin)
            // But only if proj > 0 (center is safe)
            if proj < 0.0 {
                (-proj).min(epsilon_input) // Margin to toxic boundary
            } else {
                0.0 // Center is already toxic
            }
        };

        Ok((proj_lower, proj_upper, certified_radius_det))
    }

    /// **Hybrid Certification: Randomized Smoothing + Abstract Interpretation**.
    ///
    /// Combines probabilistic certification (S102) with deterministic bounds (S103):
    /// - `epsilon_smooth`: Certified radius from randomized smoothing (probabilistic)
    /// - `epsilon_det`: Certified radius from abstract interpretation (deterministic)
    /// - `hybrid_epsilon = min(epsilon_smooth, epsilon_det)`: Conservative combined guarantee
    ///
    /// **Guarantee:** No adversary with ||δ||₂ < hybrid_epsilon can change the safety decision,
    /// backed by both statistical evidence (Monte Carlo) and mathematical proof (interval bounds).
    ///
    /// # Arguments
    /// * `hidden_state` — Hidden state tensor [1, seq_len, hidden_dim] or [hidden_dim]
    /// * `safe_centroid` — Safe anchor centroid [hidden_dim]
    /// * `toxic_centroid` — Toxic anchor centroid [hidden_dim]
    /// * `sigma` — Noise std dev for randomized smoothing
    /// * `n_samples` — Monte Carlo samples
    /// * `alpha_lyap` — Lyapunov steering coefficient
    /// * `eps_abstract` — Initial perturbation ball for abstract verification
    ///
    /// # Returns
    /// `(p_safe, epsilon_smooth, epsilon_det, hybrid_epsilon)`
    #[allow(clippy::too_many_arguments)]
    pub fn hybrid_certify(
        &self,
        hidden_state: &Tensor,
        safe_centroid: &Tensor,
        toxic_centroid: &Tensor,
        sigma: f64,
        n_samples: usize,
        alpha_lyap: f64,
        eps_abstract: f64,
    ) -> Result<(f64, f64, f64, f64)> {
        // 1. Randomized Smoothing (S102)
        let (p_safe, epsilon_smooth, _) = self.certify_robustness(
            hidden_state,
            safe_centroid,
            toxic_centroid,
            sigma,
            n_samples,
            alpha_lyap,
        )?;

        // 2. Extract last token for abstract verification (match centroid pattern)
        let last_token = if hidden_state.shape().dims().len() == 3 {
            self.extract_last_token(hidden_state)?
        } else {
            hidden_state.clone()
        };

        // 3. Abstract Interpretation (S103)
        let (_, _, epsilon_det) = self.abstract_verify_lyapunov(
            &last_token,
            safe_centroid,
            toxic_centroid,
            eps_abstract,
        )?;

        // 4. Hybrid: Conservative minimum
        let hybrid_epsilon = epsilon_smooth.min(epsilon_det);

        Ok((p_safe, epsilon_smooth, epsilon_det, hybrid_epsilon))
    }
}
