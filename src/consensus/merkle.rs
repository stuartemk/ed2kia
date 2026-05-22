//! Merkle Tree - Generación y verificación de raíces Merkle para batches de features
//!
//! Proporciona integridad criptográfica para batches de sparse features,
//! permitiendo que múltiples nodos verifiquen coherencia sin revelar datos completos.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// ============================================================================
// Merkle Tree
// ============================================================================

/// Nodo del árbol Merkle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleNode {
    /// Hash del nodo
    pub hash: String,
    /// Si es hoja (dato original) o interno (hash combinado)
    pub is_leaf: bool,
    /// Hash izquierdo (solo nodos internos)
    pub left: Option<Box<MerkleNode>>,
    /// Hash derecho (solo nodos internos)
    pub right: Option<Box<MerkleNode>>,
}

/// Árbol Merkle completo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleTree {
    /// Raíz del árbol
    pub root: MerkleNode,
    /// Número de hojas
    pub leaf_count: usize,
    /// Profundidad del árbol
    pub depth: usize,
}

impl MerkleTree {
    /// Construir árbol Merkle desde datos
    ///
    /// # Arguments
    /// * `data` - Lista de bytes (hashes de features individuales)
    ///
    /// # Returns
    /// `MerkleTree` con raíz calculada
    pub fn from_data(data: Vec<Vec<u8>>) -> Result<Self> {
        if data.is_empty() {
            return Err(anyhow::anyhow!("MerkleTree requiere al menos un dato"));
        }

        let leaf_count = data.len();

        // Crear nodos hoja
        let leaves: Vec<MerkleNode> = data
            .iter()
            .map(|d| {
                let hash = hash_bytes(d);
                MerkleNode {
                    hash,
                    is_leaf: true,
                    left: None,
                    right: None,
                }
            })
            .collect();

        // Construir árbol bottom-up
        let root = Self::build_tree(leaves)?;
        let depth = Self::calculate_depth(leaf_count);

        Ok(Self {
            root,
            leaf_count,
            depth,
        })
    }

    /// Construir árbol desde nodos hoja
    fn build_tree(mut leaves: Vec<MerkleNode>) -> Result<MerkleNode> {
        if leaves.len() == 1 {
            return Ok(leaves.remove(0));
        }

        // Si número impar, duplicar última hoja
        if !leaves.len().is_multiple_of(2) {
            let last = leaves.last().unwrap().clone();
            leaves.push(last);
        }

        let mut current_level = Vec::new();

        // Combinar pares de nodos
        for chunk in leaves.chunks(2) {
            let left = &chunk[0];
            let right = &chunk[1];

            let combined_hash = hash_combined(&left.hash, &right.hash);

            let parent = MerkleNode {
                hash: combined_hash,
                is_leaf: false,
                left: Some(Box::new(left.clone())),
                right: Some(Box::new(right.clone())),
            };

            current_level.push(parent);
        }

        // Recursión hasta llegar a la raíz
        Self::build_tree(current_level)
    }

    /// Calcular profundidad del árbol
    fn calculate_depth(leaf_count: usize) -> usize {
        if leaf_count <= 1 {
            return 0;
        }
        let mut depth = 0;
        let mut n = leaf_count;
        while n > 1 {
            n = n.div_ceil(2);
            depth += 1;
        }
        depth
    }

    /// Obtener hash de la raíz
    pub fn root_hash(&self) -> String {
        self.root.hash.clone()
    }

    /// Generar proof de inclusión para un índice de hoja
    ///
    /// Retorna la lista de hashes hermanos necesarios para verificar
    /// que un dato específico está incluido en el árbol.
    pub fn generate_proof(&self, leaf_index: usize) -> Result<Vec<String>> {
        if leaf_index >= self.leaf_count {
            return Err(anyhow::anyhow!(
                "Índice {} fuera de rango (leaf_count={})",
                leaf_index,
                self.leaf_count
            ));
        }

        let mut proof = Vec::new();
        self.collect_proof(&self.root, leaf_index, 0, &mut proof);
        Ok(proof)
    }

    /// Recursión para recolectar proof
    #[allow(clippy::only_used_in_recursion)]
    fn collect_proof(
        &self,
        node: &MerkleNode,
        target_index: usize,
        current_index: usize,
        proof: &mut Vec<String>,
    ) {
        if node.is_leaf {
            return;
        }

        let left = node.left.as_ref().unwrap();
        let right = node.right.as_ref().unwrap();

        // Determinar si el target está en el subárbol izquierdo o derecho
        let left_leaf_count = self.leaf_count_at_depth(node, self.depth);

        if target_index < left_leaf_count {
            // Target en izquierdo → agregar hash derecho al proof
            proof.push(right.hash.clone());
            self.collect_proof(left, target_index, current_index, proof);
        } else {
            // Target en derecho → agregar hash izquierdo al proof
            proof.push(left.hash.clone());
            self.collect_proof(right, target_index - left_leaf_count, current_index, proof);
        }
    }

