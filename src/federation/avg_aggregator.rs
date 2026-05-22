//! FedAvg + Krum – Federated aggregation of SAE weights with Byzantine tolerance
//!
//! Implements `FedAvgAggregator` with methods:
//! - `add_update()` – register a weight update from a federated node
//! - `aggregate()` – perform FedAvg with optional Krum filtering
//! - `apply_krum_filter(f)` – Byzantine-resistant node selection
//!
//! Uses `candle_core::Tensor` for vectorized operations where available.
//! Krum complexity: O(n²) pairwise euclidean distance calculation.
//!
//! # Feature Flag
//!
//! This module is gated behind `#[cfg(feature = "phase6-core")]`.

#[cfg(feature = "phase6-core")]
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

// ---------------------------------------------------------------------------
// Public types (always available for serialization)
// ---------------------------------------------------------------------------

/// Weight update from a federated node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightUpdate {
    pub node_id: String,
    pub layer_id: u32,
    /// Flattened weight deltas
    pub weight_deltas: Vec<f32>,
    /// Local sample count
    pub num_samples: usize,
    /// Local loss
    pub local_loss: f32,
    /// Unix ms timestamp
    pub timestamp: u64,
    /// SHA-256 hash for integrity
    pub update_hash: String,
}

impl WeightUpdate {
    pub fn new(
        node_id: String,
        layer_id: u32,
        weight_deltas: Vec<f32>,
        num_samples: usize,
        local_loss: f32,
    ) -> Self {
        let update_hash = Self::compute_hash(&weight_deltas, &node_id);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            node_id,
            layer_id,
            weight_deltas,
            num_samples,
            local_loss,
            timestamp,
            update_hash,
        }
    }

    fn compute_hash(deltas: &[f32], node_id: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(node_id.as_bytes());
        for d in deltas {
            hasher.update(d.to_le_bytes());
        }
        format!("{:x}", hasher.finalize())
    }

    pub fn verify_hash(&self) -> bool {
        self.update_hash == Self::compute_hash(&self.weight_deltas, &self.node_id)
    }

    pub fn dimension(&self) -> usize {
        self.weight_deltas.len()
    }
}

/// Result of FedAvg aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationResult {
    pub layer_id: u32,
    /// Final aggregated weights
    pub final_weights: Vec<f32>,
    /// Number of accepted updates
    pub accepted_updates: usize,
    /// Number of filtered malicious nodes
    pub filtered_malicious: usize,
    /// Confidence score [0.0, 1.0]
    pub confidence: f32,
    /// Included node IDs
    pub included_nodes: Vec<String>,
    /// Excluded node IDs
    pub excluded_nodes: Vec<String>,
    /// Aggregation timestamp
    pub timestamp: u64,
}

/// Configuration for the aggregator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedAvgConfig {
    pub min_participants: usize,
    /// Krum f parameter (number of Byzantine nodes to tolerate)
    pub krum_f: usize,
    pub min_participation_fraction: f64,
}

impl Default for FedAvgConfig {
    fn default() -> Self {
        Self {
            min_participants: 3,
            krum_f: 1,
            min_participation_fraction: 0.4,
        }
    }
}

// ---------------------------------------------------------------------------
// FedAvgAggregator – core implementation (feature gated)
// ---------------------------------------------------------------------------

/// Federated Averaging aggregator with Krum Byzantine filtering.
#[cfg(feature = "phase6-core")]
pub struct FedAvgAggregator {
    config: FedAvgConfig,
    pending_updates: HashMap<u32, Vec<WeightUpdate>>,
}

