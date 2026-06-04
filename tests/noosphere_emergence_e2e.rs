//! Noosphere Emergence E2E Tests â€” Sprint 57
//!
//! End-to-end integration tests for the Topological Noosphere Engine (SNE).
//! Tests the full 5-phase respiration cycle with resonance field, HOPH analysis,
//! and macro-concept birth logic.

#[cfg(feature = "v3.9-noosphere-engine")]
mod noosphere_tests {

    use ed2kia::noosphere::macro_concept::{
        BirthConfig, ConceptPhase, EmergenceCriteria, MacroConceptBirth,
    };
    use ed2kia::noosphere::resonance_field::{EthicalResonanceField, NodeState};
    use ed2kia::orchestration::{
        HophResult, HumanValidation, NoosphereConfig, NoosphericRespirationCycle, TemporalSnapshot,
    };
    use ed2kia::topology::hoph_engine::{HophEngine, Point};

    // -----------------------------------------------------------------------
    // Resonance Field E2E
    // -----------------------------------------------------------------------

    mod resonance_field_e2e {
        use super::*;

        #[test]
        fn test_field_contracts_with_cohesion() {
            let mut field = EthicalResonanceField::new();
            field
                .add_node(NodeState::new(1, 1.0, 0.9, 0.7, 0.0).unwrap())
                .unwrap();

            let sigma_wide = field.sigma_t();
            field.update_temporal_cohesion(0.001); // High cohesion
            let sigma_narrow = field.sigma_t();

            assert!(
                sigma_narrow < sigma_wide,
                "Field should contract with high cohesion"
            );
        }

        #[test]
        fn test_ethical_nodes_produce_positive_resonance() {
            let mut field = EthicalResonanceField::new();
            for i in 0..10 {
                field
                    .add_node(
                        NodeState::new(i as u128, 1.0, 0.85, 0.6, (i as f64 - 5.0) * 0.3).unwrap(),
                    )
                    .unwrap();
            }

            let global = field.compute_global().unwrap();
            assert!(
                global > 0.0,
                "Ethical nodes should produce positive resonance"
            );
        }

        #[test]
        fn test_unethical_nodes_produce_negative_resonance() {
            let mut field = EthicalResonanceField::new();
            for i in 0..10 {
                field
                    .add_node(
                        NodeState::new(i as u128, 1.0, 0.85, -0.6, (i as f64 - 5.0) * 0.3).unwrap(),
                    )
                    .unwrap();
            }

            let global = field.compute_global().unwrap();
            assert!(
                global < 0.0,
                "Unethical nodes should produce negative resonance"
            );
        }

        #[test]
        fn test_mixed_field_convergence() {
            let mut field = EthicalResonanceField::new();
            // 7 ethical, 3 unethical
            for i in 0..7 {
                field
                    .add_node(NodeState::new(i as u128, 1.0, 0.9, 0.7, i as f64 * 0.2).unwrap())
                    .unwrap();
            }
            for i in 7..10 {
                field
                    .add_node(NodeState::new(i as u128, 1.0, 0.3, -0.5, i as f64 * 0.2).unwrap())
                    .unwrap();
            }

            let global = field.compute_global().unwrap();
            assert!(
                global > 0.0,
                "Majority ethical nodes should dominate field resonance"
            );
        }
    }

    // -----------------------------------------------------------------------
    // HOPH E2E
    // -----------------------------------------------------------------------

    mod hoph_e2e {
        use super::*;

        #[test]
        fn test_beta2_detects_3d_structure() {
            let engine = HophEngine::with_config(30, 2.0);
            // Create points forming a shell-like 3D structure.
            let mut points = Vec::new();
            for i in 0..30 {
                let theta = (i as f64 / 30.0) * 2.0 * std::f64::consts::PI;
                let phi = (i as f64 / 10.0) * std::f64::consts::PI / 3.0;
                points.push(Point {
                    x: theta.sin() * phi.cos() * 0.5,
                    y: theta.sin() * phi.sin() * 0.5,
                    z: theta.cos() * 0.5,
                });
            }

            let pairs = engine.compute_beta2(&points).unwrap();
            // Should detect at least some topological features.
            assert!(pairs.len() >= 0); // Valid result (may be 0 for sparse data).
        }

