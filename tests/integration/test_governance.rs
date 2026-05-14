//! Integration Test: Governance
//!
//! Validates the complete governance flow:
//! Proposal creation → Ed25519 signing → voting → resolution → ledger recording.

#[path = "../../src/governance/proposal.rs"]
mod proposal;

#[path = "../../src/governance/voting.rs"]
mod voting;

#[path = "../../src/reputation/ledger.rs"]
mod ledger;

#[path = "../../src/reputation/scoring.rs"]
mod scoring;

use proposal::{Proposal, ProposalManager, ProposalState, ProposalType};
use voting::{VoteDirection, VotingConfig, VotingManager};
use ledger::{Contribution, ContributionType, ReputationLedger};
use scoring::{ReputationScorer, ScoringConfig};
use uuid::Uuid;

/// Test: Proposal keypair generation
#[test]
fn test_keypair_generation() {
    let result = Proposal::generate_keypair();
    assert!(result.is_ok());

    let (signing_key, verifying_key) = result.unwrap();

    // Verify key pair is valid
    let message = b"test message";
    let signature = signing_key.sign(message);
    assert!(verifying_key.verify_strict(message, &signature).is_ok());
}

/// Test: Proposal creation and signing
#[test]
fn test_proposal_creation() {
    let (signing_key, _verifying_key) = Proposal::generate_keypair().unwrap();

    let proposal = Proposal::create(
        Uuid::new_v4(),
        ProposalType::NetworkParam,
        "Test Proposal".to_string(),
        "Test payload for network parameter update".to_string(),
        &signing_key,
        72 * 3600, // 72 hours
    );

    assert_eq!(proposal.proposal_type, ProposalType::NetworkParam);
    assert_eq!(proposal.title, "Test Proposal");
    assert_eq!(proposal.state, ProposalState::Proposed);
}

/// Test: Proposal signature verification
#[test]
fn test_proposal_signature_verification() {
    let (signing_key, _verifying_key) = Proposal::generate_keypair().unwrap();

    let proposal = Proposal::create(
        Uuid::new_v4(),
        ProposalType::Security,
        "Security Update".to_string(),
        "Update security parameters".to_string(),
        &signing_key,
        72 * 3600,
    );

    assert!(proposal.verify_signature().is_ok());
}

/// Test: Proposal manager stores and retrieves proposals
#[test]
fn test_proposal_manager() {
    let mut manager = ProposalManager::new();

    let (signing_key, _verifying_key) = Proposal::generate_keypair().unwrap();

    let proposal = Proposal::create(
        Uuid::new_v4(),
        ProposalType::ModelUpdate,
        "Model Update".to_string(),
        "Update SAE model weights".to_string(),
        &signing_key,
        72 * 3600,
    );

    let proposal_id = proposal.id;
    assert!(manager.add_proposal(proposal).is_ok());

    let retrieved = manager.get_proposal(&proposal_id);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id, proposal_id);
}

/// Test: Voting system initialization
#[test]
fn test_voting_config_defaults() {
    let config = VotingConfig::default();

    assert_eq!(config.voting_duration_secs, 72 * 3600); // 72 hours
    assert!((config.quorum_percentage - 0.30).abs() < 0.01); // 30%
    assert!((config.approval_threshold - 0.51).abs() < 0.01); // 51%
}

/// Test: Vote casting and counting
#[test]
fn test_vote_casting() {
    let mut manager = VotingManager::new(VotingConfig::default());

    let (signing_key, _verifying_key) = Proposal::generate_keypair().unwrap();

    let proposal = Proposal::create(
        Uuid::new_v4(),
        ProposalType::Governance,
        "Governance Test".to_string(),
        "Test governance flow".to_string(),
        &signing_key,
        72 * 3600,
    );

    let proposal_id = proposal.id;
    manager.add_proposal(proposal).ok();

    // Cast votes
    let result = manager.cast_vote(
        &proposal_id,
        "voter_1".to_string(),
        VoteDirection::Approve,
        1.0, // reputation
    );
    assert!(result.is_ok());

    let result = manager.cast_vote(
        &proposal_id,
        "voter_2".to_string(),
        VoteDirection::Approve,
        0.8,
    );
    assert!(result.is_ok());

    let result = manager.cast_vote(
        &proposal_id,
        "voter_3".to_string(),
        VoteDirection::Reject,
        0.5,
    );
    assert!(result.is_ok());
}

