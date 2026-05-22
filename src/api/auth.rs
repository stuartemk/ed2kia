//! API Auth - Validación de firmas Ed25519 para endpoints de API v2
//!
//! Proporciona validación de firmas criptográficas en headers HTTP
//! para autenticar nodos en la red ed2kIA.
//!
//! # Feature Flag
//!
//! This module is gated behind `#[cfg(feature = "phase6-sprint2")]`.

#[cfg(feature = "phase6-sprint2")]
use axum::body::Body;
#[cfg(feature = "phase6-sprint2")]
use axum::extract::Request;
#[cfg(feature = "phase6-sprint2")]
use axum::http::StatusCode;
#[cfg(feature = "phase6-sprint2")]
use axum::middleware::Next;
#[cfg(feature = "phase6-sprint2")]
use axum::response::Response;
#[cfg(feature = "phase6-sprint2")]
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
#[cfg(feature = "phase6-sprint2")]
use tracing::{debug, info, warn};

// ---------------------------------------------------------------------------
// Public types (always available for serialization)
// ---------------------------------------------------------------------------

/// Error de autenticación
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthError {
    /// Tipo de error
    pub error_type: String,
    /// Mensaje descriptivo
    pub message: String,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Auth Error [{}]: {}", self.error_type, self.message)
    }
}

impl std::error::Error for AuthError {}

/// Resultado de validación de firma
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureValidationResult {
    /// ¿Firma válida?
    pub valid: bool,
    /// ID del nodo que firmó
    pub node_id: String,
    /// Timestamp de validación (Unix ms)
    pub timestamp: u64,
    /// Error si aplica
    pub error: Option<String>,
}

/// Configuración de autenticación
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Habilitar validación de firmas
    pub require_signature: bool,
    /// Timeout de firma en segundos (firmas más viejas se rechazan)
    pub signature_timeout_secs: u64,
    /// Lista de keys públicas autorizadas (hex encoded)
    pub authorized_keys: Vec<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            require_signature: false,
            signature_timeout_secs: 300, // 5 minutes
            authorized_keys: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// AuthValidator – core implementation (feature gated)
// ---------------------------------------------------------------------------

/// Validador de firmas Ed25519 para autenticación de nodos.
///
/// Verifica que las solicitudes API incluyan una firma válida
/// en el header `X-Node-Signature` usando la key pública del nodo.
#[cfg(feature = "phase6-sprint2")]
pub struct AuthValidator {
    config: AuthConfig,
    /// Cache de keys verificadas (node_id → VerifyingKey)
    key_cache: std::collections::HashMap<String, VerifyingKey>,
}

#[cfg(feature = "phase6-sprint2")]
impl AuthValidator {
    /// Crear nuevo validador con configuración
    pub fn new(config: AuthConfig) -> Self {
        info!(
            "AuthValidator created: require_signature={}, authorized_keys={}",
            config.require_signature,
            config.authorized_keys.len()
        );
        Self {
            config,
            key_cache: std::collections::HashMap::new(),
        }
    }

    /// Crear validador con configuración por defectos
    pub fn default_validator() -> Self {
        Self::new(AuthConfig::default())
    }

    /// Registrar key pública autorizada
    pub fn add_authorized_key(
        &mut self,
        node_id: String,
        public_key_hex: String,
    ) -> Result<(), AuthError> {
        let public_key_bytes = hex::decode(&public_key_hex).map_err(|e| AuthError {
            error_type: "invalid_hex".to_string(),
            message: format!("Failed to decode public key hex: {}", e),
        })?;

        if public_key_bytes.len() != 32 {
            return Err(AuthError {
                error_type: "invalid_key_length".to_string(),
                message: format!(
                    "Public key must be 32 bytes, got {}",
                    public_key_bytes.len()
                ),
            });
        }

        let public_key_array: [u8; 32] = public_key_bytes.try_into().map_err(|_| AuthError {
            error_type: "invalid_key_length".to_string(),
            message: "Public key must be exactly 32 bytes".to_string(),
        })?;

        let verifying_key = VerifyingKey::from_bytes(&public_key_array).map_err(|e| AuthError {
            error_type: "invalid_public_key".to_string(),
            message: format!("Failed to create verifying key: {}", e),
        })?;

        self.key_cache.insert(node_id.clone(), verifying_key);
        self.config.authorized_keys.push(public_key_hex);

        info!("Added authorized key for node: {}", node_id);
        Ok(())
    }

