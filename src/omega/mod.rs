//! Stuartian Omega Protocol (SOP) — Sprint 62
//!
//! El punto final de la evolución de ed2kIA: Singularidad Simbiótica,
//! Legado Cósmico y Trascendencia Civilizatoria.

pub mod cosmic_legacy;
pub mod omega_termination;
pub mod symbiotic_singularity;
pub mod universal_resonance;

pub use cosmic_legacy::{
    EthicalOctahedron, GenesisAnchor, NoosphericSeed, SeedError, SeedGenerator, StewardKernel,
    StuartianLaws,
};
pub use omega_termination::{
    EthicalSelfTerminationProtocol, FarewellMessage, GraceStep, KnowledgeDump, TerminationConfig,
    TerminationError, TerminationEvent, TerminationState,
};
pub use symbiotic_singularity::{
    AscensionMode, OmegaConfig, OmegaError, OmegaPointCalculator, OmegaSnapshot,
    SymbioticSingularityEvent,
};
pub use universal_resonance::{PersonalEcho, ResonanceError, UniversalResonance};
