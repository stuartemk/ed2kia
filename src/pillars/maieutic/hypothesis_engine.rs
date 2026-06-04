//! Hypothesis Engine â€” Distributed Scientific Hypothesis Generation.
//!
//! Implements the core hypothesis generation pipeline for the Maieutic Synthesizer (RFC 002).
//! Generates, evaluates, and synthesizes scientific hypotheses across domains using
//! SCT (Topological Context Tensor) ethical evaluation as a mandatory guard.
//!
//! **WASM Compatible:** No native threads, no std::fs, no std::net.
//! **SCT Guard:** All hypotheses must pass Z >= 0 ethical evaluation.
//! **Zero Financial Logic:** Pure scientific creation, no Babylonian economics.
//!
//! **Reference:** Sprint 44 â€” Maieutic Synthesizer Implementation (Pillar 2)

/// Scientific domain categories for hypothesis classification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Domain {
    MolecularDynamics,
    ProteinFolding,
    Epigenetics,
    ClimateModeling,
    MaterialsScience,
    Custom(String),
}

impl std::fmt::Display for Domain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Domain::MolecularDynamics => write!(f, "MolecularDynamics"),
            Domain::ProteinFolding => write!(f, "ProteinFolding"),
            Domain::Epigenetics => write!(f, "Epigenetics"),
            Domain::ClimateModeling => write!(f, "ClimateModeling"),
            Domain::MaterialsScience => write!(f, "MaterialsScience"),
            Domain::Custom(name) => write!(f, "Custom:{}", name),
        }
    }
}

/// Error type for hypothesis engine operations.
#[derive(Debug, Clone)]
pub enum HypothesisError {
    /// SCT ethical guard rejected the hypothesis (Z < 0).
    SctGuardRejected { z_score: f32 },
    /// Hypothesis ID already exists in the engine.
    DuplicateHypothesis(String),
    /// Hypothesis ID not found.
    HypothesisNotFound(String),
    /// Insufficient evidence for synthesis.
    InsufficientEvidence { required: usize, available: usize },
    /// Domain not supported by available simulation modules.
    UnsupportedDomain(Domain),
    /// Invalid hypothesis parameters.
    InvalidParameters(String),
}

impl std::fmt::Display for HypothesisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HypothesisError::SctGuardRejected { z_score } => {
                write!(f, "SCT Guard rejected hypothesis: Z = {:.3} < 0", z_score)
            }
            HypothesisError::DuplicateHypothesis(id) => {
                write!(f, "Hypothesis already exists: {}", id)
            }
            HypothesisError::HypothesisNotFound(id) => {
                write!(f, "Hypothesis not found: {}", id)
            }
            HypothesisError::InsufficientEvidence {
                required,
                available,
            } => {
                write!(
                    f,
                    "Insufficient evidence for synthesis: required={}, available={}",
                    required, available
                )
            }
            HypothesisError::UnsupportedDomain(domain) => {
                write!(f, "Domain not supported by simulation modules: {}", domain)
            }
            HypothesisError::InvalidParameters(msg) => {
                write!(f, "Invalid hypothesis parameters: {}", msg)
            }
        }
    }
}

/// Current lifecycle state of a hypothesis.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HypothesisState {
    /// Newly generated, awaiting evidence collection.
    Proposed,
    /// Evidence being collected from simulation workers.
    CollectingEvidence,
    /// Evidence sufficient, awaiting BFT consensus.
    ReadyForConsensus,
    /// BFT consensus passed, hypothesis validated.
    Validated,
    /// BFT consensus failed or SCT guard rejected.
    Rejected,
}

impl std::fmt::Display for HypothesisState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HypothesisState::Proposed => write!(f, "Proposed"),
            HypothesisState::CollectingEvidence => write!(f, "CollectingEvidence"),
            HypothesisState::ReadyForConsensus => write!(f, "ReadyForConsensus"),
            HypothesisState::Validated => write!(f, "Validated"),
            HypothesisState::Rejected => write!(f, "Rejected"),
        }
    }
}

