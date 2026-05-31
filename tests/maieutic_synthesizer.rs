//! Maieutic Synthesizer Integration Tests — Sprint 44
//!
//! Tests the full Maieutic Synthesizer pipeline:
//! Hypothesis Generation → Evidence Collection → BFT Consensus → Validation.

#[cfg(all(feature = "v3.0-maieutic-synthesizer", feature = "v3.0-orchestration"))]
mod hypothesis_engine_tests {
    use ed2kia::pillars::maieutic::hypothesis_engine::{
        Domain, Evidence, HypothesisEngine, HypothesisState,
    };

    fn make_evidence(source: &str, domain: Domain, z: f32) -> Evidence {
        Evidence {
            source_node: source.to_string(),
            domain,
            payload: b"test".to_vec(),
            z_score: z,
            timestamp_ms: 1000,
        }
    }

    #[test]
    fn test_hypothesis_lifecycle() {
        let mut engine = HypothesisEngine::with_config(2, 0.0);

        // Generate hypothesis.
        let h = engine.generate_hypothesis(
            "h1".to_string(),
            Domain::ProteinFolding,
            "Protein X folds to beta-sheet".to_string(),
            0.5,
        );
        assert!(h.is_ok());
        assert_eq!(h.unwrap().state, HypothesisState::Proposed);

        // Submit evidence.
        engine
            .submit_evidence("h1", make_evidence("n1", Domain::ProteinFolding, 0.5))
            .unwrap();
        let h = engine.get_hypothesis("h1").unwrap();
        assert_eq!(h.state, HypothesisState::CollectingEvidence);

        engine
            .submit_evidence("h1", make_evidence("n2", Domain::ProteinFolding, 0.3))
            .unwrap();
        let h = engine.get_hypothesis("h1").unwrap();
        assert_eq!(h.state, HypothesisState::ReadyForConsensus);
    }

    #[test]
    fn test_sct_guard_rejects_negative_z() {
        let mut engine = HypothesisEngine::new();
        let result = engine.generate_hypothesis(
            "h1".to_string(),
            Domain::Epigenetics,
            "Destructive".to_string(),
            -0.5,
        );
        assert!(result.is_err());
        assert!(engine.is_empty());
    }

    #[test]
    fn test_cross_domain_hypotheses() {
        let mut engine = HypothesisEngine::new();
        engine
            .generate_hypothesis(
                "h1".to_string(),
                Domain::ProteinFolding,
                "A".to_string(),
                0.5,
            )
            .unwrap();
        engine
            .generate_hypothesis(
                "h2".to_string(),
                Domain::MolecularDynamics,
                "B".to_string(),
                0.5,
            )
            .unwrap();
        engine
            .generate_hypothesis("h3".to_string(), Domain::Epigenetics, "C".to_string(), 0.5)
            .unwrap();

        assert_eq!(engine.list_by_domain(&Domain::ProteinFolding).len(), 1);
        assert_eq!(engine.list_by_domain(&Domain::MolecularDynamics).len(), 1);
        assert_eq!(engine.list_by_domain(&Domain::Epigenetics).len(), 1);
        assert_eq!(engine.all_hypotheses().len(), 3);
    }

    #[test]
    fn test_consensus_result_update() {
        let mut engine = HypothesisEngine::new();
        engine
            .generate_hypothesis(
                "h1".to_string(),
                Domain::ClimateModeling,
                "Test".to_string(),
                0.5,
            )
            .unwrap();

        let h = engine.update_consensus_result("h1", true).unwrap();
        assert_eq!(h.state, HypothesisState::Validated);

        engine
            .generate_hypothesis(
                "h2".to_string(),
                Domain::ClimateModeling,
                "Test2".to_string(),
                0.5,
            )
            .unwrap();
        let h = engine.update_consensus_result("h2", false).unwrap();
        assert_eq!(h.state, HypothesisState::Rejected);
    }
}

#[cfg(all(feature = "v3.0-maieutic-synthesizer", feature = "v3.0-orchestration"))]
mod bio_sim_worker_tests {
    use ed2kia::pillars::maieutic::bio_sim_worker::{BioSimWorker, SimConfig};
    use ed2kia::pillars::maieutic::hypothesis_engine::Domain;

    fn test_input() -> Vec<u8> {
        vec![100, 150, 200, 50, 128, 255, 0, 64]
    }

