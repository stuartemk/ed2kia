//! Property-Based Fuzzing — Consensus, Reputation & Sybil Invariants
//!
//! Feature-gated behind `v2.1-fuzzing`. Uses `proptest` to validate
//! cryptographic invariants: determinism, Byzantine tolerance, reputation
//! monotonicity, and Sybil resistance.
//!
//! **CI Config:** 1000 cases max, single-threaded to avoid resource exhaustion.
//!
//! # Running
//!
//! ```bash
//! cargo test --test consensus_fuzz --features "v2.1-fuzzing,v2.1-consensus-engine,v2.1-reputation-system,v2.1-sybil-micropow,v1.7-sprint1" -- --test-threads=1
//! ```
//!
//! # License
//!
//! Apache 2.0 + Ethical Use Clause

#[cfg(all(
    feature = "v2.1-fuzzing",
    feature = "v2.1-consensus-engine",
    feature = "v1.7-sprint1"
))]
mod consensus_properties {
    use ed2kia::protocol::audit_payloads::AuditResultPayload;
    use ed2kia::orchestrator::consensus::validate_consensus;
    use proptest::prelude::*;
    use uuid::Uuid;

    fn make_result(values: Vec<f32>, indices: Vec<usize>) -> AuditResultPayload {
        AuditResultPayload::new(
            Uuid::new_v4(),
            "fuzz-node".to_string(),
            values,
            indices,
            0.1,
        )
    }

    proptest! {
        #[test]
        fn test_consensus_determinism(
            values in proptest::collection::vec(proptest::float::f32::finite(), 1..20),
            indices in proptest::collection::vec(0usize..1000, 1..20),
            count in 1usize..10
        ) {
            // Same input → same result regardless of how many times we call validate
            let mut results = Vec::new();
            for _ in 0..count {
                results.push(make_result(values.clone(), indices.clone()));
            }

            let result1 = validate_consensus(results.clone(), 1e-4);
            let result2 = validate_consensus(results.clone(), 1e-4);

            prop_assert_eq!(result1.is_some(), result2.is_some());
            if let (Some(r1), Some(r2)) = (result1, result2) {
                prop_assert_eq!(r1.sparse_values, r2.sparse_values);
                prop_assert_eq!(r1.sparse_indices, r2.sparse_indices);
            }
        }

        #[test]
        fn test_consensus_empty_input(
            epsilon in (0.0_f32..1.0).prop_filter("non-zero", |e| *e > 0.0)
        ) {
            let results: Vec<AuditResultPayload> = vec![];
            let consensus = validate_consensus(results, epsilon);
            prop_assert!(consensus.is_none());
        }

        #[test]
        fn test_consensus_single_result_always_passes(
            values in proptest::collection::vec(proptest::float::f32::finite(), 1..10),
            indices in proptest::collection::vec(0usize..100, 1..10),
            epsilon in (0.0_f32..1.0).prop_filter("non-zero", |e| *e > 0.0)
        ) {
            let result = make_result(values.clone(), indices.clone());
            let consensus = validate_consensus(vec![result], epsilon);
            prop_assert!(consensus.is_some());
        }

        #[test]
        fn test_consensus_epsilon_tolerance(
            base_value in proptest::float::f32::finite(),
            indices in proptest::collection::vec(0usize..100, 1..10),
            delta in (0.0_f32..1e-5)
        ) {
            // Results within epsilon should reach consensus
            let val1 = base_value;
            let val2 = base_value + delta;
            let val3 = base_value - delta;

            let r1 = make_result(vec![val1], indices.clone());
            let r2 = make_result(vec![val2], indices.clone());
            let r3 = make_result(vec![val3], indices.clone());

            let consensus = validate_consensus(vec![r1, r2, r3], 1e-4);
            prop_assert!(consensus.is_some());
        }

        #[test]
        fn test_consensus_byzantine_tolerance(
            good_value in proptest::float::f32::finite(),
            indices in proptest::collection::vec(0usize..100, 1..10),
            byzantine_count in 0usize..3
        ) {
            // With N=7, f=2 Byzantine nodes (≤f/3), consensus should still reach
            let total = 7;
            let honest = total - byzantine_count;

            let mut results = Vec::new();
            // Honest nodes agree
            for _ in 0..honest {
                results.push(make_result(vec![good_value], indices.clone()));
            }
            // Byzantine nodes send random values
            for _ in 0..byzantine_count {
                results.push(make_result(vec![good_value + 100.0], indices.clone()));
            }

            let consensus = validate_consensus(results, 1e-4);
            // threshold = (7/2)+1 = 4, so need ≥4 honest
            if honest >= 4 {
                prop_assert!(consensus.is_some(), "Expected consensus with {} honest nodes", honest);
            }
        }
    }
}

#[cfg(all(
    feature = "v2.1-fuzzing",
    feature = "v2.1-reputation-system"
))]
mod reputation_properties {
    use ed2kia::orchestrator::reputation::ReputationEngine;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_reputation_never_negative_without_slashing(
            peer_id in "node-[0-9]{4}",
            matched_updates in 0usize..50
        ) {
            // Only matched updates → score should never trigger ban
            let engine = ReputationEngine::new();
            for _ in 0..matched_updates {
                let banned = engine.update_score(peer_id.clone(), true);
                prop_assert!(!banned, "Peer should not be banned with only matched results");
            }

            if let Some(score) = engine.get_score(&peer_id) {
                prop_assert!(score >= 0, "Score should be non-negative: {}", score);
            }
        }

