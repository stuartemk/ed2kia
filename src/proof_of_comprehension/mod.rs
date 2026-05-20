//! Proof of Comprehension — Prueba criptográfica de trabajo útil.
//!
//! **Stuartian Law 2 (Reconocimiento del Error):** SAEs, validación de gradientes,
//! auditoría transparente. Cada nodo demuestra comprensión real, no hash vacío.
//!
//! **Feature Gate:** `v2.1-proof-of-comprehension`

pub mod task;
pub mod verifier;

pub use task::{ComprehensionTask, ComprehensionTaskError};
pub use verifier::{ComprehensionVerifier, ComprehensionVerifierError};
