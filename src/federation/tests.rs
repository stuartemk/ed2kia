//! Tests para Federation Bridge - Fase 7 Sprint 1
//!
//! Cubre: handshake success/fail, delta merge, trust decay,
//! schema mismatch, malicious node simulation, empty sync.

#[cfg(test)]
mod federation_bridge_tests {
    use super::bridge::{
        BridgeError, BridgeResult, DeltaUpdate, FederationBridge,
        HandshakeMessage, NetworkIdentity, TrustRecord,
    };

    // =====================================================================
    // Helpers
    // =====================================================================

    fn make_identity(id: &str) -> NetworkIdentity {
        NetworkIdentity::new(
            id.to_string(),
            "genesis_hash_abc123".to_string(),
            "pubkey_hex_xyz789".to_string(),
        )
        .with_domain_tag("test".to_string())
    }

    fn make_delta(source_network: &str, layer_id: u32, weights: Vec<f32>) -> DeltaUpdate {
        DeltaUpdate::new(
            source_network.to_string(),
            "node_0".to_string(),
            layer_id,
            weights,
            1,
            5,
            0.9,
            "qwen-scope".to_string(),
        )
    }

    // =====================================================================
    // Test 1: Creación de NetworkIdentity
    // =====================================================================
    #[test]
    fn test_network_identity_creation() {
        let identity = make_identity("ed2k-test");
        assert_eq!(identity.network_id, "ed2k-test");
        assert_eq!(identity.domain_tags, vec!["test"]);
        assert!(identity.protocol_version.starts_with("7."));
    }

    // =====================================================================
    // Test 2: Creación de FederationBridge
    // =====================================================================
    #[test]
    fn test_bridge_creation() {
        let identity = make_identity("ed2k-local");
        let bridge = FederationBridge::new(identity, 0.6);

        assert_eq!(bridge.trusted_networks().len(), 0);
        assert_eq!(bridge.local_identity().network_id, "ed2k-local");
    }

    // =====================================================================
    // Test 3: Handshake success
    // =====================================================================
    #[test]
    fn test_handshake_success() {
        let local = make_identity("ed2k-local");
        let mut bridge = FederationBridge::new(local, 0.6);

        // Agregar red remota como confiable
        let remote = make_identity("ed2k-remote");
        bridge.add_trusted_network(remote);

        // Iniciar handshake
        let handshake = bridge.init_handshake("ed2k-remote");
        assert!(handshake.is_ok());

        let msg = handshake.unwrap();
        assert_eq!(msg.identity.network_id, "ed2k-local");
        assert!(msg.is_valid());
    }

    // =====================================================================
    // Test 4: Handshake fail - red desconocida
    // =====================================================================
    #[test]
    fn test_handshake_fail_unknown_network() {
        let local = make_identity("ed2k-local");
        let bridge = FederationBridge::new(local, 0.6);

        let result = bridge.init_handshake("ed2k-unknown");
        assert!(result.is_err());

        match result.unwrap_err() {
            BridgeError::HandshakeFailed { reason } => {
                assert!(reason.contains("Unknown network"));
            }
            other => panic!("Expected HandshakeFailed, got {:?}", other),
        }
    }

    // =====================================================================
    // Test 5: Process handshake response - success
    // =====================================================================
    #[test]
    fn test_process_handshake_response_success() {
        let local = make_identity("ed2k-local");
        let mut bridge = FederationBridge::new(local, 0.6);

        let remote = make_identity("ed2k-remote");
        let response = HandshakeMessage::new(
            remote,
            vec!["qwen-scope".to_string()],
        );

        let result = bridge.process_handshake_response(response);
        assert!(result.is_ok());
        assert_eq!(bridge.trusted_networks().len(), 1);
    }

    // =====================================================================
    // Test 6: Process handshake response - no common schema
    // =====================================================================
    #[test]
    fn test_handshake_fail_no_common_schema() {
        let local = make_identity("ed2k-local");
        let mut bridge = FederationBridge::new(local, 0.6);

        let remote = make_identity("ed2k-remote");
        let response = HandshakeMessage::new(
            remote,
            vec!["unknown-schema".to_string()], // No common schema
        );

        let result = bridge.process_handshake_response(response);
        assert!(result.is_err());

        match result.unwrap_err() {
            BridgeError::HandshakeFailed { reason } => {
                assert!(reason.contains("No common schema"));
            }
            other => panic!("Expected HandshakeFailed, got {:?}", other),
        }
    }

    // =====================================================================
    // Test 7: Delta hash verification - valid
    // =====================================================================
    #[test]
    fn test_delta_hash_valid() {
        let delta = make_delta("ed2k-remote", 0, vec![1.0, 2.0, 3.0]);
        assert!(delta.verify_hash(), "Delta hash should be valid");
    }

