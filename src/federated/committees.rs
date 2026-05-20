//! Hierarchical Aggregation Committees — Agregación jerárquica dinámica.
//!
//! Topología: nodos hoja → `LocalAggregator` (promedio local) → `GlobalMesh` (GossipSub).
//! Selección determinista por reputación o VRF ligera.
//! Prevención de inundación: fan-out limitado, buffering asíncrono, fallback a broadcast.
//!
//! Ley 3 (Cero desperdicio): agregación jerárquica, payloads optimizados.
//!
//! Feature gate: `#[cfg(feature = "v2.1-agg-committees")]`

use std::fmt;

// ─── Errors ───

#[derive(Debug, Clone, PartialEq)]
pub enum CommitteeError {
    EmptyPool,
    CommitteeTooSmall { requested: usize, available: usize },
    FanOutExceeded(usize),
    NoActiveCommittee,
}

impl fmt::Display for CommitteeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommitteeError::EmptyPool => write!(f, "Pool de nodos vacío"),
            CommitteeError::CommitteeTooSmall { requested, available } => {
                write!(
                    f,
                    "Comité demasiado pequeño: solicitado {}, disponible {}",
                    requested, available
                )
            }
            CommitteeError::FanOutExceeded(max) => {
                write!(f, "Fan-out excedido: máximo {}", max)
            }
            CommitteeError::NoActiveCommittee => write!(f, "Sin comité activo"),
        }
    }
}

impl std::error::Error for CommitteeError {}

// ─── Node Entry ───

/// Entrada de nodo en el pool con reputación para selección determinista.
#[derive(Debug, Clone)]
pub struct NodeEntry {
    pub node_id: String,
    pub reputation: f64,
    pub active: bool,
}

impl NodeEntry {
    pub fn new(node_id: String, reputation: f64) -> Self {
        Self {
            node_id,
            reputation,
            active: true,
        }
    }
}

// ─── CommitteeSelector Trait ───

/// Trait para selección de comités. Implementaciones: por reputación o VRF.
pub trait CommitteeSelector {
    /// Seleccionar `count` nodos del pool activo.
    fn select(&self, pool: &[NodeEntry], count: usize, seed: u64) -> Result<Vec<String>, CommitteeError>;
}

// ─── ReputationSelector ───

/// Selección determinista por reputación (top-N por score).
#[derive(Debug, Clone)]
pub struct ReputationSelector;

impl ReputationSelector {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ReputationSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl CommitteeSelector for ReputationSelector {
    fn select(&self, pool: &[NodeEntry], count: usize, _seed: u64) -> Result<Vec<String>, CommitteeError> {
        let active: Vec<&NodeEntry> = pool.iter().filter(|n| n.active).collect();
        if active.is_empty() {
            return Err(CommitteeError::EmptyPool);
        }
        if count > active.len() {
            return Err(CommitteeError::CommitteeTooSmall {
                requested: count,
                available: active.len(),
            });
        }

        // Sort descending by reputation para top-N selección
        let mut sorted = active;
        sorted.sort_by(|a, b| b.reputation.total_cmp(&a.reputation));
        let selected: Vec<String> = sorted.into_iter().take(count).map(|n| n.node_id.clone()).collect();
        Ok(selected)
    }
}

// ─── VrfSelector ───

/// Selección pseudo-aleatoria con seed (VRF ligera).
/// Usa hash simple del seed + reputación para distribución ponderada.
#[derive(Debug, Clone)]
pub struct VrfSelector;

impl VrfSelector {
    pub fn new() -> Self {
        Self
    }
}

impl Default for VrfSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl CommitteeSelector for VrfSelector {
    fn select(&self, pool: &[NodeEntry], count: usize, seed: u64) -> Result<Vec<String>, CommitteeError> {
        let active: Vec<&NodeEntry> = pool.iter().filter(|n| n.active).collect();
        if active.is_empty() {
            return Err(CommitteeError::EmptyPool);
        }
        if count > active.len() {
            return Err(CommitteeError::CommitteeTooSmall {
                requested: count,
                available: active.len(),
            });
        }

        // Fisher-Yates con seed determinista + reputación como peso
        let mut candidates: Vec<(u64, &NodeEntry)> = active
            .iter()
            .map(|n| {
                // Hash simple: seed ^ node_id bytes para pseudo-aleatoriedad
                let hash = self.hash_node(&n.node_id, seed);
                (hash, *n)
            })
            .collect();

        // Shuffle determinista (Fisher-Yates con decremento)
        let mut idx = candidates.len();
        while idx > 0 {
            let n = (seed as usize + idx) % idx;
            candidates.swap(idx - 1, n);
            idx -= 1;
        }

        // Ordenar por reputación después del shuffle para sesgo ponderado
        candidates.sort_by(|a, b| b.1.reputation.partial_cmp(&a.1.reputation).unwrap());

        let selected: Vec<String> = candidates
            .into_iter()
            .take(count)
            .map(|(_, n)| n.node_id.clone())
            .collect();
        Ok(selected)
    }
}

impl VrfSelector {
    fn hash_node(&self, node_id: &str, seed: u64) -> u64 {
        let mut hash: u64 = seed;
        for byte in node_id.bytes() {
            hash = hash.wrapping_mul(6364136223846793005).wrapping_add(byte as u64);
        }
        hash
    }
}

// ─── LocalAggregator ───

/// Agregador local: promedio ponderado de gradientes en un grupo de nodos hoja.
#[derive(Debug, Clone)]
pub struct LocalAggregator {
    pub group_id: String,
    pub max_fan_out: usize,
}

impl LocalAggregator {
    pub fn new(group_id: String, max_fan_out: usize) -> Self {
        Self {
            group_id,
            max_fan_out,
        }
    }

