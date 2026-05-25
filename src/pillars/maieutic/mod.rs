//! Maieutic Synthesizer — Motor de Sabiduría.
//!
//! **RFC 002:** Evolves ed2kIA from knowledge audit to distributed scientific creation.
//! Orchestrates a 4-phase pipeline: Scientific Decomposition → P2P Distribution →
//! BFT Aggregation → Maieutic Synthesis.
//!
//! **Simulation Modules (WASM-Compatible):**
//! - Molecular Dynamics (Verlet integration + CHARMM36 force field).
//! - Protein Folding (AlphaFold-lite inference).
//! - Epigenetics (methylation analysis + DESeq2-like differential expression).
//!
//! **Validation:**
//! - BFT consensus for hypothesis validation.
//! - SCT ethical evaluation (Z >= 0 required).
//! - Cross-domain synthesis for emergent scientific insights.
//!
//! **Feature Gate:** `v3.0-maieutic-synthesizer`
//!
//! **Reference:** Sprint 44 — Maieutic Synthesizer Implementation (Pillar 2)

pub mod hypothesis_engine;
pub mod bio_sim_worker;
pub mod scientific_consensus;
#[cfg(feature = "v3.2-genesis-manifold")]
pub mod workloads;

use crate::orchestration::PillarId;
use crate::pillars::{PillarError, PillarInterface};

#[cfg(feature = "v3.0-maieutic-synthesizer")]
use hypothesis_engine::HypothesisEngine;

#[cfg(feature = "v3.0-maieutic-synthesizer")]
use scientific_consensus::ScientificConsensus;

/// Maieutic Synthesizer Engine — Distributed scientific creation coordinator.
///
/// Manages the lifecycle of scientific hypotheses from decomposition through
/// BFT-validated synthesis. Integrates with ed2kIA's CRDT layer for evidence
/// aggregation and SCT for ethical evaluation.
///
/// **Expected Flow:**
/// 1. Problem statement decomposed into computable sub-problems.
/// 2. Sub-problems distributed via libp2p streams to participating nodes.
/// 3. Evidence aggregated with BFT consensus (f < n/3 tolerance).
/// 4. Maieutic synthesis generates cross-domain hypotheses.
/// 5. SCT evaluation ensures Z >= 0 (constructive science).
pub struct MaieuticEngine {
    #[cfg(feature = "v3.0-maieutic-synthesizer")]
    hypothesis_engine: HypothesisEngine,

    #[cfg(feature = "v3.0-maieutic-synthesizer")]
    consensus: ScientificConsensus,
}

impl MaieuticEngine {
    /// Create a new Maieutic Synthesizer Engine.
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "v3.0-maieutic-synthesizer")]
            hypothesis_engine: HypothesisEngine::new(),

            #[cfg(feature = "v3.0-maieutic-synthesizer")]
            consensus: ScientificConsensus::new(),
        }
    }

    /// Generate a new scientific hypothesis with SCT Guard validation.
    ///
    /// **SCT Guard:** Rejects hypotheses with Z < 0 immediately.
    #[cfg(feature = "v3.0-maieutic-synthesizer")]
    pub fn generate_hypothesis(
        &mut self,
        id: String,
        domain: hypothesis_engine::Domain,
        statement: String,
        z_score: f32,
    ) -> Result<hypothesis_engine::Hypothesis, hypothesis_engine::HypothesisError> {
        self.hypothesis_engine
            .generate_hypothesis(id, domain, statement, z_score)
    }

    /// Register a validator node for BFT consensus.
    #[cfg(feature = "v3.0-maieutic-synthesizer")]
    pub fn register_validator(&mut self, id: String) {
        self.consensus.register_validator(id);
    }

    /// Submit evidence for a hypothesis.
    #[cfg(feature = "v3.0-maieutic-synthesizer")]
    pub fn submit_evidence(
        &mut self,
        hypothesis_id: &str,
        evidence: hypothesis_engine::Evidence,
    ) -> Result<(), String> {
        // Submit to consensus engine first.
        self.consensus.submit_evidence(hypothesis_id, evidence.clone())
            .map_err(|e| format!("Consensus error: {}", e))?;
        // Then to hypothesis engine.
        self.hypothesis_engine
            .submit_evidence(hypothesis_id, evidence)
            .map_err(|e| format!("Hypothesis error: {}", e))?;
        Ok(())
    }

    /// Run BFT consensus for a hypothesis.
    #[cfg(feature = "v3.0-maieutic-synthesizer")]
    pub fn run_consensus(
        &self,
        hypothesis_id: &str,
        domain: &hypothesis_engine::Domain,
    ) -> Result<scientific_consensus::ConsensusResult, scientific_consensus::ConsensusError> {
        self.consensus.run_consensus(hypothesis_id, domain)
    }

    /// Get a hypothesis by ID.
    #[cfg(feature = "v3.0-maieutic-synthesizer")]
    pub fn get_hypothesis(
        &self,
        id: &str,
    ) -> Result<hypothesis_engine::Hypothesis, hypothesis_engine::HypothesisError> {
        self.hypothesis_engine.get_hypothesis(id)
    }

    /// List all hypotheses ready for consensus.
    #[cfg(feature = "v3.0-maieutic-synthesizer")]
    pub fn ready_for_consensus(&self) -> Vec<hypothesis_engine::Hypothesis> {
        self.hypothesis_engine.ready_for_consensus()
    }
}

