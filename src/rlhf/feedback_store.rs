//! Feedback Store - Almacenamiento embebido (redb) + export JSONL
//!
//! Usa `redb` (base de datos embebida, cero dependencias externas) para
//! almacenar feedback humano, timestamps, layer_id, feature_idx y decisión.
//! Exporta a formato JSONL compatible con `llama.cpp`/`vLLM` para fine-tuning offline.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use parking_lot::RwLock;
// FIX: redb 1.5 - Import TableDefinition + ReadableTable trait (provides len/get/range)
use redb::{Database, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

// FIX: redb 1.5 - Define table definitions at module level (required by redb 1.5 API)
const FEEDBACK_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("feedback_entries");
const STATISTICS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("feedback_statistics");
const ANNOTATORS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("annotator_info");

/// Decisión de feedback humano
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FeedbackDecision {
    Approved,
    Rejected,
    Corrected,
    Uncertain,
}

impl std::fmt::Display for FeedbackDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeedbackDecision::Approved => write!(f, "approved"),
            FeedbackDecision::Rejected => write!(f, "rejected"),
            FeedbackDecision::Corrected => write!(f, "corrected"),
            FeedbackDecision::Uncertain => write!(f, "uncertain"),
        }
    }
}

/// Entrada de feedback almacenada
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackEntry {
    /// ID único del feedback
    pub id: String,
    /// Timestamp en epoch milliseconds
    pub timestamp_ms: u64,
    /// ID de la capa SAE
    pub layer_id: String,
    /// Índice de feature
    pub feature_idx: u32,
    /// Valor de la feature
    pub feature_value: f64,
    /// Concepto asociado (si existe)
    pub concept: Option<String>,
    /// Decisión del anotador
    pub decision: FeedbackDecision,
    /// Corrección proporcionada (si aplica)
    pub correction: Option<String>,
    /// Confianza del modelo (0.0 - 1.0)
    pub model_confidence: f64,
    /// ID del anotador
    pub annotator_id: String,
    /// Metadata adicional
    pub metadata: Option<String>,
}

impl FeedbackEntry {
    pub fn new(
        id: String,
        layer_id: String,
        feature_idx: u32,
        feature_value: f64,
        decision: FeedbackDecision,
        annotator_id: String,
    ) -> Self {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0));

        Self {
            id,
            timestamp_ms: duration.as_millis() as u64,
            layer_id,
            feature_idx,
            feature_value,
            concept: None,
            decision,
            correction: None,
            model_confidence: 0.0,
            annotator_id,
            metadata: None,
        }
    }

    /// Convierte a formato JSONL para fine-tuning
    pub fn to_jsonl_training_format(&self) -> String {
        let label = match &self.decision {
            FeedbackDecision::Approved => "entailment",
            FeedbackDecision::Rejected => "contradiction",
            FeedbackDecision::Corrected => "corrected",
            FeedbackDecision::Uncertain => "neutral",
        };

        let text = match &self.concept {
            Some(concept) => concept.clone(),
            None => format!("feature_{}_value_{}", self.feature_idx, self.feature_value),
        };

        serde_json::json!({
            "text": text,
            "label": label,
            "confidence": self.model_confidence,
            "layer_id": self.layer_id,
            "feature_idx": self.feature_idx,
            "annotator_id": self.annotator_id,
            "timestamp_ms": self.timestamp_ms,
        })
        .to_string()
    }
}

/// Configuración del FeedbackStore
#[derive(Debug, Clone)]
pub struct FeedbackStoreConfig {
    /// Ruta al archivo de base de datos
    pub db_path: PathBuf,
    /// Tamaño máximo de la DB en MB
    pub max_db_size_mb: u64,
    /// Habilitar compresión automática
    pub auto_compact: bool,
    /// Intervalo de compaction en segundos
    pub compact_interval_secs: u64,
}

impl Default for FeedbackStoreConfig {
    fn default() -> Self {
        Self {
            db_path: PathBuf::from("ed2kia_feedback.redb"),
            max_db_size_mb: 512,
            auto_compact: true,
            compact_interval_secs: 3600,
        }
    }
}

/// Store principal de feedback
pub struct FeedbackStore {
    /// Base de datos redb
    db: RwLock<Database>,
    /// Configuración
    config: FeedbackStoreConfig,
    /// Contador de entradas en memoria (para stats rápidos)
    entry_count: RwLock<u64>,
}

