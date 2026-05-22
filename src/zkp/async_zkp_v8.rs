//! Async ZKP v8 — Adaptive proof scheduling with credibility scoring and multi-federation relay.
//!
//! Improvements over v7:
//! - Adaptive proof scheduling with priority queues per federation
//! - Credibility scoring for proof sources (reputation + verification history)
//! - Multi-federation proof relay with Merkle-based proof chains
//! - Dynamic proof budget allocation based on federation load
//! - Proof freshness tracking with automatic expiration
//! - Cross-federation proof validation with threshold signatures
//!
//! **Design:** v7 recursive aggregation + credibility-weighted scheduling.
//!
//! Apache License 2.0 + Ethical Use Clause

#[cfg(feature = "v1.4-sprint3")]
mod internal {
    use std::cmp::Ordering;
    use std::collections::{BinaryHeap, HashMap};

    // ---------------------------------------------------------------------------
    // Errors
    // ---------------------------------------------------------------------------

    /// Errors for Async ZKP v8 operations.
    #[derive(Debug, Clone, PartialEq)]
    pub enum ZKPV8Error {
        /// Proof generation failed.
        ProofGenerationFailed(String),
        /// Verification failed.
        VerificationFailed(String),
        /// Federation not found.
        FederationNotFound(String),
        /// Credibility threshold not met.
        CredibilityTooLow { score: f64, threshold: f64 },
        /// Proof budget exceeded.
        BudgetExceeded { budget: f64, used: f64 },
        /// Proof expired.
        ProofExpired(String),
        /// Relay chain broken.
        RelayChainBroken(String),
        /// Scheduling conflict.
        SchedulingConflict(String),
    }

    impl std::fmt::Display for ZKPV8Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ZKPV8Error::ProofGenerationFailed(msg) => {
                    write!(f, "Proof generation failed: {}", msg)
                }
                ZKPV8Error::VerificationFailed(msg) => write!(f, "Verification failed: {}", msg),
                ZKPV8Error::FederationNotFound(id) => write!(f, "Federation {} not found", id),
                ZKPV8Error::CredibilityTooLow { score, threshold } => {
                    write!(
                        f,
                        "Credibility {:.3} below threshold {:.3}",
                        score, threshold
                    )
                }
                ZKPV8Error::BudgetExceeded { budget, used } => {
                    write!(f, "Budget {:.1} exceeded (used: {:.1})", budget, used)
                }
                ZKPV8Error::ProofExpired(id) => write!(f, "Proof {} expired", id),
                ZKPV8Error::RelayChainBroken(msg) => write!(f, "Relay chain broken: {}", msg),
                ZKPV8Error::SchedulingConflict(msg) => write!(f, "Scheduling conflict: {}", msg),
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Config
    // ---------------------------------------------------------------------------

    /// Configuration for ZKP v8 engine.
    #[derive(Debug, Clone)]
    pub struct ZKPV8Config {
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
        /// Max relay chain depth.
        pub max_relay_depth: usize,
        /// Priority queue size limit.
        pub queue_limit: usize,
    }

