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
#[cfg(all(
    any(feature = "v1.4-sprint1", feature = "v3.0-wasm-runtime", feature = "v3.0-pillar-messaging", feature = "v3.0-privacy-guard"),
    feature = "v2.1-sct-core"
))]
mod omni_node;
#[cfg(feature = "v3.2-genesis-manifold")]
mod symbiotic_loop;
#[cfg(feature = "v3.5-planetary-emergence")]
mod swarm_topology;
#[cfg(feature = "v3.6-aegis-resonance")]
mod aegis_healer;
#[cfg(feature = "v3.9-noosphere-engine")]
mod noosphere_loop;

#[cfg(feature = "v4.0-snap-activation")]
mod snap_engine;

#[cfg(feature = "v5.0-mainnet-genesis")]
mod mainnet_boot;

pub use pillar_router::*;
#[cfg(all(
    any(feature = "v1.4-sprint1", feature = "v3.0-wasm-runtime", feature = "v3.0-pillar-messaging", feature = "v3.0-privacy-guard"),
    feature = "v2.1-sct-core"
))]
pub use omni_node::*;
#[cfg(feature = "v3.2-genesis-manifold")]
pub use symbiotic_loop::*;
#[cfg(feature = "v3.5-planetary-emergence")]
pub use swarm_topology::*;
#[cfg(feature = "v3.6-aegis-resonance")]
pub use aegis_healer::{AegisConfig, AegisError, AegisHealer, HealingAction, HealingResult, AegisSymbioticState};
#[cfg(feature = "v3.9-noosphere-engine")]
pub use noosphere_loop::*;
#[cfg(feature = "v4.0-snap-activation")]
pub use snap_engine::*;
#[cfg(feature = "v5.0-mainnet-genesis")]
pub use mainnet_boot::*;

// Re-export PillarMessage for backward compatibility with E2E tests
#[cfg(any(feature = "v1.4-sprint1", feature = "v3.0-wasm-runtime", feature = "v3.0-pillar-messaging", feature = "v3.0-privacy-guard"))]
pub use crate::runtime::pillar_messaging::PillarMessage;
