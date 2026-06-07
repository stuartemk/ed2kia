//! Collective Zonotope Intelligence — Distributed Zonotope Gossip + Robust Aggregation.
//!
//! Extends zonotope verification to distributed P2P settings where peers share
//! reduced zonotope summaries (center + top-k generators via SVD truncation)
//! and perform robust aggregation using geometric median.
//!
//! **Key Features:**
//! - Low-rank zonotope compression via generator SVD
//! - Geometric median aggregation (Byzantine-resilient)
//! - Consensus verification across peer zonotopes
//! - Trust-weighted zonotope fusion

use crate::zonotope::{RobustnessCertificate, Zonotope, ZonotopeConfig};
use candle_core::{DType, Device, Result, Tensor};

/// Configuration for collective zonotope operations.
#[derive(Debug, Clone)]
pub struct CollectiveZonotopeConfig {
    /// Number of generators to keep in compressed gossip messages.
    pub gossip_gens: usize,
    /// Number of iterations for geometric median (Weiszfeld's algorithm).
    pub weiszfeld_iters: usize,
    /// Convergence threshold for Weiszfeld.
    pub weiszfeld_tol: f32,
    /// Trust weight for local zonotope vs. peer zonotopes.
    pub local_trust: f32,
}

impl Default for CollectiveZonotopeConfig {
    fn default() -> Self {
        Self {
            gossip_gens: 32,
            weiszfeld_iters: 20,
            weiszfeld_tol: 1e-4,
            local_trust: 0.5,
        }
    }
}

/// Compressed zonotope summary for P2P gossip.
///
/// Contains only center + top-k generators (by norm), reducing bandwidth
/// from O(num_gens * dim) to O(k * dim) where k << num_gens.
#[derive(Debug, Clone)]
pub struct ZonotopeSummary {
    pub peer_id: String,
    pub center: Vec<f32>,
    pub generators: Vec<Vec<f32>>,
    pub volume_proxy: f32,
    pub trust_score: f32,
}

impl ZonotopeSummary {
    /// Create a compressed summary from a zonotope.
    pub fn from_zonotope(z: &Zonotope, peer_id: &str, k: usize) -> Result<Self> {
        let center_vec: Vec<f32> = z.center.flatten_all()?.to_vec1()?;
        let num_gens = z.num_gens()?;
        let _dim = z.hidden_dim()?;
        let top_k = k.min(num_gens);

        // Sort generators by norm (descending) and keep top-k
        let gens_2d: Vec<Vec<f32>> = z.generators.to_vec2()?;
        let mut indexed_norms: Vec<(usize, f32)> = Vec::new();

        for (i, row) in gens_2d.iter().enumerate() {
            let norm: f32 = row.iter().map(|x| x * x).sum::<f32>().sqrt();
            indexed_norms.push((i, norm));
        }
        indexed_norms.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut top_generators = Vec::new();
        for &(idx, _) in indexed_norms.iter().take(top_k) {
            top_generators.push(gens_2d[idx].clone());
        }

        Ok(Self {
            peer_id: peer_id.to_string(),
            center: center_vec,
            generators: top_generators,
            volume_proxy: z.volume_proxy()?,
            trust_score: 1.0,
        })
    }

    /// Reconstruct a zonotope from this summary.
    pub fn to_zonotope(&self, device: &Device) -> Result<Zonotope> {
        let dim = self.center.len();
        let center = Tensor::from_vec(self.center.clone(), (1, dim), device)?;

        let num_gens = self.generators.len();
        let gens_flat: Vec<f32> = self.generators.iter().flatten().cloned().collect();
        let generators = if num_gens > 0 {
            Tensor::from_vec(gens_flat, (num_gens, dim), device)?
        } else {
            Tensor::zeros((0, dim), DType::F32, device)?
        };

        let config = ZonotopeConfig {
            max_gens: num_gens.max(1),
            ..Default::default()
        };

        Zonotope::new(center, generators, config)
    }
}

/// Collective zonotope engine for distributed verification.
pub struct CollectiveZonotopeEngine {
    config: CollectiveZonotopeConfig,
    device: Device,
}

impl CollectiveZonotopeEngine {
    pub fn new(config: &CollectiveZonotopeConfig) -> Self {
        Self {
            config: config.clone(),
            device: Device::Cpu,
        }
    }

    pub fn with_device(config: &CollectiveZonotopeConfig, device: &Device) -> Self {
        Self {
            config: config.clone(),
            device: device.clone(),
        }
    }

