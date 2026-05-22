//! Economics Module — Sprint 29
//!
//! Existential Credit and Proof of Symbiosis: non-transferable alignment
//! metrics for ethical compute verification and anti-Sybil consensus.

#[cfg(feature = "v2.1-proof-of-symbiosis")]
pub mod existential_credit;

#[cfg(all(
    feature = "v2.1-proof-of-symbiosis",
    feature = "v2.1-network-apoptosis"
))]
pub mod proof_of_symbiosis;
