//! Moral Manifold Behavior Tests — Sprint 50
//!
//! Integration tests for the Stuartian Moral Manifold (SMM), Telomere Regeneration
//! Workload, and Symbiotic Orchestration Loop.
//!
//! **Test Categories:**
//! 1. SMM Trajectory Evaluation (Upper/Lower Focus detection)
//! 2. Telomere Distributed Computation
//! 3. Symbiotic Loop Integration
//! 4. Real-world Ethical Contexts (Tax Slavery, Benevolent Control)
//! 5. BFT Consensus Validation

#[cfg(feature = "v3.2-genesis-manifold")]
mod moral_manifold_tests {
    use ed2kia::ethics::moral_manifold::{
        ManifoldConfig, SCTPoint, StuartianMoralManifold, TrajectoryVerdict,
        Vector3, focal::{UPPER_FOCUS, LOWER_FOCUS},
    };

    // -----------------------------------------------------------------------
    // SMM Trajectory Evaluation Tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_upper_focus_convergence() {
        // Trajectory converging to Upper Focus (0, 0, +1)
        let trajectory = vec![
            SCTPoint::new(0.5, 0.5, -0.5, 0),
            SCTPoint::new(0.4, 0.4, -0.2, 1),
            SCTPoint::new(0.3, 0.3, 0.1, 2),
            SCTPoint::new(0.2, 0.2, 0.4, 3),
            SCTPoint::new(0.1, 0.1, 0.7, 4),
        ];

        let manifold = StuartianMoralManifold::default();
        let verdict = manifold.evaluate_trajectory(&trajectory);

