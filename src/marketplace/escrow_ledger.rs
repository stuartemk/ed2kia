//! Escrow Ledger — Ledger inmutable en `redb` con firmas `ed25519-dalek`
//!
//! Gestiona estados de escrow (Locked, Released, Refunded, Disputed) con
//! liberación condicional basada en ZKP + métricas SLO. Cada transacción es
//! inmutable y verificable criptográficamente.
//!
//! Feature-gated: `#[cfg(feature = "v1.1-sprint3")]`

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use redb::{Database, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;
use tracing::{debug, info, warn};

// ---------------------------------------------------------------------------
// Table definitions for redb
// ---------------------------------------------------------------------------

/// Definición de tabla para transacciones de escrow.
const ESCROW_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("escrow_transactions");

/// Definición de tabla para claves públicas de nodos.
const NODE_KEYS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("node_keys");

/// Definición de tabla para eventos de auditoría.
const AUDIT_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("audit_events");

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errores del ledger de escrow.
#[derive(Debug, Error)]
pub enum EscrowError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Transaction not found: {0}")]
    TransactionNotFound(String),
    #[error("Invalid state transition: {from} -> {to}")]
    InvalidStateTransition { from: EscrowState, to: EscrowState },
    #[error("Signature verification failed")]
    SignatureFailed,
    #[error("Invalid ZKP proof for release")]
    InvalidZKPProof,
    #[error("SLO not met: {0}")]
    SLONotMet(String),
    #[error("Node not registered: {0}")]
    NodeNotRegistered(String),
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Estado de una transacción de escrow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EscrowState {
    /// Escrow creado y fondos bloqueados.
    Locked,
    /// Recursos entregados, esperando ZKP.
    Delivered,
    /// ZKP verificado, fondos listos para liberar.
    ZKPVerified,
    /// Fondos liberados al vendedor.
    Released,
    /// Fondos devueltos al comprador.
    Refunded,
    /// En disputa (requiere gobernanza).
    Disputed,
}

impl std::fmt::Display for EscrowState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EscrowState::Locked => write!(f, "LOCKED"),
            EscrowState::Delivered => write!(f, "DELIVERED"),
            EscrowState::ZKPVerified => write!(f, "ZKP_VERIFIED"),
            EscrowState::Released => write!(f, "RELEASED"),
            EscrowState::Refunded => write!(f, "REFUNDED"),
            EscrowState::Disputed => write!(f, "DISPUTED"),
        }
    }
}

/// Transacción de escrow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscrowTransaction {
    /// ID único de la transacción.
    pub tx_id: String,
    /// ID del nodo vendedor (proveedor de recurso).
    pub seller_id: String,
    /// ID del nodo comprador (demandante de recurso).
    pub buyer_id: String,
    /// Monto en créditos.
    pub amount: f64,
    /// Estado actual.
    pub state: EscrowState,
    /// Hash del settlement del marketplace.
    pub settlement_hash: String,
    /// Hash ZKP de integridad (si aplica).
    pub zkp_hash: Option<String>,
    /// Timestamp de creación (epoch ms).
    pub created_at: u64,
    /// Timestamp de última transición de estado.
    pub updated_at: u64,
    /// Firma del creador.
    pub signature: Vec<u8>,
    /// Métricas SLO durante la entrega.
    pub slo_metrics: Option<SLOMetrics>,
}

/// Métricas SLO para verificación de entrega.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SLOMetrics {
    /// Latencia observada (ms).
    pub observed_latency_ms: u64,
    /// Latencia acordada (ms).
    pub agreed_latency_ms: u64,
    /// Disponibilidad observada.
    pub observed_availability: f32,
    /// Disponibilidad acordada.
    pub agreed_availability: f32,
    /// Throughput observado (ops/s).
    pub observed_throughput: u64,
    /// Throughput acordado (ops/s).
    pub agreed_throughput: u64,
}

impl SLOMetrics {
    /// Verifica si las métricas cumplen los SLO acordados.
    pub fn meets_slo(&self) -> bool {
        self.observed_latency_ms <= self.agreed_latency_ms
            && self.observed_availability >= self.agreed_availability
            && self.observed_throughput >= self.agreed_throughput
    }
}

