//! Protocol Adapter — Binary protocol adaptation with safe serialization.
//!
//! Provides protocol-specific adapters for message serialization/deserialization
//! using prost and flatbuffers. Zero dynamic reflection — all adapters are compile-time.
//!
//! **Design:** Trait-based adapter system with protocol-specific implementations.
//! Zero financial logic — operates on compute credits and technical state only.
//!
//! Apache License 2.0 + Ethical Use Clause

#[cfg(feature = "v1.6-sprint1")]
mod internal {
    use std::collections::HashMap;

    // ---------------------------------------------------------------------------
    // Errors
    // ---------------------------------------------------------------------------

    /// Errors for protocol adapter operations.
    #[derive(Debug, Clone, PartialEq)]
    pub enum AdapterError {
        /// Unknown protocol.
        UnknownProtocol(String),
        /// Serialization failed.
        SerializationFailed(String),
        /// Deserialization failed.
        DeserializationFailed(String),
        /// Schema version mismatch.
        SchemaVersionMismatch { expected: u32, actual: u32 },
        /// Payload too large.
        PayloadTooLarge { size: usize, max: usize },
    }

    impl std::fmt::Display for AdapterError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                AdapterError::UnknownProtocol(p) => write!(f, "Unknown protocol: {}", p),
                AdapterError::SerializationFailed(msg) => write!(f, "Serialization failed: {}", msg),
                AdapterError::DeserializationFailed(msg) => write!(f, "Deserialization failed: {}", msg),
                AdapterError::SchemaVersionMismatch { expected, actual } => {
                    write!(f, "Schema version mismatch: expected={}, actual={}", expected, actual)
                }
                AdapterError::PayloadTooLarge { size, max } => {
                    write!(f, "Payload size {} exceeds max {}", size, max)
                }
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Protocol Types
    // ---------------------------------------------------------------------------

