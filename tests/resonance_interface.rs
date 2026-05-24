//! Resonance Interface Integration Tests — Sprint 46
//!
//! Tests the complete biometric → homeostasis → resonance pipeline
//! for the Resonance Interface (Pillar 4).

#[cfg(feature = "v3.0-resonance-interface")]
mod biometric_analysis_tests {
    use ed2kIA::pillars::resonance::biometric_analyzer::{
        LocalBiometricAnalyzer, BiometricState, AnalyzerConfig,
    };

    fn make_rppg_samples(count: usize) -> Vec<f32> {
        (0..count).map(|i| (i as f32 * 0.01).sin() * 0.5 + 0.5).collect()
    }

    fn make_voice_samples(count: usize) -> Vec<f32> {
        (0..count).map(|i| (i as f32 * 0.02).sin() * 0.3 + 0.5).collect()
    }

    fn make_expression_samples(count: usize) -> Vec<f32> {
        (0..count).map(|_| 0.5).collect()
    }

    #[test]
    fn test_analyzer_creation() {
        let analyzer = LocalBiometricAnalyzer::new();
        // Should create without error
    }

    #[test]
    fn test_analyzer_custom_config() {
        let config = AnalyzerConfig {
            min_rppg_samples: 512,
            min_voice_samples: 256,
            min_expression_samples: 32,
            ..Default::default()
        };
        let _analyzer = LocalBiometricAnalyzer::with_config(config);
    }

    #[test]
    fn test_analyze_stream_full() {
        let mut analyzer = LocalBiometricAnalyzer::new();
        let rppg = make_rppg_samples(256);
        let voice = make_voice_samples(128);
        let expr = make_expression_samples(16);

        let state = analyzer.analyze_stream(&rppg, &voice, &expr).unwrap();
        assert!(state.stress_index >= 0.0 && state.stress_index <= 1.0);
        assert!(state.coherence >= 0.0 && state.coherence <= 1.0);
        assert!(state.dominant_frequency >= 0.0);
        assert!(state.valence >= -1.0 && state.valence <= 1.0);
        assert!(state.arousal >= 0.0 && state.arousal <= 1.0);
    }

    #[test]
    fn test_analyze_stream_too_short_rppg() {
        let mut analyzer = LocalBiometricAnalyzer::new();
        let rppg = make_rppg_samples(10); // Too short
        let voice = make_voice_samples(128);
        let expr = make_expression_samples(16);

        let result = analyzer.analyze_stream(&rppg, &voice, &expr);
        assert!(result.is_err());
    }

    #[test]
    fn test_biometric_state_validation() {
        // Valid state
        let state = BiometricState::new(0.5, 0.7, 1.5, 0.3, 0.6).unwrap();
        assert_eq!(state.stress_index, 0.5);

        // Invalid stress_index
        match BiometricState::new(1.5, 0.7, 1.5, 0.3, 0.6) {
            Err(_) => {} // Expected
            Ok(_) => panic!("Should reject stress_index > 1.0"),
        }
    }

    #[test]
    fn test_homeostasis_score_calculation() {
        let calm = BiometricState::new(0.1, 0.9, 1.0, 0.0, 0.5).unwrap();
        let stressed = BiometricState::new(0.9, 0.1, 3.0, 0.8, 0.9).unwrap();

        let calm_score = calm.homeostasis_score();
        let stressed_score = stressed.homeostasis_score();

        assert!(calm_score > stressed_score, "Calm state should have higher homeostasis score");
    }

    #[test]
    fn test_clear_buffers() {
        let mut analyzer = LocalBiometricAnalyzer::new();
        analyzer.clear_buffers();
        // Should not panic
    }
}

#[cfg(feature = "v3.0-resonance-interface")]
mod homeostasis_engine_tests {
    use ed2kIA::pillars::resonance::biometric_analyzer::BiometricState;
    use ed2kIA::pillars::resonance::homeostasis_engine::{
        HomeostasisEngine, HomeostasisConfig, EngineError,
    };

    fn make_calm_state() -> BiometricState {
        BiometricState::new(0.1, 0.9, 1.0, 0.5, 0.3).unwrap()
    }

    fn make_stressed_state() -> BiometricState {
        BiometricState::new(0.8, 0.2, 2.5, -0.3, 0.9).unwrap()
    }

    #[test]
    fn test_engine_creation() {
        let engine = HomeostasisEngine::new().unwrap();
        assert!(engine.get_baseline().is_none());
    }

    #[test]
    fn test_calibrate_and_deviation() {
        let mut engine = HomeostasisEngine::new().unwrap();
        let calm = make_calm_state();
        engine.calibrate_baseline(&calm);

        let delta = engine.calculate_deviation(&calm).unwrap();
        assert!((delta.stress_delta - 0.0).abs() < f32::EPSILON);
        assert!(delta.is_ethically_approved());
    }

    #[test]
    fn test_deviation_stressed_from_calm() {
        let mut engine = HomeostasisEngine::new().unwrap();
        let calm = make_calm_state();
        engine.calibrate_baseline(&calm);

        let stressed = make_stressed_state();
        let delta = engine.calculate_deviation(&stressed).unwrap();
        assert!(delta.stress_delta > 0.0, "Stress should increase");
        assert!(delta.coherence_delta < 0.0, "Coherence should decrease");
    }

