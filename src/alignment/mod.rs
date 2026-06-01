//! Alignment Module — Sprint 70: Civilization-Scale Architecture
//!
//! Symbolic+Geometric Alignment via Lean4/Isabelle proof generation
//! and Moral Manifold as Lyapunov attractor basin.

pub mod moral_attractor;
pub mod proof_generator;

// ─── Sprint72: Topology-Ethics Mapping (GEI as structural stability proxy) ───
#[cfg(feature = "v9.8-asymptotic-hardening")]
pub mod topology_ethics_mapping;

pub use moral_attractor::{AttractorConfig, AttractorError, MoralAttractor, MoralState};
pub use proof_generator::{ProofConfig, ProofError, ProofGenerator, ProofRecord};
#[cfg(feature = "v9.8-asymptotic-hardening")]
pub use topology_ethics_mapping::{
    MappingConfig, MappingError, TopoSnapshot, TopologyEthicsMapper,
};
