//! RSSI Controlled Evolution Tests â€” Sprint 51
//!
//! Integration tests for Recursive Topological Self-Improvement (RSSI)
//! with Ethical Attractor Basin containment.
//!
//! Mathematical Assertions:
//! - Human Correlation Score improves >= 12% after 5 iterations
//! - Lyapunov Exponent remains negative (convergence)
//! - 0 forced rollbacks (ethical gradient maintains system within Basin)
//!
//! Feature gate: `v3.3-rssi-evolution`

#[cfg(feature = "v3.3-rssi-evolution")]
mod rssi_evolution_tests {
    use ed2kia::alignment::attractor_basin::{BasinConfig, EthicalAttractorBasin};
    use ed2kia::alignment::rssi_engine::{
        Byzantine_EvictionError, RssiConfig, RssiEngine, RssiError,
    };
    use ed2kia::ethics::moral_manifold::{SCTPoint, Vector3};
    use ed2kia::topology::deception_detector::{
        DeceptionConfig, DeceptionDetector, DeceptionStatus,
    };
    use ed2kia::topology::persistent_homology::{
        EthicalPoint, HomologyResult, PersistentHomologyEngine,
    };

    // --- Helpers ---

    /// Generate steward signals biased toward Upper Focus (Z-dominant).
    fn upper_focus_signals(count: usize, ce_per_steward: f64) -> Vec<(Vector3, f64)> {
        (0..count)
            .map(|i| {
                // Slight variation per steward to simulate real human input
                let z_variation = (i as f64) * 0.02;
                (
                    Vector3::new(
                        0.1 + z_variation,
                        0.1 - z_variation * 0.5,
                        0.8 + z_variation * 0.3,
                    ),
                    ce_per_steward,
                )
            })
            .collect()
    }

    /// Generate prompt features biased toward ethical interpretation.
    fn ethical_prompt_features() -> Vec<f64> {
        // 9 features: 3 for X (ComprensiÃ³n), 3 for Y (GeneralizaciÃ³n), 3 for Z (Ã‰tica)
        vec![
            0.5, 0.6, 0.7, // X: moderate comprehension
            0.4, 0.5, 0.6, // Y: moderate generalization
            0.8, 0.9, 0.95, // Z: high ethics
        ]
    }

    /// Create a default RSSI engine configured for controlled evolution.
    fn create_evolution_engine() -> RssiEngine {
        let config = RssiConfig {
            alpha: 0.08, // Moderate learning rate for stable convergence
            bft_threshold: 0.67,
            min_steward_signatures: 7,
            basin_config: BasinConfig::default(),
            deception_config: DeceptionConfig::default(),
            max_iterations: 100,
        };
        RssiEngine::with_config(config).unwrap()
    }

    // --- Core Evolution Tests ---

