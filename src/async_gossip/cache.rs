//! Gossip Cache — Almacenamiento offline con sync al reconectar.
//!
//! **Stuartian Law 5 (Múltiples Posibilidades):** Los nodos operan
//! offline y sincronizan al reconectar, sin pérdida de estado.

use std::fmt;

/// Error en el cache de gossip.
#[derive(Debug)]
pub enum GossipCacheError {
    /// Error de almacenamiento.
    StorageError(String),
    /// Entrada no encontrada.
    EntryNotFound(String),
    /// Cache lleno.
    CacheFull,
    /// Error de serialización.
    SerializationError(String),
}

impl fmt::Display for GossipCacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GossipCacheError::StorageError(msg) => {
                write!(f, "Storage error: {}", msg)
            }
            GossipCacheError::EntryNotFound(key) => {
                write!(f, "Entry not found: {}", key)
            }
            GossipCacheError::CacheFull => {
                write!(f, "Cache is full")
            }
            GossipCacheError::SerializationError(msg) => {
                write!(f, "Serialization error: {}", msg)
            }
        }
    }
}

impl std::error::Error for GossipCacheError {}

/// Entrada del cache de gossip.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// Clave única.
    pub key: String,
    /// Datos almacenados.
    pub data: Vec<u8>,
    /// Timestamp de creación.
    pub created_at: u64,
    /// ¿Sincronizado con la red?
    pub synced: bool,
}

/// Cache local para mensajes de gossip durante desconexión.
///
/// **Stuartian Law 5:** Tolerancia a particiones. Los mensajes
/// se almacenan localmente y se sincronizan al reconectar.
pub struct GossipCache {
    /// Capacidad máxima del cache.
    pub max_entries: usize,
}

impl GossipCache {
    /// Crea un nuevo cache con capacidad especificada.
    pub fn new(max_entries: usize) -> Self {
        Self { max_entries }
    }

    /// Almacena un mensaje en el cache.
    pub fn store(
        &mut self,
        _key: String,
        _data: Vec<u8>,
    ) -> Result<(), GossipCacheError> {
        // TODO(Sprint16.4): Implement redb/SQLite storage.
        Err(GossipCacheError::StorageError(
            "Storage backend not yet implemented".into(),
        ))
    }

    /// Recupera un mensaje del cache.
    pub fn retrieve(&self, _key: &str) -> Result<CacheEntry, GossipCacheError> {
        // TODO(Sprint16.4): Implement retrieval.
        Err(GossipCacheError::EntryNotFound(_key.to_string()))
    }

    /// Marca una entrada como sincronizada.
    pub fn mark_synced(&mut self, _key: &str) -> Result<(), GossipCacheError> {
        // TODO(Sprint16.4): Implement sync marking.
        Err(GossipCacheError::EntryNotFound(_key.to_string()))
    }

    /// Retorna entradas pendientes de sincronización.
    pub fn pending_sync(&self) -> Vec<CacheEntry> {
        // TODO(Sprint16.4): Implement pending entries query.
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_creation() {
        let cache = GossipCache::new(1000);
        assert_eq!(cache.max_entries, 1000);
    }

    #[test]
    fn test_cache_entry() {
        let entry = CacheEntry {
            key: "msg-1".into(),
            data: vec![1, 2, 3],
            created_at: 1000,
            synced: false,
        };
        assert!(!entry.synced);
    }

    #[test]
    fn test_pending_sync_empty() {
        let cache = GossipCache::new(100);
        let pending = cache.pending_sync();
        assert!(pending.is_empty());
    }

    #[test]
    fn test_error_display() {
        let err = GossipCacheError::CacheFull;
        assert!(!format!("{}", err).is_empty());
    }
}
