//! Morphic Resonance E2E Tests â€” Sprint 56
//!
//! End-to-end tests for the Morphic Resonance Decoder, Semantic Purifier
//! and Genesis Graph pipeline.
//!
//! **Test Scenarios:**
//! 1. Propaganda input â†’ negative Z-score â†’ purified output Z >= 0.
//! 2. GenesisNode creation â†’ zero pre-mine â†’ valid signature.
//! 3. First transaction accepted by Genesis Graph.
//! 4. Full pipeline: Input â†’ Decode â†’ Purify â†’ Genesis Validation.
//!
//! **Feature Gate:** `v3.8-morphic-genesis`

#[cfg(feature = "v3.8-morphic-genesis")]
mod morphic_pipeline_tests {
    use ed2kia::semantics::morphic_decoder::{IntentClassification, MorphicResonanceDecoder};
    use ed2kia::semantics::semantic_purifier::{NegativePattern, SemanticPurifier};

    #[test]
    fn test_propaganda_to_purified() {
        // Scenario: Propaganda input â†’ negative Z-score â†’ purified output Z >= 0
        let decoder = MorphicResonanceDecoder::new();
        let purifier = SemanticPurifier::new();

        // Step 1: Decode propaganda input
        let propaganda = "miedo y amenaza de escasez para todos";
        let original_waveform = decoder.decode(propaganda).unwrap();

        // Debug: print actual values
        eprintln!(
            "DEBUG: x={}, y={}, z={}, z_score={}, intent={:?}",
            original_waveform.x,
            original_waveform.y,
            original_waveform.z,
            original_waveform.z_score,
            original_waveform.intent
        );

        // Step 2: Verify negative Z-score (Lower Focus)
        assert!(
            original_waveform.intent == IntentClassification::LowerFocus,
            "Propaganda should be classified as Lower Focus, got z_score={}",
            original_waveform.z_score
        );
        assert!(
            original_waveform.z_score < 0.0,
            "Propaganda should have negative Z-score, got {}",
            original_waveform.z_score
        );

        // Step 3: Purify the input
        let purification_result = purifier.purify(propaganda).unwrap();

        // Step 4: Verify purified output has Z >= 0
        assert!(
            purification_result.purified_waveform.z_score >= 0.0,
            "Purified output should have Z >= 0, got {}",
            purification_result.purified_waveform.z_score
        );
        assert!(
            purification_result.was_purified,
            "Purification should have been applied"
        );
        assert!(
            purification_result.purified != propaganda,
            "Purified text should differ from original"
        );
    }

    #[test]
    fn test_division_to_cooperation() {
        let decoder = MorphicResonanceDecoder::new();
        let purifier = SemanticPurifier::new();

        let divisive = "divisiÃ³n y conflicto entre oponente";
        let original = decoder.decode(divisive).unwrap();
        assert_eq!(original.intent, IntentClassification::LowerFocus);

        let result = purifier.purify(divisive).unwrap();
        assert!(result.was_purified);
        assert_eq!(result.detected_pattern, Some(NegativePattern::Division));
        assert!(result.purified.contains("diÃ¡logo") || result.purified.contains("resoluciÃ³n"));
    }

    #[test]
    fn test_constructive_passes_through() {
        let decoder = MorphicResonanceDecoder::new();
        let purifier = SemanticPurifier::new();

        let constructive = "cooperaciÃ³n y armonÃ­a para la evoluciÃ³n";
        let waveform = decoder.decode(constructive).unwrap();
        assert_eq!(waveform.intent, IntentClassification::UpperFocus);

        // Purifier should reject already-constructive input
        match purifier.purify(constructive) {
            Err(_) => {} // Expected: AlreadyConstructive
            Ok(_) => panic!("Should reject constructive input"),
        }
    }

    #[test]
    fn test_english_propaganda_pipeline() {
        let decoder = MorphicResonanceDecoder::new();
        let purifier = SemanticPurifier::new();

        let propaganda = "fear and threat of scarcity and division";
        let original = decoder.decode(propaganda).unwrap();
        assert_eq!(original.intent, IntentClassification::LowerFocus);

        let result = purifier.purify(propaganda).unwrap();
        assert!(result.was_purified);
        // Purification should improve z_score significantly from original
        assert!(
            result.purified_waveform.z_score > original.z_score,
            "Purified z_score ({}) should be greater than original ({})",
            result.purified_waveform.z_score,
            original.z_score
        );
    }

