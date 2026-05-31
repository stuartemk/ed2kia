//! Corpuscular Bridge Integration Tests — Sprint 43
//!
//! Validates the complete Corpuscular Bridge stack:
//! - Local hardware registration (LOCAL_ONLY enforcement)
//! - CE voucher minting and redemption
//! - Replay protection
//! - Orchestrator routing via PillarMessage

#[cfg(all(
    feature = "v3.0-corpuscular-bridge",
    feature = "v3.0-pillar-messaging",
    feature = "v3.0-orchestration"
))]
mod local_hardware_tests {
    use ed2kia::pillars::corpuscular::iot_adapter::{
        AdapterError, HardwareConfig, HardwareId, LocalHardwareAdapter,
    };
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    fn make_local_config(port: u16) -> HardwareConfig {
        HardwareConfig {
            endpoint: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
            device_type: "3d_printer".to_string(),
            node_signature: vec![1, 2, 3, 4],
            max_payload_bytes: 4096,
        }
    }

    #[test]
    fn test_local_hardware_registration() {
        let mut adapter = LocalHardwareAdapter::new();
        let id = HardwareId("printer-local-1".to_string());
        let config = make_local_config(8080);

        // Registration should succeed for loopback endpoint.
        assert!(adapter.register_local_device(id.clone(), config).is_ok());
        assert_eq!(adapter.device_count(), 1);
        assert!(adapter.get_device(&id).is_some());
    }

    #[test]
    fn test_non_local_endpoint_rejected() {
        let config = HardwareConfig {
            endpoint: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 50)), 8080),
            device_type: "external_device".to_string(),
            node_signature: vec![1],
            max_payload_bytes: 1024,
        };

        match HardwareConfig::validate_local_endpoint(&config.endpoint) {
            Err(AdapterError::NonLocalEndpoint(_)) => {} // Expected — LOCAL_ONLY enforced.
            other => panic!("Expected NonLocalEndpoint, got {:?}", other),
        }
    }

    #[test]
    fn test_command_routing_local_only() {
        let mut adapter = LocalHardwareAdapter::new();
        let id = HardwareId("printer-1".to_string());
        adapter
            .register_local_device(id.clone(), make_local_config(8080))
            .unwrap();

        let payload = b"print_layer_1";
        let response = adapter.route_command(id, payload).unwrap();
        assert!(response.starts_with(b"OK:"));
    }

    #[test]
    fn test_duplicate_registration_rejected() {
        let mut adapter = LocalHardwareAdapter::new();
        let id = HardwareId("device-1".to_string());
        assert!(adapter
            .register_local_device(id.clone(), make_local_config(8080))
            .is_ok());

        match adapter.register_local_device(id, make_local_config(8081)) {
            Err(AdapterError::DeviceAlreadyRegistered(_)) => {} // Expected
            other => panic!("Expected DeviceAlreadyRegistered, got {:?}", other),
        }
    }

    #[test]
    fn test_empty_signature_rejected() {
        let mut adapter = LocalHardwareAdapter::new();
        let id = HardwareId("unsigned".to_string());
        let mut config = make_local_config(8080);
        config.node_signature.clear();

        match adapter.register_local_device(id, config) {
            Err(AdapterError::InvalidSignature) => {} // Expected
            other => panic!("Expected InvalidSignature, got {:?}", other),
        }
    }
}

#[cfg(all(feature = "v3.0-corpuscular-bridge",))]
mod ce_voucher_tests {
    use ed2kia::pillars::corpuscular::ce_exchange::{CEExchangeEngine, ExchangeError};
    use ed2kia::pillars::ResourceType;

    #[test]
    fn test_ce_voucher_mint_redeem() {
        let mut engine = CEExchangeEngine::new();

        // Mint voucher with valid CE and positive Z.
        let voucher = engine.mint_voucher(15.0, ResourceType::Print3DHours(3.0), 0.7, 42);
        assert!(voucher.is_ok());
        let voucher = voucher.unwrap();
        assert_eq!(voucher.ce_amount, 15.0);
        assert!(!voucher.signature.is_empty());

        // Redeem voucher.
        let fulfillment = engine.redeem_physical_resource(&voucher, b"print_complete".to_vec());
        assert!(fulfillment.is_ok());
        let fulfillment = fulfillment.unwrap();
        assert_eq!(fulfillment.ce_consumed, 15.0);
        assert_eq!(fulfillment.hardware_response, b"print_complete");
    }

    #[test]
    fn test_mint_zero_ce_rejected() {
        let mut engine = CEExchangeEngine::new();
        match engine.mint_voucher(0.0, ResourceType::SolarEnergyKwh(1.0), 0.5, 1) {
            Err(ExchangeError::InvalidCEAmount(0.0)) => {} // Expected
            other => panic!("Expected InvalidCEAmount, got {:?}", other),
        }
    }

    #[test]
    fn test_mint_negative_z_rejected() {
        let mut engine = CEExchangeEngine::new();
        match engine.mint_voucher(10.0, ResourceType::SolarEnergyKwh(1.0), -0.5, 1) {
            Err(ExchangeError::NegativeZScore(z)) if z < 0.0 => {} // Expected
            other => panic!("Expected NegativeZScore, got {:?}", other),
        }
    }

    #[test]
    fn test_redeem_empty_signature_rejected() {
        let mut engine = CEExchangeEngine::new();
        let voucher = ed2kia::pillars::CEVoucher {
            ce_amount: 10.0,
            resource_type: ResourceType::Print3DHours(1.0),
            signature: vec![],
        };
        match engine.redeem_physical_resource(&voucher, vec![]) {
            Err(ExchangeError::InvalidSignature) => {} // Expected
            other => panic!("Expected InvalidSignature, got {:?}", other),
        }
    }
}

