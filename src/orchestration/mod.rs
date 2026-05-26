//! Cross-Pillar Orchestration Layer — Sprint 41
//!
//! Provides the `PillarOrchestrator` struct and routing logic for distributing
//! requests across the 4 Evolutionary Pillars (Corpuscular, Maieutic, Steganographic, Resonance).
//!
//! **Architecture Principles:**
//! - Symbiotic integration between pillars and ed2kIA core (P2P, SCT, CE Ledger, CRDTs).
//! - Ed25519 signature validation on all pillar payloads.
//! - CE > 0 and Z > 0 (SCT) requirements for request authorization.
//! - Zero telemetry for biometric data (Resonance pillar enforces LOCAL_ONLY via WASM).
//!
//! **Feature Gate:** `v3.0-orchestration`

mod pillar_router;
mod omni_node;
#[cfg(feature = "v3.2-genesis-manifold")]
mod symbiotic_loop;
#[cfg(feature = "v3.5-planetary-emergence")]
mod swarm_topology;
#[cfg(feature = "v3.6-aegis-resonance")]
mod aegis_healer;

pub use pillar_router::*;
pub use omni_node::*;
#[cfg(feature = "v3.2-genesis-manifold")]
pub use symbiotic_loop::*;
#[cfg(feature = "v3.5-planetary-emergence")]
pub use swarm_topology::*;
#[cfg(feature = "v3.6-aegis-resonance")]
pub use aegis_healer::{AegisConfig, AegisError, AegisHealer, HealingAction, HealingResult, AegisSymbioticState};

// Re-export PillarMessage for backward compatibility with E2E tests
pub use crate::runtime::pillar_messaging::PillarMessage;