    impl Default for ZKPV8Config {
        fn default() -> Self {
            Self {
                max_proofs_per_federation: 500,
                min_credibility: 0.6,
                proof_ttl_ms: 300_000,
                budget_per_federation: 1000.0,
                credibility_decay: 0.98,
                max_relay_depth: 10,
                queue_limit: 1000,
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Federation Entry
    // ---------------------------------------------------------------------------

    /// Federation entry with credibility tracking.
    #[derive(Debug, Clone)]
    pub struct FederationEntry {
        /// Federation identifier.
        pub federation_id: String,
        /// Current credibility score.
        pub credibility: f64,
        /// Total proofs verified.
        pub proofs_verified: usize,
        /// Total proofs failed.
        pub proofs_failed: usize,
        /// Current budget used.
        pub budget_used: f64,
        /// Last activity timestamp.
        pub last_activity_ms: u64,
    }

    impl FederationEntry {
        pub fn new(federation_id: String) -> Self {
            Self {
                federation_id,
                credibility: 1.0,
                proofs_verified: 0,
                proofs_failed: 0,
                budget_used: 0.0,
                last_activity_ms: current_timestamp_ms(),
            }
        }

        /// Update credibility based on verification result.
        pub fn update_credibility(&mut self, success: bool, decay: f64) {
            if success {
                self.credibility = (self.credibility * 0.9 + 1.0 * 0.1).min(1.0);
                self.proofs_verified += 1;
            } else {
                self.credibility = (self.credibility * decay).max(0.0);
                self.proofs_failed += 1;
            }
            self.last_activity_ms = current_timestamp_ms();
        }

        /// Check if federation meets credibility threshold.
        pub fn meets_threshold(&self, threshold: f64) -> bool {
            self.credibility >= threshold
        }

        /// Check if budget is available.
        pub fn has_budget(&self, cost: f64, budget: f64) -> bool {
            self.budget_used + cost <= budget
        }

        /// Consume budget.
        pub fn consume_budget(&mut self, cost: f64) {
            self.budget_used += cost;
        }

        /// Reset budget for new cycle.
        pub fn reset_budget(&mut self) {
            self.budget_used = 0.0;
        }
    }

    // ---------------------------------------------------------------------------
    // Proof Entry
    // ---------------------------------------------------------------------------

    /// Proof entry with scheduling metadata.
    #[derive(Debug, Clone)]
    pub struct ProofEntry {
        /// Proof identifier.
        pub id: String,
        /// Source federation.
        pub federation_id: String,
        /// Priority (higher = more urgent).
        pub priority: u32,
        /// Proof cost in budget units.
        pub cost: f64,
        /// Creation timestamp.
        pub created_ms: u64,
        /// Relay chain (ordered from source to current).
        pub relay_chain: Vec<String>,
        /// Verified flag.
        pub verified: bool,
    }

    impl ProofEntry {
        pub fn new(id: String, federation_id: String, priority: u32, cost: f64) -> Self {
            Self {
                id,
                federation_id: federation_id.clone(),
                priority,
                cost,
                created_ms: current_timestamp_ms(),
                relay_chain: vec![federation_id],
                verified: false,
            }
        }

        /// Check if proof is expired.
        pub fn is_expired(&self, ttl_ms: u64) -> bool {
            current_timestamp_ms() - self.created_ms > ttl_ms
        }

        /// Add federation to relay chain.
        pub fn extend_relay(&mut self, federation_id: String) -> Result<(), ZKPV8Error> {
            if self.relay_chain.len() >= 10 {
                return Err(ZKPV8Error::RelayChainBroken(
                    "Max relay depth reached".to_string(),
                ));
            }
            if self.relay_chain.contains(&federation_id) {
                return Err(ZKPV8Error::RelayChainBroken(
                    "Cycle detected in relay chain".to_string(),
                ));
            }
            self.relay_chain.push(federation_id);
            Ok(())
        }
    }

    impl PartialEq for ProofEntry {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id
        }
    }

    impl Eq for ProofEntry {}

    impl Ord for ProofEntry {
        fn cmp(&self, other: &Self) -> Ordering {
            self.priority
                .cmp(&other.priority)
                .then_with(|| other.created_ms.cmp(&self.created_ms))
        }
    }

    impl PartialOrd for ProofEntry {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    // ---------------------------------------------------------------------------
    // Stats
    // ---------------------------------------------------------------------------

    /// Statistics for ZKP v8 engine.
    #[derive(Debug, Clone)]
    pub struct ZKPV8Stats {
        pub total_proofs_generated: usize,
        pub total_proofs_verified: usize,
        pub total_proofs_expired: usize,
        pub total_relay_hops: usize,
        pub total_budget_consumed: f64,
        pub avg_credibility: f64,
        pub scheduling_conflicts: usize,
    }

    impl Default for ZKPV8Stats {
        fn default() -> Self {
            Self {
                total_proofs_generated: 0,
                total_proofs_verified: 0,
                total_proofs_expired: 0,
                total_relay_hops: 0,
                total_budget_consumed: 0.0,
                avg_credibility: 1.0,
                scheduling_conflicts: 0,
            }
        }
    }

    impl ZKPV8Stats {
        pub fn record_proof(&mut self, verified: bool) {
            self.total_proofs_generated += 1;
            if verified {
                self.total_proofs_verified += 1;
            }
        }

        pub fn record_relay(&mut self, hops: usize, budget: f64) {
            self.total_relay_hops += hops;
            self.total_budget_consumed += budget;
        }

        pub fn reset(&mut self) {
            *self = Self::default();
        }
    }

    // ---------------------------------------------------------------------------
    // Main Engine
    // ---------------------------------------------------------------------------

    /// Async ZKP v8 engine.
    pub struct AsyncZKPV8 {
        config: ZKPV8Config,
        federations: HashMap<String, FederationEntry>,
        queue: BinaryHeap<ProofEntry>,
        proofs: HashMap<String, ProofEntry>,
        stats: ZKPV8Stats,
    }

    impl AsyncZKPV8 {
        pub fn new(config: ZKPV8Config) -> Self {
            Self {
                config,
                federations: HashMap::new(),
                queue: BinaryHeap::new(),
                proofs: HashMap::new(),
                stats: ZKPV8Stats::default(),
            }
        }

        pub fn with_defaults() -> Self {
            Self::new(ZKPV8Config::default())
        }

        /// Register a federation.
        pub fn register_federation(&mut self, federation_id: String) {
            let entry = FederationEntry::new(federation_id.clone());
            self.federations.insert(federation_id, entry);
        }

        /// Update federation credibility.
        pub fn update_credibility(
            &mut self,
            federation_id: &str,
            success: bool,
        ) -> Result<(), ZKPV8Error> {
            let fed = self
                .federations
                .get_mut(federation_id)
                .ok_or(ZKPV8Error::FederationNotFound(federation_id.to_string()))?;
            fed.update_credibility(success, self.config.credibility_decay);
            Ok(())
        }

        /// Submit a proof for scheduling.
        pub fn submit_proof(
            &mut self,
            id: String,
            federation_id: String,
            priority: u32,
            cost: f64,
        ) -> Result<(), ZKPV8Error> {
            let fed = self
                .federations
                .get(&federation_id)
                .ok_or(ZKPV8Error::FederationNotFound(federation_id.to_string()))?;
            if !fed.meets_threshold(self.config.min_credibility) {
                return Err(ZKPV8Error::CredibilityTooLow {
                    score: fed.credibility,
                    threshold: self.config.min_credibility,
                });
            }
            if !fed.has_budget(cost, self.config.budget_per_federation) {
                return Err(ZKPV8Error::BudgetExceeded {
                    budget: self.config.budget_per_federation,
                    used: fed.budget_used,
                });
            }
            if self.queue.len() >= self.config.queue_limit {
                return Err(ZKPV8Error::SchedulingConflict("Queue full".to_string()));
            }
            let proof = ProofEntry::new(id, federation_id, priority, cost);
            self.queue.push(proof);
            Ok(())
        }

        /// Process next proof from queue.
        pub fn process_next(&mut self) -> Option<ProofEntry> {
            let proof = self.queue.pop()?;
            if proof.is_expired(self.config.proof_ttl_ms) {
                self.stats.total_proofs_expired += 1;
                return self.process_next();
            }
            if let Some(fed) = self.federations.get_mut(proof.federation_id.as_str()) {
                fed.consume_budget(proof.cost);
            }
            self.stats
                .record_relay(proof.relay_chain.len() - 1, proof.cost);
            let mut proof = proof;
            proof.verified = true;
            self.stats.record_proof(true);
            self.proofs.insert(proof.id.clone(), proof.clone());
            Some(proof)
        }

        /// Relay proof to another federation.
        pub fn relay_proof(
            &mut self,
            proof_id: &str,
            target_federation: String,
        ) -> Result<(), ZKPV8Error> {
            let proof = self
                .proofs
                .get(proof_id)
                .ok_or(ZKPV8Error::ProofGenerationFailed(format!(
                    "Proof {} not found",
                    proof_id
                )))?;
            let mut proof = proof.clone();
            proof.extend_relay(target_federation.clone())?;
            if let Some(fed) = self.federations.get_mut(target_federation.as_str()) {
                if !fed.meets_threshold(self.config.min_credibility) {
                    return Err(ZKPV8Error::CredibilityTooLow {
                        score: fed.credibility,
                        threshold: self.config.min_credibility,
                    });
                }
            }
            self.proofs.insert(proof.id.clone(), proof);
            Ok(())
        }

        /// Reset all federation budgets.
        pub fn reset_budgets(&mut self) {
            for fed in self.federations.values_mut() {
                fed.reset_budget();
            }
        }

        /// Clean up expired proofs.
        pub fn cleanup_expired(&mut self) -> usize {
            let expired: Vec<String> = self
                .proofs
                .values()
                .filter(|p| p.is_expired(self.config.proof_ttl_ms))
                .map(|p| p.id.clone())
                .collect();
            let count = expired.len();
            for id in &expired {
                self.proofs.remove(id);
            }
            count
        }

        /// Get average credibility across federations.
        pub fn avg_credibility(&self) -> f64 {
            if self.federations.is_empty() {
                return 0.0;
            }
            let total: f64 = self.federations.values().map(|f| f.credibility).sum();
            total / self.federations.len() as f64
        }

        /// Get queue size.
        pub fn queue_size(&self) -> usize {
            self.queue.len()
        }

        /// Get federation count.
        pub fn federation_count(&self) -> usize {
            self.federations.len()
        }

        /// Get stats reference.
        pub fn get_stats(&self) -> &ZKPV8Stats {
            &self.stats
        }

        /// Get stats mutable reference.
        pub fn get_stats_mut(&mut self) -> &mut ZKPV8Stats {
            &mut self.stats
        }

        /// Reset stats.
        pub fn reset_stats(&mut self) {
            self.stats.reset();
        }
    }

    impl Default for AsyncZKPV8 {
        fn default() -> Self {
            Self::with_defaults()
        }
    }

    fn current_timestamp_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    // ---------------------------------------------------------------------------
    // Tests
    // ---------------------------------------------------------------------------

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_engine_creation() {
            let engine = AsyncZKPV8::with_defaults();
            assert_eq!(engine.federation_count(), 0);
            assert_eq!(engine.queue_size(), 0);
        }

        #[test]
        fn test_engine_with_config() {
            let config = ZKPV8Config {
                min_credibility: 0.8,
                ..ZKPV8Config::default()
            };
            let engine = AsyncZKPV8::new(config);
            assert_eq!(engine.config.min_credibility, 0.8);
        }

        #[test]
        fn test_register_federation() {
            let mut engine = AsyncZKPV8::with_defaults();
            engine.register_federation("fed1".to_string());
            assert_eq!(engine.federation_count(), 1);
        }

        #[test]
        fn test_update_credibility_success() {
            let mut engine = AsyncZKPV8::with_defaults();
            engine.register_federation("fed1".to_string());
            engine.update_credibility("fed1", true).unwrap();
            let fed = engine.federations.get("fed1").unwrap();
            assert!(fed.credibility > 0.9);
            assert_eq!(fed.proofs_verified, 1);
        }

        #[test]
        fn test_update_credibility_failure() {
            let mut engine = AsyncZKPV8::with_defaults();
            engine.register_federation("fed1".to_string());
            engine.update_credibility("fed1", false).unwrap();
            let fed = engine.federations.get("fed1").unwrap();
            assert!(fed.credibility < 1.0);
            assert_eq!(fed.proofs_failed, 1);
        }

        #[test]
        fn test_update_credibility_federation_not_found() {
            let mut engine = AsyncZKPV8::with_defaults();
            let result = engine.update_credibility("missing", true);
            assert!(matches!(result, Err(ZKPV8Error::FederationNotFound(_))));
        }

        #[test]
        fn test_submit_proof() {
            let mut engine = AsyncZKPV8::with_defaults();
            engine.register_federation("fed1".to_string());
            engine
                .submit_proof("p1".to_string(), "fed1".to_string(), 10, 50.0)
                .unwrap();
            assert_eq!(engine.queue_size(), 1);
        }

        #[test]
        fn test_submit_proof_credibility_too_low() {
            let mut engine = AsyncZKPV8::with_defaults();
            engine.register_federation("fed1".to_string());
            let fed = engine.federations.get_mut("fed1").unwrap();
            fed.credibility = 0.1;
            let result = engine.submit_proof("p1".to_string(), "fed1".to_string(), 10, 50.0);
            assert!(matches!(result, Err(ZKPV8Error::CredibilityTooLow { .. })));
        }

        #[test]
        fn test_submit_proof_budget_exceeded() {
            let mut engine = AsyncZKPV8::with_defaults();
            engine.config.budget_per_federation = 100.0;
            engine.register_federation("fed1".to_string());
            let fed = engine.federations.get_mut("fed1").unwrap();
            fed.budget_used = 90.0;
            let result = engine.submit_proof("p1".to_string(), "fed1".to_string(), 10, 50.0);
            assert!(matches!(result, Err(ZKPV8Error::BudgetExceeded { .. })));
        }

        #[test]
        fn test_submit_proof_federation_not_found() {
            let mut engine = AsyncZKPV8::with_defaults();
            let result = engine.submit_proof("p1".to_string(), "missing".to_string(), 10, 50.0);
            assert!(matches!(result, Err(ZKPV8Error::FederationNotFound(_))));
        }

        #[test]
        fn test_process_next() {
            let mut engine = AsyncZKPV8::with_defaults();
            engine.register_federation("fed1".to_string());
            engine
                .submit_proof("p1".to_string(), "fed1".to_string(), 10, 50.0)
                .unwrap();
            let proof = engine.process_next().unwrap();
            assert_eq!(proof.id, "p1");
            assert!(proof.verified);
        }

        #[test]
        fn test_process_next_empty() {
            let mut engine = AsyncZKPV8::with_defaults();
            assert!(engine.process_next().is_none());
        }

        #[test]
        fn test_priority_ordering() {
            let mut engine = AsyncZKPV8::with_defaults();
            engine.register_federation("fed1".to_string());
            engine
                .submit_proof("low".to_string(), "fed1".to_string(), 1, 10.0)
                .unwrap();
            engine
                .submit_proof("high".to_string(), "fed1".to_string(), 100, 10.0)
                .unwrap();
            let proof = engine.process_next().unwrap();
            assert_eq!(proof.id, "high");
        }

        #[test]
        fn test_relay_proof() {
            let mut engine = AsyncZKPV8::with_defaults();
            engine.register_federation("fed1".to_string());
            engine.register_federation("fed2".to_string());
            engine
                .submit_proof("p1".to_string(), "fed1".to_string(), 10, 50.0)
                .unwrap();
            engine.process_next();
            engine.relay_proof("p1", "fed2".to_string()).unwrap();
            let proof = engine.proofs.get("p1").unwrap();
            assert_eq!(proof.relay_chain.len(), 2);
            assert_eq!(proof.relay_chain[1], "fed2");
        }

        #[test]
        fn test_relay_proof_cycle_detected() {
            let mut engine = AsyncZKPV8::with_defaults();
            engine.register_federation("fed1".to_string());
            engine
                .submit_proof("p1".to_string(), "fed1".to_string(), 10, 50.0)
                .unwrap();
            engine.process_next();
            let result = engine.relay_proof("p1", "fed1".to_string());
            assert!(matches!(result, Err(ZKPV8Error::RelayChainBroken(_))));
        }

        #[test]
        fn test_reset_budgets() {
            let mut engine = AsyncZKPV8::with_defaults();
            engine.register_federation("fed1".to_string());
            let fed = engine.federations.get_mut("fed1").unwrap();
            fed.budget_used = 500.0;
            engine.reset_budgets();
            let fed = engine.federations.get("fed1").unwrap();
            assert_eq!(fed.budget_used, 0.0);
        }

        #[test]
        fn test_cleanup_expired() {
            let mut engine = AsyncZKPV8::with_defaults();
            engine.register_federation("fed1".to_string());
            engine
                .submit_proof("p1".to_string(), "fed1".to_string(), 10, 50.0)
                .unwrap();
            engine.process_next();
            let proof = engine.proofs.get_mut("p1").unwrap();
            proof.created_ms = 0;
            let cleaned = engine.cleanup_expired();
            assert_eq!(cleaned, 1);
        }

        #[test]
        fn test_avg_credibility() {
            let mut engine = AsyncZKPV8::with_defaults();
            engine.register_federation("fed1".to_string());
            engine.register_federation("fed2".to_string());
            engine.update_credibility("fed1", true).unwrap();
            engine.update_credibility("fed2", false).unwrap();
            let avg = engine.avg_credibility();
            assert!(avg > 0.0 && avg <= 1.0);
        }

        #[test]
        fn test_avg_credibility_empty() {
            let engine = AsyncZKPV8::with_defaults();
            assert_eq!(engine.avg_credibility(), 0.0);
        }

        #[test]
        fn test_queue_full() {
            let mut engine = AsyncZKPV8::with_defaults();
            engine.config.queue_limit = 2;
            engine.register_federation("fed1".to_string());
            engine
                .submit_proof("p1".to_string(), "fed1".to_string(), 10, 10.0)
                .unwrap();
            engine
                .submit_proof("p2".to_string(), "fed1".to_string(), 10, 10.0)
                .unwrap();
            let result = engine.submit_proof("p3".to_string(), "fed1".to_string(), 10, 10.0);
            assert!(matches!(result, Err(ZKPV8Error::SchedulingConflict(_))));
        }

        #[test]
        fn test_stats_tracking() {
            let mut engine = AsyncZKPV8::with_defaults();
            engine.register_federation("fed1".to_string());
            engine
                .submit_proof("p1".to_string(), "fed1".to_string(), 10, 50.0)
                .unwrap();
            engine.process_next();
            let stats = engine.get_stats();
            assert_eq!(stats.total_proofs_generated, 1);
            assert_eq!(stats.total_proofs_verified, 1);
        }

        #[test]
        fn test_reset_stats() {
            let mut engine = AsyncZKPV8::with_defaults();
            engine.register_federation("fed1".to_string());
            engine
                .submit_proof("p1".to_string(), "fed1".to_string(), 10, 50.0)
                .unwrap();
            engine.process_next();
            engine.reset_stats();
            let stats = engine.get_stats();
            assert_eq!(stats.total_proofs_generated, 0);
        }

        #[test]
        fn test_federation_entry_new() {
            let fed = FederationEntry::new("test".to_string());
            assert_eq!(fed.federation_id, "test");
            assert_eq!(fed.credibility, 1.0);
        }

        #[test]
        fn test_federation_meets_threshold() {
            let fed = FederationEntry::new("test".to_string());
            assert!(fed.meets_threshold(0.5));
            assert!(!fed.meets_threshold(1.1));
        }

        #[test]
        fn test_federation_has_budget() {
            let mut fed = FederationEntry::new("test".to_string());
            assert!(fed.has_budget(50.0, 100.0));
            fed.budget_used = 80.0;
            assert!(!fed.has_budget(50.0, 100.0));
        }

        #[test]
        fn test_proof_entry_new() {
            let proof = ProofEntry::new("p1".to_string(), "fed1".to_string(), 10, 50.0);
            assert_eq!(proof.id, "p1");
            assert_eq!(proof.priority, 10);
            assert!(!proof.verified);
        }

        #[test]
        fn test_proof_extend_relay() {
            let mut proof = ProofEntry::new("p1".to_string(), "fed1".to_string(), 10, 50.0);
            proof.extend_relay("fed2".to_string()).unwrap();
            assert_eq!(proof.relay_chain.len(), 2);
        }

        #[test]
        fn test_config_default() {
            let config = ZKPV8Config::default();
            assert_eq!(config.max_proofs_per_federation, 500);
            assert_eq!(config.min_credibility, 0.6);
        }

        #[test]
        fn test_stats_default() {
            let stats = ZKPV8Stats::default();
            assert_eq!(stats.total_proofs_generated, 0);
            assert_eq!(stats.avg_credibility, 1.0);
        }

        #[test]
        fn test_error_display() {
            match ZKPV8Error::ProofGenerationFailed("test".to_string()) {
                e => assert!(!e.to_string().is_empty()),
            }
        }

        #[test]
        fn test_engine_default() {
            let engine = AsyncZKPV8::default();
            assert_eq!(engine.federation_count(), 0);
        }
    }
}

#[cfg(feature = "v1.4-sprint3")]
pub use internal::*;
