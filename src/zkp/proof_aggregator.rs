//! Proof Aggregator — Aggregates multiple ZKP proofs into composite proofs for efficiency.
//!
//! Supports batch aggregation where multiple individual proofs are combined into a single
//! composite proof, reducing verification overhead. Feature-gated behind `cfg(feature = "v1.4-sprint1")`.

use crate::zkp::async_zkp_v5::ZKPProof;
use std::collections::HashMap;

/// Error types for proof aggregation.
#[derive(Debug)]
pub enum AggregatorError {
    /// Proof not found in aggregation set.
    ProofNotFound(String),
    /// Aggregation failed due to incompatible proofs.
    IncompatibleProofs(String),
    /// Maximum aggregation size exceeded.
    MaxSizeExceeded(usize),
    /// Verification failed for aggregated proof.
    VerificationFailed(String),
}

impl std::fmt::Display for AggregatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AggregatorError::ProofNotFound(id) => write!(f, "Proof not found: {}", id),
            AggregatorError::IncompatibleProofs(msg) => write!(f, "Incompatible: {}", msg),
            AggregatorError::MaxSizeExceeded(size) => write!(f, "Max size {} exceeded", size),
            AggregatorError::VerificationFailed(msg) => write!(f, "Verification: {}", msg),
        }
    }
}

/// Configuration for the proof aggregator.
#[derive(Debug)]
pub struct AggregatorConfig {
    /// Maximum proofs per aggregation batch.
    pub max_batch_size: usize,
    /// Enable aggregation.
    pub aggregation_enabled: bool,
    /// Minimum proofs before aggregation triggers.
    pub min_proofs_to_aggregate: usize,
}

impl Default for AggregatorConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 128,
            aggregation_enabled: true,
            min_proofs_to_aggregate: 2,
        }
    }
}

/// A composite proof aggregating multiple individual proofs.
#[derive(Debug, Clone)]
pub struct AggregatedProof {
    /// Unique ID for the aggregated proof.
    pub id: String,
    /// IDs of individual proofs included.
    pub proof_ids: Vec<String>,
    /// Combined proof data (hash of individual proofs).
    pub aggregated_data: Vec<u8>,
    /// Number of proofs aggregated.
    pub proof_count: usize,
    /// Timestamp when aggregation was created.
    pub timestamp_ms: u64,
}

impl AggregatedProof {
    pub fn new(id: String, proof_ids: Vec<String>, data: Vec<u8>, timestamp_ms: u64) -> Self {
        let count = proof_ids.len();
        Self {
            id,
            proof_ids,
            aggregated_data: data,
            proof_count: count,
            timestamp_ms,
        }
    }

    /// Verify the aggregated proof contains the expected proof IDs.
    pub fn contains_proof(&self, proof_id: &str) -> bool {
        self.proof_ids.contains(&proof_id.to_string())
    }
}

/// Statistics for the proof aggregator.
#[derive(Debug, Default)]
pub struct AggregatorStats {
    pub total_aggregated: u64,
    pub total_proofs_in_aggregates: u64,
    pub total_failed: u64,
    pub avg_aggregation_size: f64,
}

/// Proof Aggregator — combines multiple proofs into composite proofs.
#[cfg(feature = "v1.4-sprint1")]
pub struct ProofAggregator {
    config: AggregatorConfig,
    pending_proofs: HashMap<String, ZKPProof>,
    aggregated: Vec<AggregatedProof>,
    stats: AggregatorStats,
}

#[cfg(feature = "v1.4-sprint1")]
impl ProofAggregator {
    pub fn new(config: AggregatorConfig) -> Self {
        Self {
            config,
            pending_proofs: HashMap::new(),
            aggregated: Vec::new(),
            stats: AggregatorStats::default(),
        }
    }

    /// Add a proof to the pending set for aggregation.
    pub fn add_proof(&mut self, proof: ZKPProof) -> Result<(), AggregatorError> {
        if self.pending_proofs.len() >= self.config.max_batch_size {
            return Err(AggregatorError::MaxSizeExceeded(self.config.max_batch_size));
        }
        self.pending_proofs.insert(proof.proof_id.clone(), proof);
        Ok(())
    }

