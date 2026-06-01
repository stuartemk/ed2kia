//! Universal Feature Dictionary — Cross-model feature merging with Lyapunov stability.
//!
//! Implements FedAvg weighted by CE×SCT-Z, contrastive disentanglement
//! to prevent feature collapse, and Lyapunov contraction verification.

use std::collections::HashMap;

/// Error types for Universal Feature Dictionary operations.
#[derive(Debug, Clone, PartialEq)]
pub enum MergeError {
    /// Empty feature set provided.
    EmptyFeatures,
    /// Lyapunov contraction factor exceeded threshold (>= 0.95).
    LyapunovDivergence { gamma: f64 },
    /// Invalid weight (negative or NaN).
    InvalidWeight { weight: f64 },
}

impl std::fmt::Display for MergeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MergeError::EmptyFeatures => write!(f, "empty feature set provided"),
            MergeError::LyapunovDivergence { gamma } => {
                write!(f, "Lyapunov divergence: gamma={:.4} >= 0.95", gamma)
            }
            MergeError::InvalidWeight { weight } => {
                write!(f, "invalid weight: {:.4}", weight)
            }
        }
    }
}

impl std::error::Error for MergeError {}

/// Configuration for cross-model feature merging.
#[derive(Debug, Clone)]
pub struct MergeConfig {
    /// Lyapunov contraction threshold (must be < 1.0).
    pub lyapunov_threshold: f64,
    /// Contrastive regularization strength.
    pub contrastive_lambda: f64,
    /// Maximum features in dictionary.
    pub max_features: usize,
}

impl MergeConfig {
    pub fn default_stuartian() -> Self {
        Self {
            lyapunov_threshold: 0.95,
            contrastive_lambda: 0.1,
            max_features: 10_000,
        }
    }
}

impl Default for MergeConfig {
    fn default() -> Self {
        Self::default_stuartian()
    }
}

/// A single feature entry in the universal dictionary.
#[derive(Debug, Clone)]
pub struct FeatureEntry {
    /// Unique feature identifier.
    pub id: u64,
    /// Feature vector (activation pattern).
    pub vector: Vec<f64>,
    /// Source model identifier.
    pub source_model: String,
    /// Existential Credit weight.
    pub ce_weight: f64,
    /// SCT-Z calibration.
    pub z_axis: f64,
    /// GEI fingerprint (8 dimensions).
    pub gei: [f64; 8],
}

impl FeatureEntry {
    pub fn new(
        id: u64,
        vector: Vec<f64>,
        source_model: String,
        ce_weight: f64,
        z_axis: f64,
        gei: [f64; 8],
    ) -> Result<Self, MergeError> {
        if ce_weight < 0.0 || ce_weight.is_nan() {
            return Err(MergeError::InvalidWeight { weight: ce_weight });
        }
        Ok(Self {
            id,
            vector,
            source_model,
            ce_weight,
            z_axis,
            gei,
        })
    }

    /// Compute FedAvg weight: (CE/1000) * (1 + clamp(Z, -0.5, 0.5))
    pub fn fedavg_weight(&self) -> f64 {
        (self.ce_weight / 1000.0) * (1.0 + self.z_axis.clamp(-0.5, 0.5))
    }
}

/// Universal Feature Dictionary — merges features across models.
#[derive(Debug, Clone)]
pub struct UniversalFeatureDict {
    entries: HashMap<u64, FeatureEntry>,
    config: MergeConfig,
}

impl UniversalFeatureDict {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            config: MergeConfig::default(),
        }
    }

    pub fn with_config(config: MergeConfig) -> Self {
        Self {
            entries: HashMap::new(),
            config,
        }
    }

    /// Add a feature entry to the dictionary.
    pub fn add(&mut self, entry: FeatureEntry) -> Result<(), MergeError> {
        if self.entries.len() >= self.config.max_features {
            return Err(MergeError::EmptyFeatures); // Dictionary full
        }
        self.entries.insert(entry.id, entry);
        Ok(())
    }

    /// Merge features from multiple models using FedAvg weighted by CE×Z.
    pub fn merge(&self, feature_ids: &[u64]) -> Result<Vec<f64>, MergeError> {
        if feature_ids.is_empty() {
            return Err(MergeError::EmptyFeatures);
        }

        let mut total_weight = 0.0;
        let mut merged = Vec::new();

        // Get vector dimension from first entry
        let first = self
            .entries
            .get(&feature_ids[0])
            .ok_or(MergeError::EmptyFeatures)?;
        let dim = first.vector.len();
        merged.resize(dim, 0.0);

        // Weighted accumulation
        for &id in feature_ids {
            let entry = self.entries.get(&id).ok_or(MergeError::EmptyFeatures)?;
            let weight = entry.fedavg_weight();
            total_weight += weight;

            for (i, val) in entry.vector.iter().enumerate() {
                merged[i] += val * weight;
            }
        }

        // Normalize
        if total_weight > 0.0 {
            for val in merged.iter_mut() {
                *val /= total_weight;
            }
        }

        Ok(merged)
    }

    /// Compute Lyapunov contraction factor between two merged states.
    pub fn lyapunov_contraction(&self, prev: &[f64], next: &[f64], equilibrium: &[f64]) -> f64 {
        let dist_prev = Self::euclidean_distance(prev, equilibrium);
        let dist_next = Self::euclidean_distance(next, equilibrium);

        if dist_prev < 1e-10 {
            return 0.0;
        }
        dist_next / dist_prev
    }

    /// Verify Lyapunov stability (gamma < threshold).
    pub fn verify_stability(
        &self,
        prev: &[f64],
        next: &[f64],
        equilibrium: &[f64],
    ) -> Result<f64, MergeError> {
        let gamma = self.lyapunov_contraction(prev, next, equilibrium);
        if gamma >= self.config.lyapunov_threshold {
            return Err(MergeError::LyapunovDivergence { gamma });
        }
        Ok(gamma)
    }

    /// Apply contrastive disentanglement: penalize features too similar to existing.
    pub fn contrastive_penalty(&self, new_vector: &[f64], existing_id: u64) -> f64 {
        let existing = match self.entries.get(&existing_id) {
            Some(e) => e,
            None => return 0.0,
        };

        let similarity = Self::cosine_similarity(new_vector, &existing.vector);
        self.config.contrastive_lambda * similarity
    }

    /// Get feature count.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if dictionary is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn euclidean_distance(a: &[f64], b: &[f64]) -> f64 {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f64>()
            .sqrt()
    }

    fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
        let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f64 = a.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
        let norm_b: f64 = b.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();

        if norm_a < 1e-10 || norm_b < 1e-10 {
            return 0.0;
        }
        dot / (norm_a * norm_b)
    }
}

