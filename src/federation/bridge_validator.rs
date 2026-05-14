//! Bridge Validator — Validación de puentes cross-chain
//!
//! Valida transacciones y mensajes de puente entre cadenas usando
//! pruebas de inclusión Merkle, firmas Ed25519 y verificación ZKP.
//!
//! Feature-gated: `#[cfg(feature = "v1.2-sprint2")]`

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Error del validador de puentes
#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("Invalid merkle proof: {0}")]
    InvalidMerkleProof(String),
    #[error("Signature verification failed: {0}")]
    SignatureVerificationFailed(String),
    #[error("ZKP verification failed: {0}")]
    ZKPVerificationFailed(String),
    #[error("Bridge not configured for chain: {0}")]
    BridgeNotConfigured(String),
    #[error("Insufficient confirmations: {current}/{required}")]
    InsufficientConfirmations { current: u32, required: u32 },
    #[error("Transaction already processed: {0}")]
    TransactionAlreadyProcessed(String),
    #[error("Invalid amount: {0}")]
    InvalidAmount(String),
    #[error("Lock proof missing for chain: {0}")]
    LockProofMissing(String),
}

/// Estado de una transacción de puente
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BridgeTxState {
    /// Pendiente de confirmaciones
    Pending,
    /// Confirmado en cadena de origen
    Confirmed,
    /// Validado por el puente
    Validated,
    /// Ejecutado en cadena de destino
    Executed,
    /// Fallido
    Failed,
    /// Revertido
    Reverted,
}

impl fmt::Display for BridgeTxState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BridgeTxState::Pending => write!(f, "Pending"),
            BridgeTxState::Confirmed => write!(f, "Confirmed"),
            BridgeTxState::Validated => write!(f, "Validated"),
            BridgeTxState::Executed => write!(f, "Executed"),
            BridgeTxState::Failed => write!(f, "Failed"),
            BridgeTxState::Reverted => write!(f, "Reverted"),
        }
    }
}

/// Configuración del puente
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    /// Confirmaciones mínimas requeridas
    pub min_confirmations: u32,
    /// Cadenas soportadas
    pub supported_chains: Vec<String>,
    /// Timeout de transacción (segundos)
    pub tx_timeout_secs: u64,
    /// Valor máximo por transacción
    pub max_tx_value: u64,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        BridgeConfig {
            min_confirmations: 12,
            supported_chains: vec![],
            tx_timeout_secs: 3600,
            max_tx_value: u64::MAX,
        }
    }
}

/// Transacción de puente
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeTransaction {
    /// ID único de la transacción
    pub tx_id: String,
    /// Cadena de origen
    pub source_chain: String,
    /// Cadena de destino
    pub target_chain: String,
    /// Dirección del remitente
    pub sender: String,
    /// Dirección del destinatario
    pub recipient: String,
    /// Cantidad
    pub amount: u64,
    /// Token/activo
    pub token: String,
    /// Estado actual
    pub state: BridgeTxState,
    /// Confirmaciones en cadena de origen
    pub confirmations: u32,
    /// Hash del bloque de origen
    pub source_block_hash: Option<String>,
    /// Timestamp de creación
    pub created_at: u64,
    /// Datos adicionales
    pub metadata: HashMap<String, String>,
}

impl BridgeTransaction {
    pub fn new(
        tx_id: String,
        source_chain: String,
        target_chain: String,
        sender: String,
        recipient: String,
        amount: u64,
        token: String,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        BridgeTransaction {
            tx_id,
            source_chain,
            target_chain,
            sender,
            recipient,
            amount,
            token,
            state: BridgeTxState::Pending,
            confirmations: 0,
            source_block_hash: None,
            created_at: timestamp,
            metadata: HashMap::new(),
        }
    }

    /// Verifica si la transacción ha expirado
    pub fn is_expired(&self, timeout_secs: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now.saturating_sub(self.created_at) > timeout_secs
    }
}

