//! Real-Time Frontier Model Auditing — Sprint 70: Civilization-Scale Architecture
//!
//! Activation hooking, ZKP verification, and deception detection
//! for frontier model auditing at civilization scale.

pub mod frontier_hook;
pub mod zkp_verification;

pub use frontier_hook::{ActivationHook, HookConfig, HookError};
pub use zkp_verification::{ProofRecord, VerificationError, ZkpVerifier};
