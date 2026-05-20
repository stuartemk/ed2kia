//! QLoRA Payload — Compresión y serialización para distribución GossipSub.
//!
//! **Stuartian Law 3 (Inteligencia Holística):** Cero desperdicio computacional.
//! Los payloads QLoRA se comprimen con zstd para distribución P2P eficiente.
//!
//! **Características:**
//! - Compresión zstd (ratio 10:1 - 100:1 típicamente)
//! - Serialización bincode (WASM-friendly, <50ms latencia)
//! - Validación de tamaño MAX_PAYLOAD_BYTES (1 MB)
//! - Compatible con GossipSub message-size-limits

use std::fmt;

/// Límite máximo de payload para GossipSub (1 MB).
/// Los nodos rechazan mensajes > MAX_PAYLOAD_BYTES para prevenir DoS.
pub const MAX_PAYLOAD_BYTES: usize = 1_048_576;

/// Error al crear, comprimir o descomprimir un QLoRA Payload.
#[derive(Debug)]
pub enum QloraPayloadError {
    /// Payload excede MAX_PAYLOAD_BYTES.
    PayloadTooLarge(usize),
    /// Error de compresión zstd.
    CompressionError(String),
    /// Error de descompresión zstd.
    DecompressionError(String),
    /// Error de serialización bincode.
    SerializationError(String),
    /// Data corrupta o inválida.
    CorruptedData(String),
    /// Error de I/O.
    Io(std::io::Error),
}

impl fmt::Display for QloraPayloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QloraPayloadError::PayloadTooLarge(size) => {
                write!(
                    f,
                    "Payload too large: {} bytes (max: {} bytes)",
                    size, MAX_PAYLOAD_BYTES
                )
            }
            QloraPayloadError::CompressionError(msg) => {
                write!(f, "Compression error: {}", msg)
            }
            QloraPayloadError::DecompressionError(msg) => {
                write!(f, "Decompression error: {}", msg)
            }
            QloraPayloadError::SerializationError(msg) => {
                write!(f, "Serialization error: {}", msg)
            }
            QloraPayloadError::CorruptedData(msg) => {
                write!(f, "Corrupted data: {}", msg)
            }
            QloraPayloadError::Io(err) => write!(f, "I/O error: {}", err),
        }
    }
}

impl std::error::Error for QloraPayloadError {}

impl From<std::io::Error> for QloraPayloadError {
    fn from(err: std::io::Error) -> Self {
        QloraPayloadError::Io(err)
    }
}

/// Payload QLoRA comprimido para distribución P2P.
///
/// Contiene los bytes serializados de un QLoRAAdapter
/// comprimidos con zstd para minimizar el tamaño en red.
///
/// **Flujo de serialización:**
/// 1. QLoRAAdapter -> bincode -> Vec<u8>
/// 2. Vec<u8> -> zstd compress -> Vec<u8> (KB range)
/// 3. Vec<u8> -> GossipSub publish
///
/// **Flujo de deserialización:**
/// 1. GossipSub receive -> Vec<u8>
/// 2. Vec<u8> -> zstd decompress -> Vec<u8>
/// 3. Vec<u8> -> bincode -> QLoRAAdapter
#[derive(Debug, Clone)]
pub struct QloraPayload {
    /// Bytes comprimidos (zstd).
    pub compressed: Vec<u8>,
    /// Tamaño original antes de compresión.
    pub original_size: usize,
    /// Identificador del adapter (para deduplicación).
    pub adapter_id: String,
    /// Checksum SHA256 del modelo base.
    pub base_model_sha256: String,
}

impl QloraPayload {
    /// Comprime datos crudos con zstd.
    ///
    /// # Arguments
    /// * `adapter_id` - Identificador del adapter.
    /// * `data` - Datos crudos a comprimir (output de bincode).
    ///
    /// # Returns
    /// Payload comprimido listo para distribución P2P.
    #[cfg(feature = "v2.1-qlora-gguf")]
    pub fn compress(
        adapter_id: String,
        data: &[u8],
    ) -> Result<Self, QloraPayloadError> {
        let original_size = data.len();

        // Validate size before compression
        if original_size > MAX_PAYLOAD_BYTES {
            return Err(QloraPayloadError::PayloadTooLarge(original_size));
        }

        // Compress with zstd (level 3 = good speed/size tradeoff)
        let compressed = zstd::encode_all(std::io::Cursor::new(data), 3)
            .map_err(|e| QloraPayloadError::CompressionError(e.to_string()))?;

        Ok(QloraPayload {
            compressed,
            original_size,
            adapter_id,
            base_model_sha256: String::new(), // Set from adapter metadata
        })
    }

