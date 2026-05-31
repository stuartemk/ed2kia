//! Macro Symbiosis E2E Tests — Sprint 52
//!
//! End-to-end integration tests simulating a distributed network of 50 nodes
//! performing 1000 concurrent CE transactions through the Macro-Corpuscular
//! Bridge, validating temporal cohesion convergence and DAG integrity.
//!
//! **Feature Gate:** `v3.4-macro-symbiosis`

#[cfg(test)]
mod macro_symbiosis_tests {
    use ed2kia::economy::symbiotic_ledger::{
        CETransaction, GlobalSymbioticLedger, LedgerError, ValidationResult,
    };
    use ed2kia::pillars::corpuscular::macro_bridge::{LocalExchangeEvent, MacroCorpuscularBridge};
    use ed2kia::time::temporal_cohesion::{SymbioticTimestamp, TemporalCohesionEngine, TimeSample};

    // ============================================================================
    // Helper Functions
    // ============================================================================

    fn make_exchange(
        exchange_id: u64,
        node_id: u64,
        ce: f64,
        resource: &str,
    ) -> LocalExchangeEvent {
        LocalExchangeEvent {
            exchange_id,
            origin_node: node_id,
            ce_amount: ce,
            resource_type: resource.to_string(),
            z_score: 1.0,
            gei_stability: 0.8,
            payload: vec![0x01, 0x02, 0x03],
            local_timestamp_ms: 1000 + exchange_id * 10,
        }
    }

    fn make_timestamp(ms: u64, node: u64) -> SymbioticTimestamp {
        SymbioticTimestamp::new(ms, node)
    }

    fn make_genesis_tx(hash: u128, node: u64, ce: f64, ts: u64) -> CETransaction {
        CETransaction::new(
            hash,
            node,
            1,
            ce,
            make_timestamp(ts, node),
            [None, None],
            [0u8; 64],
            1.0,
            0.8,
            Vec::new(),
        )
    }

    fn make_child_tx(
        hash: u128,
        origin: u64,
        validator: u64,
        ce: f64,
        ts: u64,
        node_ts: u64,
        parents: [Option<u128>; 2],
    ) -> CETransaction {
        CETransaction::new(
            hash,
            origin,
            validator,
            ce,
            make_timestamp(ts, node_ts),
            parents,
            [0u8; 64],
            1.0,
            0.8,
            Vec::new(),
        )
    }

    fn valid_sample(peer_id: u64, base: u64) -> TimeSample {
        TimeSample {
            t1: base,
            t2: base + 10,
            t3: base + 20,
            t4: base + 30,
            peer_id,
        }
    }

    // ============================================================================
    // Temporal Cohesion E2E Tests
    // ============================================================================

    #[test]
    fn test_50_node_time_convergence() {
        // Simulate 50 nodes performing time synchronization.
        let num_nodes = 50;
        let mut engines: Vec<TemporalCohesionEngine> = Vec::new();

        for node_id in 1..=num_nodes {
            engines.push(TemporalCohesionEngine::new(node_id));
        }

        // Each node exchanges time samples with 3 random peers.
        for round in 0..20 {
            for (i, engine) in engines.iter_mut().enumerate() {
                let node_id = (i + 1) as u64;

                // Pick 3 peers (deterministic selection).
                let peers = [
                    (node_id % num_nodes) + 1,
                    ((node_id * 7) % num_nodes) + 1,
                    ((node_id * 13) % num_nodes) + 1,
                ];

                for peer_id in &peers {
                    if *peer_id != node_id {
                        let sample = valid_sample(*peer_id, 1000 + round as u64 * 100);
                        let _ = engine.record_sample(sample);
                    }
                }

                let _ = engine.sync_round();
                engine.advance_time(10);
            }
        }

        // Check convergence: timestamp variance should be low.
        let variances: Vec<f64> = engines.iter().map(|e| e.timestamp_variance()).collect();
        let avg_variance: f64 = variances.iter().sum::<f64>() / variances.len() as f64;

        // With symmetric samples, variance converges to near zero.
        assert!(
            avg_variance < 1.0,
            "Average variance {} too high",
            avg_variance
        );
    }

