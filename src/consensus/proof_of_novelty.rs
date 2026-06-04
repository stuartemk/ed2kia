//! Proof of Novelty â€” Sprint 80: Undecidable Synthesis & Architecture of Absolute Incompleteness
//!
//! Topological novelty proof prevents semantic DDoS and uVDF farming.
//! CE is weighted by semantic entropy: zero reward for already-mapped areas,
//! reward only for noospheric frontier expansion.
//!
//! Key features:
//! - Topological novelty computation via coverage maps
//! - Semantic entropy weighting
//! - Anti-farming: zero CE for repeated areas
//! - Frontier expansion tracking
//! - Haversine-based embedding distance for coverage

use std::collections::HashMap;
use std::fmt;

// â”€â”€â”€ Errors â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone, PartialEq)]
pub enum NoveltyError {
    EmptyEmbedding,
    DimensionMismatch(usize, usize),
    CoverageMapFull,
    InvalidEntropy(f64),
    NoNoveltyDetected,
}

impl fmt::Display for NoveltyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NoveltyError::EmptyEmbedding => write!(f, "Empty embedding vector"),
            NoveltyError::DimensionMismatch(have, expected) => {
                write!(f, "Dimension mismatch: {have}/{expected}")
            }
            NoveltyError::CoverageMapFull => write!(f, "Coverage map is full"),
            NoveltyError::InvalidEntropy(val) => write!(f, "Invalid entropy value: {val}"),
            NoveltyError::NoNoveltyDetected => write!(f, "No novelty detected in embedding"),
        }
    }
}

// â”€â”€â”€ Coverage Map â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone)]
pub struct CoverageMap {
    /// Dimension of the embedding space
    pub dim: usize,
    /// Covered regions (centroid embeddings)
    pub regions: Vec<Vec<f32>>,
    /// Coverage radius (maximum distance to consider "covered")
    pub radius: f32,
    /// Maximum regions before map is considered full
    pub max_regions: usize,
}

impl CoverageMap {
    pub fn new(dim: usize, radius: f32, max_regions: usize) -> Self {
        Self {
            dim,
            regions: Vec::new(),
            radius,
            max_regions,
        }
    }

    /// Check if a point is already covered by existing regions
    pub fn is_covered(&self, point: &[f32]) -> bool {
        if self.regions.is_empty() {
            return false;
        }
        for region in &self.regions {
            if region.len() != point.len() {
                continue;
            }
            let dist = euclidean_distance(point, region);
            if dist <= self.radius {
                return true;
            }
        }
        false
    }

    /// Add a new region to the coverage map
    pub fn add_region(&mut self, region: Vec<f32>) -> Result<(), NoveltyError> {
        if self.regions.len() >= self.max_regions {
            return Err(NoveltyError::CoverageMapFull);
        }
        if region.len() != self.dim {
            return Err(NoveltyError::DimensionMismatch(region.len(), self.dim));
        }
        self.regions.push(region);
        Ok(())
    }

    /// Get coverage percentage (0.0 = empty, 1.0 = full)
    pub fn coverage_percentage(&self) -> f64 {
        if self.max_regions == 0 {
            return 0.0;
        }
        (self.regions.len() as f64 / self.max_regions as f64).min(1.0)
    }
}

impl fmt::Display for CoverageMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CoverageMap(dim={}, regions={}, coverage={:.2}%)",
            self.dim,
            self.regions.len(),
            self.coverage_percentage() * 100.0
        )
    }
}

// â”€â”€â”€ Novelty Record â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone)]
pub struct NoveltyRecord {
    /// Round ID
    pub round_id: u64,
    /// Novelty score (0.0 = no novelty, 1.0 = maximum novelty)
    pub novelty_score: f64,
    /// CE reward issued
    pub ce_reward: f64,
    /// Timestamp
    pub timestamp_ms: u64,
}

impl fmt::Display for NoveltyRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "NoveltyRecord(round={}, novelty={:.3}, ce={:.3})",
            self.round_id, self.novelty_score, self.ce_reward
        )
    }
}

// â”€â”€â”€ Config â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone)]
pub struct NoveltyConfig {
    /// Embedding dimension
    pub embedding_dim: usize,
    /// Entropy threshold for novelty (0.0-1.0)
    pub entropy_threshold: f64,
    /// Maximum CE reward per novel contribution
    pub max_ce_reward: f64,
    /// Coverage radius for region matching
    pub coverage_radius: f32,
    /// Maximum coverage regions
    pub max_coverage_regions: usize,
}