#[cfg(all(feature = "v3.0-corpuscular-bridge",))]
mod replay_protection_tests {
    use ed2kia::pillars::corpuscular::ce_exchange::CEExchangeEngine;
    use ed2kia::pillars::ResourceType;

    #[test]
    fn test_replay_protection() {
        let mut engine = CEExchangeEngine::new();

        // Mint and redeem first voucher.
        let voucher = engine
            .mint_voucher(10.0, ResourceType::Print3DHours(1.0), 0.5, 100)
            .unwrap();
        assert!(engine
            .redeem_physical_resource(&voucher, b"ok".to_vec())
            .is_ok());

        // Attempt to redeem the same voucher again — should be detected as replay.
        match engine.redeem_physical_resource(&voucher, b"ok".to_vec()) {
            Err(ed2kia::pillars::corpuscular::ce_exchange::ExchangeError::ReplayDetected(_)) => {
                // Expected — replay protection active.
            }
            other => panic!("Expected ReplayDetected, got {:?}", other),
        }
    }

    #[test]
    fn test_different_vouchers_no_replay() {
        let mut engine = CEExchangeEngine::new();

        // Two different vouchers (different CE amounts = different surrogate nonce).
        let v1 = engine
            .mint_voucher(10.0, ResourceType::Print3DHours(1.0), 0.5, 1)
            .unwrap();
        let v2 = engine
            .mint_voucher(20.0, ResourceType::SolarEnergyKwh(2.0), 0.5, 2)
            .unwrap();

        assert!(engine
            .redeem_physical_resource(&v1, b"ok1".to_vec())
            .is_ok());
        assert!(engine
            .redeem_physical_resource(&v2, b"ok2".to_vec())
            .is_ok());
    }

    #[test]
    fn test_ce_window_limit() {
        let mut engine = CEExchangeEngine::new();

        // Mint 800 CE.
        let v1 = engine.mint_voucher(800.0, ResourceType::Print3DHours(10.0), 0.5, 1);
        assert!(v1.is_ok());

        // Mint another 200 CE (total = 1000, at limit).
        let v2 = engine.mint_voucher(200.0, ResourceType::Print3DHours(5.0), 0.5, 2);
        assert!(v2.is_ok());

        // Try to mint 1 more CE — should exceed window limit.
        match engine.mint_voucher(1.0, ResourceType::Print3DHours(1.0), 0.5, 3) {
            Err(
                ed2kia::pillars::corpuscular::ce_exchange::ExchangeError::CEWindowLimitExceeded,
            ) => {
                // Expected — window limit enforced.
            }
            other => panic!("Expected CEWindowLimitExceeded, got {:?}", other),
        }
    }
}

#[cfg(all(
    feature = "v3.0-corpuscular-bridge",
    feature = "v3.0-pillar-messaging",
    feature = "v3.0-orchestration"
))]
mod orchestrator_routing_tests {
    use ed2kia::orchestration::PillarId;
    use ed2kia::pillars::corpuscular::CorpuscularEngine;
    use ed2kia::pillars::PillarInterface;
    use ed2kia::runtime::pillar_messaging::PillarMessage;

    fn make_message(pillar_id: PillarId, ce_weight: f64, payload: Vec<u8>) -> PillarMessage {
        PillarMessage::new(
            payload,
            vec![0xAB, 0xCD], // Signature.
            pillar_id,
            1_000_000, // Timestamp.
            1,         // Nonce.
            ce_weight,
        )
    }

    #[test]
    fn test_orchestrator_routing_valid_message() {
        let mut engine = CorpuscularEngine::new();
        let msg = make_message(PillarId::CorpuscularBridge, 5.0, b"route_test".to_vec());

        let response = engine.handle_request(&msg);
        assert!(response.is_ok());
        let resp = response.unwrap();
        assert_eq!(resp.ce_consumed, 5.0);
        assert!(resp.data.len() > 0);
    }

    #[test]
    fn test_orchestrator_routing_wrong_pillar() {
        let mut engine = CorpuscularEngine::new();
        let msg = make_message(
            PillarId::ResonanceInterface, // Wrong pillar.
            5.0,
            b"test".to_vec(),
        );

        match engine.handle_request(&msg) {
            Err(ed2kia::pillars::PillarError::UnsupportedResource) => {} // Expected
            other => panic!("Expected UnsupportedResource, got {:?}", other),
        }
    }

    #[test]
    fn test_orchestrator_routing_zero_ce() {
        let mut engine = CorpuscularEngine::new();
        let msg = make_message(
            PillarId::CorpuscularBridge,
            0.0, // Zero CE.
            b"test".to_vec(),
        );

        match engine.handle_request(&msg) {
            Err(ed2kia::pillars::PillarError::InsufficientCE) => {} // Expected
            other => panic!("Expected InsufficientCE, got {:?}", other),
        }
    }

    #[test]
    fn test_pillar_interface_integration() {
        let engine = CorpuscularEngine::new();
        assert_eq!(CorpuscularEngine::id(), PillarId::CorpuscularBridge);
        assert!(engine.validate_local_constraint());
    }

    #[test]
    fn test_empty_payload_returns_status() {
        let mut engine = CorpuscularEngine::new();
        let msg = make_message(
            PillarId::CorpuscularBridge,
            3.0,
            vec![], // Empty payload.
        );

        let response = engine.handle_request(&msg).unwrap();
        assert!(response.data.starts_with(b"corpuscular-status:"));
    }
}