    /// Estimar número de hojas en un subárbol
    fn leaf_count_at_depth(&self, _node: &MerkleNode, depth: usize) -> usize {
        // Simplificación: dividir hojas equitativamente
        if depth == 0 {
            return 1;
        }
        self.leaf_count.div_ceil(2)
    }

    /// Verificar proof de inclusión
    ///
    /// # Arguments
    /// * `leaf_hash` - Hash del dato original
    /// * `proof` - Lista de hashes hermanos del proof
    /// * `root_hash` - Hash esperado de la raíz
    /// * `position` - Posición izquierda/derecha del leaf
    ///
    /// # Returns
    /// `true` si el proof es válido
    pub fn verify_proof(
        leaf_hash: &str,
        proof: &[String],
        expected_root: &str,
        position: usize,
    ) -> bool {
        let mut current_hash = leaf_hash.to_string();
        let mut current_position = position;

        for sibling_hash in proof {
            let (left, right) = if current_position.is_multiple_of(2) {
                (&current_hash, sibling_hash)
            } else {
                (sibling_hash, &current_hash)
            };

            current_hash = hash_combined(left, right);
            current_position /= 2;
        }

        current_hash == expected_root
    }
}

// ============================================================================
// Funciones de Hash
// ============================================================================

/// Hash de bytes usando SHA-256
pub fn hash_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Hash combinado de dos hashes (padre Merkle)
pub fn hash_combined(left: &str, right: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(left.as_bytes());
    hasher.update(right.as_bytes());
    hex::encode(hasher.finalize())
}

/// Hash de string
pub fn hash_string(s: &str) -> String {
    hash_bytes(s.as_bytes())
}

// ============================================================================
// Feature Batch Hashing
// ============================================================================

/// Hash de un batch de features para Merkle tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureBatchHash {
    /// ID del batch
    pub batch_id: String,
    /// Hash individual de cada feature en el batch
    pub feature_hashes: Vec<String>,
    /// Raíz Merkle del batch
    pub merkle_root: String,
    /// Número de features en el batch
    pub feature_count: usize,
    /// Timestamp (Unix epoch ms)
    pub timestamp: u64,
}

impl FeatureBatchHash {
    /// Crear hash de batch desde features serializadas
    pub fn from_serialized_features(
        batch_id: String,
        serialized_features: Vec<Vec<u8>>,
    ) -> Result<Self> {
        let feature_hashes: Vec<String> =
            serialized_features.iter().map(|f| hash_bytes(f)).collect();

        // Construir Merkle tree
        let tree = MerkleTree::from_data(serialized_features)
            .with_context(|| "Error constructing Merkle tree for feature batch")?;

        Ok(Self {
            batch_id,
            feature_hashes,
            merkle_root: tree.root_hash(),
            feature_count: tree.leaf_count,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        })
    }

    /// Verificar que un feature hash está incluido en el batch
    pub fn verify_feature_inclusion(&self, feature_hash: &str) -> bool {
        self.feature_hashes.contains(&feature_hash.to_string())
    }

    /// Comparar raíz Merkle con otro batch
    pub fn roots_match(&self, other: &FeatureBatchHash) -> bool {
        self.merkle_root == other.merkle_root
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_bytes() {
        let hash = hash_bytes(b"hello");
        assert_eq!(hash.len(), 64); // SHA-256 = 32 bytes = 64 hex chars
    }

    #[test]
    fn test_merkle_tree_single_leaf() {
        let data = vec![vec![1u8, 2, 3]];
        let tree = MerkleTree::from_data(data).unwrap();
        assert_eq!(tree.leaf_count, 1);
        assert_eq!(tree.depth, 0);
    }

    #[test]
    fn test_merkle_tree_multiple_leaves() {
        let data = vec![vec![1u8], vec![2u8], vec![3u8], vec![4u8]];
        let tree = MerkleTree::from_data(data).unwrap();
        assert_eq!(tree.leaf_count, 4);
        assert!(tree.depth > 0);
    }

    #[test]
    fn test_merkle_tree_odd_leaves() {
        let data = vec![vec![1u8], vec![2u8], vec![3u8]];
        let tree = MerkleTree::from_data(data).unwrap();
        assert_eq!(tree.leaf_count, 3);
    }

    #[test]
    fn test_feature_batch_hash() {
        let features = vec![vec![1u8, 2], vec![3u8, 4], vec![5u8, 6]];
        let batch =
            FeatureBatchHash::from_serialized_features("test-batch".to_string(), features).unwrap();
        assert_eq!(batch.feature_count, 3);
        assert!(!batch.merkle_root.is_empty());
    }

    #[test]
    fn test_same_data_same_root() {
        let data = vec![vec![1u8], vec![2u8], vec![3u8], vec![4u8]];
        let tree1 = MerkleTree::from_data(data.clone()).unwrap();
        let tree2 = MerkleTree::from_data(data).unwrap();
        assert_eq!(tree1.root_hash(), tree2.root_hash());
    }
}
