//! Federated Learning — Aprendizaje federado seguro con privacidad diferencial.
//!
//! Sprint16.2: Entrenamiento Distribuido 100% & Robustez BFT
//! - `committees`: Agregación jerárquica dinámica (Law 3)
//! - `staleness`: Ponderación por obsolescencia asíncrona (Law 2)
//! - `bft_aggregator`: Agregación tolerante a fallas bizantinas (Law 2)

#[cfg(feature = "v2.1-federated-agg")]
pub mod aggregator;

#[cfg(feature = "v2.1-agg-committees")]
pub mod committees;

#[cfg(feature = "v2.1-staleness-aware")]
pub mod staleness;

#[cfg(feature = "v2.1-bft-aggregation")]
pub mod bft_aggregator;
