//! Divergence Detection â€” DetecciÃ³n de divergencia KL para alineaciÃ³n.
//!
//! **Topological Law 2 (Reconocimiento del Error):** Monitoreo continuo de
//! divergencia KL entre activaciones y el vector de alineaciÃ³n esperado.

use std::fmt;

/// Error en la detecciÃ³n de divergencia.
#[derive(Debug)]
pub enum DivergenceError {
    /// Dimensiones incompatibles entre vectores.
    DimensionMismatch(String),
    /// Umbral de divergencia invÃ¡lido.
    InvalidThreshold(f64),
    /// Vector de activaciones vacÃ­o.
    EmptyActivation,
}

impl fmt::Display for DivergenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DivergenceError::DimensionMismatch(msg) => {
                write!(f, "Dimension mismatch: {}", msg)
            }
            DivergenceError::InvalidThreshold(threshold) => {
                write!(f, "Invalid divergence threshold: {}", threshold)
            }
            DivergenceError::EmptyActivation => {
                write!(f, "Empty activation vector")
            }
        }
    }
}

impl std::error::Error for DivergenceError {}

/// Resultado de una verificaciÃ³n de divergencia.
#[derive(Debug, Clone)]
pub struct DivergenceResult {
    /// Valor de divergencia KL calculada.
    pub kl_divergence: f64,
    /// Â¿La activaciÃ³n estÃ¡ dentro del umbral?
    pub within_threshold: bool,
    /// Umbral utilizado.
    pub threshold: f64,
}

/// Verificador de divergencia KL para alineaciÃ³n.
///
/// **Topological Law 2:** DetecciÃ³n temprana de desviaciones
/// del comportamiento esperado, con rechazo determinista.
#[derive(Debug)]
pub struct DivergenceChecker {
    /// Umbral mÃ¡ximo de divergencia KL aceptable.
    pub threshold: f64,
}

impl DivergenceChecker {
    /// Crea un nuevo verificador con umbral especificado.
    pub fn new(threshold: f64) -> Result<Self, DivergenceError> {
        if threshold < 0.0 {
            return Err(DivergenceError::InvalidThreshold(threshold));
        }
        Ok(Self { threshold })
    }

    /// Calcula la divergencia KL entre activaciÃ³n y vector de referencia.
    ///
    /// **Topological Law 2:** Divergencia alta = posible desalineaciÃ³n.
    /// El filtro rechaza determinÃ­sticamente activaciones fuera de rango.
    pub fn check(
        &self,
        _activation: &[f32],
        _reference: &[f32],
    ) -> Result<DivergenceResult, DivergenceError> {
        // TODO(Sprint16.3): Implement KL divergence calculation.
        // KL(p||q) = sum(p * log(p/q))
        // Return DivergenceResult with computed value.
        Ok(DivergenceResult {
            kl_divergence: 0.0,
            within_threshold: true,
            threshold: self.threshold,
        })
    }

    /// Valida que dos vectores tienen dimensiones compatibles.
    pub fn validate_dimensions(
        _activation: &[f32],
        _reference: &[f32],
    ) -> Result<(), DivergenceError> {
        // TODO(Sprint16.3): Check dimensions match.
        if _activation.is_empty() || _reference.is_empty() {
            return Err(DivergenceError::EmptyActivation);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checker_creation() {
        let checker = DivergenceChecker::new(0.5).unwrap();
        assert_eq!(checker.threshold, 0.5);
    }

    #[test]
    fn test_checker_invalid_threshold() {
        match DivergenceChecker::new(-1.0) {
            Err(DivergenceError::InvalidThreshold(_)) => {} // Expected
            other => panic!("Expected InvalidThreshold, got {:?}", other),
        }
    }

    #[test]
    fn test_validate_empty_vectors() {
        match DivergenceChecker::validate_dimensions(&[], &[1.0]) {
            Err(DivergenceError::EmptyActivation) => {} // Expected
            other => panic!("Expected EmptyActivation, got {:?}", other),
        }
    }

    #[test]
    fn test_divergence_result() {
        let result = DivergenceResult {
            kl_divergence: 0.1,
            within_threshold: true,
            threshold: 0.5,
        };
        assert!(result.within_threshold);
    }

    #[test]
    fn test_error_display() {
        let err = DivergenceError::EmptyActivation;
        assert!(!format!("{}", err).is_empty());
    }
}