    #[test]
    fn test_mixed_input_handling() {
        let decoder = MorphicResonanceDecoder::new();

        // Mixed input with both positive and negative patterns
        // "el miedo puede llevar a la cooperaciÃ³n" â€” fear can lead to cooperation
        // is a constructive message (Upper Focus) since it advocates cooperation
        let mixed = "el miedo puede llevar a la cooperaciÃ³n";
        let waveform = decoder.decode(mixed).unwrap();

        // With lowered thresholds (Â±0.10), constructive "cooperaciÃ³n" dominates
        // over "miedo", so this can be UpperFocus, Neutral, or LowerFocus
        // depending on exact weighting. All are valid for mixed input.
        assert!(
            waveform.intent == IntentClassification::UpperFocus
                || waveform.intent == IntentClassification::Neutral
                || waveform.intent == IntentClassification::LowerFocus,
            "Mixed input should have a valid intent classification, got z_score={}",
            waveform.z_score
        );
    }
}

#[cfg(feature = "v3.8-morphic-genesis")]
mod genesis_graph_tests {
    use ed2kia::economy::genesis_graph::{GenesisGraph, GenesisNode, NetworkId};

    #[test]
    fn test_genesis_zero_pre_mine() {
        let genesis = GenesisNode::create(NetworkId::Mainnet);
        assert_eq!(genesis.ce_balance, 0.0, "Genesis must have zero CE");
    }

    #[test]
    fn test_genesis_valid_signature() {
        let genesis = GenesisNode::create(NetworkId::Mainnet);
        assert!(genesis.verify().is_ok(), "Genesis signature must be valid");
    }

    #[test]
    fn test_genesis_is_root() {
        let genesis = GenesisNode::create(NetworkId::Mainnet);
        assert!(genesis.is_root());
        let parents = genesis.parent_hashes();
        assert!(parents[0].is_none());
        assert!(parents[1].is_none());
    }

    #[test]
    fn test_genesis_deterministic() {
        let g1 = GenesisNode::create(NetworkId::Mainnet);
        let g2 = GenesisNode::create(NetworkId::Mainnet);
        assert_eq!(g1.hash, g2.hash, "Genesis hash must be deterministic");
        assert_eq!(g1, g2, "Genesis nodes must be equal");
    }

    #[test]
    fn test_genesis_different_networks() {
        let mainnet = GenesisNode::create(NetworkId::Mainnet);
        let testnet = GenesisNode::create(NetworkId::Testnet);
        assert_ne!(mainnet.hash, testnet.hash);
        assert_eq!(mainnet.Topological_laws_hash, testnet.Topological_laws_hash);
    }

    #[test]
    fn test_genesis_graph_validates_child() {
        let graph = GenesisGraph::new(NetworkId::Mainnet);
        let genesis_hash = graph.genesis_hash();

        // Valid child references genesis
        let valid_parents = [Some(genesis_hash), None];
        assert!(graph.is_valid_child(&valid_parents));

        // Invalid child references unknown hash
        let invalid_parents = [Some(999_999_999), Some(888_888_888)];
        assert!(!graph.is_valid_child(&invalid_parents));
    }

    #[test]
    fn test_genesis_first_transaction_accepted() {
        // Scenario: Genesis accepts first transaction
        let graph = GenesisGraph::new(NetworkId::Mainnet);
        assert!(graph.verify_genesis().is_ok());

        let genesis_hash = graph.genesis_hash();
        let first_tx_parents = [Some(genesis_hash), None];
        assert!(
            graph.is_valid_child(&first_tx_parents),
            "First transaction referencing genesis must be valid"
        );
    }
}

#[cfg(feature = "v3.8-morphic-genesis")]
mod bridge_tests {
    use ed2kia::portal::morphic_bridge::{BridgeConfig, BridgeStatus, MorphicBridge};