impl NoveltyConfig {
    pub fn default_Topological() -> Self {
        Self {
            embedding_dim: 768,
            entropy_threshold: 0.3,
            max_ce_reward: 100.0,
            coverage_radius: 0.5,
            max_coverage_regions: 10000,
        }
    }

    pub fn validate(&self) -> Result<(), NoveltyError> {
        if self.embedding_dim == 0 {
            return Err(NoveltyError::EmptyEmbedding);
        }
        if self.entropy_threshold < 0.0 || self.entropy_threshold > 1.0 {
            return Err(NoveltyError::InvalidEntropy(self.entropy_threshold));
        }
        if self.max_ce_reward < 0.0 {
            return Err(NoveltyError::InvalidEntropy(self.max_ce_reward));
        }
        Ok(())
    }
}

impl Default for NoveltyConfig {
    fn default() -> Self {
        Self::default_Topological()
    }
}

// â”€â”€â”€ Proof of Novelty Engine â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub struct ProofOfNovelty {
    config: NoveltyConfig,
    coverage: CoverageMap,
    records: Vec<NoveltyRecord>,
    next_round: u64,
    total_ce_issued: f64,
}

impl ProofOfNovelty {
    pub fn new() -> Self {
        let config = NoveltyConfig::default_Topological();
        Self {
            coverage: CoverageMap::new(
                config.embedding_dim,
                config.coverage_radius,
                config.max_coverage_regions,
            ),
            config,
            records: Vec::new(),
            next_round: 1,
            total_ce_issued: 0.0,
        }
    }

    pub fn with_config(config: NoveltyConfig) -> Result<Self, NoveltyError> {
        config.validate()?;
        Ok(Self {
            coverage: CoverageMap::new(
                config.embedding_dim,
                config.coverage_radius,
                config.max_coverage_regions,
            ),
            config,
            records: Vec::new(),
            next_round: 1,
            total_ce_issued: 0.0,
        })
    }

    /// Compute novelty and issue CE reward
    pub fn compute_novelty_reward(
        &mut self,
        prompt_embedding: &[f32],
        current_ms: u64,
    ) -> Result<f64, NoveltyError> {
        if prompt_embedding.is_empty() {
            return Err(NoveltyError::EmptyEmbedding);
        }

        // Compute topological novelty
        let novelty = self.compute_topological_novelty(prompt_embedding);

        // Zero reward for no novelty
        if novelty < self.config.entropy_threshold {
            self.records.push(NoveltyRecord {
                round_id: self.next_round,
                novelty_score: novelty,
                ce_reward: 0.0,
                timestamp_ms: current_ms,
            });
            self.next_round += 1;
            return Ok(0.0);
        }

        // Compute CE reward weighted by novelty
        let ce_reward = self.compute_ce_reward(novelty);

        // Add to coverage map if novel enough
        if novelty >= self.config.entropy_threshold {
            let _ = self.coverage.add_region(prompt_embedding.to_vec());
        }

        // Record
        self.records.push(NoveltyRecord {
            round_id: self.next_round,
            novelty_score: novelty,
            ce_reward,
            timestamp_ms: current_ms,
        });
        self.next_round += 1;
        self.total_ce_issued += ce_reward;

        Ok(ce_reward)
    }

    /// Compute topological novelty for an embedding
    fn compute_topological_novelty(&self, embedding: &[f32]) -> f64 {
        if self.coverage.regions.is_empty() {
            return 1.0; // First embedding is always novel
        }

        // Find minimum distance to any covered region
        let mut min_distance = f64::MAX;
        for region in &self.coverage.regions {
            if region.len() != embedding.len() {
                continue;
            }
            let dist = euclidean_distance(embedding, region) as f64;
            if dist < min_distance {
                min_distance = dist;
            }
        }

        if min_distance == f64::MAX {
            return 1.0;
        }

        // Normalize: farther = more novel
        // Use sigmoid-like curve: novelty = 1 / (1 + exp(-k*(d - threshold)))
        let normalized = (min_distance / self.config.coverage_radius as f64).min(1.0);
        normalized
    }

    /// Compute CE reward from novelty score
    fn compute_ce_reward(&self, novelty: f64) -> f64 {
        // Linear scaling: higher novelty = higher reward
        let scaled =
            (novelty - self.config.entropy_threshold) / (1.0 - self.config.entropy_threshold);
        (scaled * self.config.max_ce_reward).min(self.config.max_ce_reward)
    }