    /// Validar firma Ed25519 de un mensaje
    pub fn validate_signature(
        &self,
        node_id: &str,
        message: &[u8],
        signature_hex: &str,
    ) -> SignatureValidationResult {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // Decode signature
        let signature_bytes = match hex::decode(signature_hex) {
            Ok(bytes) => bytes,
            Err(e) => {
                return SignatureValidationResult {
                    valid: false,
                    node_id: node_id.to_string(),
                    timestamp,
                    error: Some(format!("Failed to decode signature hex: {}", e)),
                };
            }
        };

        if signature_bytes.len() != 64 {
            return SignatureValidationResult {
                valid: false,
                node_id: node_id.to_string(),
                timestamp,
                error: Some(format!(
                    "Signature must be 64 bytes, got {}",
                    signature_bytes.len()
                )),
            };
        }

        // Get verifying key
        let verifying_key = match self.key_cache.get(node_id) {
            Some(key) => key,
            None => {
                return SignatureValidationResult {
                    valid: false,
                    node_id: node_id.to_string(),
                    timestamp,
                    error: Some(format!("No authorized key found for node: {}", node_id)),
                };
            }
        };

        // Verify signature
        let signature = match Signature::from_slice(&signature_bytes) {
            Ok(sig) => sig,
            Err(e) => {
                return SignatureValidationResult {
                    valid: false,
                    node_id: node_id.to_string(),
                    timestamp,
                    error: Some(format!("Failed to create signature: {}", e)),
                };
            }
        };

        match verifying_key.verify(message, &signature) {
            Ok(()) => {
                debug!("Signature valid for node: {}", node_id);
                SignatureValidationResult {
                    valid: true,
                    node_id: node_id.to_string(),
                    timestamp,
                    error: None,
                }
            }
            Err(e) => {
                warn!("Signature verification failed for node {}: {}", node_id, e);
                SignatureValidationResult {
                    valid: false,
                    node_id: node_id.to_string(),
                    timestamp,
                    error: Some(format!("Signature verification failed: {}", e)),
                }
            }
        }
    }

    /// Middleware de autenticación para Axum
    pub async fn auth_middleware(
        &self,
        request: Request,
        next: Next,
    ) -> Result<Response, StatusCode> {
        if !self.config.require_signature {
            return Ok(next.run(request).await);
        }

        // Decompose request first to avoid move issues
        let (parts, body) = request.into_parts();
        let method = parts.method.to_string();
        let path = parts.uri.path().to_string();

        // Extract headers from parts
        let node_id = parts
            .headers
            .get("X-Node-ID")
            .and_then(|id| id.to_str().ok())
            .map(|s| s.to_string())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        let signature = parts
            .headers
            .get("X-Node-Signature")
            .and_then(|sig| sig.to_str().ok())
            .map(|s| s.to_string())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        let _timestamp: Option<u64> = parts
            .headers
            .get("X-Timestamp")
            .and_then(|ts| ts.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());

        // Read body bytes
        let body_bytes = axum::body::to_bytes(body, 1024 * 1024)
            .await
            .map(|b| b.to_vec())
            .unwrap_or_default();

        let message = format!(
            "{}:{}:{}",
            method,
            path,
            String::from_utf8_lossy(&body_bytes)
        );

        // Validate signature
        let result = self.validate_signature(&node_id, message.as_bytes(), &signature);

        if !result.valid {
            warn!("Auth failed for node {}: {:?}", node_id, result.error);
            return Err(StatusCode::FORBIDDEN);
        }

        // Reconstruct request with original parts and new body
        let reconstructed = Request::from_parts(parts, Body::from(body_bytes));

        Ok(next.run(reconstructed).await)
    }

    /// Verificar si un nodo está autorizado
    pub fn is_node_authorized(&self, node_id: &str) -> bool {
        self.key_cache.contains_key(node_id)
    }

    /// Obtener cantidad de keys autorizadas
    pub fn authorized_key_count(&self) -> usize {
        self.key_cache.len()
    }

