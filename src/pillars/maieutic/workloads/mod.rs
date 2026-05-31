//! Distributed Workloads for Maieutic Synthesizer.
//!
//! This module contains distributed computational workloads that can be executed
//! across WASM-compatible nodes with BFT consensus validation.

pub mod telomere_genesis;

pub use telomere_genesis::{
    DistributedWorkload, EpigeneticNoiseModel, SyntaxCorrection, TelomereRegenerationTask,
    WorkloadContext, WorkloadCost, WorkloadError, WorkloadResult,
};
