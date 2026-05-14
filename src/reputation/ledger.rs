//! Reputation Ledger - Registro inmutable de contribuciones verificadas
//!
//! Almacena contribuciones de nodos con verificación ZKP, usando redb
//! como base de datos embebida para persistencia local.

use redb::{Database, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
// CLEANUP: removed unused import PathBuf
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tracing::{info, warn};
// CLEANUP: removed unused import error

/// Error del ledger de reputación
#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Entry not found: {0}")]
    NotFound(String),
    #[error("Duplicate entry: node={node}, batch={batch}")]
    Duplicate { node: String, batch: String },
    #[error("Invalid hash: expected={expected}, got={got}")]
    InvalidHash { expected: String, got: String },
}

/// Tabla redb para contribuciones
const CONTRIBUTIONS_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("contributions");

/// Tipo de contribución
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContributionType {
    /// Forward pass SAE verificado
    SaeForward,
    /// Batch de consenso validado
    ConsensusBatch,
    /// Feedback humano proporcionado
    HumanFeedback,
    /// Concepto aprendido en semantic_map
    ConceptLearned,
    /// Propuesta de gobernanza creada
    GovernanceProposal,
    /// Voto en gobernanza
    GovernanceVote,
    /// Sincronización de modelo desde ecosistema
    ModelSync,
}

impl std::fmt::Display for ContributionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContributionType::SaeForward => write!(f, "SaeForward"),
            ContributionType::ConsensusBatch => write!(f, "ConsensusBatch"),
            ContributionType::HumanFeedback => write!(f, "HumanFeedback"),
            ContributionType::ConceptLearned => write!(f, "ConceptLearned"),
            ContributionType::GovernanceProposal => write!(f, "GovernanceProposal"),
            ContributionType::GovernanceVote => write!(f, "GovernanceVote"),
            ContributionType::ModelSync => write!(f, "ModelSync"),
        }
    }
}

/// Registro de contribución individual
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contribution {
    /// ID del nodo que contribuyó
    pub node_id: String,
    /// ID de la capa SAE (si aplica)
    pub layer_id: Option<String>,
    /// Hash del batch procesado
    pub batch_hash: String,
    /// Si fue verificado con ZKP
    pub zkp_verified: bool,
    /// Tipo de contribución
    pub contribution_type: ContributionType,
    /// Timestamp de la contribución (epoch seconds)
    pub timestamp: u64,
    /// Créditos base otorgados
    pub base_credits: f64,
    /// Hash del registro anterior (cadena inmutable)
    pub previous_hash: Option<String>,
}

impl Contribution {
    /// Crear nueva contribución
    pub fn new(
        node_id: String,
        layer_id: Option<String>,
        batch_hash: String,
        zkp_verified: bool,
        contribution_type: ContributionType,
        base_credits: f64,
        previous_hash: Option<String>,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Contribution {
            node_id,
            layer_id,
            batch_hash,
            zkp_verified,
            contribution_type,
            timestamp,
            base_credits,
            previous_hash,
        }
    }

    /// Calcular hash propio del registro
    pub fn compute_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.node_id.as_bytes());
        if let Some(ref layer) = self.layer_id {
            hasher.update(layer.as_bytes());
        }
        hasher.update(self.batch_hash.as_bytes());
        hasher.update((self.zkp_verified as u8).to_le_bytes()); // CLEANUP: Removed needless borrow
        hasher.update(self.timestamp.to_le_bytes());
        hasher.update(self.base_credits.to_le_bytes());
        hex::encode(hasher.finalize())
    }

    /// Verificar integridad de la cadena
    pub fn verify_chain(&self, expected_previous: &str) -> bool {
        match &self.previous_hash {
            Some(prev) => prev == expected_previous,
            None => true, // Primer registro no tiene anterior
        }
    }
}

/// Ledger de reputación con persistencia redb
pub struct ReputationLedger {
    db: Database,
    last_hash: Option<String>,
}

impl ReputationLedger {
    /// Crear/abrir ledger en el path especificado
    pub fn open(path: &Path) -> Result<Self, LedgerError> {
        let db = Database::create(path)
            .map_err(|e| LedgerError::Database(format!("Failed to create DB: {e}")))?;

        // Inicializar tabla
        let write_txn = db
            .begin_write()
            .map_err(|e| LedgerError::Database(format!("Failed to begin write txn: {e}")))?;

        {
            let table = write_txn
                .open_table(CONTRIBUTIONS_TABLE)
                .map_err(|e| LedgerError::Database(format!("Failed to open table: {e}")))?;

            // FIX: E0599/E0282 - .range() returns Result<Range>, need to handle Result before .last()
            let _last_hash = table
                .range(""..)
                .map_err(|e| LedgerError::Database(format!("Failed to open range: {e}")))?
                .last()
                .map(|result| {
                    result.ok().map(|(_key, value)| {
                        String::from_utf8_lossy(value.value()).to_string()
                    })
                });
        }

        write_txn
            .commit()
            .map_err(|e| LedgerError::Database(format!("Failed to commit: {e}")))?;

        let last_hash = Self::get_last_hash(&db).ok().flatten();

        info!(
            db_path = %path.display(),
            "Reputation ledger opened"
        );

        Ok(Self { db, last_hash })
    }

