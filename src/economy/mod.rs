//! Global Symbiotic Economy â€” DAG-based Ledger for Existence Credit (CE).
//!
//! Provides a DAG (Directed Acyclic Graph) ledger for cooperative tracking
//! of CE transactions with Ed25519 validation and SCT Guard Economic rejection.
//!
//! ### Feature Gates
//! | Feature | MÃ³dulo | DescripciÃ³n |
//! |---|---|---|
//! | `v3.4-macro-symbiosis` | symbiotic_ledger | GlobalSymbioticLedger â€” DAG-based CE Ledger |
//! | `v3.8-morphic-genesis` | genesis_graph | GenesisNode â€” DAG root with Topological Laws hash, zero pre-mine |

pub mod symbiotic_ledger;

#[cfg(feature = "v3.8-morphic-genesis")]
pub mod genesis_graph;

#[cfg(feature = "v5.0-mainnet-genesis")]
pub mod mainnet_genesis;