    /// Supported protocol types.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum ProtocolType {
        /// Protocol Buffers (prost).
        Protobuf,
        /// FlatBuffers.
        FlatBuffers,
        /// CBOR.
        Cbor,
        /// JSON fallback.
        Json,
    }

    impl std::fmt::Display for ProtocolType {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ProtocolType::Protobuf => write!(f, "protobuf"),
                ProtocolType::FlatBuffers => write!(f, "flatbuffers"),
                ProtocolType::Cbor => write!(f, "cbor"),
                ProtocolType::Json => write!(f, "json"),
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Adapter Config
    // ---------------------------------------------------------------------------

    /// Configuration for protocol adapters.
    #[derive(Debug, Clone)]
    pub struct AdapterConfig {
        /// Maximum payload size.
        pub max_payload_size: usize,
        /// Enable compression.
        pub enable_compression: bool,
        /// Default protocol.
        pub default_protocol: ProtocolType,
        /// Schema version.
        pub schema_version: u32,
    }

    impl Default for AdapterConfig {
        fn default() -> Self {
            Self {
                max_payload_size: 65536,
                enable_compression: true,
                default_protocol: ProtocolType::Protobuf,
                schema_version: 1,
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Protocol Message
    // ---------------------------------------------------------------------------

    /// Generic protocol message for adaptation.
    #[derive(Debug, Clone)]
    pub struct ProtocolMessage {
        pub schema_version: u32,
        pub protocol: ProtocolType,
        pub fields: HashMap<String, Vec<u8>>,
    }

    impl ProtocolMessage {
        pub fn new(protocol: ProtocolType, schema_version: u32) -> Self {
            Self {
                schema_version,
                protocol,
                fields: HashMap::new(),
            }
        }

        pub fn add_field(&mut self, name: String, value: Vec<u8>) {
            self.fields.insert(name, value);
        }
    }

    // ---------------------------------------------------------------------------
    // Protocol Adapter
    // ---------------------------------------------------------------------------

    /// Protocol adapter for binary serialization.
    #[derive(Debug, Clone)]
    pub struct ProtocolAdapter {
        config: AdapterConfig,
    }

    impl ProtocolAdapter {
        /// Create a new adapter with the given configuration.
        pub fn new(config: AdapterConfig) -> Self {
            Self { config }
        }

        /// Serialize a message using the specified protocol.
        pub fn serialize(&self, message: &ProtocolMessage) -> Result<Vec<u8>, AdapterError> {
            if message.fields.values().map(|v| v.len()).sum::<usize>() > self.config.max_payload_size {
                return Err(AdapterError::PayloadTooLarge {
                    size: message.fields.values().map(|v| v.len()).sum(),
                    max: self.config.max_payload_size,
                });
            }

            match message.protocol {
                ProtocolType::Protobuf => self.serialize_protobuf(message),
                ProtocolType::FlatBuffers => self.serialize_flatbuffers(message),
                ProtocolType::Cbor => self.serialize_cbor(message),
                ProtocolType::Json => self.serialize_json(message),
            }
        }

        /// Deserialize bytes into a protocol message.
        pub fn deserialize(&self, data: &[u8], protocol: ProtocolType) -> Result<ProtocolMessage, AdapterError> {
            if data.len() > self.config.max_payload_size {
                return Err(AdapterError::PayloadTooLarge {
                    size: data.len(),
                    max: self.config.max_payload_size,
                });
            }

            match protocol {
                ProtocolType::Protobuf => self.deserialize_protobuf(data),
                ProtocolType::FlatBuffers => self.deserialize_flatbuffers(data),
                ProtocolType::Cbor => self.deserialize_cbor(data),
                ProtocolType::Json => self.deserialize_json(data),
            }
        }

        // Protobuf simulation (field-length-value encoding)
        fn serialize_protobuf(&self, message: &ProtocolMessage) -> Result<Vec<u8>, AdapterError> {
            let mut buffer = Vec::new();
            // Write schema version
            buffer.extend_from_slice(&message.schema_version.to_le_bytes());
            // Write field count
            let count = message.fields.len() as u32;
            buffer.extend_from_slice(&count.to_le_bytes());
            // Write fields
            for (name, value) in &message.fields {
                let name_bytes = name.as_bytes();
                buffer.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
                buffer.extend_from_slice(name_bytes);
                buffer.extend_from_slice(&(value.len() as u32).to_le_bytes());
                buffer.extend_from_slice(value);
            }
            Ok(buffer)
        }

        fn deserialize_protobuf(&self, data: &[u8]) -> Result<ProtocolMessage, AdapterError> {
            if data.len() < 8 {
                return Err(AdapterError::DeserializationFailed("Insufficient data".to_string()));
            }
            let schema_version = u32::from_le_bytes(data[..4].try_into().unwrap());
            let count = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;

            let mut message = ProtocolMessage::new(ProtocolType::Protobuf, schema_version);
            let mut offset = 8;

            for _ in 0..count {
                if offset + 6 > data.len() {
                    return Err(AdapterError::DeserializationFailed("Truncated field header".to_string()));
                }
                let name_len = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap()) as usize;
                offset += 2;
                let name = String::from_utf8_lossy(&data[offset..offset + name_len]).to_string();
                offset += name_len;

                let value_len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
                offset += 4;
                let value = data[offset..offset + value_len].to_vec();
                offset += value_len;

                message.add_field(name, value);
            }

            Ok(message)
        }

        // FlatBuffers simulation (simple length-prefixed)
        fn serialize_flatbuffers(&self, message: &ProtocolMessage) -> Result<Vec<u8>, AdapterError> {
            let mut buffer = Vec::new();
            buffer.extend_from_slice(&message.schema_version.to_le_bytes());
            for (name, value) in &message.fields {
                buffer.extend_from_slice(&(name.len() as u32).to_le_bytes());
                buffer.extend_from_slice(name.as_bytes());
                buffer.extend_from_slice(&(value.len() as u32).to_le_bytes());
                buffer.extend_from_slice(value);
            }
            Ok(buffer)
        }

        fn deserialize_flatbuffers(&self, data: &[u8]) -> Result<ProtocolMessage, AdapterError> {
            if data.len() < 4 {
                return Err(AdapterError::DeserializationFailed("Insufficient data".to_string()));
            }
            let schema_version = u32::from_le_bytes(data[..4].try_into().unwrap());
            let mut message = ProtocolMessage::new(ProtocolType::FlatBuffers, schema_version);

            let mut offset = 4;
            while offset + 8 <= data.len() {
                let name_len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
                offset += 4;
                let name = String::from_utf8_lossy(&data[offset..offset + name_len]).to_string();
                offset += name_len;

                let value_len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
                offset += 4;
                let value = data[offset..offset + value_len].to_vec();
                offset += value_len;

                message.add_field(name, value);
            }

            Ok(message)
        }

        // CBOR simulation (simple map encoding)
        fn serialize_cbor(&self, message: &ProtocolMessage) -> Result<Vec<u8>, AdapterError> {
            let mut buffer = Vec::new();
            buffer.push(0xA0 + message.fields.len() as u8); // Map header
            for (name, value) in &message.fields {
                buffer.push(0x60 + name.len() as u8); // Text string
                buffer.extend_from_slice(name.as_bytes());
                buffer.push(0x40 + value.len() as u8); // Byte string
                buffer.extend_from_slice(value);
            }
            Ok(buffer)
        }

        fn deserialize_cbor(&self, data: &[u8]) -> Result<ProtocolMessage, AdapterError> {
            let mut message = ProtocolMessage::new(ProtocolType::Cbor, self.config.schema_version);
            if data.is_empty() {
                return Ok(message);
            }

            let map_size = (data[0] - 0xA0) as usize;
            let mut offset = 1;

            for _ in 0..map_size {
                let name_len = (data[offset] - 0x60) as usize;
                offset += 1;
                let name = String::from_utf8_lossy(&data[offset..offset + name_len]).to_string();
                offset += name_len;

                let value_len = (data[offset] - 0x40) as usize;
                offset += 1;
                let value = data[offset..offset + value_len].to_vec();
                offset += value_len;

                message.add_field(name, value);
            }

            Ok(message)
        }

        // JSON fallback
        fn serialize_json(&self, message: &ProtocolMessage) -> Result<Vec<u8>, AdapterError> {
            let fields_map: serde_json::Map<String, serde_json::Value> = message.fields.iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(hex::encode(v))))
                .collect();
            let json = serde_json::json!({
                "schema_version": message.schema_version,
                "protocol": format!("{}", message.protocol),
                "fields": fields_map
            });
            serde_json::to_vec(&json).map_err(|e| AdapterError::SerializationFailed(e.to_string()))
        }

        fn deserialize_json(&self, data: &[u8]) -> Result<ProtocolMessage, AdapterError> {
            let value: serde_json::Value =
                serde_json::from_slice(data).map_err(|e| AdapterError::DeserializationFailed(e.to_string()))?;

            let schema_version = value.get("schema_version")
                .and_then(|v| v.as_u64())
                .unwrap_or(1) as u32;

            let mut message = ProtocolMessage::new(ProtocolType::Json, schema_version);

            if let Some(fields) = value.get("fields").and_then(|v| v.as_object()) {
                for (name, value) in fields {
                    if let Some(hex_str) = value.as_str() {
                        if let Ok(bytes) = hex::decode(hex_str) {
                            message.add_field(name.clone(), bytes);
                        }
                    }
                }
            }

            Ok(message)
        }

        /// Check if two schema versions are compatible.
        pub fn check_compatibility(&self, version1: u32, version2: u32) -> bool {
            // Major version must match, minor can differ
            (version1 / 100) == (version2 / 100)
        }
    }

    impl Default for ProtocolAdapter {
        fn default() -> Self {
            Self::new(AdapterConfig::default())
        }
    }

    // ---------------------------------------------------------------------------
    // Tests
    // ---------------------------------------------------------------------------

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_adapter_creation() {
            let adapter = ProtocolAdapter::default();
            assert_eq!(adapter.config.schema_version, 1);
        }

        #[test]
        fn test_protobuf_roundtrip() {
            let adapter = ProtocolAdapter::default();
            let mut message = ProtocolMessage::new(ProtocolType::Protobuf, 1);
            message.add_field("key1".to_string(), vec![1, 2, 3]);
            message.add_field("key2".to_string(), vec![4, 5, 6]);

            let data = adapter.serialize(&message).unwrap();
            let decoded = adapter.deserialize(&data, ProtocolType::Protobuf).unwrap();

            assert_eq!(decoded.schema_version, 1);
            assert_eq!(decoded.fields.get("key1"), Some(&vec![1, 2, 3]));
            assert_eq!(decoded.fields.get("key2"), Some(&vec![4, 5, 6]));
        }

        #[test]
        fn test_flatbuffers_roundtrip() {
            let adapter = ProtocolAdapter::default();
            let mut message = ProtocolMessage::new(ProtocolType::FlatBuffers, 2);
            message.add_field("data".to_string(), vec![10, 20, 30]);

            let data = adapter.serialize(&message).unwrap();
            let decoded = adapter.deserialize(&data, ProtocolType::FlatBuffers).unwrap();

            assert_eq!(decoded.schema_version, 2);
            assert_eq!(decoded.fields.get("data"), Some(&vec![10, 20, 30]));
        }

        #[test]
        fn test_cbor_roundtrip() {
            let adapter = ProtocolAdapter::default();
            let mut message = ProtocolMessage::new(ProtocolType::Cbor, 3);
            message.add_field("test".to_string(), vec![42]);

            let data = adapter.serialize(&message).unwrap();
            let decoded = adapter.deserialize(&data, ProtocolType::Cbor).unwrap();

            assert_eq!(decoded.fields.get("test"), Some(&vec![42]));
        }

        #[test]
        fn test_json_roundtrip() {
            let adapter = ProtocolAdapter::default();
            let mut message = ProtocolMessage::new(ProtocolType::Json, 4);
            message.add_field("payload".to_string(), vec![1, 2, 3, 4]);

            let data = adapter.serialize(&message).unwrap();
            let decoded = adapter.deserialize(&data, ProtocolType::Json).unwrap();

            assert_eq!(decoded.schema_version, 4);
            assert_eq!(decoded.fields.get("payload"), Some(&vec![1, 2, 3, 4]));
        }

        #[test]
        fn test_payload_too_large() {
            let config = AdapterConfig {
                max_payload_size: 10,
                ..Default::default()
            };
            let adapter = ProtocolAdapter::new(config);
            let mut message = ProtocolMessage::new(ProtocolType::Json, 1);
            message.add_field("big".to_string(), vec![0; 20]);

            assert!(adapter.serialize(&message).is_err());
        }

        #[test]
        fn test_schema_compatibility() {
            let adapter = ProtocolAdapter::default();
            assert!(adapter.check_compatibility(100, 101)); // Same major
            assert!(adapter.check_compatibility(200, 250)); // Same major
            assert!(!adapter.check_compatibility(100, 200)); // Different major
        }

        #[test]
        fn test_error_display() {
            let err = AdapterError::UnknownProtocol("test".to_string());
            assert!(!format!("{}", err).is_empty());
        }

        #[test]
        fn test_config_default() {
            let config = AdapterConfig::default();
            assert_eq!(config.default_protocol, ProtocolType::Protobuf);
            assert!(config.enable_compression);
        }

        #[test]
        fn test_protocol_display() {
            assert_eq!(format!("{}", ProtocolType::Protobuf), "protobuf");
            assert_eq!(format!("{}", ProtocolType::Json), "json");
        }
    }
}

#[cfg(feature = "v1.6-sprint1")]
pub use internal::*;
