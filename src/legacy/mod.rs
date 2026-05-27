//! Stuartian Legacy Protocol (SLP) — Sprint 61
//!
//! Infraestructura ética viva de la humanidad. Este módulo contiene los tres
//! pilares del protocolo de legado estuardiano:
//!
//! - **NoosphericDna** — Memoria colectiva inmortal anclada al Genesis Block
//! - **NCI Calculator** — Índice de Civilización Noosférica con Amplificación Simbiótica
//! - **Handover Protocol** — Safeguards irrevocables y transición a Propiedad Común

pub mod civilization_index;
pub mod handover_protocol;
pub mod noospheric_dna;

pub use civilization_index::{ASymConfig, MaturityTracker, NciCalculator, NciError, NciSnapshot, NciWeights};
pub use handover_protocol::{HandoverError, HandoverProtocol, HandoverState, LegacySafeguards, MaturityDeclarationEvent, OverrideProposal, ProposalState};
pub use noospheric_dna::{DnaConfig, EthicalFieldSnapshot, MacroConceptRecord, NoosphericDna, ResurrectionPayload, TestamentProposal};
