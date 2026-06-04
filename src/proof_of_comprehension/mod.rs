//! Proof of Comprehension â€” Prueba criptogrÃ¡fica de trabajo Ãºtil.
//!
//! **Topological Law 2 (Reconocimiento del Error):** SAEs, validaciÃ³n de gradientes,
//! auditorÃ­a transparente. Cada nodo demuestra comprensiÃ³n real, no hash vacÃ­o.
//!
//! **Feature Gate:** `v2.1-proof-of-comprehension`

pub mod task;
pub mod verifier;

pub use task::{ComprehensionTask, ComprehensionTaskError};
pub use verifier::{ComprehensionVerifier, ComprehensionVerifierError};
