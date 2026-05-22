//! v1.6.0 Sprint 1 E2E Integration Tests
//!
//! Covers LP-140 (Cross-Chain Bridge v3), LP-141 (Interop Layer v2), LP-142 (State Sync v2)

#[cfg(feature = "v1.6-sprint1")]
mod e2e {
    use std::collections::HashMap;
    use std::time::Instant;

    // LP-140: Cross-Chain Bridge v3
    use ed2kia::bridge::bridge_validator::BridgeValidator;
    use ed2kia::bridge::cross_chain_bridge_v3::{CrossChainBridgeV3, CrossChainBridgeV3Config};
    use ed2kia::bridge::relay_manager::RelayManager;

    // LP-141: Interop Layer v2
    use ed2kia::interop::interop_layer_v2::{InteropLayerV2, InteropMessage};
    use ed2kia::interop::protocol_adapter::{ProtocolAdapter, ProtocolMessage, ProtocolType};
    use ed2kia::interop::schema_negotiator::{SchemaDefinition, SchemaNegotiator};

    // LP-142: State Sync v2
    use ed2kia::state::merkle_aggregator::MerkleAggregator;
    use ed2kia::state::snapshot_manager::{SnapshotConfig, SnapshotManager};
    use ed2kia::state::state_sync_v2::{StateEntry, StateSyncV2};
    use sha2::Digest;

    fn current_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    // === LP-140: Cross-Chain Bridge v3 E2E ===

    #[test]
    fn test_e2e_bridge_v3_message_lifecycle() {
        let mut bridge = CrossChainBridgeV3::new(CrossChainBridgeV3Config {
            max_message_size: 1024,
            proof_ttl_ms: 60000,
            verification_threshold: 0.67,
            enable_merkle_proof: true,
            ..CrossChainBridgeV3Config::default()
        });

        bridge
            .register_chain("chain_a".to_string(), 0.9, 100.0)
            .unwrap();
        bridge
            .register_chain("chain_b".to_string(), 0.85, 80.0)
            .unwrap();

        let msg_id = bridge
            .submit_message("chain_a", "chain_b", b"hello cross-chain".to_vec())
            .unwrap();

        let verified = bridge.verify_message(&msg_id).unwrap();
        assert!(verified);

        bridge
            .add_signature(&msg_id, "signer_1".to_string())
            .unwrap();
        bridge
            .add_signature(&msg_id, "signer_2".to_string())
            .unwrap();
        bridge
            .add_signature(&msg_id, "signer_3".to_string())
            .unwrap();

        bridge.relay_message(&msg_id).unwrap();
    }

    #[test]
    fn test_e2e_bridge_v3_multi_chain_relay() {
        let mut bridge = CrossChainBridgeV3::default();

        for i in 0..4 {
            bridge
                .register_chain(format!("chain_{}", i), 0.9, 100.0)
                .unwrap();
        }

        let msg_id = bridge
            .submit_message("chain_0", "chain_3", b"multi-hop".to_vec())
            .unwrap();

        bridge.verify_message(&msg_id).unwrap();

        for j in 0..3 {
            bridge
                .add_signature(&msg_id, format!("relay_{}", j))
                .unwrap();
        }

        bridge.relay_message(&msg_id).unwrap();
    }

    #[test]
    fn test_e2e_bridge_validator_hash_verification() {
        let validator = BridgeValidator::default();

        let payload = b"verified_payload";
        // Compute expected hash using the same algorithm
        let mut hasher = sha2::Sha256::new();
        hasher.update(payload);
        let expected_hash = hex::encode(hasher.finalize());

        let result = validator.validate(payload, &expected_hash);
        assert!(result.valid);
        assert_eq!(result.score, 1.0);
    }

    #[test]
    fn test_e2e_relay_manager_quorum_coordination() {
        let mut relay = RelayManager::default();

        relay.register_node("relay_a".to_string(), 0.95).unwrap();
        relay.register_node("relay_b".to_string(), 0.90).unwrap();
        relay.register_node("relay_c".to_string(), 0.85).unwrap();

        let best = relay.get_best_nodes(2);
        assert_eq!(best.len(), 2);
        assert!(best[0].relay_score() >= best[1].relay_score());
    }

    // === LP-141: Interop Layer v2 E2E ===