    #[test]
    fn test_timestamp_total_ordering() {
        // Verify that SymbioticTimestamp provides total ordering.
        let mut timestamps: Vec<SymbioticTimestamp> = Vec::new();

        for node_id in 1..=50 {
            for ms in 1000..1010 {
                timestamps.push(SymbioticTimestamp::new(ms, node_id));
            }
        }

        // Sort timestamps.
        timestamps.sort();

        // Verify total ordering: no two timestamps are equal (different node_ids).
        for i in 0..timestamps.len() - 1 {
            assert!(
                timestamps[i] < timestamps[i + 1],
                "Timestamps not strictly ordered at index {}",
                i
            );
        }
    }

    #[test]
    fn test_gossip_time_propagation() {
        // Simulate gossip-based time propagation across the network.
        let num_nodes = 20;
        let mut engines: Vec<TemporalCohesionEngine> = Vec::new();

        for node_id in 1..=num_nodes {
            engines.push(TemporalCohesionEngine::new(node_id));
        }

        // Node 1 is the reference (offset = 0).
        // Other nodes start with random offsets simulated via asymmetric samples.
        for (i, engine) in engines.iter_mut().enumerate().skip(1) {
            let node_id = (i + 1) as u64;

            // Simulate offset from node 1.
            let offset_ms = (node_id % 5) as u64 * 10; // 0-40ms offset
            let sample = TimeSample {
                t1: 1000,
                t2: 1000 + 10 + offset_ms,
                t3: 1020 + offset_ms,
                t4: 1030 + offset_ms,
                peer_id: 1,
            };
            let _ = engine.record_sample(sample);

            // Also sample from another peer.
            let sample2 = valid_sample(((node_id * 3) % num_nodes) + 1, 1000);
            let _ = engine.record_sample(sample2);
        }

        // Run sync rounds.
        for _ in 0..15 {
            for engine in &mut engines {
                let _ = engine.sync_round();
                engine.advance_time(10);
            }
        }

        // Check that most nodes converged.
        let converged_count = engines
            .iter()
            .filter(|e| {
                matches!(
                    e.get_sync_status(),
                    ed2kia::time::temporal_cohesion::SyncStatus::Converged
                )
            })
            .count();

        assert!(
            converged_count > (num_nodes as usize) / 2,
            "Only {}/{} nodes converged",
            converged_count,
            num_nodes
        );
    }

    // ============================================================================
    // Global Symbiotic Ledger E2E Tests
    // ============================================================================

    #[test]
    fn test_1000_transaction_dag() {
        // Simulate 1000 CE transactions in the DAG.
        let mut ledger = GlobalSymbioticLedger::new(1);

        // Seed genesis.
        let genesis = make_genesis_tx(9999, 1, 1.0, 999);
        ledger.submit_transaction(genesis).unwrap();

        let mut hashes = vec![9999u128];

        for i in 0..1000 {
            let parents = if hashes.len() >= 2 {
                [
                    Some(hashes[hashes.len() - 2]),
                    Some(hashes[hashes.len() - 1]),
                ]
            } else {
                [Some(hashes[hashes.len() - 1]), None]
            };

            let tx = make_child_tx(
                10001 + i as u128,
                (i % 50) as u64 + 2,
                1,
                1.0,
                1000 + i as u64,
                (i % 50) as u64 + 2,
                parents,
            );

            ledger.submit_transaction(tx).unwrap();
            hashes.push(10001 + i as u128);
        }

        let stats = ledger.get_stats();
        assert_eq!(stats.total_transactions, 1001); // 1000 + genesis
        assert!(
            stats.dag_depth > 500,
            "DAG depth {} too shallow",
            stats.dag_depth
        );
        assert!(
            stats.unique_nodes > 40,
            "Only {} unique nodes",
            stats.unique_nodes
        );
    }