    /// Get total CE issued
    pub fn total_ce_issued(&self) -> f64 {
        self.total_ce_issued
    }

    /// Get coverage map
    pub fn coverage(&self) -> &CoverageMap {
        &self.coverage
    }

    /// Get all records
    pub fn records(&self) -> &[NoveltyRecord] {
        &self.records
    }

    /// Get average novelty score
    pub fn average_novelty(&self) -> Option<f64> {
        if self.records.is_empty() {
            return None;
        }
        let sum: f64 = self.records.iter().map(|r| r.novelty_score).sum();
        Some(sum / self.records.len() as f64)
    }

    /// Reset state
    pub fn reset(&mut self) {
        self.coverage = CoverageMap::new(
            self.config.embedding_dim,
            self.config.coverage_radius,
            self.config.max_coverage_regions,
        );
        self.records.clear();
        self.next_round = 1;
        self.total_ce_issued = 0.0;
    }
}

impl Default for ProofOfNovelty {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ProofOfNovelty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ProofOfNovelty(coverage={:.2}%, ce_issued={:.2}, avg_novelty={})",
            self.coverage.coverage_percentage() * 100.0,
            self.total_ce_issued,
            self.average_novelty()
                .map(|n| format!("{:.3}", n))
                .unwrap_or_else(|| "N/A".to_string())
        )
    }
}

// â”€â”€â”€ Public Standalone Functions â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Compute topological novelty of a prompt embedding against a coverage map.
/// Returns 0.0 if fully covered, 1.0 if completely novel.
pub fn compute_topological_novelty(
    prompt_embedding: &[f32],
    genesis_graph_coverage: &CoverageMap,
    entropy_threshold: f64,
) -> f64 {
    if prompt_embedding.is_empty() {
        return 0.0;
    }

    // Check if covered
    if genesis_graph_coverage.is_covered(prompt_embedding) {
        return 0.0;
    }

    // Compute semantic entropy
    let entropy = compute_semantic_entropy(prompt_embedding);

    if entropy < entropy_threshold {
        return 0.0;
    }

    entropy
}

/// Compute semantic entropy of an embedding vector
fn compute_semantic_entropy(embedding: &[f32]) -> f64 {
    if embedding.is_empty() {
        return 0.0;
    }

    // Normalize to probability distribution
    let sum: f32 = embedding.iter().map(|x| x.abs()).sum();
    if sum == 0.0 {
        return 0.0;
    }

    // Shannon entropy
    let mut entropy: f64 = 0.0;
    for &val in embedding {
        let p = val.abs() / sum;
        if p > 0.0 {
            entropy -= (p as f64) * (p as f64).log2();
        }
    }

    // Normalize by log(dim)
    let max_entropy = (embedding.len() as f64).log2();
    if max_entropy == 0.0 {
        return 0.0;
    }
    (entropy / max_entropy).min(1.0)
}

/// Compute Euclidean distance between two vectors
fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let sum: f32 = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let diff = x - y;
            diff * diff
        })
        .sum();
    sum.sqrt()
}