    #[test]
    fn test_e2e_interop_v2_path_discovery() {
        let mut interop = InteropLayerV2::default();

        interop
            .register_federation("fed_a".to_string(), vec!["ep_a".to_string()])
            .unwrap();
        interop
            .register_federation("fed_b".to_string(), vec!["ep_b".to_string()])
            .unwrap();
        interop
            .register_federation("fed_c".to_string(), vec!["ep_c".to_string()])
            .unwrap();
        interop
            .register_federation("fed_d".to_string(), vec!["ep_d".to_string()])
            .unwrap();

        interop.add_connection("fed_a", "fed_b").unwrap();
        interop.add_connection("fed_b", "fed_c").unwrap();
        interop.add_connection("fed_c", "fed_d").unwrap();

        let path = interop.discover_path("fed_a", "fed_d").unwrap();
        assert_eq!(path.len(), 4);
        assert_eq!(path[0], "fed_a");
        assert_eq!(path[3], "fed_d");
    }

    #[test]
    fn test_e2e_interop_v2_message_routing() {
        let mut interop = InteropLayerV2::default();

        interop
            .register_federation("src".to_string(), vec!["ep_src".to_string()])
            .unwrap();
        interop
            .register_federation("dst".to_string(), vec!["ep_dst".to_string()])
            .unwrap();
        interop.add_connection("src", "dst").unwrap();

        let msg = InteropMessage {
            message_id: "msg_001".to_string(),
            source: "src".to_string(),
            destination: "dst".to_string(),
            payload: b"routed_message".to_vec(),
            schema_version: 1,
            compressed: false,
        };

        let result = interop.route_message(msg).unwrap();
        assert_eq!(result.hops, 1);
        assert_eq!(result.path.len(), 2);
    }

    #[test]
    fn test_e2e_protocol_adapter_roundtrip() {
        let adapter = ProtocolAdapter::default();

        let mut msg = ProtocolMessage::new(ProtocolType::Protobuf, 1);
        msg.fields.insert("key1".to_string(), vec![1, 2, 3]);
        msg.fields.insert("key2".to_string(), vec![4, 5, 6]);

        let serialized = adapter.serialize(&msg).unwrap();
        let deserialized = adapter
            .deserialize(&serialized, ProtocolType::Protobuf)
            .unwrap();

        assert_eq!(deserialized.protocol, ProtocolType::Protobuf);
        assert_eq!(deserialized.schema_version, 1);
        assert_eq!(deserialized.fields.len(), 2);
    }

    #[test]
    fn test_e2e_schema_negotiator_compatibility() {
        let mut negotiator = SchemaNegotiator::default();

        let mut schema_a = SchemaDefinition::new("test_schema".to_string(), 1);
        schema_a.add_field("field1".to_string(), "int32".to_string(), true);

        let mut schema_b = SchemaDefinition::new("test_schema".to_string(), 1);
        schema_b.add_field("field1".to_string(), "int32".to_string(), true);

        negotiator.register_schema(schema_a);
        negotiator.register_schema(schema_b);

        let result = negotiator
            .negotiate("test_schema", 1, "test_schema", 1)
            .unwrap();
        assert!(result.compatible);
    }

    // === LP-142: State Sync v2 E2E ===

    #[test]
    fn test_e2e_state_sync_v2_divergence_detection() {
        let mut sync_a = StateSyncV2::default();

        sync_a
            .register_state("key1".to_string(), vec![1, 2, 3])
            .unwrap();
        sync_a
            .register_state("key2".to_string(), vec![4, 5, 6])
            .unwrap();

        // Build peer state with divergence on key2
        let mut peer_state = HashMap::new();
        peer_state.insert(
            "key1".to_string(),
            StateEntry::new("key1".to_string(), vec![1, 2, 3]),
        );
        peer_state.insert(
            "key2".to_string(),
            StateEntry::new("key2".to_string(), vec![7, 8, 9]),
        );

        let result = sync_a.sync_state(&peer_state);
        assert_eq!(result.divergences.len(), 1);
    }

    #[test]
    fn test_e2e_state_sync_v2_full_sync() {
        let mut sync_a = StateSyncV2::default();

        // Build peer state with 20 matching keys
        let mut peer_state = HashMap::new();
        for i in 0..20 {
            let key = format!("key_{}", i);
            let value = vec![i as u8; 4];
            sync_a.register_state(key.clone(), value.clone()).unwrap();
            peer_state.insert(key.clone(), StateEntry::new(key, value));
        }

        let result = sync_a.sync_state(&peer_state);
        assert_eq!(result.divergences.len(), 0);
        assert_eq!(result.synced_keys, 20);
    }

