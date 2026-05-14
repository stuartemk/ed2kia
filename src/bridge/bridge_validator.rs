//! Bridge Validator — Light validation engine for cross-chain message verification.
//!
//! Provides hash-based validation, Merkle proof fallback, and VRF-based randomness
//! for validation sampling. Designed for sub-150ms validation latency.
//!
//! **Design:** Stateless validator with configurable thresholds and fallback strategies.
//! Zero financial logic — operates on compute credits and technical state only.
//!
//! Apache License 2.0 + Ethical Use Clause

#[cfg(feature = "v1.6-sprint1")]
mod internal {
    use sha2::{Digest, Sha256};

    // ---------------------------------------------------------------------------
    // Errors
    // ---------------------------------------------------------------------------

    /// Errors for bridge validation operations.
    #[derive(Debug, Clone, PartialEq)]
    pub enum ValidatorError {
        /// Invalid hash format.
        InvalidHash(String),
        /// Merkle proof verification failed.
        MerkleProofFailed,
        /// Validation threshold not met.
        ThresholdNotMet { current: f64, required: f64 },
        /// VRF verification failed.
        VRFVerificationFailed,
        /// Payload too large.
        PayloadTooLarge { size: usize, max: usize },
    }

    impl std::fmt::Display for ValidatorError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ValidatorError::InvalidHash(h) => write!(f, "Invalid hash: {}", h),
                ValidatorError::MerkleProofFailed => write!(f, "Merkle proof verification failed"),
                ValidatorError::ThresholdNotMet { current, required } => {
                    write!(
                        f,
                        "Threshold not met: {:.4} < {:.4}",
                        current, required
                    )
                }
                ValidatorError::VRFVerificationFailed => write!(f, "VRF verification failed"),
                ValidatorError::PayloadTooLarge { size, max } => {
                    write!(f, "Payload size {} exceeds max {}", size, max)
                }
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Config
    // ---------------------------------------------------------------------------

    /// Configuration for the bridge validator.
    #[derive(Debug, Clone)]
    pub struct ValidatorConfig {
        /// Validation threshold (0.0-1.0).
        pub threshold: f64,
        /// Enable Merkle proof fallback.
        pub enable_merkle_fallback: bool,
        /// Enable VRF-based sampling.
        pub enable_vrf_sampling: bool,
        /// Maximum payload size for validation.
        pub max_payload_size: usize,
        /// Merkle tree depth.
        pub merkle_depth: u32,
        /// VRF seed for deterministic sampling.
        pub vrf_seed: u64,
    }

