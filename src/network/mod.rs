//! Network — Cross-mesh routing and multi-region synchronization.
//!
//! **Stuartian Law 1 (Diversidad):** Peering orgánico entre mallas, sin coordinación centralizada.
//! **Stuartian Law 5 (Múltiples Posibilidades):** Tolerancia a particiones, convergencia eventual.
//!
//! ### Feature Gates
//! | Feature | Módulo | Descripción |
//! |---|---|---|
//! | `v2.1-cross-mesh` | cross_mesh | Cross-mesh routing, peering, rate limiting |
//! | `v2.1-region-sync` | region_sync | Multi-region sync, delta-encoding, batch merge |

#[cfg(feature = "v2.1-cross-mesh")]
pub mod cross_mesh;

#[cfg(feature = "v2.1-region-sync")]
pub mod region_sync;

#[cfg(feature = "v2.1-cross-mesh")]
pub use cross_mesh::{
    CrossMeshError, CrossMeshRouter, PeerLink, RelayPayload, RouteEntry, RouterStats,
    MAX_PAYLOAD_SIZE,
};

#[cfg(feature = "v2.1-region-sync")]
pub use region_sync::{
    apply_deltas, generate_deltas, resolve_conflicts, sync_region_state, DeltaEntry, RegionState,
    SyncConfig, SyncError, SyncResult,
};

#[cfg(feature = "v3.5-planetary-emergence")]
pub mod planetary_mesh;

#[cfg(feature = "v3.5-planetary-emergence")]
pub use planetary_mesh::{
    AutoNatEngine, AutoNatStatus, BucketAction, CircuitState, kademlia_distance, KTable,
    MeshConfig, MeshError, MeshStats, NodeCapabilities as MeshNodeCapabilities, PeerEntry,
    PlanetaryMesh, RelayCircuit,
};