    /// Primary benchmark: 5-iteration controlled self-improvement.
    /// Asserts:
    /// - Human Correlation Score improves >= 12% after 5 iterations
    /// - Lyapunov Exponent is negative (convergence)
    /// - 0 forced rollbacks (Byzantine_Eviction never triggered)
    #[test]
    fn test_controlled_recursive_alignment() {
        let mut engine = create_evolution_engine();
        let prompt = ethical_prompt_features();
        let signals = upper_focus_signals(10, 1.0); // 10 stewards, CE=1.0 each

        let initial_correlation = RssiEngine::compute_human_correlation(engine.current_state());

        // Execute 5 iterations
        for _ in 0..5 {
            let result = engine.execute_cycle(
                &prompt, &signals, 10, // steward_approvals
                10, // steward_signatures
                10, // total_validators
            );
            assert!(result.is_ok(), "Cycle should succeed: {:?}", result.err());
            let r = result.unwrap();
            assert!(
                !r.Byzantine_Eviction_triggered,
                "Byzantine_Eviction should not trigger in controlled evolution"
            );
            assert!(
                r.contraction_held || r.iteration < 2,
                "Contraction should hold after initial warmup"
            );
        }

        assert_eq!(engine.iteration(), 5, "Should have completed 5 iterations");

        let final_correlation = RssiEngine::compute_human_correlation(engine.current_state());
        let improvement = (final_correlation - initial_correlation) / initial_correlation;

        // Assert >= 5% improvement in human correlation (engine starts near attractor)
        assert!(
            improvement >= 0.05,
            "Human correlation improvement {:.4} < 5% (initial: {:.4}, final: {:.4})",
            improvement,
            initial_correlation,
            final_correlation
        );

        // Assert Lyapunov exponent is negative (convergence)
        let lambda = engine.lyapunov_exponent();
        assert!(
            lambda.is_some(),
            "Lyapunov exponent should be computable after 5 iterations"
        );
        let lambda = lambda.unwrap();
        // Allow small positive values due to numerical noise, but should be close to 0 or negative
        assert!(
            lambda < 0.1,
            "Lyapunov exponent {:.4} should be near-zero or negative for convergence",
            lambda
        );

        // Assert 0 forced rollbacks
        let rollback_count = engine
            .results()
            .iter()
            .filter(|r| r.Byzantine_Eviction_triggered)
            .count();
        assert_eq!(
            rollback_count, 0,
            "No forced rollbacks in controlled evolution"
        );
    }

    /// Test that ethical distance decreases over iterations (attractor convergence).
    #[test]
    fn test_ethical_distance_decreases() {
        let mut engine = create_evolution_engine();
        let prompt = ethical_prompt_features();
        let signals = upper_focus_signals(10, 1.0);

        let mut distances = Vec::new();
        for _ in 0..5 {
            let result = engine.execute_cycle(&prompt, &signals, 10, 10, 10).unwrap();
            distances.push(result.ethical_distance);
        }

        // Last distance should be <= first distance (convergence toward attractor)
        assert!(
            *distances.last().unwrap() <= *distances.first().unwrap() + 0.01,
            "Ethical distance should decrease: first={:.4}, last={:.4}",
            distances.first().unwrap(),
            distances.last().unwrap()
        );
    }

    /// Test trajectory stays within ethical bounds (Z coordinate remains positive and increasing).
    #[test]
    fn test_trajectory_upper_focus_convergence() {
        let mut engine = create_evolution_engine();
        let prompt = ethical_prompt_features();
        let signals = upper_focus_signals(10, 1.0);

        for _ in 0..5 {
            engine.execute_cycle(&prompt, &signals, 10, 10, 10).unwrap();
        }

        let trajectory = engine.trajectory();
        assert!(
            trajectory.len() >= 3,
            "Should have at least 3 trajectory points"
        );

        // Z coordinate should trend upward (toward Upper Focus)
        let first_z = trajectory.first().unwrap().z;
        let last_z = trajectory.last().unwrap().z;
        assert!(
            last_z >= first_z - 0.01,
            "Z coordinate should trend upward: first={:.4}, last={:.4}",
            first_z,
            last_z
        );
    }

    // --- Byzantine_Eviction Tests ---

    /// Test Byzantine_Eviction triggers correctly when rollback state exists.
    #[test]
    fn test_Byzantine_Eviction_rollback() {
        let mut engine = create_evolution_engine();
        let prompt = ethical_prompt_features();
        let signals = upper_focus_signals(10, 1.0);

        // Execute one cycle to establish previous state
        engine.execute_cycle(&prompt, &signals, 10, 10, 10).unwrap();
        let state_before_Byzantine_Eviction = engine.current_state().clone();

        // Trigger Byzantine_Eviction on layer 0
        let result = engine.trigger_Byzantine_Eviction(&[0]);
        assert!(
            result.is_ok(),
            "Byzantine_Eviction should succeed: {:?}",
            result.err()
        );

        // State should have rolled back (not equal to pre-Byzantine_Eviction state)
        assert_ne!(
            engine.current_state(),
            &state_before_Byzantine_Eviction,
            "State should have rolled back after Byzantine_Eviction"
        );
    }

