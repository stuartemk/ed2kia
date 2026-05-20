//! QLoRA Payload — Compresión y serialización para distribución GossipSub.
//!
//! **Stuartian Law 3 (Inteligencia Holística):** Payloads ≤MB para
//! eficiencia termodinámica en la red P2P.

use std::fmt;

/// Error al crear o procesar un payload QLoRA.
#[derive(Debug)]
pub enum QloraPayloadError {
    /// Payload excede el límite de tamaño.
    PayloadTooLarge(usize),
    /// Error de compresión.
    CompressionError(String),
    /// Error de descompresión.
    DecompressionError(String),
    /// Payload corrupto.
    CorruptPayload(String),
}

impl fmt::Display for QloraPayloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QloraPayloadError::PayloadTooLarge(size) => {
                write!(f, "Payload too large: {} bytes", size)
            }
            QloraPayloadError::CompressionError(msg) => {
                write!(f, "Compression error: {}", msg)
            }
            QloraPayloadError::DecompressionError(msg) => {
                write!(f, "Decompression error: {}", msg)
            }
            QloraPayloadError::CorruptPayload(msg) => {
                write!(f, "Corrupt payload: {}", msg)
            }
        }
    }
}

impl std::error::Error for QloraPayloadError {}

/// Límite máximo de payload en bytes (1 MB).
/// **Stuartian Law 3:** Cero desperdicio, payloads ligeros.
pub const MAX_PAYLOAD_BYTES: usize = 1_048_576;

/// Payload comprimido de un adapter QLoRA para distribución P2P.
#[derive(Debug)]
pub struct QloraPayload {
    /// Identificador del adapter original.
    pub adapter_id: String,
    /// Datos comprimidos.
    pub compressed_data: Vec<u8>,
    /// Tamaño original en bytes.
    pub original_size: usize,
    /// Tamaño comprimido en bytes.
    pub compressed_size: usize,
    /// Ratio de compresión.
    pub compression_ratio: f32,
}

impl QloraPayload {
    /// Crea un payload comprimido desde datos de adapter.
    ///
    /// Valida que el tamaño no exceda `MAX_PAYLOAD_BYTES`.
    pub fn compress(
        _adapter_id: String,
        _data: &[u8],
    ) -> Result<Self, QloraPayloadError> {
        // TODO(Sprint16.1): Implement compression (e.g., zstd/lz4).
        if _data.len() > MAX_PAYLOAD_BYTES {
            return Err(QloraPayloadError::PayloadTooLarge(_data.len()));
        }
        Err(QloraPayloadError::CompressionError(
            "Compression not yet implemented".into(),
        ))
    }

    /// Descomprime el payload a datos de adapter originales.
    pub fn decompress(&self) -> Result<Vec<u8>, QloraPayloadError> {
        // TODO(Sprint16.1): Implement decompression.
        Err(QloraPayloadError::DecompressionError(
            "Decompression not yet implemented".into(),
        ))
    }

    /// Valida la integridad del payload.
    pub fn validate(&self) -> Result<(), QloraPayloadError> {
        if self.compressed_data.is_empty() {
            return Err(QloraPayloadError::CorruptPayload(
                "Empty compressed data".into(),
            ));
        }
        if self.compressed_size != self.compressed_data.len() {
            return Err(QloraPayloadError::CorruptPayload(
                "Size mismatch".into(),
            ));
        }
        Ok(())
    }

    /// Serializa el payload para GossipSub.
    pub fn to_gossipsub_bytes(&self) -> Vec<u8> {
        // TODO(Sprint16.1): Implement bincode/serde serialization.
        self.compressed_data.clone()
    }

    /// Deserializa desde bytes de GossipSub.
    pub fn from_gossipsub_bytes(_data: &[u8]) -> Result<Self, QloraPayloadError> {
        // TODO(Sprint16.1): Implement deserialization.
        Err(QloraPayloadError::CorruptPayload(
            "Deserialization not yet implemented".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_payload_constant() {
        assert_eq!(MAX_PAYLOAD_BYTES, 1_048_576);
    }

    #[test]
    fn test_compress_too_large() {
        let big_data = vec![0u8; MAX_PAYLOAD_BYTES + 1];
        match QloraPayload::compress("test".into(), &big_data) {
            Err(QloraPayloadError::PayloadTooLarge(_)) => {} // Expected
            other => panic!("Expected PayloadTooLarge, got {:?}", other),
        }
    }

    #[test]
    fn test_error_display() {
        let err = QloraPayloadError::PayloadTooLarge(999);
        assert!(!format!("{}", err).is_empty());
    }

    #[test]
    fn test_payload_validate_empty() {
        let payload = QloraPayload {
            adapter_id: "test".into(),
            compressed_data: vec![],
            original_size: 0,
            compressed_size: 0,
            compression_ratio: 0.0,
        };
        match payload.validate() {
            Err(QloraPayloadError::CorruptPayload(_)) => {} // Expected
            other => panic!("Expected CorruptPayload, got {:?}", other),
        }
    }
}