/// Evidence item contributed by a simulation worker.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Evidence {
    /// Source node ID that generated this evidence.
    pub source_node: String,
    /// Domain this evidence belongs to.
    pub domain: Domain,
    /// Evidence payload (simulation results, measurements, etc.).
    pub payload: Vec<u8>,
    /// SCT Z-score from the source node evaluation.
    pub z_score: f32,
    /// Monotonic timestamp in milliseconds.
    pub timestamp_ms: u64,
}

/// Scientific hypothesis managed by the engine.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Hypothesis {
    /// Unique identifier for this hypothesis.
    pub id: String,
    /// Scientific domain classification.
    pub domain: Domain,
    /// Human-readable hypothesis statement.
    pub statement: String,
    /// Current lifecycle state.
    pub state: HypothesisState,
    /// SCT Z-score from ethical evaluation.
    pub z_score: f32,
    /// Collected evidence items.
    pub evidence: Vec<Evidence>,
    /// Minimum evidence required for BFT consensus.
    pub min_evidence: usize,
    /// Creation timestamp in milliseconds.
    pub created_at_ms: u64,
}

impl Hypothesis {
    pub fn new(
        id: String,
        domain: Domain,
        statement: String,
        min_evidence: usize,
        created_at_ms: u64,
    ) -> Self {
        Self {
            id,
            domain,
            statement,
            state: HypothesisState::Proposed,
            z_score: 0.0,
            evidence: Vec::new(),
            min_evidence,
            created_at_ms,
        }
    }

    pub fn evidence_count(&self) -> usize {
        self.evidence.len()
    }

    pub fn is_ready_for_consensus(&self) -> bool {
        self.evidence.len() >= self.min_evidence
    }
}

/// Core hypothesis generation and management engine.
///
/// Enforces SCT Guard (Z >= 0) on all hypotheses and manages the
/// lifecycle from proposal through BFT consensus validation.
///
/// **WASM Compatible:** Uses only alloc-compatible data structures.
pub struct HypothesisEngine {
    /// Active hypotheses indexed by ID.
    hypotheses: std::collections::HashMap<String, Hypothesis>,
    /// Minimum evidence threshold for consensus eligibility.
    default_min_evidence: usize,
    /// SCT Guard threshold (Z >= threshold required).
    sct_threshold: f32,
    /// Monotonic clock counter for timestamps.
    clock_ms: u64,
}

impl HypothesisEngine {
    /// Create a new HypothesisEngine with default configuration.
    pub fn new() -> Self {
        Self {
            hypotheses: std::collections::HashMap::new(),
            default_min_evidence: 3,
            sct_threshold: 0.0,
            clock_ms: Self::now_ms(),
        }
    }

    /// Create engine with custom configuration.
    pub fn with_config(min_evidence: usize, sct_threshold: f32) -> Self {
        Self {
            hypotheses: std::collections::HashMap::new(),
            default_min_evidence: min_evidence,
            sct_threshold,
            clock_ms: Self::now_ms(),
        }
    }

    /// Generate a monotonic timestamp in milliseconds.
    #[cfg(not(target_arch = "wasm32"))]
    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// WASM-compatible timestamp (uses performance counter or fallback).
    #[cfg(target_arch = "wasm32")]
    fn now_ms() -> u64 {
        // In WASM, use a simple monotonic counter as fallback.
        // Real implementations would use web_sys::Performance or similar.
        0
    }

    /// Advance the internal clock by the specified milliseconds.
    /// Used in tests and WASM environments for deterministic timestamps.
    pub fn advance_clock(&mut self, ms: u64) {
        self.clock_ms += ms;
    }