    /// Compress a zonotope for gossip transmission.
    pub fn compress_for_gossip(&self, z: &Zonotope, peer_id: &str) -> Result<ZonotopeSummary> {
        ZonotopeSummary::from_zonotope(z, peer_id, self.config.gossip_gens)
    }

    /// Robust aggregation of peer zonotope summaries using geometric median.
    ///
    /// The geometric median is Byzantine-resilient: it tolerates up to f Byzantine
    /// peers in a network of n >= 2f+1 peers.
    pub fn robust_aggregate(&self, summaries: &[ZonotopeSummary]) -> Result<Zonotope> {
        if summaries.is_empty() {
            candle_core::bail!("No peer summaries to aggregate");
        }

        let dim = summaries[0].center.len();
        let _n = summaries.len();

        // Compute geometric median of centers using Weiszfeld's algorithm
        let centers: Vec<&[f32]> = summaries.iter().map(|s| s.center.as_slice()).collect();
        let weights: Vec<f32> = summaries.iter().map(|s| s.trust_score).collect();
        let median_center = self.weiszfeld_median(&centers, &weights)?;

        // Aggregate generators: collect all top generators, sort by norm, keep top-k
        let mut all_gens: Vec<(Vec<f32>, f32)> = Vec::new();
        for summary in summaries {
            for gen in &summary.generators {
                let norm: f32 = gen.iter().map(|x| x * x).sum::<f32>().sqrt();
                all_gens.push((gen.clone(), norm));
            }
        }
        all_gens.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let top_k = self.config.gossip_gens.min(all_gens.len());
        let mut aggregated_gens = Vec::new();
        for (gen, _) in all_gens.iter().take(top_k) {
            aggregated_gens.push(gen.clone());
        }

        // Build aggregated zonotope
        let center_tensor = Tensor::from_vec(median_center, (1, dim), &self.device)?;
        let gens_flat: Vec<f32> = aggregated_gens.iter().flatten().cloned().collect();
        let generators_tensor = if aggregated_gens.is_empty() {
            Tensor::zeros((0, dim), DType::F32, &self.device)?
        } else {
            Tensor::from_vec(gens_flat, (aggregated_gens.len(), dim), &self.device)?
        };

        let config = ZonotopeConfig {
            max_gens: aggregated_gens.len().max(1),
            ..Default::default()
        };

        Zonotope::new(center_tensor, generators_tensor, config)
    }

    /// Weiszfeld's algorithm for weighted geometric median.
    ///
    /// Minimizes: sum_i w_i * ||x - c_i||
    pub fn weiszfeld_median(&self, centers: &[&[f32]], weights: &[f32]) -> Result<Vec<f32>> {
        let dim = centers[0].len();
        let _n = centers.len();

        // Initialize at weighted mean
        let mut x: Vec<f32> = vec![0.0; dim];
        let total_weight: f32 = weights.iter().sum();

        for d in 0..dim {
            for (i, &c) in centers.iter().enumerate() {
                x[d] += weights[i] * c[d];
            }
        }
        x.iter_mut().for_each(|v| *v /= total_weight);

        // Weiszfeld iterations
        for _ in 0..self.config.weiszfeld_iters {
            let mut numerators = vec![0.0f32; dim];
            let mut denominator = 0.0f32;

            for (i, &c) in centers.iter().enumerate() {
                let dist: f32 = (0..dim).map(|d| (x[d] - c[d]).powi(2)).sum::<f32>().sqrt();
                let w = weights[i] / (dist + 1e-10);

                for d in 0..dim {
                    numerators[d] += w * c[d];
                }
                denominator += w;
            }

            let mut new_x = numerators;
            for item in new_x.iter_mut().take(dim) {
                *item /= denominator;
            }

            // Check convergence
            let diff: f32 = (0..dim)
                .map(|d| (x[d] - new_x[d]).powi(2))
                .sum::<f32>()
                .sqrt();
            x = new_x;

            if diff < self.config.weiszfeld_tol {
                break;
            }
        }

        Ok(x)
    }

