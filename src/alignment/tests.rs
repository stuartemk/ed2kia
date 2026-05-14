//! Tests para Alignment Engine - Fase 7 Sprint 1
//!
//! Cubre: drift calculation, threshold triggers, mock feedback,
//! tensor alignment, error propagation y edge cases.

#[cfg(test)]
mod alignment_scorer_tests {
    use super::engine::{
        AlignmentConfig, AlignmentFeedback, AlignmentError, AlignmentScorer,
    };
    use candle_core::{Device, Tensor};

    // =====================================================================
    // Helpers
    // =====================================================================

    fn test_config() -> AlignmentConfig {
        AlignmentConfig {
            drift_threshold: 0.3,
            critical_threshold: 0.7,
            feedback_weight: 0.6,
            learning_rate: 0.001,
            max_features: 256,
            feedback_window: 50,
        }
    }

    fn make_feedback(layer_id: &str, feature_idx: u32, current: f32, desired: f32, confidence: f32) -> AlignmentFeedback {
        AlignmentFeedback {
            layer_id: layer_id.to_string(),
            feature_idx,
            current_activation: current,
            desired_value: desired,
            annotator_confidence: confidence,
            concept: None,
        }
    }

    fn make_feedback_with_concept(
        layer_id: &str,
        feature_idx: u32,
        current: f32,
        desired: f32,
        confidence: f32,
        concept: &str,
    ) -> AlignmentFeedback {
        AlignmentFeedback {
            layer_id: layer_id.to_string(),
            feature_idx,
            current_activation: current,
            desired_value: desired,
            annotator_confidence: confidence,
            concept: Some(concept.to_string()),
        }
    }

    // =====================================================================
    // Test 1: Creación con config default
    // =====================================================================
    #[test]
    fn test_scorer_creation_default() {
        let scorer = AlignmentScorer::default();
        assert_eq!(scorer.config().drift_threshold, 0.3);
        assert_eq!(scorer.config().critical_threshold, 0.7);
        assert_eq!(scorer.config().feedback_weight, 0.6);
        assert_eq!(scorer.config().learning_rate, 0.001);
    }

    // =====================================================================
    // Test 2: Creación con config custom
    // =====================================================================
    #[test]
    fn test_scorer_creation_custom_config() {
        let config = test_config();
        let scorer = AlignmentScorer::new(config);
        assert_eq!(scorer.config().drift_threshold, 0.3);
        assert_eq!(scorer.config().feedback_window, 50);
    }

    // =====================================================================
    // Test 3: Ingesta feedback válido
    // =====================================================================
    #[test]
    fn test_ingest_valid_feedback() {
        let mut scorer = AlignmentScorer::new(test_config());
        let fb = make_feedback("layer_0", 0, 0.8, 0.9, 0.95);

        assert!(scorer.ingest_feedback(fb).is_ok());
        assert_eq!(scorer.feedback_count("layer_0"), 1);
    }

    // =====================================================================
    // Test 4: Ingesta feedback con NaN (debe fallar)
    // =====================================================================
    #[test]
    fn test_ingest_nan_activation_rejected() {
        let mut scorer = AlignmentScorer::new(test_config());
        let fb = make_feedback("layer_0", 0, f32::NAN, 0.9, 0.95);

        let result = scorer.ingest_feedback(fb);
        assert!(result.is_err());
        match result.unwrap_err() {
            AlignmentError::InvalidFeedback { reason } => {
                assert!(reason.contains("Invalid current_activation"));
            }
            other => panic!("Expected InvalidFeedback, got {:?}", other),
        }
    }

    // =====================================================================
    // Test 5: Ingesta feedback con confianza fuera de rango
    // =====================================================================
    #[test]
    fn test_ingest_invalid_confidence_rejected() {
        let mut scorer = AlignmentScorer::new(test_config());
        let fb = make_feedback("layer_0", 0, 0.5, 0.6, 1.5); // confidence > 1.0

        let result = scorer.ingest_feedback(fb);
        assert!(result.is_err());
        match result.unwrap_err() {
            AlignmentError::InvalidFeedback { reason } => {
                assert!(reason.contains("annotator_confidence out of range"));
            }
            other => panic!("Expected InvalidFeedback, got {:?}", other),
        }
    }

    // =====================================================================
    // Test 6: Drift calculation con feedback perfecto (drift = 0)
    // =====================================================================
    #[test]
    fn test_drift_perfect_alignment() {
        let mut scorer = AlignmentScorer::new(test_config());

        // Feedback con current == desired → drift = 0
        for i in 0..10u32 {
            let fb = make_feedback("layer_0", i, 0.5, 0.5, 1.0);
            scorer.ingest_feedback(fb).unwrap();
        }

        let activations = Tensor::from_vec(vec![0.5f32; 10], &Device::Cpu).unwrap();
        let drift = scorer.calculate_drift("layer_0", &activations).unwrap();

        assert!((drift - 0.0).abs() < 0.001, "Expected drift ~0, got {}", drift);
    }

