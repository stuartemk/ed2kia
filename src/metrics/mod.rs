//! Metrics â€” Cooperative objective and loss functions for ethical alignment.
//!
//! Contains the Love = Zero Conflict formalization as a pairwise L2 divergence
//! with KL divergence entropy proxy for policy diversity.

#[cfg(feature = "v9.4-validation-layer")]
pub mod cooperative_objective;

#[cfg(feature = "v9.4-validation-layer")]
pub use cooperative_objective::{
    compute_love_metric_loss, kl_divergence_entropy, pairwise_l2_divergence, BenchmarkScore,
    EPSILON, LAMBDA, MU,
};

// â”€â”€â”€ Sprint80: Undecidable Grace (paradox detection + singularity marking) â”€â”€â”€
#[cfg(feature = "v9.16-undecidable-synthesis")]
pub mod undecidable_grace;

#[cfg(feature = "v9.16-undecidable-synthesis")]
pub use undecidable_grace::{
    detect_undecidable_paradox, invoke_undecidable_grace, undecidableGrace, undecidableNode,
    GraceConfig, GraceRecord, GraceState, NodeId,
};

// â”€â”€â”€ Sprint81: Paradox Cost & Fractal Triage (CE burning + anti-DDoS Undecidable) â”€â”€â”€
#[cfg(feature = "v9.17-biological-bridge")]
pub mod paradox_cost_triage;

#[cfg(feature = "v9.17-biological-bridge")]
pub use paradox_cost_triage::{
    apply_paradox_cost, cluster_paradoxes, CEBurnResult, MetaParadox, ParadoxCostTriage,
    TriageConfig,
};
