//! v1.6.0 Sprint 1 Benchmarks
//!
//! Performance benchmarks for Cross-Chain Bridge v3, Interop Layer v2, and State Sync v2.
//!
//! | Benchmark | Description | Target |
//! |-----------|-------------|--------|
//! | `bench_bridge_v3_submit_100` | Submit 100 cross-chain messages | < 100ms |
//! | `bench_bridge_v3_verify_100` | Verify 100 messages | < 50ms |
//! | `bench_bridge_v3_relay_100` | Relay 100 messages with quorum | < 200ms |
//! | `bench_interop_v2_route_100` | Route 100 messages through federation | < 100ms |
//! | `bench_interop_v2_path_discovery` | BFS path discovery in 50-node graph | < 10ms |
//! | `bench_protocol_adapter_serialize` | Serialize 1000 protocol messages | < 50ms |
//! | `bench_state_sync_v2_sync_500` | Sync 500 keys between peers | < 100ms |
//! | `bench_merkle_aggregator_256` | Build Merkle tree with 256 leaves | < 50ms |
//! | `bench_snapshot_create_100` | Create 100 snapshots | < 200ms |
//! | `bench_snapshot_verify_100` | Verify 100 snapshot integrity | < 100ms |
//!
//! # Running Benchmarks
//!
//! ```bash
//! cargo test --features v1.6-sprint1 --test v1_6_sprint1_bench
//! ```

#[cfg(feature = "v1.6-sprint1")]
mod benchmarks {
    use std::collections::HashMap;
    use std::time::Instant;

    use ed2kia::bridge::cross_chain_bridge_v3::{CrossChainBridgeV3, CrossChainBridgeV3Config};
    use ed2kia::interop::interop_layer_v2::{InteropLayerV2, InteropMessage};
    use ed2kia::interop::protocol_adapter::{ProtocolAdapter, ProtocolMessage, ProtocolType};
    use ed2kia::state::merkle_aggregator::MerkleAggregator;
    use ed2kia::state::snapshot_manager::{SnapshotConfig, SnapshotManager};
    use ed2kia::state::state_sync_v2::{StateEntry, StateSyncV2};

    // === Bridge v3 Benchmarks ===

    #[test]
    fn bench_bridge_v3_submit_100() {
        let mut bridge = CrossChainBridgeV3::new(CrossChainBridgeV3Config {
            max_message_size: 1024,
            proof_ttl_ms: 60000,
            verification_threshold: 0.67,
            enable_merkle_proof: true,
            ..CrossChainBridgeV3Config::default()
        });
        bridge
            .register_chain("src".to_string(), 0.95, 500.0)
            .unwrap();
        bridge
            .register_chain("dst".to_string(), 0.90, 400.0)
            .unwrap();

        let start = Instant::now();
        for i in 0..100 {
            bridge
                .submit_message("src", "dst", format!("msg_{}", i).into_bytes())
                .unwrap();
        }
        let elapsed = start.elapsed();

        eprintln!(
            "bench_bridge_v3_submit_100: {:?} ({:.2} us/msg)",
            elapsed,
            elapsed.as_micros() as f64 / 100.0
        );
        assert!(elapsed.as_millis() < 100);
    }

    #[test]
    fn bench_bridge_v3_verify_100() {
        let mut bridge = CrossChainBridgeV3::default();
        bridge
            .register_chain("src".to_string(), 0.95, 500.0)
            .unwrap();
        bridge
            .register_chain("dst".to_string(), 0.90, 400.0)
            .unwrap();

        let msg_ids: Vec<String> = (0..100)
            .map(|i| {
                bridge
                    .submit_message("src", "dst", format!("msg_{}", i).into_bytes())
                    .unwrap()
            })
            .collect();

        let start = Instant::now();
        for id in &msg_ids {
            bridge.verify_message(id).unwrap();
        }
        let elapsed = start.elapsed();

        eprintln!(
            "bench_bridge_v3_verify_100: {:?} ({:.2} us/msg)",
            elapsed,
            elapsed.as_micros() as f64 / 100.0
        );
        assert!(elapsed.as_millis() < 50);
    }

    #[test]
    fn bench_bridge_v3_relay_100() {
        let mut bridge = CrossChainBridgeV3::default();
        bridge
            .register_chain("src".to_string(), 0.95, 500.0)
            .unwrap();
        bridge
            .register_chain("dst".to_string(), 0.90, 400.0)
            .unwrap();

        let msg_ids: Vec<String> = (0..100)
            .map(|i| {
                let id = bridge
                    .submit_message("src", "dst", format!("msg_{}", i).into_bytes())
                    .unwrap();
                bridge.verify_message(&id).unwrap();
                bridge.add_signature(&id, "s1".to_string()).unwrap();
                bridge.add_signature(&id, "s2".to_string()).unwrap();
                bridge.add_signature(&id, "s3".to_string()).unwrap();
                id
            })
            .collect();

        let start = Instant::now();
        for id in &msg_ids {
            bridge.relay_message(id).unwrap();
        }
        let elapsed = start.elapsed();

        eprintln!(
            "bench_bridge_v3_relay_100: {:?} ({:.2} us/msg)",
            elapsed,
            elapsed.as_micros() as f64 / 100.0
        );
        assert!(elapsed.as_millis() < 200);
    }