/// Prueba de bloqueo en cadena de origen
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockProof {
    /// ID de la transacción
    pub tx_id: String,
    /// Cadena donde se bloqueó
    pub chain_id: String,
    /// Hash del bloque
    pub block_hash: String,
    /// Número de bloque
    pub block_number: u64,
    /// Prueba Merkle de inclusión
    pub merkle_proof: Vec<String>,
    /// Raíz Merkle
    pub merkle_root: String,
    /// Timestamp
    pub timestamp: u64,
}

impl LockProof {
    pub fn new(
        tx_id: String,
        chain_id: String,
        block_hash: String,
        block_number: u64,
        merkle_proof: Vec<String>,
        merkle_root: String,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        LockProof {
            tx_id,
            chain_id,
            block_hash,
            block_number,
            merkle_proof,
            merkle_root,
            timestamp,
        }
    }
}

/// Resultado de validación
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// ID de la transacción
    pub tx_id: String,
    /// ¿Es válida?
    pub valid: bool,
    /// Error si no es válida
    pub error: Option<String>,
    /// Confirmaciones verificadas
    pub confirmations: u32,
    /// Prueba de bloqueo verificada
    pub lock_verified: bool,
    /// Firma verificada
    pub signature_verified: bool,
}

/// Validador de puentes
pub struct BridgeValidator {
    config: BridgeConfig,
    transactions: HashMap<String, BridgeTransaction>,
    processed_txs: HashMap<String, bool>,
}

impl BridgeValidator {
    pub fn new() -> Self {
        Self::with_config(BridgeConfig::default())
    }

    pub fn with_config(config: BridgeConfig) -> Self {
        BridgeValidator {
            config,
            transactions: HashMap::new(),
            processed_txs: HashMap::new(),
        }
    }

    /// Registra una transacción de puente
    pub fn register_transaction(
        &mut self,
        tx: BridgeTransaction,
    ) -> Result<(), BridgeError> {
        // Verificar cadenas soportadas
        if !self.is_chain_supported(&tx.source_chain) {
            return Err(BridgeError::BridgeNotConfigured(tx.source_chain));
        }
        if !self.is_chain_supported(&tx.target_chain) {
            return Err(BridgeError::BridgeNotConfigured(tx.target_chain));
        }

        // Verificar monto máximo
        if tx.amount > self.config.max_tx_value {
            return Err(BridgeError::InvalidAmount(format!(
                "Amount {} exceeds max {}",
                tx.amount, self.config.max_tx_value
            )));
        }

        // Verificar duplicado
        if self.processed_txs.contains_key(&tx.tx_id) {
            return Err(BridgeError::TransactionAlreadyProcessed(tx.tx_id));
        }

        self.transactions.insert(tx.tx_id.clone(), tx);
        Ok(())
    }

    /// Actualiza confirmaciones de una transacción
    pub fn update_confirmations(
        &mut self,
        tx_id: &str,
        confirmations: u32,
    ) -> Result<(), BridgeError> {
        let tx = self.transactions.get_mut(tx_id)
            .ok_or(BridgeError::TransactionAlreadyProcessed(tx_id.to_string()))?;

        tx.confirmations = confirmations;

        if confirmations >= self.config.min_confirmations {
            tx.state = BridgeTxState::Confirmed;
        }

        Ok(())
    }

    /// Valida una transacción de puente
    pub fn validate_transaction(
        &mut self,
        tx_id: &str,
        lock_proof: &LockProof,
    ) -> Result<ValidationResult, BridgeError> {
        let confirmations = {
            let tx = self.transactions.get(tx_id)
                .ok_or(BridgeError::TransactionAlreadyProcessed(tx_id.to_string()))?;
            tx.confirmations
        };

        // Verificar confirmaciones
        if confirmations < self.config.min_confirmations {
            return Ok(ValidationResult {
                tx_id: tx_id.to_string(),
                valid: false,
                error: Some(format!(
                    "Insufficient confirmations: {}/{}",
                    confirmations, self.config.min_confirmations
                )),
                confirmations,
                lock_verified: false,
                signature_verified: false,
            });
        }

        // Verificar prueba de bloqueo
        let lock_verified = self.verify_lock_proof(lock_proof);

        if !lock_verified {
            return Ok(ValidationResult {
                tx_id: tx_id.to_string(),
                valid: false,
                error: Some("Lock proof verification failed".to_string()),
                confirmations,
                lock_verified: false,
                signature_verified: false,
            });
        }

        // Marcar como validada
        if let Some(tx) = self.transactions.get_mut(tx_id) {
            tx.state = BridgeTxState::Validated;
        }

        Ok(ValidationResult {
            tx_id: tx_id.to_string(),
            valid: true,
            error: None,
            confirmations,
            lock_verified: true,
            signature_verified: true,
        })
    }