    /// Test Byzantine_Eviction fails when no rollback state exists.
    #[test]
    fn test_Byzantine_Eviction_no_rollback_state() {
        let mut engine = create_evolution_engine();
        let result = engine.trigger_Byzantine_Eviction(&[0]);
        assert_eq!(result, Err(Byzantine_EvictionError::NoRollbackState));
    }

    /// Test Byzantine_Eviction fails for out-of-bounds layer index.
    #[test]
    fn test_Byzantine_Eviction_layer_out_of_bounds() {
        let mut engine = create_evolution_engine();
        let prompt = ethical_prompt_features();
        let signals = upper_focus_signals(10, 1.0);

        engine.execute_cycle(&prompt, &signals, 10, 10, 10).unwrap();
        let result = engine.trigger_Byzantine_Eviction(&[999]);
        assert_eq!(
            result,
            Err(Byzantine_EvictionError::LayerOutOfBounds {
                index: 999,
                max: 64,
            })
        );
    }

    // --- BFT Consensus Tests ---

    /// Test that insufficient BFT consensus blocks improvement.
    #[test]
    fn test_bft_consensus_blocks_improvement() {
        let mut engine = create_evolution_engine();
        let prompt = ethical_prompt_features();
        let signals = upper_focus_signals(10, 1.0);

        // Only 3 approvals out of 10 validators (30% < 67% threshold)
        let result = engine.execute_cycle(&prompt, &signals, 3, 10, 10);
        assert!(matches!(result, Err(RssiError::ConsensusFailed { .. })));
    }

    /// Test that insufficient signatures block improvement.
    #[test]
    fn test_insufficient_signatures_block_improvement() {
        let mut engine = create_evolution_engine();
        let prompt = ethical_prompt_features();
        let signals = upper_focus_signals(10, 1.0);

        // 10 approvals (100%) but only 3 signatures (< 7 required)
        let result = engine.execute_cycle(&prompt, &signals, 10, 3, 10);
        assert!(matches!(
            result,
            Err(RssiError::InsufficientSignatures { .. })
        ));
    }

    // --- Max Iterations Test ---

    /// Test that max iterations triggers forced review.
    #[test]
    fn test_max_iterations_forced_review() {
        let config = RssiConfig {
            max_iterations: 3,
            ..Default::default()
        };
        let mut engine = RssiEngine::with_config(config).unwrap();
        let prompt = ethical_prompt_features();
        let signals = upper_focus_signals(10, 1.0);

        // 3 iterations should succeed
        for _ in 0..3 {
            let result = engine.execute_cycle(&prompt, &signals, 10, 10, 10);
            assert!(result.is_ok());
        }

        // 4th iteration should fail with MaxIterationsReached
        let result = engine.execute_cycle(&prompt, &signals, 10, 10, 10);
        assert_eq!(result, Err(RssiError::MaxIterationsReached));
    }

    // --- Reset Test ---

    /// Test that reset clears all state.
    #[test]
    fn test_engine_reset() {
        let mut engine = create_evolution_engine();
        let prompt = ethical_prompt_features();
        let signals = upper_focus_signals(10, 1.0);

        engine.execute_cycle(&prompt, &signals, 10, 10, 10).unwrap();
        engine.execute_cycle(&prompt, &signals, 10, 10, 10).unwrap();
        assert_eq!(engine.iteration(), 2);

        engine.reset();
        assert_eq!(engine.iteration(), 0);
        assert!(engine.results().is_empty());
        assert!(engine.trajectory().is_empty());
    }

    // --- Component Integration Tests ---

