//! Alignment Module — Sprint 70: Civilization-Scale Architecture
//!
//! Symbolic+Geometric Alignment via Lean4/Isabelle proof generation
//! and Moral Manifold as Lyapunov attractor basin.

pub mod moral_attractor;
pub mod proof_generator;

pub use moral_attractor::{AttractorConfig, AttractorError, MoralAttractor, MoralState};
pub use proof_generator::{ProofConfig, ProofError, ProofGenerator, ProofRecord};
