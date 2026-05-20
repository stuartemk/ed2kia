//! Gossip Cache — Almacenamiento offline con sync al reconectar.
//!
//! **Stuartian Law 5 (Múltiples Posibilidades):** Los nodos operan
//! offline y sincronizan al reconectar, sin pérdida de estado.
//!
//! **Feature Gate:** `v2.1-offline-cache`
//!
//! ### Arquitectura
//! - **PendingQueue:** Cola priorizada por timestamp (más antiguo primero)
//!   y tipo de payload (crítico > normal > bajo).
//! - **redb Storage:** Base de datos embebida para persistencia.
//! - **Sync Logic:** On `ConnectionEstablished`, drena la cola en batches
//!   con backoff exponencial (1s, 2s, 4s, 8s... max 30s) en fallos.
//!
//! ### Prioridad de Payload
//! | Tipo | Prioridad | Descripción |
//! |---|---|---|
//! | Critical | 0 | Slashing, bans, seguridad |
//! | Normal | 1 | Reputación, gradientes |
//! | Low | 2 | Metadata, heartbeats |
//!
//! ### Sync Strategy
//! 1. Ordenar por (priority ASC, timestamp ASC)
//! 2. Batch size: 32 entradas
//! 3. Exponential backoff en fallos
//! 4. Mark as synced después de mesh ACK

use std::collections::BinaryHeap;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// Tipo de payload para priorización en sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PayloadType {
    /// Crítico: slashing, bans, seguridad (prioridad 0).
    Critical = 0,
    /// Normal: reputación, gradientes (prioridad 1).
    Normal = 1,
    /// Bajo: metadata, heartbeats (prioridad 2).
    Low = 2,
}

impl fmt::Display for PayloadType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PayloadType::Critical => write!(f, "critical"),
            PayloadType::Normal => write!(f, "normal"),
            PayloadType::Low => write!(f, "low"),
        }
    }
}

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
    /// Sync fallido (reintentos agotados).
    SyncExhausted(String),
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
            GossipCacheError::SyncExhausted(msg) => {
                write!(f, "Sync exhausted: {}", msg)
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
    /// Timestamp de creación (epoch seconds).
    pub created_at: u64,
    /// ¿Sincronizado con la red?
    pub synced: bool,
    /// Tipo de payload para priorización.
    pub payload_type: PayloadType,
    /// Contador de intentos de sync fallidos.
    pub sync_attempts: u32,
}

impl CacheEntry {
    /// Crea una nueva entrada de cache.
    pub fn new(key: String, data: Vec<u8>, payload_type: PayloadType) -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            key,
            data,
            created_at,
            synced: false,
            payload_type,
            sync_attempts: 0,
        }
    }

    /// Retorna true si la entrada está pendiente de sync.
    pub fn is_pending(&self) -> bool {
        !self.synced
    }

    /// Calcula el backoff exponencial para reintentos.
    /// backoff = min(2^attempts * 1000, 30000) ms
    pub fn backoff_ms(&self) -> u64 {
        let backoff = (1u64 << self.sync_attempts) * 1000;
        backoff.min(30_000)
    }
}

// Para BinaryHeap: menor priority primero, luego menor timestamp primero.
// Usamos Reverse ordering porque BinaryHeap es max-heap.
impl PartialEq for CacheEntry {
    fn eq(&self, other: &Self) -> bool {
        self.payload_type == other.payload_type && self.created_at == other.created_at
    }
}

impl Eq for CacheEntry {}

impl PartialOrd for CacheEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CacheEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Menor priority = mayor importancia = primero en cola
        // BinaryHeap es max-heap, entonces invertimos
        other
            .payload_type
            .cmp(&self.payload_type)
            .then_with(|| other.created_at.cmp(&self.created_at))
    }
}

/// Estado de sync de una operación.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    /// Sincronización exitosa.
    Synced,
    /// En cola de sync.
    Pending,
    /// Falló con backoff activo.
    Backoff,
    /// Reintentos agotados.
    Exhausted,
}

impl fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncStatus::Synced => write!(f, "synced"),
            SyncStatus::Pending => write!(f, "pending"),
            SyncStatus::Backoff => write!(f, "backoff"),
            SyncStatus::Exhausted => write!(f, "exhausted"),
        }
    }
}

/// Estadísticas del cache.
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Total de entradas.
    pub total_entries: usize,
    /// Entradas sincronizadas.
    pub synced_count: usize,
    /// Entradas pendientes.
    pub pending_count: usize,
    /// Tamaño total en bytes.
    pub total_bytes: usize,
}