    /// Comprime datos crudos (versión sin feature gate para testing).
    #[cfg(not(feature = "v2.1-qlora-gguf"))]
    pub fn compress(
        adapter_id: String,
        data: &[u8],
    ) -> Result<Self, QloraPayloadError> {
        let original_size = data.len();

        if original_size > MAX_PAYLOAD_BYTES {
            return Err(QloraPayloadError::PayloadTooLarge(original_size));
        }

        // Without zstd, return data as-is (no compression)
        Ok(QloraPayload {
            compressed: data.to_vec(),
            original_size,
            adapter_id,
            base_model_sha256: String::new(),
        })
    }

    /// Descomprime los datos del payload.
    ///
    /// # Returns
    /// Bytes descomprimidos (input para bincode deserialization).
    #[cfg(feature = "v2.1-qlora-gguf")]
    pub fn decompress(&self) -> Result<Vec<u8>, QloraPayloadError> {
        zstd::decode_all(std::io::Cursor::new(&self.compressed))
            .map_err(|e| QloraPayloadError::DecompressionError(e.to_string()))
    }

    /// Descomprime los datos del payload (versión sin feature gate).
    #[cfg(not(feature = "v2.1-qlora-gguf"))]
    pub fn decompress(&self) -> Result<Vec<u8>, QloraPayloadError> {
        Ok(self.compressed.clone())
    }

    /// Valida el payload.
    ///
    /// Verifica:
    /// - compressed no está vacío
    /// - original_size > 0
    /// - original_size < MAX_PAYLOAD_BYTES
    /// - adapter_id no está vacío
    pub fn validate(&self) -> Result<(), QloraPayloadError> {
        if self.compressed.is_empty() {
            return Err(QloraPayloadError::CorruptedData(
                "Compressed data is empty".into(),
            ));
        }

        if self.original_size == 0 {
            return Err(QloraPayloadError::CorruptedData(
                "Original size is 0".into(),
            ));
        }

        if self.original_size > MAX_PAYLOAD_BYTES {
            return Err(QloraPayloadError::PayloadTooLarge(self.original_size));
        }

        if self.adapter_id.is_empty() {
            return Err(QloraPayloadError::CorruptedData(
                "Adapter ID is empty".into(),
            ));
        }

        Ok(())
    }

    /// Convierte el payload a bytes para GossipSub.
    ///
    /// Formato: [adapter_id_len(4)] [adapter_id] [base_sha256_len(4)] [base_sha256] [original_size(8)] [compressed_len(8)] [compressed]
    ///
    /// Este formato es compatible con la deserialización en nodos
    /// que reciben el mensaje vía GossipSub.
    pub fn to_gossipsub_bytes(&self) -> Vec<u8> {
        let mut data = Vec::new();

        // adapter_id
        let adapter_id_bytes = self.adapter_id.as_bytes();
        data.extend_from_slice(&(adapter_id_bytes.len() as u32).to_le_bytes());
        data.extend_from_slice(adapter_id_bytes);

        // base_model_sha256
        let sha256_bytes = self.base_model_sha256.as_bytes();
        data.extend_from_slice(&(sha256_bytes.len() as u32).to_le_bytes());
        data.extend_from_slice(sha256_bytes);

        // original_size
        data.extend_from_slice(&(self.original_size as u64).to_le_bytes());

        // compressed data
        let compressed_len = self.compressed.len();
        data.extend_from_slice(&(compressed_len as u64).to_le_bytes());
        data.extend_from_slice(&self.compressed);

        data
    }

