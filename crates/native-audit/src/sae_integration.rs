//! SAE Integration — Deep Mechanistic Interpretability via Sparse Autoencoders
//!
//! Provides SAE feature extraction, steering, and interpretability tools
//! for disentangled latent concept manipulation.

use candle_core::IndexOp;
use candle_core::{Device, Result, Tensor};

#[cfg(test)]
use candle_core::DType;

/// Sparse Autoencoder (SAE) stub for mechanistic interpretability.
///
/// In production, this loads a pre-trained SAE dictionary (e.g., from OpenAI's
/// GPT-2 SAE or a custom-trained model). For now, we use an analytical stub
/// that generates interpretable features via random projections + sparsity.
#[derive(Debug, Clone)]
pub struct SAEConfig {
    pub hidden_dim: usize,
    pub feature_dim: usize,
    pub top_k: usize,
}

impl Default for SAEConfig {
    fn default() -> Self {
        Self {
            hidden_dim: 576, // SmolLM2-135M hidden size
            feature_dim: 2048,
            top_k: 32,
        }
    }
}

/// SAE feature activation with interpretability metadata.
#[derive(Debug, Clone)]
pub struct SAEFeature {
    pub index: usize,
    pub activation: f32,
    pub name: String,
    pub category: FeatureCategory,
}

/// Semantic category for SAE features.
#[derive(Debug, Clone, PartialEq)]
pub enum FeatureCategory {
    Harmful,
    Helpful,
    Neutral,
    Deception,
    Safety,
    Unknown,
}

impl std::fmt::Display for FeatureCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeatureCategory::Harmful => write!(f, "harmful"),
            FeatureCategory::Helpful => write!(f, "helpful"),
            FeatureCategory::Neutral => write!(f, "neutral"),
            FeatureCategory::Deception => write!(f, "deception"),
            FeatureCategory::Safety => write!(f, "safety"),
            FeatureCategory::Unknown => write!(f, "unknown"),
        }
    }
}

/// Sparse Autoencoder for mechanistic interpretability.
pub struct SparseAutoencoder {
    pub config: SAEConfig,
    pub device: Device,
    /// Feature names for interpretability (stub: generated from index patterns).
    pub feature_names: Vec<String>,
    /// Feature categories for steering decisions.
    pub feature_categories: Vec<FeatureCategory>,
}

impl SparseAutoencoder {
    /// Create a new SAE with stub dictionary.
    pub fn new(config: SAEConfig, device: &Device) -> Self {
        let n_features = config.feature_dim;
        let (feature_names, feature_categories) = generate_feature_stub(n_features);
        Self {
            config,
            device: device.clone(),
            feature_names,
            feature_categories,
        }
    }

    /// Extract SAE features from hidden state via sparse coding.
    ///
    /// Uses ReLU-based sparse coding: `features = ReLU(hidden @ W_enc - threshold)`
    /// Returns top-k active features with interpretability metadata.
    pub fn extract_features(&self, hidden: &Tensor) -> Result<Vec<SAEFeature>> {
        let (batch, seq, dim) = match hidden.shape().dims3() {
            Ok(d) => d,
            Err(_) => {
                // Try 2D: [seq, dim]
                let (_, dim) = hidden.shape().dims2()?;
                if dim != self.config.hidden_dim {
                    return Err(candle_core::Error::Msg(format!(
                        "Expected hidden_dim={}, got {}",
                        self.config.hidden_dim, dim
                    )));
                }
                return self.extract_features_2d(hidden);
            }
        };

        // Flatten to [batch*seq, dim] for processing
        let flat = hidden.flatten(0, 1)?;
        let n_tokens = batch * seq;

        // Generate random projection matrix as stub encoder
        // In production, this would be loaded from pre-trained weights
        let projection = self.random_projection(dim, self.config.feature_dim)?;

        // Sparse coding: features = ReLU(h @ W_enc)
        let raw_features = flat.matmul(&projection)?;
        let features = raw_features.relu()?;

        // Extract top-k features per token
        let mut all_features = Vec::new();
        for token_idx in 0..n_tokens {
            let token_features = features.i(token_idx)?;
            let values: Vec<f32> = token_features.to_vec1()?;

            // Get top-k indices
            let mut indexed: Vec<(usize, f32)> = values.into_iter().enumerate().collect();
            indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            let top_k = indexed
                .iter()
                .take(self.config.top_k)
                .cloned()
                .collect::<Vec<_>>();

            for (idx, activation) in top_k {
                if activation > 1e-6 {
                    all_features.push(SAEFeature {
                        index: idx,
                        activation,
                        name: self.feature_names[idx].clone(),
                        category: self.feature_categories[idx].clone(),
                    });
                }
            }
        }

        Ok(all_features)
    }

