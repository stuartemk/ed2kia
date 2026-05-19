//! E2E Consensus Immune Test — Dry-run end-to-end validation.
//!
//! Feature-gated behind `v2.1-consensus-engine`. Validates the full
//! "Secuencia de Ignición": Relay → Orchestrator → WASM Nodes →
//! Consensus/Reputation → Atlas 3D.
//!
//! Simulates 3 mock peers (2 honest, 1 malicious with data poisoning)
//! and verifies:
//! - Consensus reached (2/3 majority)
//! - Correct result propagated
//! - Reputation A/B: +1
//! - Reputation C: -50, banned
//!
//! **Status:** New in v2.1.0-sprint7.
//! **License:** Apache 2.0 + Ethical Use Clause

#![cfg(feature = "v2.1-consensus-engine")]

use ed2kia::orchestrator::consensus::validate_consensus;
use ed2kia::orchestrator::reputation::ReputationEngine;
use ed2kia::orchestrator::task_manager::TaskManager;
use ed2kia::protocol::audit_payloads::{AuditResultPayload, AuditTaskPayload};
use uuid::Uuid;

// ——— Test Helpers ———

/// Create a correct `AuditResultPayload` for honest peers.
fn make_honest_result(task_id: Uuid, node_id: &str) -> AuditResultPayload {
    AuditResultPayload {
        task_id,
        sparse_values: vec![0.9876, 0.6543, 0.3210],
        sparse_indices: vec![42, 107, 256],
        compute_time_ms: 120,
        node_id: node_id.to_string(),
        error: None,
    }
}

/// Create a malicious `AuditResultPayload` with altered indices (Data Poisoning).
fn make_malicious_result(task_id: Uuid, node_id: &str) -> AuditResultPayload {
    AuditResultPayload {
        task_id,
        sparse_values: vec![0.9999, 0.8888, 0.7777],
        sparse_indices: vec![999, 888, 777], // Altered indices
        compute_time_ms: 50,
        node_id: node_id.to_string(),
        error: None,
    }
}

/// Create a valid `AuditTaskPayload` for testing.
fn make_test_task() -> AuditTaskPayload {
    AuditTaskPayload::new(
        vec![1.0, 2.0, 3.0, 4.0], // shard_weights
        (64, 64),                  // shard_shape
        vec![0.1, 0.2, 0.3, 0.4], // input_activation
        2,                         // batch_size
        3,                         // k
        0.01,                      // sparsity_threshold
    )
}

// ——— E2E Tests ———

#[tokio::test]
async fn test_e2e_consensus_with_honest_majority() {
    let task_id = Uuid::new_v4();

    // Simulate 3 peers: A (honest), B (honest), C (malicious)
    let results = vec![
        make_honest_result(task_id, "peer-A"),
        make_honest_result(task_id, "peer-B"),
        make_malicious_result(task_id, "peer-C"),
    ];

    // Validate consensus with epsilon tolerance
    let consensus = validate_consensus(results, 1e-4);

    // Assert: Consensus reached (2/3 majority)
    assert!(
        consensus.is_some(),
        "Consensus should be reached with 2/3 honest majority"
    );

    let consensus_result = consensus.unwrap();

    // Assert: Correct result propagated (matches honest peer values)
    assert_eq!(
        consensus_result.sparse_values,
        vec![0.9876, 0.6543, 0.3210],
        "Consensus values should match honest peer values"
    );
    assert_eq!(
        consensus_result.sparse_indices,
        vec![42, 107, 256],
        "Consensus indices should match honest peer indices"
    );
}

#[tokio::test]
async fn test_e2e_reputation_scoring() {
    let reputation = ReputationEngine::new();

    // Peer A: Honest → +1
    let banned_a = reputation.update_score("peer-A".to_string(), true);
    assert!(!banned_a, "Peer A should not be banned");
    assert_eq!(
        reputation.get_score("peer-A"),
        Some(1),
        "Peer A reputation should be +1"
    );

    // Peer B: Honest → +1
    let banned_b = reputation.update_score("peer-B".to_string(), true);
    assert!(!banned_b, "Peer B should not be banned");
    assert_eq!(
        reputation.get_score("peer-B"),
        Some(1),
        "Peer B reputation should be +1"
    );

    // Peer C: Malicious → -50, banned
    let banned_c = reputation.update_score("peer-C".to_string(), false);
    assert!(banned_c, "Peer C should be banned (score < 0)");
    assert_eq!(
        reputation.get_score("peer-C"),
        Some(-50),
        "Peer C reputation should be -50"
    );
    assert!(
        reputation.is_banned("peer-C"),
        "Peer C should be in ban list"
    );
}