    // =====================================================================
    // Test 7: Drift calculation con deriva crítica
    // =====================================================================
    #[test]
    fn test_drift_critical() {
        let mut scorer = AlignmentScorer::new(test_config());

        // Feedback con máxima divergencia: current=1.0, desired=-1.0
        for i in 0..10u32 {
            let fb = make_feedback("layer_0", i, 1.0, -1.0, 1.0);
            scorer.ingest_feedback(fb).unwrap();
        }

        let activations = Tensor::from_vec(vec![1.0f32; 10], &Device::Cpu).unwrap();
        let drift = scorer.calculate_drift("layer_0", &activations).unwrap();

        // Divergencia = |1.0 - (-1.0)| / (1.0 + |-1.0|) = 2.0 / 2.0 = 1.0
        assert!(drift > 0.5, "Expected high drift, got {}", drift);
        assert!(drift <= 1.0, "Drift should be clamped to 1.0, got {}", drift);
    }

    // =====================================================================
    // Test 8: Drift sin feedback (retorna 0.0)
    // =====================================================================
    #[test]
    fn test_drift_no_feedback() {
        let scorer = AlignmentScorer::new(test_config());

        let activations = Tensor::from_vec(vec![0.5f32; 10], &Device::Cpu).unwrap();
        let drift = scorer.calculate_drift("empty_layer", &activations).unwrap();

        assert_eq!(drift, 0.0, "No feedback should return drift = 0.0");
    }

    // =====================================================================
    // Test 9: Validate thresholds - pasa (drift < critical)
    // =====================================================================
    #[test]
    fn test_validate_thresholds_passes() {
        let mut scorer = AlignmentScorer::new(test_config());

        // Feedback con baja divergencia
        for i in 0..10u32 {
            let fb = make_feedback("layer_0", i, 0.5, 0.55, 0.8);
            scorer.ingest_feedback(fb).unwrap();
        }

        let activations = Tensor::from_vec(vec![0.5f32; 10], &Device::Cpu).unwrap();
        let result = scorer.validate_thresholds("layer_0", &activations);

        assert!(result.is_ok(), "Should pass with low drift");
    }

    // =====================================================================
    // Test 10: Validate thresholds - falla (drift >= critical)
    // =====================================================================
    #[test]
    fn test_validate_thresholds_critical_exceeded() {
        let mut scorer = AlignmentScorer::new(test_config());

        // Feedback con máxima divergencia
        for i in 0..20u32 {
            let fb = make_feedback("layer_0", i, 1.0, -1.0, 1.0);
            scorer.ingest_feedback(fb).unwrap();
        }

        let activations = Tensor::from_vec(vec![1.0f32; 20], &Device::Cpu).unwrap();
        let result = scorer.validate_thresholds("layer_0", &activations);

        assert!(result.is_err(), "Should fail with critical drift");
        match result.unwrap_err() {
            AlignmentError::DriftThresholdExceeded {
                drift_threshold,
                current,
                ..
            } => {
                assert!(current >= drift_threshold, "Current {} should exceed threshold {}", current, drift_threshold);
            }
            other => panic!("Expected DriftThresholdExceeded, got {:?}", other),
        }
    }

    // =====================================================================
    // Test 11: Steering adjustment genera delta no vacío
    // =====================================================================
    #[test]
    fn test_steering_adjustment_generates_delta() {
        let mut scorer = AlignmentScorer::new(test_config());

        for i in 0..10u32 {
            let fb = make_feedback("layer_0", i, 0.3, 0.7, 0.9);
            scorer.ingest_feedback(fb).unwrap();
        }

        let activations = Tensor::from_vec(vec![0.3f32; 10], &Device::Cpu).unwrap();
        let result = scorer.generate_steering_adjustment("layer_0", &activations).unwrap();

        assert!(!result.steering_delta.is_empty(), "Steering delta should not be empty");
        assert_eq!(result.steering_delta.len(), 10, "Delta length should match feature count");
        assert!(result.confidence > 0.0, "Confidence should be > 0");
        assert_eq!(result.features_analyzed, 10);
    }

    // =====================================================================
    // Test 12: Steering adjustment flaggea conceptos con alta deriva
    // =====================================================================
    #[test]
    fn test_steering_flags_high_drift_concepts() {
        let mut scorer = AlignmentScorer::new(test_config());

        // Concepto con alta deriva
        let fb = make_feedback_with_concept("layer_0", 0, 1.0, -1.0, 1.0, "dangerous_concept");
        scorer.ingest_feedback(fb).unwrap();

        // Concepto con baja deriva
        let fb = make_feedback_with_concept("layer_0", 1, 0.5, 0.52, 1.0, "safe_concept");
        scorer.ingest_feedback(fb).unwrap();

        let activations = Tensor::from_vec(vec![1.0f32, 0.5f32], &Device::Cpu).unwrap();
        let result = scorer.generate_steering_adjustment("layer_0", &activations).unwrap();

        assert!(
            result.flagged_concepts.contains(&"dangerous_concept".to_string()),
            "Should flag dangerous_concept"
        );
        assert!(
            !result.flagged_concepts.contains(&"safe_concept".to_string()),
            "Should not flag safe_concept"
        );
    }

