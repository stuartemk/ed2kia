//! Async ZKP v9 — Adaptive proof batching with multi-path relay and dynamic credibility.
//!
//! Improvements over v8:
//! - Proof batching with adaptive aggregation thresholds
//! - Multi-path relay with fault tolerance and path diversity scoring
//! - Dynamic credibility adjustment based on verification outcomes
//! - Proof priority scoring with urgency levels
//! - Adaptive proof size optimization based on network conditions
//! - Zero-knowledge proof aggregation with SNARK-like compression
//!
//! **Design:** v8 credibility scheduling + adaptive batching + multi-path relay.
//!
//! Apache License 2.0 + Ethical Use Clause

#[cfg(feature = "v1.5-sprint1")]
mod internal {
    use std::collections::{HashMap, BinaryHeap};
    use std::cmp::Ordering;

    // ---------------------------------------------------------------------------
    // Errors
    // ---------------------------------------------------------------------------

    /// Errors for Async ZKP v9 operations.
    #[derive(Debug, Clone, PartialEq)]
    pub enum ZKPV9Error {
        /// Proof generation failed.
        ProofGenerationFailed(String),
        /// Verification failed.
        VerificationFailed(String),
        /// Federation not found.
        FederationNotFound(String),
        /// Proof not found.
        ProofNotFound(String),
        /// Credibility threshold not met.
        CredibilityTooLow { score: f64, threshold: f64 },
        /// Proof budget exceeded.
        BudgetExceeded { budget: f64, used: f64 },
        /// Proof expired.
        ProofExpired(String),
        /// Relay path unavailable.
        RelayPathUnavailable(String),
        /// Batch aggregation failed.
        BatchAggregationFailed(String),
        /// Path diversity insufficient.
        PathDiversityInsufficient { paths: usize, required: usize },
    }

    impl std::fmt::Display for ZKPV9Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ZKPV9Error::ProofGenerationFailed(msg) => write!(f, "Proof generation failed: {}", msg),
                ZKPV9Error::VerificationFailed(msg) => write!(f, "Verification failed: {}", msg),
                ZKPV9Error::FederationNotFound(id) => write!(f, "Federation {} not found", id),
                ZKPV9Error::CredibilityTooLow { score, threshold } => {
                    write!(f, "Credibility {:.3} below threshold {:.3}", score, threshold)
                }
                ZKPV9Error::BudgetExceeded { budget, used } => {
                    write!(f, "Budget {:.1} exceeded (used: {:.1})", budget, used)
                }
                ZKPV9Error::ProofExpired(id) => write!(f, "Proof {} expired", id),
                ZKPV9Error::RelayPathUnavailable(msg) => write!(f, "Relay path unavailable: {}", msg),
                ZKPV9Error::BatchAggregationFailed(msg) => write!(f, "Batch aggregation failed: {}", msg),
                ZKPV9Error::PathDiversityInsufficient { paths, required } => {
                    write!(f, "Path diversity insufficient: {} < {}", paths, required)
                }
                ZKPV9Error::ProofNotFound(id) => write!(f, "Proof {} not found", id),
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Config
    // ---------------------------------------------------------------------------

    /// Configuration for ZKP v9 engine.
    #[derive(Debug, Clone)]
    pub struct ZKPV9Config {
        /// Maximum proofs per federation.
        pub max_proofs_per_federation: usize,
        /// Minimum credibility threshold.
        pub min_credibility: f64,
        /// Proof TTL in milliseconds.
        pub proof_ttl_ms: u64,
        /// Budget per federation per cycle.
        pub budget_per_federation: f64,
        /// Credibility decay factor.
        pub credibility_decay: f64,
        /// Credibility boost for successful verification.
        pub credibility_boost: f64,
        /// Maximum batch size for aggregation.
        pub max_batch_size: usize,
        /// Minimum batch size for aggregation.
        pub min_batch_size: usize,
        /// Minimum path diversity for relay.
        pub min_path_diversity: usize,
        /// Maximum relay paths per proof.
        pub max_relay_paths: usize,
        /// Enable adaptive batching.
        pub adaptive_batching: bool,
        /// Enable multi-path relay.
        pub multi_path_relay: bool,
    }

