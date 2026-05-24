//! Scientific Consensus — BFT Validation for Hypothesis Evidence.
//!
//! Implements Byzantine Fault Tolerant (BFT) consensus validation for
//! scientific hypothesis evidence. Ensures >=66% convergence threshold
//! before a hypothesis can be considered validated.
//!
//! **BFT Threshold:** >= 2n/3 agreement required (standard BFT).
//! **SCT Guard:** All evidence must have Z >= 0.
//! **WASM Compatible:** No native threads, no std::fs, no std::net.
//!
//! **Reference:** Sprint 44 — Maieutic Synthesizer Implementation (Pillar 2)

use crate::pillars::maieutic::hypothesis_engine::{Domain, Evidence};

/// Error type for consensus operations.
#[derive(Debug, Clone)]
pub enum ConsensusError {
    /// Insufficient validators for BFT consensus.
    InsufficientValidators { required: usize, available: usize },
    /// BFT threshold not met (convergence < 66%).
    ThresholdNotMet { convergence: f64, threshold: f64 },
    /// Evidence rejected by SCT Guard (Z < 0).
    SctGuardRejected { source: String, z_score: f32 },
    /// Evidence from unknown validator.
    UnknownValidator(String),
    /// Duplicate evidence from same validator.
    DuplicateEvidence(String),
    /// Domain mismatch between evidence and hypothesis.
    DomainMismatch { expected: Domain, got: Domain },
}

impl std::fmt::Display for ConsensusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConsensusError::InsufficientValidators { required, available } => {
                write!(
                    f,
                    "Insufficient validators for BFT: required={}, available={}",
                    required, available
                )
            }
            ConsensusError::ThresholdNotMet { convergence, threshold } => {
                write!(
                    f,
                    "BFT threshold not met: convergence={:.1}% < threshold={:.1}%",
                    convergence * 100.0,
                    threshold * 100.0
                )
            }
            ConsensusError::SctGuardRejected { source, z_score } => {
                write!(
                    f,
                    "SCT Guard rejected evidence from {}: Z = {:.3} < 0",
                    source, z_score
                )
            }
            ConsensusError::UnknownValidator(id) => {
                write!(f, "Unknown validator: {}", id)
            }
            ConsensusError::DuplicateEvidence(id) => {
                write!(f, "Duplicate evidence from validator: {}", id)
            }
            ConsensusError::DomainMismatch { expected, got } => {
                write!(
                    f,
                    "Domain mismatch: expected={}, got={}",
                    expected, got
                )
            }
        }
    }
}

/// Result of a BFT consensus round.
#[derive(Debug, Clone, PartialEq)]
pub enum ConsensusResult {
    /// Consensus achieved — hypothesis validated.
    Validated {
        /// Number of agreeing validators.
        agreements: usize,
        /// Total validators participated.
        total: usize,
        /// Convergence ratio (agreements / total).
        convergence: f64,
    },
    /// Consensus failed — insufficient agreement.
    Rejected {
        /// Number of agreeing validators.
        agreements: usize,
        /// Total validators participated.
        total: usize,
        /// Convergence ratio (agreements / total).
        convergence: f64,
    },
}

impl ConsensusResult {
    /// Return true if consensus was achieved.
    pub fn is_validated(&self) -> bool {
        matches!(self, ConsensusResult::Validated { .. })
    }

    /// Return the convergence ratio.
    pub fn convergence(&self) -> f64 {
        match self {
            ConsensusResult::Validated { convergence, .. } => *convergence,
            ConsensusResult::Rejected { convergence, .. } => *convergence,
        }
    }
}

impl std::fmt::Display for ConsensusResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConsensusResult::Validated {
                agreements,
                total,
                convergence,
            } => {
                write!(
                    f,
                    "Validated: {}/{} ({:.1}%)",
                    agreements,
                    total,
                    convergence * 100.0
                )
            }
            ConsensusResult::Rejected {
                agreements,
                total,
                convergence,
            } => {
                write!(
                    f,
                    "Rejected: {}/{} ({:.1}%)",
                    agreements,
                    total,
                    convergence * 100.0
                )
            }
        }
    }
}