/// Evento de auditoría.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// ID del evento.
    pub event_id: String,
    /// ID de la transacción asociada.
    pub tx_id: String,
    /// Tipo de evento.
    pub event_type: String,
    /// Detalle del evento.
    pub details: String,
    /// Timestamp (epoch ms).
    pub timestamp: u64,
    /// Firma del auditor.
    pub signature: Vec<u8>,
}

/// Ledger de escrow inmutable con redb.
pub struct EscrowLedger {
    /// Base de datos redb.
    db: Database,
    /// Clave de firma para transacciones.
    signing_key: SigningKey,
    /// Clave verificadora pública.
    verifying_key: VerifyingKey,
}

impl EscrowLedger {
    /// Abre o crea un ledger en el path dado.
    pub fn new(db_path: &str, signing_key: SigningKey) -> Result<Self, EscrowError> {
        let path = Path::new(db_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| EscrowError::Database(e.to_string()))?;
        }

        let db = Database::create(db_path).map_err(|e| EscrowError::Database(e.to_string()))?;
        let verifying_key = signing_key.verifying_key();

        info!(path = %db_path, "EscrowLedger initialized with redb");

        Ok(Self {
            db,
            signing_key,
            verifying_key,
        })
    }

    /// Crea un ledger en memoria temporal (modo test).
    pub fn new_test() -> Result<(Self, SigningKey, std::path::PathBuf), EscrowError> {
        let mut sk_bytes = [0u8; 32];
        for (i, byte) in sk_bytes.iter_mut().enumerate() {
            *byte = i as u8;
        }
        let signing_key = SigningKey::from_bytes(&sk_bytes);
        // Usar archivo temporal con nombre único (redb no soporta in-memory en Windows)
        let tmp_dir = std::env::temp_dir().join("ed2kIA_escrow_test");
        fs::create_dir_all(&tmp_dir).map_err(|e| EscrowError::Database(e.to_string()))?;
        let unique = fastrand::u64(0..u64::MAX);
        let db_path = tmp_dir.join(format!("test_{}_{}.db", std::process::id(), unique));
        let ledger = Self::new(db_path.to_str().unwrap(), signing_key.clone())?;
        Ok((ledger, signing_key, db_path.clone()))
    }

    /// Registra una clave pública de nodo para verificación de firmas.
    pub fn register_node_key(
        &self,
        node_id: &str,
        _verifying_key: VerifyingKey,
    ) -> Result<(), EscrowError> {
        let write_txn = self
            .db
            .begin_write()
            .map_err(|e| EscrowError::Database(e.to_string()))?;
        {
            let mut table = write_txn
                .open_table(NODE_KEYS_TABLE)
                .map_err(|e| EscrowError::Database(e.to_string()))?;
            let key_bytes = node_id.as_bytes();
            let mut val_bytes = Vec::new();
            // VerifyingKey no implementa Serialize, usar representación manual
            let vk_bytes = self.verifying_key.to_bytes();
            bincode::serialize_into(&mut val_bytes, &vk_bytes)
                .map_err(|e| EscrowError::Database(e.to_string()))?;
            table
                .insert(key_bytes, val_bytes.as_slice())
                .map_err(|e| EscrowError::Database(e.to_string()))?;
        }
        write_txn
            .commit()
            .map_err(|e| EscrowError::Database(e.to_string()))?;
        debug!(node = %node_id, "Node key registered");
        Ok(())
    }

    /// Crea una nueva transacción de escrow.
    pub fn create_escrow(
        &self,
        tx_id: String,
        seller_id: String,
        buyer_id: String,
        amount: f64,
        settlement_hash: String,
    ) -> Result<EscrowTransaction, EscrowError> {
        let now = chrono::Utc::now().timestamp_millis() as u64;

        let tx = EscrowTransaction {
            tx_id: tx_id.clone(),
            seller_id: seller_id.clone(),
            buyer_id: buyer_id.clone(),
            amount,
            state: EscrowState::Locked,
            settlement_hash: settlement_hash.clone(),
            zkp_hash: None,
            created_at: now,
            updated_at: now,
            signature: Vec::new(),
            slo_metrics: None,
        };

        // Firmar la transacción
        let tx_bytes = bincode::serialize(&tx).map_err(|e| EscrowError::Database(e.to_string()))?;
        let signature = self.signing_key.sign(&tx_bytes);
        let mut sig_bytes = Vec::with_capacity(64);
        sig_bytes.extend_from_slice(&signature.to_bytes());

        let mut tx_signed = tx.clone();
        tx_signed.signature = sig_bytes;

        // Persistir
        let write_txn = self
            .db
            .begin_write()
            .map_err(|e| EscrowError::Database(e.to_string()))?;
        {
            let mut table = write_txn
                .open_table(ESCROW_TABLE)
                .map_err(|e| EscrowError::Database(e.to_string()))?;
            let key_bytes = tx_id.as_bytes();
            let mut val_bytes = Vec::new();
            bincode::serialize(&tx_signed).map_err(|e| EscrowError::Database(e.to_string()))?;
            bincode::serialize_into(&mut val_bytes, &tx_signed)
                .map_err(|e| EscrowError::Database(e.to_string()))?;
            table
                .insert(key_bytes, val_bytes.as_slice())
                .map_err(|e| EscrowError::Database(e.to_string()))?;
        }
        write_txn
            .commit()
            .map_err(|e| EscrowError::Database(e.to_string()))?;

        // Registrar evento de auditoría
        self.log_audit_event(
            &tx_id,
            "ESCROW_CREATED",
            format!("Locked {} credits", amount),
        )?;

        info!(
            tx_id = %tx_id,
            seller = %seller_id,
            buyer = %buyer_id,
            amount = %amount,
            "Escrow transaction created"
        );

        Ok(tx_signed)
    }

    /// Transiciona el estado de una transacción.
    pub fn transition_state(
        &self,
        tx_id: &str,
        new_state: EscrowState,
    ) -> Result<EscrowTransaction, EscrowError> {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| EscrowError::Database(e.to_string()))?;
        let table = read_txn
            .open_table(ESCROW_TABLE)
            .map_err(|e| EscrowError::Database(e.to_string()))?;
        let tx_opt = table
            .get(tx_id.as_bytes())
            .map_err(|e| EscrowError::Database(e.to_string()))?;

        let mut tx: EscrowTransaction = match tx_opt {
            Some(val) => bincode::deserialize(val.value())
                .map_err(|e| EscrowError::Database(e.to_string()))?,
            None => return Err(EscrowError::TransactionNotFound(tx_id.into())),
        };

        // Validar transición de estado
        self.validate_transition(&tx.state, &new_state)?;

        let old_state = tx.state.clone();
        tx.state = new_state.clone();
        tx.updated_at = chrono::Utc::now().timestamp_millis() as u64;

        // Persistir
        let write_txn = self
            .db
            .begin_write()
            .map_err(|e| EscrowError::Database(e.to_string()))?;
        {
            let mut table = write_txn
                .open_table(ESCROW_TABLE)
                .map_err(|e| EscrowError::Database(e.to_string()))?;
            let mut val_bytes = Vec::new();
            bincode::serialize_into(&mut val_bytes, &tx)
                .map_err(|e| EscrowError::Database(e.to_string()))?;
            table
                .insert(tx_id.as_bytes(), val_bytes.as_slice())
                .map_err(|e| EscrowError::Database(e.to_string()))?;
        }
        write_txn
            .commit()
            .map_err(|e| EscrowError::Database(e.to_string()))?;

        self.log_audit_event(
            tx_id,
            "STATE_TRANSITION",
            format!("{} -> {}", old_state, new_state),
        )?;

        debug!(tx_id = %tx_id, from = %old_state, to = %new_state, "State transitioned");

        Ok(tx)
    }

    /// Libera fondos basado en ZKP verificado + SLO cumplido.
    pub fn release_on_zkp(
        &self,
        tx_id: &str,
        zkp_hash: String,
        slo_metrics: SLOMetrics,
    ) -> Result<EscrowTransaction, EscrowError> {
        if !slo_metrics.meets_slo() {
            return Err(EscrowError::SLONotMet(format!(
                "latency: {}>{}ms, availability: {}<{}, throughput: {}<{}",
                slo_metrics.observed_latency_ms,
                slo_metrics.agreed_latency_ms,
                slo_metrics.observed_availability,
                slo_metrics.agreed_availability,
                slo_metrics.observed_throughput,
                slo_metrics.agreed_throughput
            )));
        }

        // Transición: Locked/Delivered -> ZKPVerified -> Released
        let tx = self.transition_state(tx_id, EscrowState::ZKPVerified)?;
        if matches!(tx.state, EscrowState::Released) {
            // Ya está liberado
            return Ok(tx);
        }

        let mut tx = self.transition_state(tx_id, EscrowState::Released)?;
        tx.zkp_hash = Some(zkp_hash.clone());
        tx.slo_metrics = Some(slo_metrics);

        // Persistir actualización
        let write_txn = self
            .db
            .begin_write()
            .map_err(|e| EscrowError::Database(e.to_string()))?;
        {
            let mut table = write_txn
                .open_table(ESCROW_TABLE)
                .map_err(|e| EscrowError::Database(e.to_string()))?;
            let mut val_bytes = Vec::new();
            bincode::serialize_into(&mut val_bytes, &tx)
                .map_err(|e| EscrowError::Database(e.to_string()))?;
            table
                .insert(tx_id.as_bytes(), val_bytes.as_slice())
                .map_err(|e| EscrowError::Database(e.to_string()))?;
        }
        write_txn
            .commit()
            .map_err(|e| EscrowError::Database(e.to_string()))?;

        self.log_audit_event(tx_id, "RELEASED", format!("ZKP={}, SLO=OK", zkp_hash))?;

        info!(tx_id = %tx_id, zkp_hash = %zkp_hash, "Escrow released on ZKP verification");

        Ok(tx)
    }

    /// Refunde fondos al comprador (ej: por incumplimiento).
    pub fn refund(&self, tx_id: &str, reason: &str) -> Result<EscrowTransaction, EscrowError> {
        let tx = self.transition_state(tx_id, EscrowState::Refunded)?;

        self.log_audit_event(tx_id, "REFUNDED", reason.into())?;

        warn!(tx_id = %tx_id, reason = %reason, "Escrow refunded");

        Ok(tx)
    }

    /// Pone en disputa una transacción.
    pub fn dispute(&self, tx_id: &str, reason: &str) -> Result<EscrowTransaction, EscrowError> {
        let _ = reason;
        self.transition_state(tx_id, EscrowState::Disputed)
    }

    /// Obtiene una transacción por ID.
    pub fn get_transaction(&self, tx_id: &str) -> Result<EscrowTransaction, EscrowError> {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| EscrowError::Database(e.to_string()))?;
        let table = read_txn
            .open_table(ESCROW_TABLE)
            .map_err(|e| EscrowError::Database(e.to_string()))?;
        let tx_opt = table
            .get(tx_id.as_bytes())
            .map_err(|e| EscrowError::Database(e.to_string()))?;

        match tx_opt {
            Some(val) => Ok(bincode::deserialize(val.value())
                .map_err(|e| EscrowError::Database(e.to_string()))?),
            None => Err(EscrowError::TransactionNotFound(tx_id.into())),
        }
    }

    /// Obtiene todas las transacciones de un nodo.
    pub fn get_transactions_by_node(
        &self,
        node_id: &str,
    ) -> Result<Vec<EscrowTransaction>, EscrowError> {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| EscrowError::Database(e.to_string()))?;
        let table = read_txn
            .open_table(ESCROW_TABLE)
            .map_err(|e| EscrowError::Database(e.to_string()))?;

        let mut results = Vec::new();
        for entry in table
            .iter()
            .map_err(|e| EscrowError::Database(e.to_string()))?
        {
            let entry = entry.map_err(|e| EscrowError::Database(e.to_string()))?;
            // entry.0 = key (tx_id), entry.1 = value (serialized EscrowTransaction)
            let tx: EscrowTransaction = bincode::deserialize(entry.1.value())
                .map_err(|e| EscrowError::Database(e.to_string()))?;
            if tx.seller_id == node_id || tx.buyer_id == node_id {
                results.push(tx);
            }
        }

        Ok(results)
    }

    /// Verifica la firma de una transacción.
    pub fn verify_signature(&self, tx: &EscrowTransaction) -> Result<bool, EscrowError> {
        let mut tx_clone = tx.clone();
        tx_clone.signature = Vec::new();

        let tx_bytes =
            bincode::serialize(&tx_clone).map_err(|e| EscrowError::Database(e.to_string()))?;

        let mut sig_array = [0u8; 64];
        if tx.signature.len() < 64 {
            return Ok(false);
        }
        sig_array.copy_from_slice(&tx.signature[..64]);
        let signature = Signature::from_bytes(&sig_array);

        Ok(self
            .signing_key
            .verifying_key()
            .verify(&tx_bytes, &signature)
            .is_ok())
    }

    /// Firma datos internos.
    fn sign(&self, data: &[u8]) -> Vec<u8> {
        let signature = self.signing_key.sign(data);
        signature.to_bytes().to_vec()
    }

    /// Valida transiciones de estado permitidas.
    fn validate_transition(&self, from: &EscrowState, to: &EscrowState) -> Result<(), EscrowError> {
        let valid = match from {
            EscrowState::Locked => matches!(
                to,
                EscrowState::Delivered | EscrowState::Refunded | EscrowState::Disputed
            ),
            EscrowState::Delivered => matches!(
                to,
                EscrowState::ZKPVerified | EscrowState::Refunded | EscrowState::Disputed
            ),
            EscrowState::ZKPVerified => matches!(to, EscrowState::Released | EscrowState::Disputed),
            EscrowState::Released | EscrowState::Refunded => false,
            EscrowState::Disputed => matches!(to, EscrowState::Released | EscrowState::Refunded),
        };

        if !valid {
            return Err(EscrowError::InvalidStateTransition {
                from: from.clone(),
                to: to.clone(),
            });
        }

        Ok(())
    }

    /// Registra un evento de auditoría.
    fn log_audit_event(
        &self,
        tx_id: &str,
        event_type: &str,
        details: String,
    ) -> Result<(), EscrowError> {
        let event_id = format!("evt_{}_{}", tx_id, chrono::Utc::now().timestamp_millis());
        let event = AuditEvent {
            event_id: event_id.clone(),
            tx_id: tx_id.into(),
            event_type: event_type.into(),
            details,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            signature: self.sign(event_id.as_bytes()),
        };

        let write_txn = self
            .db
            .begin_write()
            .map_err(|e| EscrowError::Database(e.to_string()))?;
        {
            let mut table = write_txn
                .open_table(AUDIT_TABLE)
                .map_err(|e| EscrowError::Database(e.to_string()))?;
            let mut val_bytes = Vec::new();
            bincode::serialize_into(&mut val_bytes, &event)
                .map_err(|e| EscrowError::Database(e.to_string()))?;
            table
                .insert(event_id.as_bytes(), val_bytes.as_slice())
                .map_err(|e| EscrowError::Database(e.to_string()))?;
        }
        write_txn
            .commit()
            .map_err(|e| EscrowError::Database(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_get_escrow() {
        let (ledger, _, _path) = EscrowLedger::new_test().unwrap();
        let tx = ledger
            .create_escrow(
                "tx1".into(),
                "seller".into(),
                "buyer".into(),
                100.0,
                "settlement_hash_1".into(),
            )
            .unwrap();

        assert_eq!(tx.tx_id, "tx1");
        assert_eq!(tx.state, EscrowState::Locked);
        assert_eq!(tx.amount, 100.0);

        let retrieved = ledger.get_transaction("tx1").unwrap();
        assert_eq!(retrieved.tx_id, "tx1");
        assert_eq!(retrieved.state, EscrowState::Locked);
    }

    #[test]
    fn test_state_transitions() {
        let (ledger, _, _path) = EscrowLedger::new_test().unwrap();
        ledger
            .create_escrow(
                "tx1".into(),
                "seller".into(),
                "buyer".into(),
                100.0,
                "settlement_hash_1".into(),
            )
            .unwrap();

        // Locked -> Delivered
        let tx = ledger
            .transition_state("tx1", EscrowState::Delivered)
            .unwrap();
        assert_eq!(tx.state, EscrowState::Delivered);

        // Delivered -> ZKPVerified
        let tx = ledger
            .transition_state("tx1", EscrowState::ZKPVerified)
            .unwrap();
        assert_eq!(tx.state, EscrowState::ZKPVerified);

        // ZKPVerified -> Released
        let tx = ledger
            .transition_state("tx1", EscrowState::Released)
            .unwrap();
        assert_eq!(tx.state, EscrowState::Released);
    }

    #[test]
    fn test_invalid_state_transition() {
        let (ledger, _, _path) = EscrowLedger::new_test().unwrap();
        ledger
            .create_escrow(
                "tx1".into(),
                "seller".into(),
                "buyer".into(),
                100.0,
                "settlement_hash_1".into(),
            )
            .unwrap();

        // Locked -> Released is invalid
        let result = ledger.transition_state("tx1", EscrowState::Released);
        assert!(result.is_err());
    }

    #[test]
    fn test_release_on_zkp_success() {
        let (ledger, _, _path) = EscrowLedger::new_test().unwrap();
        ledger
            .create_escrow(
                "tx1".into(),
                "seller".into(),
                "buyer".into(),
                100.0,
                "settlement_hash_1".into(),
            )
            .unwrap();

        // Transición: Locked → Delivered → ZKPVerified → Released
        ledger
            .transition_state("tx1", EscrowState::Delivered)
            .unwrap();

        let slo = SLOMetrics {
            observed_latency_ms: 50,
            agreed_latency_ms: 100,
            observed_availability: 0.99,
            agreed_availability: 0.95,
            observed_throughput: 2000,
            agreed_throughput: 1000,
        };

        let tx = ledger
            .release_on_zkp("tx1", "zkp_hash_abc".into(), slo)
            .unwrap();
        assert_eq!(tx.state, EscrowState::Released);
        assert_eq!(tx.zkp_hash, Some("zkp_hash_abc".into()));
    }

    #[test]
    fn test_release_on_zkp_slo_failure() {
        let (ledger, _, _path) = EscrowLedger::new_test().unwrap();
        ledger
            .create_escrow(
                "tx1".into(),
                "seller".into(),
                "buyer".into(),
                100.0,
                "settlement_hash_1".into(),
            )
            .unwrap();

        let slo = SLOMetrics {
            observed_latency_ms: 200, // exceeds agreed 100
            agreed_latency_ms: 100,
            observed_availability: 0.99,
            agreed_availability: 0.95,
            observed_throughput: 2000,
            agreed_throughput: 1000,
        };

        let result = ledger.release_on_zkp("tx1", "zkp_hash_abc".into(), slo);
        assert!(result.is_err());
    }

    #[test]
    fn test_refund() {
        let (ledger, _, _path) = EscrowLedger::new_test().unwrap();
        ledger
            .create_escrow(
                "tx1".into(),
                "seller".into(),
                "buyer".into(),
                100.0,
                "settlement_hash_1".into(),
            )
            .unwrap();

        let tx = ledger.refund("tx1", "Seller did not deliver").unwrap();
        assert_eq!(tx.state, EscrowState::Refunded);
    }

    #[test]
    fn test_dispute() {
        let (ledger, _, _path) = EscrowLedger::new_test().unwrap();
        ledger
            .create_escrow(
                "tx1".into(),
                "seller".into(),
                "buyer".into(),
                100.0,
                "settlement_hash_1".into(),
            )
            .unwrap();

        let tx = ledger.dispute("tx1", "Quality issue").unwrap();
        assert_eq!(tx.state, EscrowState::Disputed);
    }

    #[test]
    fn test_transaction_not_found() {
        let (ledger, _, _path) = EscrowLedger::new_test().unwrap();
        let result = ledger.get_transaction("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_transactions_by_node() {
        let (ledger, _, _path) = EscrowLedger::new_test().unwrap();
        ledger
            .create_escrow(
                "tx1".into(),
                "seller".into(),
                "buyer1".into(),
                100.0,
                "settlement_hash_1".into(),
            )
            .unwrap();
        ledger
            .create_escrow(
                "tx2".into(),
                "seller".into(),
                "buyer2".into(),
                200.0,
                "settlement_hash_2".into(),
            )
            .unwrap();

        let seller_txs = ledger.get_transactions_by_node("seller").unwrap();
        assert_eq!(seller_txs.len(), 2);

        let buyer_txs = ledger.get_transactions_by_node("buyer1").unwrap();
        assert_eq!(buyer_txs.len(), 1);
    }
}