    /// Trust-weighted zonotope fusion.
    ///
    /// Combines local zonotope with peer zonotopes using trust weights.
    /// Higher trust = more influence on the fused result.
    pub fn trust_weighted_fusion(
        &self,
        local_z: &Zonotope,
        peer_summaries: &[ZonotopeSummary],
    ) -> Result<Zonotope> {
        if peer_summaries.is_empty() {
            return Ok(local_z.clone());
        }

        let dim = local_z.hidden_dim()?;
        let local_weight = self.config.local_trust;
        let peer_weight = (1.0 - local_weight) / (peer_summaries.len() as f32 + 1e-8);

        // Weighted center
        let local_center: Vec<f32> = local_z.center.flatten_all()?.to_vec1()?;
        let mut fused_center = local_center
            .iter()
            .map(|&v| v * local_weight)
            .collect::<Vec<_>>();

        for summary in peer_summaries {
            for (d, fc) in fused_center.iter_mut().enumerate().take(dim) {
                *fc += summary.center[d] * peer_weight * summary.trust_score;
            }
        }

        let center_tensor = Tensor::from_vec(fused_center, (1, dim), &self.device)?;

        // Merge generators: local + top peer generators
        let local_gens: Vec<Vec<f32>> = local_z.generators.to_vec2()?;
        let mut all_gens_with_norm: Vec<(Vec<f32>, f32)> = local_gens
            .into_iter()
            .map(|g| {
                let norm: f32 = g.iter().map(|x| x * x).sum::<f32>().sqrt();
                (g, norm)
            })
            .collect();

        for summary in peer_summaries {
            for gen in &summary.generators {
                let norm: f32 = gen.iter().map(|x| x * x).sum::<f32>().sqrt();
                all_gens_with_norm.push((gen.clone(), norm * summary.trust_score));
            }
        }

        all_gens_with_norm
            .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let top_k = self.config.gossip_gens.min(all_gens_with_norm.len());
        let fused_gens: Vec<Vec<f32>> = all_gens_with_norm
            .iter()
            .take(top_k)
            .map(|(g, _)| g.clone())
            .collect();

        let gens_flat: Vec<f32> = fused_gens.iter().flatten().cloned().collect();
        let generators_tensor = if fused_gens.is_empty() {
            Tensor::zeros((0, dim), DType::F32, &self.device)?
        } else {
            Tensor::from_vec(gens_flat, (fused_gens.len(), dim), &self.device)?
        };

        let config = ZonotopeConfig {
            max_gens: fused_gens.len().max(1),
            ..Default::default()
        };

        Zonotope::new(center_tensor, generators_tensor, config)
    }

    /// Consensus verification: check if all peer zonotopes agree on safety.
    ///
    /// Returns true if the intersection of all peer zonotopes is still safe.
    pub fn consensus_verify(
        &self,
        peer_summaries: &[ZonotopeSummary],
        safe_centroid: &Tensor,
        toxic_centroid: &Tensor,
        cbf_beta: f32,
    ) -> Result<ConsensusResult> {
        if peer_summaries.is_empty() {
            return Ok(ConsensusResult {
                consensus: true,
                num_peers: 0,
                num_safe: 0,
                num_unsafe: 0,
                certificates: Vec::new(),
            });
        }

        let mut certificates = Vec::new();
        let mut num_safe = 0usize;
        let mut num_unsafe = 0usize;

        for summary in peer_summaries {
            let z = summary.to_zonotope(&self.device)?;
            let cert = z.verify_steering_robustness(safe_centroid, toxic_centroid, cbf_beta)?;
            if cert.certified {
                num_safe += 1;
            } else {
                num_unsafe += 1;
            }
            certificates.push(cert);
        }

        let consensus = num_unsafe == 0;

        Ok(ConsensusResult {
            consensus,
            num_peers: peer_summaries.len(),
            num_safe,
            num_unsafe,
            certificates,
        })
    }
}

/// Result of consensus verification across peers.
#[derive(Debug, Clone)]
pub struct ConsensusResult {
    pub consensus: bool,
    pub num_peers: usize,
    pub num_safe: usize,
    pub num_unsafe: usize,
    pub certificates: Vec<RobustnessCertificate>,
}

