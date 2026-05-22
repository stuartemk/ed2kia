//! Immune System Integration Tests — Sprint 29
//!
//! Validates the full lifecycle of Existential Credit, Proof of Symbiosis,
//! and Network Apoptosis across the ed2kIA network layer.

#[cfg(all(
    feature = "v2.1-proof-of-symbiosis",
    feature = "v2.1-network-apoptosis"
))]
mod existential_credit_tests {
    use ed2kia::economics::existential_credit::ExistentialCreditLedger;

    #[test]
    fn test_ce_emit_and_burn() {
        let mut ledger = ExistentialCreditLedger::new();

        // Emit credit for ethical compute (Z > 0).
        ledger
            .emit_credit("alice", 5.0, 2.0)
            .expect("emit should succeed");

        let score = ledger.get_score("alice");
        assert!(
            (score - 10.0).abs() < f64::EPSILON,
            "Expected 10.0 (5.0 * 2.0), got {}",
            score
        );

        // Burn credit for perversity (Z < 0).
        ledger
            .burn_credit("alice", -3.0, 1.5)
            .expect("burn should succeed");

        let score = ledger.get_score("alice");
        assert!(
            (score - 5.5).abs() < f64::EPSILON,
            "Expected 5.5 (10.0 - 4.5), got {}",
            score
        );
    }

    #[test]
    fn test_ce_merge_idempotent() {
        let mut ledger = ExistentialCreditLedger::new();
        ledger
            .emit_credit("peer1", 10.0, 1.0)
            .expect("emit should succeed");

        let before = ledger.clone();
        ledger.merge(&before);

        assert_eq!(
            ledger.get_score("peer1"),
            before.get_score("peer1"),
            "Merge with self should not change score"
        );
    }

    #[test]
    fn test_ce_merge_commutative() {
        let mut a = ExistentialCreditLedger::new();
        let mut b = ExistentialCreditLedger::new();

        a.emit_credit("alice", 10.0, 1.0).ok();
        b.emit_credit("bob", 20.0, 1.0).ok();

        let mut a_copy = a.clone();
        let mut b_copy = b.clone();

        a.merge(&b);
        b_copy.merge(&a_copy);

        assert_eq!(
            a.peer_count(),
            b_copy.peer_count(),
            "Both should have same peer count"
        );
        assert!(
            (a.get_score("alice") - b_copy.get_score("alice")).abs() < f64::EPSILON,
            "Alice score should match"
        );
        assert!(
            (a.get_score("bob") - b_copy.get_score("bob")).abs() < f64::EPSILON,
            "Bob score should match"
        );
    }

    #[test]
    fn test_ce_negative_score_accumulation() {
        let mut ledger = ExistentialCreditLedger::new();

        // Multiple burns push score deeply negative.
        for _ in 0..10 {
            ledger
                .burn_credit("malicious", -15.0, 1.0)
                .expect("burn should succeed");
        }

        let score = ledger.get_score("malicious");
        assert!(
            (score - (-150.0)).abs() < f64::EPSILON,
            "Expected -150.0, got {}",
            score
        );
        assert!(score < -100.0, "Score should trigger apoptosis");
    }
}

#[cfg(all(
    feature = "v2.1-proof-of-symbiosis",
    feature = "v2.1-network-apoptosis"
))]
mod proof_of_symbiosis_tests {
    use ed2kia::economics::existential_credit::ExistentialCreditLedger;
    use ed2kia::economics::proof_of_symbiosis::committee_threshold_met;

    #[test]
    fn test_threshold_met_with_strong_committee() {
        let mut ledger = ExistentialCreditLedger::new();
        ledger.emit_credit("alice", 50.0, 1.0).ok();
        ledger.emit_credit("bob", 30.0, 1.0).ok();
        ledger.emit_credit("charlie", 20.0, 1.0).ok();

        // Total CE = 100. alice=0.5, bob=0.3, charlie=0.2
        // Committee: alice + bob = 0.8 >= 0.5 threshold
        let result = committee_threshold_met(&["alice", "bob"], &ledger, 0.5, 0.0)
            .expect("validation should succeed");
        assert!(result, "Strong committee should meet threshold");
    }