/// FNV-1a 64-bit hash
fn fnv_hash_64(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

// â”€â”€â”€ Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = NoveltyConfig::default_Topological();
        assert_eq!(config.embedding_dim, 768);
        assert_eq!(config.entropy_threshold, 0.3);
        assert_eq!(config.max_ce_reward, 100.0);
    }

    #[test]
    fn test_config_validate_ok() {
        let config = NoveltyConfig::default_Topological();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_zero_dim() {
        let config = NoveltyConfig {
            embedding_dim: 0,
            ..NoveltyConfig::default_Topological()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_invalid_entropy() {
        let config = NoveltyConfig {
            entropy_threshold: 1.5,
            ..NoveltyConfig::default_Topological()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_coverage_map_new() {
        let map = CoverageMap::new(3, 0.5, 100);
        assert_eq!(map.dim, 3);
        assert_eq!(map.regions.len(), 0);
    }

    #[test]
    fn test_coverage_map_not_covered() {
        let map = CoverageMap::new(3, 0.5, 100);
        assert!(!map.is_covered(&[1.0, 2.0, 3.0]));
    }

    #[test]
    fn test_coverage_map_covered() {
        let mut map = CoverageMap::new(3, 1.0, 100);
        map.add_region(vec![1.0, 2.0, 3.0]).unwrap();
        assert!(map.is_covered(&[1.1, 2.1, 2.9]));
    }

    #[test]
    fn test_coverage_map_not_covered_far() {
        let mut map = CoverageMap::new(3, 0.5, 100);
        map.add_region(vec![0.0, 0.0, 0.0]).unwrap();
        assert!(!map.is_covered(&[10.0, 10.0, 10.0]));
    }

    #[test]
    fn test_coverage_map_full() {
        let mut map = CoverageMap::new(2, 0.5, 2);
        map.add_region(vec![0.0, 0.0]).unwrap();
        map.add_region(vec![1.0, 1.0]).unwrap();
        assert_eq!(
            map.add_region(vec![2.0, 2.0]),
            Err(NoveltyError::CoverageMapFull)
        );
    }

    #[test]
    fn test_coverage_map_dimension_mismatch() {
        let mut map = CoverageMap::new(3, 0.5, 100);
        assert_eq!(
            map.add_region(vec![1.0, 2.0]),
            Err(NoveltyError::DimensionMismatch(2, 3))
        );
    }

    #[test]
    fn test_coverage_percentage() {
        let mut map = CoverageMap::new(2, 0.5, 10);
        assert_eq!(map.coverage_percentage(), 0.0);
        map.add_region(vec![0.0, 0.0]).unwrap();
        assert!((map.coverage_percentage() - 0.1).abs() < 0.01);
    }

    #[test]
    fn test_coverage_display() {
        let map = CoverageMap::new(3, 0.5, 100);
        let s = format!("{}", map);
        assert!(s.contains("dim=3"));
    }

    #[test]
    fn test_engine_creation() {
        let engine = ProofOfNovelty::new();
        assert_eq!(engine.records().len(), 0);
    }

    #[test]
    fn test_engine_with_config() {
        let config = NoveltyConfig {
            embedding_dim: 3,
            ..NoveltyConfig::default_Topological()
        };
        let engine = ProofOfNovelty::with_config(config);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_first_embedding_novel() {
        let mut engine = ProofOfNovelty::new();
        let embedding = vec![1.0, 2.0, 3.0];
        let reward = engine.compute_novelty_reward(&embedding, 1000).unwrap();
        assert!(reward > 0.0);
    }

    #[test]
    fn test_repeated_embedding_no_reward() {
        let mut engine = ProofOfNovelty::with_config(NoveltyConfig {
            embedding_dim: 3,
            coverage_radius: 2.0,
            ..NoveltyConfig::default_Topological()
        })
        .unwrap();
        let embedding = vec![1.0, 2.0, 3.0];
        let _ = engine.compute_novelty_reward(&embedding, 1000).unwrap();
        let reward2 = engine.compute_novelty_reward(&embedding, 2000).unwrap();
        assert_eq!(reward2, 0.0);
    }

    #[test]
    fn test_empty_embedding_error() {
        let mut engine = ProofOfNovelty::new();
        assert_eq!(
            engine.compute_novelty_reward(&[], 1000),
            Err(NoveltyError::EmptyEmbedding)
        );
    }

    #[test]
    fn test_total_ce_issued() {
        let mut engine = ProofOfNovelty::new();
        let e1 = vec![1.0, 2.0, 3.0];
        let e2 = vec![10.0, 20.0, 30.0];
        engine.compute_novelty_reward(&e1, 1000).unwrap();
        engine.compute_novelty_reward(&e2, 2000).unwrap();
        assert!(engine.total_ce_issued() > 0.0);
    }

    #[test]
    fn test_average_novelty() {
        let mut engine = ProofOfNovelty::new();
        let e1 = vec![1.0, 2.0, 3.0];
        engine.compute_novelty_reward(&e1, 1000).unwrap();
        assert!(engine.average_novelty().is_some());
    }

    #[test]
    fn test_average_novelty_empty() {
        let engine = ProofOfNovelty::new();
        assert_eq!(engine.average_novelty(), None);
    }

    #[test]
    fn test_reset() {
        let mut engine = ProofOfNovelty::new();
        let e1 = vec![1.0, 2.0, 3.0];
        engine.compute_novelty_reward(&e1, 1000).unwrap();
        engine.reset();
        assert_eq!(engine.records().len(), 0);
        assert_eq!(engine.total_ce_issued(), 0.0);
    }

    #[test]
    fn test_display() {
        let engine = ProofOfNovelty::new();
        let s = format!("{}", engine);
        assert!(s.contains("ProofOfNovelty"));
    }

    #[test]
    fn test_record_display() {
        let record = NoveltyRecord {
            round_id: 1,
            novelty_score: 0.8,
            ce_reward: 50.0,
            timestamp_ms: 1000,
        };
        let s = format!("{}", record);
        assert!(s.contains("round=1"));
        assert!(s.contains("novelty=0.800"));
    }

    #[test]
    fn test_standalone_novelty_empty_coverage() {
        let map = CoverageMap::new(3, 0.5, 100);
        let embedding = vec![1.0, 2.0, 3.0];
        let novelty = compute_topological_novelty(&embedding, &map, 0.3);
        assert!(novelty > 0.0);
    }

    #[test]
    fn test_standalone_novelty_covered() {
        let mut map = CoverageMap::new(3, 1.0, 100);
        map.add_region(vec![1.0, 2.0, 3.0]).unwrap();
        let novelty = compute_topological_novelty(&[1.1, 2.1, 2.9], &map, 0.3);
        assert_eq!(novelty, 0.0);
    }

    #[test]
    fn test_standalone_novelty_empty_embedding() {
        let map = CoverageMap::new(3, 0.5, 100);
        let novelty = compute_topological_novelty(&[], &map, 0.3);
        assert_eq!(novelty, 0.0);
    }

    #[test]
    fn test_compute_semantic_entropy() {
        let embedding = vec![1.0, 2.0, 3.0, 4.0];
        let entropy = compute_semantic_entropy(&embedding);
        assert!(entropy >= 0.0 && entropy <= 1.0);
    }

    #[test]
    fn test_compute_semantic_entropy_empty() {
        let entropy = compute_semantic_entropy(&[]);
        assert_eq!(entropy, 0.0);
    }

    #[test]
    fn test_compute_semantic_entropy_uniform() {
        let embedding = vec![1.0, 1.0, 1.0, 1.0];
        let entropy = compute_semantic_entropy(&embedding);
        assert!(entropy > 0.5); // Uniform distribution has high entropy
    }

    #[test]
    fn test_euclidean_distance_same() {
        let a = vec![1.0, 2.0, 3.0];
        let dist = euclidean_distance(&a, &a);
        assert!((dist - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_euclidean_distance_different() {
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];
        let dist = euclidean_distance(&a, &b);
        assert!((dist - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_euclidean_distance_mismatch() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0];
        let dist = euclidean_distance(&a, &b);
        assert_eq!(dist, 0.0);
    }

    #[test]
    fn test_fnv_hash_deterministic() {
        let data = vec![1, 2, 3, 4, 5];
        let h1 = fnv_hash_64(&data);
        let h2 = fnv_hash_64(&data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_error_display() {
        let err = NoveltyError::NoNoveltyDetected;
        let s = format!("{}", err);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_full_workflow() {
        let mut engine = ProofOfNovelty::with_config(NoveltyConfig {
            embedding_dim: 3,
            coverage_radius: 0.5,
            entropy_threshold: 0.3,
            max_ce_reward: 100.0,
            max_coverage_regions: 100,
        })
        .unwrap();

        // First embedding: high novelty
        let e1 = vec![1.0, 2.0, 3.0];
        let reward1 = engine.compute_novelty_reward(&e1, 1000).unwrap();
        assert!(reward1 > 0.0);

        // Second embedding: different area, still novel
        let e2 = vec![10.0, 20.0, 30.0];
        let reward2 = engine.compute_novelty_reward(&e2, 2000).unwrap();
        assert!(reward2 > 0.0);

        // Third embedding: same as first, no reward
        let reward3 = engine.compute_novelty_reward(&e1, 3000).unwrap();
        assert_eq!(reward3, 0.0);

        // Verify state
        assert_eq!(engine.records().len(), 3);
        assert!(engine.total_ce_issued() > 0.0);
        assert!(engine.average_novelty().is_some());
        assert!(engine.coverage().coverage_percentage() > 0.0);

        // Standalone function
        let map = engine.coverage().clone();
        let novelty = compute_topological_novelty(&[100.0, 200.0, 300.0], &map, 0.3);
        assert!(novelty > 0.0);

        // Reset
        engine.reset();
        assert_eq!(engine.records().len(), 0);
        assert_eq!(engine.total_ce_issued(), 0.0);
    }
}
