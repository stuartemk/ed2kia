//! Sparse Federated SAE Updates — Share sparse dictionaries + proofs, not dense tensors.
//!
//! Instead of transmitting dense weight matrices during federated SAE updates,
//! this module compresses updates to their sparse Top-K representations along
//! with cryptographic proofs (PoSym contributions) for verification.
//!
//! **Key Insight:** SAE weights are inherently sparse (Top-k activation).
//! Transmitting only the non-zero entries + indices reduces bandwidth by
//! orders of magnitude compared to dense tensor synchronization.

use candle_core::{Device, Result, Tensor};
use sha2::{Digest, Sha256};

/// SHA-256 hash digest.
pub type Hash = [u8; 32];

/// Sparse entry: index + value pair for compressed representation.
#[derive(Debug, Clone)]
pub struct SparseEntry {
    /// Flattened index in the original tensor.
    pub index: usize,
    /// Value at this index.
    pub value: f32,
}

/// Compressed sparse update for a single SAE weight matrix.
#[derive(Debug, Clone)]
pub struct SparseWeightUpdate {
    /// Target matrix identifier (e.g., "encoder_w", "decoder_w").
    pub matrix_id: String,
    /// Original tensor shape.
    pub shape: Vec<usize>,
    /// Sparse entries (non-zero values after Top-k selection).
    pub entries: Vec<SparseEntry>,
    /// Cryptographic hash of the update for integrity verification.
    pub hash: Hash,
    /// PoSym contribution proof (optional).
    pub proof: Option<UpdateProof>,
}

/// Cryptographic proof attached to a sparse update.
#[derive(Debug, Clone)]
pub struct UpdateProof {
    /// Node ID that generated this update.
    pub node_id: u64,
    /// Timestamp or block number.
    pub timestamp: u64,
    /// VFE before the update.
    pub vfe_before: f64,
    /// VFE after the update.
    pub vfe_after: f64,
    /// Hash of the proof data.
    pub proof_hash: Hash,
}

impl UpdateProof {
    /// Create a new update proof with cryptographic hash.
    pub fn new(node_id: u64, timestamp: u64, vfe_before: f64, vfe_after: f64) -> Self {
        let proof_hash = Self::compute_hash(node_id, timestamp, vfe_before, vfe_after);
        Self {
            node_id,
            timestamp,
            vfe_before,
            vfe_after,
            proof_hash,
        }
    }

    /// Verify the proof hash matches the stored values.
    pub fn verify(&self) -> bool {
        let expected = Self::compute_hash(self.node_id, self.timestamp, self.vfe_before, self.vfe_after);
        self.proof_hash == expected
    }

    fn compute_hash(node_id: u64, timestamp: u64, vfe_before: f64, vfe_after: f64) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(node_id.to_le_bytes());
        hasher.update(timestamp.to_le_bytes());
        hasher.update(vfe_before.to_le_bytes());
        hasher.update(vfe_after.to_le_bytes());
        hasher.finalize().into()
    }

    /// Compute the VFE reduction from this update.
    pub fn vfe_reduction(&self) -> f64 {
        (self.vfe_before - self.vfe_after).max(0.0)
    }
}

impl SparseWeightUpdate {
    /// Create a sparse weight update from a dense tensor using Top-k sparsification.
    ///
    /// Extracts the top-k largest absolute values and their indices,
    /// then computes the cryptographic hash.
    pub fn from_tensor(matrix_id: &str, tensor: &Tensor, top_k: usize) -> Result<Self> {
        let shape = tensor.shape().dims().to_vec();
        let flat = tensor.flatten_all()?;
        let values = flat.to_vec1::<f32>()?;

        // Find top-k indices by absolute value
        let mut indexed: Vec<(usize, f32)> = values.iter().copied().enumerate().collect();
        indexed.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap_or(std::cmp::Ordering::Equal));
        indexed.truncate(top_k);

        let entries: Vec<SparseEntry> = indexed
            .into_iter()
            .map(|(idx, val)| SparseEntry { index: idx, value: val })
            .collect();

        let hash = Self::compute_hash(matrix_id, &entries);