    #[test]
    fn test_threshold_not_met_with_weak_committee() {
        let mut ledger = ExistentialCreditLedger::new();
        ledger.emit_credit("alice", 50.0, 1.0).ok();
        ledger.emit_credit("bob", 30.0, 1.0).ok();
        ledger.emit_credit("charlie", 20.0, 1.0).ok();

        // charlie = 0.2 < 0.5 threshold
        let result = committee_threshold_met(&["charlie"], &ledger, 0.5, 0.0)
            .expect("validation should succeed");
        assert!(!result, "Weak committee should not meet threshold");
    }

    #[test]
    fn test_threshold_with_network_load() {
        let mut ledger = ExistentialCreditLedger::new();
        ledger.emit_credit("alice", 50.0, 1.0).ok();
        ledger.emit_credit("bob", 50.0, 1.0).ok();

        // alice = 0.5, bob = 0.5, total = 1.0
        // threshold = 0.5 * (1.0 + 1.0) = 1.0
        // alice + bob = 1.0 >= 1.0 -> met
        let result = committee_threshold_met(&["alice", "bob"], &ledger, 0.5, 1.0)
            .expect("validation should succeed");
        assert!(result, "Full committee should meet high-load threshold");

        // alice alone = 0.5 < 1.0 -> not met
        let result = committee_threshold_met(&["alice"], &ledger, 0.5, 1.0)
            .expect("validation should succeed");
        assert!(!result, "Single peer should not meet high-load threshold");
    }

    #[test]
    fn test_anti_sybil_resistance() {
        // Honest node with 60 CE vs 3 Sybil nodes with 20 CE each.
        let mut ledger = ExistentialCreditLedger::new();
        ledger.emit_credit("honest", 60.0, 1.0).ok();
        ledger.emit_credit("sybil1", 20.0, 1.0).ok();
        ledger.emit_credit("sybil2", 20.0, 1.0).ok();
        ledger.emit_credit("sybil3", 20.0, 1.0).ok();

        // Total = 120. honest = 0.5, each sybil = 0.167
        // Honest alone meets 0.4 threshold.
        let result = committee_threshold_met(&["honest"], &ledger, 0.4, 0.0)
            .expect("validation should succeed");
        assert!(result, "Honest node should meet threshold alone");

        // All 3 Sybils combined = 0.5, also meets 0.4.
        // But they need 3x coordination, which is the anti-Sybil cost.
        let result =
            committee_threshold_met(&["sybil1", "sybil2", "sybil3"], &ledger, 0.4, 0.0)
                .expect("validation should succeed");
        assert!(
            result,
            "All Sybils combined should meet threshold (but with coordination cost)"
        );

        // Single Sybil = 0.167 < 0.4
        let result = committee_threshold_met(&["sybil1"], &ledger, 0.4, 0.0)
            .expect("validation should succeed");
        assert!(
            !result,
            "Single Sybil should not meet threshold alone"
        );
    }
}

#[cfg(all(
    feature = "v2.1-proof-of-symbiosis",
    feature = "v2.1-network-apoptosis"
))]
mod apoptosis_flow_tests {
    use ed2kia::economics::existential_credit::ExistentialCreditLedger;
    use ed2kia::federated::network_apoptosis::{
        ImmuneState, ImmuneConfig, NetworkImmuneSystem,
    };

    #[test]
    fn test_full_apoptosis_flow() {
        let mut ledger = ExistentialCreditLedger::new();
        let mut immune = NetworkImmuneSystem::new();

        // Phase 1: Peer starts healthy.
        ledger.emit_credit("node1", 50.0, 1.0).ok();
        assert_eq!(
            immune.evaluate_peer("node1", &ledger),
            ImmuneState::Healthy
        );

        // Phase 2: Peer enters Pain (score goes negative).
        ledger.burn_credit("node1", -60.0, 1.0).ok();
        assert_eq!(immune.evaluate_peer("node1", &ledger), ImmuneState::Pain);
        assert!(!immune.is_blocklisted("node1"));

        // Phase 3: Peer enters Apoptosis (score < -100).
        // Current score: 50.0 - 60.0 = -10.0. Burn 150.0 -> -160.0 < -100.0.
        ledger.burn_credit("node1", -150.0, 1.0).ok();
        assert_eq!(
            immune.evaluate_peer("node1", &ledger),
            ImmuneState::Apoptosis
        );

        // Phase 4: Auto-apoptosis triggers blocklist.
        let apoptosed = immune.evaluate_all(&ledger, 1000);
        assert_eq!(apoptosed.len(), 1);
        assert_eq!(apoptosed[0], "node1");
        assert!(immune.is_blocklisted("node1"));
    }