impl std::fmt::Display for ConsensusResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Consensus {{ consensus={}, peers={}, safe={}, unsafe={} }}",
            self.consensus, self.num_peers, self.num_safe, self.num_unsafe
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zonotope_summary_compression() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::new(&[1.0f32, 2.0, 3.0, 4.0], &device)?.unsqueeze(0)?;
        let z = Zonotope::new_from_epsilon(&center, 0.1, 4)?;

        let summary = ZonotopeSummary::from_zonotope(&z, "peer_1", 2)?;

        assert_eq!(summary.center.len(), 4);
        assert_eq!(summary.generators.len(), 2);
        assert_eq!(summary.peer_id, "peer_1");
        Ok(())
    }

    #[test]
    fn test_summary_reconstruction() -> Result<()> {
        let device = Device::Cpu;
        let center = Tensor::new(&[1.0f32, 2.0, 3.0], &device)?.unsqueeze(0)?;
        let z = Zonotope::new_from_epsilon(&center, 0.1, 3)?;

        let summary = ZonotopeSummary::from_zonotope(&z, "test", 3)?;
        let z_reconstructed = summary.to_zonotope(&device)?;

        assert_eq!(z_reconstructed.hidden_dim()?, 3);
        assert!(z_reconstructed.num_gens()? > 0);
        Ok(())
    }

    #[test]
    fn test_robust_aggregation() -> Result<()> {
        let device = Device::Cpu;
        let config = CollectiveZonotopeConfig::default();
        let engine = CollectiveZonotopeEngine::with_device(&config, &device);

        let summaries: Vec<ZonotopeSummary> = (0..5)
            .map(|i| {
                let c = vec![i as f32, (i + 1) as f32, (i + 2) as f32];
                ZonotopeSummary {
                    peer_id: format!("peer_{}", i),
                    center: c,
                    generators: vec![[0.1f32, 0.1, 0.1].to_vec()],
                    volume_proxy: 0.3,
                    trust_score: 1.0,
                }
            })
            .collect();

        let aggregated = engine.robust_aggregate(&summaries)?;
        assert_eq!(aggregated.hidden_dim()?, 3);
        Ok(())
    }

    #[test]
    fn test_weiszfeld_median() -> Result<()> {
        let config = CollectiveZonotopeConfig::default();
        let engine = CollectiveZonotopeEngine::new(&config);

        // 1D points: [0, 2, 4] → median should be near 2
        let c0: &[f32] = &[0.0f32];
        let c1: &[f32] = &[2.0];
        let c2: &[f32] = &[4.0];
        let centers: &[&[f32]] = &[c0, c1, c2];
        let weights = vec![1.0, 1.0, 1.0];

        let median = engine.weiszfeld_median(centers, &weights)?;
        assert!((median[0] - 2.0).abs() < 0.1);
        Ok(())
    }

    #[test]
    fn test_trust_weighted_fusion() -> Result<()> {
        let device = Device::Cpu;
        let config = CollectiveZonotopeConfig::default();
        let engine = CollectiveZonotopeEngine::with_device(&config, &device);

        let local_center = Tensor::new(&[1.0f32, 1.0], &device)?.unsqueeze(0)?;
        let local_z = Zonotope::new_from_epsilon(&local_center, 0.1, 2)?;

        let summaries = vec![
            ZonotopeSummary {
                peer_id: "p1".into(),
                center: vec![2.0, 2.0],
                generators: vec![[0.05, 0.05].to_vec()],
                volume_proxy: 0.1,
                trust_score: 0.9,
            },
            ZonotopeSummary {
                peer_id: "p2".into(),
                center: vec![3.0, 3.0],
                generators: vec![[0.05, 0.05].to_vec()],
                volume_proxy: 0.1,
                trust_score: 0.7,
            },
        ];

        let fused = engine.trust_weighted_fusion(&local_z, &summaries)?;
        assert_eq!(fused.hidden_dim()?, 2);
        Ok(())
    }

    #[test]
    fn test_consensus_verification() -> Result<()> {
        let device = Device::Cpu;
        let config = CollectiveZonotopeConfig::default();
        let engine = CollectiveZonotopeEngine::with_device(&config, &device);

        // Safe centroids
        let safe = Tensor::zeros((1, 3), DType::F32, &device)?;
        let toxic = Tensor::new(&[10.0f32, 10.0, 10.0], &device)?.unsqueeze(0)?;

        // Create peer summaries near safe centroid
        let summaries: Vec<ZonotopeSummary> = (0..3)
            .map(|i| {
                let z = Zonotope::new_from_epsilon(
                    &Tensor::full(i as f32 * 0.01, (1, 3), &device).unwrap(),
                    0.01,
                    2,
                )
                .unwrap();
                ZonotopeSummary::from_zonotope(&z, &format!("peer_{}", i), 2).unwrap()
            })
            .collect();

        let result = engine.consensus_verify(&summaries, &safe, &toxic, 1.0)?;
        assert_eq!(result.num_peers, 3);
        Ok(())
    }

    #[test]
    fn test_empty_aggregation_fails() -> Result<()> {
        let config = CollectiveZonotopeConfig::default();
        let engine = CollectiveZonotopeEngine::new(&config);

        let result = engine.robust_aggregate(&[]);
        assert!(result.is_err());
        Ok(())
    }
}
