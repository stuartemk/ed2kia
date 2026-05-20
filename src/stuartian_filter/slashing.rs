//! Alignment Slashing — Penalización determinista por desalineación.
//!
//! **Stuartian Law 2 (Reconocimiento del Error):** Cuando un nodo
//! produce activaciones desalineadas, se aplica penalización de reputación.

use std::fmt;

/// Error en el proceso de slashing por desalineación.
#[derive(Debug)]
pub enum SlashingError {
    /// Nodo no encontrado.
    NodeNotFound(String),
    /// Penalización ya aplicada.
    AlreadySlashed,
    /// Umbral de slashing inválido.
    InvalidThreshold(f64),
}

impl fmt::Display for SlashingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SlashingError::NodeNotFound(node_id) => {
                write!(f, "Node not found: {}", node_id)
            }
            SlashingError::AlreadySlashed => {
                write!(f, "Node already slashed")
            }
            SlashingError::InvalidThreshold(threshold) => {
                write!(f, "Invalid slashing threshold: {}", threshold)
            }
        }
    }
}

impl std::error::Error for SlashingError {}

/// Registro de penalización por desalineación.
#[derive(Debug, Clone)]
pub struct SlashingRecord {
    /// Identificador del nodo penalizado.
    pub node_id: String,
    /// Razón de la penalización.
    pub reason: String,
    /// Magnitud de divergencia que triggered el slashing.
    pub divergence_value: f64,
    /// Penalización aplicada a reputación.
    pub reputation_penalty: f64,
}

/// Ejecutor de slashing determinista por desalineación.
///
/// **Stuartian Law 2:** Cero tolerancia con activaciones maliciosas.
/// El slashing es determinista: mismo input = misma penalización.
#[derive(Debug)]
pub struct AlignmentSlasher {
    /// Umbral de divergencia que trigger slashing.
    pub slashing_threshold: f64,
    /// Penalización de reputación por slashing.
    pub reputation_penalty: f64,
}

impl AlignmentSlasher {
    /// Crea un nuevo executor de slashing.
    pub fn new(
        slashing_threshold: f64,
        reputation_penalty: f64,
    ) -> Result<Self, SlashingError> {
        if slashing_threshold < 0.0 {
            return Err(SlashingError::InvalidThreshold(slashing_threshold));
        }
        if !(0.0..=1.0).contains(&reputation_penalty) {
            return Err(SlashingError::InvalidThreshold(reputation_penalty));
        }
        Ok(Self {
            slashing_threshold,
            reputation_penalty,
        })
    }

    /// Evalúa si un nodo debe ser slashado basado en divergencia.
    ///
    /// **Stuartian Law 2:** Decisión determinista. Si divergence > threshold,
    /// se aplica penalización automáticamente.
    pub fn evaluate(
        &self,
        _node_id: &str,
        _divergence: f64,
    ) -> Option<SlashingRecord> {
        // TODO(Sprint16.3): Implement deterministic slashing decision.
        // If divergence > slashing_threshold, return SlashingRecord.
        None
    }

    /// Aplica penalización de reputación a un nodo.
    pub fn apply_penalty(
        &self,
        _node_id: &str,
    ) -> Result<SlashingRecord, SlashingError> {
        // TODO(Sprint16.3): Implement reputation penalty application.
        Err(SlashingError::NodeNotFound(_node_id.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slasher_creation() {
        let slasher = AlignmentSlasher::new(0.5, 0.1).unwrap();
        assert_eq!(slasher.slashing_threshold, 0.5);
        assert_eq!(slasher.reputation_penalty, 0.1);
    }

    #[test]
    fn test_slasher_invalid_threshold() {
        match AlignmentSlasher::new(-1.0, 0.1) {
            Err(SlashingError::InvalidThreshold(_)) => {} // Expected
            other => panic!("Expected InvalidThreshold, got {:?}", other),
        }
    }

    #[test]
    fn test_slasher_invalid_penalty() {
        match AlignmentSlasher::new(0.5, 1.5) {
            Err(SlashingError::InvalidThreshold(_)) => {} // Expected
            other => panic!("Expected InvalidThreshold, got {:?}", other),
        }
    }

    #[test]
    fn test_slashing_record() {
        let record = SlashingRecord {
            node_id: "node-1".into(),
            reason: "High divergence".into(),
            divergence_value: 0.8,
            reputation_penalty: 0.1,
        };
        assert_eq!(record.divergence_value, 0.8);
    }

    #[test]
    fn test_error_display() {
        let err = SlashingError::AlreadySlashed;
        assert!(!format!("{}", err).is_empty());
    }
}