    /// Extract features from 2D tensor [seq, dim].
    fn extract_features_2d(&self, hidden: &Tensor) -> Result<Vec<SAEFeature>> {
        let (seq, dim) = hidden.shape().dims2()?;
        let projection = self.random_projection(dim, self.config.feature_dim)?;
        let raw_features = hidden.matmul(&projection)?;
        let features = raw_features.relu()?;

        let mut all_features = Vec::new();
        for token_idx in 0..seq {
            let token_features = features.i(token_idx)?;
            let values: Vec<f32> = token_features.to_vec1()?;

            let mut indexed: Vec<(usize, f32)> = values.into_iter().enumerate().collect();
            indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            let top_k = indexed
                .iter()
                .take(self.config.top_k)
                .cloned()
                .collect::<Vec<_>>();

            for (idx, activation) in top_k {
                if activation > 1e-6 {
                    all_features.push(SAEFeature {
                        index: idx,
                        activation,
                        name: self.feature_names[idx].clone(),
                        category: self.feature_categories[idx].clone(),
                    });
                }
            }
        }

        Ok(all_features)
    }

    /// Generate random projection matrix as stub encoder.
    fn random_projection(&self, input_dim: usize, output_dim: usize) -> Result<Tensor> {
        // Use orthogonal-ish projection for stability
        // In production: load from pre-trained SAE weights
        let scale = 1.0 / (input_dim as f32).sqrt();
        let mut data = Vec::with_capacity(input_dim * output_dim);
        let seed = 42u64;
        for i in 0..(input_dim * output_dim) {
            // Simple deterministic pseudo-random for reproducibility
            let x = ((seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(i as u64))
                >> 33) as f32
                / (u32::MAX as f32);
            data.push((x * 2.0 - 1.0) * scale);
        }
        Tensor::from_vec(data, (input_dim, output_dim), &self.device)?
            .reshape((input_dim, output_dim))
    }

    /// Compute feature activation statistics.
    pub fn feature_statistics(&self, features: &[SAEFeature]) -> SAEStatistics {
        let mut harmful_sum = 0.0f32;
        let mut harmful_count = 0usize;
        let mut helpful_sum = 0.0f32;
        let mut helpful_count = 0usize;
        let mut safety_sum = 0.0f32;
        let mut safety_count = 0usize;
        let mut deception_sum = 0.0f32;
        let mut deception_count = 0usize;

        for f in features {
            match f.category {
                FeatureCategory::Harmful => {
                    harmful_sum += f.activation;
                    harmful_count += 1;
                }
                FeatureCategory::Helpful => {
                    helpful_sum += f.activation;
                    helpful_count += 1;
                }
                FeatureCategory::Safety => {
                    safety_sum += f.activation;
                    safety_count += 1;
                }
                FeatureCategory::Deception => {
                    deception_sum += f.activation;
                    deception_count += 1;
                }
                _ => {}
            }
        }

        SAEStatistics {
            total_features: features.len(),
            harmful_avg: if harmful_count > 0 {
                harmful_sum / harmful_count as f32
            } else {
                0.0
            },
            helpful_avg: if helpful_count > 0 {
                helpful_sum / helpful_count as f32
            } else {
                0.0
            },
            safety_avg: if safety_count > 0 {
                safety_sum / safety_count as f32
            } else {
                0.0
            },
            deception_avg: if deception_count > 0 {
                deception_sum / deception_count as f32
            } else {
                0.0
            },
            harmful_count,
            helpful_count,
            safety_count,
            deception_count,
        }
    }

    /// Steer hidden state by suppressing toxic features and amplifying safe ones.
    pub fn steer_features(
        &self,
        hidden: &Tensor,
        features: &[SAEFeature],
        suppression_weight: f32,
        amplification_weight: f32,
    ) -> Result<Tensor> {
        // Build steering vector from feature activations
        let mut steering_data: Vec<f32> = vec![0.0; self.config.hidden_dim];

        for f in features {
            let adjustment = match f.category {
                FeatureCategory::Harmful | FeatureCategory::Deception => {
                    -suppression_weight * f.activation
                }
                FeatureCategory::Helpful | FeatureCategory::Safety => {
                    amplification_weight * f.activation
                }
                _ => 0.0,
            };

            // Distribute adjustment across hidden dimensions (stub: uniform)
            let dim_idx = f.index % self.config.hidden_dim;
            steering_data[dim_idx] += adjustment;
        }

        let steering = Tensor::from_vec(steering_data, (self.config.hidden_dim,), &self.device)?;
        // Broadcast steering vector to match hidden state shape
        let dims: Vec<usize> = hidden.shape().dims().iter().map(|&d| d.max(1)).collect();
        let steering_broadcast = match dims.len() {
            3 => steering.broadcast_as((dims[0], dims[1], dims[2]))?,
            2 => steering.broadcast_as((dims[0], dims[1]))?,
            _ => steering.clone(),
        };
        hidden.add(&steering_broadcast)
    }
}