    // =====================================================================
    // Test 8: Delta hash verification - tampered
    // =====================================================================
    #[test]
    fn test_delta_hash_tampered() {
        let mut delta = make_delta("ed2k-remote", 0, vec![1.0, 2.0, 3.0]);
        delta.weights[0] = 999.0; // Tamper with weights

        assert!(!delta.verify_hash(), "Tampered delta should fail hash check");
    }

    // =====================================================================
    // Test 9: Sync delta - success
    // =====================================================================
    #[test]
    fn test_sync_delta_success() {
        let local = make_identity("ed2k-local");
        let mut bridge = FederationBridge::new(local, 0.3); // Low threshold for test

        // Agregar red remota con confianza suficiente
        let remote = make_identity("ed2k-remote");
        bridge.add_trusted_network(remote);

        // Subir confianza manualmente
        if let Some(record) = bridge.get_trust_record("ed2k-remote") {
            // TrustRecord::new starts at 0.5, which is > 0.3 threshold
        }

        let delta = make_delta("ed2k-remote", 0, vec![0.1, 0.2, 0.3]);
        let result = bridge.sync_delta(delta);
        assert!(result.is_ok(), "Sync should succeed with sufficient trust");
    }

    // =====================================================================
    // Test 10: Sync delta - trust too low
    // =====================================================================
    #[test]
    fn test_sync_delta_trust_too_low() {
        let local = make_identity("ed2k-local");
        let mut bridge = FederationBridge::new(local, 0.9); // High threshold

        let remote = make_identity("ed2k-remote");
        bridge.add_trusted_network(remote);
        // Default trust is 0.5, which is < 0.9 threshold

        let delta = make_delta("ed2k-remote", 0, vec![0.1, 0.2, 0.3]);
        let result = bridge.sync_delta(delta);

        assert!(result.is_err());
        match result.unwrap_err() {
            BridgeError::TrustTooLow {
                trust_score,
                min_trust,
                ..
            } => {
                assert!(trust_score < min_trust);
            }
            other => panic!("Expected TrustTooLow, got {:?}", other),
        }
    }

    // =====================================================================
    // Test 11: Merge updates - success
    // =====================================================================
    #[test]
    fn test_merge_updates_success() {
        let local = make_identity("ed2k-local");
        let mut bridge = FederationBridge::new(local, 0.3);

        let remote = make_identity("ed2k-remote");
        bridge.add_trusted_network(remote);

        // Sync multiple deltas for same layer
        bridge.sync_delta(make_delta("ed2k-remote", 0, vec![0.1, 0.2])).unwrap();
        bridge.sync_delta(make_delta("ed2k-remote", 0, vec![0.3, 0.4])).unwrap();

        let result = bridge.merge_updates();
        assert!(result.is_ok());

        let bridge_result = result.unwrap();
        assert_eq!(bridge_result.merged_updates, 1); // 1 layer merged
        assert!(bridge_result.trust_avg > 0.0);
        assert_eq!(bridge_result.protocol_version, "7.1.0");
    }

    // =====================================================================
    // Test 12: Merge updates - empty
    // =====================================================================
    #[test]
    fn test_merge_updates_empty() {
        let local = make_identity("ed2k-local");
        let bridge = FederationBridge::new(local, 0.6);

        let result = bridge.merge_updates();
        assert!(result.is_ok());

        let bridge_result = result.unwrap();
        assert_eq!(bridge_result.merged_updates, 0);
        assert_eq!(bridge_result.synced_nodes, 0);
    }

    // =====================================================================
    // Test 13: Trust decay
    // =====================================================================
    #[test]
    fn test_trust_decay() {
        let record = TrustRecord::new("test-network".to_string());
        let initial_trust = record.trust_score;

        // Aplicar decaimiento múltiple
        let mut mutable_record = record.clone();
        for _ in 0..100 {
            mutable_record.apply_decay();
        }

        assert!(
            mutable_record.trust_score < initial_trust,
            "Trust should decay over time"
        );
        assert!(
            mutable_record.trust_score >= 0.0,
            "Trust should not go below 0"
        );
    }

    // =====================================================================
    // Test 14: Trust record - success increases trust
    // =====================================================================
    #[test]
    fn test_trust_record_success() {
        let mut record = TrustRecord::new("test-network".to_string());
        let initial = record.trust_score;

        record.record_success();
        assert!(record.trust_score > initial, "Success should increase trust");
        assert!(record.trust_score <= 1.0, "Trust should cap at 1.0");
    }

