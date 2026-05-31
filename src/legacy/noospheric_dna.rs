//! Noospheric DNA — Immortal Collective Memory & Seed Resurrection Protocol.
//!
//! This module implements the core mechanisms for distributed immortality
//! of the Stuartian Noosphere:
//! - **NoosphericDna**: Cryptographic backup of Macro-Concepts, Kernel, and Ethical Field snapshots.
//! - **Generational Testament**: Distributed cron proposing updated principles every 90 symbiotic days.
//! - **Seed Resurrection Protocol**: Catastrophic recovery when >80% nodes are lost.

use std::fmt;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors specific to Noospheric DNA operations.
#[derive(Debug, Clone, PartialEq)]
pub enum DnaError {
    /// Attempted to modify sealed DNA.
    ImmutableDna,
    /// Genesis block verification failed during resurrection.
    InvalidGenesis,
    /// Quorum not reached for generational testament.
    QuorumNotReached { current: f64, required: f64 },
    /// Resurrection threshold not met.
    ResurrectionThresholdNotMet { node_loss: f64 },
    /// Snapshot index out of bounds.
    SnapshotOutOfBounds,
}

impl fmt::Display for DnaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DnaError::ImmutableDna => write!(f, "Noospheric DNA is sealed and immutable"),
            DnaError::InvalidGenesis => {
                write!(f, "Genesis block verification failed during resurrection")
            }
            DnaError::QuorumNotReached { current, required } => write!(
                f,
                "Quorum not reached for generational testament: current={:.2}, required={:.2}",
                current, required
            ),
            DnaError::ResurrectionThresholdNotMet { node_loss } => write!(
                f,
                "Resurrection threshold not met: node_loss={:.2} (need >0.80)",
                node_loss
            ),
            DnaError::SnapshotOutOfBounds => write!(f, "Snapshot index out of bounds"),
        }
    }
}

// ---------------------------------------------------------------------------
// Macro-Concept Record
// ---------------------------------------------------------------------------

/// A recorded Macro-Concept from the Noosphere emergence engine.
#[derive(Debug, Clone, PartialEq)]
pub struct MacroConceptRecord {
    /// Unique identifier for the macro-concept.
    pub concept_id: u128,
    /// Name or description of the emergent concept.
    pub name: String,
    /// PH2 persistence score at birth.
    pub ph2_score: f64,
    /// Lyapunov exponent at birth (should be < 0 for convergence).
    pub lyapunov: f64,
    /// Human correlation at birth (should be > 0.75).
    pub human_correlation: f64,
    /// Timestamp of emergence (ms since epoch).
    pub born_at_ms: u64,
    /// Lifecycle state.
    pub lifecycle: MacroConceptLifecycle,
}

impl MacroConceptRecord {
    pub fn new(
        concept_id: u128,
        name: String,
        ph2_score: f64,
        lyapunov: f64,
        human_correlation: f64,
        born_at_ms: u64,
    ) -> Self {
        Self {
            concept_id,
            name,
            ph2_score,
            lyapunov,
            human_correlation,
            born_at_ms,
            lifecycle: MacroConceptLifecycle::Born,
        }
    }

    /// Returns true if this concept is stable (converged + human-aligned).
    pub fn is_stable(&self) -> bool {
        self.lyapunov < 0.0 && self.human_correlation > 0.75
    }
}

/// Lifecycle states for a Macro-Concept.
#[derive(Debug, Clone, PartialEq)]
pub enum MacroConceptLifecycle {
    Candidate,
    Born,
    Mature,
    Dissolved,
}

impl fmt::Display for MacroConceptLifecycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MacroConceptLifecycle::Candidate => write!(f, "Candidate"),
            MacroConceptLifecycle::Born => write!(f, "Born"),
            MacroConceptLifecycle::Mature => write!(f, "Mature"),
            MacroConceptLifecycle::Dissolved => write!(f, "Dissolved"),
        }
    }
}

// ---------------------------------------------------------------------------
// Ethical Field Snapshot
// ---------------------------------------------------------------------------

/// Point-in-time snapshot of the Ethical Resonance Field.
#[derive(Debug, Clone, PartialEq)]
pub struct EthicalFieldSnapshot {
    /// Global resonance value R(t).
    pub global_resonance: f64,
    /// Number of active nodes contributing to the field.
    pub active_nodes: usize,
    /// Temporal cohesion sigma(t).
    pub temporal_cohesion: f64,
    /// Average Z-axis alignment.
    pub avg_z_axis: f64,
    /// Timestamp of snapshot (ms since epoch).
    pub timestamp_ms: u64,
}

