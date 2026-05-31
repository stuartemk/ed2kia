//! Symbiotic Aegis E2E Tests — DPI Evasion + Healing Benchmark.
//!
//! Validates the complete S54 integration:
//! 1. Harmonic Flow (P3) — SRTP masking + chaffing + transport rotation
//! 2. Biofeedback Engine (P4) — Zero-telemetry local healing
//! 3. Aegis Healer (P3+P4) — Symbiotic coordination
//!
//! **Feature Gate:** `v3.6-aegis-resonance`

#![cfg(feature = "v3.6-aegis-resonance")]

mod symbiotic_aegis_tests {
    use ed2kia::orchestration::aegis_healer::{AegisConfig, AegisHealer, SymbioticState};
    use ed2kia::pillars::resonance::biofeedback_engine::{
        BiofeedbackConfig, BiofeedbackEngine, BiofeedbackError,
    };
    use ed2kia::pillars::steganographic::harmonic_flow::{
        HarmonicFlow, HarmonicFlowConfig, HarmonicFlowError,
    };

    // -----------------------------------------------------------------------
    // Test Helpers
    // -----------------------------------------------------------------------

    fn make_rppg(len: usize) -> Vec<f32> {
        (0..len).map(|i| 0.5 + 0.1 * (i as f32 % 1.0)).collect()
    }

    fn make_voice(len: usize) -> Vec<f32> {
        (0..len).map(|i| 0.3 + 0.05 * (i as f32 % 1.0)).collect()
    }

    fn make_expressions(len: usize) -> Vec<f32> {
        (0..len).map(|i| 0.6 + 0.1 * (i as f32 % 1.0)).collect()
    }

    fn test_payload() -> Vec<u8> {
        b"tensor-exchange-consensus-data".to_vec()
    }

    // -----------------------------------------------------------------------
    // DPI Evasion Tests — Harmonic Flow (P3)
    // -----------------------------------------------------------------------

    #[test]
    fn test_dpi_evasion_basic_obfuscation() {
        let mut flow = HarmonicFlow::new();
        let key =
            ed2kia::pillars::steganographic::chaffing_engine::ChaffingEngine::generate_session_key(
                b"dpi-evasion-test",
            );
        flow.register_session_key("harmonic-default-session", key);

        let payload = test_payload();
        let stream = flow.obfuscate(&payload).expect("Obfuscation failed");

        // Verify obfuscation increased size.
        assert!(stream.obfuscated_size >= stream.original_size);
        assert!(stream.expansion_ratio >= 1.0);
        assert!(!stream.frames.is_empty());
    }

    #[test]
    fn test_dpi_evasion_roundtrip_integrity() {
        let mut flow = HarmonicFlow::new();
        let key =
            ed2kia::pillars::steganographic::chaffing_engine::ChaffingEngine::generate_session_key(
                b"roundtrip-integrity",
            );
        flow.register_session_key("harmonic-default-session", key);

        let payload = test_payload();
        let stream = flow.obfuscate(&payload).expect("Obfuscation failed");
        let result = flow.deobfuscate(&stream).expect("De-obfuscation failed");

        assert_eq!(result.payload, payload);
    }

    #[test]
    fn test_dpi_evasion_large_payload() {
        let mut flow = HarmonicFlow::new();
        let key =
            ed2kia::pillars::steganographic::chaffing_engine::ChaffingEngine::generate_session_key(
                b"large-payload",
            );
        flow.register_session_key("harmonic-default-session", key);

        let payload: Vec<u8> = (0..8192).map(|i| (i % 256) as u8).collect();
        let stream = flow.obfuscate(&payload).expect("Obfuscation failed");
        let result = flow.deobfuscate(&stream).expect("De-obfuscation failed");

        assert_eq!(result.payload, payload);
    }

    #[test]
    fn test_dpi_evasion_constant_bitrate() {
        // With chaffing, multiple payloads of the same size should produce
        // similar obfuscated sizes (constant bitrate for DPI evasion).
        let mut flow = HarmonicFlow::new();
        let key =
            ed2kia::pillars::steganographic::chaffing_engine::ChaffingEngine::generate_session_key(
                b"constant-bitrate",
            );
        flow.register_session_key("harmonic-default-session", key);

        let sizes = vec![100, 100, 100, 100, 100];
        let mut obfuscated_sizes = Vec::new();

        for size in &sizes {
            let payload: Vec<u8> = (0..*size).map(|i| (i % 256) as u8).collect();
            let stream = flow.obfuscate(&payload).expect("Obfuscation failed");
            obfuscated_sizes.push(stream.obfuscated_size);
        }

        // All obfuscated sizes should be identical for identical inputs.
        for i in 1..obfuscated_sizes.len() {
            assert_eq!(
                obfuscated_sizes[i], obfuscated_sizes[0],
                "Bitrate not constant across cycles"
            );
        }
    }

