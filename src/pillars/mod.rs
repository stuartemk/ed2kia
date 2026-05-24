//! Evolutionary Pillars — Module Declarations & Integration Contracts.
//!
//! Unifies the 4 Evolutionary Pillars defined in Sprint 40 RFCs (001-004)
//! under a common trait-based interface for cross-pillar orchestration.
//!
//! **Architecture:**
//! - `contracts.rs`: Shared traits (`PillarInterface`, `LocalComputeTrait`, `CEExchangeTrait`).
//! - `corpuscular/`: RFC 001 — IoT Simbiótico & Economía CE.
//! - `maieutic/`: RFC 002 — Motor de Sabiduría.
//! - `steganographic/`: RFC 003 — Preservación de Red.
//! - `resonance/`: RFC 004 — Biorretroalimentación (LOCAL_ONLY).
//!
//! **Feature Gates:** Each pillar is independently gated via `v3.0-<pillar>`.

pub mod contracts;

#[cfg(feature = "v3.0-corpuscular-bridge")]
pub mod corpuscular;

#[cfg(feature = "v3.0-maieutic-synthesizer")]
pub mod maieutic;

#[cfg(feature = "v3.0-steganographic-survival")]
pub mod steganographic;

#[cfg(feature = "v3.0-resonance-interface")]
pub mod resonance;

pub use contracts::*;