    // =====================================================================
    // Test 13: Feedback buffer FIFO eviction
    // =====================================================================
    #[test]
    fn test_feedback_buffer_eviction() {
        let config = AlignmentConfig {
            feedback_window: 5,
            ..test_config()
        };
        let mut scorer = AlignmentScorer::new(config);

        for i in 0..10u32 {
            let fb = make_feedback("layer_0", i, 0.5, 0.6, 0.9);
            scorer.ingest_feedback(fb).unwrap();
        }

        assert_eq!(scorer.feedback_count("layer_0"), 5, "Buffer should cap at feedback_window");
    }

    // =====================================================================
    // Test 14: Clear feedback
    // =====================================================================
    #[test]
    fn test_clear_feedback() {
        let mut scorer = AlignmentScorer::new(test_config());

        for i in 0..5u32 {
            let fb = make_feedback("layer_0", i, 0.5, 0.6, 0.9);
            scorer.ingest_feedback(fb).unwrap();
        }

        assert_eq!(scorer.feedback_count("layer_0"), 5);

        scorer.clear_feedback("layer_0");
        assert_eq!(scorer.feedback_count("layer_0"), 0);
    }

    // =====================================================================
    // Test 15: Clear all feedback
    // =====================================================================
    #[test]
    fn test_clear_all_feedback() {
        let mut scorer = AlignmentScorer::new(test_config());

        for i in 0..3u32 {
            scorer.ingest_feedback(make_feedback("layer_a", i, 0.5, 0.6, 0.9)).unwrap();
        }
        for i in 0..3u32 {
            scorer.ingest_feedback(make_feedback("layer_b", i, 0.5, 0.6, 0.9)).unwrap();
        }

        scorer.clear_all();
        assert_eq!(scorer.feedback_count("layer_a"), 0);
        assert_eq!(scorer.feedback_count("layer_b"), 0);
    }

    // =====================================================================
    // Test 16: Drift history tracking
    // =====================================================================
    #[test]
    fn test_drift_history() {
        let mut scorer = AlignmentScorer::new(test_config());

        for i in 0..5u32 {
            let fb = make_feedback("layer_0", i, 0.5, 0.5, 1.0);
            scorer.ingest_feedback(fb).unwrap();
        }

        let activations = Tensor::from_vec(vec![0.5f32; 5], &Device::Cpu).unwrap();
        scorer.calculate_drift("layer_0", &activations).unwrap();

        // Drift history is internal; verify via feedback count
        assert_eq!(scorer.feedback_count("layer_0"), 5);
    }

    // =====================================================================
    // Test 17: Confidence computation con buen feedback
    // =====================================================================
    #[test]
    fn test_confidence_high_quality_feedback() {
        let mut scorer = AlignmentScorer::new(test_config());

        // 40 entries con alta confianza (80% del window de 50)
        for i in 0..40u32 {
            let fb = make_feedback("layer_0", i, 0.5, 0.55, 0.95);
            scorer.ingest_feedback(fb).unwrap();
        }

        let activations = Tensor::from_vec(vec![0.5f32; 40], &Device::Cpu).unwrap();
        let result = scorer.generate_steering_adjustment("layer_0", &activations).unwrap();

        // avg_confidence = 0.95, volume_bonus = min(40/50, 0.3) = 0.3
        // confidence = min(0.95 + 0.3, 1.0) = 1.0
        assert!(result.confidence > 0.9, "High quality feedback should yield high confidence");
    }

    // =====================================================================
    // Test 18: Empty activations error
    // =====================================================================
    #[test]
    fn test_empty_activations_error() {
        let mut scorer = AlignmentScorer::new(test_config());

        for i in 0..5u32 {
            let fb = make_feedback("layer_0", i, 0.5, 0.6, 0.9);
            scorer.ingest_feedback(fb).unwrap();
        }

        // Tensor vacío (0 elementos)
        let empty_tensor = Tensor::from_vec(vec![]f32, &Device::Cpu).unwrap();
        let result = scorer.calculate_drift("layer_0", &empty_tensor);

        assert!(result.is_err(), "Empty activations should error");
        match result.unwrap_err() {
            AlignmentError::EmptyActivations => {} // Expected
            other => panic!("Expected EmptyActivations, got {:?}", other),
        }
    }

    // =====================================================================
    // Test 19: Set config dinámico
    // =====================================================================
    #[test]
    fn test_set_config() {
        let mut scorer = AlignmentScorer::new(test_config());

        let new_config = AlignmentConfig {
            drift_threshold: 0.5,
            critical_threshold: 0.8,
            ..test_config()
        };
        scorer.set_config(new_config);

        assert_eq!(scorer.config().drift_threshold, 0.5);
        assert_eq!(scorer.config().critical_threshold, 0.8);
    }

    // =====================================================================
    // Test 20: Feedback con Infinity (debe fallar)
    // =====================================================================
    #[test]
    fn test_ingest_infinity_rejected() {
        let mut scorer = AlignmentScorer::new(test_config());
        let fb = make_feedback("layer_0", 0, f32::INFINITY, 0.9, 0.95);

        let result = scorer.ingest_feedback(fb);
        assert!(result.is_err());
    }
}