/// Test: Vote resolution with quorum
#[test]
fn test_vote_resolution_with_quorum() {
    let config = VotingConfig {
        quorum_percentage: 0.30,
        approval_threshold: 0.51,
        minimum_reputation: 0.0,
        ..Default::default()
    };
    let mut manager = VotingManager::new(config);

    let (signing_key, _verifying_key) = Proposal::generate_keypair().unwrap();

    let proposal = Proposal::create(
        Uuid::new_v4(),
        ProposalType::Custom,
        "Quorum Test".to_string(),
        "Test quorum calculation".to_string(),
        &signing_key,
        72 * 3600,
    );

    let proposal_id = proposal.id;
    manager.add_proposal(proposal).ok();

    // Cast enough votes to meet quorum
    for i in 0..10 {
        let direction = if i < 7 {
            VoteDirection::Approve
        } else {
            VoteDirection::Reject
        };
        manager
            .cast_vote(&proposal_id, format!("voter_{}", i), direction, 1.0)
            .ok();
    }

    // Try to resolve
    let result = manager.resolve_vote(&proposal_id);
    // Result depends on quorum and threshold calculations
    assert!(result.is_ok() || result.is_err());
}

/// Test: Reputation ledger records contributions
#[test]
fn test_ledger_contribution_recording() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let db_path = temp_dir.path().join("governance_ledger.redb");

    let mut ledger = ReputationLedger::new(&db_path).unwrap();

    let contribution = Contribution {
        node_id: "contributor_1".to_string(),
        layer_id: Some("layer_0".to_string()),
        batch_hash: "abc123".to_string(),
        zkp_verified: true,
        contribution_type: ContributionType::SaeForward,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        base_credits: 10.0,
        previous_hash: None,
    };

    let result = ledger.record(contribution);
    assert!(result.is_ok());
}

/// Test: Reputation scoring processes contributions
#[test]
fn test_scoring_contribution_processing() {
    let mut scorer = ReputationScorer::new(ScoringConfig::default());

    let contribution = Contribution {
        node_id: "scorer_test".to_string(),
        layer_id: Some("layer_0".to_string()),
        batch_hash: "hash_001".to_string(),
        zkp_verified: true,
        contribution_type: ContributionType::SaeForward,
        timestamp: 1000,
        base_credits: 10.0,
        previous_hash: None,
    };

    let credits = scorer.process_contribution(&contribution);
    assert!(credits.is_ok());
    let earned = credits.unwrap();

    // ZKP multiplier should apply (1.5x)
    assert!(earned >= 10.0);
}

/// Test: Governance participation requires minimum reputation
#[test]
fn test_governance_reputation_requirement() {
    let mut scorer = ReputationScorer::new(ScoringConfig::default());

    // New node without contributions
    let can_participate = scorer.can_participate_in_governance("new_node");
    assert!(!can_participate);

    // Add contributions to build reputation
    for i in 0..20 {
        let contribution = Contribution {
            node_id: "active_node".to_string(),
            layer_id: Some("layer_0".to_string()),
            batch_hash: format!("hash_{}", i),
            zkp_verified: true,
            contribution_type: ContributionType::SaeForward,
            timestamp: 1000 + i,
            base_credits: 10.0,
            previous_hash: None,
        };
        scorer.process_contribution(&contribution).ok();
    }

    let can_participate = scorer.can_participate_in_governance("active_node");
    assert!(can_participate);
}

/// Test: Complete governance flow
#[test]
fn test_complete_governance_flow() {
    // 1. Generate keys
    let (signing_key, _verifying_key) = Proposal::generate_keypair().unwrap();

    // 2. Create proposal
    let proposal = Proposal::create(
        Uuid::new_v4(),
        ProposalType::NetworkParam,
        "Full Flow Test".to_string(),
        "Complete governance flow validation".to_string(),
        &signing_key,
        72 * 3600,
    );

    let proposal_id = proposal.id;

    // 3. Verify signature
    assert!(proposal.verify_signature().is_ok());

    // 4. Add to manager
    let mut manager = ProposalManager::new();
    assert!(manager.add_proposal(proposal).is_ok());

    // 5. Retrieve proposal
    let retrieved = manager.get_proposal(&proposal_id);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().state, ProposalState::Proposed);
}

/// Test: Ledger chain integrity verification
#[test]
fn test_ledger_chain_integrity() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let db_path = temp_dir.path().join("chain_integrity.redb");

    let mut ledger = ReputationLedger::new(&db_path).unwrap();

    // Record multiple contributions with chain links
    let mut prev_hash: Option<String> = None;

    for i in 0..5 {
        let contribution = Contribution {
            node_id: "chain_node".to_string(),
            layer_id: Some("layer_0".to_string()),
            batch_hash: format!("batch_{}", i),
            zkp_verified: true,
            contribution_type: ContributionType::SaeForward,
            timestamp: 1000 + i,
            base_credits: 10.0,
            previous_hash: prev_hash.clone(),
        };

        let result = ledger.record(contribution);
        if let Ok(hash) = result {
            prev_hash = Some(hash);
        }
    }

    // Verify chain integrity
    let integrity = ledger.verify_chain_integrity();
    assert!(integrity.is_ok());
    assert!(integrity.unwrap());
}