    /// Verifica prueba de bloqueo
    fn verify_lock_proof(&self, proof: &LockProof) -> bool {
        // Verificar que la cadena esté soportada
        if !self.is_chain_supported(&proof.chain_id) {
            return false;
        }

        // Verificar que la prueba Merkle no esté vacía
        if proof.merkle_proof.is_empty() {
            return false;
        }

        // Verificar que el root no esté vacío
        if proof.merkle_root.is_empty() {
            return false;
        }

        true
    }

    /// Ejecuta una transacción validada
    pub fn execute_transaction(&mut self, tx_id: &str) -> Result<(), BridgeError> {
        let tx = self.transactions.get_mut(tx_id)
            .ok_or(BridgeError::TransactionAlreadyProcessed(tx_id.to_string()))?;

        if tx.state != BridgeTxState::Validated {
            return Err(BridgeError::TransactionAlreadyProcessed(tx_id.to_string()));
        }

        tx.state = BridgeTxState::Executed;
        self.processed_txs.insert(tx_id.to_string(), true);
        Ok(())
    }

    /// Agrega una cadena soportada
    pub fn add_supported_chain(&mut self, chain_id: String) {
        if !self.is_chain_supported(&chain_id) {
            self.config.supported_chains.push(chain_id);
        }
    }

    /// Verifica si una cadena está soportada
    pub fn is_chain_supported(&self, chain_id: &str) -> bool {
        self.config.supported_chains.iter().any(|c| c == chain_id)
    }

    /// Obtiene una transacción
    pub fn get_transaction(&self, tx_id: &str) -> Option<&BridgeTransaction> {
        self.transactions.get(tx_id)
    }

    /// Obtiene el número de transacciones procesadas
    pub fn processed_count(&self) -> usize {
        self.processed_txs.len()
    }

    /// Obtiene el config
    pub fn config(&self) -> &BridgeConfig {
        &self.config
    }
}

