//! Metrics — Cooperative objective and loss functions for ethical alignment.
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

// ─── Sprint80: Gödelian Grace (paradox detection + singularity marking) ───
#[cfg(feature = "v9.16-godelian-synthesis")]
pub mod godelian_grace;

#[cfg(feature = "v9.16-godelian-synthesis")]
pub use godelian_grace::{
    detect_godelian_paradox, invoke_godelian_grace, GodelianGrace, GodelianNode, GraceConfig,
    GraceRecord, GraceState, NodeId,
};

// ─── Sprint81: Paradox Cost & Fractal Triage (CE burning + anti-DDoS Gödelian) ───
#[cfg(feature = "v9.17-biological-bridge")]
pub mod paradox_cost_triage;

#[cfg(feature = "v9.17-biological-bridge")]
pub use paradox_cost_triage::{
    apply_paradox_cost, cluster_paradoxes, CEBurnResult, MetaParadox, ParadoxCostTriage,
    TriageConfig,
};