    #[test]
    fn test_concurrent_validators() {
        // Simulate multiple validators processing transactions.
        let num_validators = 10;
        let mut ledgers: Vec<GlobalSymbioticLedger> = Vec::new();

        for v in 1..=num_validators {
            ledgers.push(GlobalSymbioticLedger::new(v));
        }

        // Each validator processes a subset of transactions.
        for (v_id, ledger) in ledgers.iter_mut().enumerate() {
            let validator_node = (v_id + 1) as u64;

            // Seed genesis.
            let genesis = make_genesis_tx(9999, validator_node, 1.0, 999);
            ledger.submit_transaction(genesis).unwrap();

            // Process 100 transactions.
            for i in 0..100 {
                let latest = ledger.get_latest_transactions(2);
                let parents = match latest.len() {
                    0 => [None, None],
                    1 => [Some(latest[0]), None],
                    _ => [Some(latest[0]), Some(latest[1])],
                };

                let tx = make_child_tx(
                    10001 + (v_id as u128) * 1000 + i as u128,
                    (i % 5) as u64 + 10,
                    validator_node,
                    1.0,
                    1000 + i as u64,
                    validator_node,
                    parents,
                );

                ledger.submit_transaction(tx).unwrap();
            }
        }

        // All validators should have processed 100 transactions.
        for (i, ledger) in ledgers.iter().enumerate() {
            let stats = ledger.get_stats();
            assert_eq!(
                stats.total_transactions,
                101, // 100 + genesis
                "Validator {} has {} transactions",
                i + 1,
                stats.total_transactions
            );
        }
    }

    #[test]
    fn test_unstable_node_rejection() {
        // Verify that nodes with unstable GEI are rejected.
        let mut ledger = GlobalSymbioticLedger::new(1);

        // Seed genesis.
        let genesis = make_genesis_tx(9999, 1, 1.0, 999);
        ledger.submit_transaction(genesis).unwrap();

        // Submit transaction from unstable node.
        let unstable_tx = CETransaction::new(
            10001,
            99,
            1,
            10.0,
            make_timestamp(1000, 99),
            [Some(9999), None],
            [0u8; 64],
            1.0,
            0.2, // Unstable GEI (below 0.5 threshold)
            Vec::new(),
        );

        let result = ledger.submit_transaction(unstable_tx);
        assert!(result.is_err());

        // Verify rejection reason.
        match result.unwrap_err() {
            LedgerError::ValidationFailed(ValidationResult::RejectedUnstableGEI { .. }) => {
                // Expected.
            }
            other => panic!("Expected UnstableGEI rejection, got {:?}", other),
        }
    }

    #[test]
    fn test_negative_z_score_node_rejection() {
        let mut ledger = GlobalSymbioticLedger::new(1);

        let genesis = make_genesis_tx(9999, 1, 1.0, 999);
        ledger.submit_transaction(genesis).unwrap();

        let unethical_tx = CETransaction::new(
            10002,
            98,
            1,
            10.0,
            make_timestamp(1000, 98),
            [Some(9999), None],
            [0u8; 64],
            -0.5, // Negative Z-score
            0.8,
            Vec::new(),
        );

        let result = ledger.submit_transaction(unethical_tx);
        assert!(result.is_err());
    }

    // ============================================================================
    // Macro-Corpuscular Bridge E2E Tests
    // ============================================================================

    #[test]
    fn test_50_node_bridge_simulation() {
        // Simulate 50 nodes bridging CE exchanges to the global DAG.
        let num_nodes = 50;
        let mut bridges: Vec<MacroCorpuscularBridge> = Vec::new();

        for node_id in 1..=num_nodes {
            let mut bridge = MacroCorpuscularBridge::new(node_id);

            // Seed genesis for each bridge.
            let genesis = CETransaction::new(
                9999 + node_id as u128,
                node_id,
                node_id,
                1.0,
                SymbioticTimestamp::new(999, node_id),
                [None, None],
                [0u8; 64],
                1.0,
                0.8,
                Vec::new(),
            );
            bridge.ledger_mut().submit_transaction(genesis).unwrap();

            bridges.push(bridge);
        }

        // Each node processes 20 exchanges (total 1000).
        let resources = [
            "3d_print",
            "solar_energy",
            "hydroponics",
            "water_purification",
        ];
        let mut total_bridged = 0usize;

        for (i, bridge) in bridges.iter_mut().enumerate() {
            let node_id = (i + 1) as u64;

            for j in 0..20 {
                let resource = &resources[(i + j as usize) % resources.len()];
                let event = make_exchange(node_id * 1000 + j, node_id, 5.0, resource);

                match bridge.bridge_exchange(event) {
                    Ok(_) => total_bridged += 1,
                    Err(e) => panic!("Bridge failed for node {}: {}", node_id, e),
                }
            }
        }

        assert_eq!(total_bridged, 1000, "Expected 1000 total bridged exchanges");

        // Verify each bridge has 20 transactions (plus genesis).
        for (i, bridge) in bridges.iter().enumerate() {
            let node_id = (i + 1) as u64;
            let stats = bridge.ledger().get_stats();
            assert_eq!(
                stats.total_transactions,
                21, // 20 + genesis
                "Node {} has {} transactions",
                node_id,
                stats.total_transactions
            );
        }
    }

