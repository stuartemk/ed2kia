//! Federated Plasticity Integration Tests — Sprint 30
//!
//! Tests for Neuroplastic Aggregation, Steering Bridge and Async Feedback Queue.
//! Validates CE+Z weighted aggregation, human feedback flow and conflict resolution.

#[cfg(all(
    feature = "v2.1-neuroplasticity",
    feature = "v2.1-steering-bridge",
    feature = "v2.1-quantum-feedback"
))]
mod neuroplastic_aggregation_tests {
    use candle_core::Tensor;
    use ed2kia::alignment::sct_core::StuartianTensor;
    use ed2kia::async_gossip::crdt_symbols::SymbolRegistry;
    use ed2kia::economics::existential_credit::ExistentialCreditLedger;
    use ed2kia::federation::neuroplastic_engine::NeuroplasticAggregator;

    #[test]
    fn test_ce_z_weighted_aggregation() {
        let mut ce = ExistentialCreditLedger::new();
        let mut sct = SymbolRegistry::new("test-node");

        // High CE + positive Z peer
        ce.emit_credit("ethical-peer", 0.9, 200.0).unwrap(); // CE = 180
        let token_a = NeuroplasticAggregator::peer_id_to_token("ethical-peer");
        sct.insert_symbol(token_a, StuartianTensor::new(0.6, 0.4, 0.5).unwrap(), 1000);

        // Low CE + negative Z peer
        ce.emit_credit("weak-peer", 0.1, 50.0).unwrap(); // CE = 5
        let token_b = NeuroplasticAggregator::peer_id_to_token("weak-peer");
        sct.insert_symbol(token_b, StuartianTensor::new(0.3, 0.2, -0.4).unwrap(), 1000);

        let agg = NeuroplasticAggregator::new(ce, sct);

        // Ethical peer should have higher weight
        let weight_ethical = agg.compute_weight("ethical-peer");
        let weight_weak = agg.compute_weight("weak-peer");

        assert!(
            weight_ethical > weight_weak,
            "Ethical peer (CE=180, Z=0.5) should have higher weight than weak peer (CE=5, Z=-0.4). Ethical: {}, Weak: {}",
            weight_ethical, weight_weak
        );

        // Verify weight formulas
        // Ethical: ce_factor = 180/1000 = 0.18, z_factor = 1.0 + 0.5 = 1.5, weight = 0.27
        assert!(
            (weight_ethical - 0.27_f64).abs() < 0.01,
            "Expected ethical weight ~0.27, got {}",
            weight_ethical
        );

        // Weak: ce_factor = 5/1000 = 0.005, z_factor = 1.0 + (-0.4) = 0.6, weight = 0.003
        assert!(
            (weight_weak - 0.003_f64).abs() < 0.001,
            "Expected weak weight ~0.003, got {}",
            weight_weak
        );
    }

    #[test]
    fn test_gradient_scaling_by_ethical_weight() {
        let mut ce = ExistentialCreditLedger::new();
        let sct = SymbolRegistry::new("test-node");

        ce.emit_credit("peer-1", 0.5, 100.0).unwrap(); // CE = 50
        let agg = NeuroplasticAggregator::new(ce, sct);

        let device = candle_core::Device::Cpu;
        let grads = Tensor::from_vec(vec![2.0_f32, 4.0, 6.0, 8.0], 4, &device).unwrap();

        let weighted = agg.aggregate_gradients(&grads, "peer-1").unwrap();
        let result: Vec<f32> = weighted.to_vec1().unwrap();

        // CE = 50, ce_factor = 50/1000 = 0.05, z_factor = 1.0 (no SCT), weight = 0.05
        let expected: Vec<f32> = vec![2.0, 4.0, 6.0, 8.0]
            .into_iter()
            .map(|v| v * 0.05)
            .collect();

        for (i, (a, b)) in result.iter().zip(expected.iter()).enumerate() {
            assert!(
                (a - b).abs() < 0.01,
                "Element {}: expected {}, got {}",
                i,
                b,
                a
            );
        }
    }
}

#[cfg(all(
    feature = "v2.1-neuroplasticity",
    feature = "v2.1-steering-bridge",
    feature = "v2.1-quantum-feedback"
))]
mod steering_bridge_tests {
    use ed25519_dalek::SigningKey;
    use ed2kia::alignment::steering_bridge::SteeringBridge;
    use ed2kia::async_gossip::crdt_symbols::SymbolRegistry;
    use ed2kia::economics::existential_credit::ExistentialCreditLedger;