    #[test]
    fn test_e2e_merkle_aggregator_proof_verification() {
        let mut aggregator = MerkleAggregator::new();

        let data: Vec<Vec<u8>> = (0..16).map(|i| vec![i as u8; 4]).collect();
        aggregator.build_from_leaves(&data);

        let root = aggregator.get_root().unwrap();
        let proof = aggregator.generate_proof(5).unwrap();

        let verified = aggregator.verify_proof(&data[5], &root, &proof, 5).unwrap();
        assert!(verified);
    }

    #[test]
    fn test_e2e_snapshot_manager_lifecycle() {
        let config = SnapshotConfig {
            max_snapshots: 10,
            enable_compression: false,
            enable_merkle_verification: true,
            snapshot_ttl_ms: 0,
        };
        let mut manager = SnapshotManager::new(config);

        // Create initial snapshot
        let mut state1 = HashMap::new();
        state1.insert("config".to_string(), vec![1, 0, 0]);
        state1.insert("data".to_string(), vec![10, 20, 30]);
        manager
            .create_snapshot("snap_v1".to_string(), state1, None)
            .unwrap();

        // Create updated snapshot
        let mut state2 = HashMap::new();
        state2.insert("config".to_string(), vec![1, 0, 1]);
        state2.insert("data".to_string(), vec![10, 20, 30]);
        state2.insert("new_key".to_string(), vec![99]);
        manager
            .create_snapshot("snap_v2".to_string(), state2, Some("snap_v1".to_string()))
            .unwrap();

        // Verify diff
        let diffs = manager.compute_diff("snap_v1", "snap_v2").unwrap();
        assert_eq!(diffs.len(), 2); // config modified + new_key added

        // Verify integrity
        let valid = manager.verify_integrity("snap_v2").unwrap();
        assert!(valid);

        // Restore
        let restored = manager.restore("snap_v1").unwrap();
        assert_eq!(restored.len(), 2);
    }

    // === CROSS-MODULE E2E ===

    #[test]
    fn test_e2e_cross_module_bridge_interop_state() {
        let mut bridge = CrossChainBridgeV3::default();
        let mut interop = InteropLayerV2::default();
        let mut state_sync = StateSyncV2::default();

        // Register chains and federations
        bridge
            .register_chain("chain_a".to_string(), 0.9, 100.0)
            .unwrap();
        bridge
            .register_chain("chain_b".to_string(), 0.85, 80.0)
            .unwrap();

        interop
            .register_federation("fed_a".to_string(), vec!["ep_a".to_string()])
            .unwrap();
        interop
            .register_federation("fed_b".to_string(), vec!["ep_b".to_string()])
            .unwrap();
        interop.add_connection("fed_a", "fed_b").unwrap();

        // Submit cross-chain message
        let msg_id = bridge
            .submit_message("chain_a", "chain_b", b"state_update".to_vec())
            .unwrap();
        bridge.verify_message(&msg_id).unwrap();

        // Route through interop layer
        let msg = InteropMessage {
            message_id: "interop_001".to_string(),
            source: "fed_a".to_string(),
            destination: "fed_b".to_string(),
            payload: b"state_update".to_vec(),
            schema_version: 1,
            compressed: false,
        };
        let route = interop.route_message(msg).unwrap();
        assert_eq!(route.hops, 1);

        // Sync state
        state_sync
            .register_state("msg_state".to_string(), b"verified".to_vec())
            .unwrap();
        let root = state_sync.compute_merkle_root();
        assert!(!root.is_empty());
    }

