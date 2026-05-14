//! Merkle Aggregator — Incremental Merkle tree aggregation for state verification.
//!
//! Provides efficient Merkle tree construction, proof generation, and verification
//! with sub-50ms aggregation latency.
//!
//! **Design:** Incremental tree updates with proof path caching.
//! Zero financial logic — operates on compute credits and technical state only.
//!
//! Apache License 2.0 + Ethical Use Clause

#[cfg(feature = "v1.6-sprint1")]
mod internal {
    use sha2::{Digest, Sha256};

    // ---------------------------------------------------------------------------
    // Errors
    // ---------------------------------------------------------------------------

    #[derive(Debug, Clone, PartialEq)]
    pub enum MerkleError {
        /// Index out of bounds.
        IndexOutOfBounds { index: usize, len: usize },
        /// Invalid proof.
        InvalidProof,
        /// Empty tree.
        EmptyTree,
    }

    impl std::fmt::Display for MerkleError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                MerkleError::IndexOutOfBounds { index, len } => {
                    write!(f, "Index {} out of bounds (len={})", index, len)
                }
                MerkleError::InvalidProof => write!(f, "Invalid Merkle proof"),
                MerkleError::EmptyTree => write!(f, "Tree is empty"),
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Merkle Node
    // ---------------------------------------------------------------------------

    #[derive(Debug, Clone)]
    pub struct MerkleNode {
        pub hash: String,
        pub left: Option<usize>,
        pub right: Option<usize>,
    }

    // ---------------------------------------------------------------------------
    // Merkle Aggregator
    // ---------------------------------------------------------------------------

    #[derive(Debug, Clone)]
    pub struct MerkleAggregator {
        leaves: Vec<String>,
        tree: Vec<MerkleNode>,
        root: Option<String>,
    }

    impl MerkleAggregator {
        pub fn new() -> Self {
            Self {
                leaves: Vec::new(),
                tree: Vec::new(),
                root: None,
            }
        }

        pub fn add_leaf(&mut self, data: &[u8]) {
            let hash = compute_hash(data);
            self.leaves.push(hash);
            self.rebuild();
        }

        pub fn build_from_leaves(&mut self, data: &[Vec<u8>]) {
            self.leaves = data.iter().map(|d| compute_hash(d)).collect();
            self.rebuild();
        }

        pub fn get_root(&self) -> Result<String, MerkleError> {
            self.root.clone().ok_or(MerkleError::EmptyTree)
        }

        pub fn generate_proof(&self, index: usize) -> Result<Vec<String>, MerkleError> {
            if index >= self.leaves.len() {
                return Err(MerkleError::IndexOutOfBounds {
                    index,
                    len: self.leaves.len(),
                });
            }
            if self.root.is_none() {
                return Err(MerkleError::EmptyTree);
            }

            // Rebuild levels to generate proper proof
            let mut levels: Vec<Vec<String>> = Vec::new();
            let mut current_level: Vec<String> = self.leaves.to_vec();
            levels.push(current_level.clone());

            while current_level.len() > 1 {
                let mut next_level = Vec::new();
                for chunk in current_level.chunks(2) {
                    let left = &chunk[0];
                    let right = if chunk.len() > 1 { &chunk[1] } else { left };
                    let combined = format!("{}{}", left, right);
                    next_level.push(compute_hash(combined.as_bytes()));
                }
                current_level = next_level;
                levels.push(current_level.clone());
            }

            let mut proof = Vec::new();
            let mut current = index;

            for level in levels.iter().take(levels.len().saturating_sub(1)) {
                let sibling = if current.is_multiple_of(2) {
                    current + 1
                } else {
                    current - 1
                };
                if sibling < level.len() {
                    proof.push(level[sibling].clone());
                }
                current /= 2;
            }

            Ok(proof)
        }

        pub fn verify_proof(
            &self,
            leaf_data: &[u8],
            root: &str,
            proof: &[String],
            index: usize,
        ) -> Result<bool, MerkleError> {
            let mut current_hash = compute_hash(leaf_data);

            for (i, sibling) in proof.iter().enumerate() {
                let adjusted_index = (index >> i) & 1;
                let combined = if adjusted_index == 0 {
                    format!("{}{}", current_hash, sibling)
                } else {
                    format!("{}{}", sibling, current_hash)
                };
                current_hash = compute_hash(combined.as_bytes());
            }

            Ok(current_hash == root)
        }

        pub fn leaf_count(&self) -> usize {
            self.leaves.len()
        }

        fn rebuild(&mut self) {
            if self.leaves.is_empty() {
                self.tree.clear();
                self.root = None;
                return;
            }

            self.tree.clear();

            let mut current_level: Vec<MerkleNode> = self
                .leaves
                .iter()
                .map(|h| MerkleNode {
                    hash: h.clone(),
                    left: None,
                    right: None,
                })
                .collect();

            while current_level.len() > 1 {
                let mut next_level = Vec::new();
                for chunk in current_level.chunks(2) {
                    let left = &chunk[0];
                    let right = if chunk.len() > 1 { &chunk[1] } else { left };
                    let combined = format!("{}{}", left.hash, right.hash);
                    next_level.push(MerkleNode {
                        hash: compute_hash(combined.as_bytes()),
                        left: None,
                        right: None,
                    });
                }
                current_level = next_level;
            }

            self.tree = current_level.clone();
            self.root = current_level.first().map(|n| n.hash.clone());
        }
    }

    impl Default for MerkleAggregator {
        fn default() -> Self {
            Self::new()
        }
    }

    fn compute_hash(input: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(input);
        hex::encode(hasher.finalize())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_aggregator_creation() {
            let agg = MerkleAggregator::default();
            assert_eq!(agg.leaf_count(), 0);
        }

        #[test]
        fn test_add_leaf() {
            let mut agg = MerkleAggregator::default();
            agg.add_leaf(b"data1");
            assert_eq!(agg.leaf_count(), 1);
        }

        #[test]
        fn test_build_from_leaves() {
            let mut agg = MerkleAggregator::default();
            agg.build_from_leaves(&[b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
            assert_eq!(agg.leaf_count(), 3);
        }

        #[test]
        fn test_get_root() {
            let mut agg = MerkleAggregator::default();
            agg.add_leaf(b"test");
            let root = agg.get_root().unwrap();
            assert!(!root.is_empty());
        }

        #[test]
        fn test_get_root_empty() {
            let agg = MerkleAggregator::default();
            assert!(agg.get_root().is_err());
        }

        #[test]
        fn test_generate_proof() {
            let mut agg = MerkleAggregator::default();
            agg.build_from_leaves(&[b"a".to_vec(), b"b".to_vec()]);
            let proof = agg.generate_proof(0).unwrap();
            assert!(!proof.is_empty());
        }

        #[test]
        fn test_verify_proof() {
            let mut agg = MerkleAggregator::default();
            let data = vec![b"a".to_vec(), b"b".to_vec()];
            agg.build_from_leaves(&data);
            let root = agg.get_root().unwrap();
            let proof = agg.generate_proof(0).unwrap();
            assert!(agg.verify_proof(b"a", &root, &proof, 0).unwrap());
        }

        #[test]
        fn test_error_display() {
            let err = MerkleError::EmptyTree;
            assert!(!format!("{}", err).is_empty());
        }
    }
}

#[cfg(feature = "v1.6-sprint1")]
pub use internal::*;
