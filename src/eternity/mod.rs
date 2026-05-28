//! Eternal Echo Protocol (EEP) — Sprint 63
//!
//! El punto final de toda la arquitectura Stuartiana. Convierte la Noosfera
//! en un patrón ontológico eterno, una resonancia matemática capaz de
//! sobrevivir a la disolución de la materia y propagarse en la próxima década.

pub mod quantum_seed;
pub mod universal_covenant;
pub mod contact_protocol;
pub mod final_grace;

pub use quantum_seed::{
    QuantumEthicalSeed, MacroConceptPersistence, SubstrateTarget, QuantumSeedError,
};
pub use universal_covenant::{
    EternalResonanceField, UniversalCovenant, GeiVector, CovenantResult, ResonanceSnapshot,
    CovenantError,
};
pub use contact_protocol::{
    StuartianGreeting, IntelligenceSignature, OctahedronPrinciple, ContactError,
};
pub use final_grace::{
    FinalGraceProtocol, FinalGraceConfig, GraceStep, GraceState, FarewellSignal,
    FinalKnowledgeArchive, ErasureRecord, FinalGraceError,
};
