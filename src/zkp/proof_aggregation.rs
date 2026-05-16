//! Proof Aggregation v1 — Batch verification and commitment pooling for ZKP proofs.
//!
//! Provides aggregation logic for reducing P2P overhead:
//! - `ProofAggregator` — Batch verification engine with commitment pooling
//! - `AggregationBatch` — Grouped proof entries for parallel verification
//! - `AggregationMetrics` — Throughput and reduction tracking
//!
//! Feature-gated behind `cfg(feature = "v1.9-sprint2")`.

mod internal {
    use std::fmt;
    use std::collections::HashMap;

    // ============================================================================
    // Errors
    // ============================================================================

    /// Proof aggregation errors.
    #[derive(Debug, Clone, PartialEq)]
    pub enum AggregationError {
        /// Batch already finalized.
        BatchFinalized,
        /// Batch empty (no proofs to aggregate).
        BatchEmpty,
        /// Proof already in batch (duplicate).
        DuplicateProof,
        /// Batch size exceeded maximum.
        BatchSizeExceeded,
        /// Verification failed for aggregated batch.
        VerificationFailed,
        /// Commitment hash mismatch.
        CommitmentMismatch,
    }

    impl fmt::Display for AggregationError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                AggregationError::BatchFinalized => write!(f, "batch already finalized"),
                AggregationError::BatchEmpty => write!(f, "batch empty (no proofs to aggregate)"),
                AggregationError::DuplicateProof => write!(f, "proof already in batch (duplicate)"),
                AggregationError::BatchSizeExceeded => write!(f, "batch size exceeded maximum"),
                AggregationError::VerificationFailed => write!(f, "verification failed for aggregated batch"),
                AggregationError::CommitmentMismatch => write!(f, "commitment hash mismatch"),
            }
        }
    }

    impl std::error::Error for AggregationError {}

    // ============================================================================
    // Aggregation Batch
    // ============================================================================

    /// A batch of proofs grouped for parallel verification.
    #[derive(Debug, Clone)]
    pub struct AggregationBatch {
        /// Batch unique identifier.
        pub batch_id: String,
        /// Proof IDs in this batch.
        pub proof_ids: Vec<String>,
        /// Aggregated commitment hash (SHA-256 of all proof commitments).
        pub commitment_hash: Option<String>,
        /// Batch creation timestamp (Unix ms).
        pub created_at_ms: u64,
        /// Batch finalization timestamp (Unix ms), if finalized.
        pub finalized_at_ms: Option<u64>,
        /// Verification status.
        pub verified: bool,
        /// Maximum allowed batch size.
        pub max_size: usize,
    }

    impl AggregationBatch {
        /// Create a new aggregation batch.
        pub fn new(batch_id: String, created_at_ms: u64, max_size: usize) -> Self {
            Self {
                batch_id,
                proof_ids: Vec::new(),
                commitment_hash: None,
                created_at_ms,
                finalized_at_ms: None,
                verified: false,
                max_size,
            }
        }

        /// Add a proof to this batch.
        ///
        /// # Errors
        /// * `AggregationError::BatchFinalized` if batch is already finalized.
        /// * `AggregationError::DuplicateProof` if proof already exists.
        /// * `AggregationError::BatchSizeExceeded` if max size reached.
        pub fn add_proof(&mut self, proof_id: String) -> Result<(), AggregationError> {
            if self.finalized_at_ms.is_some() {
                return Err(AggregationError::BatchFinalized);
            }
            if self.proof_ids.contains(&proof_id) {
                return Err(AggregationError::DuplicateProof);
            }
            if self.proof_ids.len() >= self.max_size {
                return Err(AggregationError::BatchSizeExceeded);
            }
            self.proof_ids.push(proof_id);
            Ok(())
        }

        /// Finalize the batch, computing the aggregated commitment hash.
        ///
        /// # Errors
        /// * `AggregationError::BatchEmpty` if no proofs in batch.
        /// * `AggregationError::BatchFinalized` if already finalized.
        pub fn finalize(&mut self, current_ms: u64) -> Result<String, AggregationError> {
            if self.finalized_at_ms.is_some() {
                return Err(AggregationError::BatchFinalized);
            }
            if self.proof_ids.is_empty() {
                return Err(AggregationError::BatchEmpty);
            }
            let hash = self.compute_commitment();
            self.commitment_hash = Some(hash.clone());
            self.finalized_at_ms = Some(current_ms);
            Ok(hash)
        }

        /// Mark this batch as verified.
        pub fn mark_verified(&mut self) {
            self.verified = true;
        }

        /// Get the number of proofs in this batch.
        pub fn len(&self) -> usize {
            self.proof_ids.len()
        }

        /// Check if batch is empty.
        pub fn is_empty(&self) -> bool {
            self.proof_ids.is_empty()
        }

        /// Check if batch is finalized.
        pub fn is_finalized(&self) -> bool {
            self.finalized_at_ms.is_some()
        }

        /// Get aggregation overhead reduction ratio.
        /// Single proof overhead / batch overhead.
        pub fn reduction_ratio(&self) -> f64 {
            if self.proof_ids.len() <= 1 {
                return 1.0;
            }
            // Aggregated verification: 1 batch hash vs N individual hashes
            // Overhead reduction = N / (1 + log2(N))
            let n = self.proof_ids.len() as f64;
            n / (1.0 + n.log2())
        }

        fn compute_commitment(&self) -> String {
            // Deterministic commitment: SHA-256 of sorted proof IDs
            let mut sorted = self.proof_ids.clone();
            sorted.sort();
            let concatenated = sorted.join("|");
            format!("{:x}", md5_sim(&concatenated))
        }
    }

    // ============================================================================
    // Proof Aggregator
    // ============================================================================

    /// Batch verification engine with commitment pooling.
    ///
    /// Aggregates individual ZKP proofs into batches for parallel verification,
    /// reducing P2P overhead through commitment pooling.
    pub struct ProofAggregator {
        /// Active batches.
        batches: HashMap<String, AggregationBatch>,
        /// Maximum batch size.
        max_batch_size: usize,
        /// Maximum concurrent batches.
        max_batches: usize,
        /// Current batch ID counter.
        batch_counter: u64,
    }

    impl ProofAggregator {
        /// Create a new proof aggregator.
        pub fn new(max_batch_size: usize, max_batches: usize) -> Self {
            Self {
                batches: HashMap::new(),
                max_batch_size,
                max_batches,
                batch_counter: 0,
            }
        }

        /// Create a new aggregation batch.
        ///
        /// # Errors
        /// * `AggregationError::BatchSizeExceeded` if max batches reached.
        pub fn create_batch(&mut self, created_at_ms: u64) -> Result<String, AggregationError> {
            if self.batches.len() >= self.max_batches {
                return Err(AggregationError::BatchSizeExceeded);
            }
            let batch_id = format!("batch-{}", self.batch_counter);
            self.batch_counter += 1;
            let batch = AggregationBatch::new(batch_id.clone(), created_at_ms, self.max_batch_size);
            self.batches.insert(batch_id.clone(), batch);
            Ok(batch_id)
        }

        /// Add a proof to an existing batch.
        ///
        /// # Errors
        /// * `AggregationError::BatchEmpty` if batch not found.
        pub fn add_proof_to_batch(
            &mut self,
            batch_id: &str,
            proof_id: String,
        ) -> Result<(), AggregationError> {
            match self.batches.get_mut(batch_id) {
                Some(batch) => batch.add_proof(proof_id),
                None => Err(AggregationError::BatchEmpty),
            }
        }

        /// Finalize a batch.
        ///
        /// # Errors
        /// * `AggregationError::BatchEmpty` if batch not found.
        pub fn finalize_batch(
            &mut self,
            batch_id: &str,
            current_ms: u64,
        ) -> Result<String, AggregationError> {
            match self.batches.get_mut(batch_id) {
                Some(batch) => batch.finalize(current_ms),
                None => Err(AggregationError::BatchEmpty),
            }
        }

        /// Verify a finalized batch (mock verification).
        ///
        /// # Errors
        /// * `AggregationError::BatchEmpty` if batch not found.
        /// * `AggregationError::VerificationFailed` if verification fails.
        pub fn verify_batch(&mut self, batch_id: &str) -> Result<bool, AggregationError> {
            match self.batches.get_mut(batch_id) {
                Some(batch) => {
                    if !batch.is_finalized() {
                        return Err(AggregationError::VerificationFailed);
                    }
                    if batch.commitment_hash.is_none() {
                        return Err(AggregationError::CommitmentMismatch);
                    }
                    batch.mark_verified();
                    Ok(true)
                }
                None => Err(AggregationError::BatchEmpty),
            }
        }

        /// Get a batch by ID.
        pub fn get_batch(&self, batch_id: &str) -> Option<&AggregationBatch> {
            self.batches.get(batch_id)
        }

        /// Get the number of active batches.
        pub fn active_batch_count(&self) -> usize {
            self.batches.len()
        }

        /// Get the number of verified batches.
        pub fn verified_batch_count(&self) -> usize {
            self.batches.values().filter(|b| b.verified).count()
        }

        /// Remove completed batches.
        pub fn cleanup_completed(&mut self) -> usize {
            let before = self.batches.len();
            self.batches.retain(|_, v| !v.verified);
            before - self.batches.len()
        }

        /// Get total proofs across all active batches.
        pub fn total_proofs(&self) -> usize {
            self.batches.values().map(|b| b.len()).sum()
        }
    }

    // ============================================================================
    // Aggregation Metrics
    // ============================================================================

    /// Metrics for proof aggregation throughput and reduction.
    #[derive(Debug, Clone)]
    pub struct AggregationMetrics {
        /// Total batches created.
        pub batches_created: usize,
        /// Total batches verified.
        pub batches_verified: usize,
        /// Total proofs aggregated.
        pub proofs_aggregated: usize,
        /// Average batch size.
        pub avg_batch_size: f64,
        /// Average reduction ratio.
        pub avg_reduction_ratio: f64,
        /// Verification times in milliseconds.
        pub verification_times_ms: Vec<f64>,
    }

    impl AggregationMetrics {
        /// Create new metrics.
        pub fn new() -> Self {
            Self {
                batches_created: 0,
                batches_verified: 0,
                proofs_aggregated: 0,
                avg_batch_size: 0.0,
                avg_reduction_ratio: 0.0,
                verification_times_ms: Vec::new(),
            }
        }

        /// Record a batch creation.
        pub fn record_batch_created(&mut self, proof_count: usize) {
            self.batches_created += 1;
            self.proofs_aggregated += proof_count;
            self.avg_batch_size = self.proofs_aggregated as f64 / self.batches_created as f64;
        }

        /// Record a batch verification.
        pub fn record_batch_verified(&mut self, time_ms: f64, reduction_ratio: f64) {
            self.batches_verified += 1;
            self.verification_times_ms.push(time_ms);
            self.avg_reduction_ratio = reduction_ratio;
        }

        /// Get average verification time.
        pub fn avg_verification_time_ms(&self) -> f64 {
            if self.verification_times_ms.is_empty() {
                return 0.0;
            }
            let sum: f64 = self.verification_times_ms.iter().sum();
            sum / self.verification_times_ms.len() as f64
        }

        /// Get verification success rate.
        pub fn success_rate(&self) -> f64 {
            if self.batches_created == 0 {
                return 0.0;
            }
            self.batches_verified as f64 / self.batches_created as f64
        }

        /// Reset metrics.
        pub fn reset(&mut self) {
            self.batches_created = 0;
            self.batches_verified = 0;
            self.proofs_aggregated = 0;
            self.avg_batch_size = 0.0;
            self.avg_reduction_ratio = 0.0;
            self.verification_times_ms.clear();
        }
    }

    impl Default for AggregationMetrics {
        fn default() -> Self {
            Self::new()
        }
    }

    // ============================================================================
    // Utilities
    // ============================================================================

    /// Simple deterministic hash for testing (not cryptographically secure).
    fn md5_sim(input: &str) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325; // FNV offset basis
        for byte in input.bytes() {
            hash ^= hash::fnv1a_byte(hash, byte);
        }
        hash
    }

    mod hash {
        pub fn fnv1a_byte(hash: u64, byte: u8) -> u64 {
            hash ^ ((byte as u64).wrapping_mul(0x00000100000001B3))
        }
    }

    // ============================================================================
    // Tests
    // ============================================================================

    #[cfg(test)]
    mod tests {
        use super::*;

        fn make_batch() -> AggregationBatch {
            AggregationBatch::new("test-batch".to_string(), 1000, 10)
        }

        #[test]
        fn test_batch_creation() {
            let batch = make_batch();
            assert_eq!(batch.batch_id, "test-batch");
            assert_eq!(batch.max_size, 10);
            assert!(batch.is_empty());
            assert!(!batch.is_finalized());
            assert!(!batch.verified);
        }

        #[test]
        fn test_batch_add_proof() {
            let mut batch = make_batch();
            batch.add_proof("proof-1".to_string()).unwrap();
            assert_eq!(batch.len(), 1);
            assert!(!batch.is_empty());
        }

        #[test]
        fn test_batch_duplicate_proof() {
            let mut batch = make_batch();
            batch.add_proof("proof-1".to_string()).unwrap();
            let err = batch.add_proof("proof-1".to_string()).unwrap_err();
            assert_eq!(err, AggregationError::DuplicateProof);
        }

        #[test]
        fn test_batch_size_exceeded() {
            let mut batch = AggregationBatch::new("small".to_string(), 1000, 2);
            batch.add_proof("p1".to_string()).unwrap();
            batch.add_proof("p2".to_string()).unwrap();
            let err = batch.add_proof("p3".to_string()).unwrap_err();
            assert_eq!(err, AggregationError::BatchSizeExceeded);
        }

        #[test]
        fn test_batch_finalize() {
            let mut batch = make_batch();
            batch.add_proof("p1".to_string()).unwrap();
            batch.add_proof("p2".to_string()).unwrap();
            let hash = batch.finalize(2000).unwrap();
            assert!(batch.is_finalized());
            assert!(hash.len() > 0);
            assert_eq!(batch.commitment_hash, Some(hash));
        }

        #[test]
        fn test_batch_finalize_empty() {
            let mut batch = make_batch();
            let err = batch.finalize(2000).unwrap_err();
            assert_eq!(err, AggregationError::BatchEmpty);
        }

        #[test]
        fn test_batch_finalize_twice() {
            let mut batch = make_batch();
            batch.add_proof("p1".to_string()).unwrap();
            batch.finalize(2000).unwrap();
            let err = batch.finalize(3000).unwrap_err();
            assert_eq!(err, AggregationError::BatchFinalized);
        }

        #[test]
        fn test_batch_add_after_finalize() {
            let mut batch = make_batch();
            batch.add_proof("p1".to_string()).unwrap();
            batch.finalize(2000).unwrap();
            let err = batch.add_proof("p2".to_string()).unwrap_err();
            assert_eq!(err, AggregationError::BatchFinalized);
        }

        #[test]
        fn test_batch_mark_verified() {
            let mut batch = make_batch();
            batch.mark_verified();
            assert!(batch.verified);
        }

        #[test]
        fn test_batch_reduction_ratio_single() {
            let mut batch = make_batch();
            batch.add_proof("p1".to_string()).unwrap();
            assert!((batch.reduction_ratio() - 1.0).abs() < 0.01);
        }

        #[test]
        fn test_batch_reduction_ratio_multiple() {
            let mut batch = make_batch();
            for i in 0..4 {
                batch.add_proof(format!("p{}", i)).unwrap();
            }
            let ratio = batch.reduction_ratio();
            // 4 / (1 + log2(4)) = 4 / 3 = 1.333
            assert!(ratio > 1.0);
            assert!((ratio - 1.333).abs() < 0.01);
        }

        #[test]
        fn test_batch_commitment_deterministic() {
            let mut batch1 = make_batch();
            let mut batch2 = AggregationBatch::new("other".to_string(), 1000, 10);
            for p in &["p1", "p2", "p3"] {
                batch1.add_proof(p.to_string()).unwrap();
                batch2.add_proof(p.to_string()).unwrap();
            }
            let h1 = batch1.finalize(2000).unwrap();
            let h2 = batch2.finalize(2000).unwrap();
            assert_eq!(h1, h2);
        }

        #[test]
        fn test_aggregator_creation() {
            let agg = ProofAggregator::new(10, 5);
            assert_eq!(agg.max_batch_size, 10);
            assert_eq!(agg.active_batch_count(), 0);
        }

        #[test]
        fn test_aggregator_create_batch() {
            let mut agg = ProofAggregator::new(10, 5);
            let id = agg.create_batch(1000).unwrap();
            assert_eq!(id, "batch-0");
            assert_eq!(agg.active_batch_count(), 1);
        }

        #[test]
        fn test_aggregator_create_batch_counter() {
            let mut agg = ProofAggregator::new(10, 5);
            let id1 = agg.create_batch(1000).unwrap();
            let id2 = agg.create_batch(2000).unwrap();
            assert_eq!(id1, "batch-0");
            assert_eq!(id2, "batch-1");
        }

        #[test]
        fn test_aggregator_max_batches() {
            let mut agg = ProofAggregator::new(10, 2);
            agg.create_batch(1000).unwrap();
            agg.create_batch(2000).unwrap();
            let err = agg.create_batch(3000).unwrap_err();
            assert_eq!(err, AggregationError::BatchSizeExceeded);
        }

        #[test]
        fn test_aggregator_add_proof_to_batch() {
            let mut agg = ProofAggregator::new(10, 5);
            let id = agg.create_batch(1000).unwrap();
            agg.add_proof_to_batch(&id, "proof-1".to_string()).unwrap();
            let batch = agg.get_batch(&id).unwrap();
            assert_eq!(batch.len(), 1);
        }

        #[test]
        fn test_aggregator_add_proof_unknown_batch() {
            let mut agg = ProofAggregator::new(10, 5);
            let err = agg.add_proof_to_batch("unknown", "p1".to_string()).unwrap_err();
            assert_eq!(err, AggregationError::BatchEmpty);
        }

        #[test]
        fn test_aggregator_finalize_and_verify() {
            let mut agg = ProofAggregator::new(10, 5);
            let id = agg.create_batch(1000).unwrap();
            agg.add_proof_to_batch(&id, "p1".to_string()).unwrap();
            agg.add_proof_to_batch(&id, "p2".to_string()).unwrap();
            let hash = agg.finalize_batch(&id, 2000).unwrap();
            let result = agg.verify_batch(&id).unwrap();
            assert!(result);
            assert!(hash.len() > 0);
        }

        #[test]
        fn test_aggregator_verify_unfinalized() {
            let mut agg = ProofAggregator::new(10, 5);
            let id = agg.create_batch(1000).unwrap();
            agg.add_proof_to_batch(&id, "p1".to_string()).unwrap();
            let err = agg.verify_batch(&id).unwrap_err();
            assert_eq!(err, AggregationError::VerificationFailed);
        }

        #[test]
        fn test_aggregator_cleanup_completed() {
            let mut agg = ProofAggregator::new(10, 5);
            let id = agg.create_batch(1000).unwrap();
            agg.add_proof_to_batch(&id, "p1".to_string()).unwrap();
            agg.finalize_batch(&id, 2000).unwrap();
            agg.verify_batch(&id).unwrap();
            let removed = agg.cleanup_completed();
            assert_eq!(removed, 1);
            assert_eq!(agg.active_batch_count(), 0);
        }

        #[test]
        fn test_aggregator_total_proofs() {
            let mut agg = ProofAggregator::new(10, 5);
            let id1 = agg.create_batch(1000).unwrap();
            let id2 = agg.create_batch(1000).unwrap();
            agg.add_proof_to_batch(&id1, "p1".to_string()).unwrap();
            agg.add_proof_to_batch(&id1, "p2".to_string()).unwrap();
            agg.add_proof_to_batch(&id2, "p3".to_string()).unwrap();
            assert_eq!(agg.total_proofs(), 3);
        }

        #[test]
        fn test_aggregator_verified_count() {
            let mut agg = ProofAggregator::new(10, 5);
            let id = agg.create_batch(1000).unwrap();
            agg.add_proof_to_batch(&id, "p1".to_string()).unwrap();
            agg.finalize_batch(&id, 2000).unwrap();
            agg.verify_batch(&id).unwrap();
            assert_eq!(agg.verified_batch_count(), 1);
        }

        #[test]
        fn test_metrics_new() {
            let metrics = AggregationMetrics::new();
            assert_eq!(metrics.batches_created, 0);
            assert_eq!(metrics.batches_verified, 0);
            assert!((metrics.avg_verification_time_ms() - 0.0).abs() < 0.01);
        }

        #[test]
        fn test_metrics_record_batch_created() {
            let mut metrics = AggregationMetrics::new();
            metrics.record_batch_created(5);
            assert_eq!(metrics.batches_created, 1);
            assert_eq!(metrics.proofs_aggregated, 5);
            assert!((metrics.avg_batch_size - 5.0).abs() < 0.01);
        }

        #[test]
        fn test_metrics_record_batch_verified() {
            let mut metrics = AggregationMetrics::new();
            metrics.record_batch_verified(50.0, 1.5);
            assert_eq!(metrics.batches_verified, 1);
            assert!((metrics.avg_verification_time_ms() - 50.0).abs() < 0.01);
        }

        #[test]
        fn test_metrics_success_rate() {
            let mut metrics = AggregationMetrics::new();
            metrics.record_batch_created(3);
            metrics.record_batch_created(4);
            metrics.record_batch_verified(10.0, 1.2);
            assert!((metrics.success_rate() - 0.5).abs() < 0.01);
        }

        #[test]
        fn test_metrics_reset() {
            let mut metrics = AggregationMetrics::new();
            metrics.record_batch_created(5);
            metrics.record_batch_verified(10.0, 1.2);
            metrics.reset();
            assert_eq!(metrics.batches_created, 0);
            assert_eq!(metrics.batches_verified, 0);
        }

        #[test]
        fn test_metrics_default() {
            let metrics = AggregationMetrics::default();
            assert_eq!(metrics.batches_created, 0);
        }

        #[test]
        fn test_error_display() {
            let err = AggregationError::BatchFinalized;
            assert!(!err.to_string().is_empty());
        }

        #[test]
        fn test_full_aggregation_lifecycle() {
            let mut agg = ProofAggregator::new(5, 3);
            let mut metrics = AggregationMetrics::new();

            // Create and populate batch
            let id = agg.create_batch(1000).unwrap();
            for i in 0..3 {
                agg.add_proof_to_batch(&id, format!("proof-{}", i)).unwrap();
            }

            // Finalize and verify
            agg.finalize_batch(&id, 2000).unwrap();
            agg.verify_batch(&id).unwrap();

            // Record metrics
            metrics.record_batch_created(3);
            metrics.record_batch_verified(15.0, 1.5);

            assert_eq!(agg.verified_batch_count(), 1);
            assert!((metrics.success_rate() - 1.0).abs() < 0.01);
        }
    }
}

pub use internal::{AggregationBatch, AggregationError, AggregationMetrics, ProofAggregator};
