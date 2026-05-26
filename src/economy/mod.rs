//! Global Symbiotic Economy — DAG-based Ledger for Existence Credit (CE).
//!
//! Provides a DAG (Directed Acyclic Graph) ledger for cooperative tracking
//! of CE transactions with Ed25519 validation and SCT Guard Economic rejection.
//!
//! ### Feature Gates
//! | Feature | Módulo | Descripción |
//! |---|---|---|
//! | `v3.4-macro-symbiosis` | symbiotic_ledger | GlobalSymbioticLedger — DAG-based CE Ledger |
//! | `v3.8-morphic-genesis` | genesis_graph | GenesisNode — DAG root with Stuartian Laws hash, zero pre-mine |

pub mod symbiotic_ledger;

#[cfg(feature = "v3.8-morphic-genesis")]
pub mod genesis_graph;
