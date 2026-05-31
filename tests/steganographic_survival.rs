//! Steganographic Survival Integration Tests — Sprint 45
//!
//! Validates the complete Pillar 3 implementation: Traffic Masking,
//! Chaffing & Winnowing, and Transport Rotation.

#[cfg(all(feature = "v3.0-steganographic-survival",))]
mod traffic_masking_tests {
    use ed2kia::pillars::steganographic::{MaskerConfig, TrafficMasker};

    #[test]
    fn test_srtp_masking_integrity() {
        let mut masker = TrafficMasker::new();
        let payload = b"ed2kIA cooperative preservation tensor data";
        let frames = masker.mask_payload(payload).unwrap();

        // Verify frames were created
        assert!(!frames.is_empty());

        // Verify each frame can be unmasked
        for frame in &frames {
            let (extracted, _total, _idx) = masker.unmask_frame(frame).unwrap();
            assert!(!extracted.is_empty());
        }

        // Verify full roundtrip
        let recovered = masker.unmask_payload(&frames).unwrap();
        assert_eq!(recovered, payload);
    }

    #[test]
    fn test_masking_fragmentation() {
        let config = MaskerConfig {
            max_payload_size: 64,
            ..MaskerConfig::default()
        };
        let mut masker = TrafficMasker::with_config(config);
        let payload = vec![0xAB; 300];
        let frames = masker.mask_payload(&payload).unwrap();

        // 300 / 64 = 5 frames
        assert_eq!(frames.len(), 5);

        let recovered = masker.unmask_payload(&frames).unwrap();
        assert_eq!(recovered, payload);
    }

    #[test]
    fn test_masking_corruption_detection() {
        let mut masker = TrafficMasker::new();
        let payload = b"integrity test data";
        let mut frames = masker.mask_payload(payload).unwrap();

        // Corrupt a byte
        frames[0][10] ^= 0xFF;

        match masker.unmask_frame(&frames[0]) {
            Err(e) => assert!(e.to_string().contains("Checksum")),
            Ok(_) => panic!("Should have detected corruption"),
        }
    }

    #[test]
    fn test_masking_sequence_continuity() {
        let mut masker = TrafficMasker::new();
        let frames1 = masker.mask_payload(b"first message").unwrap();
        let frames2 = masker.mask_payload(b"second message").unwrap();

        // Both should succeed
        assert!(!frames1.is_empty());
        assert!(!frames2.is_empty());

        // Frame counter should increment
        assert!(masker.frame_count() >= frames1.len() + frames2.len());
    }
}

#[cfg(all(feature = "v3.0-steganographic-survival",))]
mod chaffing_winnowing_tests {
    use ed2kia::pillars::steganographic::{ChaffConfig, ChaffingEngine};

    fn setup_engine() -> ChaffingEngine {
        let mut engine = ChaffingEngine::new();
        let key = ChaffingEngine::generate_session_key(b"integration-test");
        engine.register_session_key("test-session".to_string(), key);
        engine
    }

    #[test]
    fn test_chaffing_winnowing_roundtrip() {
        let mut engine = setup_engine();
        let original = (0..=255u8).collect::<Vec<_>>();

        let chaffed = engine.inject_chaff(&original, "test-session").unwrap();
        let recovered = engine.winnow(&chaffed, "test-session").unwrap();

        assert_eq!(recovered, original);
    }

    #[test]
    fn test_chaff_increases_entropy() {
        let mut engine = setup_engine();
        let stream = b"cooperative traffic stream";
        let chaffed = engine.inject_chaff(stream, "test-session").unwrap();

        // Chaffed stream should have more packets than original
        assert!(chaffed.len() > 1);

        let (real, chaff) = engine.stats();
        assert!(real > 0);
        assert!(chaff > 0);
    }

    #[test]
    fn test_chaffing_custom_ratio() {
        let mut engine = ChaffingEngine::with_config(ChaffConfig {
            chaff_ratio: 0.8,
            ..ChaffConfig::default()
        });
        let key = ChaffingEngine::generate_session_key(b"high-ratio");
        engine.register_session_key("high".to_string(), key);

        let stream = b"high ratio chaffing test";
        let chaffed = engine.inject_chaff(stream, "high").unwrap();
        let recovered = engine.winnow(&chaffed, "high").unwrap();

        assert_eq!(recovered, stream);
    }

    #[test]
    fn test_chaffing_zero_ratio() {
        let mut engine = ChaffingEngine::with_config(ChaffConfig {
            chaff_ratio: 0.0,
            ..ChaffConfig::default()
        });
        let key = ChaffingEngine::generate_session_key(b"no-chaff");
        engine.register_session_key("none".to_string(), key);

        let stream = b"no chaff test";
        let _chaffed = engine.inject_chaff(stream, "none").unwrap();
        let (real, chaff) = engine.stats();

        assert!(real > 0);
        assert_eq!(chaff, 0);
    }
}

