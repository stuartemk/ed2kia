//! Async Gossip with CRDTs â€” GossipSub asÃ­ncrono con tolerancia a particiones.
//!
//! **Topological Law 5 (MÃºltiples Posibilidades):** Async, tolerancia a particiones,
//! CRDTs para convergencia eventual sin coordinaciÃ³n centralizada.
//!
//! ### Feature Gates
//! | Feature | MÃ³dulo | DescripciÃ³n |
//! |---|---|---|
//! | `v2.1-async-gossip` | mesh | GossipSub async config |
//! | `v2.1-offline-cache` | cache | redb offline storage |
//! | `v2.1-crdt-state` | crdt | GCounter, PNCounter, ORSet |

#[cfg(feature = "v2.1-async-gossip")]
pub mod mesh;

#[cfg(feature = "v2.1-offline-cache")]
pub mod cache;

#[cfg(feature = "v2.1-crdt-state")]
pub mod crdt;

#[cfg(feature = "v2.1-async-gossip")]
pub use mesh::{GossipMesh, GossipMeshError, MeshConfig, MeshMessage, PeerInfo, PeerState};

#[cfg(feature = "v2.1-offline-cache")]
pub use cache::{GossipCache, GossipCacheError, CacheEntry, PayloadType, SyncStatus, CacheStats};

#[cfg(feature = "v2.1-crdt-state")]
pub use crdt::{GCounter, PNCounter, ORSet, ReputationCrdt, VersionVector, CrdtError};

// â”€â”€â”€ Sprint28: Symbol Registry CRDT â”€â”€â”€
#[cfg(feature = "v2.1-crdt-symbols")]
pub mod crdt_symbols;

#[cfg(feature = "v2.1-crdt-symbols")]
pub use crdt_symbols::{SymbolRegistry, SymbolRegistryError, SymbolEntry};