    fn setup_bridge() -> SteeringBridge {
        let sct = SymbolRegistry::new("test-node");
        let ce = ExistentialCreditLedger::new();
        // Deterministic key generation from 32-byte seed
        let seed = [42u8; 32];
        let signer = SigningKey::from(&seed);
        SteeringBridge::new(sct, ce, signer)
    }

    #[test]
    fn test_steering_bridge_feedback_flow() {
        let sct = SymbolRegistry::new("test-node");
        let ce = ExistentialCreditLedger::new();
        let seed = [42u8; 32];
        let signer = SigningKey::from(&seed);
        let public_key = signer.verifying_key();

        let mut bridge = SteeringBridge::new(sct, ce, signer);

        // Process constructive feedback
        let event = bridge
            .process_feedback("human-1", "reforzar autonomía ética", 42)
            .unwrap();

        // Verify event structure
        assert_eq!(event.token_id, 42);
        assert_eq!(event.peer_id, "human-1");
        assert!(
            event.delta_sct.2 > 0.0,
            "ΔZ should be positive for constructive feedback"
        );
        assert!(!event.signature.is_empty(), "Signature should not be empty");
        assert!(event.timestamp > 0, "Timestamp should be > 0");

        // Verify signature
        assert!(
            SteeringBridge::verify_event(&event, &public_key),
            "Signature should verify with correct public key"
        );

        // Verify CE was emitted
        let ce_score = bridge.ce_ledger().get_score("human-1");
        assert!(
            ce_score > 0.0,
            "CE should be positive after constructive feedback"
        );

        // Verify SCT was updated
        let symbol = bridge.sct_dict().get_symbol(42);
        assert!(symbol.is_some(), "SCT should be updated in registry");
        assert!(
            symbol.unwrap().sct.z > 0.0,
            "Z should be positive after constructive feedback"
        );
    }

    #[test]
    fn test_destructive_feedback_burns_ce() {
        let sct = SymbolRegistry::new("test-node");
        let mut ce = ExistentialCreditLedger::new();
        let seed = [42u8; 32];
        let signer = SigningKey::from(&seed);

        // Pre-load CE
        ce.emit_credit("human-2", 0.8, 100.0).unwrap(); // CE = 80

        let mut bridge = SteeringBridge::new(sct, ce, signer);

        // Process destructive feedback
        bridge
            .process_feedback("human-2", "rechazar manipulación", 99)
            .unwrap();

        // CE should be reduced
        let ce_score = bridge.ce_ledger().get_score("human-2");
        assert!(
            ce_score < 80.0,
            "CE should be reduced after destructive feedback. Got: {}",
            ce_score
        );
    }

    #[test]
    fn test_signature_tampering_detection() {
        let sct = SymbolRegistry::new("test-node");
        let ce = ExistentialCreditLedger::new();
        let seed = [42u8; 32];
        let signer = SigningKey::from(&seed);
        let public_key = signer.verifying_key();

        let mut bridge = SteeringBridge::new(sct, ce, signer);
        let mut event = bridge
            .process_feedback("human-3", "reforzar etico", 77)
            .unwrap();

        // Tamper with signature
        event.signature[0] ^= 0xFF;

        assert!(
            !SteeringBridge::verify_event(&event, &public_key),
            "Tampered signature should fail verification"
        );
    }
}

#[cfg(all(
    feature = "v2.1-neuroplasticity",
    feature = "v2.1-steering-bridge",
    feature = "v2.1-quantum-feedback"
))]
mod async_feedback_tests {
    use ed2kia::alignment::steering_bridge::SteeringEvent;
    use ed2kia::economics::existential_credit::ExistentialCreditLedger;
    use ed2kia::quantum_feedback::AsyncFeedbackQueue;

    fn make_event(token_id: u32, peer_id: &str, delta_z: f32, timestamp: u64) -> SteeringEvent {
        SteeringEvent {
            token_id,
            delta_sct: (0.05, 0.05, delta_z),
            signature: vec![1, 2, 3, 4],
            timestamp,
            peer_id: peer_id.to_string(),
            feedback_text: "test feedback".to_string(),
        }
    }

    #[test]
    fn test_async_feedback_conflict_resolution() {
        let mut ce = ExistentialCreditLedger::new();
        ce.emit_credit("peer-high", 0.9, 200.0).unwrap(); // CE = 180
        ce.emit_credit("peer-low", 0.1, 50.0).unwrap(); // CE = 5

        let mut queue = AsyncFeedbackQueue::new("node-1", ce);

        // Low priority event first (CE=5, Z=0.2 → priority=1.0)
        let event_low = make_event(42, "peer-low", 0.2, 1000);
        queue.enqueue(event_low).unwrap();

        // High priority event (CE=180, Z=0.3 → priority=54.0)
        let event_high = make_event(42, "peer-high", 0.3, 2000);
        queue.enqueue(event_high).unwrap();

        // High priority should win
        assert_eq!(queue.len(), 1);
        let entry = queue.get(42).unwrap();
        assert_eq!(
            entry.event.peer_id, "peer-high",
            "Higher CE*Z priority should win"
        );
        assert_eq!(entry.event.timestamp, 2000);
    }