    #[test]
    fn test_all_domains_simulate() {
        let domains = vec![
            Domain::MolecularDynamics,
            Domain::ProteinFolding,
            Domain::Epigenetics,
            Domain::ClimateModeling,
            Domain::MaterialsScience,
        ];

        for domain in domains {
            let mut worker = BioSimWorker::for_domain(domain.clone(), "w1".to_string()).unwrap();
            let result = worker.execute(&test_input()).unwrap();
            assert!(result.iterations > 0);
            assert!(result.z_score >= 0.0);
        }
    }

    #[test]
    fn test_simulation_to_evidence() {
        let mut worker =
            BioSimWorker::for_domain(Domain::ProteinFolding, "sim-node".to_string()).unwrap();
        let result = worker.execute(&test_input()).unwrap();
        let evidence = worker.to_evidence(&result);
        assert_eq!(evidence.source_node, "sim-node");
        assert_eq!(evidence.domain, Domain::ProteinFolding);
        assert!(evidence.z_score >= 0.0);
    }

    #[test]
    fn test_worker_config_validation() {
        let config = SimConfig::new(Domain::Epigenetics, "w".to_string()).with_max_iterations(0);
        assert!(BioSimWorker::new(config).is_err());

        let config = SimConfig::new(Domain::Epigenetics, "w".to_string()).with_precision(-1.0);
        assert!(BioSimWorker::new(config).is_err());
    }

    #[test]
    fn test_empty_input_rejected() {
        let mut worker =
            BioSimWorker::for_domain(Domain::MolecularDynamics, "w".to_string()).unwrap();
        assert!(worker.execute(&[]).is_err());
    }
}

#[cfg(all(feature = "v3.0-maieutic-synthesizer", feature = "v3.0-orchestration"))]
mod scientific_consensus_tests {
    use ed2kia::pillars::maieutic::hypothesis_engine::{Domain, Evidence};
    use ed2kia::pillars::maieutic::scientific_consensus::{ConsensusError, ScientificConsensus};

    fn make_evidence(source: &str, domain: Domain, z: f32) -> Evidence {
        Evidence {
            source_node: source.to_string(),
            domain,
            payload: b"e".to_vec(),
            z_score: z,
            timestamp_ms: 1000,
        }
    }

    #[test]
    fn test_bft_consensus_passes_at_threshold() {
        let mut consensus = ScientificConsensus::new();
        consensus.register_validator("v1".to_string());
        consensus.register_validator("v2".to_string());
        consensus.register_validator("v3".to_string());

        consensus
            .submit_evidence("h1", make_evidence("v1", Domain::Epigenetics, 0.5))
            .unwrap();
        consensus
            .submit_evidence("h1", make_evidence("v2", Domain::Epigenetics, 0.3))
            .unwrap();
        consensus
            .submit_evidence("h1", make_evidence("v3", Domain::Epigenetics, 0.4))
            .unwrap();

        let result = consensus.run_consensus("h1", &Domain::Epigenetics).unwrap();
        assert!(result.is_validated());
    }

    #[test]
    fn test_bft_consensus_fails_below_threshold() {
        let mut consensus = ScientificConsensus::new();
        consensus.register_validator("v1".to_string());
        consensus.register_validator("v2".to_string());
        consensus.register_validator("v3".to_string());

        consensus
            .submit_evidence("h1", make_evidence("v1", Domain::Epigenetics, 0.5))
            .unwrap();
        consensus
            .submit_evidence("h1", make_evidence("v2", Domain::ProteinFolding, 0.3))
            .unwrap();
        consensus
            .submit_evidence("h1", make_evidence("v3", Domain::ProteinFolding, 0.4))
            .unwrap();

        let result = consensus.run_consensus("h1", &Domain::Epigenetics).unwrap();
        assert!(!result.is_validated());
    }

    #[test]
    fn test_sct_guard_in_consensus() {
        let mut consensus = ScientificConsensus::new();
        consensus.register_validator("v1".to_string());
        let result =
            consensus.submit_evidence("h1", make_evidence("v1", Domain::Epigenetics, -0.5));
        match result {
            Err(ConsensusError::SctGuardRejected { z_score, .. }) => {
                assert!(z_score < 0.0);
            }
            _ => panic!("Expected SctGuardRejected"),
        }
    }

