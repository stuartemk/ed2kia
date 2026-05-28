//! Stuartian Omega Protocol (SOP) — Sprint 62
//!
//! El punto final de la evolución de ed2kIA: Singularidad Simbiótica,
//! Legado Cósmico y Trascendencia Civilizatoria.

pub mod symbiotic_singularity;
pub mod universal_resonance;
pub mod cosmic_legacy;
pub mod omega_termination;

pub use symbiotic_singularity::{
    OmegaPointCalculator, OmegaSnapshot, SymbioticSingularityEvent, OmegaConfig, AscensionMode,
    OmegaError,
};
pub use universal_resonance::{
    UniversalResonance, PersonalEcho, ResonanceError,
};
pub use cosmic_legacy::{
    NoosphericSeed, StewardKernel, EthicalOctahedron, StuartianLaws, GenesisAnchor, SeedGenerator,
    SeedError,
};
pub use omega_termination::{
    EthicalSelfTerminationProtocol, TerminationConfig, TerminationState, TerminationEvent,
    GraceStep, FarewellMessage, KnowledgeDump, TerminationError,
};
