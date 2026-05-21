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
    CrossMeshRouter, CrossMeshError, RelayPayload, PeerLink, RouteEntry, RouterStats,
    MAX_PAYLOAD_SIZE,
};

#[cfg(feature = "v2.1-region-sync")]
pub use region_sync::{
    RegionState, SyncResult, SyncError, SyncConfig, DeltaEntry,
    generate_deltas, apply_deltas, resolve_conflicts, sync_region_state,
};