    #[test]
    fn test_duplicate_evidence_rejected() {
        let mut consensus = ScientificConsensus::new();
        consensus.register_validator("v1".to_string());
        consensus
            .submit_evidence("h1", make_evidence("v1", Domain::Epigenetics, 0.5))
            .unwrap();
        let result = consensus.submit_evidence("h1", make_evidence("v1", Domain::Epigenetics, 0.3));
        assert!(matches!(result, Err(ConsensusError::DuplicateEvidence(_))));
    }

    #[test]
    fn test_custom_threshold() {
        let mut consensus = ScientificConsensus::with_threshold(0.5, 0.0);
        consensus.register_validator("v1".to_string());
        consensus.register_validator("v2".to_string());

        consensus
            .submit_evidence("h1", make_evidence("v1", Domain::Epigenetics, 0.5))
            .unwrap();
        // 1 out of 2 = 50% — exactly at threshold.
        let result = consensus.run_consensus("h1", &Domain::Epigenetics).unwrap();
        assert!(result.is_validated());
    }
}

#[cfg(all(feature = "v3.0-maieutic-synthesizer", feature = "v3.0-orchestration"))]
mod maieutic_engine_integration_tests {
    use ed2kia::pillars::maieutic::hypothesis_engine::{Domain, Evidence};
    use ed2kia::pillars::maieutic::MaieuticEngine;
    use ed2kia::pillars::PillarInterface;

    fn make_evidence(source: &str, domain: Domain, z: f32) -> Evidence {
        Evidence {
            source_node: source.to_string(),
            domain,
            payload: b"e".to_vec(),
            z_score: z,
            timestamp_ms: 1000,
        }
    }

    #[test]
    fn test_pillar_interface() {
        let engine = MaieuticEngine::new();
        assert!(engine.validate_local_constraint());
        assert!(engine.consume_ce(1.0).is_ok());
        assert!(engine.consume_ce(0.0).is_err());
    }

    #[test]
    fn test_full_pipeline() {
        let mut engine = MaieuticEngine::new();

        // Generate hypothesis.
        engine
            .generate_hypothesis(
                "h1".to_string(),
                Domain::ProteinFolding,
                "Protein X stabilizes under condition Y".to_string(),
                0.5,
            )
            .unwrap();

        // Register validators.
        engine.register_validator("v1".to_string());
        engine.register_validator("v2".to_string());
        engine.register_validator("v3".to_string());

        // Submit evidence.
        engine
            .submit_evidence("h1", make_evidence("v1", Domain::ProteinFolding, 0.5))
            .unwrap();
        engine
            .submit_evidence("h1", make_evidence("v2", Domain::ProteinFolding, 0.3))
            .unwrap();
        engine
            .submit_evidence("h1", make_evidence("v3", Domain::ProteinFolding, 0.4))
            .unwrap();

        // Run consensus.
        let result = engine.run_consensus("h1", &Domain::ProteinFolding).unwrap();
        assert!(result.is_validated());

        // Verify hypothesis state.
        let h = engine.get_hypothesis("h1").unwrap();
        assert_eq!(h.id, "h1");
        assert!(h.evidence_count() >= 3);
    }

    #[test]
    fn test_sct_guard_full_pipeline() {
        let mut engine = MaieuticEngine::new();
        let result = engine.generate_hypothesis(
            "h1".to_string(),
            Domain::Epigenetics,
            "Destructive".to_string(),
            -0.5,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_ready_for_consensus_list() {
        let mut engine = MaieuticEngine::new();
        engine
            .generate_hypothesis(
                "h1".to_string(),
                Domain::MolecularDynamics,
                "A".to_string(),
                0.5,
            )
            .unwrap();
        engine.register_validator("v1".to_string());
        engine.register_validator("v2".to_string());
        engine.register_validator("v3".to_string());

        engine
            .submit_evidence("h1", make_evidence("v1", Domain::MolecularDynamics, 0.5))
            .unwrap();
        engine
            .submit_evidence("h1", make_evidence("v2", Domain::MolecularDynamics, 0.3))
            .unwrap();
        engine
            .submit_evidence("h1", make_evidence("v3", Domain::MolecularDynamics, 0.4))
            .unwrap();

        let ready = engine.ready_for_consensus();
        assert_eq!(ready.len(), 1);
    }
}