impl PillarInterface for MaieuticEngine {
    fn id() -> PillarId {
        PillarId::MaieuticSynthesizer
    }

    fn validate_local_constraint(&self) -> bool {
        // Maieutic Synthesizer distributes computation via P2P — no LOCAL_ONLY constraint.
        // Scientific evidence is public knowledge, not sensitive personal data.
        true
    }

    fn consume_ce(&self, amount: f64) -> Result<(), PillarError> {
        if amount <= 0.0 {
            return Err(PillarError::InsufficientCE);
        }
        // CE consumed for compute credit allocation in scientific simulation.
        // Wire ExistentialCreditLedger for actual CE deduction.
        Ok(())
    }
}

impl Default for MaieuticEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let _engine = MaieuticEngine::new();
    }

    #[test]
    fn test_pillar_id() {
        assert_eq!(MaieuticEngine::id(), PillarId::MaieuticSynthesizer);
    }

    #[test]
    fn test_local_constraint() {
        let engine = MaieuticEngine::new();
        assert!(engine.validate_local_constraint());
    }

    #[test]
    fn test_consume_ce_valid() {
        let engine = MaieuticEngine::new();
        assert!(engine.consume_ce(1.0).is_ok());
    }

    #[test]
    fn test_consume_ce_zero_rejected() {
        let engine = MaieuticEngine::new();
        match engine.consume_ce(0.0) {
            Err(PillarError::InsufficientCE) => {}
            _ => panic!("Expected InsufficientCE"),
        }
    }

    #[test]
    fn test_consume_ce_negative_rejected() {
        let engine = MaieuticEngine::new();
        match engine.consume_ce(-1.0) {
            Err(PillarError::InsufficientCE) => {}
            _ => panic!("Expected InsufficientCE"),
        }
    }

    #[test]
    fn test_default() {
        let engine = MaieuticEngine::default();
        assert!(engine.validate_local_constraint());
    }

    #[cfg(feature = "v3.0-maieutic-synthesizer")]
    #[test]
    fn test_generate_hypothesis_with_sct_guard() {
        let mut engine = MaieuticEngine::new();
        let result = engine.generate_hypothesis(
            "h1".to_string(),
            hypothesis_engine::Domain::ProteinFolding,
            "Test hypothesis".to_string(),
            0.5,
        );
        assert!(result.is_ok());
    }

    #[cfg(feature = "v3.0-maieutic-synthesizer")]
    #[test]
    fn test_sct_guard_rejects_negative_z() {
        let mut engine = MaieuticEngine::new();
        let result = engine.generate_hypothesis(
            "h1".to_string(),
            hypothesis_engine::Domain::ProteinFolding,
            "Destructive".to_string(),
            -0.5,
        );
        assert!(result.is_err());
    }

    #[cfg(feature = "v3.0-maieutic-synthesizer")]
    #[test]
    fn test_full_consensus_flow() {
        let mut engine = MaieuticEngine::new();

        // Generate hypothesis.
        engine.generate_hypothesis(
            "h1".to_string(),
            hypothesis_engine::Domain::Epigenetics,
            "Methylation affects gene X".to_string(),
            0.5,
        )
        .unwrap();

        // Register validators.
        engine.register_validator("v1".to_string());
        engine.register_validator("v2".to_string());
        engine.register_validator("v3".to_string());

        // Submit evidence.
        engine.submit_evidence(
            "h1",
            hypothesis_engine::Evidence {
                source_node: "v1".to_string(),
                domain: hypothesis_engine::Domain::Epigenetics,
                payload: b"e1".to_vec(),
                z_score: 0.5,
                timestamp_ms: 1000,
            },
        )
        .unwrap();

        engine.submit_evidence(
            "h1",
            hypothesis_engine::Evidence {
                source_node: "v2".to_string(),
                domain: hypothesis_engine::Domain::Epigenetics,
                payload: b"e2".to_vec(),
                z_score: 0.3,
                timestamp_ms: 1001,
            },
        )
        .unwrap();

        engine.submit_evidence(
            "h1",
            hypothesis_engine::Evidence {
                source_node: "v3".to_string(),
                domain: hypothesis_engine::Domain::Epigenetics,
                payload: b"e3".to_vec(),
                z_score: 0.4,
                timestamp_ms: 1002,
            },
        )
        .unwrap();

        // Run consensus.
        let result = engine.run_consensus("h1", &hypothesis_engine::Domain::Epigenetics).unwrap();
        assert!(result.is_validated());
    }
}
