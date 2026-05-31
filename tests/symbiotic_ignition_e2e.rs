//! Symbiotic Ignition E2E Tests — Sprint 47
//!
//! Full end-to-end test cycle demonstrating the complete Omni-Node integration:
//! 1. Migration Handshake (Steganographic Pillar)
//! 2. Hypothesis Generation (Maieutic Pillar)
//! 3. Consensus Distribution (Corpuscular Pillar)
//! 4. CE Exchange & Homeostasis (Resonance Pillar)
//! 5. SCT Guard Supervision across all inter-pillar communication
//!
//! **Symbiotic Ignition Sequence:**
//! ```text
//! Migration → Hypothesis → Consensus → Exchange → Homeostasis
//! ```

#[cfg(all(feature = "v3.0-omni-integration",))]
mod omni_node_tests {
    use ed2kia::alignment::sct_core::StuartianTensor;
    use ed2kia::orchestration::{
        ExistentialCreditLedger, OmniNode, PillarId, PillarMessage, PillarRegistry, PillarStatus,
        RoutingError, SymbiosisValidator,
    };

    fn make_message(pillar_id: PillarId, ce_weight: f64) -> PillarMessage {
        PillarMessage::new(
            b"test payload".to_vec(),
            b"valid_signature".to_vec(),
            pillar_id,
            1000,
            1,
            ce_weight,
        )
    }

    fn make_valid_tensor(z: f32) -> StuartianTensor {
        StuartianTensor { x: 0.7, y: 0.3, z }
    }

    #[test]
    fn test_omni_node_initialization() {
        let mut node = OmniNode::new();
        node.initialize_pillars(100.0);

        assert_eq!(node.registered_pillars().len(), 4);
        assert_eq!(node.ce_ledger().balance(PillarId::CorpuscularBridge), 100.0);
        assert_eq!(
            node.ce_ledger().balance(PillarId::MaieuticSynthesizer),
            100.0
        );
        assert_eq!(
            node.ce_ledger().balance(PillarId::SteganographicSurvival),
            100.0
        );
        assert_eq!(
            node.ce_ledger().balance(PillarId::ResonanceInterface),
            100.0
        );
    }

    #[test]
    fn test_symbiotic_ignition_full_cycle() {
        let mut node = OmniNode::new();
        node.initialize_pillars(200.0);

        // Phase 1: Migration → Hypothesis (Steganographic → Maieutic)
        let msg1 = make_message(PillarId::MaieuticSynthesizer, 10.0);
        let tensor1 = make_valid_tensor(0.8);
        let result1 = node.route_message(
            PillarId::SteganographicSurvival,
            PillarId::MaieuticSynthesizer,
            &msg1,
            &tensor1,
        );
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap().status, PillarStatus::Success);

        // Phase 2: Hypothesis → Consensus (Maieutic → Corpuscular)
        let msg2 = make_message(PillarId::CorpuscularBridge, 15.0);
        let tensor2 = make_valid_tensor(0.6);
        let result2 = node.route_message(
            PillarId::MaieuticSynthesizer,
            PillarId::CorpuscularBridge,
            &msg2,
            &tensor2,
        );
        assert!(result2.is_ok());

        // Phase 3: Consensus → Homeostasis (Corpuscular → Resonance)
        let msg3 = make_message(PillarId::ResonanceInterface, 20.0);
        let tensor3 = make_valid_tensor(0.9);
        let result3 = node.route_message(
            PillarId::CorpuscularBridge,
            PillarId::ResonanceInterface,
            &msg3,
            &tensor3,
        );
        assert!(result3.is_ok());

        // Verify CE consumption
        assert!(node.ce_ledger().balance(PillarId::SteganographicSurvival) < 200.0);
        assert!(node.ce_ledger().balance(PillarId::MaieuticSynthesizer) < 200.0);
        assert!(node.ce_ledger().balance(PillarId::CorpuscularBridge) < 200.0);

