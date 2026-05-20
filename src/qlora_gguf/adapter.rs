//! QLoRA Adapter — Aplicación de diffs QLoRA sobre modelos base GGUF.
//!
//! **Stuartian Law 3 (Inteligencia Holística):** Solo se distribuyen
//! los adapters (KB/MB), no el modelo completo (GB).

use std::fmt;

/// Error al aplicar o gestionar un adapter QLoRA.
#[derive(Debug)]
pub enum QloraAdapterError {
    /// Adapter incompatible con el modelo base.
    IncompatibleBase(String),
    /// Rango de rank inválido.
    InvalidRank(usize),
    /// Error de serialización.
    Serialization(String),
    /// Adapter corrupto.
    CorruptAdapter(String),
}

impl fmt::Display for QloraAdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QloraAdapterError::IncompatibleBase(msg) => {
                write!(f, "Adapter incompatible with base model: {}", msg)
            }
            QloraAdapterError::InvalidRank(rank) => {
                write!(f, "Invalid adapter rank: {}", rank)
            }
            QloraAdapterError::Serialization(msg) => {
                write!(f, "Serialization error: {}", msg)
            }
            QloraAdapterError::CorruptAdapter(msg) => {
                write!(f, "Corrupt adapter: {}", msg)
            }
        }
    }
}

impl std::error::Error for QloraAdapterError {}

/// Metadata de un adapter QLoRA.
#[derive(Debug, Clone)]
pub struct AdapterInfo {
    /// Identificador único del adapter.
    pub id: String,
    /// Modelo base con el que es compatible.
    pub base_model: String,
    /// Rank del adapter (típicamente 4, 8, 16, 32).
    pub rank: usize,
    /// Alpha scaling factor.
    pub alpha: f32,
    /// Tamaño comprimido en bytes.
    pub size_bytes: u64,
    /// Hash de integridad.
    pub integrity_hash: String,
}

/// Adapter QLoRA para aplicar sobre un modelo base GGUF.
///
/// **Stuartian Law 3:** El adapter es un diff ligero (KB/MB)
/// que se aplica sobre el modelo base inmutable (GB).
pub struct QloraAdapter {
    /// Metadata del adapter.
    pub info: AdapterInfo,
    /// Datos del adapter (pesos A y B, bias, etc.)
    /// TODO(Sprint16.1): Reemplazar con candle-core tensors.
    _weights: Vec<u8>,
}

impl QloraAdapter {
    /// Crea un nuevo adapter QLoRA desde datos serializados.
    pub fn from_bytes(
        _data: &[u8],
    ) -> Result<Self, QloraAdapterError> {
        // TODO(Sprint16.1): Implement deserialization from candle-core format.
        Err(QloraAdapterError::Serialization(
            "Adapter deserialization not yet implemented".into(),
        ))
    }

    /// Serializa el adapter a bytes para distribución GossipSub.
    pub fn to_bytes(&self) -> Result<Vec<u8>, QloraAdapterError> {
        // TODO(Sprint16.1): Implement serialization.
        Ok(self._weights.clone())
    }

    /// Valida la integridad del adapter.
    pub fn validate(&self) -> Result<(), QloraAdapterError> {
        // TODO(Sprint16.1): Implement integrity check (hash verification).
        if self._weights.is_empty() {
            return Err(QloraAdapterError::CorruptAdapter(
                "Empty adapter weights".into(),
            ));
        }
        Ok(())
    }

    /// Aplica el adapter sobre un modelo base GGUF.
    ///
    /// **Stuartian Law 3:** Operación in-place sobre tensors,
    /// sin copiar el modelo base completo.
    pub fn apply(
        &self,
        _base_model: &str,
    ) -> Result<(), QloraAdapterError> {
        // TODO(Sprint16.1): Implement candle-core tensor application.
        Err(QloraAdapterError::IncompatibleBase(
            "Adapter application not yet implemented".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = QloraAdapterError::InvalidRank(0);
        assert!(!format!("{}", err).is_empty());
    }

    #[test]
    fn test_adapter_info_creation() {
        let info = AdapterInfo {
            id: "test-adapter".into(),
            base_model: "qwen2-7b".into(),
            rank: 8,
            alpha: 16.0,
            size_bytes: 1024,
            integrity_hash: "abc123".into(),
        };
        assert_eq!(info.rank, 8);
    }
}