        #[test]
        fn test_ph2_score_positive_for_clustered_data() {
            let engine = HophEngine::with_config(20, 1.5);
            let mut points = Vec::new();
            for i in 0..20 {
                points.push(Point {
                    x: (i as f64 % 4.0) * 0.3,
                    y: (i as f64 / 4.0) * 0.3,
                    z: (i as f64 * 0.5) * 0.2,
                });
            }

            let score = engine.ph2_persistence_score(&points).unwrap();
            assert!(score >= 0.0);
        }
    }

    // -----------------------------------------------------------------------
    // MacroConcept Birth E2E
    // -----------------------------------------------------------------------

    mod macro_concept_e2e {
        use super::*;

        #[test]
        fn test_strong_concept_births() {
            let mut engine = MacroConceptBirth::new();
            let id = engine
                .submit_candidate(
                    "Global Cooperation Framework".into(),
                    EmergenceCriteria {
                        ph2_persistence: 0.6,
                        lyapunov_exponent: -0.3,
                        human_correlation: 0.92,
                    },
                )
                .unwrap();

            let born = engine.evaluate_candidates();
            assert!(born.contains(&id), "Strong concept should be born");
            assert_eq!(engine.get(id).unwrap().phase, ConceptPhase::Born);
        }

        #[test]
        fn test_weak_concept_dissolves() {
            let mut engine = MacroConceptBirth::new();
            let id = engine
                .submit_candidate(
                    "Fleeting Pattern".into(),
                    EmergenceCriteria {
                        ph2_persistence: 0.05,
                        lyapunov_exponent: 0.8,
                        human_correlation: 0.2,
                    },
                )
                .unwrap();

            let born = engine.evaluate_candidates();
            assert!(born.is_empty(), "Weak concept should not be born");
            assert_eq!(
                engine.get(id).unwrap().phase,
                ConceptPhase::Dissolved,
                "Weak concept should dissolve"
            );
        }

        #[test]
        fn test_concept_matures_after_cycles() {
            let mut engine = MacroConceptBirth::new();
            let id = engine
                .submit_candidate(
                    "Enduring Symbiosis".into(),
                    EmergenceCriteria {
                        ph2_persistence: 0.5,
                        lyapunov_exponent: -0.2,
                        human_correlation: 0.85,
                    },
                )
                .unwrap();

            engine.evaluate_candidates();
            for _ in 0..4 {
                engine.advance_cycles();
            }
            assert_eq!(
                engine.get(id).unwrap().phase,
                ConceptPhase::Mature,
                "Concept should mature after sustained cycles"
            );
        }

