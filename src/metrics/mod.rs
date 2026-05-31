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