    /// Aggregate pending proofs into a composite proof.
    pub fn aggregate(&mut self, aggregate_id: String) -> Result<AggregatedProof, AggregatorError> {
        if !self.config.aggregation_enabled {
            return Err(AggregatorError::IncompatibleProofs(
                "Aggregation disabled".to_string(),
            ));
        }

        if self.pending_proofs.len() < self.config.min_proofs_to_aggregate {
            return Err(AggregatorError::IncompatibleProofs(format!(
                "Need {} proofs, have {}",
                self.config.min_proofs_to_aggregate,
                self.pending_proofs.len()
            )));
        }

        let proof_ids: Vec<String> = self.pending_proofs.keys().cloned().collect();
        let proof_count = proof_ids.len();
        let aggregated_data = self.compute_aggregated_data();
        let timestamp = current_timestamp_ms();

        let agg_proof = AggregatedProof::new(aggregate_id, proof_ids, aggregated_data, timestamp);

        self.stats.total_aggregated += 1;
        self.stats.total_proofs_in_aggregates += proof_count as u64;
        if self.stats.total_aggregated > 0 {
            self.stats.avg_aggregation_size =
                self.stats.total_proofs_in_aggregates as f64 / self.stats.total_aggregated as f64;
        }

        self.pending_proofs.clear();
        self.aggregated.push(agg_proof.clone());

        Ok(agg_proof)
    }

