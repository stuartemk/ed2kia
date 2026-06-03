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
    kademlia_distance, AutoNatEngine, AutoNatStatus, BucketAction, CircuitState, KTable,
    MeshConfig, MeshError, MeshStats, NodeCapabilities as MeshNodeCapabilities, PeerEntry,
    PlanetaryMesh, RelayCircuit,
};

#[cfg(feature = "v3.7-symbiotic-portal")]
pub mod bootstrap;

#[cfg(feature = "v3.7-symbiotic-portal")]
pub use bootstrap::{
    BootstrapConfig, BootstrapProtocol, BootstrapStats, BootstrapStrategy, DiscoveryResult,
    SeedNode, TransportType,
};

#[cfg(feature = "v4.0-snap-activation")]
pub mod proliferation;

#[cfg(feature = "v4.0-snap-activation")]
pub use proliferation::{
    DeploymentArtifact, Platform, ProliferationConfig, ProliferationError, SymbioticProliferator,
};

#[cfg(feature = "v9.4-validation-layer")]
pub mod spectral_coherence;

#[cfg(feature = "v9.4-validation-layer")]
pub use spectral_coherence::{
    algebraic_connectivity, compute_spectral_coherence, cross_correlation, pearson_correlation,
    sync_rate, SpectralCoherenceResult,
};

#[cfg(feature = "v9.5-testnet-hardening")]
pub mod workload_scheduler;

#[cfg(feature = "v9.5-testnet-hardening")]
pub use workload_scheduler::{
    build_assignment_map, distribute_shards, load_balance_ratio, NodeTier, SchedulerState,
    ShardAssignment, LATENCY_THRESHOLD_MS,
};

#[cfg(feature = "v9.6-civilization-scale")]
pub mod hierarchical_gossip;

#[cfg(feature = "v9.6-civilization-scale")]
pub use hierarchical_gossip::{
    Committee, GossipConfig, GossipError, GossipNode, GossipUpdate, HierarchicalGossip,
};

#[cfg(feature = "v9.7-bootstrap-resilience")]
pub mod global_bootstrap;

#[cfg(feature = "v9.7-bootstrap-resilience")]
pub use global_bootstrap::{
    run_ignition_sequence, BootstrapError, BootstrapNode, BootstrapPhase, BootstrapProtocolConfig,
    BootstrapState, GlobalBootstrap,
};

// ─── Sprint81: Async Mesh & Sneakernet (offline DAG + VersionVector merging) ───
#[cfg(feature = "v9.17-biological-bridge")]
pub mod async_mesh_sneakernet;

#[cfg(feature = "v9.17-biological-bridge")]
pub use async_mesh_sneakernet::{
    merge_offline_dags, AsyncMeshSneakernet, DagState, MergeResult, VersionVector,
};