impl FeedbackStore {
    /// Crea o abre un FeedbackStore
    pub fn new(config: FeedbackStoreConfig) -> Result<Self, FeedbackStoreError> {
        // FIX: redb 1.5 - Database::create returns Result<_, DatabaseError>
        let db = Database::create(config.db_path.as_path())?;

        // FIX: redb 1.5 - begin_write returns Result<_, TransactionError>
        let write_txn = db.begin_write()?;
        {
            // FIX: redb 1.5 - Use TableDefinition constants instead of open_table::<K, V>(name)
            write_txn.open_table(FEEDBACK_TABLE)?;
            write_txn.open_table(STATISTICS_TABLE)?;
            write_txn.open_table(ANNOTATORS_TABLE)?;
        }
        // FIX: redb 1.5 - commit returns Result<_, CommitError>
        write_txn.commit()?;

        // Contar entradas existentes
        // FIX: redb 1.5 - Drop read_txn before moving db into RwLock to avoid E0505
        let count: u64;
        {
            let read_txn = db.begin_read()?;
            let feedback_table = read_txn.open_table(FEEDBACK_TABLE)?;
            count = feedback_table.len()?;
        }

        Ok(Self {
            db: RwLock::new(db),
            config,
            entry_count: RwLock::new(count),
        })
    }

    /// Crea store con ruta por defecto
    pub fn default_store() -> Result<Self, FeedbackStoreError> {
        Self::new(FeedbackStoreConfig::default())
    }

    /// Almacena entrada de feedback
    pub fn store_feedback(&self, entry: &FeedbackEntry) -> Result<(), FeedbackStoreError> {
        let bytes = serde_json::to_vec(entry)?;
        let id = entry.id.clone();
        // FIX: redb 1.5 - Drop db ref before updating entry_count to avoid E0505
        {
            let db = self.db.read();
            let write_txn = db.begin_write()?;
            {
                let mut feedback_table = write_txn.open_table(FEEDBACK_TABLE)?;
                feedback_table.insert(id.as_str(), bytes.as_slice())?;
            }
            write_txn.commit()?;
        }
        *self.entry_count.write() += 1;

        debug!(
            id = &id,
            decision = %entry.decision,
            "Feedback stored"
        );
        Ok(())
    }

    /// Obtiene entrada de feedback por ID
    pub fn get_feedback(&self, id: &str) -> Result<Option<FeedbackEntry>, FeedbackStoreError> {
        // FIX: redb 1.5 - Save match to variable before return to drop AccessGuard before dropping table (E0597)
        let result: Result<Option<FeedbackEntry>, FeedbackStoreError> = (|| {
            let db = self.db.read();
            let read_txn = db.begin_read()?;
            let feedback_table = read_txn.open_table(FEEDBACK_TABLE)?;
            let x = match feedback_table.get(id)? {
                Some(value) => {
                    let entry: FeedbackEntry = serde_json::from_slice(value.value())?;
                    Ok(Some(entry))
                }
                None => Ok(None),
            };
            x
        })();
        result
    }

    /// Obtiene feedback por layer_id
    pub fn get_feedback_by_layer(
        &self,
        layer_id: &str,
    ) -> Result<Vec<FeedbackEntry>, FeedbackStoreError> {
        // FIX: redb 1.5 - Save result to variable before return to drop AccessGuard (E0597)
        let result: Result<Vec<FeedbackEntry>, FeedbackStoreError> = (|| {
            let db = self.db.read();
            let read_txn = db.begin_read()?;
            let feedback_table = read_txn.open_table(FEEDBACK_TABLE)?;
            let mut results = Vec::new();
            for entry_result in feedback_table.range::<&str>(..)? {
                let (_, value) = entry_result?;
                let entry: FeedbackEntry = serde_json::from_slice(value.value())?;
                if entry.layer_id == layer_id {
                    results.push(entry);
                }
            }
            Ok(results)
        })();
        result
    }