#[cfg(all(feature = "v3.0-steganographic-survival",))]
mod transport_rotation_tests {
    use ed2kia::pillars::steganographic::{TransportHealth, TransportRotator, TransportType};

    #[test]
    fn test_transport_rotation_resilience() {
        let mut rotator = TransportRotator::new();

        // Simulate TCP being unhealthy
        let tcp_health = TransportHealth::new(TransportType::Tcp, 800.0, 0.3, 50_000.0);
        let quic_health = TransportHealth::new(TransportType::Quic, 25.0, 0.0, 1_200_000.0);
        let ws_health = TransportHealth::new(TransportType::WebSocket, 120.0, 0.01, 600_000.0);

        rotator.update_health(tcp_health);
        rotator.update_health(quic_health);
        rotator.update_health(ws_health);

        // Should select QUIC as best
        let best = rotator.select_best();
        assert_eq!(best, Some(TransportType::Quic));

        // Rotate should pick QUIC
        let new_transport = rotator.rotate().unwrap();
        assert_eq!(new_transport, TransportType::Quic);
    }

    #[test]
    fn test_rotation_fallback_on_no_health() {
        let mut rotator = TransportRotator::new();
        // No health data — should fallback to cycling
        let transport1 = rotator.rotate().unwrap();
        let _transport2 = rotator.rotate().unwrap();
        // Should cycle through protocols
        assert_ne!(transport1, TransportType::Tcp); // Was Tcp, should rotate
    }

    #[test]
    fn test_force_transport_switch() {
        let mut rotator = TransportRotator::new();
        rotator.force_transport(&TransportType::WebRtc).unwrap();
        assert_eq!(*rotator.current_transport(), TransportType::WebRtc);
    }

    #[test]
    fn test_health_score_comparison() {
        let excellent = TransportHealth::new(TransportType::Quic, 10.0, 0.0, 2_000_000.0);
        let poor = TransportHealth::new(TransportType::Tcp, 500.0, 0.2, 100_000.0);

        assert!(excellent.score() > poor.score());
        assert!(excellent.is_healthy);
        assert!(!poor.is_healthy);
    }
}

#[cfg(all(feature = "v3.0-steganographic-survival",))]
mod orchestrator_integration_tests {
    use ed2kia::pillars::steganographic::{
        ChaffingEngine, SteganographicEngine, TransportHealth, TransportType,
    };
    use ed2kia::pillars::PillarInterface;

    #[test]
    fn test_pillar_interface() {
        let engine = SteganographicEngine::new();
        assert!(engine.validate_local_constraint());
        assert!(engine.consume_ce(5.0).is_ok());
    }

    #[test]
    fn test_full_obfuscation_pipeline() {
        let mut engine = SteganographicEngine::new();

        // Register session key
        let key = ChaffingEngine::generate_session_key(b"pipeline-test");
        engine
            .chaffing_engine_mut()
            .register_session_key("pipeline".to_string(), key);

        // Update health to prefer QUIC
        let health = TransportHealth::new(TransportType::Quic, 20.0, 0.0, 1_500_000.0);
        engine.update_health(health);

        let payload = b"ed2kIA steganographic preservation pipeline test";
        let (frames, chaffed, transport) = engine.obfuscate(payload, "pipeline").unwrap();

        // Verify pipeline output
        assert!(!frames.is_empty(), "Should produce SRTP frames");
        assert!(!chaffed.is_empty(), "Should produce chaffed packets");
        assert_eq!(
            transport,
            TransportType::Quic,
            "Should select best transport"
        );
    }

    #[test]
    fn test_transport_rotation_after_obfuscation() {
        let mut engine = SteganographicEngine::new();
        let initial = engine.current_transport().clone();

        // Rotate transport
        let new_transport = engine.rotate_transport().unwrap();
        assert_ne!(initial, new_transport);
        assert_eq!(*engine.current_transport(), new_transport);
    }

    #[test]
    fn test_ce_consumption_for_obfuscation() {
        let engine = SteganographicEngine::new();

        // Valid CE consumption
        assert!(engine.consume_ce(1.0).is_ok());

        // Zero CE rejected
        match engine.consume_ce(0.0) {
            Err(_) => {} // Expected
            Ok(_) => panic!("Zero CE should be rejected"),
        }

        // Negative CE rejected
        match engine.consume_ce(-1.0) {
            Err(_) => {} // Expected
            Ok(_) => panic!("Negative CE should be rejected"),
        }
    }
}