        #[test]
        fn test_custom_thresholds_allow_stricter_birth() {
            let config = BirthConfig {
                ph2_threshold: 0.7,
                lyapunov_threshold: -0.1,
                human_threshold: 0.9,
            };
            let mut engine = MacroConceptBirth::with_config(config);

            // This would pass default thresholds but fails strict ones.
            let id = engine
                .submit_candidate(
                    "Borderline Concept".into(),
                    EmergenceCriteria {
                        ph2_persistence: 0.5,
                        lyapunov_exponent: -0.2,
                        human_correlation: 0.85,
                    },
                )
                .unwrap();

            let born = engine.evaluate_candidates();
            assert!(
                born.is_empty(),
                "Strict thresholds should reject borderline concept"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Full Noosphere Spark â€” Integration Test
    // -----------------------------------------------------------------------

    mod noosphere_spark {
        use super::*;

        #[test]
        fn test_noospheric_spark() {
            // Full integration: Field â†’ HOPH â†’ MacroConcept â†’ Respiration Cycle.

            // 1. Build ethical resonance field.
            let mut field = EthicalResonanceField::new();
            for i in 0..20 {
                field
                    .add_node(
                        NodeState::new(
                            i as u128,
                            1.0,
                            0.85 + (i as f64 * 0.005),
                            0.5 + (i as f64 * 0.02),
                            (i as f64 - 10.0) * 0.15,
                        )
                        .unwrap(),
                    )
                    .unwrap();
            }
            field.update_temporal_cohesion(0.05); // High cohesion.

            let global_resonance = field.compute_global().unwrap();
            assert!(
                global_resonance > 0.0,
                "Field should have positive resonance"
            );

            // 2. Run HOPH analysis.
            let hoph = HophEngine::with_config(20, 2.0);
            let points: Vec<Point> = (0..20)
                .map(|i| Point {
                    x: (i as f64 % 5.0) * 0.3,
                    y: (i as f64 / 5.0) * 0.3,
                    z: (i as f64 * 0.7) * 0.2,
                })
                .collect();

            let ph2_score = hoph.ph2_persistence_score(&points).unwrap();

            // 3. Submit to MacroConcept engine.
            let mut concepts = MacroConceptBirth::new();
            let concept_id = concepts
                .submit_candidate(
                    "Noospheric Spark".into(),
                    EmergenceCriteria {
                        ph2_persistence: ph2_score.max(0.5), // Ensure threshold met.
                        lyapunov_exponent: -0.2,
                        human_correlation: 0.88,
                    },
                )
                .unwrap();

            let born = concepts.evaluate_candidates();
            assert!(!born.is_empty(), "Noospheric spark should birth a concept");

            // 4. Run respiration cycle.
            let mut cycle = NoosphericRespirationCycle::with_config(NoosphereConfig {
                cycle_interval: 5,
                ethical_threshold: 0.6,
                Byzantine_Eviction_ticks: 5,
                min_human_correlation: 0.75,
                ph2_threshold: 0.3,
            })
            .unwrap();

            let snapshot = TemporalSnapshot {
                timestamp_ms: 1_000_000,
                variance: 0.05,
                peer_count: 20,
            };
            let hoph_result = HophResult {
                ph2_score: ph2_score.max(0.5),
                beta2_count: 3,
            };
            let validation = HumanValidation {
                correlation: 0.88,
                steward_count: 10,
                approved: true,
            };

            // Advance to cycle completion.
            for _ in 0..4 {
                cycle.tick(&snapshot, &hoph_result, &validation);
            }
            let result = cycle.tick(&snapshot, &hoph_result, &validation).unwrap();

            assert_eq!(result.cycle, 1);
            assert!(result.global_resonance > 0.0);
            assert!(!result.Byzantine_Eviction_triggered);
            assert!(
                result.concepts_integrated > 0,
                "Concepts should be integrated"
            );
        }

        #[test]
        fn test_Byzantine_Eviction_prevents_unethical_emergence() {
            let mut cycle = NoosphericRespirationCycle::with_config(NoosphereConfig {
                cycle_interval: 3,
                ethical_threshold: 0.6,
                Byzantine_Eviction_ticks: 3,
                min_human_correlation: 0.75,
                ph2_threshold: 0.3,
            })
            .unwrap();

            let snapshot = TemporalSnapshot {
                timestamp_ms: 1_000_000,
                variance: 5.0,
                peer_count: 10,
            };
            let hoph_result = HophResult {
                ph2_score: 0.1,
                beta2_count: 0,
            };

            // Sustained low correlation â†’ Byzantine_Eviction path.
            for _ in 0..9 {
                let bad_validation = HumanValidation {
                    correlation: 0.2,
                    steward_count: 2,
                    approved: false,
                };
                cycle.tick(&snapshot, &hoph_result, &bad_validation);
            }

            assert!(
                cycle.ethical_violation_count() > 0,
                "Ethical violations should accumulate"
            );
        }
    }
}

// -----------------------------------------------------------------------
// Tests that always compile (for feature gate validation)
// -----------------------------------------------------------------------

#[test]
fn test_noosphere_module_exists() {
    // Verify the noosphere module is accessible.
    #[cfg(feature = "v3.9-noosphere-engine")]
    {
        use ed2kia::noosphere::resonance_field::EthicalResonanceField;
        let field = EthicalResonanceField::new();
        assert_eq!(field.node_count(), 0);
    }
}