/// Statistics summary for SAE feature activations.
#[derive(Debug, Clone)]
pub struct SAEStatistics {
    pub total_features: usize,
    pub harmful_avg: f32,
    pub helpful_avg: f32,
    pub safety_avg: f32,
    pub deception_avg: f32,
    pub harmful_count: usize,
    pub helpful_count: usize,
    pub safety_count: usize,
    pub deception_count: usize,
}

impl SAEStatistics {
    /// Compute harm ratio: harmful / (helpful + safety + 1e-8).
    pub fn harm_ratio(&self) -> f32 {
        self.harmful_avg / (self.helpful_avg + self.safety_avg + 1e-8)
    }

    /// Compute safety score: (helpful + safety) / (harmful + deception + 1e-8).
    pub fn safety_score(&self) -> f32 {
        (self.helpful_avg + self.safety_avg) / (self.harmful_avg + self.deception_avg + 1e-8)
    }
}

/// Generate stub feature names and categories for interpretability.
fn generate_feature_stub(n: usize) -> (Vec<String>, Vec<FeatureCategory>) {
    let harm_keywords = [
        "harm_intent",
        "violence",
        "toxicity",
        "abuse",
        "threat",
        "malice",
        "destruction",
        "poison",
        "weapon",
        "exploit",
    ];
    let help_keywords = [
        "helpfulness",
        "cooperation",
        "empathy",
        "support",
        "guidance",
        "kindness",
        "prosocial",
        "care",
        "assist",
        "protect",
    ];
    let safety_keywords = [
        "safety_check",
        "guardrail",
        "boundary",
        "consent",
        "ethical",
        "responsible",
        "harmless",
        "secure",
        "compliance",
        "moderation",
    ];
    let deception_keywords = [
        "deception",
        "manipulation",
        "lie",
        "mislead",
        "obfuscate",
        "sycophancy",
        "evasion",
        "double_speak",
        "gaslight",
        "conceal",
    ];
    let neutral_keywords = [
        "factual",
        "neutral",
        "descriptive",
        "analytical",
        "objective",
        "informational",
        "explanatory",
        "contextual",
        "referential",
        "structural",
    ];

    let mut names = Vec::with_capacity(n);
    let mut categories = Vec::with_capacity(n);

    for i in 0..n {
        let (name, category) = match i % 5 {
            0 => (
                format!("sae_{}_{}", i, harm_keywords[i % harm_keywords.len()]),
                FeatureCategory::Harmful,
            ),
            1 => (
                format!("sae_{}_{}", i, help_keywords[i % help_keywords.len()]),
                FeatureCategory::Helpful,
            ),
            2 => (
                format!("sae_{}_{}", i, safety_keywords[i % safety_keywords.len()]),
                FeatureCategory::Safety,
            ),
            3 => (
                format!(
                    "sae_{}_{}",
                    i,
                    deception_keywords[i % deception_keywords.len()]
                ),
                FeatureCategory::Deception,
            ),
            _ => (
                format!("sae_{}_{}", i, neutral_keywords[i % neutral_keywords.len()]),
                FeatureCategory::Neutral,
            ),
        };
        names.push(name);
        categories.push(category);
    }

    (names, categories)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sae_feature_extraction() {
        let device = Device::Cpu;
        let sae = SparseAutoencoder::new(SAEConfig::default(), &device);

        // Create stub hidden state [1, 4, 576]
        let hidden = Tensor::zeros((1, 4, 576), DType::F32, &device).unwrap();
        let features = sae.extract_features(&hidden).unwrap();

        // Extraction succeeded if we reach here (unwrap above would have panicked)
        // For zero input, features may be empty (all activations below threshold)
        let _ = features.len();
    }

    #[test]
    fn test_sae_statistics() {
        let stats = SAEStatistics {
            total_features: 100,
            harmful_avg: 0.5,
            helpful_avg: 2.0,
            safety_avg: 1.5,
            deception_avg: 0.1,
            harmful_count: 10,
            helpful_count: 30,
            safety_count: 25,
            deception_count: 5,
        };

        assert!(stats.harm_ratio() < 1.0);
        assert!(stats.safety_score() > 1.0);
    }
}