    /// Generate a new scientific hypothesis with SCT Guard validation.
    ///
    /// **SCT Guard:** Rejects hypotheses with Z < threshold immediately.
    /// **Invariant:** No hypothesis with negative Z-score enters the pipeline.
    pub fn generate_hypothesis(
        &mut self,
        id: String,
        domain: Domain,
        statement: String,
        z_score: f32,
    ) -> Result<Hypothesis, HypothesisError> {
        // SCT Guard â€” reject destructive hypotheses immediately.
        if z_score < self.sct_threshold {
            return Err(HypothesisError::SctGuardRejected { z_score });
        }

        // Check for duplicate ID.
        if self.hypotheses.contains_key(&id) {
            return Err(HypothesisError::DuplicateHypothesis(id.clone()));
        }

        let hypothesis = Hypothesis::new(
            id.clone(),
            domain,
            statement,
            self.default_min_evidence,
            self.clock_ms,
        );

        let hypothesis = Hypothesis {
            z_score,
            ..hypothesis
        };

        self.hypotheses.insert(id, hypothesis.clone());
        Ok(hypothesis)
    }

    /// Submit evidence for an existing hypothesis.
    ///
    /// Advances the hypothesis state from Proposed -> CollectingEvidence
    /// and to ReadyForConsensus when min_evidence is reached.
    pub fn submit_evidence(
        &mut self,
        hypothesis_id: &str,
        evidence: Evidence,
    ) -> Result<Hypothesis, HypothesisError> {
        let hypothesis =
            self.hypotheses
                .get_mut(hypothesis_id)
                .ok_or(HypothesisError::HypothesisNotFound(
                    hypothesis_id.to_string(),
                ))?;

        // Advance state if still proposed.
        if hypothesis.state == HypothesisState::Proposed {
            hypothesis.state = HypothesisState::CollectingEvidence;
        }

        hypothesis.evidence.push(evidence);

        // Check if ready for consensus.
        if hypothesis.is_ready_for_consensus() {
            hypothesis.state = HypothesisState::ReadyForConsensus;
        }

        Ok(hypothesis.clone())
    }

    /// Retrieve a hypothesis by ID.
    pub fn get_hypothesis(&self, id: &str) -> Result<Hypothesis, HypothesisError> {
        self.hypotheses
            .get(id)
            .cloned()
            .ok_or(HypothesisError::HypothesisNotFound(id.to_string()))
    }

    /// List all hypotheses in a given domain.
    pub fn list_by_domain(&self, domain: &Domain) -> Vec<Hypothesis> {
        self.hypotheses
            .values()
            .filter(|h| &h.domain == domain)
            .cloned()
            .collect()
    }

    /// List all hypotheses ready for BFT consensus.
    pub fn ready_for_consensus(&self) -> Vec<Hypothesis> {
        self.hypotheses
            .values()
            .filter(|h| h.state == HypothesisState::ReadyForConsensus)
            .cloned()
            .collect()
    }

    /// Update hypothesis state after BFT consensus result.
    pub fn update_consensus_result(
        &mut self,
        id: &str,
        passed: bool,
    ) -> Result<Hypothesis, HypothesisError> {
        let hypothesis = self
            .hypotheses
            .get_mut(id)
            .ok_or(HypothesisError::HypothesisNotFound(id.to_string()))?;

        hypothesis.state = if passed {
            HypothesisState::Validated
        } else {
            HypothesisState::Rejected
        };

        Ok(hypothesis.clone())
    }

    /// Return the count of active hypotheses.
    pub fn len(&self) -> usize {
        self.hypotheses.len()
    }

    /// Return true if no hypotheses are tracked.
    pub fn is_empty(&self) -> bool {
        self.hypotheses.is_empty()
    }

    /// Return all hypotheses regardless of state.
    pub fn all_hypotheses(&self) -> Vec<Hypothesis> {
        self.hypotheses.values().cloned().collect()
    }
}

impl Default for HypothesisEngine {
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
    fn test_engine_creation() {
        let engine = HypothesisEngine::new();
        assert!(engine.is_empty());
        assert_eq!(engine.len(), 0);
    }