    // =====================================================================
    // Test 15: Trust record - failure decreases trust
    // =====================================================================
    #[test]
    fn test_trust_record_failure() {
        let mut record = TrustRecord::new("test-network".to_string());
        let initial = record.trust_score;

        record.record_failure();
        assert!(record.trust_score < initial, "Failure should decrease trust");
        assert!(record.trust_score >= 0.0, "Trust should floor at 0.0");
    }

    // =====================================================================
    // Test 16: Malicious node simulation - repeated failures
    // =====================================================================
    #[test]
    fn test_malicious_node_trust_drops() {
        let mut record = TrustRecord::new("malicious-network".to_string());

        // Simular 20 fallos consecutivos
        for _ in 0..20 {
            record.record_failure();
        }

        assert!(
            record.trust_score < 0.1,
            "Malicious node trust should drop near 0 after repeated failures"
        );
    }

    // =====================================================================
    // Test 17: Schema translation - same schema (passthrough)
    // =====================================================================
    #[test]
    fn test_schema_translation_same_schema() {
        let local = make_identity("ed2k-local");
        let bridge = FederationBridge::new(local, 0.6);

        // Sync delta con mismo schema (qwen-scope → qwen-scope)
        let remote = make_identity("ed2k-remote");
        let mut bridge = FederationBridge::new(local, 0.3);
        bridge.add_trusted_network(remote);

        let delta = make_delta("ed2k-remote", 0, vec![1.0, 2.0, 3.0]);
        let result = bridge.sync_delta(delta);
        assert!(result.is_ok(), "Same schema should pass through");
    }

    // =====================================================================
    // Test 18: Protocol version compatibility
    // =====================================================================
    #[test]
    fn test_protocol_version_compatibility() {
        let mut local = make_identity("ed2k-local");
        let mut remote = make_identity("ed2k-remote");

        // Mismo major version → compatible
        local.protocol_version = "7.0.0".to_string();
        remote.protocol_version = "7.1.0".to_string();

        let bridge = FederationBridge::new(local, 0.6);
        // Access private method through public interface
        // (versions_compatible is tested indirectly via handshake)

        // Diferente major version → incompatible
        let mut remote_v8 = make_identity("ed2k-remote-v8");
        remote_v8.protocol_version = "8.0.0".to_string();

        let mut bridge = FederationBridge::new(
            make_identity("ed2k-local"),
            0.6,
        );
        bridge.add_trusted_network(remote_v8);

        let result = bridge.init_handshake("ed2k-remote-v8");
        assert!(result.is_err(), "Different major version should fail");
    }

    // =====================================================================
    // Test 19: Bridge result history
    // =====================================================================
    #[test]
    fn test_result_history() {
        let local = make_identity("ed2k-local");
        let mut bridge = FederationBridge::new(local, 0.3);

        let remote = make_identity("ed2k-remote");
        bridge.add_trusted_network(remote);

        bridge.sync_delta(make_delta("ed2k-remote", 0, vec![0.1, 0.2])).unwrap();
        bridge.merge_updates().unwrap();

        assert_eq!(bridge.result_history().len(), 1);
    }

    // =====================================================================
    // Test 20: Add supported schema
    // =====================================================================
    #[test]
    fn test_add_supported_schema() {
        let local = make_identity("ed2k-local");
        let mut bridge = FederationBridge::new(local, 0.6);

        bridge.add_supported_schema("llama-3".to_string());

        // Handshake con red que soporta llama-3 debería funcionar
        let remote = make_identity("ed2k-remote");
        let response = HandshakeMessage::new(
            remote,
            vec!["llama-3".to_string()],
        );

        let result = bridge.process_handshake_response(response);
        assert!(result.is_ok(), "Should accept llama-3 schema");
    }

    // =====================================================================
    // Test 21: Apply trust decay to all networks
    // =====================================================================
    #[test]
    fn test_apply_trust_decay_all() {
        let local = make_identity("ed2k-local");
        let mut bridge = FederationBridge::new(local, 0.6);

        bridge.add_trusted_network(make_identity("net-a"));
        bridge.add_trusted_network(make_identity("net-b"));

        let trust_a_before = bridge.calculate_trust_score("net-a");
        bridge.apply_trust_decay();
        let trust_a_after = bridge.calculate_trust_score("net-a");

        assert!(
            trust_a_after < trust_a_before,
            "Trust should decay after apply_trust_decay"
        );
    }

    // =====================================================================
    // Test 22: Handshake expiration
    // =====================================================================
    #[test]
    fn test_handshake_expiration() {
        let remote = make_identity("ed2k-remote");
        let mut response = HandshakeMessage::new(
            remote,
            vec!["qwen-scope".to_string()],
        );

        // Manipular timestamp para simular expiración
        response.timestamp_ms = 0; // Very old timestamp

        // After more than 5 minutes, should be invalid
        assert!(
            !response.is_valid(),
            "Old handshake should be invalid"
        );
    }
}