    /// Obtener último hash del ledger
    fn get_last_hash(db: &Database) -> Result<Option<String>, LedgerError> {
        let read_txn = db
            .begin_read()
            .map_err(|e| LedgerError::Database(format!("Failed to begin read txn: {e}")))?;

        let table = read_txn
            .open_table(CONTRIBUTIONS_TABLE)
            .map_err(|e| LedgerError::Database(format!("Failed to open table: {e}")))?;

        // FIX: E0599/E0282 - .range() returns Result<Range>, handle Result before .last()
        let range = table
            .range(""..)
            .map_err(|e| LedgerError::Database(format!("Failed to open range: {e}")))?;
        let last = range.last()
            .and_then(|result| {
                result.ok().map(|(_key, value)| {
                    String::from_utf8_lossy(value.value()).to_string()
                })
            });

        Ok(last)
    }

    /// Registrar contribución
    pub fn record(&mut self, mut contribution: Contribution) -> Result<String, LedgerError> {
        // Verificar duplicado
        if self.is_duplicate(&contribution.node_id, &contribution.batch_hash)? {
            return Err(LedgerError::Duplicate {
                node: contribution.node_id.clone(),
                batch: contribution.batch_hash.clone(),
            });
        }

        // Establecer hash anterior para cadena inmutable
        contribution.previous_hash = self.last_hash.clone();

        // Calcular hash del registro
        let record_hash = contribution.compute_hash();

        // Serializar y almacenar
        let serialized = serde_json::to_string(&contribution)
            .map_err(|e| LedgerError::Serialization(e.to_string()))?;

        let write_txn = self
            .db
            .begin_write()
            .map_err(|e| LedgerError::Database(format!("Failed to begin write txn: {e}")))?;

        {
            let mut table = write_txn
                .open_table(CONTRIBUTIONS_TABLE)
                .map_err(|e| LedgerError::Database(format!("Failed to open table: {e}")))?;

            // Key: "{node_id}_{timestamp}_{hash}" para unicidad
            let key = format!(
                "{}_{}_{}",
                contribution.node_id, contribution.timestamp, &record_hash[..8]
            );

            // FIX: trait bound - use key.as_str() to match TableDefinition<&str, &[u8]>
            table
                .insert(key.as_str(), serialized.as_bytes())
                .map_err(|e| LedgerError::Database(format!("Failed to insert: {e}")))?;
        }

        write_txn
            .commit()
            .map_err(|e| LedgerError::Database(format!("Failed to commit: {e}")))?;

        // Actualizar último hash
        self.last_hash = Some(record_hash.clone());

        info!(
            node_id = %contribution.node_id,
            batch_hash = %contribution.batch_hash,
            contribution_type = %contribution.contribution_type,
            zkp_verified = contribution.zkp_verified,
            credits = contribution.base_credits,
            "Contribution recorded"
        );

        Ok(record_hash)
    }

    /// Verificar si ya existe contribución duplicada
    fn is_duplicate(&self, node_id: &str, batch_hash: &str) -> Result<bool, LedgerError> {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| LedgerError::Database(format!("Failed to begin read txn: {e}")))?;

        let table = read_txn
            .open_table(CONTRIBUTIONS_TABLE)
            .map_err(|e| LedgerError::Database(format!("Failed to open table: {e}")))?;

        // FIX: E0599/E0282 - table.iter() returns Result<Range>, need ? first
        for entry in table.iter().map_err(|e| LedgerError::Database(e.to_string()))? {
            let (_key, value) = entry.map_err(|e: redb::StorageError| LedgerError::Database(e.to_string()))?;
            let contribution: Contribution =
                serde_json::from_slice(value.value()).map_err(|e| {
                    LedgerError::Serialization(format!("Failed to deserialize entry: {e}"))
                })?;

            if contribution.node_id == node_id && contribution.batch_hash == batch_hash {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Obtener todas las contribuciones de un nodo
    pub fn get_node_contributions(&self, node_id: &str) -> Result<Vec<Contribution>, LedgerError> {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| LedgerError::Database(format!("Failed to begin read txn: {e}")))?;

        let table = read_txn
            .open_table(CONTRIBUTIONS_TABLE)
            .map_err(|e| LedgerError::Database(format!("Failed to open table: {e}")))?;

        let mut contributions = Vec::new();

        // FIX: E0599/E0282 - table.iter() returns Result<Range>, need ? first
        for entry in table.iter().map_err(|e| LedgerError::Database(e.to_string()))? {
            let (_key, value) = entry.map_err(|e: redb::StorageError| LedgerError::Database(e.to_string()))?;
            let contribution: Contribution =
                serde_json::from_slice(value.value()).map_err(|e| {
                    LedgerError::Serialization(format!("Failed to deserialize: {e}"))
                })?;

            if contribution.node_id == node_id {
                contributions.push(contribution);
            }
        }

        Ok(contributions)
    }

    /// Obtener contribuciones por tipo
    pub fn get_by_type(
        &self,
        contribution_type: &ContributionType,
    ) -> Result<Vec<Contribution>, LedgerError> {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| LedgerError::Database(format!("Failed to begin read txn: {e}")))?;

        let table = read_txn
            .open_table(CONTRIBUTIONS_TABLE)
            .map_err(|e| LedgerError::Database(format!("Failed to open table: {e}")))?;

        let mut contributions = Vec::new();

        // FIX: E0599/E0282 - table.iter() returns Result<Range>, need ? first
        for entry in table.iter().map_err(|e| LedgerError::Database(e.to_string()))? {
            let (_key, value) = entry.map_err(|e: redb::StorageError| LedgerError::Database(e.to_string()))?;
            let contribution: Contribution =
                serde_json::from_slice(value.value()).map_err(|e| {
                    LedgerError::Serialization(format!("Failed to deserialize: {e}"))
                })?;

            if contribution.contribution_type == *contribution_type {
                contributions.push(contribution);
            }
        }

        Ok(contributions)
    }