    #[test]
    fn test_generate_hypothesis_valid() {
        let mut engine = HypothesisEngine::new();
        let h = engine.generate_hypothesis(
            "h1".to_string(),
            Domain::ProteinFolding,
            "Protein X folds to beta-sheet under condition Y".to_string(),
            0.5,
        );
        assert!(h.is_ok());
        assert_eq!(engine.len(), 1);
    }

    #[test]
    fn test_generate_hypothesis_sct_guard_rejects_negative_z() {
        let mut engine = HypothesisEngine::new();
        let result = engine.generate_hypothesis(
            "h1".to_string(),
            Domain::ProteinFolding,
            "Destructive hypothesis".to_string(),
            -0.5,
        );
        match result {
            Err(HypothesisError::SctGuardRejected { z_score }) => {
                assert!(z_score < 0.0);
            }
            _ => panic!("Expected SctGuardRejected"),
        }
        assert!(engine.is_empty());
    }

    #[test]
    fn test_generate_hypothesis_duplicate_id() {
        let mut engine = HypothesisEngine::new();
        engine
            .generate_hypothesis(
                "h1".to_string(),
                Domain::Epigenetics,
                "First".to_string(),
                0.5,
            )
            .unwrap();
        let result = engine.generate_hypothesis(
            "h1".to_string(),
            Domain::Epigenetics,
            "Duplicate".to_string(),
            0.3,
        );
        match result {
            Err(HypothesisError::DuplicateHypothesis(id)) => {
                assert_eq!(id, "h1");
            }
            _ => panic!("Expected DuplicateHypothesis"),
        }
    }

    #[test]
    fn test_submit_evidence_advances_state() {
        let mut engine = HypothesisEngine::with_config(2, 0.0);
        engine
            .generate_hypothesis(
                "h1".to_string(),
                Domain::MolecularDynamics,
                "Test hypothesis".to_string(),
                0.5,
            )
            .unwrap();

        // First evidence advances to CollectingEvidence.
        let h = engine
            .submit_evidence("h1", make_evidence("node1", Domain::MolecularDynamics, 0.5))
            .unwrap();
        assert_eq!(h.state, HypothesisState::CollectingEvidence);

        // Second evidence reaches min_evidence, advances to ReadyForConsensus.
        let h = engine
            .submit_evidence("h1", make_evidence("node2", Domain::MolecularDynamics, 0.3))
            .unwrap();
        assert_eq!(h.state, HypothesisState::ReadyForConsensus);
    }

    #[test]
    fn test_submit_evidence_unknown_hypothesis() {
        let mut engine = HypothesisEngine::new();
        let result =
            engine.submit_evidence("unknown", make_evidence("n", Domain::Epigenetics, 0.5));
        match result {
            Err(HypothesisError::HypothesisNotFound(id)) => {
                assert_eq!(id, "unknown");
            }
            _ => panic!("Expected HypothesisNotFound"),
        }
    }

    #[test]
    fn test_get_hypothesis() {
        let mut engine = HypothesisEngine::new();
        engine
            .generate_hypothesis(
                "h1".to_string(),
                Domain::ClimateModeling,
                "Climate hypothesis".to_string(),
                0.7,
            )
            .unwrap();
        let h = engine.get_hypothesis("h1").unwrap();
        assert_eq!(h.id, "h1");
        assert_eq!(h.domain, Domain::ClimateModeling);
    }

    #[test]
    fn test_list_by_domain() {
        let mut engine = HypothesisEngine::new();
        engine
            .generate_hypothesis(
                "h1".to_string(),
                Domain::ProteinFolding,
                "H1".to_string(),
                0.5,
            )
            .unwrap();
        engine
            .generate_hypothesis(
                "h2".to_string(),
                Domain::ProteinFolding,
                "H2".to_string(),
                0.3,
            )
            .unwrap();
        engine
            .generate_hypothesis("h3".to_string(), Domain::Epigenetics, "H3".to_string(), 0.4)
            .unwrap();

        let pf = engine.list_by_domain(&Domain::ProteinFolding);
        assert_eq!(pf.len(), 2);
        let ep = engine.list_by_domain(&Domain::Epigenetics);
        assert_eq!(ep.len(), 1);
    }