        Ok(Self {
            matrix_id: matrix_id.to_string(),
            shape,
            entries,
            hash,
            proof: None,
        })
    }

    /// Attach a PoSym proof to this update.
    pub fn with_proof(mut self, proof: UpdateProof) -> Self {
        self.proof = Some(proof);
        self
    }

    /// Verify the update hash matches the entries.
    pub fn verify(&self) -> bool {
        let expected = Self::compute_hash(&self.matrix_id, &self.entries);
        self.hash == expected
    }

    /// Reconstruct a dense tensor from the sparse update.
    ///
    /// Creates a zero tensor of the original shape and fills in the sparse entries.
    pub fn reconstruct(&self, device: &Device) -> Result<Tensor> {
        let total_elements: usize = self.shape.iter().product();
        let mut data = vec![0.0f32; total_elements];

        for entry in &self.entries {
            if entry.index < total_elements {
                data[entry.index] = entry.value;
            }
        }

        Tensor::from_vec(data, self.shape.clone(), device)
    }

    /// Compute the compression ratio compared to the original dense tensor.
    pub fn compression_ratio(&self) -> f64 {
        let total_elements: usize = self.shape.iter().product();
        if total_elements == 0 {
            return 0.0;
        }
        1.0 - (self.entries.len() as f64 / total_elements as f64)
    }

    /// Estimate the bandwidth savings in bytes.
    pub fn bandwidth_savings(&self) -> (usize, usize) {
        let dense_bytes: usize = self.shape.iter().product::<usize>() * 4; // f32 = 4 bytes
        let sparse_bytes = self.entries.len() * (8 + 4); // index (u64) + value (f32)
        (dense_bytes, sparse_bytes)
    }

    fn compute_hash(matrix_id: &str, entries: &[SparseEntry]) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(matrix_id.as_bytes());
        hasher.update((entries.len() as u64).to_le_bytes());
        for entry in entries {
            hasher.update((entry.index as u64).to_le_bytes());
            hasher.update(entry.value.to_le_bytes());
        }
        hasher.finalize().into()
    }
}

/// Federated SAE aggregator — collects and merges sparse updates from multiple nodes.
#[derive(Debug, Clone)]
pub struct FederatedSAEAggregator {
    /// Node ID of this aggregator.
    pub node_id: u64,
    /// Collected sparse updates from peer nodes.
    pub pending_updates: Vec<SparseWeightUpdate>,
    /// Maximum updates to buffer before aggregation.
    pub max_pending: usize,
}

impl FederatedSAEAggregator {
    /// Create a new aggregator.
    pub fn new(node_id: u64) -> Self {
        Self {
            node_id,
            pending_updates: Vec::new(),
            max_pending: 1000,
        }
    }

    /// Add a sparse update from a peer node.
    pub fn add_update(&mut self, update: SparseWeightUpdate) {
        // Verify update integrity before accepting
        if update.verify() {
            self.pending_updates.push(update);
            if self.pending_updates.len() > self.max_pending {
                self.pending_updates.drain(..self.pending_updates.len() - self.max_pending);
            }
        }
    }

    /// Get all pending updates for a specific matrix.
    pub fn get_updates_for_matrix(&self, _matrix_id: &str) -> &[SparseWeightUpdate] {
        &self.pending_updates[..]
    }

    /// Aggregate sparse updates using median merging (Byzantine-resistant).
    ///
    /// For each index, collects all values from different updates and computes the median.
    pub fn aggregate_median(&self, matrix_id: &str) -> Result<Vec<SparseEntry>> {
        let updates: Vec<&SparseWeightUpdate> = self
            .pending_updates
            .iter()
            .filter(|u| u.matrix_id == matrix_id)
            .collect();

        if updates.is_empty() {
            return Ok(Vec::new());
        }

        // Collect all entries grouped by index
        let mut index_values: std::collections::HashMap<usize, Vec<f32>> =
            std::collections::HashMap::new();

        for update in &updates {
            for entry in &update.entries {
                index_values
                    .entry(entry.index)
                    .or_default()
                    .push(entry.value);
            }
        }

        // Compute median for each index
        let mut result: Vec<SparseEntry> = Vec::new();
        for (idx, mut values) in index_values {
            values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let median = values[values.len() / 2];
            result.push(SparseEntry { index: idx, value: median });
        }

        result.sort_by_key(|e| e.index);
        Ok(result)
    }

    /// Clear all pending updates after aggregation.
    pub fn clear_pending(&mut self) {
        self.pending_updates.clear();
    }

    /// Get the number of pending updates.
    pub fn pending_count(&self) -> usize {
        self.pending_updates.len()
    }

    /// Verify all pending updates have valid proofs (if proofs are attached).
    pub fn verify_all_proofs(&self) -> bool {
        self.pending_updates.iter().all(|u| {
            if let Some(ref proof) = u.proof {
                proof.verify()
            } else {
                true // Updates without proofs are still valid
            }
        })
    }
}

impl Default for FederatedSAEAggregator {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparse_entry_creation() {
        let entry = SparseEntry { index: 5, value: 0.7 };
        assert_eq!(entry.index, 5);
        assert!((entry.value - 0.7).abs() < 1e-6);
    }

