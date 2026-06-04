//! Topological Filter â€” Filtro Determinista de AlineaciÃ³n.
//!
//! **Topological Law 2 (Reconocimiento del Error):** DetecciÃ³n de divergencia
//! KL y rechazo determinista de activaciones que se desvÃ­an del vector de alineaciÃ³n.
//!
//! **Feature Gate:** `v2.1-Topological-filter`

pub mod divergence;
pub mod slashing;

pub use divergence::{DivergenceChecker, DivergenceError};
pub use slashing::{AlignmentSlasher, SlashingError};