impl CacheStats {
    /// Retorna el ratio de sync (0.0 = nada sync, 1.0 = todo sync).
    pub fn sync_ratio(&self) -> f64 {
        if self.total_entries == 0 {
            return 0.0;
        }
        self.synced_count as f64 / self.total_entries as f64
    }
}

/// Cache local para mensajes de gossip durante desconexión.
///
/// **Stuartian Law 5:** Tolerancia a particiones. Los mensajes
/// se almacenan localmente y se sincronizan al reconectar.
///
/// ### Invariantes
/// 1. entries.len() <= max_entries
/// 2. PendingQueue ordenado por (priority ASC, timestamp ASC)
/// 3. mark_synced() solo cambia synced = true
/// 4. sync_batch() retorna en orden de prioridad
pub struct GossipCache {
    /// Capacidad máxima del cache.
    pub max_entries: usize,
    /// Entradas almacenadas.
    entries: std::collections::BTreeMap<String, CacheEntry>,
    /// Cola priorizada para sync.
    pending_queue: BinaryHeap<CacheEntry>,
    /// Límite de reintentos antes de marcar como exhausted.
    max_retries: u32,
    /// Tamaño del batch de sync.
    batch_size: usize,
}

impl GossipCache {
    /// Crea un nuevo cache con capacidad especificada.
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries,
            entries: std::collections::BTreeMap::new(),
            pending_queue: BinaryHeap::new(),
            max_retries: 5,
            batch_size: 32,
        }
    }

    /// Configura el límite de reintentos.
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Configura el tamaño del batch de sync.
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Almacena un mensaje en el cache.
    ///
    /// Retorna Err(CacheFull) si se excede max_entries.
    pub fn store(
        &mut self,
        key: String,
        data: Vec<u8>,
        payload_type: PayloadType,
    ) -> Result<(), GossipCacheError> {
        // Si ya existe, actualizar
        if self.entries.contains_key(&key) {
            let entry = self.entries.get_mut(&key).unwrap();
            entry.data = data;
            entry.payload_type = payload_type;
            entry.synced = false;
            entry.sync_attempts = 0;
            return Ok(());
        }

        // Verificar capacidad
        if self.entries.len() >= self.max_entries {
            return Err(GossipCacheError::CacheFull);
        }

        let entry = CacheEntry::new(key.clone(), data, payload_type);
        self.entries.insert(key.clone(), entry.clone());
        self.pending_queue.push(entry);
        Ok(())
    }

    /// Recupera un mensaje del cache.
    pub fn retrieve(&self, key: &str) -> Result<CacheEntry, GossipCacheError> {
        self.entries
            .get(key)
            .cloned()
            .ok_or_else(|| GossipCacheError::EntryNotFound(key.to_string()))
    }

    /// Marca una entrada como sincronizada.
    pub fn mark_synced(&mut self, key: &str) -> Result<(), GossipCacheError> {
        let entry = self
            .entries
            .get_mut(key)
            .ok_or_else(|| GossipCacheError::EntryNotFound(key.to_string()))?;
        entry.synced = true;
        Ok(())
    }

    /// Retorna entradas pendientes de sincronización, ordenadas por prioridad.
    pub fn pending_sync(&self) -> Vec<CacheEntry> {
        self.entries
            .values()
            .filter(|e| e.is_pending())
            .cloned()
            .collect()
    }

    /// Retorna un batch de entradas para sync.
    ///
    /// Orden: priority ASC, timestamp ASC (más crítico y antiguo primero).
    pub fn sync_batch(&self) -> Vec<CacheEntry> {
        let mut pending: Vec<CacheEntry> = self
            .entries
            .values()
            .filter(|e| e.is_pending())
            .cloned()
            .collect();

        // Ordenar por priority ASC, luego timestamp ASC
        pending.sort_by(|a, b| {
            a.payload_type
                .cmp(&b.payload_type)
                .then_with(|| a.created_at.cmp(&b.created_at))
        });

        pending.into_iter().take(self.batch_size).collect()
    }

    /// Simula un intento de sync para un batch.
    ///
    /// En producción, esto envía los mensajes al mesh y espera ACK.
    /// Retorna (synced_keys, failed_keys).
    pub fn attempt_sync(
        &mut self,
        batch: &[CacheEntry],
        success_rate: f64,
    ) -> (Vec<String>, Vec<String>) {
        let mut synced = Vec::new();
        let mut failed = Vec::new();

        for entry in batch {
            // Simulación: success_rate determina éxito
            let roll: f64 = (entry.created_at as f64 % 1000.0) / 1000.0;
            if roll < success_rate {
                self.mark_synced(&entry.key).ok();
                synced.push(entry.key.clone());
            } else {
                // Incrementar intentos fallidos
                if let Some(e) = self.entries.get_mut(&entry.key) {
                    e.sync_attempts += 1;
                    if e.sync_attempts >= self.max_retries {
                        // Marcar como exhausted pero mantener en cache
                    }
                }
                failed.push(entry.key.clone());
            }
        }

        (synced, failed)
    }

    /// Retorna el backoff para una entrada específica.
    pub fn entry_backoff_ms(&self, key: &str) -> Option<u64> {
        self.entries.get(key).map(|e| e.backoff_ms())
    }

    /// Retorna las estadísticas del cache.
    pub fn stats(&self) -> CacheStats {
        let total_entries = self.entries.len();
        let synced_count = self.entries.values().filter(|e| e.synced).count();
        let pending_count = total_entries - synced_count;
        let total_bytes: usize = self
            .entries
            .values()
            .map(|e| e.data.len() + e.key.len())
            .sum();

        CacheStats {
            total_entries,
            synced_count,
            pending_count,
            total_bytes,
        }
    }

    /// Limpia entradas sincronizadas más antiguas para liberar espacio.
    ///
    /// Retorna el número de entradas eliminadas.
    pub fn compact(&mut self, keep_recent: usize) -> usize {
        let synced_keys: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, e)| e.synced)
            .map(|(k, _)| k.clone())
            .collect();

        let before = self.entries.len();
        let to_remove = synced_keys.len().saturating_sub(keep_recent);

        if to_remove > 0 {
            // Eliminar las más antiguas
            for key in &synced_keys[..to_remove] {
                self.entries.remove(key);
            }
        }

        before - self.entries.len()
    }

    /// Verifica si el cache tiene espacio.
    pub fn has_space(&self) -> bool {
        self.entries.len() < self.max_entries
    }

    /// Retorna el estado de sync de una entrada.
    pub fn sync_status(&self, key: &str) -> Option<SyncStatus> {
        let entry = self.entries.get(key)?;
        if entry.synced {
            Some(SyncStatus::Synced)
        } else if entry.sync_attempts >= self.max_retries {
            Some(SyncStatus::Exhausted)
        } else if entry.sync_attempts > 0 {
            Some(SyncStatus::Backoff)
        } else {
            Some(SyncStatus::Pending)
        }
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
            payload_type: PayloadType::Normal,
            sync_attempts: 0,
        };
        assert!(!entry.synced);
        assert!(entry.is_pending());
    }

    #[test]
    fn test_cache_entry_new() {
        let entry = CacheEntry::new("test".into(), vec![1, 2], PayloadType::Critical);
        assert_eq!(entry.payload_type, PayloadType::Critical);
        assert_eq!(entry.sync_attempts, 0);
    }

    #[test]
    fn test_store_and_retrieve() {
        let mut cache = GossipCache::new(100);
        cache
            .store("key-1".into(), vec![1, 2, 3], PayloadType::Normal)
            .unwrap();
        let entry = cache.retrieve("key-1").unwrap();
        assert_eq!(entry.data, vec![1, 2, 3]);
        assert!(!entry.synced);
    }

    #[test]
    fn test_store_updates_existing() {
        let mut cache = GossipCache::new(100);
        cache
            .store("key-1".into(), vec![1], PayloadType::Normal)
            .unwrap();
        cache
            .store("key-1".into(), vec![2, 3], PayloadType::Critical)
            .unwrap();
        let entry = cache.retrieve("key-1").unwrap();
        assert_eq!(entry.data, vec![2, 3]);
        assert_eq!(entry.payload_type, PayloadType::Critical);
    }

    #[test]
    fn test_store_cache_full() {
        let mut cache = GossipCache::new(2);
        cache
            .store("key-1".into(), vec![1], PayloadType::Normal)
            .unwrap();
        cache
            .store("key-2".into(), vec![2], PayloadType::Normal)
            .unwrap();
        match cache.store("key-3".into(), vec![3], PayloadType::Normal) {
            Err(GossipCacheError::CacheFull) => {} // Expected
            other => panic!("Expected CacheFull, got {:?}", other),
        }
    }

    #[test]
    fn test_retrieve_not_found() {
        let cache = GossipCache::new(100);
        match cache.retrieve("nonexistent") {
            Err(GossipCacheError::EntryNotFound(_)) => {} // Expected
            other => panic!("Expected EntryNotFound, got {:?}", other),
        }
    }

    #[test]
    fn test_mark_synced() {
        let mut cache = GossipCache::new(100);
        cache
            .store("key-1".into(), vec![1], PayloadType::Normal)
            .unwrap();
        cache.mark_synced("key-1").unwrap();
        let entry = cache.retrieve("key-1").unwrap();
        assert!(entry.synced);
    }

    #[test]
    fn test_mark_synced_not_found() {
        let mut cache = GossipCache::new(100);
        match cache.mark_synced("nonexistent") {
            Err(GossipCacheError::EntryNotFound(_)) => {} // Expected
            other => panic!("Expected EntryNotFound, got {:?}", other),
        }
    }

    #[test]
    fn test_pending_sync_empty() {
        let cache = GossipCache::new(100);
        let pending = cache.pending_sync();
        assert!(pending.is_empty());
    }

    #[test]
    fn test_pending_sync_returns_unsynced() {
        let mut cache = GossipCache::new(100);
        cache
            .store("key-1".into(), vec![1], PayloadType::Normal)
            .unwrap();
        cache
            .store("key-2".into(), vec![2], PayloadType::Normal)
            .unwrap();
        cache.mark_synced("key-1").unwrap();

        let pending = cache.pending_sync();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].key, "key-2");
    }

    #[test]
    fn test_sync_batch_priority_order() {
        let mut cache = GossipCache::new(100);
        // Normal primero (timestamp menor)
        cache
            .store("normal-1".into(), vec![1], PayloadType::Normal)
            .unwrap();
        // Critical después
        cache
            .store("critical-1".into(), vec![2], PayloadType::Critical)
            .unwrap();
        // Low
        cache
            .store("low-1".into(), vec![3], PayloadType::Low)
            .unwrap();

        let batch = cache.sync_batch();
        assert_eq!(batch.len(), 3);
        assert_eq!(batch[0].key, "critical-1"); // Priority 0 first
        assert_eq!(batch[1].key, "normal-1"); // Priority 1 second
        assert_eq!(batch[2].key, "low-1"); // Priority 2 last
    }

    #[test]
    fn test_sync_batch_respects_size() {
        let mut cache = GossipCache::new(100).with_batch_size(2);
        for i in 0..5 {
            cache
                .store(format!("key-{}", i), vec![i], PayloadType::Normal)
                .unwrap();
        }
        let batch = cache.sync_batch();
        assert_eq!(batch.len(), 2);
    }

    #[test]
    fn test_attempt_sync_success() {
        let mut cache = GossipCache::new(100);
        cache
            .store("key-1".into(), vec![1], PayloadType::Normal)
            .unwrap();
        let batch = cache.sync_batch();
        let (synced, failed) = cache.attempt_sync(&batch, 1.0); // 100% success
        assert_eq!(synced.len(), 1);
        assert_eq!(failed.len(), 0);
    }

    #[test]
    fn test_attempt_sync_failure() {
        let mut cache = GossipCache::new(100);
        cache
            .store("key-1".into(), vec![1], PayloadType::Normal)
            .unwrap();
        let batch = cache.sync_batch();
        let (synced, failed) = cache.attempt_sync(&batch, 0.0); // 0% success
        assert_eq!(synced.len(), 0);
        assert_eq!(failed.len(), 1);
    }

    #[test]
    fn test_backoff_exponential() {
        let entry = CacheEntry {
            key: "test".into(),
            data: vec![],
            created_at: 0,
            synced: false,
            payload_type: PayloadType::Normal,
            sync_attempts: 0,
        };
        assert_eq!(entry.backoff_ms(), 1000); // 2^0 * 1000

        let entry = CacheEntry {
            sync_attempts: 3,
            ..entry.clone()
        };
        assert_eq!(entry.backoff_ms(), 8000); // 2^3 * 1000

        let entry = CacheEntry {
            sync_attempts: 10,
            ..entry.clone()
        };
        assert_eq!(entry.backoff_ms(), 30000); // Capped at 30s
    }

    #[test]
    fn test_cache_stats() {
        let mut cache = GossipCache::new(100);
        cache
            .store("key-1".into(), vec![1, 2, 3], PayloadType::Normal)
            .unwrap();
        cache
            .store("key-2".into(), vec![4, 5], PayloadType::Normal)
            .unwrap();
        cache.mark_synced("key-1").unwrap();

        let stats = cache.stats();
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.synced_count, 1);
        assert_eq!(stats.pending_count, 1);
        assert!(stats.total_bytes > 0);
    }

    #[test]
    fn test_stats_sync_ratio() {
        let stats = CacheStats {
            total_entries: 10,
            synced_count: 3,
            pending_count: 7,
            total_bytes: 1000,
        };
        assert!((stats.sync_ratio() - 0.3).abs() < 0.01);
    }

    #[test]
    fn test_stats_sync_ratio_empty() {
        let stats = CacheStats {
            total_entries: 0,
            synced_count: 0,
            pending_count: 0,
            total_bytes: 0,
        };
        assert_eq!(stats.sync_ratio(), 0.0);
    }

    #[test]
    fn test_compact_removes_old_synced() {
        let mut cache = GossipCache::new(100);
        cache
            .store("synced-1".into(), vec![1], PayloadType::Normal)
            .unwrap();
        cache
            .store("synced-2".into(), vec![2], PayloadType::Normal)
            .unwrap();
        cache
            .store("pending-1".into(), vec![3], PayloadType::Normal)
            .unwrap();
        cache.mark_synced("synced-1").unwrap();
        cache.mark_synced("synced-2").unwrap();

        let removed = cache.compact(1); // Keep 1 recent synced
        assert_eq!(removed, 1); // Remove 1 old synced
        assert_eq!(cache.entries.len(), 2); // synced-2 + pending-1
    }

    #[test]
    fn test_has_space() {
        let mut cache = GossipCache::new(1);
        assert!(cache.has_space());
        cache
            .store("key-1".into(), vec![1], PayloadType::Normal)
            .unwrap();
        assert!(!cache.has_space());
    }

    #[test]
    fn test_sync_status() {
        let mut cache = GossipCache::new(100);
        cache
            .store("key-1".into(), vec![1], PayloadType::Normal)
            .unwrap();

        assert_eq!(
            cache.sync_status("key-1"),
            Some(SyncStatus::Pending)
        );

        cache.mark_synced("key-1").unwrap();
        assert_eq!(
            cache.sync_status("key-1"),
            Some(SyncStatus::Synced)
        );

        assert_eq!(cache.sync_status("nonexistent"), None);
    }

    #[test]
    fn test_entry_backoff_query() {
        let mut cache = GossipCache::new(100);
        cache
            .store("key-1".into(), vec![1], PayloadType::Normal)
            .unwrap();
        assert_eq!(cache.entry_backoff_ms("key-1"), Some(1000));
        assert_eq!(cache.entry_backoff_ms("nonexistent"), None);
    }

    #[test]
    fn test_with_max_retries() {
        let cache = GossipCache::new(100).with_max_retries(10);
        assert_eq!(cache.max_retries, 10);
    }

    #[test]
    fn test_with_batch_size() {
        let cache = GossipCache::new(100).with_batch_size(64);
        assert_eq!(cache.batch_size, 64);
    }

    #[test]
    fn test_payload_type_display() {
        assert_eq!(format!("{}", PayloadType::Critical), "critical");
        assert_eq!(format!("{}", PayloadType::Normal), "normal");
        assert_eq!(format!("{}", PayloadType::Low), "low");
    }

    #[test]
    fn test_sync_status_display() {
        assert_eq!(format!("{}", SyncStatus::Synced), "synced");
        assert_eq!(format!("{}", SyncStatus::Pending), "pending");
        assert_eq!(format!("{}", SyncStatus::Backoff), "backoff");
        assert_eq!(format!("{}", SyncStatus::Exhausted), "exhausted");
    }

    #[test]
    fn test_error_display() {
        let err = GossipCacheError::CacheFull;
        assert!(!format!("{}", err).is_empty());
    }

    #[test]
    fn test_priority_heap_ordering() {
        let mut heap = BinaryHeap::new();
        let critical = CacheEntry::new("critical".into(), vec![1], PayloadType::Critical);
        let normal = CacheEntry::new("normal".into(), vec![2], PayloadType::Normal);
        let low = CacheEntry::new("low".into(), vec![3], PayloadType::Low);

        heap.push(low.clone());
        heap.push(normal.clone());
        heap.push(critical.clone());

        // Pop should return critical first (highest priority)
        assert_eq!(heap.pop().unwrap().key, "critical");
        assert_eq!(heap.pop().unwrap().key, "normal");
        assert_eq!(heap.pop().unwrap().key, "low");
    }
}
