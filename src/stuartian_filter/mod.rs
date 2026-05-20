//! Stuartian Filter — Filtro Determinista de Alineación.
//!
//! **Stuartian Law 2 (Reconocimiento del Error):** Detección de divergencia
//! KL y rechazo determinista de activaciones que se desvían del vector de alineación.
//!
//! **Feature Gate:** `v2.1-stuartian-filter`

pub mod divergence;
pub mod slashing;

pub use divergence::{DivergenceChecker, DivergenceError};
pub use slashing::{AlignmentSlasher, SlashingError};