        // Verify no rejections
        assert_eq!(node.rejection_count(), 0);
    }

    #[test]
    fn test_sct_guard_blocks_unethical_migration() {
        let mut node = OmniNode::new();
        node.initialize_pillars(100.0);

        let msg = make_message(PillarId::MaieuticSynthesizer, 10.0);
        let tensor = make_valid_tensor(-0.5); // Negative Z

        let result = node.route_message(
            PillarId::SteganographicSurvival,
            PillarId::MaieuticSynthesizer,
            &msg,
            &tensor,
        );
        assert!(result.is_err());
        assert_eq!(node.rejection_count(), 1);

        // Verify rejection record
        let rejections = node.get_rejections();
        assert_eq!(rejections[0].from, PillarId::SteganographicSurvival);
        assert_eq!(rejections[0].to, PillarId::MaieuticSynthesizer);
        assert_eq!(rejections[0].z_score, -0.5);
    }

    #[test]
    fn test_ce_exhaustion_prevents_routing() {
        let mut node = OmniNode::new();
        node.initialize_pillars(10.0);

        // First message succeeds
        let msg1 = make_message(PillarId::MaieuticSynthesizer, 5.0);
        let tensor = make_valid_tensor(0.5);
        let result1 = node.route_message(
            PillarId::CorpuscularBridge,
            PillarId::MaieuticSynthesizer,
            &msg1,
            &tensor,
        );
        assert!(result1.is_ok());

        // Second message fails (insufficient CE)
        let msg2 = make_message(PillarId::MaieuticSynthesizer, 10.0);
        let result2 = node.route_message(
            PillarId::CorpuscularBridge,
            PillarId::MaieuticSynthesizer,
            &msg2,
            &tensor,
        );
        assert!(matches!(result2, Err(RoutingError::InsufficientCE)));
    }

    #[test]
    fn test_diagnostics_after_full_cycle() {
        let mut node = OmniNode::new();
        node.initialize_pillars(100.0);

        // Run a few routes
        let msg = make_message(PillarId::MaieuticSynthesizer, 10.0);
        let tensor = make_valid_tensor(0.5);
        let _ = node.route_message(
            PillarId::CorpuscularBridge,
            PillarId::MaieuticSynthesizer,
            &msg,
            &tensor,
        );

        let diagnostics = node.diagnose();
        assert_eq!(diagnostics.len(), 4);
        for (_, diag) in &diagnostics {
            assert!(diag.contains("CE:"));
        }
    }

    #[test]
    fn test_cross_pillar_ce_ledger_consistency() {
        let mut node = OmniNode::new();
        node.initialize_pillars(100.0);

        let initial_total = node.ce_ledger().total_emitted();
        assert_eq!(initial_total, 400.0); // 4 pillars * 100 CE

        // Route message
        let msg = make_message(PillarId::MaieuticSynthesizer, 25.0);
        let tensor = make_valid_tensor(0.5);
        let _ = node.route_message(
            PillarId::CorpuscularBridge,
            PillarId::MaieuticSynthesizer,
            &msg,
            &tensor,
        );

        // Verify CE consumed
        assert_eq!(node.ce_ledger().total_consumed(), 25.0);
        assert_eq!(node.ce_ledger().balance(PillarId::CorpuscularBridge), 75.0);
    }
}

#[cfg(all(feature = "v3.0-omni-integration",))]
mod migration_tests {
    use ed2kia::alignment::sct_core::StuartianTensor;
    use ed2kia::pillars::steganographic::transport_rotator::TransportType;
    use ed2kia::pillars::steganographic::{
        MigrationError, MigrationHandshake, MigrationNegotiator, MigrationStatus, MigrationToken,
    };

    fn make_handshake(cluster_id: &str, capacity: u64, ce_budget: f64) -> MigrationHandshake {
        MigrationHandshake::new(
            cluster_id.to_string(),
            capacity,
            vec![TransportType::Tcp, TransportType::Quic],
            b"valid_signature".to_vec(),
            ce_budget,
        )
        .unwrap()
    }

    fn make_valid_tensor(z: f32) -> StuartianTensor {
        StuartianTensor { x: 0.7, y: 0.3, z }
    }

    #[test]
    fn test_migration_handshake_success() {
        let mut negotiator = MigrationNegotiator::new();
        let handshake = make_handshake("datacenter-alpha", 5000, 40.0);
        let tensor = make_valid_tensor(0.8);

        let result = negotiator.negotiate_migration(&handshake, &tensor);
        assert!(result.is_ok());

        let token = result.unwrap();
        assert_eq!(token.cluster_id, "datacenter-alpha");
        assert!(token.is_valid());
        assert_eq!(negotiator.cluster_count(), 1);
    }