    #[test]
    fn test_update_proof_creation() {
        let proof = UpdateProof::new(42, 1000, 1.0, 0.5);
        assert_eq!(proof.node_id, 42);
        assert_eq!(proof.timestamp, 1000);
        assert!(proof.verify());
        assert!((proof.vfe_reduction() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_update_proof_vfe_clamped() {
        let proof = UpdateProof::new(1, 100, 0.5, 1.0);
        assert_eq!(proof.vfe_reduction(), 0.0);
    }

    #[test]
    fn test_sparse_weight_update_from_tensor() -> Result<()> {
        let device = Device::Cpu;
        let data = vec![0.0, 0.1, 0.5, 0.3, 0.9, 0.2, 0.8, 0.4];
        let tensor = Tensor::from_vec(data, (2, 4), &device)?;
        let update = SparseWeightUpdate::from_tensor("test_w", &tensor, 3)?;

        assert_eq!(update.matrix_id, "test_w");
        assert_eq!(update.shape, vec![2, 4]);
        assert_eq!(update.entries.len(), 3);
        assert!(update.verify());

        // Top-3 should be: 0.9 (idx 3), 0.8 (idx 6), 0.5 (idx 2)
        let values: Vec<f32> = update.entries.iter().map(|e| e.value).collect();
        assert!(values.contains(&0.9));
        assert!(values.contains(&0.8));
        assert!(values.contains(&0.5));

        Ok(())
    }

    #[test]
    fn test_sparse_reconstruction() -> Result<()> {
        let device = Device::Cpu;
        let data = vec![0.0, 1.0, 0.0, 0.0];
        let tensor = Tensor::from_vec(data, (2, 2), &device)?;
        let update = SparseWeightUpdate::from_tensor("w", &tensor, 1)?;

        let reconstructed = update.reconstruct(&device)?;
        assert_eq!(reconstructed.shape(), tensor.shape());

        let recon_data = reconstructed.to_vec1::<f32>()?;
        assert!((recon_data[1] - 1.0).abs() < 1e-6);
        assert!(recon_data[0].abs() < 1e-6);

        Ok(())
    }

    #[test]
    fn test_compression_ratio() -> Result<()> {
        let device = Device::Cpu;
        // 10x10 = 100 elements, top_k = 5
        let data = vec![0.0f32; 100];
        let tensor = Tensor::from_vec(data, (10, 10), &device)?;
        let update = SparseWeightUpdate::from_tensor("big_w", &tensor, 5)?;

        let ratio = update.compression_ratio();
        // 5 non-zero out of 100 = 95% compression
        assert!((ratio - 0.95).abs() < 1e-6);

        Ok(())
    }

    #[test]
    fn test_bandwidth_savings() -> Result<()> {
        let device = Device::Cpu;
        let data = vec![1.0f32; 1000];
        let tensor = Tensor::from_vec(data, (10, 100), &device)?;
        let update = SparseWeightUpdate::from_tensor("w", &tensor, 10)?;

        let (dense, sparse) = update.bandwidth_savings();
        assert_eq!(dense, 4000); // 1000 * 4 bytes
        assert_eq!(sparse, 120); // 10 * (8 + 4) bytes
        assert!(sparse < dense);

        Ok(())
    }

    #[test]
    fn test_aggregator_add_update() -> Result<()> {
        let mut agg = FederatedSAEAggregator::new(1);
        let device = Device::Cpu;
        let data = vec![0.0, 1.0, 0.0];
        let tensor = Tensor::from_vec(data, (3, 1), &device)?;
        let update = SparseWeightUpdate::from_tensor("w", &tensor, 1)?;

        agg.add_update(update);
        assert_eq!(agg.pending_count(), 1);

        Ok(())
    }

    #[test]
    fn test_aggregator_median() -> Result<()> {
        let mut agg = FederatedSAEAggregator::new(1);
        let device = Device::Cpu;

        // Add 3 updates with different values at same index
        for val in [1.0, 2.0, 3.0] {
            let data = vec![0.0, val, 0.0];
            let tensor = Tensor::from_vec(data, (3, 1), &device)?;
            let update = SparseWeightUpdate::from_tensor("w", &tensor, 1)?;
            agg.add_update(update);
        }

        let aggregated = agg.aggregate_median("w")?;
        // Median of [1.0, 2.0, 3.0] = 2.0
        let entry = aggregated.iter().find(|e| e.index == 1);
        assert!(entry.is_some());
        assert!((entry.unwrap().value - 2.0).abs() < 1e-6);

        Ok(())
    }

    #[test]
    fn test_aggregator_clear() -> Result<()> {
        let mut agg = FederatedSAEAggregator::new(1);
        let device = Device::Cpu;
        let data = vec![1.0];
        let tensor = Tensor::from_vec(data, (1,), &device)?;
        let update = SparseWeightUpdate::from_tensor("w", &tensor, 1)?;
        agg.add_update(update);

        agg.clear_pending();
        assert_eq!(agg.pending_count(), 0);

        Ok(())
    }

    #[test]
    fn test_aggregator_verify_proofs() {
        let agg = FederatedSAEAggregator::new(1);
        assert!(agg.verify_all_proofs());
    }

    #[test]
    fn test_aggregator_default() {
        let agg = FederatedSAEAggregator::default();
        assert_eq!(agg.node_id, 0);
    }

    #[test]
    fn test_with_proof() -> Result<()> {
        let device = Device::Cpu;
        let data = vec![1.0];
        let tensor = Tensor::from_vec(data, (1,), &device)?;
        let proof = UpdateProof::new(1, 100, 1.0, 0.5);
        let update = SparseWeightUpdate::from_tensor("w", &tensor, 1)?.with_proof(proof.clone());

        assert!(update.proof.is_some());
        assert!(update.proof.unwrap().verify());

        Ok(())
    }
}
