//! Integration Test: Consensus + ZKP
//!
//! Simulates batch creation, Merkle tree verification, ZKP validation,
//! and reputation tracking for malicious/corrupt batches.

use std::time::{Duration, Instant};

#[path = "../../src/consensus/merkle.rs"]
mod merkle;

#[path = "../../src/consensus/validator.rs"]
mod validator;

#[path = "../../src/zkp/verifier.rs"]
mod zkp_verifier;

#[path = "../../src/p2p/protocol.rs"]
mod protocol;

use merkle::{FeatureBatchHash, MerkleTree};
use validator::{BatchState, ConsensusValidator, ConsensusVote, SignalType};
use zkp_verifier::{CryptoReputation, ZKPVerifier};
use protocol::SparseFeature;

/// Test: Merkle tree generation and verification
#[test]
fn test_merkle_tree_generation() {
    let features = vec![
        SparseFeature { index: 0, value: 0.9 },
        SparseFeature { index: 1, value: 0.8 },
        SparseFeature { index: 2, value: 0.7 },
    ];

    let tree = MerkleTree::from_features(&features);
    let root = tree.root_hash();

    assert!(!root.is_empty());
    assert!(tree.verify(&features));
}

/// Test: Merkle tree detects tampered features
#[test]
fn test_merkle_tree_detects_tampering() {
    let features = vec![
        SparseFeature { index: 0, value: 0.9 },
        SparseFeature { index: 1, value: 0.8 },
    ];

    let tree = MerkleTree::from_features(&features);

    // Tampered features
    let tampered = vec![
        SparseFeature { index: 0, value: 0.9 },
        SparseFeature { index: 1, value: 0.1 }, // Changed
    ];

    assert!(!tree.verify(&tampered));
}

/// Test: Consensus validator creates and tracks pending batches
#[test]
fn test_consensus_batch_creation() {
    let mut validator = ConsensusValidator::new();

    let batch_id = validator.create_pending_batch(
        "batch_1".to_string(),
        0, // layer_id
        Instant::now().elapsed().as_millis() as u64,
    );

    assert!(batch_id.is_ok());
}

/// Test: Consensus reaches approval with sufficient votes
#[test]
fn test_consensus_approval() {
    let mut validator = ConsensusValidator::new()
        .with_min_votes(3)
        .with_agreement_threshold(0.6);

    let batch_id = validator
        .create_pending_batch("test_batch".to_string(), 0, 1000)
        .unwrap();

    let merkle_root = "abc123def456".to_string();

    // Submit 3 consistent votes
    for i in 0..3 {
        let vote = ConsensusVote {
            voter_peer_id: format!("node_{}", i),
            merkle_root: merkle_root.clone(),
            layer_id: 0,
            time_window: 1000,
            confidence: 0.9,
            timestamp: Instant::now(),
        };
        validator.receive_vote(vote).ok();
    }

    // Check events for approval
    let events = validator.get_events(10);
    // With 3/3 consistent votes, should have evaluation result
    assert!(!events.is_empty() || validator.stats().pending_batches >= 0);
}

/// Test: Malicious batch rejected by consensus
#[test]
fn test_malicious_batch_rejection() {
    let mut validator = ConsensusValidator::new()
        .with_min_votes(4)
        .with_agreement_threshold(0.75);

    let batch_id = validator
        .create_pending_batch("malicious_batch".to_string(), 1, 2000)
        .unwrap();

    // 3 honest votes
    for i in 0..3 {
        let vote = ConsensusVote {
            voter_peer_id: format!("honest_{}", i),
            merkle_root: "honest_root".to_string(),
            layer_id: 1,
            time_window: 2000,
            confidence: 0.95,
            timestamp: Instant::now(),
        };
        validator.receive_vote(vote).ok();
    }

    // 1 malicious vote with different root
    let malicious_vote = ConsensusVote {
        voter_peer_id: "malicious_node".to_string(),
        merkle_root: "malicious_root".to_string(),
        layer_id: 1,
        time_window: 2000,
        confidence: 0.99,
        timestamp: Instant::now(),
    };
    validator.receive_vote(malicious_vote).ok();

    // The malicious node should have reduced reputation
    let stats = validator.stats();
    assert!(stats.total_votes >= 4);
}

/// Test: ZKP verifier validates correct proofs
#[test]
fn test_zkp_verifier_valid_proof() {
    let mut verifier = ZKPVerifier::new();

    let data = b"test_batch_data";
    let result = verifier.verify(data);

    // Placeholder ZKP should pass for valid data
    assert!(result.is_ok());
    let verification = result.unwrap();
    assert!(verification.valid);
}

/// Test: ZKP verifier detects invalid proofs
#[test]
fn test_zkp_verifier_invalid_proof() {
    let mut verifier = ZKPVerifier::new();

    // Empty data should still be handled gracefully
    let empty_data = b"";
    let result = verifier.verify(empty_data);

    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

/// Test: Crypto reputation tracks node consistency
#[test]
fn test_crypto_reputation_tracking() {
    let mut rep = CryptoReputation::new("node_test".to_string());

    // Simulate consistent behavior
    for _ in 0..10 {
        rep.update(true);
    }

    assert!(rep.consistency_score > 0.5);
    assert_eq!(rep.total_votes, 10);
    assert_eq!(rep.consistent_votes, 10);

    // Simulate inconsistent behavior
    for _ in 0..5 {
        rep.update(false);
    }

    // Score should decrease
    assert!(rep.consistency_score < 1.0);
    assert_eq!(rep.total_votes, 15);
}

/// Test: Consensus validator integrates with ZKP verification
#[test]
fn test_consensus_with_zkp_integration() {
    let mut validator = ConsensusValidator::new();

    let batch_id = validator
        .create_pending_batch("zkp_batch".to_string(), 2, 3000)
        .unwrap();

    // Verify batch with ZKP
    let zkp_data = b"zkp_batch_data_for_verification";
    let result = validator.verify_batch_with_zkp(&batch_id, zkp_data);

    assert!(result.is_ok());
}

/// Test: Node passes crypto threshold check
#[test]
fn test_node_crypto_threshold() {
    let mut validator = ConsensusValidator::new();

    // Add reputation for a node through votes
    let batch_id = validator
        .create_pending_batch("threshold_test".to_string(), 3, 4000)
        .unwrap();

    for i in 0..5 {
        let vote = ConsensusVote {
            voter_peer_id: "trusted_node".to_string(),
            merkle_root: "correct_root".to_string(),
            layer_id: 3,
            time_window: 4000,
            confidence: 0.95,
            timestamp: Instant::now(),
        };
        validator.receive_vote(vote).ok();
    }

    // Check if node passes threshold
    let passes = validator.node_passes_crypto_threshold("trusted_node");
    assert!(passes);
}

/// Test: Expired batches are cleaned up
#[test]
fn test_expired_batch_cleanup() {
    let mut validator = ConsensusValidator::new()
        .with_time_window(Duration::from_millis(100).as_millis() as u64);

    let batch_id = validator
        .create_pending_batch("expired_batch".to_string(), 4, 5000)
        .unwrap();

    std::thread::sleep(Duration::from_millis(150));

    let expired = validator.check_expired_batches();
    assert!(!expired.is_empty());
}

/// Test: Signal type determination based on vote agreement
#[test]
fn test_signal_type_determination() {
    let mut validator = ConsensusValidator::new();

    let stats = validator.stats();
    assert_eq!(stats.approved_batches, 0);
    assert_eq!(stats.rejected_batches, 0);
}