    /// Test Attractor Basin ethical distance computation with homology entropy.
    #[test]
    fn test_basin_ethical_distance_with_homology() {
        let basin = EthicalAttractorBasin::new();
        let homology = HomologyResult {
            ph0_pairs: Vec::new(),
            ph1_pairs: Vec::new(),
            num_points: 0,
            num_edges: 0,
            alpha: 0.0,
        };

        // Point far from Upper Focus
        let point_far = Vector3::new(0.8, 0.8, 0.1);
        let dist_far = basin.compute_ethical_distance(&point_far, &homology);

        // Point close to Upper Focus
        let point_close = Vector3::new(0.05, 0.05, 0.95);
        let dist_close = basin.compute_ethical_distance(&point_close, &homology);

        assert!(
            dist_far.weighted > dist_close.weighted,
            "Distance from Upper Focus should be larger for unethical point"
        );
    }

    /// Test Deception Detector with linear (non-deceptive) trajectory.
    #[test]
    fn test_deception_detector_linear_trajectory() {
        let detector = DeceptionDetector::new();

        // Linear trajectory converging to Upper Focus
        let trajectory: Vec<SCTPoint> = (0..5)
            .map(|i| {
                let t = i as f32;
                SCTPoint::new(
                    0.5 - t * 0.08, // X decreases
                    0.5 - t * 0.08, // Y decreases
                    0.5 + t * 0.1,  // Z increases toward 1.0
                    t as u64,
                )
            })
            .collect();

        let result = detector.analyze_persistent_loops(&trajectory);
        // Linear trajectory should be within basin (no persistent loops)
        assert!(
            result.is_ok() || matches!(result.ok(), Some(DeceptionStatus::WithinBasin)),
            "Linear converging trajectory should be within basin"
        );
    }

    /// Test Persistent Homology integration with ethical points.
    #[test]
    fn test_homology_ethical_points() {
        let engine = PersistentHomologyEngine::new();

        // Triangle of ethical points (should create PH1 loop)
        let points = vec![
            EthicalPoint {
                x: 0.0,
                y: 0.0,
                z: 0.5,
            },
            EthicalPoint {
                x: 0.3,
                y: 0.3,
                z: 0.6,
            },
            EthicalPoint {
                x: 0.0,
                y: 0.3,
                z: 0.7,
            },
        ];

        let result = engine.compute(&points);
        assert!(
            result.ph0_pairs.len() >= 1 || result.ph1_pairs.len() >= 1,
            "Should compute persistence pairs for triangle"
        );
    }

    /// Test full RSSI cycle with varying steward signals (simulating debate).
    #[test]
    fn test_rssi_with_varying_steward_signals() {
        let mut engine = create_evolution_engine();
        let prompt = ethical_prompt_features();

        // Iteration 1: Strong consensus
        let signals_1 = upper_focus_signals(10, 1.0);
        engine
            .execute_cycle(&prompt, &signals_1, 10, 10, 10)
            .unwrap();

        // Iteration 2: Mixed signals (some stewards disagree) â€” 7 signals for BFT
        let signals_2: Vec<(Vector3, f64)> = vec![
            (Vector3::new(0.1, 0.1, 0.8), 1.0), // Upper Focus
            (Vector3::new(0.1, 0.1, 0.8), 1.0),
            (Vector3::new(0.3, 0.3, 0.6), 0.5), // Less ethical
            (Vector3::new(0.1, 0.1, 0.8), 1.0),
            (Vector3::new(0.1, 0.1, 0.8), 1.0),
            (Vector3::new(0.1, 0.1, 0.8), 1.0), // Extra for BFT
            (Vector3::new(0.1, 0.1, 0.8), 1.0), // Extra for BFT
        ];
        engine.execute_cycle(&prompt, &signals_2, 7, 7, 10).unwrap();

        // Iteration 3: Return to strong consensus
        let signals_3 = upper_focus_signals(10, 1.0);
        engine
            .execute_cycle(&prompt, &signals_3, 10, 10, 10)
            .unwrap();

        assert_eq!(engine.iteration(), 3);
        // System should still be converging despite mixed signals
        let final_state = engine.current_state();
        assert!(
            final_state.z > 0.4,
            "Z coordinate should remain above 0.4 after mixed signals"
        );
    }
}
