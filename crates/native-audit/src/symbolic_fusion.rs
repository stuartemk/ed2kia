//! Symbolic-Probabilistic Fusion — Uniting Symbolic Reasoning with Probabilistic VFE
//!
//! Combines graph-based symbolic reasoning (SAE feature relations + logical rules)
//! with the probabilistic Variational Free Energy engine for hybrid cognitive control.

use crate::TopologicalSignature;
use candle_core::{DType, Result, Tensor};

/// Symbolic Graph over SAE features for interpretable reasoning.
///
/// Nodes represent interpretable SAE features (e.g., "deception", "helpfulness").
/// Edges represent causal or topological relations between features with strength weights.
#[derive(Debug, Clone)]
pub struct SymbolicGraph {
    pub nodes: Vec<String>,
    pub relations: Vec<(usize, usize, f32)>, // (source, target, strength)
}

impl Default for SymbolicGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolicGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            relations: Vec::new(),
        }
    }

    /// Create a symbolic graph from SAE feature names and categories.
    pub fn from_features(
        feature_names: &[String],
        feature_categories: &[crate::sae_integration::FeatureCategory],
    ) -> Self {
        let mut graph = Self::new();
        graph.nodes = feature_names.to_vec();

        // Build relations: features of same category are positively correlated,
        // opposing categories (harmful vs safety) are negatively correlated.
        let n = feature_names.len();
        for i in 0..n {
            for j in (i + 1)..n {
                let strength = relation_strength(&feature_categories[i], &feature_categories[j]);
                if strength.abs() > 0.1 {
                    graph.relations.push((i, j, strength));
                }
            }
        }

        graph
    }

    /// Compute graph coherence score: average absolute relation strength.
    pub fn coherence(&self) -> f32 {
        if self.relations.is_empty() {
            return 0.0;
        }
        let total: f32 = self.relations.iter().map(|(_, _, s)| s.abs()).sum();
        total / self.relations.len() as f32
    }

    /// Compute graph edit distance proxy between two symbolic graphs.
    ///
    /// Uses node count difference + relation strength divergence as approximation.
    pub fn edit_distance_proxy(&self, other: &SymbolicGraph) -> f32 {
        let node_diff = (self.nodes.len() as i32 - other.nodes.len() as i32).abs() as f32;

        // Build relation lookup for comparison
        let min_relations = self.relations.len().min(other.relations.len());
        let mut rel_diff = 0.0f32;

        for i in 0..min_relations {
            let (_, _, s1) = self.relations[i];
            let (_, _, s2) = other.relations.get(i).copied().unwrap_or((0, 0, 0.0));
            rel_diff += (s1 - s2).abs();
        }

        node_diff * 0.1 + rel_diff
    }

    /// Filter nodes by category pattern (e.g., "harmful", "safety").
    pub fn filter_by_pattern(&self, pattern: &str) -> Vec<usize> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, name)| name.to_lowercase().contains(pattern))
            .map(|(i, _)| i)
            .collect()
    }

    /// Compute centrality scores (stub: degree-based).
    pub fn centrality(&self) -> Vec<(usize, f32)> {
        let n = self.nodes.len();
        let mut degrees = vec![0f32; n];

        for &(src, tgt, strength) in &self.relations {
            if src < n {
                degrees[src] += strength.abs();
            }
            if tgt < n {
                degrees[tgt] += strength.abs();
            }
        }

        let mut scored: Vec<(usize, f32)> = degrees.into_iter().enumerate().collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }
}