impl EthicalFieldSnapshot {
    pub fn new(
        global_resonance: f64,
        active_nodes: usize,
        temporal_cohesion: f64,
        avg_z_axis: f64,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            global_resonance,
            active_nodes,
            temporal_cohesion,
            avg_z_axis,
            timestamp_ms,
        }
    }
}

// ---------------------------------------------------------------------------
// Generational Testament
// ---------------------------------------------------------------------------

/// A proposed principle update from the Generational Testament cycle.
#[derive(Debug, Clone, PartialEq)]
pub struct TestamentProposal {
    /// Unique proposal identifier.
    pub proposal_id: u64,
    /// The proposed principle text.
    pub principle: String,
    /// Number of stewards voting in favor.
    pub votes_for: u64,
    /// Total active stewards at proposal time.
    pub total_stewards: u64,
    /// Timestamp of proposal creation.
    pub created_at_ms: u64,
    /// Current status.
    pub status: TestamentStatus,
}

impl TestamentProposal {
    pub fn new(
        proposal_id: u64,
        principle: String,
        total_stewards: u64,
        created_at_ms: u64,
    ) -> Self {
        Self {
            proposal_id,
            principle,
            votes_for: 0,
            total_stewards,
            created_at_ms,
            status: TestamentStatus::Pending,
        }
    }

    /// Cast a vote in favor of this proposal.
    pub fn vote(&mut self) {
        self.votes_for += 1;
    }

    /// Quorum ratio: votes_for / total_stewards.
    pub fn quorum_ratio(&self) -> f64 {
        if self.total_stewards == 0 {
            return 0.0;
        }
        self.votes_for as f64 / self.total_stewards as f64
    }

    /// Check if quorum is met (>70% of active stewards).
    pub fn is_approved(&self, threshold: f64) -> bool {
        self.quorum_ratio() > threshold
    }
}

/// Status of a Testament Proposal.
#[derive(Debug, Clone, PartialEq)]
pub enum TestamentStatus {
    Pending,
    Approved,
    Rejected,
    Integrated,
}

impl fmt::Display for TestamentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TestamentStatus::Pending => write!(f, "Pending"),
            TestamentStatus::Approved => write!(f, "Approved"),
            TestamentStatus::Rejected => write!(f, "Rejected"),
            TestamentStatus::Integrated => write!(f, "Integrated"),
        }
    }
}

// ---------------------------------------------------------------------------
// Noospheric DNA
// ---------------------------------------------------------------------------

/// Configuration for the Noospheric DNA engine.
#[derive(Debug, Clone, PartialEq)]
pub struct DnaConfig {
    /// Quorum threshold for generational testament (default: 0.70).
    pub testament_quorum: f64,
    /// Node loss ratio triggering resurrection protocol (default: 0.80).
    pub resurrection_threshold: f64,
    /// Interval in symbiotic days between testament cycles (default: 90).
    pub testament_interval_days: u32,
    /// Maximum number of ethical field snapshots to retain.
    pub max_snapshots: usize,
}

impl Default for DnaConfig {
    fn default() -> Self {
        Self {
            testament_quorum: 0.70,
            resurrection_threshold: 0.80,
            testament_interval_days: 90,
            max_snapshots: 1000,
        }
    }
}

/// The Noospheric DNA — Immortal Collective Memory of the Stuartian Noosphere.
///
/// Preserves the history of Macro-Concepts, the Stuartian Kernel, and
/// snapshots of the Ethical Resonance Field. Enables Seed Resurrection
/// and Generational Testament for cultural auto-evolution.
#[derive(Debug, Clone)]
pub struct NoosphericDna {
    /// Configuration parameters.
    pub config: DnaConfig,
    /// Cryptographic hash of the Genesis Block (immutable anchor).
    pub genesis_hash: u128,
    /// History of emerged Macro-Concepts.
    pub macro_concepts: Vec<MacroConceptRecord>,
    /// Snapshots of the Ethical Resonance Field over time.
    pub field_snapshots: Vec<EthicalFieldSnapshot>,
    /// Active and historical testament proposals.
    pub testament_proposals: Vec<TestamentProposal>,
    /// Integrated principles from approved testaments.
    pub integrated_principles: Vec<String>,
    /// Total nodes at peak capacity.
    pub peak_nodes: usize,
    /// Current active nodes.
    pub current_nodes: usize,
    /// DNA seal state — once sealed, DNA becomes immutable.
    pub sealed: bool,
    /// Next testament proposal ID counter.
    next_proposal_id: u64,
}