    #[test]
    fn test_sct_guard_rejection() {
        let mut engine = HomeostasisEngine::new().unwrap();
        engine.config.sct_z_threshold = 1.0; // Impossible threshold

        let state = make_calm_state();
        engine.calibrate_baseline(&state);

        let stressed = make_stressed_state();
        match engine.calculate_deviation(&stressed) {
            Err(EngineError::EthicalRejection { z }) => assert!(z < 1.0),
            _ => panic!("Expected EthicalRejection"),
        }
    }

    #[test]
    fn test_adapt_baseline() {
        let mut engine = HomeostasisEngine::new().unwrap();
        let calm = make_calm_state();
        engine.calibrate_baseline(&calm);

        let slightly_different = BiometricState::new(0.3, 0.7, 1.5, 0.3, 0.5).unwrap();
        engine.adapt_baseline(&slightly_different).unwrap();

        let adapted = engine.get_baseline().unwrap();
        assert!(adapted.stress_index > calm.stress_index);
    }

    #[test]
    fn test_needs_recalibration() {
        let mut engine = HomeostasisEngine::new().unwrap();
        let calm = make_calm_state();
        engine.calibrate_baseline(&calm);

        let very_different = BiometricState::new(0.9, 0.1, 4.0, -0.9, 0.95).unwrap();
        assert!(engine.needs_recalibration(&very_different));
    }

    #[test]
    fn test_invalid_config() {
        let config = HomeostasisConfig {
            target_coherence: 1.5,
            ..Default::default()
        };
        match HomeostasisEngine::with_config(config) {
            Err(EngineError::InvalidTargetCoherence { .. }) => {}
            _ => panic!("Expected InvalidTargetCoherence"),
        }
    }

    #[test]
    fn test_reset() {
        let mut engine = HomeostasisEngine::new().unwrap();
        engine.calibrate_baseline(&make_calm_state());
        engine.reset();
        assert!(engine.get_baseline().is_none());
    }
}

#[cfg(feature = "v3.0-resonance-interface")]
mod resonance_generator_tests {
    use ed2kIA::pillars::resonance::biometric_analyzer::BiometricState;
    use ed2kIA::pillars::resonance::homeostasis_engine::HomeostasisDelta;
    use ed2kIA::pillars::resonance::resonance_generator::{
        ResonanceGenerator, ResonanceConfig, ResonanceError, BinauralBeat, IsochronicTone, SemanticResponse,
    };

    fn make_calm_state() -> BiometricState {
        BiometricState::new(0.1, 0.9, 1.0, 0.5, 0.3).unwrap()
    }

    fn make_positive_delta() -> HomeostasisDelta {
        HomeostasisDelta {
            stress_delta: -0.2,
            coherence_delta: 0.3,
            frequency_delta: 0.1,
            valence_delta: 0.4,
            arousal_delta: -0.1,
            homeostasis_score: 0.8,
            sct_z: 0.5,
            correction_magnitude: 0.2,
        }
    }

    #[test]
    fn test_generator_creation() {
        let gen = ResonanceGenerator::new();
        assert!(gen.config().enable_binaural);
    }

    #[test]
    fn test_binaural_beat_creation() {
        let beat = BinauralBeat::new(200.0, 10.0, 60.0, 0.7).unwrap();
        assert_eq!(beat.left_freq_hz, 200.0);
        assert_eq!(beat.right_freq_hz, 190.0);
        assert_eq!(beat.brainwave_band(), "alpha");
    }

    #[test]
    fn test_isochronic_tone_creation() {
        let tone = IsochronicTone::new(200.0, 10.0, 60.0, 0.7).unwrap();
        assert_eq!(tone.base_freq_hz, 200.0);
        assert_eq!(tone.brainwave_band(), "alpha");
    }

    #[test]
    fn test_semantic_response_creation() {
        let resp = SemanticResponse::new(
            "Respuesta constructiva de resonancia.".to_string(),
            0.5,
            "alpha".to_string(),
            0.8,
        ).unwrap();
        assert_eq!(resp.sct_z, 0.5);
    }

    #[test]
    fn test_semantic_prohibited_word_rejection() {
        match SemanticResponse::new(
            "Texto con guerra prohibida.".to_string(),
            0.5,
            "alpha".to_string(),
            0.8,
        ) {
            Err(ResonanceError::SctRejection { z }) => assert_eq!(z, -1.0),
            _ => panic!("Expected SctRejection for prohibited word"),
        }
    }

    #[test]
    fn test_generate_response_valid() {
        let gen = ResonanceGenerator::new();
        let delta = make_positive_delta();
        let state = make_calm_state();

        let response = gen.generate_response(&delta, &state).unwrap();
        assert!(response.is_approved());
        assert!(response.binaural.is_some());
        assert!(response.isochronic.is_some());
    }