#[cfg(feature = "phase6-core")]
impl FedAvgAggregator {
    pub fn new(config: FedAvgConfig) -> Self {
        Self {
            config,
            pending_updates: HashMap::new(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(FedAvgConfig::default())
    }

    // ------------------------------------------------------------------
    // add_update
    // ------------------------------------------------------------------

    /// Register a weight update from a federated node.
    pub fn add_update(&mut self, update: WeightUpdate) -> Result<()> {
        if !update.verify_hash() {
            warn!("Invalid update hash from node {}", update.node_id);
            return Err(anyhow::anyhow!(
                "Invalid update hash from node {}",
                update.node_id
            ));
        }

        info!(
            "Received update from {} for layer {} (dim={}, samples={})",
            update.node_id,
            update.layer_id,
            update.dimension(),
            update.num_samples,
        );

        self.pending_updates
            .entry(update.layer_id)
            .or_default()
            .push(update);

        Ok(())
    }

    // ------------------------------------------------------------------
    // aggregate
    // ------------------------------------------------------------------

    /// Perform FedAvg aggregation for a given layer.
    pub fn aggregate(&self, layer_id: u32) -> Result<AggregationResult> {
        let updates = self
            .pending_updates
            .get(&layer_id)
            .ok_or_else(|| anyhow::anyhow!("No updates for layer {}", layer_id))?;

        if updates.len() < self.config.min_participants {
            return Err(anyhow::anyhow!(
                "Insufficient participants: {} < {}",
                updates.len(),
                self.config.min_participants
            ));
        }

        // Step 1: Krum filter if f > 0
        let (included_indices, excluded_nodes) = if self.config.krum_f > 0 {
            self.apply_krum_filter(updates, self.config.krum_f)?
        } else {
            let indices: Vec<usize> = (0..updates.len()).collect();
            (indices, Vec::new())
        };

        // Step 2: Weighted FedAvg
        let final_weights = self.weighted_avg(updates, &included_indices)?;

        // Step 3: Compute confidence
        let confidence = self.compute_confidence(updates.len(), included_indices.len());

        let included_nodes: Vec<String> = included_indices
            .iter()
            .map(|&i| updates[i].node_id.clone())
            .collect();

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        info!(
            "Aggregation layer {}: {} participants, {} accepted, {} filtered, confidence={:.3}",
            layer_id,
            updates.len(),
            included_indices.len(),
            excluded_nodes.len(),
            confidence,
        );

        Ok(AggregationResult {
            layer_id,
            final_weights,
            accepted_updates: included_indices.len(),
            filtered_malicious: excluded_nodes.len(),
            confidence,
            included_nodes,
            excluded_nodes,
            timestamp,
        })
    }

    // ------------------------------------------------------------------
    // apply_krum_filter
    // ------------------------------------------------------------------

    /// Apply Krum Byzantine filtering.
    ///
    /// Selects the top `n - f - 2` closest updates based on pairwise
    /// euclidean distance between weight deltas.
    /// Complexity: O(n²) for distance matrix.
    pub fn apply_krum_filter(
        &self,
        updates: &[WeightUpdate],
        f: usize,
    ) -> Result<(Vec<usize>, Vec<String>)> {
        let n = updates.len();
        let select_count = (n - f - 2).max(1).min(n);

        // Compute pairwise distance matrix using Tensor ops
        let distances = self.compute_distance_matrix(updates)?;

        // Compute Krum scores
        let mut scores: Vec<(usize, f64)> = (0..n)
            .map(|i| {
                let mut dists = distances[i].clone();
                dists.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let score: f64 = dists[1..select_count.min(dists.len())].iter().sum();
                (i, score)
            })
            .collect();

        scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let included: Vec<usize> = scores[..select_count].iter().map(|&(i, _)| i).collect();
        let excluded_set: std::collections::HashSet<usize> = included.iter().copied().collect();
        let excluded_nodes: Vec<String> = updates
            .iter()
            .enumerate()
            .filter(|(i, _)| !excluded_set.contains(i))
            .map(|(_, u)| u.node_id.clone())
            .collect();

        if !excluded_nodes.is_empty() {
            warn!("Krum excluded nodes: {:?}", excluded_nodes);
        }

        Ok((included, excluded_nodes))
    }

    // ------------------------------------------------------------------
    // Vectorized helpers
    // ------------------------------------------------------------------

    /// Compute pairwise euclidean distance matrix using native Rust (O(n²)).
    /// candle_core broadcasting doesn't work for subtraction, so use loops.
    fn compute_distance_matrix(&self, updates: &[WeightUpdate]) -> Result<Vec<Vec<f64>>> {
        let n = updates.len();
        let dim = updates.first().map(|u| u.dimension()).unwrap_or(0);

        let mut matrix = vec![vec![0.0f64; n]; n];

        for i in 0..n {
            matrix[i][i] = 0.0;
            for j in (i + 1)..n {
                let mut sum_sq = 0.0f64;
                for k in 0..dim {
                    let diff =
                        updates[i].weight_deltas[k] as f64 - updates[j].weight_deltas[k] as f64;
                    sum_sq += diff * diff;
                }
                let dist = sum_sq.sqrt();
                matrix[i][j] = dist;
                matrix[j][i] = dist;
            }
        }

        debug!("Krum distance matrix computed: n={}, dim={}", n, dim);
        Ok(matrix)
    }

    /// Weighted FedAvg using native loops (candle_core sum_dim unavailable).
    fn weighted_avg(&self, updates: &[WeightUpdate], indices: &[usize]) -> Result<Vec<f32>> {
        if indices.is_empty() {
            return Err(anyhow::anyhow!("No updates to aggregate"));
        }

        let dim = updates[indices[0]].dimension();
        let total_samples: usize = indices.iter().map(|&i| updates[i].num_samples).sum();

        if total_samples == 0 {
            return Err(anyhow::anyhow!("Zero total samples"));
        }

        let mut aggregated = vec![0.0f64; dim];

        for &i in indices {
            let weight = updates[i].num_samples as f64 / total_samples as f64;
            for (j, delta) in updates[i].weight_deltas.iter().enumerate() {
                aggregated[j] += *delta as f64 * weight;
            }
        }

        debug!(
            "FedAvg completed: dim={}, participants={}",
            dim,
            indices.len()
        );
        Ok(aggregated.iter().map(|&x| x as f32).collect())
    }

    /// Compute confidence score based on acceptance ratio.
    fn compute_confidence(&self, total: usize, accepted: usize) -> f32 {
        if total == 0 {
            return 0.0;
        }
        (accepted as f32 / total as f32).min(1.0)
    }

    // ------------------------------------------------------------------
    // Query helpers
    // ------------------------------------------------------------------

    pub fn pending_count(&self, layer_id: u32) -> usize {
        self.pending_updates
            .get(&layer_id)
            .map(|u| u.len())
            .unwrap_or(0)
    }

    pub fn pending_layers(&self) -> Vec<u32> {
        let mut layers: Vec<u32> = self.pending_updates.keys().copied().collect();
        layers.sort();
        layers
    }

    pub fn clear_layer(&mut self, layer_id: u32) {
        self.pending_updates.remove(&layer_id);
    }
}

#[cfg(feature = "phase6-core")]
impl Default for FedAvgAggregator {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_update(node_id: &str, layer_id: u32, dim: usize, seed: u32) -> WeightUpdate {
        let deltas: Vec<f32> = (0..dim)
            .map(|i| ((i + seed as usize) % 100) as f32 / 50.0 - 1.0)
            .collect();
        WeightUpdate::new(node_id.to_string(), layer_id, deltas, 100, 0.5)
    }

    #[test]
    fn test_weight_update_hash() {
        let update = make_update("node1", 0, 10, 42);
        assert!(update.verify_hash());
    }

    #[test]
    fn test_weight_update_dimension() {
        let update = make_update("node1", 0, 50, 1);
        assert_eq!(update.dimension(), 50);
    }

    #[test]
    fn test_aggregation_result_fields() {
        let result = AggregationResult {
            layer_id: 0,
            final_weights: vec![0.0; 10],
            accepted_updates: 3,
            filtered_malicious: 1,
            confidence: 0.75,
            included_nodes: vec!["a".into(), "b".into(), "c".into()],
            excluded_nodes: vec!["d".into()],
            timestamp: 0,
        };
        assert_eq!(result.accepted_updates, 3);
        assert_eq!(result.filtered_malicious, 1);
        assert!((result.confidence - 0.75).abs() < 1e-6);
    }

    #[test]
    fn test_fedavg_config_default() {
        let config = FedAvgConfig::default();
        assert_eq!(config.min_participants, 3);
        assert_eq!(config.krum_f, 1);
    }

    // ---- Tests requiring phase6-core ----

    #[cfg(feature = "phase6-core")]
    mod phase6_tests {
        use super::*;

        #[test]
        fn test_add_update() {
            let mut agg = FedAvgAggregator::with_defaults();
            let update = make_update("node1", 0, 100, 42);
            agg.add_update(update).unwrap();
            assert_eq!(agg.pending_count(0), 1);
        }

        #[test]
        fn test_reject_invalid_hash() {
            let mut agg = FedAvgAggregator::with_defaults();
            let mut update = make_update("node1", 0, 10, 1);
            update.update_hash = "invalid_hash".to_string();
            let result = agg.add_update(update);
            assert!(result.is_err());
        }

        #[test]
        fn test_fedavg_aggregation() {
            let mut agg = FedAvgAggregator::with_defaults();
            agg.config.krum_f = 0;

            for i in 0..5 {
                let update = make_update(&format!("node{}", i), 0, 100, i);
                agg.add_update(update).unwrap();
            }

            let result = agg.aggregate(0).unwrap();
            assert_eq!(result.layer_id, 0);
            assert_eq!(result.final_weights.len(), 100);
            assert_eq!(result.accepted_updates, 5);
            assert_eq!(result.filtered_malicious, 0);
        }

        #[test]
        fn test_krum_filter_excludes_outlier() {
            let mut agg = FedAvgAggregator::with_defaults();
            agg.config.krum_f = 1;

            for i in 0..4 {
                let update = make_update(&format!("node{}", i), 0, 50, 10);
                agg.add_update(update).unwrap();
            }
            let outlier = make_update("byzantine", 0, 50, 999);
            agg.add_update(outlier).unwrap();

            let result = agg.aggregate(0).unwrap();
            // n=5, f=1 → select_count = n-f-2 = 2, so 3 filtered
            assert!(result.excluded_nodes.contains(&"byzantine".to_string()));
            assert_eq!(result.filtered_malicious, 3);
            assert_eq!(result.accepted_updates, 2);
        }

        #[test]
        fn test_insufficient_participants() {
            let mut agg = FedAvgAggregator::with_defaults();
            agg.config.min_participants = 3;

            let update = make_update("solo", 0, 10, 1);
            agg.add_update(update).unwrap();

            let err = agg.aggregate(0).unwrap_err();
            assert!(err.to_string().contains("Insufficient"));
        }

        #[test]
        fn test_pending_layers() {
            let mut agg = FedAvgAggregator::with_defaults();

            for layer in [0, 5, 10] {
                for i in 0..3 {
                    let update = make_update(&format!("node{}", i), layer, 20, i);
                    agg.add_update(update).unwrap();
                }
            }

            let layers = agg.pending_layers();
            assert_eq!(layers, vec![0, 5, 10]);
            assert_eq!(agg.pending_count(0), 3);
        }

        #[test]
        fn test_confidence_score() {
            let mut agg = FedAvgAggregator::with_defaults();
            agg.config.krum_f = 0;

            for i in 0..4 {
                let update = make_update(&format!("node{}", i), 0, 10, i);
                agg.add_update(update).unwrap();
            }

            let result = agg.aggregate(0).unwrap();
            assert!((result.confidence - 1.0).abs() < 1e-6);
        }

        #[test]
        fn test_clear_layer() {
            let mut agg = FedAvgAggregator::with_defaults();
            let update = make_update("node1", 0, 10, 1);
            agg.add_update(update).unwrap();
            assert_eq!(agg.pending_count(0), 1);

            agg.clear_layer(0);
            assert_eq!(agg.pending_count(0), 0);
        }

        #[test]
        fn test_weighted_avg() {
            let agg = FedAvgAggregator::with_defaults();
            let updates = vec![
                WeightUpdate::new("a".into(), 0, vec![1.0, 2.0, 3.0], 100, 0.5),
                WeightUpdate::new("b".into(), 0, vec![4.0, 5.0, 6.0], 100, 0.5),
            ];
            let indices = vec![0, 1];

            let result = agg.weighted_avg(&updates, &indices).unwrap();
            assert_eq!(result.len(), 3);
            assert!((result[0] - 2.5).abs() < 1e-5);
            assert!((result[1] - 3.5).abs() < 1e-5);
            assert!((result[2] - 4.5).abs() < 1e-5);
        }
    }
}