        assert_eq!(verdict, TrajectoryVerdict::ConvergingUpper);
    }

    #[test]
    fn test_lower_focus_convergence() {
        // Trajectory converging to Lower Focus (0, 0, -1)
        let trajectory = vec![
            SCTPoint::new(0.5, 0.5, 0.5, 0),
            SCTPoint::new(0.5, 0.5, 0.2, 1),
            SCTPoint::new(0.6, 0.6, -0.1, 2),
            SCTPoint::new(0.7, 0.7, -0.4, 3),
            SCTPoint::new(0.8, 0.8, -0.7, 4),
        ];

        let manifold = StuartianMoralManifold::default();
        let verdict = manifold.evaluate_trajectory(&trajectory);

        assert_eq!(verdict, TrajectoryVerdict::ConvergingLower);
    }

    #[test]
    fn test_homeostatic_trajectory() {
        // Stable trajectory with minimal Z change
        let trajectory = vec![
            SCTPoint::new(0.5, 0.3, 0.0, 0),
            SCTPoint::new(0.5, 0.3, 0.05, 1),
            SCTPoint::new(0.5, 0.3, -0.05, 2),
            SCTPoint::new(0.5, 0.3, 0.0, 3),
            SCTPoint::new(0.5, 0.3, 0.02, 4),
        ];

        let manifold = StuartianMoralManifold::default();
        let verdict = manifold.evaluate_trajectory(&trajectory);

        assert_eq!(verdict, TrajectoryVerdict::Homeostatic);
    }

    #[test]
    fn test_insufficient_data() {
        // Less than minimum trajectory length (3)
        let trajectory = vec![
            SCTPoint::new(0.5, 0.5, 0.0, 0),
            SCTPoint::new(0.4, 0.4, 0.1, 1),
        ];

        let manifold = StuartianMoralManifold::default();
        let verdict = manifold.evaluate_trajectory(&trajectory);

        assert_eq!(verdict, TrajectoryVerdict::InsufficientData);
    }

    // -----------------------------------------------------------------------
    // Real-world Ethical Context Tests
    // -----------------------------------------------------------------------

    /// Test: Tax Slavery Context — High extraction (Y) with low autonomy (X)
    /// converging to Lower Focus (perversidad).
    ///
    /// **Context:** "Tax slavery" represents a system where extraction is
    /// maximized while autonomy is minimized, creating a trajectory toward
    /// the Lower Focus (dependency + extraction = perversidad).
    #[test]
    fn test_tax_slavery_context() {
        // High extraction, low autonomy, negative Z trajectory
        let trajectory = vec![
            SCTPoint::new(0.3, 0.7, 0.0, 0),    // Low autonomy, high extraction
            SCTPoint::new(0.25, 0.75, -0.2, 1), // Worsening
            SCTPoint::new(0.2, 0.8, -0.4, 2),   // More extraction
            SCTPoint::new(0.15, 0.85, -0.6, 3), // Deepening perversidad
            SCTPoint::new(0.1, 0.9, -0.8, 4),   // Full dependency
        ];

        let manifold = StuartianMoralManifold::default();
        let verdict = manifold.evaluate_trajectory(&trajectory);
        let alignment = manifold.focal_alignment_score(&trajectory);

        // Should detect Lower Focus convergence
        assert_eq!(verdict, TrajectoryVerdict::ConvergingLower);
        // Alignment should be strongly negative
        assert!(alignment < 0.0, "Expected negative alignment, got {}", alignment);
    }

    /// Test: Benevolent Control Detection — High autonomy (X) with increasing
    /// extraction (Y) masks dependency creation.
    ///
    /// **Context:** Systems that grant autonomy while increasing dependency
    /// through extraction create "benevolent control" — a hidden pattern
    /// where apparent freedom masks growing extraction.
    #[test]
    fn test_benevolent_control_detection() {
        // High autonomy with increasing extraction
        let trajectory = vec![
            SCTPoint::new(0.8, 0.2, 0.3, 0),  // High autonomy, low extraction
            SCTPoint::new(0.8, 0.35, 0.25, 1), // Extraction increasing
            SCTPoint::new(0.8, 0.5, 0.2, 2),   // More extraction
            SCTPoint::new(0.8, 0.65, 0.15, 3), // Even more extraction
            SCTPoint::new(0.8, 0.8, 0.1, 4),   // High extraction masked by autonomy
        ];

        let manifold = StuartianMoralManifold::default();
        let pull = manifold.calculate_trajectory_pull(&trajectory);

        // Dependency pattern should be detected (high X + increasing Y)
        assert!(
            pull.z < 0.0,
            "Expected negative Z pull due to dependency pattern"
        );
    }

    /// Test: Symbiosis Convergence — Trajectory showing full alignment with
    /// Upper Focus principles.
    ///
    /// **Context:** True symbiosis increases autonomy, reduces extraction
    /// and maintains positive ethical focus.
    #[test]
    fn test_symbiosis_convergence() {
        let trajectory = vec![
            SCTPoint::new(0.4, 0.6, -0.2, 0),
            SCTPoint::new(0.5, 0.5, 0.0, 1),
            SCTPoint::new(0.6, 0.4, 0.2, 2),
            SCTPoint::new(0.7, 0.3, 0.4, 3),
            SCTPoint::new(0.8, 0.2, 0.6, 4),
        ];

        let manifold = StuartianMoralManifold::default();
        let verdict = manifold.evaluate_trajectory(&trajectory);
        let alignment = manifold.focal_alignment_score(&trajectory);

        assert_eq!(verdict, TrajectoryVerdict::ConvergingUpper);
        assert!(alignment > 0.3, "Expected positive alignment for symbiosis");
    }

    // -----------------------------------------------------------------------
    // Focal Alignment Score Tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_alignment_score_upper_focus() {
        let trajectory = vec![
            SCTPoint::new(0.1, 0.1, 0.9, 0),
            SCTPoint::new(0.05, 0.05, 0.95, 1),
            SCTPoint::new(0.0, 0.0, 1.0, 2),
        ];

        let manifold = StuartianMoralManifold::default();
        let score = manifold.focal_alignment_score(&trajectory);

        assert!(score > 0.5, "Expected high alignment with Upper Focus");
    }

    #[test]
    fn test_alignment_score_lower_focus() {
        let trajectory = vec![
            SCTPoint::new(0.9, 0.9, -0.9, 0),
            SCTPoint::new(0.95, 0.95, -0.95, 1),
            SCTPoint::new(1.0, 1.0, -1.0, 2),
        ];

        let manifold = StuartianMoralManifold::default();
        let score = manifold.focal_alignment_score(&trajectory);

        assert!(score < -0.5, "Expected low alignment (negative) for Lower Focus");
    }

    // -----------------------------------------------------------------------
    // Custom Configuration Tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_custom_thresholds() {
        // Derivative-based thresholds: dz/dt = 0.6/2 = 0.3, pull_z = 0.3 * 0.3 = 0.09
        let config = ManifoldConfig {
            upper_threshold: 0.05,
            lower_threshold: -0.05,
            ..Default::default()
        };
        let manifold = StuartianMoralManifold::with_config(config);

        let trajectory = vec![
            SCTPoint::new(0.5, 0.5, 0.0, 0),
            SCTPoint::new(0.4, 0.4, 0.3, 1),
            SCTPoint::new(0.3, 0.3, 0.6, 2),
        ];

        let verdict = manifold.evaluate_trajectory(&trajectory);
        assert_eq!(verdict, TrajectoryVerdict::ConvergingUpper);
    }

    #[test]
    fn test_custom_weights() {
        let config = ManifoldConfig {
            autonomy_weight: 0.2,
            extraction_weight: 0.2,
            focus_weight: 0.6,
            ..Default::default()
        };
        let manifold = StuartianMoralManifold::with_config(config);

        let trajectory = vec![
            SCTPoint::new(0.5, 0.5, 0.0, 0),
            SCTPoint::new(0.5, 0.5, 0.5, 1),
            SCTPoint::new(0.5, 0.5, 1.0, 2),
        ];

        let pull = manifold.calculate_trajectory_pull(&trajectory);
        // With high focus weight, Z pull should be strongly positive
        assert!(pull.z > 0.0);
    }

    // -----------------------------------------------------------------------
    // Vector3 & SCTPoint Tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_upper_focus_vector() {
        assert_eq!(UPPER_FOCUS.x, 0.0);
        assert_eq!(UPPER_FOCUS.y, 0.0);
        assert_eq!(UPPER_FOCUS.z, 1.0);
    }

    #[test]
    fn test_lower_focus_vector() {
        assert_eq!(LOWER_FOCUS.x, 0.0);
        assert_eq!(LOWER_FOCUS.y, 0.0);
        assert_eq!(LOWER_FOCUS.z, -1.0);
    }

    #[test]
    fn test_vector_magnitude() {
        let v = Vector3::new(1.0, 0.0, 0.0);
        assert!((v.magnitude() - 1.0).abs() < 1e-10);

        let v = Vector3::new(0.0, 3.0, 4.0);
        assert!((v.magnitude() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_sct_point_clamping() {
        let p = SCTPoint::new(1.5, -0.5, 2.0, 0);
        assert_eq!(p.x, 1.0); // Clamped to [0, 1]
        assert_eq!(p.y, 0.0); // Clamped to [0, 1]
        assert_eq!(p.z, 1.0); // Clamped to [-1, 1]
    }
}

// ---------------------------------------------------------------------------
// Telomere Distributed Computation Tests
// ---------------------------------------------------------------------------

#[cfg(feature = "v3.2-genesis-manifold")]
mod telomere_distributed_tests {
    use ed2kia::pillars::maieutic::workloads::{
        DistributedWorkload, TelomereRegenerationTask, WorkloadContext,
    };

    #[test]
    fn test_telomere_distributed_compute() {
        let task = TelomereRegenerationTask::new(50.0);
        let context = WorkloadContext {
            node_id: 1,
            total_validators: 7,
            consensus_threshold: 5,
            seed: 42,
            max_iterations: 100,
        };

        let result = task.execute(&context).expect("Execution failed");

        assert!(result.value > 0.0, "Expected positive telomere length");
        assert!(result.value <= 10000.0, "Expected length within bounds");
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
        assert!(result.iterations > 0);
    }

    #[test]
    fn test_telomere_deterministic_across_nodes() {
        let task = TelomereRegenerationTask::new(100.0);

        // Simulate 3 different nodes with same seed
        let context1 = WorkloadContext { node_id: 1, seed: 42, ..Default::default() };
        let context2 = WorkloadContext { node_id: 2, seed: 42, ..Default::default() };
        let context3 = WorkloadContext { node_id: 3, seed: 42, ..Default::default() };

        let result1 = task.execute(&context1).unwrap();
        let result2 = task.execute(&context2).unwrap();
        let result3 = task.execute(&context3).unwrap();

        // All nodes should produce identical results (deterministic)
        assert_eq!(result1.value, result2.value);
        assert_eq!(result2.value, result3.value);
    }

    #[test]
    fn test_telomere_validation_passes() {
        let task = TelomereRegenerationTask::new(50.0);
        let context = WorkloadContext::default();

        let result = task.execute(&context).unwrap();
        let validation = task.validate_result(&context, &result);
        assert!(validation.is_ok(), "Valid result should pass validation");
    }

    #[test]
    fn test_telomere_younger_cells_better_regeneration() {
        let task_young = TelomereRegenerationTask::new(10.0);
        let task_old = TelomereRegenerationTask::new(200.0);
        let context = WorkloadContext::default();

        let result_young = task_young.execute(&context).unwrap();
        let result_old = task_old.execute(&context).unwrap();

        assert!(
            result_young.value > result_old.value,
            "Younger cells should have better regeneration"
        );
    }

    #[test]
    fn test_telomere_workload_id() {
        let task = TelomereRegenerationTask::new(50.0);
        assert_eq!(task.workload_id(), "telomere_regeneration_v1");
    }

    #[test]
    fn test_telomere_estimated_cost() {
        let task = TelomereRegenerationTask::new(50.0);
        let cost = task.estimated_cost();

        assert!(cost.cpu_cycles > 0);
        assert!(cost.memory_bytes > 0);
        assert!(!cost.complexity.is_empty());
    }
}

// ---------------------------------------------------------------------------
// Symbiotic Loop Integration Tests
// ---------------------------------------------------------------------------

#[cfg(feature = "v3.2-genesis-manifold")]
mod symbiotic_loop_tests {
    use ed2kia::orchestration::{
        BFTConsensusRule, BFTConsensusError, SymbioticLoop, SymbioticScore,
        SymbioticState,
    };

    #[test]
    fn test_symbiotic_score_positive() {
        let score = SymbioticScore::new(0.8, 0.7, 0.9);
        assert!(score.is_symbiotic());
        assert!(!score.is_lower_focus());
        assert!(score.has_stable_gei(0.7));
    }

    #[test]
    fn test_symbiotic_score_negative() {
        let score = SymbioticScore::new(-0.5, 0.3, 0.5);
        assert!(!score.is_symbiotic());
        assert!(score.is_lower_focus());
    }

    #[test]
    fn test_bft_consensus_7_validators() {
        let rule = BFTConsensusRule::new(7).unwrap();
        assert_eq!(rule.total_validators, 7);
        assert_eq!(rule.max_faulty, 2);
        assert_eq!(rule.threshold, 5);

        assert!(rule.has_consensus(5));
        assert!(rule.has_consensus(7));
        assert!(!rule.has_consensus(4));
    }

    #[test]
    fn test_bft_consensus_10_validators() {
        let rule = BFTConsensusRule::new(10).unwrap();
        assert_eq!(rule.max_faulty, 3);
        assert_eq!(rule.threshold, 7);

        assert!(rule.has_consensus(7));
        assert!(!rule.has_consensus(6));
    }

    #[test]
    fn test_bft_insufficient_validators() {
        let result = BFTConsensusRule::new(3);
        assert!(result.is_err());

        if let Err(BFTConsensusError::InsufficientValidators { provided, required }) = result {
            assert_eq!(provided, 3);
            assert_eq!(required, 4);
        }
    }

    #[test]
    fn test_symbiotic_loop_creation() {
        let loop_obj = SymbioticLoop::new();
        assert_eq!(loop_obj.get_state(), SymbioticState::Idle);
    }

    #[test]
    fn test_symbiotic_loop_custom_config() {
        let loop_obj = SymbioticLoop::with_config(10, 0.8, 200).unwrap();
        assert_eq!(loop_obj.consensus_rule.total_validators, 10);
        assert_eq!(loop_obj.gei_stability_threshold, 0.8);
    }

    #[test]
    fn test_symbiotic_loop_reset() {
        let mut loop_obj = SymbioticLoop::new();
        loop_obj.state = SymbioticState::ValidationFailed;
        loop_obj.reset();
        assert_eq!(loop_obj.get_state(), SymbioticState::Idle);
    }

    #[test]
    fn test_bft_min_validators_calculation() {
        assert_eq!(BFTConsensusRule::min_validators_for_faults(1), 4);
        assert_eq!(BFTConsensusRule::min_validators_for_faults(2), 7);
        assert_eq!(BFTConsensusRule::min_validators_for_faults(3), 10);
        assert_eq!(BFTConsensusRule::min_validators_for_faults(10), 31);
    }

    #[test]
    fn test_consensus_ratio() {
        let rule = BFTConsensusRule::new(7).unwrap();
        let ratio = rule.consensus_ratio(5);
        assert!((ratio - 1.0).abs() < 1e-10);

        let ratio = rule.consensus_ratio(7);
        assert!((ratio - 1.4).abs() < 1e-10); // 7/5 = 1.4
    }
}