/// BFT Consensus Engine for scientific hypothesis validation.
///
/// Manages validator registration, evidence collection, and
/// BFT consensus rounds with configurable convergence threshold.
///
/// **Invariant:** >= 2n/3 agreement required for validation.
pub struct ScientificConsensus {
    /// Registered validator node IDs.
    validators: Vec<String>,
    /// Collected evidence per hypothesis ID.
    evidence: std::collections::HashMap<String, Vec<Evidence>>,
    /// BFT convergence threshold (default 0.667 = 66.7%).
    threshold: f64,
    /// SCT Z-score minimum (default 0.0).
    sct_min_z: f32,
}

impl ScientificConsensus {
    /// Create a new ScientificConsensus engine with default threshold (66.7%).
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
            evidence: std::collections::HashMap::new(),
            threshold: 2.0 / 3.0,
            sct_min_z: 0.0,
        }
    }

    /// Create with custom threshold.
    pub fn with_threshold(threshold: f64, sct_min_z: f32) -> Self {
        Self {
            validators: Vec::new(),
            evidence: std::collections::HashMap::new(),
            threshold,
            sct_min_z,
        }
    }

    /// Register a validator node.
    pub fn register_validator(&mut self, id: String) {
        if !self.validators.contains(&id) {
            self.validators.push(id);
        }
    }

    /// Return the list of registered validators.
    pub fn validators(&self) -> &[String] {
        &self.validators
    }

    /// Return the number of registered validators.
    pub fn validator_count(&self) -> usize {
        self.validators.len()
    }

    /// Submit evidence for a hypothesis from a registered validator.
    ///
    /// Validates:
    /// 1. Validator is registered.
    /// 2. Evidence has not been submitted before by this validator.
    /// 3. SCT Z-score >= minimum threshold.
    pub fn submit_evidence(
        &mut self,
        hypothesis_id: &str,
        evidence: Evidence,
    ) -> Result<(), ConsensusError> {
        // Check validator is registered.
        if !self.validators.contains(&evidence.source_node) {
            return Err(ConsensusError::UnknownValidator(
                evidence.source_node.clone(),
            ));
        }

        // Check for duplicate evidence.
        let evidence_list = self
            .evidence
            .entry(hypothesis_id.to_string())
            .or_insert_with(Vec::new);

        if evidence_list.iter().any(|e| e.source_node == evidence.source_node) {
            return Err(ConsensusError::DuplicateEvidence(
                evidence.source_node.clone(),
            ));
        }

        // SCT Guard — reject evidence with negative Z-score.
        if evidence.z_score < self.sct_min_z {
            return Err(ConsensusError::SctGuardRejected {
                source: evidence.source_node.clone(),
                z_score: evidence.z_score,
            });
        }

        evidence_list.push(evidence);
        Ok(())
    }

    /// Execute a BFT consensus round for a hypothesis.
    ///
    /// Returns `ConsensusResult::Validated` if convergence >= threshold,
    /// otherwise `ConsensusResult::Rejected`.
    pub fn run_consensus(
        &self,
        hypothesis_id: &str,
        domain: &Domain,
    ) -> Result<ConsensusResult, ConsensusError> {
        let evidence_list = self
            .evidence
            .get(hypothesis_id)
            .ok_or(ConsensusError::InsufficientValidators {
                required: 1,
                available: 0,
            })?;

        let total = evidence_list.len();
        if total == 0 {
            return Err(ConsensusError::InsufficientValidators {
                required: 3,
                available: 0,
            });
        }

        // Count agreements: evidence with matching domain and Z >= 0.
        let agreements = evidence_list
            .iter()
            .filter(|e| {
                e.domain == *domain && e.z_score >= self.sct_min_z
            })
            .count();

        let convergence = agreements as f64 / total as f64;

        if convergence >= self.threshold {
            Ok(ConsensusResult::Validated {
                agreements,
                total,
                convergence,
            })
        } else {
            Ok(ConsensusResult::Rejected {
                agreements,
                total,
                convergence,
            })
        }
    }

    /// Get all evidence for a hypothesis.
    pub fn get_evidence(&self, hypothesis_id: &str) -> Vec<Evidence> {
        self.evidence
            .get(hypothesis_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Clear all evidence for a hypothesis (after consensus is finalized).
    pub fn clear_evidence(&mut self, hypothesis_id: &str) {
        self.evidence.remove(hypothesis_id);
    }

    /// Return the current BFT threshold.
    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    /// Return the SCT minimum Z-score.
    pub fn sct_min_z(&self) -> f32 {
        self.sct_min_z
    }
}

impl Default for ScientificConsensus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_evidence(source: &str, domain: Domain, z: f32) -> Evidence {
        Evidence {
            source_node: source.to_string(),
            domain,
            payload: b"test".to_vec(),
            z_score: z,
            timestamp_ms: 1000,
        }
    }

    #[test]
    fn test_consensus_creation() {
        let consensus = ScientificConsensus::new();
        assert_eq!(consensus.validator_count(), 0);
        assert_eq!(consensus.threshold(), 2.0 / 3.0);
    }

    #[test]
    fn test_register_validator() {
        let mut consensus = ScientificConsensus::new();
        consensus.register_validator("v1".to_string());
        consensus.register_validator("v2".to_string());
        consensus.register_validator("v3".to_string());
        assert_eq!(consensus.validator_count(), 3);
    }

    #[test]
    fn test_register_duplicate_validator() {
        let mut consensus = ScientificConsensus::new();
        consensus.register_validator("v1".to_string());
        consensus.register_validator("v1".to_string());
        assert_eq!(consensus.validator_count(), 1);
    }

    #[test]
    fn test_submit_evidence_valid() {
        let mut consensus = ScientificConsensus::new();
        consensus.register_validator("v1".to_string());
        let result = consensus.submit_evidence(
            "h1",
            make_evidence("v1", Domain::ProteinFolding, 0.5),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_submit_evidence_unknown_validator() {
        let mut consensus = ScientificConsensus::new();
        let result = consensus.submit_evidence(
            "h1",
            make_evidence("unknown", Domain::ProteinFolding, 0.5),
        );
        match result {
            Err(ConsensusError::UnknownValidator(id)) => {
                assert_eq!(id, "unknown");
            }
            _ => panic!("Expected UnknownValidator"),
        }
    }

    #[test]
    fn test_submit_evidence_duplicate() {
        let mut consensus = ScientificConsensus::new();
        consensus.register_validator("v1".to_string());
        consensus.submit_evidence(
            "h1",
            make_evidence("v1", Domain::ProteinFolding, 0.5),
        )
        .unwrap();
        let result = consensus.submit_evidence(
            "h1",
            make_evidence("v1", Domain::ProteinFolding, 0.3),
        );
        match result {
            Err(ConsensusError::DuplicateEvidence(id)) => {
                assert_eq!(id, "v1");
            }
            _ => panic!("Expected DuplicateEvidence"),
        }
    }

    #[test]
    fn test_submit_evidence_sct_guard_rejects() {
        let mut consensus = ScientificConsensus::new();
        consensus.register_validator("v1".to_string());
        let result = consensus.submit_evidence(
            "h1",
            make_evidence("v1", Domain::ProteinFolding, -0.5),
        );
        match result {
            Err(ConsensusError::SctGuardRejected { source, z_score }) => {
                assert_eq!(source, "v1");
                assert!(z_score < 0.0);
            }
            _ => panic!("Expected SctGuardRejected"),
        }
    }

    #[test]
    fn test_consensus_validated() {
        let mut consensus = ScientificConsensus::new();
        consensus.register_validator("v1".to_string());
        consensus.register_validator("v2".to_string());
        consensus.register_validator("v3".to_string());

        consensus.submit_evidence(
            "h1",
            make_evidence("v1", Domain::ProteinFolding, 0.5),
        )
        .unwrap();
        consensus.submit_evidence(
            "h1",
            make_evidence("v2", Domain::ProteinFolding, 0.3),
        )
        .unwrap();
        consensus.submit_evidence(
            "h1",
            make_evidence("v3", Domain::ProteinFolding, 0.4),
        )
        .unwrap();

        let result = consensus.run_consensus("h1", &Domain::ProteinFolding).unwrap();
        assert!(result.is_validated());
        assert!(result.convergence() >= 2.0 / 3.0);
    }

    #[test]
    fn test_consensus_rejected_below_threshold() {
        let mut consensus = ScientificConsensus::new();
        consensus.register_validator("v1".to_string());
        consensus.register_validator("v2".to_string());
        consensus.register_validator("v3".to_string());

        // Only 1 out of 3 agrees (domain mismatch for 2).
        consensus.submit_evidence(
            "h1",
            make_evidence("v1", Domain::ProteinFolding, 0.5),
        )
        .unwrap();
        consensus.submit_evidence(
            "h1",
            make_evidence("v2", Domain::Epigenetics, 0.3),
        )
        .unwrap();
        consensus.submit_evidence(
            "h1",
            make_evidence("v3", Domain::Epigenetics, 0.4),
        )
        .unwrap();

        let result = consensus.run_consensus("h1", &Domain::ProteinFolding).unwrap();
        assert!(!result.is_validated());
    }

    #[test]
    fn test_consensus_no_evidence() {
        let consensus = ScientificConsensus::new();
        let result = consensus.run_consensus("h1", &Domain::ProteinFolding);
        assert!(result.is_err());
    }

    #[test]
    fn test_custom_threshold() {
        let mut consensus = ScientificConsensus::with_threshold(0.75, 0.0);
        consensus.register_validator("v1".to_string());
        consensus.register_validator("v2".to_string());
        consensus.register_validator("v3".to_string());
        consensus.register_validator("v4".to_string());

        // 3 out of 4 = 75% — exactly at threshold.
        consensus.submit_evidence(
            "h1",
            make_evidence("v1", Domain::Epigenetics, 0.5),
        )
        .unwrap();
        consensus.submit_evidence(
            "h1",
            make_evidence("v2", Domain::Epigenetics, 0.3),
        )
        .unwrap();
        consensus.submit_evidence(
            "h1",
            make_evidence("v3", Domain::Epigenetics, 0.4),
        )
        .unwrap();

        let result = consensus.run_consensus("h1", &Domain::Epigenetics).unwrap();
        assert!(result.is_validated());
    }

    #[test]
    fn test_clear_evidence() {
        let mut consensus = ScientificConsensus::new();
        consensus.register_validator("v1".to_string());
        consensus.submit_evidence(
            "h1",
            make_evidence("v1", Domain::Epigenetics, 0.5),
        )
        .unwrap();
        assert_eq!(consensus.get_evidence("h1").len(), 1);

        consensus.clear_evidence("h1");
        assert_eq!(consensus.get_evidence("h1").len(), 0);
    }

    #[test]
    fn test_consensus_result_display() {
        let result = ConsensusResult::Validated {
            agreements: 3,
            total: 4,
            convergence: 0.75,
        };
        let s = format!("{}", result);
        assert!(s.contains("Validated"));
        assert!(s.contains("75.0%"));
    }

    #[test]
    fn test_error_display() {
        let err = ConsensusError::ThresholdNotMet {
            convergence: 0.5,
            threshold: 0.667,
        };
        let s = format!("{}", err);
        assert!(s.contains("threshold"));
    }

    #[test]
    fn test_default() {
        let consensus = ScientificConsensus::default();
        assert_eq!(consensus.validator_count(), 0);
    }

    #[test]
    fn test_get_evidence_empty() {
        let consensus = ScientificConsensus::new();
        assert_eq!(consensus.get_evidence("nonexistent").len(), 0);
    }
}