    /// Obtiene feedback por anotador
    pub fn get_feedback_by_annotator(
        &self,
        annotator_id: &str,
    ) -> Result<Vec<FeedbackEntry>, FeedbackStoreError> {
        // FIX: redb 1.5 - Save result to variable before return to drop AccessGuard (E0597)
        let result: Result<Vec<FeedbackEntry>, FeedbackStoreError> = (|| {
            let db = self.db.read();
            let read_txn = db.begin_read()?;
            let feedback_table = read_txn.open_table(FEEDBACK_TABLE)?;
            let mut results = Vec::new();
            for entry_result in feedback_table.range::<&str>(..)? {
                let (_, value) = entry_result?;
                let entry: FeedbackEntry = serde_json::from_slice(value.value())?;
                if entry.annotator_id == annotator_id {
                    results.push(entry);
                }
            }
            Ok(results)
        })();
        result
    }

    /// Obtiene feedback en rango de tiempo
    pub fn get_feedback_in_range(
        &self,
        start_ms: u64,
        end_ms: u64,
    ) -> Result<Vec<FeedbackEntry>, FeedbackStoreError> {
        // FIX: redb 1.5 - Save result to variable before return to drop AccessGuard (E0597)
        let result: Result<Vec<FeedbackEntry>, FeedbackStoreError> = (|| {
            let db = self.db.read();
            let read_txn = db.begin_read()?;
            let feedback_table = read_txn.open_table(FEEDBACK_TABLE)?;
            let mut results = Vec::new();
            for entry_result in feedback_table.range::<&str>(..)? {
                let (_, value) = entry_result?;
                let entry: FeedbackEntry = serde_json::from_slice(value.value())?;
                if entry.timestamp_ms >= start_ms && entry.timestamp_ms <= end_ms {
                    results.push(entry);
                }
            }
            Ok(results)
        })();
        result
    }

    /// Elimina entrada de feedback
    pub fn delete_feedback(&self, id: &str) -> Result<bool, FeedbackStoreError> {
        // FIX: redb 1.5 - Drop table before commit to avoid E0505
        let existed: bool;
        {
            let db = self.db.read();
            let write_txn = db.begin_write()?;
            {
                let mut feedback_table = write_txn.open_table(FEEDBACK_TABLE)?;
                existed = feedback_table.remove(id)?.is_some();
            }
            write_txn.commit()?;
        }

        if existed {
            *self.entry_count.write() = self.entry_count.read().saturating_sub(1);
        }

        Ok(existed)
    }

    /// Exporta todo el feedback a formato JSONL
    pub fn export_jsonl(&self, path: &Path) -> Result<usize, FeedbackStoreError> {
        let entries = self.get_all_feedback()?;
        let file = File::create(path)?;
        let mut writer = std::io::BufWriter::new(file);

        for entry in &entries {
            let line = entry.to_jsonl_training_format();
            writeln!(writer, "{}", line)?;
        }

        writer.flush()?;
        info!(
            path = %path.display(),
            count = entries.len(),
            "Feedback exported to JSONL"
        );

        Ok(entries.len())
    }

    /// Obtiene todas las entradas de feedback
    pub fn get_all_feedback(&self) -> Result<Vec<FeedbackEntry>, FeedbackStoreError> {
        // FIX: redb 1.5 - Save result to variable before return to drop AccessGuard (E0597)
        let result: Result<Vec<FeedbackEntry>, FeedbackStoreError> = (|| {
            let db = self.db.read();
            let read_txn = db.begin_read()?;
            let feedback_table = read_txn.open_table(FEEDBACK_TABLE)?;
            let mut results = Vec::new();
            for entry_result in feedback_table.range::<&str>(..)? {
                let (_, value) = entry_result?;
                let entry: FeedbackEntry = serde_json::from_slice(value.value())?;
                results.push(entry);
            }
            Ok(results)
        })();
        result
    }

    /// Obtiene estadísticas de feedback
    pub fn get_statistics(&self) -> FeedbackStatistics {
        let entries = self.get_all_feedback().unwrap_or_default();

        let mut approved = 0;
        let mut rejected = 0;
        let mut corrected = 0;
        let mut uncertain = 0;
        let mut total_confidence = 0.0;

        for entry in &entries {
            match entry.decision {
                FeedbackDecision::Approved => approved += 1,
                FeedbackDecision::Rejected => rejected += 1,
                FeedbackDecision::Corrected => corrected += 1,
                FeedbackDecision::Uncertain => uncertain += 1,
            }
            total_confidence += entry.model_confidence;
        }

        let total = entries.len() as f64;
        FeedbackStatistics {
            total_entries: entries.len(),
            approved,
            rejected,
            corrected,
            uncertain,
            avg_confidence: if total > 0.0 {
                total_confidence / total
            } else {
                0.0
            },
            approval_rate: if total > 0.0 {
                approved as f64 / total
            } else {
                0.0
            },
        }
    }