    // === Interop Layer v2 Benchmarks ===

    #[test]
    fn bench_interop_v2_route_100() {
        let mut interop = InteropLayerV2::default();
        for i in 0..8 {
            let fed_id = format!("fed_{}", i);
            interop
                .register_federation(fed_id.clone(), vec![format!("ep_{}", i)])
                .unwrap();
        }
        for i in 0..7 {
            interop
                .add_connection(&format!("fed_{}", i), &format!("fed_{}", i + 1))
                .unwrap();
        }

        let start = Instant::now();
        for i in 0..100 {
            let msg = InteropMessage {
                message_id: format!("msg_{}", i),
                source: "fed_0".to_string(),
                destination: "fed_7".to_string(),
                payload: format!("msg_{}", i).into_bytes(),
                schema_version: 1,
                compressed: false,
            };
            interop.route_message(msg).unwrap();
        }
        let elapsed = start.elapsed();

        eprintln!(
            "bench_interop_v2_route_100: {:?} ({:.2} us/msg)",
            elapsed,
            elapsed.as_micros() as f64 / 100.0
        );
        assert!(elapsed.as_millis() < 100);
    }

    #[test]
    fn bench_interop_v2_path_discovery() {
        let mut interop = InteropLayerV2::default();
        for i in 0..50 {
            let fed_id = format!("fed_{}", i);
            interop
                .register_federation(fed_id.clone(), vec![format!("ep_{}", i)])
                .unwrap();
        }
        for i in 0..49 {
            interop
                .add_connection(&format!("fed_{}", i), &format!("fed_{}", i + 1))
                .unwrap();
        }

        let start = Instant::now();
        let path = interop.discover_path("fed_0", "fed_49").unwrap();
        let elapsed = start.elapsed();

        eprintln!(
            "bench_interop_v2_path_discovery: {:?} ({} hops)",
            elapsed,
            path.len()
        );
        assert!(elapsed.as_millis() < 10);
    }

    // === Protocol Adapter Benchmarks ===

    #[test]
    fn bench_protocol_adapter_serialize() {
        let adapter = ProtocolAdapter::default();

        let messages: Vec<ProtocolMessage> = (0..1000)
            .map(|i| {
                let mut msg = ProtocolMessage::new(ProtocolType::Protobuf, 1);
                msg.fields.insert("id".to_string(), vec![i as u8]);
                msg.fields.insert("data".to_string(), vec![i as u8; 32]);
                msg
            })
            .collect();

        let start = Instant::now();
        for msg in &messages {
            adapter.serialize(msg).unwrap();
        }
        let elapsed = start.elapsed();

        eprintln!(
            "bench_protocol_adapter_serialize: {:?} ({:.2} us/msg)",
            elapsed,
            elapsed.as_micros() as f64 / 1000.0
        );
        assert!(elapsed.as_millis() < 50);
    }

    // === State Sync v2 Benchmarks ===

    #[test]
    fn bench_state_sync_v2_sync_500() {
        let mut sync_a = StateSyncV2::default();

        // Build peer state with 500 matching keys
        let mut peer_state = HashMap::new();
        for i in 0..500 {
            let key = format!("key_{}", i);
            let value = vec![i as u8; 16];
            sync_a.register_state(key.clone(), value.clone()).unwrap();
            peer_state.insert(key.clone(), StateEntry::new(key, value));
        }

        let start = Instant::now();
        let result = sync_a.sync_state(&peer_state);
        let elapsed = start.elapsed();

        eprintln!(
            "bench_state_sync_v2_sync_500: {:?} ({} keys, {} divergences)",
            elapsed,
            result.synced_keys,
            result.divergences.len()
        );
        assert!(elapsed.as_millis() < 100);
    }

    // === Merkle Aggregator Benchmarks ===

    #[test]
    fn bench_merkle_aggregator_256() {
        let mut aggregator = MerkleAggregator::new();
        let data: Vec<Vec<u8>> = (0..256).map(|i| vec![i as u8; 32]).collect();

        let start = Instant::now();
        aggregator.build_from_leaves(&data);
        let root = aggregator.get_root().unwrap();
        let elapsed = start.elapsed();

        eprintln!(
            "bench_merkle_aggregator_256: {:?} (root: {}...)",
            elapsed,
            &root[..16]
        );
        assert!(elapsed.as_millis() < 50);
    }

    #[test]
    fn bench_merkle_aggregator_proof_256() {
        let mut aggregator = MerkleAggregator::new();
        let data: Vec<Vec<u8>> = (0..256).map(|i| vec![i as u8; 32]).collect();
        aggregator.build_from_leaves(&data);
        let root = aggregator.get_root().unwrap();

        let start = Instant::now();
        for i in 0..256 {
            let proof = aggregator.generate_proof(i).unwrap();
            aggregator.verify_proof(&data[i], &root, &proof, i).unwrap();
        }
        let elapsed = start.elapsed();

        eprintln!(
            "bench_merkle_aggregator_proof_256: {:?} ({:.2} us/proof)",
            elapsed,
            elapsed.as_micros() as f64 / 256.0
        );
        assert!(elapsed.as_millis() < 1000);
    }