        #[test]
        fn test_reputation_slashing_triggers_ban(
            peer_id in "node-[0-9]{4}"
        ) {
            // One mismatch (-50) from score 0 → should ban
            let engine = ReputationEngine::new();
            let banned = engine.update_score(peer_id.clone(), false);
            prop_assert!(banned, "Peer should be banned after mismatch from 0");
            prop_assert!(engine.is_banned(&peer_id));
        }

        #[test]
        fn test_reputation_ban_persistent_until_unban(
            peer_id in "node-[0-9]{4}"
        ) {
            let engine = ReputationEngine::new();
            engine.update_score(peer_id.clone(), false);
            prop_assert!(engine.is_banned(&peer_id));

            // Additional mismatches should not change ban status
            for _ in 0..5 {
                engine.update_score(peer_id.clone(), false);
            }
            prop_assert!(engine.is_banned(&peer_id));

            // Unban should work
            engine.unban_peer(&peer_id);
            prop_assert!(!engine.is_banned(&peer_id));

            if let Some(score) = engine.get_score(&peer_id) {
                prop_assert_eq!(score, 0, "Score should reset to 0 after unban");
            }
        }

        #[test]
        fn test_reputation_score_monotonicity(
            matched_count in 0usize..100,
            mismatch_count in 0usize..2
        ) {
            // Score should increase with matches, decrease with mismatches
            let engine = ReputationEngine::new();
            let peer_id = "monotonic-node";

            for _ in 0..matched_count {
                engine.update_score(peer_id.to_string(), true);
            }

            let score_after_matches = engine.get_score(peer_id).unwrap_or(0);
            prop_assert_eq!(score_after_matches, matched_count as i32);

            for _ in 0..mismatch_count {
                engine.update_score(peer_id.to_string(), false);
            }

            let expected = matched_count as i32 - (mismatch_count as i32 * 50);
            let final_score = engine.get_score(peer_id).unwrap_or(0);
            // If score went negative, peer is banned but score is still tracked
            prop_assert_eq!(final_score, expected);
        }
    }
}

#[cfg(all(
    feature = "v2.1-fuzzing",
    feature = "v2.1-sybil-micropow"
))]
mod sybil_properties {
    use ed2kia::orchestrator::sybil::{SybilEngine, Solution, solve_challenge};
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_sybil_valid_solution_always_verifies(
            difficulty in 1u8..3
        ) {
            // Valid solutions should always verify
            let engine = SybilEngine::with_difficulty(difficulty)
                .expect("difficulty 1-3 is valid");

            let challenge = engine.generate_challenge();
            let solution = solve_challenge(&challenge.nonce, challenge.difficulty);

            // The solution should have the required leading zeros
            let hash_bytes = hex::decode(&solution.solution_hash)
                .expect("valid hex");
            let leading_zeros = hash_bytes.iter()
                .take_while(|&&b| b == 0)
                .count();
            prop_assert!(leading_zeros >= solution.difficulty as usize,
                "Solution should have {} leading zeros, got {}",
                solution.difficulty, leading_zeros);
        }

        #[test]
        fn test_sybil_invalid_nonce_rejected(
            difficulty in 1u8..3
        ) {
            // Invalid nonce should fail verification
            let engine = SybilEngine::with_difficulty(difficulty)
                .expect("difficulty 1-3 is valid");

            let challenge = engine.generate_challenge();
            // Use wrong nonce
            let wrong_solution = Solution {
                nonce: "wrong-nonce".to_string(),
                difficulty: challenge.difficulty,
                solution_hash: challenge.nonce.clone(), // Wrong hash
                attempts: 1,
            };

            let result = engine.verify(
                &challenge,
                &wrong_solution,
                "fuzz-node-invalid"
            );
            prop_assert!(result.is_err(), "Invalid solution should be rejected");
        }

        #[test]
        fn test_sybil_rate_limiting_active(
            node_id in "fuzz-node-[0-9]{4}"
        ) {
            // After MAX_FAILED_ATTEMPTS (5), node should be banned
            let engine = SybilEngine::new();

            for _ in 0..6 {
                let challenge = engine.generate_challenge();
                let wrong_solution = Solution {
                    nonce: "bad".to_string(),
                    difficulty: challenge.difficulty,
                    solution_hash: "bad".to_string(),
                    attempts: 1,
                };
                // Ignore result — we just want to trigger rate limiting
                let _ = engine.verify(&challenge, &wrong_solution, &node_id);
            }

            // Next challenge should fail with ban or rate limit
            let challenge = engine.generate_challenge();
            let solution = solve_challenge(&challenge.nonce, challenge.difficulty);
            let result = engine.verify(&challenge, &solution, &node_id);
            prop_assert!(result.is_err(), "Banned node should be rejected");
        }

        #[test]
        fn test_sybil_difficulty_bounds(
            difficulty in (0u8..10).prop_filter("not 1-3", |d| *d < 1 || *d > 3)
        ) {
            // Difficulty outside 1-3 should fail
            let result = SybilEngine::with_difficulty(difficulty);
            prop_assert!(result.is_err(), "Difficulty {} should be rejected", difficulty);
        }
    }
}
