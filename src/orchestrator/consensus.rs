//! Consensus Engine — Deterministic majority rule with f32 epsilon tolerance.
//!
//! Feature-gated behind `v2.1-consensus-engine`. Provides O(N) consensus
//! validation by grouping results via index hash, then comparing sparse values
//! with epsilon tolerance.
//!
//! **Status:** Functional with unit tests.
//! **License:** Apache 2.0 + Ethical Use Clause

use std::collections::HashMap;

use crate::protocol::audit_payloads::AuditResultPayload;

/// Groups results by a hash of their sparse indices for O(N) comparison.
///
/// Instead of pairwise O(N²) comparison, we hash the indices vector and group
/// results that share the same indices. Then we only compare values within
/// each group using epsilon tolerance.
fn index_hash(indices: &[usize]) -> u64 {
    // Simple FNV-1a inspired hash for index vectors
    let mut hash: u64 = 0xcbf29ce484222325;
    for &idx in indices {
        hash ^= idx as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Validates consensus among multiple audit results using majority rule.
///
/// **Algorithm:**
/// 1. Group results by hash of `sparse_indices` (O(N))
/// 2. For the largest group, verify all members match within `epsilon` tolerance
/// 3. If group size >= `threshold = (N / 2) + 1`, return the consensus result
/// 4. Otherwise return `None` (no consensus)
///
/// # Arguments
/// * `results` — Vec of audit results from different peers
/// * `epsilon` — f32 tolerance for value comparison (e.g., 1e-4)
///
/// # Returns
/// `Some(AuditResultPayload)` if consensus reached, `None` otherwise
pub fn validate_consensus(
    results: Vec<AuditResultPayload>,
    epsilon: f32,
) -> Option<AuditResultPayload> {
    if results.is_empty() {
        return None;
    }

    let threshold = (results.len() / 2) + 1;

    // Step 1: Group by index hash
    let mut groups: HashMap<u64, Vec<&AuditResultPayload>> = HashMap::new();
    for result in &results {
        let hash = index_hash(&result.sparse_indices);
        groups.entry(hash).or_default().push(result);
    }

    // Step 2: Find the largest group and verify value tolerance
    for (_hash, group) in groups {
        if group.len() < threshold {
            continue;
        }

        let reference = group[0];

        // Step 3: Verify all members match within epsilon
        let all_match = group.iter().skip(1).all(|other| {
            // Indices must be identical (same hash group)
            if reference.sparse_indices != other.sparse_indices {
                return false;
            }
            // Values must be within epsilon tolerance
            if reference.sparse_values.len() != other.sparse_values.len() {
                return false;
            }
            reference.sparse_values.iter().zip(other.sparse_values.iter()).all(|(a, b)| {
                (a - b).abs() < epsilon
            })
        });

        if all_match {
            return Some(group[0].clone());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_result(task_id: Uuid, values: Vec<f32>, indices: Vec<usize>, node: &str) -> AuditResultPayload {
        AuditResultPayload {
            task_id,
            sparse_values: values,
            sparse_indices: indices,
            compute_time_ms: 100,
            node_id: node.to_string(),
            error: None,
        }
    }

    #[test]
    fn test_consensus_single_result() {
        let task_id = Uuid::new_v4();
        let result = make_result(task_id, vec![1.0, 2.0], vec![0, 1], "peer-1");
        let consensus = validate_consensus(vec![result], 1e-4);
        assert!(consensus.is_some());
        assert_eq!(consensus.unwrap().node_id, "peer-1");
    }

    #[test]
    fn test_consensus_majority_match() {
        let task_id = Uuid::new_v4();
        let results = vec![
            make_result(task_id, vec![1.0, 2.0], vec![0, 1], "peer-1"),
            make_result(task_id, vec![1.0, 2.0], vec![0, 1], "peer-2"),
            make_result(task_id, vec![9.0, 9.0], vec![5, 6], "peer-3"), // outlier
        ];
        let consensus = validate_consensus(results, 1e-4);
        assert!(consensus.is_some());
    }

    #[test]
    fn test_consensus_no_majority() {
        let task_id = Uuid::new_v4();
        let results = vec![
            make_result(task_id, vec![1.0, 2.0], vec![0, 1], "peer-1"),
            make_result(task_id, vec![3.0, 4.0], vec![2, 3], "peer-2"),
            make_result(task_id, vec![5.0, 6.0], vec![4, 5], "peer-3"),
        ];
        let consensus = validate_consensus(results, 1e-4);
        assert!(consensus.is_none());
    }

    #[test]
    fn test_consensus_epsilon_tolerance() {
        let task_id = Uuid::new_v4();
        let results = vec![
            make_result(task_id, vec![1.0, 2.0], vec![0, 1], "peer-1"),
            make_result(task_id, vec![1.00001, 2.00001], vec![0, 1], "peer-2"),
            make_result(task_id, vec![1.00002, 2.00002], vec![0, 1], "peer-3"),
        ];
        let consensus = validate_consensus(results, 1e-3);
        assert!(consensus.is_some());
    }

    #[test]
    fn test_consensus_epsilon_rejection() {
        let task_id = Uuid::new_v4();
        let results = vec![
            make_result(task_id, vec![1.0, 2.0], vec![0, 1], "peer-1"),
            make_result(task_id, vec![1.1, 2.1], vec![0, 1], "peer-2"),
            make_result(task_id, vec![1.0, 2.0], vec![0, 1], "peer-3"),
        ];
        let consensus = validate_consensus(results, 1e-4);
        assert!(consensus.is_none());
    }

    #[test]
    fn test_consensus_empty_results() {
        let consensus = validate_consensus(Vec::new(), 1e-4);
        assert!(consensus.is_none());
    }

    #[test]
    fn test_consensus_threshold_calculation() {
        // 4 results, threshold = 3
        let task_id = Uuid::new_v4();
        let results = vec![
            make_result(task_id, vec![1.0], vec![0], "peer-1"),
            make_result(task_id, vec![1.0], vec![0], "peer-2"),
            make_result(task_id, vec![9.0], vec![5], "peer-3"),
            make_result(task_id, vec![9.0], vec![5], "peer-4"),
        ];
        // 2 vs 2 split, threshold = 3, no consensus
        let consensus = validate_consensus(results, 1e-4);
        assert!(consensus.is_none());
    }

    #[test]
    fn test_consensus_exact_half_plus_one() {
        // 5 results, threshold = 3
        let task_id = Uuid::new_v4();
        let results = vec![
            make_result(task_id, vec![1.0], vec![0], "peer-1"),
            make_result(task_id, vec![1.0], vec![0], "peer-2"),
            make_result(task_id, vec![1.0], vec![0], "peer-3"),
            make_result(task_id, vec![9.0], vec![5], "peer-4"),
            make_result(task_id, vec![9.0], vec![5], "peer-5"),
        ];
        let consensus = validate_consensus(results, 1e-4);
        assert!(consensus.is_some());
    }

    #[test]
    fn test_index_hash_deterministic() {
        let h1 = index_hash(&[0, 1, 2]);
        let h2 = index_hash(&[0, 1, 2]);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_index_hash_different() {
        let h1 = index_hash(&[0, 1, 2]);
        let h2 = index_hash(&[3, 4, 5]);
        assert_ne!(h1, h2);
    }
}