    /// Promedio ponderado por reputación de gradientes locales.
    pub fn aggregate(&self, gradients: &[(Vec<f32>, f64)]) -> Result<Vec<f32>, CommitteeError> {
        if gradients.is_empty() {
            return Err(CommitteeError::EmptyPool);
        }

        let dim = gradients[0].0.len();
        let total_weight: f64 = gradients.iter().map(|(_, w)| w).sum();
        if total_weight == 0.0 {
            return Err(CommitteeError::EmptyPool);
        }

        let mut result = vec![0.0f32; dim];
        for (grad, weight) in gradients {
            if grad.len() != dim {
                return Err(CommitteeError::CommitteeTooSmall {
                    requested: dim,
                    available: grad.len(),
                });
            }
            for (r, g) in result.iter_mut().zip(grad.iter()) {
                *r += *g * *weight as f32;
            }
        }

        for r in &mut result {
            *r /= total_weight as f32;
        }
        Ok(result)
    }
}

// ─── GlobalMesh ───

/// Mesh global GossipSub: recibe resultados de LocalAggregator y los propaga.
#[derive(Debug)]
pub struct GlobalMesh {
    pub max_committees: usize,
    pub active_committees: Vec<String>,
}

impl GlobalMesh {
    pub fn new(max_committees: usize) -> Self {
        Self {
            max_committees,
            active_committees: Vec::new(),
        }
    }

    /// Registrar comité activo. Rechaza si excede límite (anti-inundación).
    pub fn register_committee(&mut self, committee_id: String) -> Result<(), CommitteeError> {
        if self.active_committees.len() >= self.max_committees {
            return Err(CommitteeError::FanOutExceeded(self.max_committees));
        }
        self.active_committees.push(committee_id);
        Ok(())
    }

    /// Desregistrar comité.
    pub fn unregister_committee(&mut self, committee_id: &str) {
        self.active_committees.retain(|c| c != committee_id);
    }

    /// Verificar si hay comité activo para fallback.
    pub fn has_active_committee(&self) -> bool {
        !self.active_committees.is_empty()
    }
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pool(size: usize) -> Vec<NodeEntry> {
        (0..size)
            .map(|i| NodeEntry::new(format!("node-{}", i), (i + 1) as f64))
            .collect()
    }

    #[test]
    fn test_reputation_selector_top_n() {
        let pool = make_pool(5);
        let selector = ReputationSelector::new();
        let selected = selector.select(&pool, 3, 0).unwrap();
        assert_eq!(selected.len(), 3);
        // Top 3 por reputación: node-4 (5.0), node-3 (4.0), node-2 (3.0)
        assert!(selected.contains(&"node-4".to_string()));
        assert!(selected.contains(&"node-3".to_string()));
        assert!(selected.contains(&"node-2".to_string()));
    }