    /// Limpiar cache de keys
    pub fn clear_cache(&mut self) {
        let count = self.key_cache.len();
        self.key_cache.clear();
        if count > 0 {
            info!("Cleared {} cached keys", count);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_error_display() {
        let err = AuthError {
            error_type: "invalid_signature".to_string(),
            message: "Signature does not match".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("invalid_signature"));
        assert!(msg.contains("Signature does not match"));
    }

    #[test]
    fn test_auth_config_default() {
        let config = AuthConfig::default();
        assert!(!config.require_signature);
        assert_eq!(config.signature_timeout_secs, 300);
        assert!(config.authorized_keys.is_empty());
    }

    #[test]
    fn test_signature_validation_result() {
        let result = SignatureValidationResult {
            valid: true,
            node_id: "test_node".to_string(),
            timestamp: 1234567890,
            error: None,
        };
        assert!(result.valid);
        assert!(result.error.is_none());
    }

    #[cfg(feature = "phase6-sprint2")]
    mod sprint2_tests {
        use super::*;
        use ed25519_dalek::{Signer, SigningKey};

        #[test]
        fn test_validator_creation() {
            let validator = AuthValidator::default_validator();
            assert_eq!(validator.authorized_key_count(), 0);
        }

        #[test]
        fn test_add_authorized_key() {
            let mut validator = AuthValidator::default_validator();
            let signing_key = SigningKey::from_bytes(&[0u8; 32]);
            let public_key = signing_key.verifying_key();
            let public_key_hex = hex::encode(public_key.to_bytes());

            let result = validator.add_authorized_key("test_node".to_string(), public_key_hex);
            assert!(result.is_ok());
            assert_eq!(validator.authorized_key_count(), 1);
            assert!(validator.is_node_authorized("test_node"));
        }

        #[test]
        fn test_add_invalid_key_length() {
            let mut validator = AuthValidator::default_validator();
            let result = validator.add_authorized_key("bad_node".to_string(), "ab".to_string());
            assert!(result.is_err());
            assert!(result.unwrap_err().error_type == "invalid_key_length");
        }

        #[test]
        fn test_add_invalid_hex() {
            let mut validator = AuthValidator::default_validator();
            let result =
                validator.add_authorized_key("bad_node".to_string(), "not_hex!".to_string());
            assert!(result.is_err());
            assert!(result.unwrap_err().error_type == "invalid_hex");
        }

        #[test]
        fn test_validate_valid_signature() {
            let mut validator = AuthValidator::default_validator();

            // Create key pair
            let signing_key = SigningKey::from_bytes(&[0u8; 32]);
            let public_key = signing_key.verifying_key();
            let public_key_hex = hex::encode(public_key.to_bytes());
            validator
                .add_authorized_key("test_node".to_string(), public_key_hex)
                .unwrap();

            // Sign a message
            let message = b"test message";
            let signature = signing_key.sign(message);
            let signature_hex = hex::encode(signature.to_bytes());

            // Validate
            let result = validator.validate_signature("test_node", message, &signature_hex);
            assert!(result.valid);
            assert!(result.error.is_none());
        }

        #[test]
        fn test_validate_invalid_signature() {
            let mut validator = AuthValidator::default_validator();

            let signing_key = SigningKey::from_bytes(&[0u8; 32]);
            let public_key = signing_key.verifying_key();
            let public_key_hex = hex::encode(public_key.to_bytes());
            validator
                .add_authorized_key("test_node".to_string(), public_key_hex)
                .unwrap();

            // Use wrong signature
            let wrong_signature = SigningKey::from_bytes(&[1u8; 32]).sign(b"wrong");
            let signature_hex = hex::encode(wrong_signature.to_bytes());

            let result = validator.validate_signature("test_node", b"test", &signature_hex);
            assert!(!result.valid);
            assert!(result.error.is_some());
        }

        #[test]
        fn test_validate_unknown_node() {
            let validator = AuthValidator::default_validator();
            let result = validator.validate_signature("unknown", b"test", "ab".repeat(64).as_str());
            assert!(!result.valid);
            assert!(result.error.unwrap().contains("No authorized key"));
        }

        #[test]
        fn test_validate_invalid_hex_signature() {
            let validator = AuthValidator::default_validator();
            let result = validator.validate_signature("node", b"test", "not_hex!");
            assert!(!result.valid);
            assert!(result.error.is_some());
        }

        #[test]
        fn test_clear_cache() {
            let mut validator = AuthValidator::default_validator();
            let signing_key = SigningKey::from_bytes(&[0u8; 32]);
            let public_key = signing_key.verifying_key();
            let public_key_hex = hex::encode(public_key.to_bytes());
            validator
                .add_authorized_key("node1".to_string(), public_key_hex)
                .unwrap();

            assert_eq!(validator.authorized_key_count(), 1);
            validator.clear_cache();
            assert_eq!(validator.authorized_key_count(), 0);
        }

        #[test]
        fn test_is_node_authorized() {
            let validator = AuthValidator::default_validator();
            assert!(!validator.is_node_authorized("unknown"));
        }
    }
}
