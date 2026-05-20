//! Async Gossip with CRDTs — GossipSub asíncrono con tolerancia a particiones.
//!
//! **Stuartian Law 5 (Múltiples Posibilidades):** Async, tolerancia a particiones,
//! CRDTs para convergencia eventual sin coordinación centralizada.
//!
//! **Feature Gate:** `v2.1-async-gossip-crdt`

pub mod cache;
pub mod crdt;
pub mod mesh;

pub use cache::{GossipCache, GossipCacheError};
pub use crdt::{ReputationCrdt, CrdtError};
pub use mesh::{GossipMesh, GossipMeshError};