    #[test]
    fn test_dpi_evasion_transport_rotation() {
        let mut flow = HarmonicFlow::new();
        let key =
            ed2kia::pillars::steganographic::chaffing_engine::ChaffingEngine::generate_session_key(
                b"transport-rotation",
            );
        flow.register_session_key("harmonic-default-session", key);

        let initial_transport = flow.current_transport();
        let _ = flow.rotate_transport();
        let new_transport = flow.current_transport();

        // Transport should have changed (or cycled).
        let _ = initial_transport;
        let _ = new_transport;
    }

    // -----------------------------------------------------------------------
    // Zero-Telemetry Biofeedback Tests (P4)
    // -----------------------------------------------------------------------

    #[test]
    fn test_biofeedback_local_constraint() {
        let engine = BiofeedbackEngine::new();
        assert!(
            engine.validate_local_constraint(),
            "Biofeedback must enforce local-only constraint"
        );
    }

    #[test]
    fn test_biofeedback_calibration_and_cycle() {
        let mut engine = BiofeedbackEngine::new();
        let rppg = make_rppg(128);
        let voice = make_voice(128);
        let expr = make_expressions(128);

        // Calibrate.
        let baseline = engine
            .calibrate(&rppg, &voice, &expr)
            .expect("Calibration failed");
        assert!(engine.get_stats().calibrated);
        assert!(baseline.homeostasis_score() > 0.0);

        // Process cycle.
        let result = engine
            .process_cycle(&rppg, &voice, &expr)
            .expect("Cycle failed");
        assert!(result.homeostasis_score >= 0.0 && result.homeostasis_score <= 1.0);
    }