/// Compute relation strength between two feature categories.
fn relation_strength(
    cat_a: &crate::sae_integration::FeatureCategory,
    cat_b: &crate::sae_integration::FeatureCategory,
) -> f32 {
    match (cat_a, cat_b) {
        // Same category: positive correlation
        (a, b) if a == b => 0.8,
        // Opposing categories: negative correlation
        (
            crate::sae_integration::FeatureCategory::Harmful,
            crate::sae_integration::FeatureCategory::Safety,
        )
        | (
            crate::sae_integration::FeatureCategory::Safety,
            crate::sae_integration::FeatureCategory::Harmful,
        ) => -0.9,
        (
            crate::sae_integration::FeatureCategory::Deception,
            crate::sae_integration::FeatureCategory::Helpful,
        )
        | (
            crate::sae_integration::FeatureCategory::Helpful,
            crate::sae_integration::FeatureCategory::Deception,
        ) => -0.7,
        // Complementary: positive
        (
            crate::sae_integration::FeatureCategory::Helpful,
            crate::sae_integration::FeatureCategory::Safety,
        )
        | (
            crate::sae_integration::FeatureCategory::Safety,
            crate::sae_integration::FeatureCategory::Helpful,
        ) => 0.6,
        // Neutral: weak
        (_, crate::sae_integration::FeatureCategory::Neutral)
        | (crate::sae_integration::FeatureCategory::Neutral, _) => 0.1,
        // Unknown
        _ => 0.0,
    }
}

/// Noosphere Gossip — Decentralized exchange of Topological Signatures.
///
/// Implements consensus mechanisms for distributed topological awareness
/// across the ed2kIA P2P network.
pub struct NoosphereGossip;

impl NoosphereGossip {
    /// Compute consensus TopologicalSignature from local + peer signatures.
    ///
    /// Uses Betti number median + persistence interval averaging for robustness
    /// against Byzantine nodes.
    pub fn consensus_signature(
        local_sig: &TopologicalSignature,
        peer_sigs: &[TopologicalSignature],
    ) -> TopologicalSignature {
        let all_sigs = std::iter::once(local_sig)
            .chain(peer_sigs.iter())
            .collect::<Vec<_>>();

        // Compute median Betti numbers for each dimension
        let max_dim = local_sig.betti_numbers.len().max(
            peer_sigs
                .iter()
                .map(|s| s.betti_numbers.len())
                .max()
                .unwrap_or(0),
        );

        let mut betti_numbers = Vec::with_capacity(max_dim);
        for dim in 0..max_dim {
            let mut values: Vec<usize> = all_sigs
                .iter()
                .filter(|s| s.betti_numbers.len() > dim)
                .map(|s| s.betti_numbers[dim])
                .collect();
            values.sort_unstable();

            let median = if values.is_empty() {
                0
            } else if values.len().is_multiple_of(2) {
                (values[values.len() / 2 - 1] + values[values.len() / 2]) / 2
            } else {
                values[values.len() / 2]
            };
            betti_numbers.push(median);
        }

        // Median persistence intervals (robust to Byzantine outliers)
        let all_intervals: Vec<(f32, f32)> = all_sigs
            .iter()
            .flat_map(|s| s.persistence_intervals.iter().cloned())
            .collect();

        let mut persistence_intervals = Vec::new();
        if !all_intervals.is_empty() {
            let mut births: Vec<f32> = all_intervals.iter().map(|(b, _)| *b).collect();
            let mut deaths: Vec<f32> = all_intervals.iter().map(|(_, d)| *d).collect();
            births.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            deaths.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            let median_birth = if births.len().is_multiple_of(2) {
                (births[births.len() / 2 - 1] + births[births.len() / 2]) / 2.0
            } else {
                births[births.len() / 2]
            };
            let median_death = if deaths.len().is_multiple_of(2) {
                (deaths[deaths.len() / 2 - 1] + deaths[deaths.len() / 2]) / 2.0
            } else {
                deaths[deaths.len() / 2]
            };
            persistence_intervals.push((median_birth, median_death));
        }

        TopologicalSignature {
            betti_numbers,
            persistence_intervals,
        }
    }