    impl Default for ValidatorConfig {
        fn default() -> Self {
            Self {
                threshold: 0.67,
                enable_merkle_fallback: true,
                enable_vrf_sampling: true,
                max_payload_size: 8192,
                merkle_depth: 8,
                vrf_seed: 42,
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Validation Result
    // ---------------------------------------------------------------------------

    /// Result of a validation attempt.
    #[derive(Debug, Clone)]
    pub struct ValidationResult {
        /// Whether the message passed validation.
        pub valid: bool,
        /// Validation time in milliseconds.
        pub time_ms: u64,
        /// Whether fallback was used.
        pub used_fallback: bool,
        /// Validation score (0.0-1.0).
        pub score: f64,
        /// Error message if validation failed.
        pub error: Option<String>,
    }

    // ---------------------------------------------------------------------------
    // Merkle Node
    // ---------------------------------------------------------------------------

    /// Node in a Merkle tree.
    #[derive(Debug, Clone)]
    pub struct MerkleNode {
        pub hash: String,
        pub left: Option<Box<MerkleNode>>,
        pub right: Option<Box<MerkleNode>>,
    }

    impl MerkleNode {
        pub fn leaf(data: &[u8]) -> Self {
            Self {
                hash: compute_hash(data),
                left: None,
                right: None,
            }
        }

        pub fn internal(left: MerkleNode, right: MerkleNode) -> Self {
            let combined = format!("{}{}", left.hash, right.hash);
            Self {
                hash: compute_hash(combined.as_bytes()),
                left: Some(Box::new(left)),
                right: Some(Box::new(right)),
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Validator
    // ---------------------------------------------------------------------------

    /// Light validation engine for cross-chain messages.
    #[derive(Debug, Clone)]
    pub struct BridgeValidator {
        config: ValidatorConfig,
    }

    impl BridgeValidator {
        /// Create a new validator with the given configuration.
        pub fn new(config: ValidatorConfig) -> Self {
            Self { config }
        }

        /// Validate a message payload using hash-based verification.
        pub fn validate(&self, payload: &[u8], expected_hash: &str) -> ValidationResult {
            let start = std::time::Instant::now();

            // Check payload size
            if payload.len() > self.config.max_payload_size {
                return ValidationResult {
                    valid: false,
                    time_ms: start.elapsed().as_millis() as u64,
                    used_fallback: false,
                    score: 0.0,
                    error: Some(format!(
                        "Payload size {} exceeds max {}",
                        payload.len(),
                        self.config.max_payload_size
                    )),
                };
            }

            // Compute hash
            let actual_hash = compute_hash(payload);

            // Verify hash
            let hash_valid = actual_hash == expected_hash;

            // Compute score based on hash match
            let score = if hash_valid { 1.0 } else { 0.0 };

            // Check threshold
            let valid = score >= self.config.threshold;

            let time_ms = start.elapsed().as_millis() as u64;

            // Check if fallback needed (simulation — actual fallback triggers at >150ms)
            let used_fallback = time_ms > 150 && self.config.enable_merkle_fallback;

            ValidationResult {
                valid,
                time_ms,
                used_fallback,
                score,
                error: if valid {
                    None
                } else {
                    Some("Hash mismatch".to_string())
                },
            }
        }

        /// Validate using Merkle proof fallback.
        pub fn validate_merkle(
            &self,
            leaf_data: &[u8],
            merkle_root: &str,
            proof: &[String],
        ) -> Result<bool, ValidatorError> {
            // Build leaf hash
            let mut current_hash = compute_hash(leaf_data);

            // Walk up the Merkle tree using proof
            for sibling in proof {
                let combined = if current_hash < *sibling {
                    format!("{}{}", current_hash, sibling)
                } else {
                    format!("{}{}", sibling, current_hash)
                };
                current_hash = compute_hash(combined.as_bytes());
            }

            if current_hash == merkle_root {
                Ok(true)
            } else {
                Err(ValidatorError::MerkleProofFailed)
            }
        }

        /// Generate a VRF-based sample for deterministic validation ordering.
        pub fn vrf_sample(&self, input: &[u8]) -> u64 {
            let hash = compute_hash(input);
            // Simple VRF simulation using hash bytes
            let bytes = hex::decode(&hash[..8]).unwrap_or_default();
            u64::from_le_bytes(bytes.try_into().unwrap_or([0; 8]))
        }

        /// Build a Merkle tree from a list of leaves.
        pub fn build_merkle_tree(leaves: &[Vec<u8>]) -> Option<MerkleNode> {
            if leaves.is_empty() {
                return None;
            }

            let nodes: Vec<MerkleNode> = leaves.iter().map(|l| MerkleNode::leaf(l)).collect();

            if nodes.len() == 1 {
                return Some(nodes[0].clone());
            }

            let mut tree = nodes;
            while tree.len() > 1 {
                let mut next = Vec::new();
                let mut i = 0;
                while i + 1 < tree.len() {
                    next.push(MerkleNode::internal(tree[i].clone(), tree[i + 1].clone()));
                    i += 2;
                }
                if i < tree.len() {
                    next.push(tree[i].clone());
                }
                tree = next;
            }
            tree.into_iter().next()
        }

        /// Get the Merkle root from a tree.
        pub fn get_merkle_root(tree: &MerkleNode) -> String {
            tree.hash.clone()
        }
    }

    impl Default for BridgeValidator {
        fn default() -> Self {
            Self::new(ValidatorConfig::default())
        }
    }

    // ---------------------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------------------

    fn compute_hash(input: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(input);
        hex::encode(hasher.finalize())
    }

    // ---------------------------------------------------------------------------
    // Tests
    // ---------------------------------------------------------------------------

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_validator_creation() {
            let validator = BridgeValidator::default();
            assert_eq!(validator.config.threshold, 0.67);
        }

        #[test]
        fn test_validator_with_config() {
            let config = ValidatorConfig {
                threshold: 0.8,
                ..Default::default()
            };
            let validator = BridgeValidator::new(config);
            assert_eq!(validator.config.threshold, 0.8);
        }

        #[test]
        fn test_validate_valid_hash() {
            let validator = BridgeValidator::default();
            let payload = b"test message";
            let expected = compute_hash(payload);
            let result = validator.validate(payload, &expected);
            assert!(result.valid);
            assert!((result.score - 1.0).abs() < 0.01);
        }

        #[test]
        fn test_validate_invalid_hash() {
            let validator = BridgeValidator::default();
            let payload = b"test message";
            let result = validator.validate(payload, "invalid_hash");
            assert!(!result.valid);
            assert!((result.score - 0.0).abs() < 0.01);
        }

        #[test]
        fn test_validate_payload_too_large() {
            let config = ValidatorConfig {
                max_payload_size: 10,
                ..Default::default()
            };
            let validator = BridgeValidator::new(config);
            let payload = vec![0u8; 20];
            let result = validator.validate(&payload, "any_hash");
            assert!(!result.valid);
        }

        #[test]
        fn test_merkle_validation() {
            let validator = BridgeValidator::default();
            let leaf = b"leaf_data";
            let sibling = b"sibling";
            let leaf_hash = compute_hash(leaf);
            let sibling_hash = compute_hash(sibling);

            // Build root the same way validate_merkle does (lexicographic order)
            let combined = if leaf_hash < sibling_hash {
                format!("{}{}", leaf_hash, sibling_hash)
            } else {
                format!("{}{}", sibling_hash, leaf_hash)
            };
            let root = compute_hash(combined.as_bytes());

            let proof = vec![sibling_hash];
            let result = validator.validate_merkle(leaf, &root, &proof);
            assert!(result.is_ok());
        }

        #[test]
        fn test_merkle_validation_failed() {
            let validator = BridgeValidator::default();
            let leaf = b"wrong_data";
            let result = validator.validate_merkle(leaf, "invalid_root", &[]);
            assert!(result.is_err());
        }

        #[test]
        fn test_vrf_sample() {
            let validator = BridgeValidator::default();
            let input = b"deterministic_input";
            let sample1 = validator.vrf_sample(input);
            let sample2 = validator.vrf_sample(input);
            assert_eq!(sample1, sample2);
        }

        #[test]
        fn test_build_merkle_tree_empty() {
            let result = BridgeValidator::build_merkle_tree(&[]);
            assert!(result.is_none());
        }

        #[test]
        fn test_build_merkle_tree_single() {
            let leaves = vec![b"single".to_vec()];
            let tree = BridgeValidator::build_merkle_tree(&leaves).unwrap();
            assert!(tree.left.is_none());
            assert!(tree.right.is_none());
        }

        #[test]
        fn test_build_merkle_tree_multiple() {
            let leaves = vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec(), b"d".to_vec()];
            let tree = BridgeValidator::build_merkle_tree(&leaves).unwrap();
            assert!(tree.left.is_some());
            assert!(tree.right.is_some());
        }

        #[test]
        fn test_error_display() {
            let err = ValidatorError::InvalidHash("test".to_string());
            assert!(!format!("{}", err).is_empty());
        }

        #[test]
        fn test_config_default() {
            let config = ValidatorConfig::default();
            assert!(config.enable_merkle_fallback);
            assert!(config.enable_vrf_sampling);
        }
    }
}

#[cfg(feature = "v1.6-sprint1")]
pub use internal::*;