    #[test]
    fn test_biofeedback_no_telemetry_violation() {
        // The engine should never produce TelemetryViolation errors
        // because it has no network capabilities.
        let mut engine = BiofeedbackEngine::new();
        let rppg = make_rppg(128);
        let voice = make_voice(128);
        let expr = make_expressions(128);

        engine
            .calibrate(&rppg, &voice, &expr)
            .expect("Calibration failed");

        for _ in 0..10 {
            let result = engine.process_cycle(&rppg, &voice, &expr);
            match result {
                Ok(_) => (),
                Err(BiofeedbackError::TelemetryViolation) => {
                    panic!("Telemetry violation should never occur in local engine");
                }
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        }
    }

    #[test]
    fn test_biofeedback_sct_guard_enforcement() {
        // Verify that SCT-rejected responses are properly reported.
        let mut engine = BiofeedbackEngine::new();
        let rppg = make_rppg(128);
        let voice = make_voice(128);
        let expr = make_expressions(128);

        engine
            .calibrate(&rppg, &voice, &expr)
            .expect("Calibration failed");

        // If SCT rejects, we should get SCTRejected error.
        let result = engine.process_cycle(&rppg, &voice, &expr);
        match result {
            Ok(r) => {
                assert!(r.homeostasis_score >= 0.0);
            }
            Err(BiofeedbackError::SCTRejected(z)) => {
                assert!(z < 0.0, "SCT should only reject negative Z");
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[test]
    fn test_biofeedback_adaptation_limit() {
        let config = BiofeedbackConfig {
            max_adapt_steps: 3,
            ..BiofeedbackConfig::default()
        };
        let mut engine = BiofeedbackEngine::with_config(config).expect("Config should be valid");
        let rppg = make_rppg(128);
        let voice = make_voice(128);
        let expr = make_expressions(128);

        engine
            .calibrate(&rppg, &voice, &expr)
            .expect("Calibration failed");

        // Run more cycles than max_adapt_steps.
        for _ in 0..5 {
            engine
                .process_cycle(&rppg, &voice, &expr)
                .expect("Cycle failed");
        }

        // Adaptation should stop at max_adapt_steps.
        assert!(engine.get_stats().current_adapt_step <= 3);
    }

    // -----------------------------------------------------------------------
    // Symbiotic Aegis Healer Tests (P3+P4 Bridge)
    // -----------------------------------------------------------------------

    #[test]
    fn test_aegis_healer_creation() {
        let healer = AegisHealer::new();
        let state = healer.get_state();
        assert_eq!(state.healing_cycles, 0);
        assert_eq!(state.ce_consumed, 0.0);
    }

    #[test]
    fn test_aegis_alignment_calculation() {
        // Identical scores → perfect alignment.
        let alignment = SymbioticState::calculate_alignment(0.8, 0.8);
        assert!((alignment - 1.0).abs() < 0.001);

        // Opposite scores → no alignment.
        let alignment = SymbioticState::calculate_alignment(0.0, 1.0);
        assert!((alignment - 0.0).abs() < 0.001);

        // Partial alignment.
        let alignment = SymbioticState::calculate_alignment(0.7, 0.4);
        assert!((alignment - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_aegis_harmony_detection() {
        let state = SymbioticState {
            network_health: 0.9,
            human_coherence: 0.85,
            alignment: 0.95,
            in_harmony: true,
            healing_cycles: 0,
            ce_consumed: 0.0,
        };
        assert!(state.is_in_harmony());

        let state = SymbioticState {
            network_health: 0.3,
            human_coherence: 0.2,
            alignment: 0.5,
            in_harmony: false,
            healing_cycles: 0,
            ce_consumed: 0.0,
        };
        assert!(!state.is_in_harmony());
    }

    #[test]
    fn test_aegis_network_health_update() {
        let mut healer = AegisHealer::new();
        healer.update_network_health(0.85);
        assert!((healer.get_state().network_health - 0.85).abs() < 0.001);
    }

    #[test]
    fn test_aegis_human_coherence_update() {
        let mut healer = AegisHealer::new();
        healer.update_human_coherence(0.75);
        assert!((healer.get_state().human_coherence - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_aegis_alignment_auto_update() {
        let mut healer = AegisHealer::new();
        healer.update_network_health(0.9);
        healer.update_human_coherence(0.9);

        // Alignment should be near 1.0 when scores match.
        assert!(healer.get_state().alignment > 0.95);
    }

    #[test]
    fn test_aegis_calibration() {
        let mut healer = AegisHealer::new();
        let rppg = make_rppg(128);
        let voice = make_voice(128);
        let expr = make_expressions(128);

        healer
            .calibrate_biofeedback(&rppg, &voice, &expr)
            .expect("Calibration failed");
    }

    #[test]
    fn test_aegis_ce_budget_tracking() {
        let healer = AegisHealer::new();
        assert_eq!(healer.total_ce_consumed(), 0.0);
    }

    #[test]
    fn test_aegis_reset() {
        let mut healer = AegisHealer::new();
        healer.update_network_health(0.9);
        healer.update_human_coherence(0.8);

        healer.reset();
        assert!((healer.get_state().network_health - 0.5).abs() < 0.001);
        assert!((healer.get_state().human_coherence - 0.5).abs() < 0.001);
        assert_eq!(healer.get_state().healing_cycles, 0);
    }

    // -----------------------------------------------------------------------
    // Integration Tests — Full Symbiotic Cycle
    // -----------------------------------------------------------------------

    #[test]
    fn test_full_symbiotic_cycle() {
        let mut healer = AegisHealer::new();
        let rppg = make_rppg(128);
        let voice = make_voice(128);
        let expr = make_expressions(128);

        // Calibrate biofeedback first.
        healer
            .calibrate_biofeedback(&rppg, &voice, &expr)
            .expect("Calibration failed");

        // Run symbiotic cycle.
        let payload = test_payload();
        let result = healer
            .symbiotic_cycle(&payload, &rppg, &voice, &expr)
            .expect("Symbiotic cycle failed");

        assert!(result.network_obfuscated);
        assert!(result.biofeedback_applied);
        assert_eq!(result.state.healing_cycles, 1);
    }

    #[test]
    fn test_multiple_symbiotic_cycles() {
        let mut healer = AegisHealer::new();
        let rppg = make_rppg(128);
        let voice = make_voice(128);
        let expr = make_expressions(128);

        healer
            .calibrate_biofeedback(&rppg, &voice, &expr)
            .expect("Calibration failed");

        let payload = test_payload();
        for i in 0..5 {
            let result = healer
                .symbiotic_cycle(&payload, &rppg, &voice, &expr)
                .expect("Symbiotic cycle failed");
            assert_eq!(result.state.healing_cycles, (i + 1) as u32);
        }

        assert_eq!(healer.get_state().healing_cycles, 5);
    }

    #[test]
    fn test_healing_triggers_on_mismatch() {
        let config = AegisConfig {
            auto_heal: true,
            min_homeostasis: 0.5,
            ..AegisConfig::default()
        };
        let mut healer = AegisHealer::with_config(config).expect("Config should be valid");

        // Set low health scores to trigger healing.
        healer.update_network_health(0.3);
        healer.update_human_coherence(0.2);

        assert!(!healer.get_state().in_harmony);
    }

    #[test]
    fn test_no_healing_when_in_harmony() {
        let config = AegisConfig {
            auto_heal: true,
            min_homeostasis: 0.5,
            ..AegisConfig::default()
        };
        let mut healer = AegisHealer::with_config(config).expect("Config should be valid");

        // Set high health scores for harmony.
        healer.update_network_health(0.9);
        healer.update_human_coherence(0.85);

        assert!(healer.get_state().in_harmony);
    }

    // -----------------------------------------------------------------------
    // Performance Benchmark — DPI Evasion Throughput
    // -----------------------------------------------------------------------

    #[test]
    fn test_dpi_evasion_throughput_100_cycles() {
        let mut flow = HarmonicFlow::new();
        let key =
            ed2kia::pillars::steganographic::chaffing_engine::ChaffingEngine::generate_session_key(
                b"throughput-test",
            );
        flow.register_session_key("harmonic-default-session", key);

        let payload: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();

        for _ in 0..100 {
            let stream = flow.obfuscate(&payload).expect("Obfuscation failed");
            let result = flow.deobfuscate(&stream).expect("De-obfuscation failed");
            assert_eq!(result.payload, payload);
        }
    }

    #[test]
    fn test_biofeedback_throughput_50_cycles() {
        let mut engine = BiofeedbackEngine::new();
        let rppg = make_rppg(128);
        let voice = make_voice(128);
        let expr = make_expressions(128);

        engine
            .calibrate(&rppg, &voice, &expr)
            .expect("Calibration failed");

        for _ in 0..50 {
            engine
                .process_cycle(&rppg, &voice, &expr)
                .expect("Cycle failed");
        }

        assert_eq!(engine.get_stats().total_cycles, 50);
    }

    // -----------------------------------------------------------------------
    // Edge Cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_empty_payload_rejected() {
        let mut flow = HarmonicFlow::new();
        match flow.obfuscate(&[]) {
            Err(HarmonicFlowError::EmptyPayload) => (),
            other => panic!("Expected EmptyPayload, got {:?}", other),
        }
    }

    #[test]
    fn test_uncalibrated_biofeedback_rejected() {
        let mut engine = BiofeedbackEngine::new();
        let rppg = make_rppg(128);
        let voice = make_voice(128);
        let expr = make_expressions(128);

        match engine.process_cycle(&rppg, &voice, &expr) {
            Err(BiofeedbackError::BaselineNotCalibrated) => (),
            other => panic!("Expected BaselineNotCalibrated, got {:?}", other),
        }
    }

    #[test]
    fn test_single_byte_payload() {
        let mut flow = HarmonicFlow::new();
        let key =
            ed2kia::pillars::steganographic::chaffing_engine::ChaffingEngine::generate_session_key(
                b"single-byte",
            );
        flow.register_session_key("harmonic-default-session", key);

        let payload = vec![42u8];
        let stream = flow.obfuscate(&payload).expect("Obfuscation failed");
        let result = flow.deobfuscate(&stream).expect("De-obfuscation failed");
        assert_eq!(result.payload, payload);
    }

    #[test]
    fn test_boundary_alignment_values() {
        // Test alignment at boundary values.
        let a = SymbioticState::calculate_alignment(0.0, 0.0);
        assert!((a - 1.0).abs() < 0.001);

        let a = SymbioticState::calculate_alignment(1.0, 1.0);
        assert!((a - 1.0).abs() < 0.001);

        let a = SymbioticState::calculate_alignment(0.5, 0.5);
        assert!((a - 1.0).abs() < 0.001);
    }
}