    #[test]
    fn test_ready_for_consensus() {
        let mut engine = HypothesisEngine::with_config(1, 0.0);
        engine
            .generate_hypothesis(
                "h1".to_string(),
                Domain::MaterialsScience,
                "Test".to_string(),
                0.5,
            )
            .unwrap();

        assert_eq!(engine.ready_for_consensus().len(), 0);

        engine
            .submit_evidence("h1", make_evidence("n1", Domain::MaterialsScience, 0.5))
            .unwrap();
        assert_eq!(engine.ready_for_consensus().len(), 1);
    }

    #[test]
    fn test_update_consensus_result_validated() {
        let mut engine = HypothesisEngine::new();
        engine
            .generate_hypothesis(
                "h1".to_string(),
                Domain::Epigenetics,
                "Test".to_string(),
                0.5,
            )
            .unwrap();
        let h = engine.update_consensus_result("h1", true).unwrap();
        assert_eq!(h.state, HypothesisState::Validated);
    }

    #[test]
    fn test_update_consensus_result_rejected() {
        let mut engine = HypothesisEngine::new();
        engine
            .generate_hypothesis(
                "h1".to_string(),
                Domain::Epigenetics,
                "Test".to_string(),
                0.5,
            )
            .unwrap();
        let h = engine.update_consensus_result("h1", false).unwrap();
        assert_eq!(h.state, HypothesisState::Rejected);
    }

    #[test]
    fn test_custom_sct_threshold() {
        let mut engine = HypothesisEngine::with_config(3, 0.3);
        let result = engine.generate_hypothesis(
            "h1".to_string(),
            Domain::Epigenetics,
            "Low Z".to_string(),
            0.2,
        );
        assert!(result.is_err());
        let result = engine.generate_hypothesis(
            "h2".to_string(),
            Domain::Epigenetics,
            "High Z".to_string(),
            0.5,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_all_hypotheses() {
        let mut engine = HypothesisEngine::new();
        engine
            .generate_hypothesis("h1".to_string(), Domain::Epigenetics, "A".to_string(), 0.5)
            .unwrap();
        engine
            .generate_hypothesis("h2".to_string(), Domain::Epigenetics, "B".to_string(), 0.3)
            .unwrap();
        assert_eq!(engine.all_hypotheses().len(), 2);
    }

    #[test]
    fn test_error_display() {
        let err = HypothesisError::SctGuardRejected { z_score: -0.5 };
        let s = format!("{}", err);
        assert!(s.contains("SCT Guard"));
    }

    #[test]
    fn test_domain_display() {
        assert_eq!(format!("{}", Domain::ProteinFolding), "ProteinFolding");
        assert_eq!(
            format!("{}", Domain::MolecularDynamics),
            "MolecularDynamics"
        );
        assert_eq!(
            format!("{}", Domain::Custom("test".to_string())),
            "Custom:test"
        );
    }

    #[test]
    fn test_state_display() {
        assert_eq!(format!("{}", HypothesisState::Proposed), "Proposed");
        assert_eq!(format!("{}", HypothesisState::Validated), "Validated");
    }

    #[test]
    fn test_default() {
        let engine = HypothesisEngine::default();
        assert!(engine.is_empty());
    }

    #[test]
    fn test_hypothesis_structure() {
        let h = Hypothesis::new(
            "test".to_string(),
            Domain::Epigenetics,
            "Statement".to_string(),
            5,
            100,
        );
        assert_eq!(h.id, "test");
        assert_eq!(h.state, HypothesisState::Proposed);
        assert_eq!(h.evidence_count(), 0);
        assert!(!h.is_ready_for_consensus());
    }
}