    /// Ejecuta compaction de la base de datos
    pub fn compact(&self) -> Result<(), FeedbackStoreError> {
        // FIX: redb 1.5 - compact() requires write lock (mutable access)
        let mut db = self.db.write();
        // FIX: redb 1.5 - compact returns Result<_, CompactionError>
        db.compact()?;
        info!("Database compacted");
        Ok(())
    }

    /// Obtiene conteo de entradas
    pub fn entry_count(&self) -> u64 {
        *self.entry_count.read()
    }
}

/// Estadísticas de feedback
#[derive(Debug, Serialize, Deserialize)]
pub struct FeedbackStatistics {
    pub total_entries: usize,
    pub approved: usize,
    pub rejected: usize,
    pub corrected: usize,
    pub uncertain: usize,
    pub avg_confidence: f64,
    pub approval_rate: f64,
}

/// Errores del FeedbackStore
// FIX: redb 1.5 - Add all redb 1.5 error types: DatabaseError, TransactionError, TableError, CommitError, StorageError
#[derive(Debug, thiserror::Error)]
pub enum FeedbackStoreError {
    #[error("Database error: {0}")]
    Database(#[from] redb::Error),
    // FIX: redb 1.5 - Database::create returns DatabaseError
    #[error("Database creation error: {0}")]
    DatabaseCreate(#[from] redb::DatabaseError),
    // FIX: redb 1.5 - begin_write/begin_read return TransactionError
    #[error("Transaction error: {0}")]
    Transaction(#[from] redb::TransactionError),
    // FIX: redb 1.5 - open_table returns TableError
    #[error("Table error: {0}")]
    Table(#[from] redb::TableError),
    // FIX: redb 1.5 - commit returns CommitError
    #[error("Commit error: {0}")]
    Commit(#[from] redb::CommitError),
    // FIX: redb 1.5 - compact/other storage ops return StorageError
    #[error("Storage error: {0}")]
    Storage(#[from] redb::StorageError),
    // FIX: redb 1.5 - compact() returns CompactionError
    #[error("Compaction error: {0}")]
    Compaction(#[from] redb::CompactionError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Invalid entry: {0}")]
    InvalidEntry(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_feedback_store_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = FeedbackStoreConfig {
            db_path: temp_dir.path().join("test_feedback.redb"),
            ..FeedbackStoreConfig::default()
        };
        let store = FeedbackStore::new(config).unwrap();
        assert_eq!(store.entry_count(), 0);
    }

    #[test]
    fn test_store_and_retrieve_feedback() {
        let temp_dir = TempDir::new().unwrap();
        let config = FeedbackStoreConfig {
            db_path: temp_dir.path().join("test_feedback.redb"),
            ..FeedbackStoreConfig::default()
        };
        let store = FeedbackStore::new(config).unwrap();

        let entry = FeedbackEntry::new(
            "test-1".to_string(),
            "layer_0".to_string(),
            42,
            0.95,
            FeedbackDecision::Approved,
            "annotator_1".to_string(),
        );

        store.store_feedback(&entry).unwrap();
        assert_eq!(store.entry_count(), 1);

        let retrieved = store.get_feedback("test-1").unwrap().unwrap();
        assert_eq!(retrieved.id, "test-1");
        assert_eq!(retrieved.decision, FeedbackDecision::Approved);
    }

    #[test]
    fn test_jsonl_export_format() {
        let entry = FeedbackEntry::new(
            "test-1".to_string(),
            "layer_0".to_string(),
            42,
            0.95,
            FeedbackDecision::Approved,
            "annotator_1".to_string(),
        );

        let jsonl = entry.to_jsonl_training_format();
        let parsed: serde_json::Value = serde_json::from_str(&jsonl).unwrap();
        assert_eq!(parsed["label"], "entailment");
        assert_eq!(parsed["feature_idx"], 42);
    }
}