    #[test]
    fn test_e2e_full_pipeline_v1_6_sprint1() {
        let start = Instant::now();

        // Phase 1: Bridge setup and message submission
        let mut bridge = CrossChainBridgeV3::new(CrossChainBridgeV3Config {
            max_message_size: 2048,
            proof_ttl_ms: 120000,
            verification_threshold: 0.67,
            enable_merkle_proof: true,
            ..CrossChainBridgeV3Config::default()
        });
        bridge
            .register_chain("alpha".to_string(), 0.95, 200.0)
            .unwrap();
        bridge
            .register_chain("beta".to_string(), 0.90, 150.0)
            .unwrap();
        bridge
            .register_chain("gamma".to_string(), 0.85, 100.0)
            .unwrap();

        let msg_ids: Vec<String> = (0..10)
            .map(|i| {
                bridge
                    .submit_message("alpha", "beta", format!("message_{}", i).into_bytes())
                    .unwrap()
            })
            .collect();

        for id in &msg_ids {
            bridge.verify_message(id).unwrap();
            bridge.add_signature(id, "s1".to_string()).unwrap();
            bridge.add_signature(id, "s2".to_string()).unwrap();
            bridge.add_signature(id, "s3".to_string()).unwrap();
            bridge.relay_message(id).unwrap();
        }

        // Phase 2: Interop routing
        let mut interop = InteropLayerV2::default();
        for i in 0..6 {
            let fed_id = format!("fed_{}", i);
            interop
                .register_federation(fed_id.clone(), vec![format!("ep_{}", i)])
                .unwrap();
        }
        for i in 0..5 {
            interop
                .add_connection(&format!("fed_{}", i), &format!("fed_{}", i + 1))
                .unwrap();
        }

        let path = interop.discover_path("fed_0", "fed_5").unwrap();
        assert_eq!(path.len(), 6);

        // Phase 3: State sync with Merkle verification
        let mut sync_a = StateSyncV2::default();
        let mut sync_b = StateSyncV2::default();

        for i in 0..50 {
            let key = format!("state_{}", i);
            let value = vec![i as u8; 8];
            sync_a.register_state(key.clone(), value.clone()).unwrap();
            sync_b.register_state(key, value).unwrap();
        }

        // Build peer state matching sync_a
        let mut peer_state = HashMap::new();
        for i in 0..50 {
            let key = format!("state_{}", i);
            let value = vec![i as u8; 8];
            peer_state.insert(key.clone(), StateEntry::new(key, value));
        }
        let result = sync_a.sync_state(&peer_state);
        assert_eq!(result.divergences.len(), 0);
        assert_eq!(result.synced_keys, 50);

        // Phase 4: Snapshot management
        let config = SnapshotConfig {
            max_snapshots: 5,
            enable_compression: true,
            enable_merkle_verification: true,
            snapshot_ttl_ms: 0,
        };
        let mut manager = SnapshotManager::new(config);

        let mut state = HashMap::new();
        state.insert("pipeline_state".to_string(), b"complete".to_vec());
        manager
            .create_snapshot("final".to_string(), state, None)
            .unwrap();

        let valid = manager.verify_integrity("final").unwrap();
        assert!(valid);

        let elapsed = start.elapsed();
        eprintln!(
            "v1.6.0 Sprint 1 full pipeline: {:?} (10 bridge msgs, 6-fed routing, 50-key sync, snapshot)",
            elapsed
        );
    }

    #[test]
    fn test_e2e_stress_bridge_v3_high_volume() {
        let mut bridge = CrossChainBridgeV3::default();
        bridge
            .register_chain("src".to_string(), 0.95, 500.0)
            .unwrap();
        bridge
            .register_chain("dst".to_string(), 0.90, 400.0)
            .unwrap();

        for i in 0..100 {
            let msg_id = bridge
                .submit_message("src", "dst", format!("msg_{}", i).into_bytes())
                .unwrap();
            bridge.verify_message(&msg_id).unwrap();
            bridge.add_signature(&msg_id, "s1".to_string()).unwrap();
            bridge.add_signature(&msg_id, "s2".to_string()).unwrap();
            bridge.add_signature(&msg_id, "s3".to_string()).unwrap();
            bridge.relay_message(&msg_id).unwrap();
        }

        assert_eq!(bridge.active_message_count(), 100);
    }

    #[test]
    fn test_e2e_stress_interop_large_federation() {
        let mut interop = InteropLayerV2::default();

        for i in 0..50 {
            let fed_id = format!("fed_{}", i);
            interop
                .register_federation(fed_id.clone(), vec![format!("ep_{}", i)])
                .unwrap();
        }

        // Create mesh topology
        for i in 0..49 {
            interop
                .add_connection(&format!("fed_{}", i), &format!("fed_{}", i + 1))
                .unwrap();
        }

        let path = interop.discover_path("fed_0", "fed_49").unwrap();
        assert_eq!(path.len(), 50);
    }

    #[test]
    fn test_e2e_stress_state_sync_large_state() {
        let mut sync = StateSyncV2::default();

        for i in 0..500 {
            let key = format!("key_{}", i);
            let value = vec![i as u8; 16];
            sync.register_state(key, value).unwrap();
        }

        let root = sync.compute_merkle_root();
        assert!(!root.is_empty());
    }
}
