//! Intelligence Module — Autonomous Emergence Engine (Sprint 53)
//!
//! Provides the Stuartian Emergence Engine for autonomous discovery of
//! emergent capabilities through Cross-Tensor Fusion.
//!
//! **Feature Gate:** `v3.5-planetary-emergence`

#[cfg(feature = "v3.5-planetary-emergence")]
mod emergence_core;

#[cfg(feature = "v3.5-planetary-emergence")]
pub use emergence_core::*;