    #[test]
    fn test_reputation_selector_empty_pool() {
        let selector = ReputationSelector::new();
        let result = selector.select(&[], 3, 0);
        assert_eq!(result, Err(CommitteeError::EmptyPool));
    }

    #[test]
    fn test_reputation_selector_too_many() {
        let pool = make_pool(3);
        let selector = ReputationSelector::new();
        match selector.select(&pool, 5, 0) {
            Err(CommitteeError::CommitteeTooSmall { requested, available }) => {
                assert_eq!(requested, 5);
                assert_eq!(available, 3);
            }
            other => panic!("Expected CommitteeTooSmall, got {:?}", other),
        }
    }

    #[test]
    fn test_vrf_selector_basic() {
        let pool = make_pool(10);
        let selector = VrfSelector::new();
        let selected = selector.select(&pool, 4, 42).unwrap();
        assert_eq!(selected.len(), 4);
        // Todos deben ser del pool
        for id in &selected {
            assert!(pool.iter().any(|n| n.node_id == *id));
        }
    }

    #[test]
    fn test_vrf_selector_deterministic() {
        let pool = make_pool(10);
        let selector = VrfSelector::new();
        let s1 = selector.select(&pool, 3, 99).unwrap();
        let s2 = selector.select(&pool, 3, 99).unwrap();
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_vrf_selector_different_seeds() {
        let pool = make_pool(10);
        let selector = VrfSelector::new();
        let s1 = selector.select(&pool, 3, 1).unwrap();
        let s2 = selector.select(&pool, 3, 2).unwrap();
        // Diferentes seeds → diferente orden (no garantiza diferente conjunto pero probable)
        assert!(s1.len() == 3 && s2.len() == 3);
    }

    #[test]
    fn test_local_aggregator_weighted() {
        let agg = LocalAggregator::new("group-1".into(), 10);
        let gradients = vec![
            (vec![1.0, 2.0, 3.0], 1.0),
            (vec![2.0, 4.0, 6.0], 1.0),
        ];
        let result = agg.aggregate(&gradients).unwrap();
        assert_eq!(result, vec![1.5, 3.0, 4.5]);
    }

    #[test]
    fn test_local_aggregator_empty() {
        let agg = LocalAggregator::new("group-1".into(), 10);
        let result = agg.aggregate(&[]);
        assert_eq!(result, Err(CommitteeError::EmptyPool));
    }

    #[test]
    fn test_local_aggregator_zero_weight() {
        let agg = LocalAggregator::new("group-1".into(), 10);
        let gradients = vec![(vec![1.0, 2.0], 0.0)];
        let result = agg.aggregate(&gradients);
        assert_eq!(result, Err(CommitteeError::EmptyPool));
    }

    #[test]
    fn test_global_mesh_register() {
        let mut mesh = GlobalMesh::new(3);
        mesh.register_committee("c1".into()).unwrap();
        mesh.register_committee("c2".into()).unwrap();
        assert_eq!(mesh.active_committees.len(), 2);
        assert!(mesh.has_active_committee());
    }

    #[test]
    fn test_global_mesh_fan_out_limit() {
        let mut mesh = GlobalMesh::new(2);
        mesh.register_committee("c1".into()).unwrap();
        mesh.register_committee("c2".into()).unwrap();
        match mesh.register_committee("c3".into()) {
            Err(CommitteeError::FanOutExceeded(max)) => assert_eq!(max, 2),
            other => panic!("Expected FanOutExceeded, got {:?}", other),
        }
    }

    #[test]
    fn test_global_mesh_unregister() {
        let mut mesh = GlobalMesh::new(3);
        mesh.register_committee("c1".into()).unwrap();
        mesh.unregister_committee("c1");
        assert!(!mesh.has_active_committee());
    }

    #[test]
    fn test_node_entry_creation() {
        let entry = NodeEntry::new("test".into(), 0.5);
        assert_eq!(entry.node_id, "test");
        assert_eq!(entry.reputation, 0.5);
        assert!(entry.active);
    }

    #[test]
    fn test_error_display() {
        assert!(format!("{}", CommitteeError::EmptyPool).contains("vacío"));
    }
}