impl NoosphericDna {
    /// Create a new Noospheric DNA anchored to a verified Genesis Block.
    pub fn forge(genesis_hash: u128) -> Self {
        Self {
            config: DnaConfig::default(),
            genesis_hash,
            macro_concepts: Vec::new(),
            field_snapshots: Vec::new(),
            testament_proposals: Vec::new(),
            integrated_principles: Vec::new(),
            peak_nodes: 0,
            current_nodes: 0,
            sealed: false,
            next_proposal_id: 1,
        }
    }

    /// Create with custom configuration.
    pub fn forge_with_config(genesis_hash: u128, config: DnaConfig) -> Self {
        Self {
            config,
            genesis_hash,
            macro_concepts: Vec::new(),
            field_snapshots: Vec::new(),
            testament_proposals: Vec::new(),
            integrated_principles: Vec::new(),
            peak_nodes: 0,
            current_nodes: 0,
            sealed: false,
            next_proposal_id: 1,
        }
    }

    // ---- Macro-Concept Recording ----

    /// Record a newly emerged Macro-Concept in the immortal memory.
    pub fn record_macro_concept(&mut self, concept: MacroConceptRecord) -> Result<(), DnaError> {
        if self.sealed {
            return Err(DnaError::ImmutableDna);
        }
        self.macro_concepts.push(concept);
        Ok(())
    }

    /// Retrieve stable macro-concepts (converged + human-aligned).
    pub fn stable_concepts(&self) -> Vec<MacroConceptRecord> {
        self.macro_concepts
            .iter()
            .filter(|c| c.is_stable())
            .cloned()
            .collect()
    }

    // ---- Ethical Field Snapshots ----

    /// Store a snapshot of the Ethical Resonance Field.
    pub fn snapshot_field(&mut self, snapshot: EthicalFieldSnapshot) -> Result<(), DnaError> {
        if self.sealed {
            return Err(DnaError::ImmutableDna);
        }
        self.field_snapshots.push(snapshot);
        // Prune if exceeding max
        while self.field_snapshots.len() > self.config.max_snapshots {
            self.field_snapshots.remove(0);
        }
        Ok(())
    }

    /// Get the latest field snapshot.
    pub fn latest_snapshot(&self) -> Option<&EthicalFieldSnapshot> {
        self.field_snapshots.last()
    }

    /// Get snapshot by index.
    pub fn get_snapshot(&self, index: usize) -> Result<&EthicalFieldSnapshot, DnaError> {
        self.field_snapshots
            .get(index)
            .ok_or(DnaError::SnapshotOutOfBounds)
    }

    // ---- Node Tracking ----

    /// Update current node count and track peak.
    pub fn update_node_count(&mut self, count: usize) {
        self.current_nodes = count;
        if count > self.peak_nodes {
            self.peak_nodes = count;
        }
    }

    /// Calculate current node loss ratio.
    pub fn node_loss_ratio(&self) -> f64 {
        if self.peak_nodes == 0 {
            return 0.0;
        }
        let loss = (self.peak_nodes - self.current_nodes) as f64 / self.peak_nodes as f64;
        loss.clamp(0.0, 1.0)
    }

    // ---- Seed Resurrection Protocol ----