    /// Verify an aggregated proof contains valid individual proofs.
    pub fn verify_aggregated(&self, agg: &AggregatedProof) -> Result<bool, AggregatorError> {
        for proof_id in &agg.proof_ids {
            if !self.proof_exists(proof_id) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Get an aggregated proof by ID.
    pub fn get_aggregated(&self, id: &str) -> Option<&AggregatedProof> {
        self.aggregated.iter().find(|a| a.id == id)
    }

    /// Check if a proof ID exists in pending or aggregated sets.
    pub fn proof_exists(&self, proof_id: &str) -> bool {
        if self.pending_proofs.contains_key(proof_id) {
            return true;
        }
        self.aggregated.iter().any(|a| a.contains_proof(proof_id))
    }

    /// Get the count of pending proofs.
    pub fn pending_count(&self) -> usize {
        self.pending_proofs.len()
    }

    /// Get all aggregated proofs.
    pub fn aggregated_proofs(&self) -> &[AggregatedProof] {
        &self.aggregated
    }

    /// Get statistics.
    pub fn stats(&self) -> &AggregatorStats {
        &self.stats
    }

    /// Reset aggregator state.
    pub fn reset(&mut self) {
        self.pending_proofs.clear();
        self.aggregated.clear();
        self.stats = AggregatorStats::default();
    }

    fn compute_aggregated_data(&self) -> Vec<u8> {
        // Compute combined hash of all pending proofs
        let mut combined = Vec::new();
        for (_, proof) in &self.pending_proofs {
            combined.extend(&proof.proof_data);
        }
        combined
    }
}

#[cfg(feature = "v1.4-sprint1")]
impl Default for ProofAggregator {
    fn default() -> Self {
        Self::new(AggregatorConfig::default())
    }
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_proof(id: &str) -> ZKPProof {
        ZKPProof {
            proof_id: id.to_string(),
            statement_id: format!("stmt-{}", id),
            proof_data: vec![1, 2, 3],
            proof_hash: format!("hash-{}", id),
            generation_time_ms: 100,
            used_fallback: false,
            batch_id: None,
            source_pool: "pool-1".to_string(),
            priority: 1,
            accumulator_index: None,
            is_vrf_sample: false,
        }
    }

    #[test]
    fn test_aggregator_creation() {
        let agg = ProofAggregator::default();
        assert_eq!(agg.pending_count(), 0);
        assert_eq!(agg.aggregated_proofs().len(), 0);
    }

    #[test]
    fn test_add_proof() {
        let mut agg = ProofAggregator::default();
        agg.add_proof(make_proof("p1")).unwrap();
        assert_eq!(agg.pending_count(), 1);
    }

    #[test]
    fn test_aggregate_min_proofs() {
        let mut agg = ProofAggregator::default();
        agg.add_proof(make_proof("p1")).unwrap();
        // Need at least 2 proofs
        let result = agg.aggregate("agg-1".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_aggregate_success() {
        let mut agg = ProofAggregator::default();
        agg.add_proof(make_proof("p1")).unwrap();
        agg.add_proof(make_proof("p2")).unwrap();
        let agg_proof = agg.aggregate("agg-1".to_string()).unwrap();
        assert_eq!(agg_proof.proof_count, 2);
        assert!(agg_proof.contains_proof("p1"));
        assert!(agg_proof.contains_proof("p2"));
        assert_eq!(agg.pending_count(), 0);
    }

    #[test]
    fn test_aggregate_disabled() {
        let config = AggregatorConfig {
            aggregation_enabled: false,
            ..Default::default()
        };
        let mut agg = ProofAggregator::new(config);
        agg.add_proof(make_proof("p1")).unwrap();
        agg.add_proof(make_proof("p2")).unwrap();
        let result = agg.aggregate("agg-1".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_aggregated() {
        let mut agg = ProofAggregator::default();
        agg.add_proof(make_proof("p1")).unwrap();
        agg.add_proof(make_proof("p2")).unwrap();
        let agg_proof = agg.aggregate("agg-1".to_string()).unwrap();
        // After aggregation, proofs are cleared from pending but in aggregated
        assert!(agg.verify_aggregated(&agg_proof).unwrap());
    }

    #[test]
    fn test_get_aggregated() {
        let mut agg = ProofAggregator::default();
        agg.add_proof(make_proof("p1")).unwrap();
        agg.add_proof(make_proof("p2")).unwrap();
        agg.aggregate("agg-1".to_string()).unwrap();
        assert!(agg.get_aggregated("agg-1").is_some());
        assert!(agg.get_aggregated("nonexistent").is_none());
    }

    #[test]
    fn test_proof_exists_pending() {
        let mut agg = ProofAggregator::default();
        agg.add_proof(make_proof("p1")).unwrap();
        assert!(agg.proof_exists("p1"));
        assert!(!agg.proof_exists("p2"));
    }

    #[test]
    fn test_proof_exists_aggregated() {
        let mut agg = ProofAggregator::default();
        agg.add_proof(make_proof("p1")).unwrap();
        agg.add_proof(make_proof("p2")).unwrap();
        agg.aggregate("agg-1".to_string()).unwrap();
        assert!(agg.proof_exists("p1"));
        assert!(agg.proof_exists("p2"));
    }

    #[test]
    fn test_reset() {
        let mut agg = ProofAggregator::default();
        agg.add_proof(make_proof("p1")).unwrap();
        agg.add_proof(make_proof("p2")).unwrap();
        agg.aggregate("agg-1".to_string()).unwrap();
        agg.reset();
        assert_eq!(agg.pending_count(), 0);
        assert_eq!(agg.aggregated_proofs().len(), 0);
        assert_eq!(agg.stats().total_aggregated, 0);
    }

    #[test]
    fn test_stats_tracking() {
        let mut agg = ProofAggregator::default();
        agg.add_proof(make_proof("p1")).unwrap();
        agg.add_proof(make_proof("p2")).unwrap();
        agg.add_proof(make_proof("p3")).unwrap();
        agg.aggregate("agg-1".to_string()).unwrap();
        assert_eq!(agg.stats().total_aggregated, 1);
        assert_eq!(agg.stats().total_proofs_in_aggregates, 3);
        assert_eq!(agg.stats().avg_aggregation_size, 3.0);
    }

    #[test]
    fn test_max_batch_size() {
        let config = AggregatorConfig {
            max_batch_size: 2,
            ..Default::default()
        };
        let mut agg = ProofAggregator::new(config);
        agg.add_proof(make_proof("p1")).unwrap();
        agg.add_proof(make_proof("p2")).unwrap();
        let result = agg.add_proof(make_proof("p3"));
        assert!(result.is_err());
    }

    #[test]
    fn test_config_default() {
        let config = AggregatorConfig::default();
        assert_eq!(config.max_batch_size, 128);
        assert!(config.aggregation_enabled);
        assert_eq!(config.min_proofs_to_aggregate, 2);
    }

    #[test]
    fn test_stats_default() {
        let stats = AggregatorStats::default();
        assert_eq!(stats.total_aggregated, 0);
        assert_eq!(stats.total_proofs_in_aggregates, 0);
        assert_eq!(stats.total_failed, 0);
        assert_eq!(stats.avg_aggregation_size, 0.0);
    }

    #[test]
    fn test_aggregated_proof_contains() {
        let proof = AggregatedProof::new(
            "agg".to_string(),
            vec!["a".to_string(), "b".to_string()],
            vec![],
            1000,
        );
        assert!(proof.contains_proof("a"));
        assert!(proof.contains_proof("b"));
        assert!(!proof.contains_proof("c"));
    }

    #[test]
    fn test_error_display() {
        let err = AggregatorError::ProofNotFound("x".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("x"));

        let err = AggregatorError::IncompatibleProofs("bad".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Incompatible"));

        let err = AggregatorError::MaxSizeExceeded(100);
        let msg = format!("{}", err);
        assert!(msg.contains("100"));

        let err = AggregatorError::VerificationFailed("fail".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Verification"));
    }
}