    #[test]
    fn test_migration_sct_rejection() {
        let mut negotiator = MigrationNegotiator::new();
        let handshake = make_handshake("datacenter-beta", 5000, 40.0);
        let tensor = make_valid_tensor(-0.3); // Negative Z

        let result = negotiator.negotiate_migration(&handshake, &tensor);
        assert!(matches!(
            result,
            Err(MigrationError::EthicalRejection { .. })
        ));
        assert_eq!(negotiator.cluster_count(), 0);
    }

    #[test]
    fn test_migration_duplicate_cluster() {
        let mut negotiator = MigrationNegotiator::new();
        let handshake = make_handshake("datacenter-gamma", 5000, 40.0);
        let tensor = make_valid_tensor(0.5);

        let _ = negotiator.negotiate_migration(&handshake, &tensor);
        let result = negotiator.negotiate_migration(&handshake, &tensor);
        assert!(matches!(
            result,
            Err(MigrationError::ClusterAlreadyExists(_))
        ));
    }

    #[test]
    fn test_migration_capacity_exceeded() {
        let mut negotiator = MigrationNegotiator::with_config(1000, 0.0);
        let handshake = make_handshake("datacenter-delta", 5000, 40.0); // 5000 > 1000 max
        let tensor = make_valid_tensor(0.5);

        let result = negotiator.negotiate_migration(&handshake, &tensor);
        assert!(matches!(
            result,
            Err(MigrationError::CapacityExceeded { .. })
        ));
    }

    #[test]
    fn test_migration_token_bootstrap_routes() {
        let mut negotiator = MigrationNegotiator::new();
        let handshake = make_handshake("datacenter-epsilon", 5000, 40.0);
        let tensor = make_valid_tensor(0.5);

        let token = negotiator.negotiate_migration(&handshake, &tensor).unwrap();
        assert_eq!(token.bootstrap_routes.len(), 3);
        for route in &token.bootstrap_routes {
            assert!(route.contains("datacenter-epsilon"));
        }
    }

    #[test]
    fn test_migration_audit_log() {
        let mut negotiator = MigrationNegotiator::new();
        let handshake1 = make_handshake("dc-1", 1000, 25.0);
        let handshake2 = make_handshake("dc-2", 2000, 30.0);
        let tensor = make_valid_tensor(0.5);

        let _ = negotiator.negotiate_migration(&handshake1, &tensor);
        let _ = negotiator.negotiate_migration(&handshake2, &tensor);

        let log = negotiator.get_migration_log();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].cluster_id, "dc-1");
        assert_eq!(log[1].cluster_id, "dc-2");
        assert_eq!(log[0].status, MigrationStatus::Success);
    }

    #[test]
    fn test_multi_cluster_migration_sequence() {
        let mut negotiator = MigrationNegotiator::new();
        let tensor = make_valid_tensor(0.7);

        // Simulate a "Gran Migración" with multiple clusters
        let clusters = vec![
            ("cluster-alpha", 3000, 35.0),
            ("cluster-beta", 4000, 40.0),
            ("cluster-gamma", 2000, 30.0),
        ];

        for (id, capacity, ce) in &clusters {
            let handshake = make_handshake(id, *capacity, *ce);
            let result = negotiator.negotiate_migration(&handshake, &tensor);
            assert!(result.is_ok(), "Failed to migrate cluster: {}", id);
        }

        assert_eq!(negotiator.cluster_count(), 3);
        assert_eq!(negotiator.get_migration_log().len(), 3);
    }
}

#[cfg(all(feature = "v3.0-omni-integration",))]
mod integration_tests {
    use ed2kia::alignment::sct_core::StuartianTensor;
    use ed2kia::orchestration::{OmniNode, PillarId, PillarMessage, PillarStatus};
    use ed2kia::pillars::steganographic::transport_rotator::TransportType;
    use ed2kia::pillars::steganographic::{MigrationHandshake, MigrationNegotiator};

    fn make_message(pillar_id: PillarId, ce_weight: f64) -> PillarMessage {
        PillarMessage::new(
            b"integration test".to_vec(),
            b"valid_sig".to_vec(),
            pillar_id,
            1000,
            1,
            ce_weight,
        )
    }