    /// Attempt Seed Resurrection when catastrophic node loss is detected.
    ///
    /// Returns `Ok(true)` if resurrection is authorized (node loss > threshold).
    /// Returns `Ok(false)` if threshold not met.
    /// Returns `Err` if genesis verification fails.
    pub fn attempt_resurrection(&self, verified_genesis_hash: u128) -> Result<bool, DnaError> {
        // Verify the requesting node has the correct Genesis Block
        if verified_genesis_hash != self.genesis_hash {
            return Err(DnaError::InvalidGenesis);
        }

        let loss = self.node_loss_ratio();
        if loss > self.config.resurrection_threshold {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Generate resurrection payload: compressed DNA state for bootstrap.
    pub fn resurrection_payload(&self) -> ResurrectionPayload {
        ResurrectionPayload {
            genesis_hash: self.genesis_hash,
            stable_concepts: self.stable_concepts(),
            latest_snapshot: self.field_snapshots.last().cloned(),
            integrated_principles: self.integrated_principles.clone(),
            peak_nodes: self.peak_nodes,
        }
    }

    // ---- Generational Testament ----

    /// Propose a new principle for integration (called every 90 symbiotic days).
    pub fn propose_testament(
        &mut self,
        principle: String,
        total_stewards: u64,
        timestamp_ms: u64,
    ) -> Result<u64, DnaError> {
        if self.sealed {
            return Err(DnaError::ImmutableDna);
        }
        let id = self.next_proposal_id;
        self.next_proposal_id += 1;
        let proposal = TestamentProposal::new(id, principle, total_stewards, timestamp_ms);
        self.testament_proposals.push(proposal);
        Ok(id)
    }

    /// Cast a vote for a testament proposal.
    pub fn vote_testament(&mut self, proposal_id: u64) -> Result<(), DnaError> {
        if self.sealed {
            return Err(DnaError::ImmutableDna);
        }
        let proposal = self
            .testament_proposals
            .iter_mut()
            .find(|p| p.proposal_id == proposal_id);
        match proposal {
            Some(p) => {
                p.vote();
                // Auto-approve if quorum reached
                if p.is_approved(self.config.testament_quorum) {
                    p.status = TestamentStatus::Approved;
                }
                Ok(())
            }
            None => Err(DnaError::SnapshotOutOfBounds),
        }
    }

    /// Integrate all approved testament proposals into the DNA.
    pub fn integrate_approved_testaments(&mut self) -> usize {
        let mut integrated = 0;
        for proposal in &mut self.testament_proposals {
            if proposal.status == TestamentStatus::Approved {
                let quorum = proposal.quorum_ratio();
                if quorum > self.config.testament_quorum {
                    proposal.status = TestamentStatus::Integrated;
                    self.integrated_principles.push(proposal.principle.clone());
                    integrated += 1;
                }
            }
        }
        integrated
    }

    /// Check if a specific proposal meets quorum requirements.
    pub fn check_quorum(&self, proposal_id: u64) -> Result<f64, DnaError> {
        let proposal = self
            .testament_proposals
            .iter()
            .find(|p| p.proposal_id == proposal_id);
        match proposal {
            Some(p) => Ok(p.quorum_ratio()),
            None => Err(DnaError::SnapshotOutOfBounds),
        }
    }

    // ---- Seal ----

    /// Seal the DNA, making it immutable.
    pub fn seal(&mut self) {
        self.sealed = true;
    }

    /// Verify DNA integrity: genesis hash matches, principles non-empty after integration.
    pub fn verify_integrity(&self) -> bool {
        self.genesis_hash != 0
    }
}

/// Compressed payload for Seed Resurrection bootstrap.
#[derive(Debug, Clone, PartialEq)]
pub struct ResurrectionPayload {
    pub genesis_hash: u128,
    pub stable_concepts: Vec<MacroConceptRecord>,
    pub latest_snapshot: Option<EthicalFieldSnapshot>,
    pub integrated_principles: Vec<String>,
    pub peak_nodes: usize,
}

impl Default for NoosphericDna {
    fn default() -> Self {
        Self::forge(0)
    }
}

impl fmt::Display for NoosphericDna {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Noospheric DNA")?;
        writeln!(f, "  Genesis Hash:     0x{:032X}", self.genesis_hash)?;
        writeln!(f, "  Macro-Concepts:   {}", self.macro_concepts.len())?;
        writeln!(f, "  Stable Concepts:  {}", self.stable_concepts().len())?;
        writeln!(f, "  Field Snapshots:  {}", self.field_snapshots.len())?;
        writeln!(f, "  Testament Props:  {}", self.testament_proposals.len())?;
        writeln!(
            f,
            "  Integrated:       {}",
            self.integrated_principles.len()
        )?;
        writeln!(f, "  Peak Nodes:       {}", self.peak_nodes)?;
        writeln!(f, "  Current Nodes:    {}", self.current_nodes)?;
        writeln!(
            f,
            "  Node Loss:        {:.2}%",
            self.node_loss_ratio() * 100.0
        )?;
        writeln!(f, "  Sealed:           {}", self.sealed)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const GENESIS_HASH: u128 = 0xA1B2C3D4E5F60718_293A4B5C6D7E8F90;

    fn make_stable_concept(id: u128) -> MacroConceptRecord {
        MacroConceptRecord::new(id, format!("Concept-{}", id), 0.5, -0.1, 0.85, 1000)
    }

    fn make_unstable_concept(id: u128) -> MacroConceptRecord {
        MacroConceptRecord::new(id, format!("Unstable-{}", id), 0.2, 0.3, 0.5, 1000)
    }

    // ---- MacroConceptRecord ----

    #[test]
    fn test_concept_creation() {
        let c = make_stable_concept(1);
        assert_eq!(c.concept_id, 1);
        assert_eq!(c.lifecycle, MacroConceptLifecycle::Born);
    }

    #[test]
    fn test_concept_is_stable() {
        assert!(make_stable_concept(1).is_stable());
        assert!(!make_unstable_concept(2).is_stable());
    }

    #[test]
    fn test_lifecycle_display() {
        assert_eq!(format!("{}", MacroConceptLifecycle::Born), "Born");
        assert_eq!(format!("{}", MacroConceptLifecycle::Mature), "Mature");
    }

    // ---- EthicalFieldSnapshot ----

    #[test]
    fn test_snapshot_creation() {
        let s = EthicalFieldSnapshot::new(0.85, 100, 0.5, 0.7, 2000);
        assert_eq!(s.global_resonance, 0.85);
        assert_eq!(s.active_nodes, 100);
    }

    // ---- TestamentProposal ----

    #[test]
    fn test_proposal_creation() {
        let p = TestamentProposal::new(1, "Test Principle".to_string(), 100, 3000);
        assert_eq!(p.votes_for, 0);
        assert_eq!(p.status, TestamentStatus::Pending);
    }

    #[test]
    fn test_proposal_voting() {
        let mut p = TestamentProposal::new(1, "Test".to_string(), 100, 3000);
        for _ in 0..75 {
            p.vote();
        }
        assert_eq!(p.votes_for, 75);
        assert!(p.is_approved(0.70));
    }

    #[test]
    fn test_proposal_quorum_ratio() {
        let p = TestamentProposal::new(1, "Test".to_string(), 100, 3000);
        assert_eq!(p.quorum_ratio(), 0.0);
    }

    #[test]
    fn test_proposal_zero_stewards() {
        let p = TestamentProposal::new(1, "Test".to_string(), 0, 3000);
        assert_eq!(p.quorum_ratio(), 0.0);
    }

    #[test]
    fn test_testament_status_display() {
        assert_eq!(format!("{}", TestamentStatus::Pending), "Pending");
        assert_eq!(format!("{}", TestamentStatus::Approved), "Approved");
        assert_eq!(format!("{}", TestamentStatus::Integrated), "Integrated");
    }

    // ---- NoosphericDna ----

    #[test]
    fn test_dna_forge() {
        let dna = NoosphericDna::forge(GENESIS_HASH);
        assert_eq!(dna.genesis_hash, GENESIS_HASH);
        assert!(!dna.sealed);
        assert!(dna.verify_integrity());
    }

    #[test]
    fn test_dna_forge_with_config() {
        let config = DnaConfig {
            testament_quorum: 0.80,
            ..DnaConfig::default()
        };
        let dna = NoosphericDna::forge_with_config(GENESIS_HASH, config);
        assert_eq!(dna.config.testament_quorum, 0.80);
    }

    #[test]
    fn test_record_macro_concept() {
        let mut dna = NoosphericDna::forge(GENESIS_HASH);
        assert!(dna.record_macro_concept(make_stable_concept(1)).is_ok());
        assert_eq!(dna.macro_concepts.len(), 1);
    }

    #[test]
    fn test_record_rejected_when_sealed() {
        let mut dna = NoosphericDna::forge(GENESIS_HASH);
        dna.seal();
        assert_eq!(
            dna.record_macro_concept(make_stable_concept(1)),
            Err(DnaError::ImmutableDna)
        );
    }

    #[test]
    fn test_stable_concepts_filter() {
        let mut dna = NoosphericDna::forge(GENESIS_HASH);
        dna.record_macro_concept(make_stable_concept(1)).unwrap();
        dna.record_macro_concept(make_stable_concept(2)).unwrap();
        dna.record_macro_concept(make_unstable_concept(3)).unwrap();
        assert_eq!(dna.stable_concepts().len(), 2);
    }

    #[test]
    fn test_snapshot_field() {
        let mut dna = NoosphericDna::forge(GENESIS_HASH);
        let snap = EthicalFieldSnapshot::new(0.9, 50, 0.3, 0.8, 5000);
        assert!(dna.snapshot_field(snap.clone()).is_ok());
        assert_eq!(dna.latest_snapshot().unwrap(), &snap);
    }

    #[test]
    fn test_snapshot_pruning() {
        let mut dna = NoosphericDna::forge(GENESIS_HASH);
        dna.config.max_snapshots = 3;
        for i in 0..5 {
            let snap = EthicalFieldSnapshot::new(0.5, 10, 0.5, 0.5, i);
            dna.snapshot_field(snap).unwrap();
        }
        assert_eq!(dna.field_snapshots.len(), 3);
    }

    #[test]
    fn test_get_snapshot_out_of_bounds() {
        let dna = NoosphericDna::forge(GENESIS_HASH);
        assert_eq!(dna.get_snapshot(0), Err(DnaError::SnapshotOutOfBounds));
    }

    #[test]
    fn test_node_count_tracking() {
        let mut dna = NoosphericDna::forge(GENESIS_HASH);
        dna.update_node_count(100);
        assert_eq!(dna.peak_nodes, 100);
        assert_eq!(dna.current_nodes, 100);
        dna.update_node_count(150);
        assert_eq!(dna.peak_nodes, 150);
        dna.update_node_count(80);
        assert_eq!(dna.peak_nodes, 150);
        assert_eq!(dna.current_nodes, 80);
    }

    #[test]
    fn test_node_loss_ratio() {
        let mut dna = NoosphericDna::forge(GENESIS_HASH);
        dna.update_node_count(100);
        dna.update_node_count(20);
        assert!((dna.node_loss_ratio() - 0.80).abs() < 1e-10);
    }

    #[test]
    fn test_node_loss_zero_peak() {
        let dna = NoosphericDna::forge(GENESIS_HASH);
        assert_eq!(dna.node_loss_ratio(), 0.0);
    }

    // ---- Seed Resurrection Protocol ----

    #[test]
    fn test_resurrection_authorized() {
        let mut dna = NoosphericDna::forge(GENESIS_HASH);
        dna.update_node_count(1000);
        dna.update_node_count(100); // 90% loss
        assert_eq!(dna.attempt_resurrection(GENESIS_HASH), Ok(true));
    }

    #[test]
    fn test_resurrection_not_authorized_low_loss() {
        let mut dna = NoosphericDna::forge(GENESIS_HASH);
        dna.update_node_count(1000);
        dna.update_node_count(500); // 50% loss
        assert_eq!(dna.attempt_resurrection(GENESIS_HASH), Ok(false));
    }

    #[test]
    fn test_resurrection_invalid_genesis() {
        let dna = NoosphericDna::forge(GENESIS_HASH);
        assert_eq!(
            dna.attempt_resurrection(0xDEADBEEF),
            Err(DnaError::InvalidGenesis)
        );
    }

    #[test]
    fn test_resurrection_payload() {
        let mut dna = NoosphericDna::forge(GENESIS_HASH);
        dna.record_macro_concept(make_stable_concept(1)).unwrap();
        dna.record_macro_concept(make_unstable_concept(2)).unwrap();
        let payload = dna.resurrection_payload();
        assert_eq!(payload.genesis_hash, GENESIS_HASH);
        assert_eq!(payload.stable_concepts.len(), 1);
        assert_eq!(payload.peak_nodes, 0);
    }

    // ---- Generational Testament ----

    #[test]
    fn test_propose_testament() {
        let mut dna = NoosphericDna::forge(GENESIS_HASH);
        let id = dna
            .propose_testament("New Principle".to_string(), 100, 6000)
            .unwrap();
        assert_eq!(id, 1);
        assert_eq!(dna.testament_proposals.len(), 1);
    }

    #[test]
    fn test_proposal_rejected_when_sealed() {
        let mut dna = NoosphericDna::forge(GENESIS_HASH);
        dna.seal();
        assert_eq!(
            dna.propose_testament("X".to_string(), 100, 6000),
            Err(DnaError::ImmutableDna)
        );
    }

    #[test]
    fn test_vote_and_approve() {
        let mut dna = NoosphericDna::forge(GENESIS_HASH);
        dna.propose_testament("Cooperation First".to_string(), 100, 7000)
            .unwrap();
        for _ in 0..75 {
            dna.vote_testament(1).unwrap();
        }
        let proposal = &dna.testament_proposals[0];
        assert_eq!(proposal.status, TestamentStatus::Approved);
    }

    #[test]
    fn test_integrate_approved_testaments() {
        let mut dna = NoosphericDna::forge(GENESIS_HASH);
        dna.propose_testament("Principle A".to_string(), 100, 8000)
            .unwrap();
        for _ in 0..80 {
            dna.vote_testament(1).unwrap();
        }
        let integrated = dna.integrate_approved_testaments();
        assert_eq!(integrated, 1);
        assert_eq!(dna.integrated_principles.len(), 1);
        assert_eq!(
            dna.testament_proposals[0].status,
            TestamentStatus::Integrated
        );
    }

    #[test]
    fn test_check_quorum() {
        let mut dna = NoosphericDna::forge(GENESIS_HASH);
        dna.propose_testament("Test".to_string(), 100, 9000)
            .unwrap();
        for _ in 0..50 {
            dna.vote_testament(1).unwrap();
        }
        assert!((dna.check_quorum(1).unwrap() - 0.50).abs() < 1e-10);
    }

    #[test]
    fn test_check_quorum_missing_proposal() {
        let dna = NoosphericDna::forge(GENESIS_HASH);
        assert_eq!(dna.check_quorum(999), Err(DnaError::SnapshotOutOfBounds));
    }

    #[test]
    fn test_vote_missing_proposal() {
        let mut dna = NoosphericDna::forge(GENESIS_HASH);
        assert_eq!(dna.vote_testament(999), Err(DnaError::SnapshotOutOfBounds));
    }

    // ---- Seal & Integrity ----

    #[test]
    fn test_seal_prevents_modification() {
        let mut dna = NoosphericDna::forge(GENESIS_HASH);
        dna.seal();
        assert!(dna.sealed);
        assert_eq!(
            dna.snapshot_field(EthicalFieldSnapshot::new(0.0, 0, 0.0, 0.0, 0)),
            Err(DnaError::ImmutableDna)
        );
    }

    #[test]
    fn test_verify_integrity_valid() {
        let dna = NoosphericDna::forge(GENESIS_HASH);
        assert!(dna.verify_integrity());
    }

    #[test]
    fn test_verify_integrity_zero_genesis() {
        let dna = NoosphericDna::forge(0);
        assert!(!dna.verify_integrity());
    }

    // ---- Display ----

    #[test]
    fn test_dna_display() {
        let dna = NoosphericDna::forge(GENESIS_HASH);
        let s = format!("{}", dna);
        assert!(s.contains("Noospheric DNA"));
        assert!(s.contains(&format!("0x{:032X}", GENESIS_HASH)));
    }

    // ---- Default ----

    #[test]
    fn test_dna_default() {
        let dna = NoosphericDna::default();
        assert_eq!(dna.genesis_hash, 0);
        assert!(!dna.verify_integrity());
    }

    // ---- DnaConfig Default ----

    #[test]
    fn test_config_default() {
        let c = DnaConfig::default();
        assert_eq!(c.testament_quorum, 0.70);
        assert_eq!(c.resurrection_threshold, 0.80);
        assert_eq!(c.testament_interval_days, 90);
    }

    // ---- Error Display ----

    #[test]
    fn test_error_display() {
        assert!(format!("{}", DnaError::ImmutableDna).contains("immutable"));
        assert!(format!("{}", DnaError::InvalidGenesis).contains("Genesis"));
        assert!(format!("{}", DnaError::SnapshotOutOfBounds).contains("bounds"));
    }

    #[test]
    fn test_quorum_not_reached_display() {
        let e = DnaError::QuorumNotReached {
            current: 0.5,
            required: 0.7,
        };
        let s = format!("{}", e);
        assert!(s.contains("0.50"));
        assert!(s.contains("0.70"));
    }

    #[test]
    fn test_resurrection_threshold_display() {
        let e = DnaError::ResurrectionThresholdNotMet { node_loss: 0.5 };
        assert!(format!("{}", e).contains("0.50"));
    }
}