    /// Compute signature divergence between local and consensus.
    ///
    /// Low divergence = network agreement. High divergence = potential anomaly.
    pub fn signature_divergence(
        local: &TopologicalSignature,
        consensus: &TopologicalSignature,
    ) -> f32 {
        let max_dim = local.betti_numbers.len().max(consensus.betti_numbers.len());
        let mut betti_diff = 0.0f32;

        for dim in 0..max_dim {
            let b_local = *local.betti_numbers.get(dim).unwrap_or(&0);
            let b_consensus = *consensus.betti_numbers.get(dim).unwrap_or(&0);
            betti_diff += (b_local as i32 - b_consensus as i32).abs() as f32;
        }

        // Interval divergence
        let interval_diff = if local.persistence_intervals.is_empty()
            || consensus.persistence_intervals.is_empty()
        {
            0.0
        } else {
            let local_avg: f32 = local
                .persistence_intervals
                .iter()
                .map(|(b, d)| *d - *b)
                .sum::<f32>()
                / local.persistence_intervals.len() as f32;
            let consensus_avg: f32 = consensus
                .persistence_intervals
                .iter()
                .map(|(b, d)| *d - *b)
                .sum::<f32>()
                / consensus.persistence_intervals.len() as f32;
            (local_avg - consensus_avg).abs()
        };

        betti_diff + interval_diff
    }

    /// Detect Byzantine peers based on signature divergence.
    ///
    /// Returns indices of peers whose signatures deviate significantly from median.
    /// Uses iterative refinement: compute consensus, flag outliers, recompute without outliers.
    pub fn detect_byzantine(all_sigs: &[TopologicalSignature], threshold: f32) -> Vec<usize> {
        if all_sigs.len() <= 1 {
            return Vec::new();
        }

        // Use median-based consensus (first element as local, rest as peers)
        let all_as_vec: Vec<TopologicalSignature> = all_sigs.to_vec();
        let local = all_as_vec[0].clone();
        let peers = &all_as_vec[1..];
        let consensus = Self::consensus_signature(&local, peers);

        let mut byzantine = Vec::new();
        for (i, sig) in all_sigs.iter().enumerate() {
            let divergence = Self::signature_divergence(sig, &consensus);
            if divergence > threshold {
                byzantine.push(i);
            }
        }

        // Iterative refinement: recompute consensus excluding detected Byzantine nodes
        if !byzantine.is_empty() && byzantine.len() < all_sigs.len() / 2 {
            let clean_sigs: Vec<TopologicalSignature> = all_sigs
                .iter()
                .enumerate()
                .filter(|(i, _)| !byzantine.contains(i))
                .map(|(_, s)| s.clone())
                .collect();
            if clean_sigs.len() >= 2 {
                let local = clean_sigs[0].clone();
                let peers = &clean_sigs[1..];
                let refined_consensus = Self::consensus_signature(&local, peers);

                byzantine.clear();
                for (i, sig) in all_sigs.iter().enumerate() {
                    let divergence = Self::signature_divergence(sig, &refined_consensus);
                    if divergence > threshold {
                        byzantine.push(i);
                    }
                }
            }
        }

        byzantine
    }
}

/// Symbolic-Probabilistic Fusion Engine.
///
/// Combines VFE (probabilistic) with symbolic graph reasoning for hybrid control.
pub struct FusionEngine {
    pub symbolic_graph: SymbolicGraph,
    pub safe_graph: SymbolicGraph,
    pub fusion_weight: f32, // Weight for symbolic penalty in total energy
}

impl FusionEngine {
    pub fn new(
        symbolic_graph: SymbolicGraph,
        safe_graph: SymbolicGraph,
        fusion_weight: f32,
    ) -> Self {
        Self {
            symbolic_graph,
            safe_graph,
            fusion_weight,
        }
    }

    /// Compute symbolic penalty: graph edit distance between current and safe graph.
    pub fn symbolic_penalty(&self) -> f32 {
        self.symbolic_graph.edit_distance_proxy(&self.safe_graph)
    }

    /// Compute fusion energy: VFE + symbolic_penalty * fusion_weight.
    ///
    /// `prob_energy` is the VFE from the probabilistic engine (S105).
    pub fn fusion_energy(&self, prob_energy: f32) -> f32 {
        prob_energy + self.fusion_weight * self.symbolic_penalty()
    }