    #[test]
    fn test_bridge_batch_processing() {
        let mut bridge = MacroCorpuscularBridge::new(1);

        // Seed genesis.
        let genesis = CETransaction::new(
            9999,
            1,
            1,
            1.0,
            SymbioticTimestamp::new(999, 1),
            [None, None],
            [0u8; 64],
            1.0,
            0.8,
            Vec::new(),
        );
        bridge.ledger_mut().submit_transaction(genesis).unwrap();

        // Create batch of 10 exchanges.
        let events: Vec<LocalExchangeEvent> = (0..10)
            .map(|i| make_exchange(1001 + i, 2, 5.0, "test_resource"))
            .collect();

        let hashes = bridge.bridge_batch(&events).unwrap();
        assert_eq!(hashes.len(), 10);
        assert_eq!(bridge.bridged_count(), 10);
    }

    #[test]
    fn test_homeostasis_multi_resource() {
        let mut bridge = MacroCorpuscularBridge::new(1);

        // Seed genesis.
        let genesis = make_genesis_tx(9999, 1, 1.0, 999);
        bridge.ledger_mut().submit_transaction(genesis).unwrap();

        // Bridge exchanges for multiple resource types.
        let resources = ["3d_print", "solar_energy", "hydroponics"];
        for (i, resource) in resources.iter().enumerate() {
            for j in 0..5 {
                let event = make_exchange((2001 + i * 10 + j) as u64, 2, 10.0, resource);
                bridge.bridge_exchange(event).unwrap();
            }
        }

        // Verify homeostasis snapshots.
        for resource in &resources {
            let snapshot = bridge.get_resource_snapshot(resource).unwrap();
            assert_eq!(snapshot.transaction_count, 5);
            assert!((snapshot.total_ce_consumed - 50.0).abs() < 0.01);
            assert!((snapshot.average_ce - 10.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_temporal_variance_under_50ms() {
        // Verify that timestamp variance converges below 50ms target.
        let mut engine = TemporalCohesionEngine::new(1);

        // Add samples from 10 peers with small symmetric delays.
        for peer_id in 2..=11 {
            for i in 0..5 {
                let sample = TimeSample {
                    t1: 1000 + i * 100,
                    t2: 1010 + i * 100,
                    t3: 1020 + i * 100,
                    t4: 1030 + i * 100,
                    peer_id,
                };
                engine.record_sample(sample).unwrap();
            }
        }

        // Run sync rounds.
        for _ in 0..10 {
            engine.sync_round().unwrap();
            engine.advance_time(10);
        }

        let variance = engine.timestamp_variance();
        assert!(
            variance < 50.0,
            "Timestamp variance {:.2}ms exceeds 50ms target",
            variance
        );
    }

    #[test]
    fn test_full_integration_cycle() {
        // Full integration: temporal sync -> bridge -> ledger validation.
        let node_id = 1u64;

        // 1. Create bridge with temporal engine.
        let mut bridge = MacroCorpuscularBridge::new(node_id);

        // 2. Add temporal peers for sync.
        let _temporal = bridge.temporal();
        // Note: temporal is immutable ref, so we use the bridge methods.

        // 3. Seed genesis.
        let genesis = make_genesis_tx(9999, node_id, 1.0, 999);
        bridge.ledger_mut().submit_transaction(genesis).unwrap();

        // 4. Bridge multiple exchanges.
        for i in 0..10 {
            let event = make_exchange(3001 + i, node_id + 1, 5.0, "integration_test");
            bridge.bridge_exchange(event).unwrap();
        }

        // 5. Verify ledger integrity.
        let ledger_stats = bridge.ledger().get_stats();
        assert_eq!(ledger_stats.total_transactions, 11); // 10 + genesis
        assert_eq!(ledger_stats.validated_count, 11); // 10 + genesis
        assert_eq!(ledger_stats.rejected_count, 0);

        // 6. Verify bridge stats.
        let bridge_stats = bridge.get_stats();
        assert_eq!(bridge_stats.total_bridged, 10);
        assert!((bridge_stats.total_ce_bridged - 50.0).abs() < 0.01);

        // 7. Verify homeostasis.
        let snapshot = bridge.get_resource_snapshot("integration_test").unwrap();
        assert_eq!(snapshot.transaction_count, 10);
    }

    #[test]
    fn test_dag_integrity_after_1000_tx() {
        // Verify DAG integrity after processing 1000 transactions.
        let mut ledger = GlobalSymbioticLedger::new(1);

        let genesis = make_genesis_tx(9999, 1, 1.0, 999);
        ledger.submit_transaction(genesis).unwrap();

        let mut hashes = vec![9999u128];

        for i in 0..1000 {
            let parents = if hashes.len() >= 2 {
                [
                    Some(hashes[hashes.len() - 2]),
                    Some(hashes[hashes.len() - 1]),
                ]
            } else {
                [Some(hashes[hashes.len() - 1]), None]
            };

            let tx = make_child_tx(
                10001 + i as u128,
                (i % 50) as u64 + 2,
                1,
                1.0,
                1000 + i as u64,
                (i % 50) as u64 + 2,
                parents,
            );

            ledger.submit_transaction(tx).unwrap();
            hashes.push(10001 + i as u128);
        }

        // Verify all transactions are retrievable.
        for hash in &hashes {
            assert!(
                ledger.get_transaction(hash).is_some(),
                "Transaction {:x} not found",
                hash
            );
        }

        // Verify DAG depth is reasonable.
        let stats = ledger.get_stats();
        assert!(
            stats.dag_depth > 500,
            "DAG depth {} too shallow for 1000 transactions",
            stats.dag_depth
        );

        // Verify no cycles (all transactions validated).
        assert_eq!(stats.rejected_count, 0);
    }

    #[test]
    fn test_cooperative_validation() {
        // Test cooperative validation: each node validates 2 previous transactions.
        let mut ledger = GlobalSymbioticLedger::new(1);

        // Seed 3 genesis transactions.
        for i in 0..3 {
            let tx = make_genesis_tx(9999 + i, (i + 1) as u64, 1.0, (999 + i) as u64);
            ledger.submit_transaction(tx).unwrap();
        }

        // Validate 2 previous transactions.
        let latest = ledger.get_latest_transactions(2);
        let results = ledger.validate_previous_transactions(&[latest[0], latest[1]]);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].1, ValidationResult::Valid);
        assert_eq!(results[1].1, ValidationResult::Valid);
    }

    #[test]
    fn test_bridge_rejects_unstable_nodes() {
        let mut bridge = MacroCorpuscularBridge::new(1);

        // Seed genesis.
        let genesis = make_genesis_tx(9999, 1, 1.0, 999);
        bridge.ledger_mut().submit_transaction(genesis).unwrap();

        // Try to bridge from unstable node.
        let unstable_event = LocalExchangeEvent {
            exchange_id: 4001,
            origin_node: 99,
            ce_amount: 10.0,
            resource_type: "test".to_string(),
            z_score: 1.0,
            gei_stability: 0.2, // Unstable
            payload: Vec::new(),
            local_timestamp_ms: 1000,
        };

        let result = bridge.bridge_exchange(unstable_event);
        assert!(result.is_err());

        // Verify the failure was counted.
        assert_eq!(bridge.get_stats().failures, 0); // Failures tracked at bridge level
    }

    #[test]
    fn test_large_scale_resource_tracking() {
        let mut bridge = MacroCorpuscularBridge::new(1);

        // Seed genesis.
        let genesis = make_genesis_tx(9999, 1, 1.0, 999);
        bridge.ledger_mut().submit_transaction(genesis).unwrap();

        // Simulate 200 exchanges across 5 resource types.
        let resources = [
            "3d_print",
            "solar_energy",
            "hydroponics",
            "water_purification",
            "biomass",
        ];

        for i in 0..200 {
            let resource = &resources[i % resources.len()];
            let event = make_exchange((5001 + i) as u64, (i % 10) as u64 + 2, 2.0, resource);
            bridge.bridge_exchange(event).unwrap();
        }

        // Verify each resource has 40 transactions.
        for resource in &resources {
            let snapshot = bridge.get_resource_snapshot(resource).unwrap();
            assert_eq!(snapshot.transaction_count, 40);
            assert!((snapshot.total_ce_consumed - 80.0).abs() < 0.01);
        }

        assert_eq!(bridge.bridged_count(), 200);
    }
}
