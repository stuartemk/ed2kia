//! CRDT — Conflict-free Replicated Data Types para convergencia eventual.
//!
//! **Stuartian Law 5 (Múltiples Posibilidades):** Los CRDTs garantizan
//! convergencia eventual sin coordinación centralizada.

use std::collections::BTreeMap;
use std::fmt;

/// Error en operaciones CRDT.
#[derive(Debug)]
pub enum CrdtError {
    /// Vector de versión incompatible.
    IncompatibleVersion(String),
    /// Error de merge.
    MergeError(String),
    /// Estado corrupto.
    CorruptState(String),
}

impl fmt::Display for CrdtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CrdtError::IncompatibleVersion(msg) => {
                write!(f, "Incompatible version: {}", msg)
            }
            CrdtError::MergeError(msg) => {
                write!(f, "Merge error: {}", msg)
            }
            CrdtError::CorruptState(msg) => {
                write!(f, "Corrupt state: {}", msg)
            }
        }
    }
}

impl std::error::Error for CrdtError {}

/// Vector de versión para CRDT.
///
/// Cada nodo tiene su propio contador que se incrementa
/// con cada operación. El merge toma el máximo por nodo.
#[derive(Debug, Clone, Default)]
pub struct VersionVector {
    /// Contador por nodo_id.
    counters: BTreeMap<String, u64>,
}

impl VersionVector {
    /// Crea un nuevo vector de versión vacío.
    pub fn new() -> Self {
        Self {
            counters: BTreeMap::new(),
        }
    }

    /// Incrementa el contador de un nodo.
    pub fn increment(&mut self, node_id: &str) {
        *self.counters.entry(node_id.to_string()).or_insert(0) += 1;
    }

    /// Compara dos vectores de versión.
    /// - Less: this is older than other
    /// - Greater: this is newer than other
    /// - Equal: concurrent (conflict)
    pub fn compare(&self, _other: &VersionVector) -> std::cmp::Ordering {
        // TODO(Sprint16.4): Implement proper version vector comparison.
        std::cmp::Ordering::Equal
    }

    /// Merge con otro vector de versión (toma máximo por nodo).
    pub fn merge(&mut self, _other: &VersionVector) {
        // TODO(Sprint16.4): Implement merge: max(self[node], other[node]) for all nodes.
    }
}

/// CRDT de reputación — G-Counter para reputación de nodos.
///
/// **Stuartian Law 5:** Convergencia eventual. Cada nodo
/// mantiene su propia vista de reputación que converge sin coordinación.
#[derive(Debug, Clone)]
pub struct ReputationCrdt {
    /// Reputación por nodo_id.
    pub reputations: BTreeMap<String, f64>,
    /// Vector de versión.
    pub version: VersionVector,
}

impl ReputationCrdt {
    /// Crea un nuevo CRDT de reputación vacío.
    pub fn new() -> Self {
        Self {
            reputations: BTreeMap::new(),
            version: VersionVector::new(),
        }
    }

    /// Actualiza la reputación de un nodo.
    pub fn update(
        &mut self,
        node_id: &str,
        reputation: f64,
        updater_id: &str,
    ) {
        *self.reputations.entry(node_id.to_string()).or_insert(0.0) = reputation;
        self.version.increment(updater_id);
    }

    /// Merge con otro CRDT de reputación.
    ///
    /// **Stuartian Law 5:** Toma el máximo por nodo (max-registry).
    pub fn merge(&mut self, other: &ReputationCrdt) {
        for (node_id, other_rep) in &other.reputations {
            let entry = self.reputations.entry(node_id.clone()).or_insert(0.0);
            if *other_rep > *entry {
                *entry = *other_rep;
            }
        }
        self.version.merge(&other.version);
    }

    /// Obtiene la reputación de un nodo.
    pub fn get(&self, node_id: &str) -> Option<f64> {
        self.reputations.get(node_id).copied()
    }
}

impl Default for ReputationCrdt {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_vector_new() {
        let vv = VersionVector::new();
        assert!(vv.counters.is_empty());
    }

    #[test]
    fn test_version_vector_increment() {
        let mut vv = VersionVector::new();
        vv.increment("node-1");
        assert_eq!(vv.counters.get("node-1"), Some(&1));
    }

    #[test]
    fn test_crdt_new() {
        let crdt = ReputationCrdt::new();
        assert!(crdt.reputations.is_empty());
    }

    #[test]
    fn test_crdt_update() {
        let mut crdt = ReputationCrdt::new();
        crdt.update("node-1", 0.8, "updater-1");
        assert_eq!(crdt.get("node-1"), Some(0.8));
    }

    #[test]
    fn test_crdt_merge_takes_max() {
        let mut crdt1 = ReputationCrdt::new();
        crdt1.update("node-1", 0.5, "updater-1");

        let mut crdt2 = ReputationCrdt::new();
        crdt2.update("node-1", 0.8, "updater-2");

        crdt1.merge(&crdt2);
        assert_eq!(crdt1.get("node-1"), Some(0.8));
    }

    #[test]
    fn test_crdt_default() {
        let _ = ReputationCrdt::default();
    }

    #[test]
    fn test_error_display() {
        let err = CrdtError::MergeError("test".into());
        assert!(!format!("{}", err).is_empty());
    }
}