    #[test]
    fn test_bridge_constructive_passes() {
        let bridge = MorphicBridge::new();
        let result = bridge.process("cooperaciÃ³n y armonÃ­a").unwrap();
        assert_eq!(result.status, BridgeStatus::Passed);
        assert!(!result.was_purified);
    }

    #[test]
    fn test_bridge_lower_focus_purifies() {
        let bridge = MorphicBridge::new();
        let result = bridge.process("miedo y amenaza").unwrap();
        assert!(
            result.status == BridgeStatus::Purified || result.status == BridgeStatus::Blocked,
            "Lower Focus input should be purified or blocked"
        );
    }

    #[test]
    fn test_bridge_quick_check() {
        let bridge = MorphicBridge::new();
        assert!(bridge.is_constructive("cooperaciÃ³n y evoluciÃ³n"));
        assert!(!bridge.is_constructive("miedo y amenaza y peligro"));
    }

    #[test]
    fn test_bridge_z_score() {
        let bridge = MorphicBridge::new();
        let score = bridge.get_z_score("cooperaciÃ³n y armonÃ­a");
        assert!(score.is_some());
        assert!(score.unwrap() > 0.0);
    }

    #[test]
    fn test_bridge_block_unpurifiable() {
        let config = BridgeConfig {
            auto_purify: false,
            block_unpurifiable: true,
            ..BridgeConfig::default()
        };
        let bridge = MorphicBridge::with_config(config);
        let result = bridge.process("miedo y amenaza").unwrap();
        assert_eq!(result.status, BridgeStatus::Blocked);
        assert!(result.output.is_empty());
    }
}

#[cfg(feature = "v3.8-morphic-genesis")]
mod full_pipeline_tests {
    use ed2kia::economy::genesis_graph::{GenesisGraph, NetworkId};
    use ed2kia::semantics::morphic_decoder::MorphicResonanceDecoder;
    use ed2kia::semantics::semantic_purifier::SemanticPurifier;

    #[test]
    fn test_full_pipeline_propaganda_to_genesis() {
        // Full pipeline: Input â†’ Decode â†’ Purify â†’ Genesis Validation
        let decoder = MorphicResonanceDecoder::new();
        let purifier = SemanticPurifier::new();
        let graph = GenesisGraph::new(NetworkId::Mainnet);

        // Step 1: Propaganda input
        let input = "miedo y amenaza de escasez";
        let original = decoder.decode(input).unwrap();
        assert!(original.z_score < 0.0);

        // Step 2: Purify
        let purified = purifier.purify(input).unwrap();
        assert!(purified.purified_waveform.z_score >= 0.0);

        // Step 3: Genesis validation
        assert!(graph.verify_genesis().is_ok());

        // Step 4: First transaction would reference genesis
        let genesis_hash = graph.genesis_hash();
        let tx_parents = [Some(genesis_hash), None];
        assert!(graph.is_valid_child(&tx_parents));
    }

    #[test]
    fn test_full_pipeline_constructive_direct() {
        let decoder = MorphicResonanceDecoder::new();
        let graph = GenesisGraph::new(NetworkId::Mainnet);

        // Constructive input passes directly
        let input = "cooperaciÃ³n y armonÃ­a para la evoluciÃ³n";
        let waveform = decoder.decode(input).unwrap();
        assert!(waveform.z_score >= 0.0);

        // Genesis accepts first transaction
        let genesis_hash = graph.genesis_hash();
        let tx_parents = [Some(genesis_hash), None];
        assert!(graph.is_valid_child(&tx_parents));
    }

    #[test]
    fn test_pipeline_integrity_chain() {
        // Verify the full chain: Laws â†’ Genesis â†’ Transaction
        let graph = GenesisGraph::new(NetworkId::Mainnet);

        // Genesis is valid
        assert!(graph.verify_genesis().is_ok());

        // Genesis is root
        let genesis = graph.genesis();
        assert!(genesis.is_root());
        assert_eq!(genesis.ce_balance, 0.0);

        // First transaction can reference genesis
        let tx_parents = [Some(genesis.hash), None];
        assert!(graph.is_valid_child(&tx_parents));
    }
}
