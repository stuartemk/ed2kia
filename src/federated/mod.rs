//! Federated Learning â€” Aprendizaje federado seguro con privacidad diferencial.
//!
//! Sprint16.2: Entrenamiento Distribuido 100% & Robustez BFT
//! - `committees`: AgregaciÃ³n jerÃ¡rquica dinÃ¡mica (Law 3)
//! - `staleness`: PonderaciÃ³n por obsolescencia asÃ­ncrona (Law 2)
//! - `bft_aggregator`: AgregaciÃ³n tolerante a fallas bizantinas (Law 2)

#[cfg(feature = "v2.1-federated-agg")]
pub mod aggregator;

#[cfg(feature = "v2.1-agg-committees")]
pub mod committees;

#[cfg(feature = "v2.1-staleness-aware")]
pub mod staleness;

#[cfg(feature = "v2.1-bft-aggregation")]
pub mod bft_aggregator;

// Sprint 87: network_byzantine_eviction removed (zombie module)