    #[test]
    fn test_async_feedback_lww_tiebreaker() {
        let mut ce = ExistentialCreditLedger::new();
        ce.emit_credit("peer-a", 0.5, 100.0).unwrap();

        let mut queue = AsyncFeedbackQueue::new("node-1", ce);

        // Same peer, same Z → same priority
        let event1 = make_event(42, "peer-a", 0.2, 1000);
        queue.enqueue(event1).unwrap();

        let event2 = make_event(42, "peer-a", 0.2, 2000);
        queue.enqueue(event2).unwrap();

        // Newer timestamp should win (LWW)
        assert_eq!(queue.len(), 1);
        let entry = queue.get(42).unwrap();
        assert_eq!(
            entry.event.timestamp, 2000,
            "Newer timestamp should win when priority is equal"
        );
    }

    #[test]
    fn test_multi_node_sync_convergence() {
        let mut ce_a = ExistentialCreditLedger::new();
        let mut ce_b = ExistentialCreditLedger::new();

        ce_a.emit_credit("shared-peer", 0.8, 100.0).unwrap();
        ce_b.emit_credit("shared-peer", 0.8, 100.0).unwrap();

        let mut queue_a = AsyncFeedbackQueue::new("node-a", ce_a);
        let mut queue_b = AsyncFeedbackQueue::new("node-b", ce_b);

        // Queue A has token 42
        queue_a
            .enqueue(make_event(42, "shared-peer", 0.2, 1000))
            .unwrap();

        // Queue B has token 99
        queue_b
            .enqueue(make_event(99, "shared-peer", 0.3, 1000))
            .unwrap();

        // Sync both directions
        queue_a.sync_with_peer(&queue_b);
        queue_b.sync_with_peer(&queue_a);

        // Both should have both tokens (convergence)
        assert_eq!(queue_a.len(), 2);
        assert_eq!(queue_b.len(), 2);
        assert!(queue_a.get(42).is_some());
        assert!(queue_a.get(99).is_some());
        assert!(queue_b.get(42).is_some());
        assert!(queue_b.get(99).is_some());
    }

    #[test]
    fn test_sync_priority_resolution() {
        let mut ce_a = ExistentialCreditLedger::new();
        let mut ce_b = ExistentialCreditLedger::new();

        ce_a.emit_credit("peer-high", 0.9, 200.0).unwrap(); // CE = 180
        ce_a.emit_credit("peer-low", 0.1, 50.0).unwrap(); // CE = 5
        ce_b.emit_credit("peer-high", 0.9, 200.0).unwrap();
        ce_b.emit_credit("peer-low", 0.1, 50.0).unwrap();

        let mut queue_a = AsyncFeedbackQueue::new("node-a", ce_a);
        let mut queue_b = AsyncFeedbackQueue::new("node-b", ce_b);

        // Queue A: low priority
        queue_a
            .enqueue(make_event(42, "peer-low", 0.1, 1000))
            .unwrap();

        // Queue B: high priority
        queue_b
            .enqueue(make_event(42, "peer-high", 0.3, 1000))
            .unwrap();

        // Sync B into A — B's entry should win
        queue_a.sync_with_peer(&queue_b);

        let entry = queue_a.get(42).unwrap();
        assert_eq!(
            entry.event.peer_id, "peer-high",
            "Higher priority from peer should win during sync"
        );
    }

    #[test]
    fn test_queue_drain_and_rebuild() {
        let mut ce = ExistentialCreditLedger::new();
        ce.emit_credit("peer-a", 0.5, 100.0).unwrap();

        let mut queue = AsyncFeedbackQueue::new("node-1", ce);

        queue.enqueue(make_event(42, "peer-a", 0.2, 1000)).unwrap();
        queue.enqueue(make_event(99, "peer-a", 0.3, 1000)).unwrap();
        queue.enqueue(make_event(123, "peer-a", 0.1, 1000)).unwrap();

        assert_eq!(queue.len(), 3);

        let drained = queue.drain();
        assert_eq!(drained.len(), 3);
        assert!(queue.is_empty());

        // Re-enqueue drained events
        for entry in drained {
            queue.enqueue(entry.event).unwrap();
        }
        assert_eq!(queue.len(), 3);
    }
}