    #[test]
    fn test_generate_response_sct_rejected() {
        let gen = ResonanceGenerator::new();
        let delta = HomeostasisDelta {
            stress_delta: 0.5,
            coherence_delta: -0.4,
            frequency_delta: 0.3,
            valence_delta: -0.3,
            arousal_delta: 0.2,
            homeostasis_score: 0.2,
            sct_z: -0.3,
            correction_magnitude: 0.7,
        };
        let state = make_calm_state();

        match gen.generate_response(&delta, &state) {
            Err(ResonanceError::SctRejection { .. }) => {}
            _ => panic!("Expected SctRejection"),
        }
    }

    #[test]
    fn test_brainwave_band_selection() {
        let gen = ResonanceGenerator::new();
        let delta = make_positive_delta();

        // High stress → alpha
        let high_stress = BiometricState::new(0.7, 0.3, 2.0, -0.2, 0.8).unwrap();
        let band = gen.select_brainwave_band(&delta, &high_stress);
        assert_eq!(band, "alpha");

        // Low arousal → beta
        let low_arousal = BiometricState::new(0.3, 0.6, 1.0, 0.2, 0.1).unwrap();
        let band = gen.select_brainwave_band(&delta, &low_arousal);
        assert_eq!(band, "beta");
    }

    #[test]
    fn test_invalid_frequency_rejection() {
        match BinauralBeat::new(10.0, 10.0, 60.0, 0.7) {
            Err(ResonanceError::InvalidFrequency { .. }) => {}
            _ => panic!("Expected InvalidFrequency"),
        }
    }
}

#[cfg(feature = "v3.0-resonance-interface")]
mod pillar_integration_tests {
    use ed2kIA::pillars::resonance::{
        ResonanceEngine, ResonancePillarError,
        biometric_analyzer::BiometricState,
    };
    use ed2kIA::pillars::PillarInterface;

    fn make_calm_state() -> BiometricState {
        BiometricState::new(0.1, 0.9, 1.0, 0.5, 0.3).unwrap()
    }

    fn make_rppg_samples(count: usize) -> Vec<f32> {
        (0..count).map(|i| (i as f32 * 0.01).sin() * 0.5 + 0.5).collect()
    }

    fn make_voice_samples(count: usize) -> Vec<f32> {
        (0..count).map(|i| (i as f32 * 0.02).sin() * 0.3 + 0.5).collect()
    }

    fn make_expression_samples(count: usize) -> Vec<f32> {
        (0..count).map(|_| 0.5).collect()
    }

    #[test]
    fn test_pillar_interface() {
        use ed2kIA::orchestration::PillarId;
        assert_eq!(ResonanceEngine::id(), PillarId::ResonanceInterface);
    }

    #[test]
    fn test_local_constraint() {
        let engine = ResonanceEngine::new();
        if cfg!(target_arch = "wasm32") {
            assert!(engine.validate_local_constraint());
        } else {
            assert!(!engine.validate_local_constraint());
        }
    }

    #[test]
    fn test_consume_ce_valid() {
        let engine = ResonanceEngine::new();
        assert!(engine.consume_ce(1.0).is_ok());
    }

    #[test]
    fn test_consume_ce_zero_rejected() {
        let engine = ResonanceEngine::new();
        match engine.consume_ce(0.0) {
            Err(_) => {} // Expected
            _ => panic!("Expected error for zero CE"),
        }
    }

    #[test]
    fn test_process_stream_not_calibrated() {
        let mut engine = ResonanceEngine::new();
        let rppg = make_rppg_samples(256);
        let voice = make_voice_samples(128);
        let expr = make_expression_samples(16);

        match engine.process_stream(&rppg, &voice, &expr) {
            Err(ResonancePillarError::NotCalibrated) => {}
            _ => panic!("Expected NotCalibrated"),
        }
    }

    #[test]
    fn test_process_stream_insufficient_ce() {
        let mut engine = ResonanceEngine::new();
        let state = make_calm_state();
        engine.calibrate(&state);

        let rppg = make_rppg_samples(256);
        let voice = make_voice_samples(128);
        let expr = make_expression_samples(16);

        match engine.process_stream(&rppg, &voice, &expr) {
            Err(ResonancePillarError::InsufficientCE { .. }) => {}
            _ => panic!("Expected InsufficientCE"),
        }
    }

    #[test]
    fn test_calibrate_and_deposit_ce() {
        let mut engine = ResonanceEngine::new();
        let state = make_calm_state();
        engine.calibrate(&state);
        engine.deposit_ce(10.0);
        assert_eq!(engine.ce_balance(), 10.0);
    }

    #[test]
    fn test_needs_recalibration() {
        let mut engine = ResonanceEngine::new();
        let calm = make_calm_state();
        engine.calibrate(&calm);

        let very_different = BiometricState::new(0.9, 0.1, 4.0, -0.9, 0.95).unwrap();
        assert!(engine.needs_recalibration(&very_different));
    }

    #[test]
    fn test_clear_buffers() {
        let mut engine = ResonanceEngine::new();
        engine.clear_buffers();
        // Should not panic
    }

    #[test]
    fn test_default() {
        let engine = ResonanceEngine::default();
        assert_eq!(engine.ce_balance(), 0.0);
    }
}
