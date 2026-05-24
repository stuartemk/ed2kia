//! Runtime Module — Secure Execution Environment for ed2kIA v3.0.
//!
//! Provides the foundational runtime infrastructure for sandboxed pillar execution:
//! - WASM sandbox for isolated module execution
//! - Secure messaging with Ed25519 signatures and replay protection
//! - Privacy enforcer for LOCAL_ONLY constraint enforcement
//!
//! **Feature Gates:**
//! - `v3.0-wasm-runtime` — WASM execution sandbox
//! - `v3.0-pillar-messaging` — Secure pillar communication
//! - `v3.0-privacy-guard` — Privacy enforcement layer

#[cfg(feature = "v3.0-wasm-runtime")]
#[path = "wasm_sandbox.rs"]
pub mod wasm_sandbox;

#[cfg(feature = "v3.0-pillar-messaging")]
#[path = "pillar_messaging.rs"]
pub mod pillar_messaging;

#[cfg(feature = "v3.0-privacy-guard")]
#[path = "privacy_enforcer.rs"]
pub mod privacy_enforcer;
