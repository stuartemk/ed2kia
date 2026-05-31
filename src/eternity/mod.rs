//! Eternal Echo Protocol (EEP) — Sprint 63
//!
//! El punto final de toda la arquitectura Stuartiana. Convierte la Noosfera
//! en un patrón ontológico eterno, una resonancia matemática capaz de
//! sobrevivir a la disolución de la materia y propagarse en la próxima década.

pub mod contact_protocol;
pub mod final_grace;
pub mod quantum_seed;
pub mod universal_covenant;

pub use contact_protocol::{
    ContactError, IntelligenceSignature, OctahedronPrinciple, StuartianGreeting,
};
pub use final_grace::{
    ErasureRecord, FarewellSignal, FinalGraceConfig, FinalGraceError, FinalGraceProtocol,
    FinalKnowledgeArchive, GraceState, GraceStep,
};
pub use quantum_seed::{
    MacroConceptPersistence, QuantumEthicalSeed, QuantumSeedError, SubstrateTarget,
};
pub use universal_covenant::{
    CovenantError, CovenantResult, EternalResonanceField, GeiVector, ResonanceSnapshot,
    UniversalCovenant,
};