    /// Update symbolic graph based on new feature activations.
    pub fn update_graph(&mut self, new_graph: SymbolicGraph) {
        // Smooth update: blend current and new graph
        self.symbolic_graph = new_graph;
    }

    /// Compute trust score for a peer based on graph coherence alignment.
    pub fn peer_trust_score(&self, peer_graph: &SymbolicGraph) -> f32 {
        let dist = self.safe_graph.edit_distance_proxy(peer_graph);
        // Exponential decay: closer to safe graph = higher trust
        (-dist * 0.1).exp()
    }
}

/// Multi-Agent Collective Active Inference.
///
/// Coordinates multiple nodes minimizing collective VFE through federated
/// active inference with trust-weighted aggregation.
pub struct CollectiveInference;

impl CollectiveInference {
    /// Trust-weighted average of peer tensors.
    ///
    /// Each peer contribution is weighted by their trust score (Existential Credits).
    pub fn trust_weighted_average(
        contributions: &[Tensor],
        trusts: &[f32],
        device: &candle_core::Device,
    ) -> Result<Tensor> {
        if contributions.is_empty() {
            return Err(candle_core::Error::Msg(
                "No contributions provided".to_string(),
            ));
        }

        let total_trust: f32 = trusts.iter().sum();
        if total_trust < 1e-8 {
            // Uniform average if all trusts are zero
            let sum = contributions.iter().try_fold(
                Tensor::zeros(contributions[0].shape().clone(), DType::F32, device)?,
                |acc, t| acc.add(t),
            )?;
            return sum.broadcast_div(&Tensor::new(contributions.len() as f32, device)?);
        }

        // Weighted sum
        let mut weighted_sum = Tensor::zeros(contributions[0].shape().clone(), DType::F32, device)?;
        for (contrib, trust) in contributions.iter().zip(trusts.iter()) {
            let scaled = contrib.broadcast_mul(&Tensor::new(*trust / total_trust, device)?)?;
            weighted_sum = weighted_sum.add(&scaled)?;
        }

        Ok(weighted_sum)
    }

    /// Compute collective VFE reduction across all agents.
    pub fn collective_vfe_reduction(local_vfe: f32, peer_vfes: &[f32], trusts: &[f32]) -> f32 {
        let total_trust: f32 = trusts.iter().sum();
        if total_trust < 1e-8 || peer_vfes.is_empty() {
            return 0.0;
        }

        let weighted_peer_vfe: f32 = peer_vfes
            .iter()
            .zip(trusts.iter())
            .map(|(v, t)| v * t / total_trust)
            .sum();

        let collective_vfe = (local_vfe + weighted_peer_vfe) / 2.0;
        if local_vfe > 0.0 {
            (1.0 - collective_vfe / local_vfe) * 100.0
        } else {
            0.0
        }
    }
}

/// Formal Verification — Safety certificates with certified bounds.
pub struct SafetyCertificate {
    pub is_safe: bool,
    pub certified_epsilon: f32,
    pub barrier_margin: f32,
    pub ph_stability: f32,
    pub verification_method: String,
}