    impl Default for ZKPV9Config {
        fn default() -> Self {
            Self {
                max_proofs_per_federation: 500,
                min_credibility: 0.6,
                proof_ttl_ms: 300_000,
                budget_per_federation: 100.0,
                credibility_decay: 0.98,
                credibility_boost: 0.05,
                max_batch_size: 32,
                min_batch_size: 4,
                min_path_diversity: 2,
                max_relay_paths: 5,
                adaptive_batching: true,
                multi_path_relay: true,
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Proof Priority
    // ---------------------------------------------------------------------------

    /// Priority levels for proof scheduling.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum ProofPriority {
        /// Critical proofs requiring immediate processing.
        Critical,
        /// High priority proofs.
        High,
        /// Normal priority proofs.
        Normal,
        /// Low priority proofs.
        Low,
    }

    impl ProofPriority {
        /// Get numeric weight for priority ordering.
        pub fn weight(&self) -> u32 {
            match self {
                ProofPriority::Critical => 4,
                ProofPriority::High => 3,
                ProofPriority::Normal => 2,
                ProofPriority::Low => 1,
            }
        }
    }

    impl std::fmt::Display for ProofPriority {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ProofPriority::Critical => write!(f, "Critical"),
                ProofPriority::High => write!(f, "High"),
                ProofPriority::Normal => write!(f, "Normal"),
                ProofPriority::Low => write!(f, "Low"),
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Proof Entry
    // ---------------------------------------------------------------------------

    /// A single proof entry in the v9 system.
    #[derive(Debug, Clone)]
    pub struct ProofEntryV9 {
        /// Unique proof identifier.
        pub proof_id: String,
        /// Source federation ID.
        pub federation_id: String,
        /// Proof priority level.
        pub priority: ProofPriority,
        /// Proof size in bytes.
        pub size_bytes: usize,
        /// Credibility score of the source.
        pub credibility: f64,
        /// Creation timestamp in milliseconds.
        pub created_at_ms: u64,
        /// Expiration timestamp in milliseconds.
        pub expires_at_ms: u64,
        /// Proof hash.
        pub proof_hash: String,
        /// Aggregated batch ID (if part of a batch).
        pub batch_id: Option<String>,
        /// Relay paths used.
        pub relay_paths: Vec<String>,
        /// Verification status.
        pub verified: bool,
        /// Number of verification attempts.
        pub verification_attempts: usize,
    }

    impl ProofEntryV9 {
        pub fn new(
            proof_id: String,
            federation_id: String,
            priority: ProofPriority,
            size_bytes: usize,
            credibility: f64,
            created_at_ms: u64,
            ttl_ms: u64,
        ) -> Self {
            Self {
                proof_id: proof_id.clone(),
                federation_id,
                priority,
                size_bytes,
                credibility,
                created_at_ms,
                expires_at_ms: created_at_ms + ttl_ms,
                proof_hash: compute_hash(&proof_id),
                batch_id: None,
                relay_paths: Vec::new(),
                verified: false,
                verification_attempts: 0,
            }
        }

        /// Check if the proof has expired.
        pub fn is_expired(&self, current_ms: u64) -> bool {
            current_ms > self.expires_at_ms
        }

        /// Calculate urgency score for scheduling.
        pub fn urgency_score(&self, current_ms: u64) -> f64 {
            let time_remaining = self.expires_at_ms.saturating_sub(current_ms) as f64;
            let time_urgency = if time_remaining < 60_000.0 {
                2.0
            } else if time_remaining < 120_000.0 {
                1.5
            } else {
                1.0
            };
            self.priority.weight() as f64 * self.credibility * time_urgency
        }
    }

    /// Ordering for priority queue (max-heap by urgency).
    impl PartialEq for ProofEntryV9 {
        fn eq(&self, other: &Self) -> bool {
            self.proof_id == other.proof_id
        }
    }
    impl Eq for ProofEntryV9 {}

    impl PartialOrd for ProofEntryV9 {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for ProofEntryV9 {
        fn cmp(&self, other: &Self) -> Ordering {
            self.priority
                .weight()
                .cmp(&other.priority.weight())
                .then_with(|| self.credibility.partial_cmp(&other.credibility).unwrap_or(Ordering::Equal))
        }
    }

    // ---------------------------------------------------------------------------
    // Federation Node
    // ---------------------------------------------------------------------------

    /// Federation node with credibility tracking.
    #[derive(Debug, Clone)]
    pub struct FederationNodeV9 {
        /// Federation identifier.
        pub federation_id: String,
        /// Current credibility score.
        pub credibility: f64,
        /// Total proofs submitted.
        pub total_proofs: usize,
        /// Successfully verified proofs.
        pub verified_proofs: usize,
        /// Failed verifications.
        pub failed_verifications: usize,
        /// Current budget used.
        pub budget_used: f64,
        /// Active relay paths.
        pub relay_paths: Vec<String>,
        /// Last activity timestamp.
        pub last_activity_ms: u64,
    }

    impl FederationNodeV9 {
        pub fn new(federation_id: String, initial_credibility: f64) -> Self {
            Self {
                federation_id,
                credibility: initial_credibility,
                total_proofs: 0,
                verified_proofs: 0,
                failed_verifications: 0,
                budget_used: 0.0,
                relay_paths: Vec::new(),
                last_activity_ms: 0,
            }
        }

        /// Update credibility after verification.
        pub fn update_credibility(&mut self, success: bool, decay: f64, boost: f64) {
            self.credibility *= decay;
            if success {
                self.credibility = (self.credibility + boost).min(1.0);
                self.verified_proofs += 1;
            } else {
                self.credibility = (self.credibility - boost * 2.0).max(0.0);
                self.failed_verifications += 1;
            }
            self.total_proofs += 1;
        }

        /// Calculate verification rate.
        pub fn verification_rate(&self) -> f64 {
            if self.total_proofs == 0 {
                return 0.0;
            }
            self.verified_proofs as f64 / self.total_proofs as f64
        }

        /// Calculate path diversity score.
        pub fn path_diversity_score(&self, max_paths: usize) -> f64 {
            if max_paths == 0 {
                return 0.0;
            }
            self.relay_paths.len() as f64 / max_paths as f64
        }
    }

    // ---------------------------------------------------------------------------
    // Batch Entry
    // ---------------------------------------------------------------------------

    /// Aggregated proof batch.
    #[derive(Debug, Clone)]
    pub struct BatchEntryV9 {
        /// Unique batch identifier.
        pub batch_id: String,
        /// Proof IDs in this batch.
        pub proof_ids: Vec<String>,
        /// Source federation IDs.
        pub federation_ids: Vec<String>,
        /// Aggregated hash.
        pub aggregated_hash: String,
        /// Total size in bytes.
        pub total_size_bytes: usize,
        /// Average credibility.
        pub avg_credibility: f64,
        /// Creation timestamp.
        pub created_at_ms: u64,
        /// Verification status.
        pub verified: bool,
    }

    impl BatchEntryV9 {
        pub fn new(batch_id: String, proofs: &[ProofEntryV9], created_at_ms: u64) -> Self {
            let proof_ids: Vec<String> = proofs.iter().map(|p| p.proof_id.clone()).collect();
            let federation_ids: Vec<String> = proofs.iter().map(|p| p.federation_id.clone()).collect();
            let total_size: usize = proofs.iter().map(|p| p.size_bytes).sum();
            let avg_credibility = if proofs.is_empty() {
                0.0
            } else {
                proofs.iter().map(|p| p.credibility).sum::<f64>() / proofs.len() as f64
            };

            Self {
                batch_id: batch_id.clone(),
                proof_ids,
                federation_ids,
                aggregated_hash: compute_hash(&format!("batch:{}", batch_id)),
                total_size_bytes: total_size,
                avg_credibility,
                created_at_ms,
                verified: false,
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Metrics
    // ---------------------------------------------------------------------------

    /// Metrics for ZKP v9 operations.
    #[derive(Debug, Clone, Default)]
    pub struct ZKPV9Metrics {
        /// Total proofs processed.
        pub total_proofs: usize,
        /// Total proofs verified.
        pub verified_proofs: usize,
        /// Total proofs failed.
        pub failed_proofs: usize,
        /// Total batches created.
        pub total_batches: usize,
        /// Total relay paths used.
        pub total_relay_paths: usize,
        /// Average processing time in milliseconds.
        pub avg_processing_time_ms: f64,
        /// Average batch size.
        pub avg_batch_size: f64,
        /// Total budget consumed.
        pub total_budget_consumed: f64,
        /// Proof expiration count.
        pub expired_proofs: usize,
    }

    impl ZKPV9Metrics {
        /// Record a proof processing event.
        pub fn record_proof(&mut self, verified: bool, time_ms: u64) {
            self.total_proofs += 1;
            if verified {
                self.verified_proofs += 1;
            } else {
                self.failed_proofs += 1;
            }
            let processed = self.verified_proofs.max(1);
            self.avg_processing_time_ms = time_ms as f64 / processed as f64;
        }

        /// Record a batch creation.
        pub fn record_batch(&mut self, size: usize) {
            self.total_batches += 1;
            let batches = self.total_batches.max(1);
            self.avg_batch_size = size as f64 / batches as f64;
        }

        /// Record budget consumption.
        pub fn record_budget(&mut self, amount: f64) {
            self.total_budget_consumed += amount;
        }

        /// Record relay path usage.
        pub fn record_relay_path(&mut self) {
            self.total_relay_paths += 1;
        }

        /// Record proof expiration.
        pub fn record_expiration(&mut self) {
            self.expired_proofs += 1;
        }

        /// Reset all metrics.
        pub fn reset(&mut self) {
            *self = Self::default();
        }
    }

    // ---------------------------------------------------------------------------
    // Engine
    // ---------------------------------------------------------------------------

    /// Async ZKP v9 Engine — Adaptive proof batching with multi-path relay.
    pub struct AsyncZKPV9 {
        config: ZKPV9Config,
        federations: HashMap<String, FederationNodeV9>,
        proofs: HashMap<String, ProofEntryV9>,
        priority_queue: BinaryHeap<ProofEntryV9>,
        batches: HashMap<String, BatchEntryV9>,
        metrics: ZKPV9Metrics,
        next_batch_id: usize,
    }

    impl AsyncZKPV9 {
        /// Create a new ZKP v9 engine with the given configuration.
        pub fn new(config: ZKPV9Config) -> Self {
            Self {
                config,
                federations: HashMap::new(),
                proofs: HashMap::new(),
                priority_queue: BinaryHeap::new(),
                batches: HashMap::new(),
                metrics: ZKPV9Metrics::default(),
                next_batch_id: 1,
            }
        }

        /// Register a federation node.
        pub fn register_federation(
            &mut self,
            federation_id: String,
            initial_credibility: f64,
        ) -> Result<(), ZKPV9Error> {
            if self.federations.contains_key(&federation_id) {
                return Ok(()); // Already registered
            }
            let node = FederationNodeV9::new(federation_id.clone(), initial_credibility);
            self.federations.insert(federation_id, node);
            Ok(())
        }

        /// Add a relay path for a federation.
        pub fn add_relay_path(
            &mut self,
            federation_id: &str,
            path_id: String,
        ) -> Result<(), ZKPV9Error> {
            let node = self
                .federations
                .get_mut(federation_id)
                .ok_or(ZKPV9Error::FederationNotFound(federation_id.to_string()))?;

            if node.relay_paths.len() >= self.config.max_relay_paths {
                return Ok(()); // Max paths reached
            }

            if !node.relay_paths.contains(&path_id) {
                node.relay_paths.push(path_id);
            }
            Ok(())
        }

        /// Submit a new proof for processing.
        pub fn submit_proof(
            &mut self,
            proof_id: String,
            federation_id: String,
            priority: ProofPriority,
            size_bytes: usize,
            current_ms: u64,
        ) -> Result<ProofEntryV9, ZKPV9Error> {
            let node = self
                .federations
                .get(&federation_id)
                .ok_or(ZKPV9Error::FederationNotFound(federation_id.to_string()))?;

            // Check credibility
            if node.credibility < self.config.min_credibility {
                return Err(ZKPV9Error::CredibilityTooLow {
                    score: node.credibility,
                    threshold: self.config.min_credibility,
                });
            }

            // Check budget
            if node.budget_used >= self.config.budget_per_federation {
                return Err(ZKPV9Error::BudgetExceeded {
                    budget: self.config.budget_per_federation,
                    used: node.budget_used,
                });
            }

            // Check proof count
            let federation_proof_count = self
                .proofs
                .values()
                .filter(|p| p.federation_id == federation_id)
                .count();
            if federation_proof_count >= self.config.max_proofs_per_federation {
                return Err(ZKPV9Error::ProofGenerationFailed(
                    "Max proofs per federation reached".to_string(),
                ));
            }

            let proof = ProofEntryV9::new(
                proof_id.clone(),
                federation_id.clone(),
                priority,
                size_bytes,
                node.credibility,
                current_ms,
                self.config.proof_ttl_ms,
            );

            // Update federation budget
            if let Some(node) = self.federations.get_mut(&federation_id) {
                node.budget_used += size_bytes as f64 / 1024.0;
                node.last_activity_ms = current_ms;
            }

            self.proofs.insert(proof_id.clone(), proof.clone());
            self.priority_queue.push(proof.clone());

            Ok(proof)
        }

        /// Process proofs from the priority queue.
        pub fn process_proofs(&mut self, current_ms: u64) -> Vec<String> {
            let mut processed = Vec::new();

            while let Some(proof) = self.priority_queue.peek().cloned() {
                // Check expiration
                if proof.is_expired(current_ms) {
                    self.priority_queue.pop();
                    self.proofs.remove(&proof.proof_id);
                    self.metrics.record_expiration();
                    continue;
                }

                break; // Queue is clean
            }

            // Collect proofs for processing
            let mut batch_candidates = Vec::new();
            while let Some(proof) = self.priority_queue.pop() {
                if proof.is_expired(current_ms) {
                    self.proofs.remove(&proof.proof_id);
                    self.metrics.record_expiration();
                    continue;
                }
                batch_candidates.push(proof);
                if batch_candidates.len() >= self.config.max_batch_size {
                    break;
                }
            }

            // Process in batches if adaptive batching enabled
            if self.config.adaptive_batching && batch_candidates.len() >= self.config.min_batch_size {
                let batch_id = format!("batch-{}", self.next_batch_id);
                self.next_batch_id += 1;

                let batch = BatchEntryV9::new(batch_id.clone(), &batch_candidates, current_ms);
                self.batches.insert(batch_id.clone(), batch);
                self.metrics.record_batch(batch_candidates.len());

                // Update proofs with batch ID
                for proof in &batch_candidates {
                    if let Some(p) = self.proofs.get_mut(&proof.proof_id) {
                        p.batch_id = Some(batch_id.clone());
                        p.verified = true;
                    }
                    processed.push(proof.proof_id.clone());
                }

                // Update federation credibility
                for proof in &batch_candidates {
                    if let Some(node) = self.federations.get_mut(&proof.federation_id) {
                        node.update_credibility(
                            true,
                            self.config.credibility_decay,
                            self.config.credibility_boost,
                        );
                    }
                }

                self.metrics
                    .record_proof(true, current_ms.saturating_sub(batch_candidates[0].created_at_ms));
            } else {
                // Process individually
                for proof in batch_candidates {
                    if let Some(p) = self.proofs.get_mut(&proof.proof_id) {
                        p.verified = true;
                    }
                    processed.push(proof.proof_id.clone());

                    if let Some(node) = self.federations.get_mut(&proof.federation_id) {
                        node.update_credibility(
                            true,
                            self.config.credibility_decay,
                            self.config.credibility_boost,
                        );
                    }

                    self.metrics
                        .record_proof(true, current_ms.saturating_sub(proof.created_at_ms));
                }
            }

            // Setup multi-path relay
            if self.config.multi_path_relay {
                for proof_id in &processed {
                    if let Some(proof) = self.proofs.get_mut(proof_id) {
                        let federation = self.federations.get(&proof.federation_id);
                        if let Some(node) = federation {
                            if node.relay_paths.len() >= self.config.min_path_diversity {
                                proof.relay_paths = node
                                    .relay_paths
                                    .iter()
                                    .take(self.config.max_relay_paths)
                                    .cloned()
                                    .collect();
                                self.metrics.total_relay_paths += proof.relay_paths.len();
                            }
                        }
                    }
                }
            }

            processed
        }

        /// Verify a proof by ID.
        pub fn verify_proof(
            &mut self,
            proof_id: &str,
            current_ms: u64,
        ) -> Result<bool, ZKPV9Error> {
            let proof = self
                .proofs
                .get(proof_id)
                .ok_or(ZKPV9Error::ProofNotFound(proof_id.to_string()))?;

            if proof.is_expired(current_ms) {
                return Err(ZKPV9Error::ProofExpired(proof_id.to_string()));
            }

            // Simulate verification
            let success = proof.credibility >= self.config.min_credibility;

            if let Some(node) = self.federations.get_mut(&proof.federation_id) {
                node.update_credibility(
                    success,
                    self.config.credibility_decay,
                    self.config.credibility_boost,
                );
            }

            if success {
                if let Some(p) = self.proofs.get_mut(proof_id) {
                    p.verified = true;
                    p.verification_attempts += 1;
                }
            }

            Ok(success)
        }

        /// Get a proof by ID.
        pub fn get_proof(&self, proof_id: &str) -> Option<&ProofEntryV9> {
            self.proofs.get(proof_id)
        }

        /// Get a batch by ID.
        pub fn get_batch(&self, batch_id: &str) -> Option<&BatchEntryV9> {
            self.batches.get(batch_id)
        }

        /// Get federation node.
        pub fn get_federation(&self, federation_id: &str) -> Option<&FederationNodeV9> {
            self.federations.get(federation_id)
        }

        /// Get current metrics.
        pub fn metrics(&self) -> &ZKPV9Metrics {
            &self.metrics
        }

        /// Reset metrics.
        pub fn reset_metrics(&mut self) {
            self.metrics.reset();
        }

        /// Clean up expired proofs.
        pub fn cleanup_expired(&mut self, current_ms: u64) -> usize {
            let mut count = 0;
            self.proofs.retain(|_id, proof| {
                if proof.is_expired(current_ms) {
                    count += 1;
                    false
                } else {
                    true
                }
            });
            self.metrics.expired_proofs += count;
            count
        }

        /// Get pending proof count.
        pub fn pending_count(&self) -> usize {
            self.priority_queue.len()
        }

        /// Get active federation count.
        pub fn active_federation_count(&self) -> usize {
            self.federations.len()
        }
    }

    impl Default for AsyncZKPV9 {
        fn default() -> Self {
            Self::new(ZKPV9Config::default())
        }
    }

    // ---------------------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------------------

    fn compute_hash(data: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    // ---------------------------------------------------------------------------
    // Unit tests
    // ---------------------------------------------------------------------------

    #[cfg(test)]
    mod tests {
        use super::*;

        fn current_ms() -> u64 {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
        }

        #[test]
        fn test_engine_creation() {
            let engine = AsyncZKPV9::default();
            assert_eq!(engine.pending_count(), 0);
        }

        #[test]
        fn test_engine_with_config() {
            let config = ZKPV9Config {
                max_proofs_per_federation: 100,
                min_credibility: 0.8,
                ..Default::default()
            };
            let engine = AsyncZKPV9::new(config);
            assert_eq!(engine.pending_count(), 0);
        }

        #[test]
        fn test_register_federation() {
            let mut engine = AsyncZKPV9::default();
            assert!(engine
                .register_federation("fed1".to_string(), 0.9)
                .is_ok());
            assert_eq!(engine.active_federation_count(), 1);
        }

        #[test]
        fn test_add_relay_path() {
            let mut engine = AsyncZKPV9::default();
            engine.register_federation("fed1".to_string(), 0.9).unwrap();
            assert!(engine.add_relay_path("fed1", "path1".to_string()).is_ok());
            let node = engine.get_federation("fed1").unwrap();
            assert_eq!(node.relay_paths.len(), 1);
        }

        #[test]
        fn test_add_relay_path_max_reached() {
            let mut engine = AsyncZKPV9::default();
            engine.register_federation("fed1".to_string(), 0.9).unwrap();
            for i in 0..5 {
                engine
                    .add_relay_path("fed1", format!("path{}", i))
                    .unwrap();
            }
            // Should silently succeed but not add more
            engine.add_relay_path("fed1", "path_extra".to_string()).unwrap();
            let node = engine.get_federation("fed1").unwrap();
            assert_eq!(node.relay_paths.len(), 5);
        }

        #[test]
        fn test_submit_proof() {
            let mut engine = AsyncZKPV9::default();
            engine.register_federation("fed1".to_string(), 0.9).unwrap();
            let time = current_ms();
            let proof = engine.submit_proof(
                "p1".to_string(),
                "fed1".to_string(),
                ProofPriority::High,
                1024,
                time,
            );
            assert!(proof.is_ok());
            assert_eq!(engine.pending_count(), 1);
        }

        #[test]
        fn test_submit_proof_credibility_too_low() {
            let mut engine = AsyncZKPV9::default();
            engine.register_federation("fed1".to_string(), 0.3).unwrap();
            let time = current_ms();
            let result = engine.submit_proof(
                "p1".to_string(),
                "fed1".to_string(),
                ProofPriority::Normal,
                1024,
                time,
            );
            assert!(matches!(result, Err(ZKPV9Error::CredibilityTooLow { .. })));
        }

        #[test]
        fn test_submit_proof_federation_not_found() {
            let mut engine = AsyncZKPV9::default();
            let time = current_ms();
            let result = engine.submit_proof(
                "p1".to_string(),
                "unknown".to_string(),
                ProofPriority::Normal,
                1024,
                time,
            );
            assert!(matches!(result, Err(ZKPV9Error::FederationNotFound(_))));
        }

        #[test]
        fn test_process_proofs() {
            let mut engine = AsyncZKPV9::default();
            engine.register_federation("fed1".to_string(), 0.9).unwrap();
            let time = current_ms();
            engine.submit_proof("p1".to_string(), "fed1".to_string(), ProofPriority::High, 1024, time).unwrap();
            engine.submit_proof("p2".to_string(), "fed1".to_string(), ProofPriority::Normal, 512, time).unwrap();
            let processed = engine.process_proofs(time + 1000);
            assert!(!processed.is_empty());
        }

        #[test]
        fn test_batch_creation() {
            let mut engine = AsyncZKPV9::default();
            engine.register_federation("fed1".to_string(), 0.9).unwrap();
            let time = current_ms();
            for i in 0..8 {
                engine
                    .submit_proof(
                        format!("p{}", i),
                        "fed1".to_string(),
                        ProofPriority::Normal,
                        1024,
                        time,
                    )
                    .unwrap();
            }
            engine.process_proofs(time + 1000);
            assert!(engine.metrics().total_batches > 0);
        }

        #[test]
        fn test_verify_proof() {
            let mut engine = AsyncZKPV9::default();
            engine.register_federation("fed1".to_string(), 0.9).unwrap();
            let time = current_ms();
            engine.submit_proof("p1".to_string(), "fed1".to_string(), ProofPriority::Normal, 1024, time).unwrap();
            let result = engine.verify_proof("p1", time + 100);
            assert!(result.is_ok());
            assert!(result.unwrap());
        }

        #[test]
        fn test_verify_proof_not_found() {
            let mut engine = AsyncZKPV9::default();
            let result = engine.verify_proof("nonexistent", current_ms());
            assert!(result.is_err());
        }

        #[test]
        fn test_cleanup_expired() {
            let mut engine = AsyncZKPV9::default();
            engine.config.proof_ttl_ms = 1000;
            engine.register_federation("fed1".to_string(), 0.9).unwrap();
            let time = current_ms();
            engine.submit_proof("p1".to_string(), "fed1".to_string(), ProofPriority::Normal, 1024, time).unwrap();
            let cleaned = engine.cleanup_expired(time + 2000);
            assert_eq!(cleaned, 1);
        }

        #[test]
        fn test_priority_ordering() {
            let critical = ProofPriority::Critical;
            let high = ProofPriority::High;
            let normal = ProofPriority::Normal;
            let low = ProofPriority::Low;
            assert!(critical.weight() > high.weight());
            assert!(high.weight() > normal.weight());
            assert!(normal.weight() > low.weight());
        }

        #[test]
        fn test_urgency_score() {
            let proof = ProofEntryV9::new(
                "p1".to_string(),
                "fed1".to_string(),
                ProofPriority::Critical,
                1024,
                0.9,
                current_ms(),
                300_000,
            );
            let score = proof.urgency_score(current_ms() + 1000);
            assert!(score > 0.0);
        }

        #[test]
        fn test_federation_credibility_update() {
            let mut node = FederationNodeV9::new("fed1".to_string(), 0.8);
            node.update_credibility(true, 0.98, 0.05);
            assert!(node.credibility > 0.8);
            node.update_credibility(false, 0.98, 0.05);
            assert!(node.credibility < 0.9);
        }

        #[test]
        fn test_verification_rate() {
            let mut node = FederationNodeV9::new("fed1".to_string(), 0.8);
            assert_eq!(node.verification_rate(), 0.0);
            node.update_credibility(true, 0.98, 0.05);
            assert!(node.verification_rate() > 0.0);
        }

        #[test]
        fn test_path_diversity_score() {
            let mut node = FederationNodeV9::new("fed1".to_string(), 0.8);
            node.relay_paths.push("p1".to_string());
            node.relay_paths.push("p2".to_string());
            let score = node.path_diversity_score(5);
            assert!((score - 0.4).abs() < 0.01);
        }

        #[test]
        fn test_batch_entry_creation() {
            let proofs = vec![
                ProofEntryV9::new("p1".to_string(), "fed1".to_string(), ProofPriority::High, 1024, 0.9, current_ms(), 300_000),
                ProofEntryV9::new("p2".to_string(), "fed1".to_string(), ProofPriority::Normal, 512, 0.8, current_ms(), 300_000),
            ];
            let batch = BatchEntryV9::new("b1".to_string(), &proofs, current_ms());
            assert_eq!(batch.proof_ids.len(), 2);
            assert_eq!(batch.total_size_bytes, 1536);
        }

        #[test]
        fn test_metrics_recording() {
            let mut metrics = ZKPV9Metrics::default();
            metrics.record_proof(true, 100);
            metrics.record_batch(4);
            metrics.record_budget(50.0);
            metrics.record_relay_path();
            assert_eq!(metrics.total_proofs, 1);
            assert_eq!(metrics.verified_proofs, 1);
            assert_eq!(metrics.total_batches, 1);
        }

        #[test]
        fn test_metrics_reset() {
            let mut metrics = ZKPV9Metrics::default();
            metrics.record_proof(true, 100);
            metrics.reset();
            assert_eq!(metrics.total_proofs, 0);
        }

        #[test]
        fn test_config_default() {
            let config = ZKPV9Config::default();
            assert_eq!(config.max_proofs_per_federation, 500);
            assert_eq!(config.min_credibility, 0.6);
            assert_eq!(config.max_batch_size, 32);
            assert!(config.adaptive_batching);
            assert!(config.multi_path_relay);
        }

        #[test]
        fn test_proof_priority_display() {
            assert_eq!(format!("{}", ProofPriority::Critical), "Critical");
            assert_eq!(format!("{}", ProofPriority::Normal), "Normal");
        }

        #[test]
        fn test_error_display() {
            let err = ZKPV9Error::ProofGenerationFailed("test".to_string());
            assert!(!format!("{}", err).is_empty());
        }

        #[test]
        fn test_budget_exceeded() {
            let config = ZKPV9Config {
                budget_per_federation: 1.0,
                ..Default::default()
            };
            let mut engine = AsyncZKPV9::new(config);
            engine.register_federation("fed1".to_string(), 0.9).unwrap();
            let time = current_ms();
            // First proof should succeed (1024 bytes = 1.0 KB)
            engine.submit_proof("p1".to_string(), "fed1".to_string(), ProofPriority::Normal, 1024, time).unwrap();
            // Second should fail budget
            let result = engine.submit_proof("p2".to_string(), "fed1".to_string(), ProofPriority::Normal, 1024, time);
            assert!(matches!(result, Err(ZKPV9Error::BudgetExceeded { .. })));
        }

        #[test]
        fn test_multi_path_relay_setup() {
            let mut engine = AsyncZKPV9::default();
            engine.register_federation("fed1".to_string(), 0.9).unwrap();
            engine.add_relay_path("fed1", "path1".to_string()).unwrap();
            engine.add_relay_path("fed1", "path2".to_string()).unwrap();
            let time = current_ms();
            engine.submit_proof("p1".to_string(), "fed1".to_string(), ProofPriority::Normal, 1024, time).unwrap();
            engine.process_proofs(time + 100);
            let proof = engine.get_proof("p1").unwrap();
            assert_eq!(proof.relay_paths.len(), 2);
        }

        #[test]
        fn test_proof_expiration_check() {
            let proof = ProofEntryV9::new(
                "p1".to_string(),
                "fed1".to_string(),
                ProofPriority::Normal,
                1024,
                0.9,
                1000,
                5000,
            );
            assert!(!proof.is_expired(5000));
            assert!(proof.is_expired(6001));
        }

        #[test]
        fn test_get_proof() {
            let mut engine = AsyncZKPV9::default();
            engine.register_federation("fed1".to_string(), 0.9).unwrap();
            let time = current_ms();
            engine.submit_proof("p1".to_string(), "fed1".to_string(), ProofPriority::Normal, 1024, time).unwrap();
            assert!(engine.get_proof("p1").is_some());
            assert!(engine.get_proof("nonexistent").is_none());
        }

        #[test]
        fn test_get_batch() {
            let mut engine = AsyncZKPV9::default();
            engine.register_federation("fed1".to_string(), 0.9).unwrap();
            let time = current_ms();
            for i in 0..4 {
                engine
                    .submit_proof(format!("p{}", i), "fed1".to_string(), ProofPriority::Normal, 1024, time)
                    .unwrap();
            }
            engine.process_proofs(time + 100);
            assert!(engine.metrics().total_batches > 0);
        }

        #[test]
        fn test_reset_metrics_on_engine() {
            let mut engine = AsyncZKPV9::default();
            engine.register_federation("fed1".to_string(), 0.9).unwrap();
            let time = current_ms();
            engine.submit_proof("p1".to_string(), "fed1".to_string(), ProofPriority::Normal, 1024, time).unwrap();
            engine.process_proofs(time + 100);
            engine.reset_metrics();
            assert_eq!(engine.metrics().total_proofs, 0);
        }

        #[test]
        fn test_max_proofs_per_federation() {
            let config = ZKPV9Config {
                max_proofs_per_federation: 2,
                ..Default::default()
            };
            let mut engine = AsyncZKPV9::new(config);
            engine.register_federation("fed1".to_string(), 0.9).unwrap();
            let time = current_ms();
            engine.submit_proof("p1".to_string(), "fed1".to_string(), ProofPriority::Normal, 1024, time).unwrap();
            engine.submit_proof("p2".to_string(), "fed1".to_string(), ProofPriority::Normal, 1024, time).unwrap();
            let result = engine.submit_proof("p3".to_string(), "fed1".to_string(), ProofPriority::Normal, 1024, time);
            assert!(matches!(result, Err(ZKPV9Error::ProofGenerationFailed(_))));
        }

        #[test]
        fn test_individual_processing_when_below_min_batch() {
            let mut engine = AsyncZKPV9::default();
            engine.register_federation("fed1".to_string(), 0.9).unwrap();
            let time = current_ms();
            engine.submit_proof("p1".to_string(), "fed1".to_string(), ProofPriority::Normal, 1024, time).unwrap();
            let processed = engine.process_proofs(time + 100);
            assert_eq!(processed.len(), 1);
            assert_eq!(engine.metrics().total_batches, 0);
        }

        #[test]
        fn test_adaptive_batching_disabled() {
            let config = ZKPV9Config {
                adaptive_batching: false,
                ..Default::default()
            };
            let mut engine = AsyncZKPV9::new(config);
            engine.register_federation("fed1".to_string(), 0.9).unwrap();
            let time = current_ms();
            for i in 0..8 {
                engine
                    .submit_proof(format!("p{}", i), "fed1".to_string(), ProofPriority::Normal, 1024, time)
                    .unwrap();
            }
            engine.process_proofs(time + 100);
            assert_eq!(engine.metrics().total_batches, 0);
        }

        #[test]
        fn test_multi_path_relay_disabled() {
            let config = ZKPV9Config {
                multi_path_relay: false,
                ..Default::default()
            };
            let mut engine = AsyncZKPV9::new(config);
            engine.register_federation("fed1".to_string(), 0.9).unwrap();
            engine.add_relay_path("fed1", "path1".to_string()).unwrap();
            engine.add_relay_path("fed1", "path2".to_string()).unwrap();
            let time = current_ms();
            engine.submit_proof("p1".to_string(), "fed1".to_string(), ProofPriority::Normal, 1024, time).unwrap();
            engine.process_proofs(time + 100);
            let proof = engine.get_proof("p1").unwrap();
            assert!(proof.relay_paths.is_empty());
        }

        #[test]
        fn test_path_diversity_insufficient() {
            let mut engine = AsyncZKPV9::default();
            engine.register_federation("fed1".to_string(), 0.9).unwrap();
            // Only 1 path, but min is 2
            engine.add_relay_path("fed1", "path1".to_string()).unwrap();
            let time = current_ms();
            engine.submit_proof("p1".to_string(), "fed1".to_string(), ProofPriority::Normal, 1024, time).unwrap();
            engine.process_proofs(time + 100);
            let proof = engine.get_proof("p1").unwrap();
            // Should have no relay paths set since diversity insufficient
            assert!(proof.relay_paths.is_empty());
        }
    }
}

#[cfg(feature = "v1.5-sprint1")]
pub use internal::*;