    fn make_valid_tensor(z: f32) -> StuartianTensor {
        StuartianTensor { x: 0.7, y: 0.3, z }
    }

    #[test]
    fn test_full_symbiotic_ignition_with_migration() {
        // Step 1: Migrate a new cluster
        let mut negotiator = MigrationNegotiator::new();
        let handshake = MigrationHandshake::new(
            "new-cluster".to_string(),
            5000,
            vec![TransportType::Tcp, TransportType::Quic],
            b"cluster_sig".to_vec(),
            40.0,
        )
        .unwrap();
        let migration_tensor = StuartianTensor {
            x: 0.8,
            y: 0.2,
            z: 0.9,
        };

        let token = negotiator
            .negotiate_migration(&handshake, &migration_tensor)
            .unwrap();
        assert_eq!(token.cluster_id, "new-cluster");

        // Step 2: Initialize Omni-Node with all pillars
        let mut omni = OmniNode::new();
        omni.initialize_pillars(150.0);

        // Step 3: Run inter-pillar communication cycle
        let msg = make_message(PillarId::MaieuticSynthesizer, 10.0);
        let tensor = make_valid_tensor(0.7);

        let result = omni.route_message(
            PillarId::SteganographicSurvival,
            PillarId::MaieuticSynthesizer,
            &msg,
            &tensor,
        );
        assert!(result.is_ok());

        // Step 4: Verify system state
        assert_eq!(omni.rejection_count(), 0);
        assert!(omni.ce_ledger().balance(PillarId::SteganographicSurvival) < 150.0);
        assert_eq!(negotiator.cluster_count(), 1);
    }

    #[test]
    fn test_omni_node_with_custom_ce_distribution() {
        let mut omni = OmniNode::new();

        // Register pillars with different CE amounts
        omni.register_pillar(PillarId::CorpuscularBridge, 50.0);
        omni.register_pillar(PillarId::MaieuticSynthesizer, 100.0);
        omni.register_pillar(PillarId::SteganographicSurvival, 75.0);
        omni.register_pillar(PillarId::ResonanceInterface, 120.0);

        assert_eq!(omni.ce_ledger().balance(PillarId::CorpuscularBridge), 50.0);
        assert_eq!(
            omni.ce_ledger().balance(PillarId::MaieuticSynthesizer),
            100.0
        );
        assert_eq!(
            omni.ce_ledger().balance(PillarId::SteganographicSurvival),
            75.0
        );
        assert_eq!(
            omni.ce_ledger().balance(PillarId::ResonanceInterface),
            120.0
        );

        // Total CE emitted
        assert_eq!(omni.ce_ledger().total_emitted(), 345.0);
    }

    #[test]
    fn test_pillar_status_updates_on_routing() {
        let mut omni = OmniNode::new();
        omni.initialize_pillars(100.0);

        let msg = make_message(PillarId::MaieuticSynthesizer, 10.0);
        let tensor = make_valid_tensor(0.5);

        let result = omni.route_message(
            PillarId::CorpuscularBridge,
            PillarId::MaieuticSynthesizer,
            &msg,
            &tensor,
        );

        assert!(result.is_ok());
        assert_eq!(
            omni.get_pillar_status(PillarId::CorpuscularBridge),
            Some(PillarStatus::Success)
        );
        assert_eq!(
            omni.get_pillar_status(PillarId::MaieuticSynthesizer),
            Some(PillarStatus::Success)
        );
    }

    #[test]
    fn test_sct_guard_supreme_across_all_pillars() {
        let mut omni = OmniNode::new();
        omni.initialize_pillars(100.0);

        // Test that all pillar pairs reject negative Z
        let pillars = [
            PillarId::CorpuscularBridge,
            PillarId::MaieuticSynthesizer,
            PillarId::SteganographicSurvival,
            PillarId::ResonanceInterface,
        ];

        let msg = make_message(PillarId::MaieuticSynthesizer, 5.0);
        let bad_tensor = make_valid_tensor(-0.5);

        for from in &pillars {
            for to in &pillars {
                if from != to {
                    let result = omni.route_message(*from, *to, &msg, &bad_tensor);
                    assert!(result.is_err(), "Expected rejection for {} -> {}", from, to);
                }
            }
        }

        // All rejections should be recorded
        assert!(omni.rejection_count() > 0);
    }
}