impl Default for UniversalFeatureDict {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for UniversalFeatureDict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "UniversalFeatureDict {{ entries: {}, config: {:?} }}",
            self.entries.len(),
            self.config
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(id: u64, vector: Vec<f64>, ce: f64, z: f64) -> FeatureEntry {
        FeatureEntry::new(id, vector, format!("model_{}", id), ce, z, [0.0; 8]).unwrap()
    }

    #[test]
    fn test_feature_entry_creation() {
        let entry = make_entry(1, vec![1.0, 2.0, 3.0], 500.0, 0.3);
        assert_eq!(entry.id, 1);
        assert_eq!(entry.vector, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_feature_entry_invalid_weight() {
        let result = FeatureEntry::new(1, vec![1.0], "m".to_string(), -1.0, 0.0, [0.0; 8]);
        assert!(result.is_err());
    }

    #[test]
    fn test_fedavg_weight() {
        let entry = make_entry(1, vec![1.0], 1000.0, 0.0);
        assert!((entry.fedavg_weight() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_dict_add_and_merge() {
        let mut dict = UniversalFeatureDict::new();
        dict.add(make_entry(1, vec![1.0, 0.0], 1000.0, 0.0))
            .unwrap();
        dict.add(make_entry(2, vec![0.0, 1.0], 1000.0, 0.0))
            .unwrap();

        let merged = dict.merge(&[1, 2]).unwrap();
        assert!((merged[0] - 0.5).abs() < 1e-9);
        assert!((merged[1] - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_merge_empty() {
        let dict = UniversalFeatureDict::new();
        assert!(dict.merge(&[]).is_err());
    }

    #[test]
    fn test_lyapunov_contraction_converging() {
        let dict = UniversalFeatureDict::new();
        let prev = vec![1.0, 1.0];
        let next = vec![0.5, 0.5];
        let eq = vec![0.0, 0.0];
        let gamma = dict.lyapunov_contraction(&prev, &next, &eq);
        assert!(gamma < 0.95);
    }

    #[test]
    fn test_lyapunov_stability_verified() {
        let dict = UniversalFeatureDict::new();
        let prev = vec![1.0, 1.0];
        let next = vec![0.1, 0.1];
        let eq = vec![0.0, 0.0];
        let gamma = dict.verify_stability(&prev, &next, &eq).unwrap();
        assert!(gamma < 0.95);
    }

    #[test]
    fn test_lyapunov_divergence_rejected() {
        let dict = UniversalFeatureDict::new();
        let prev = vec![0.1, 0.1];
        let next = vec![1.0, 1.0];
        let eq = vec![0.0, 0.0];
        assert!(dict.verify_stability(&prev, &next, &eq).is_err());
    }

    #[test]
    fn test_contrastive_penalty_identical() {
        let mut dict = UniversalFeatureDict::new();
        dict.add(make_entry(1, vec![1.0, 0.0], 1000.0, 0.0))
            .unwrap();
        let penalty = dict.contrastive_penalty(&[1.0, 0.0], 1);
        assert!((penalty - 0.1).abs() < 1e-9); // lambda * similarity(1.0)
    }

    #[test]
    fn test_contrastive_penalty_orthogonal() {
        let mut dict = UniversalFeatureDict::new();
        dict.add(make_entry(1, vec![1.0, 0.0], 1000.0, 0.0))
            .unwrap();
        let penalty = dict.contrastive_penalty(&[0.0, 1.0], 1);
        assert!(penalty < 1e-9);
    }

    #[test]
    fn test_dict_display() {
        let dict = UniversalFeatureDict::new();
        let s = format!("{}", dict);
        assert!(s.contains("UniversalFeatureDict"));
    }

    #[test]
    fn test_default_config() {
        let config = MergeConfig::default();
        assert!((config.lyapunov_threshold - 0.95).abs() < 1e-9);
    }
}