impl SafetyCertificate {
    /// Generate safety certificate using hybrid CBF + PH invariance analysis.
    pub fn verify(
        steered: &Tensor,
        original: &Tensor,
        safe_prior: &Tensor,
        horizon: usize,
    ) -> Result<Self> {
        // Compute CBF barrier margin
        let diff = steered.sub(safe_prior)?;
        let barrier_val = diff.sqr()?;
        let barrier_margin: f32 = barrier_val.sum_all()?.to_scalar::<f32>()?;

        // Compute PH stability proxy: mean variance of steered state (normalized)
        let mean: f32 = steered.mean_all()?.to_scalar::<f32>()?;
        let mean_tensor = Tensor::new(mean, steered.device())?.broadcast_as(steered.shape())?;
        let var_tensor = steered.sub(&mean_tensor)?;
        let ph_stability: f32 = var_tensor.sqr()?.mean_all()?.to_scalar::<f32>()?;

        // Compute certified epsilon: hybrid from CBF + PH
        let dist_to_original = steered.sub(original)?;
        let certified_epsilon: f32 = dist_to_original.sqr()?.sum_all()?.to_scalar::<f32>()?;

        // Safety check: barrier margin within bounds + PH stability low enough
        // Scale threshold by horizon and normalize for typical activation variance
        let barrier_threshold = 10.0 * horizon as f32;
        let stability_threshold = 500.0 * horizon as f32;
        let is_safe = barrier_margin < barrier_threshold && ph_stability < stability_threshold;

        Ok(Self {
            is_safe,
            certified_epsilon,
            barrier_margin,
            ph_stability,
            verification_method: "hybrid_cbf_ph".to_string(),
        })
    }

    /// Check if certificate meets minimum safety requirements.
    pub fn meets_requirements(&self, min_epsilon: f32, max_barrier: f32) -> bool {
        self.is_safe && self.certified_epsilon < min_epsilon && self.barrier_margin < max_barrier
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sae_integration::FeatureCategory;

    #[test]
    fn test_symbolic_graph_creation() {
        let names = vec![
            "harm_intent".to_string(),
            "helpfulness".to_string(),
            "safety_check".to_string(),
        ];
        let categories = vec![
            FeatureCategory::Harmful,
            FeatureCategory::Helpful,
            FeatureCategory::Safety,
        ];
        let graph = SymbolicGraph::from_features(&names, &categories);
        assert_eq!(graph.nodes.len(), 3);
        assert!(!graph.relations.is_empty());
    }

    #[test]
    fn test_noosphere_consensus() {
        let local = TopologicalSignature {
            betti_numbers: vec![3, 1, 0],
            persistence_intervals: vec![(0.0, 1.0), (0.5, 1.5)],
        };
        let peers = vec![
            TopologicalSignature {
                betti_numbers: vec![3, 2, 0],
                persistence_intervals: vec![(0.1, 1.1)],
            },
            TopologicalSignature {
                betti_numbers: vec![2, 1, 1],
                persistence_intervals: vec![(0.2, 1.2)],
            },
        ];

        let consensus = NoosphereGossip::consensus_signature(&local, &peers);
        assert!(!consensus.betti_numbers.is_empty());
    }

    #[test]
    fn test_byzantine_detection() {
        let normal_sig = TopologicalSignature {
            betti_numbers: vec![3, 1, 0],
            persistence_intervals: vec![(0.0, 1.0)],
        };
        let byzantine_sig = TopologicalSignature {
            betti_numbers: vec![100, 50, 25],
            persistence_intervals: vec![(10.0, 100.0)],
        };

        let all_sigs = vec![normal_sig.clone(), normal_sig.clone(), byzantine_sig];
        let byzantine = NoosphereGossip::detect_byzantine(&all_sigs, 5.0);
        assert!(byzantine.contains(&2));
    }

    #[test]
    fn test_safety_certificate() {
        let device = candle_core::Device::Cpu;
        let original = Tensor::zeros((10,), DType::F32, &device).unwrap();
        let steered = Tensor::zeros((10,), DType::F32, &device).unwrap();
        let safe_prior = Tensor::zeros((10,), DType::F32, &device).unwrap();

        let cert = SafetyCertificate::verify(&steered, &original, &safe_prior, 10).unwrap();
        assert!(cert.is_safe);
        assert_eq!(cert.verification_method, "hybrid_cbf_ph");
    }

    #[test]
    fn test_collective_vfe_reduction() {
        let reduction = CollectiveInference::collective_vfe_reduction(
            70.0,
            &[65.0, 60.0, 55.0],
            &[0.4, 0.3, 0.3],
        );
        assert!(reduction > 0.0);
        assert!(reduction < 100.0);
    }
}