    // === Snapshot Manager Benchmarks ===

    #[test]
    fn bench_snapshot_create_100() {
        let config = SnapshotConfig {
            max_snapshots: 200,
            enable_compression: true,
            enable_merkle_verification: true,
            snapshot_ttl_ms: 0,
        };
        let mut manager = SnapshotManager::new(config);

        let start = Instant::now();
        for i in 0..100 {
            let mut state = HashMap::new();
            for j in 0..50 {
                let key = format!("key_{}", j);
                let value = vec![i as u8, j as u8];
                state.insert(key, value);
            }
            manager
                .create_snapshot(format!("snap_{}", i), state, None)
                .unwrap();
        }
        let elapsed = start.elapsed();

        eprintln!(
            "bench_snapshot_create_100: {:?} ({:.2} us/snapshot)",
            elapsed,
            elapsed.as_micros() as f64 / 100.0
        );
        assert!(elapsed.as_millis() < 200);
    }

    #[test]
    fn bench_snapshot_verify_100() {
        let config = SnapshotConfig {
            max_snapshots: 200,
            enable_compression: false,
            enable_merkle_verification: true,
            snapshot_ttl_ms: 0,
        };
        let mut manager = SnapshotManager::new(config);

        for i in 0..100 {
            let mut state = HashMap::new();
            for j in 0..50 {
                let key = format!("key_{}", j);
                let value = vec![i as u8, j as u8];
                state.insert(key, value);
            }
            manager
                .create_snapshot(format!("snap_{}", i), state, None)
                .unwrap();
        }

        let start = Instant::now();
        for i in 0..100 {
            manager.verify_integrity(&format!("snap_{}", i)).unwrap();
        }
        let elapsed = start.elapsed();

        eprintln!(
            "bench_snapshot_verify_100: {:?} ({:.2} us/verify)",
            elapsed,
            elapsed.as_micros() as f64 / 100.0
        );
        assert!(elapsed.as_millis() < 100);
    }

    // === Cross-Module Pipeline Benchmark ===

    #[test]
    fn bench_full_pipeline_v1_6_sprint1() {
        let start = Instant::now();

        // Bridge: 50 messages
        let mut bridge = CrossChainBridgeV3::default();
        bridge.register_chain("a".to_string(), 0.95, 500.0).unwrap();
        bridge.register_chain("b".to_string(), 0.90, 400.0).unwrap();
        for i in 0..50 {
            let id = bridge
                .submit_message("a", "b", format!("msg_{}", i).into_bytes())
                .unwrap();
            bridge.verify_message(&id).unwrap();
            bridge.add_signature(&id, "s1".to_string()).unwrap();
            bridge.add_signature(&id, "s2".to_string()).unwrap();
            bridge.add_signature(&id, "s3".to_string()).unwrap();
            bridge.relay_message(&id).unwrap();
        }

        // Interop: 8 federations, 50 routes
        let mut interop = InteropLayerV2::default();
        for i in 0..8 {
            interop
                .register_federation(format!("fed_{}", i), vec![format!("ep_{}", i)])
                .unwrap();
        }
        for i in 0..7 {
            interop
                .add_connection(&format!("fed_{}", i), &format!("fed_{}", i + 1))
                .unwrap();
        }
        for i in 0..50 {
            let msg = InteropMessage {
                message_id: format!("route_{}", i),
                source: "fed_0".to_string(),
                destination: "fed_7".to_string(),
                payload: format!("route_{}", i).into_bytes(),
                schema_version: 1,
                compressed: false,
            };
            interop.route_message(msg).unwrap();
        }

        // State sync: 200 keys
        let mut sync_a = StateSyncV2::default();
        let mut peer_state = HashMap::new();
        for i in 0..200 {
            let key = format!("k_{}", i);
            let value = vec![i as u8; 8];
            sync_a.register_state(key.clone(), value.clone()).unwrap();
            peer_state.insert(key.clone(), StateEntry::new(key, value));
        }
        sync_a.sync_state(&peer_state);

        // Snapshot: 10 snapshots
        let config = SnapshotConfig {
            max_snapshots: 20,
            enable_compression: true,
            enable_merkle_verification: true,
            snapshot_ttl_ms: 0,
        };
        let mut manager = SnapshotManager::new(config);
        for i in 0..10 {
            let mut state = HashMap::new();
            for j in 0..100 {
                state.insert(format!("s_{}", j), vec![i as u8, j as u8]);
            }
            manager
                .create_snapshot(format!("snap_{}", i), state, None)
                .unwrap();
        }

        let elapsed = start.elapsed();
        eprintln!(
            "bench_full_pipeline_v1_6_sprint1: {:?} (50 bridge + 50 routes + 200 sync + 10 snapshots)",
            elapsed
        );
        assert!(elapsed.as_millis() < 500);
    }
}