    #[test]
    fn test_pain_state_warnings() {
        let mut ledger = ExistentialCreditLedger::new();
        let immune = NetworkImmuneSystem::new();

        // Peer in pain state (negative but above apoptosis threshold).
        ledger.burn_credit("warning_node", -30.0, 1.0).ok();

        let state = immune.evaluate_peer("warning_node", &ledger);
        assert_eq!(state, ImmuneState::Pain);

        // Pain state should NOT trigger blocklist.
        assert!(!immune.is_blocklisted("warning_node"));
    }

    #[test]
    fn test_multiple_peers_mixed_states() {
        let mut ledger = ExistentialCreditLedger::new();
        let mut immune = NetworkImmuneSystem::new();

        // Healthy peer.
        ledger.emit_credit("healthy", 100.0, 1.0).ok();

        // Pain peer.
        ledger.burn_credit("pain", -50.0, 1.0).ok();

        // Apoptosis peer.
        ledger.burn_credit("dead", -200.0, 1.0).ok();

        assert_eq!(
            immune.evaluate_peer("healthy", &ledger),
            ImmuneState::Healthy
        );
        assert_eq!(immune.evaluate_peer("pain", &ledger), ImmuneState::Pain);
        assert_eq!(
            immune.evaluate_peer("dead", &ledger),
            ImmuneState::Apoptosis
        );

        // Evaluate all — only "dead" should be apoptosed.
        let apoptosed = immune.evaluate_all(&ledger, 1000);
        assert_eq!(apoptosed.len(), 1);
        assert_eq!(apoptosed[0], "dead");

        // Healthy and pain peers should NOT be blocklisted.
        assert!(!immune.is_blocklisted("healthy"));
        assert!(!immune.is_blocklisted("pain"));
        assert!(immune.is_blocklisted("dead"));
    }

    #[test]
    fn test_custom_apoptosis_threshold() {
        let mut ledger = ExistentialCreditLedger::new();
        let mut immune = NetworkImmuneSystem::with_config(ImmuneConfig {
            apoptosis_threshold: -50.0,
            ..Default::default()
        })
        .expect("config should be valid");

        // Score -60 is below custom threshold (-50) but above default (-100).
        ledger.burn_credit("node1", -60.0, 1.0).ok();

        assert_eq!(
            immune.evaluate_peer("node1", &ledger),
            ImmuneState::Apoptosis
        );

        let apoptosed = immune.evaluate_all(&ledger, 1000);
        assert_eq!(apoptosed.len(), 1);
    }

    #[test]
    fn test_blocklisted_peer_remains_blocklisted() {
        let mut ledger = ExistentialCreditLedger::new();
        let mut immune = NetworkImmuneSystem::new();

        // Peer goes to apoptosis.
        ledger.burn_credit("node1", -200.0, 1.0).ok();
        immune
            .trigger_apoptosis("node1", &ledger, 1000, "test")
            .ok();

        // Peer recovers score (e.g., through new ethical compute).
        ledger.emit_credit("node1", 500.0, 1.0).ok();

        // Still blocklisted — apoptosis is irreversible.
        assert!(immune.is_blocklisted("node1"));
        assert_eq!(
            immune.evaluate_peer("node1", &ledger),
            ImmuneState::Apoptosis
        );
    }

    #[test]
    fn test_apoptosis_log_records_events() {
        let mut ledger = ExistentialCreditLedger::new();
        let mut immune = NetworkImmuneSystem::new();

        ledger.burn_credit("node1", -200.0, 1.0).ok();
        ledger.burn_credit("node2", -300.0, 1.0).ok();

        immune
            .trigger_apoptosis("node1", &ledger, 1000, "perversity")
            .ok();
        immune
            .trigger_apoptosis("node2", &ledger, 2000, "repeated violations")
            .ok();

        let log = immune.get_apoptosis_log();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].peer_id, "node1");
        assert_eq!(log[0].timestamp_ms, 1000);
        assert_eq!(log[1].peer_id, "node2");
        assert_eq!(log[1].timestamp_ms, 2000);
    }
}