    /// Deserializa un payload desde bytes de GossipSub.
    ///
    /// # Arguments
    /// * `data` - Bytes recibidos de GossipSub.
    ///
    /// # Returns
    /// Payload deserializado.
    pub fn from_gossipsub_bytes(data: &[u8]) -> Result<Self, QloraPayloadError> {
        if data.len() < 24 {
            // Minimum: 4 + 4 + 8 + 8 = 24 bytes header
            return Err(QloraPayloadError::CorruptedData(
                "Data too small for valid payload".into(),
            ));
        }

        let mut offset = 0;

        // Read adapter_id
        let id_len = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += 4;

        let adapter_id =
            String::from_utf8(data[offset..offset + id_len].to_vec()).map_err(|e| {
                QloraPayloadError::CorruptedData(format!("Invalid adapter_id UTF-8: {}", e))
            })?;
        offset += id_len;

        // Read base_model_sha256
        let sha256_len = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += 4;

        let base_model_sha256 =
            String::from_utf8(data[offset..offset + sha256_len].to_vec()).map_err(|e| {
                QloraPayloadError::CorruptedData(format!("Invalid sha256 UTF-8: {}", e))
            })?;
        offset += sha256_len;

        // Read original_size
        let original_size = u64::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]) as usize;
        offset += 8;

        // Read compressed_len
        let compressed_len = u64::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]) as usize;
        offset += 8;

        // Read compressed data
        if offset + compressed_len > data.len() {
            return Err(QloraPayloadError::CorruptedData(
                "Truncated compressed data".into(),
            ));
        }
        let compressed = data[offset..offset + compressed_len].to_vec();

        Ok(QloraPayload {
            compressed,
            original_size,
            adapter_id,
            base_model_sha256,
        })
    }

    /// Calcula el ratio de compresión.
    ///
    /// # Returns
    /// Ratio original_size / compressed_size (mayor = mejor compresión).
    pub fn compression_ratio(&self) -> f64 {
        if self.compressed.is_empty() {
            return 0.0;
        }
        self.original_size as f64 / self.compressed.len() as f64
    }

    /// Retorna el tamaño del payload comprimido en bytes.
    pub fn compressed_size(&self) -> usize {
        self.compressed.len()
    }

    /// Retorna el tamaño total que ocupará en GossipSub.
    pub fn gossipsub_size(&self) -> usize {
        self.to_gossipsub_bytes().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_payload_constant() {
        assert_eq!(MAX_PAYLOAD_BYTES, 1_048_576); // 1 MB
    }

    #[test]
    fn test_compress_small_data() {
        let data = vec![0u8; 100];
        let payload = QloraPayload::compress("test-adapter".into(), &data).expect("compress");
        assert_eq!(payload.original_size, 100);
        assert_eq!(payload.adapter_id, "test-adapter");
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
    fn test_decompress_roundtrip() {
        let original = b"Hello, QLoRA World! This is test data for compression.";
        let payload = QloraPayload::compress("test".into(), original).expect("compress");
        let decompressed = payload.decompress().expect("decompress");
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_validate_valid_payload() {
        let data = vec![1u8; 50];
        let payload = QloraPayload::compress("valid-adapter".into(), &data).expect("compress");
        assert!(payload.validate().is_ok());
    }

    #[test]
    fn test_payload_validate_empty() {
        let payload = QloraPayload {
            compressed: vec![],
            original_size: 0,
            adapter_id: String::new(),
            base_model_sha256: String::new(),
        };
        match payload.validate() {
            Err(QloraPayloadError::CorruptedData(_)) => {} // Expected
            other => panic!("Expected CorruptedData, got {:?}", other),
        }
    }

    #[test]
    fn test_gossipsub_roundtrip() {
        let original = b"Test QLoRA payload data for GossipSub serialization.";
        let mut payload = QloraPayload::compress("adapter-123".into(), original).expect("compress");
        payload.base_model_sha256 = "abc123".to_string();

        let gossip_bytes = payload.to_gossipsub_bytes();
        let restored = QloraPayload::from_gossipsub_bytes(&gossip_bytes).expect("from_gossipsub");

        assert_eq!(restored.adapter_id, "adapter-123");
        assert_eq!(restored.original_size, payload.original_size);
        assert_eq!(restored.compressed, payload.compressed);
    }

    #[test]
    fn test_gossipsub_too_small() {
        let small_data = vec![0u8; 10];
        match QloraPayload::from_gossipsub_bytes(&small_data) {
            Err(QloraPayloadError::CorruptedData(_)) => {} // Expected
            other => panic!("Expected CorruptedData, got {:?}", other),
        }
    }

    #[test]
    fn test_compression_ratio() {
        // Highly compressible data (all zeros)
        let data = vec![0u8; 1000];
        let payload = QloraPayload::compress("test".into(), &data).expect("compress");
        let ratio = payload.compression_ratio();
        assert!(ratio > 1.0, "Compression ratio should be > 1 for compressible data");
    }

    #[test]
    fn test_error_display() {
        let err = QloraPayloadError::PayloadTooLarge(999_999);
        assert!(!format!("{}", err).is_empty());

        let err = QloraPayloadError::CorruptedData("test".into());
        assert!(!format!("{}", err).is_empty());
    }

    #[test]
    fn test_compressed_size() {
        let data = vec![1u8; 200];
        let payload = QloraPayload::compress("test".into(), &data).expect("compress");
        assert!(payload.compressed_size() > 0);
    }

    #[test]
    fn test_gossipsub_size() {
        let data = vec![1u8; 200];
        let payload = QloraPayload::compress("test".into(), &data).expect("compress");
        assert!(payload.gossipsub_size() > payload.compressed_size());
    }
}
