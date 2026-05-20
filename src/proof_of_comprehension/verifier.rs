//! Comprehension Verifier — Verificación criptográfica de prueba de comprensión.
//!
//! **Stuartian Law 2 (Reconocimiento del Error):** Cada prueba de trabajo útil
//! se verifica criptográficamente para garantizar transparencia y auditabilidad.

use std::fmt;

/// Error al verificar una prueba de comprensión.
#[derive(Debug)]
pub enum ComprehensionVerifierError {
    /// Prueba inválida.
    InvalidProof(String),
    /// Firma inválida.
    InvalidSignature,
    /// Tarea no encontrada.
    TaskNotFound(String),
    /// Prueba expirada.
    ProofExpired,
}

impl fmt::Display for ComprehensionVerifierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComprehensionVerifierError::InvalidProof(msg) => {
                write!(f, "Invalid proof: {}", msg)
            }
            ComprehensionVerifierError::InvalidSignature => {
                write!(f, "Invalid signature")
            }
            ComprehensionVerifierError::TaskNotFound(task_id) => {
                write!(f, "Task not found: {}", task_id)
            }
            ComprehensionVerifierError::ProofExpired => {
                write!(f, "Proof expired")
            }
        }
    }
}

impl std::error::Error for ComprehensionVerifierError {}

/// Resultado de verificación de una prueba de comprensión.
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// ¿La prueba es válida?
    pub valid: bool,
    /// Identificador de la tarea verificada.
    pub task_id: String,
    /// Identificador del nodo que presentó la prueba.
    pub node_id: String,
    /// Mensaje de verificación.
    pub message: String,
}

/// Verificador de pruebas de comprensión.
///
/// **Stuartian Law 2:** Garantiza que cada nodo demostró
/// comprensión real, no trabajo especulativo.
pub struct ComprehensionVerifier;

impl ComprehensionVerifier {
    /// Crea un nuevo verificador.
    pub fn new() -> Self {
        ComprehensionVerifier
    }

    /// Verifica una prueba de comprensión.
    ///
    /// **Stuartian Law 2:** Auditoría transparente. Cada verificación
    /// genera un registro inmutable para el ledger de reputación.
    pub fn verify(
        &self,
        _task_id: &str,
        _node_id: &str,
        _proof: &[u8],
    ) -> Result<VerificationResult, ComprehensionVerifierError> {
        // TODO(Sprint16.2): Implement cryptographic verification.
        // - Validate proof structure
        // - Verify Ed25519 signature
        // - Check activation gradients against expected ranges
        // - Generate immutable verification record
        Ok(VerificationResult {
            valid: false,
            task_id: _task_id.to_string(),
            node_id: _node_id.to_string(),
            message: "Verification not yet implemented".into(),
        })
    }

    /// Valida la estructura de una prueba sin verificar firma.
    pub fn validate_structure(&self, _proof: &[u8]) -> Result<(), ComprehensionVerifierError> {
        // TODO(Sprint16.2): Implement structural validation.
        if _proof.is_empty() {
            return Err(ComprehensionVerifierError::InvalidProof(
                "Empty proof".into(),
            ));
        }
        Ok(())
    }
}

impl Default for ComprehensionVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verifier_creation() {
        let verifier = ComprehensionVerifier::new();
        let _ = verifier;
    }

    #[test]
    fn test_verifier_default() {
        let _ = ComprehensionVerifier::default();
    }

    #[test]
    fn test_validate_empty_proof() {
        let verifier = ComprehensionVerifier::new();
        match verifier.validate_structure(&[]) {
            Err(ComprehensionVerifierError::InvalidProof(_)) => {} // Expected
            other => panic!("Expected InvalidProof, got {:?}", other),
        }
    }

    #[test]
    fn test_error_display() {
        let err = ComprehensionVerifierError::InvalidSignature;
        assert!(!format!("{}", err).is_empty());
    }

    #[test]
    fn test_verification_result() {
        let result = VerificationResult {
            valid: true,
            task_id: "task-1".into(),
            node_id: "node-1".into(),
            message: "OK".into(),
        };
        assert!(result.valid);
    }
}