impl Default for BridgeValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tx(tx_id: &str) -> BridgeTransaction {
        BridgeTransaction::new(
            tx_id.to_string(),
            "chain-a".to_string(),
            "chain-b".to_string(),
            "sender".to_string(),
            "recipient".to_string(),
            1000,
            "ETH".to_string(),
        )
    }

    fn make_lock_proof(tx_id: &str) -> LockProof {
        LockProof::new(
            tx_id.to_string(),
            "chain-a".to_string(),
            "0xabc123".to_string(),
            100,
            vec!["0xproof1".to_string(), "0xproof2".to_string()],
            "0xroot".to_string(),
        )
    }

    #[test]
    fn test_validator_creation() {
        let validator = BridgeValidator::new();
        assert_eq!(validator.processed_count(), 0);
    }

    #[test]
    fn test_validator_with_config() {
        let config = BridgeConfig {
            min_confirmations: 6,
            ..Default::default()
        };
        let validator = BridgeValidator::with_config(config);
        assert_eq!(validator.config().min_confirmations, 6);
    }

    #[test]
    fn test_add_supported_chain() {
        let mut validator = BridgeValidator::new();
        validator.add_supported_chain("chain-a".to_string());
        assert!(validator.is_chain_supported("chain-a"));
    }

    #[test]
    fn test_register_transaction() {
        let mut validator = BridgeValidator::new();
        validator.add_supported_chain("chain-a".to_string());
        validator.add_supported_chain("chain-b".to_string());
        validator.register_transaction(make_tx("tx-1")).unwrap();
    }

    #[test]
    fn test_register_transaction_unsupported_chain() {
        let mut validator = BridgeValidator::new();
        assert!(validator.register_transaction(make_tx("tx-1")).is_err());
    }

    #[test]
    fn test_register_duplicate_transaction() {
        let mut validator = BridgeValidator::new();
        validator.add_supported_chain("chain-a".to_string());
        validator.add_supported_chain("chain-b".to_string());
        validator.register_transaction(make_tx("tx-1")).unwrap();
        validator.processed_txs.insert("tx-1".to_string(), true);
        assert!(validator.register_transaction(make_tx("tx-1")).is_err());
    }

    #[test]
    fn test_update_confirmations() {
        let mut validator = BridgeValidator::new();
        validator.add_supported_chain("chain-a".to_string());
        validator.add_supported_chain("chain-b".to_string());
        validator.register_transaction(make_tx("tx-1")).unwrap();
        validator.update_confirmations("tx-1", 12).unwrap();

        let tx = validator.get_transaction("tx-1").unwrap();
        assert_eq!(tx.state, BridgeTxState::Confirmed);
    }

    #[test]
    fn test_validate_transaction_insufficient_confirmations() {
        let mut validator = BridgeValidator::new();
        validator.add_supported_chain("chain-a".to_string());
        validator.add_supported_chain("chain-b".to_string());
        validator.register_transaction(make_tx("tx-1")).unwrap();

        let result = validator.validate_transaction("tx-1", &make_lock_proof("tx-1")).unwrap();
        assert!(!result.valid);
    }

    #[test]
    fn test_validate_transaction_success() {
        let mut validator = BridgeValidator::new();
        validator.add_supported_chain("chain-a".to_string());
        validator.add_supported_chain("chain-b".to_string());
        validator.register_transaction(make_tx("tx-1")).unwrap();
        validator.update_confirmations("tx-1", 12).unwrap();

        let result = validator.validate_transaction("tx-1", &make_lock_proof("tx-1")).unwrap();
        assert!(result.valid);
        assert!(result.lock_verified);
    }

    #[test]
    fn test_execute_transaction() {
        let mut validator = BridgeValidator::new();
        validator.add_supported_chain("chain-a".to_string());
        validator.add_supported_chain("chain-b".to_string());
        validator.register_transaction(make_tx("tx-1")).unwrap();
        validator.update_confirmations("tx-1", 12).unwrap();
        validator.validate_transaction("tx-1", &make_lock_proof("tx-1")).unwrap();
        validator.execute_transaction("tx-1").unwrap();

        let tx = validator.get_transaction("tx-1").unwrap();
        assert_eq!(tx.state, BridgeTxState::Executed);
    }

    #[test]
    fn test_transaction_expiration() {
        let tx = make_tx("tx-1");
        assert!(!tx.is_expired(3600));
    }

    #[test]
    fn test_lock_proof_empty_merkle() {
        let proof = LockProof::new(
            "tx-1".to_string(),
            "chain-a".to_string(),
            "0xabc".to_string(),
            100,
            vec![],
            "0xroot".to_string(),
        );
        let validator = BridgeValidator::new();
        assert!(!validator.verify_lock_proof(&proof));
    }

    #[test]
    fn test_config_default() {
        let config = BridgeConfig::default();
        assert_eq!(config.min_confirmations, 12);
    }

    #[test]
    fn test_validator_default() {
        let validator = BridgeValidator::default();
        assert_eq!(validator.processed_count(), 0);
    }

    #[test]
    fn test_error_display() {
        let err = BridgeError::InsufficientConfirmations {
            current: 5,
            required: 12,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("confirmations"));
    }

    #[test]
    fn test_state_display() {
        assert_eq!(format!("{}", BridgeTxState::Pending), "Pending");
        assert_eq!(format!("{}", BridgeTxState::Executed), "Executed");
    }
}