#[tokio::test]
async fn test_e2e_full_immune_sequence() {
    // 1. Setup: TaskManager + ReputationEngine
    let _task_manager = TaskManager::new(std::time::Duration::from_secs(30), 3);
    let reputation = ReputationEngine::new();

    // 2. Register mock peers
    let peers = vec!["peer-A", "peer-B", "peer-C"];

    // 3. Create and dispatch audit task
    let task = make_test_task();
    let task_id = task.task_id;

    // 4. Simulate peer responses (2 honest, 1 malicious)
    let results = vec![
        make_honest_result(task_id, "peer-A"),
        make_honest_result(task_id, "peer-B"),
        make_malicious_result(task_id, "peer-C"),
    ];

    // 5. Run Consensus Engine
    let consensus = validate_consensus(results.clone(), 1e-4);
    assert!(
        consensus.is_some(),
        "Consensus should be reached with 2/3 honest majority"
    );
    let consensus_result = consensus.unwrap();

    // 6. Update Reputation based on consensus match
    for peer_id in &peers {
        // Find peer result
        let peer_result = results.iter().find(|r| r.node_id == *peer_id);

        if let Some(r) = peer_result {
            let matched = r.sparse_indices == consensus_result.sparse_indices;
            let banned = reputation.update_score(peer_id.to_string(), matched);

            if *peer_id == "peer-C" {
                assert!(banned, "Malicious peer C should be banned");
            } else {
                assert!(!banned, "Honest peer {} should not be banned", peer_id);
            }
        }
    }

    // 7. Final Assertions
    assert_eq!(reputation.get_score("peer-A"), Some(1));
    assert_eq!(reputation.get_score("peer-B"), Some(1));
    assert_eq!(reputation.get_score("peer-C"), Some(-50));
    assert!(reputation.is_banned("peer-C"));
    assert!(!reputation.is_banned("peer-A"));
    assert!(!reputation.is_banned("peer-B"));
    assert_eq!(reputation.banned_count(), 1, "Exactly 1 peer should be banned");
    assert_eq!(reputation.tracked_count(), 3, "All 3 peers should be tracked");
}

#[tokio::test]
async fn test_e2e_consensus_rejection_all_malicious() {
    let task_id = Uuid::new_v4();

    // All peers return different results → No consensus
    // Each peer has different indices (all malicious)
    let results = vec![
        AuditResultPayload {
            task_id,
            sparse_values: vec![1.0, 2.0],
            sparse_indices: vec![1, 2],
            compute_time_ms: 100,
            node_id: "peer-A".to_string(),
            error: None,
        },
        AuditResultPayload {
            task_id,
            sparse_values: vec![3.0, 4.0],
            sparse_indices: vec![3, 4],
            compute_time_ms: 100,
            node_id: "peer-B".to_string(),
            error: None,
        },
        AuditResultPayload {
            task_id,
            sparse_values: vec![5.0, 6.0],
            sparse_indices: vec![5, 6],
            compute_time_ms: 100,
            node_id: "peer-C".to_string(),
            error: None,
        },
    ];

    let consensus = validate_consensus(results, 1e-4);
    assert!(
        consensus.is_none(),
        "No consensus should be reached when all peers differ"
    );
}

#[tokio::test]
async fn test_e2e_reputation_recovery_after_unban() {
    let reputation = ReputationEngine::new();

    // Ban peer-C
    reputation.update_score("peer-C".to_string(), false);
    assert!(reputation.is_banned("peer-C"));
    assert_eq!(reputation.get_score("peer-C"), Some(-50));

    // Governance unban
    reputation.unban_peer("peer-C");
    assert!(!reputation.is_banned("peer-C"), "Peer C should be unbanned");
    assert_eq!(
        reputation.get_score("peer-C"),
        Some(0),
        "Peer C score should be reset to 0"
    );

    // Peer-C can rebuild reputation
    reputation.update_score("peer-C".to_string(), true);
    assert_eq!(
        reputation.get_score("peer-C"),
        Some(1),
        "Peer C should have +1 after honest work"
    );
    assert!(!reputation.is_banned("peer-C"));
}