    /// Verificar integridad de la cadena completa
    pub fn verify_chain_integrity(&self) -> Result<bool, LedgerError> {
        let contributions = self.get_all()?;
        let mut expected_previous: Option<String> = None;

        for contribution in &contributions {
            if !contribution.verify_chain(
                expected_previous
                    .as_deref()
                    .unwrap_or(""),
            ) {
                warn!(
                    node_id = %contribution.node_id,
                    "Chain integrity broken at contribution"
                );
                return Ok(false);
            }
            expected_previous = Some(contribution.compute_hash());
        }

        info!("Chain integrity verified successfully");
        Ok(true)
    }

    /// Obtener todas las contribuciones
    pub fn get_all(&self) -> Result<Vec<Contribution>, LedgerError> {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| LedgerError::Database(format!("Failed to begin read txn: {e}")))?;

        let table = read_txn
            .open_table(CONTRIBUTIONS_TABLE)
            .map_err(|e| LedgerError::Database(format!("Failed to open table: {e}")))?;

        let mut contributions = Vec::new();

        // FIX: E0599/E0282 - table.iter() returns Result<Range>, need ? first
        for entry in table.iter().map_err(|e| LedgerError::Database(e.to_string()))? {
            let (_key, value) = entry.map_err(|e: redb::StorageError| LedgerError::Database(e.to_string()))?;
            let contribution: Contribution =
                serde_json::from_slice(value.value()).map_err(|e| {
                    LedgerError::Serialization(format!("Failed to deserialize: {e}"))
                })?;
            contributions.push(contribution);
        }

        Ok(contributions)
    }

    /// Estadísticas del ledger
    pub fn stats(&self) -> Result<LedgerStats, LedgerError> {
        let contributions = self.get_all()?;
        let total_entries = contributions.len();

        let total_credits: f64 = contributions.iter().map(|c| c.base_credits).sum();
        let zkp_verified_count = contributions.iter().filter(|c| c.zkp_verified).count();

        // Contar por tipo
        let mut by_type: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for c in &contributions {
            *by_type
                .entry(c.contribution_type.to_string())
                .or_insert(0) += 1;
        }

        // Contar por nodo
        let mut by_node: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for c in &contributions {
            *by_node.entry(c.node_id.clone()).or_insert(0) += 1;
        }

        Ok(LedgerStats {
            total_entries,
            total_credits,
            zkp_verified_count,
            unique_nodes: by_node.len(),
            by_type,
            by_node,
        })
    }
}

/// Estadísticas del ledger
#[derive(Debug, Serialize, Deserialize)]
pub struct LedgerStats {
    pub total_entries: usize,
    pub total_credits: f64,
    pub zkp_verified_count: usize,
    pub unique_nodes: usize,
    pub by_type: std::collections::HashMap<String, usize>,
    pub by_node: std::collections::HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_ledger_creation() {
        let temp_dir = std::env::temp_dir().join("ed2kia_ledger_test");
        let _ = fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let ledger_path = temp_dir.join("ledger.redb");
        let ledger = ReputationLedger::open(&ledger_path);
        assert!(ledger.is_ok());

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_contribution_hash() {
        let contribution = Contribution::new(
            "node_1".to_string(),
            Some("layer_0".to_string()),
            "abc123".to_string(),
            true,
            ContributionType::SaeForward,
            10.0,
            None,
        );

        let hash = contribution.compute_hash();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA-256 hex
    }

    #[test]
    fn test_contribution_chain_verification() {
        let contribution = Contribution::new(
            "node_1".to_string(),
            None,
            "batch_1".to_string(),
            false,
            ContributionType::HumanFeedback,
            5.0,
            None,
        );

        // Sin hash anterior, debería pasar
        assert!(contribution.verify_chain(""));
    }
}
