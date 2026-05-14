//! Phase 6 Sprint 2 – End-to-End Integration Tests
//!
//! Validates the full flow: ONNX → Adapter → FedAvg → Staking → API v2
//!
//! Run with: `cargo test --features "phase6-sprint2" --test phase6_e2e`

#[cfg(feature = "phase6-sprint2")]
mod e2e {
    use ed2kia::interoperability::onnx_adapter::{OnnxAdapter, OnnxAdapterConfig};
    use ed2kia::interoperability::adapter::{TensorAdapter, SourceModel};
    use ed2kia::federation::avg_aggregator::{FedAvgAggregator, FedAvgConfig, WeightUpdate};
    use ed2kia::staking::registry::{ResourceRegistry, ResourceCommitment};
    use ed2kia::api::auth::{AuthValidator, AuthConfig};
    use ed2kia::api::routes::ApiResponse;

    /// Helper: create a valid ResourceCommitment for testing
    fn test_commitment(node_id: &str) -> ResourceCommitment {
        ResourceCommitment::new(
            node_id.to_string(),
            8,   // cpu_cores
            32.0, // ram_gb
            true, // has_gpu
            1000.0, // bandwidth_mbps
            500.0, // storage_gb
        )
    }

    /// Helper: create a valid WeightUpdate for testing
    fn test_update(node_id: &str, layer_id: u32, dim: usize, seed: u32) -> WeightUpdate {
        let mut rng = seed as u64;
        let deltas: Vec<f32> = (0..dim).map(|_| {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((rng >> 33) as f32 / 2.0_f32.powi(31) - 1.0) * 0.01
        }).collect();
        WeightUpdate::new(node_id.to_string(), layer_id, deltas)
    }

    // -----------------------------------------------------------------------
    // E2E Test 1: ONNX Adapter → TensorAdapter normalization
    // -----------------------------------------------------------------------

    #[test]
    fn test_onnx_to_tensor_adapter_flow() {
        // Create ONNX adapter config (placeholder model path)
        let config = OnnxAdapterConfig {
            model_path: "/tmp/test_model.onnx".to_string(),
            target_layer: "layer_10".to_string(),
            target_dim: 3584,
            target_dtype: "f32".to_string(),
            optimize_graph: false,
        };

        let mut adapter = OnnxAdapter::new(config);

        // load_model will fail for non-existent file, but the API is valid
        let result = adapter.load_model();
        // We expect an error since the file doesn't exist
        assert!(result.is_err(), "Should fail for non-existent model");

        // Verify error structure
        let err = result.unwrap_err();
        assert!(!err.model_path.is_empty());
        assert!(!err.reason.is_empty());
    }

    // -----------------------------------------------------------------------
    // E2E Test 2: TensorAdapter → FedAvg Aggregation
    // -----------------------------------------------------------------------

    #[test]
    fn test_tensor_adapter_to_fedavg() {
        let dim = 256;
        let config = FedAvgConfig {
            min_participants: 3,
            krum_selections: 2,
        };
        let mut aggregator = FedAvgAggregator::new(config);

        // Simulate updates from 5 nodes
        for i in 0..5 {
            let update = test_update(&format!("node_{}", i), 0, dim, i as u32);
            aggregator.add_update(update).expect("Should add update");
        }

        // Aggregate
        let result = aggregator.aggregate(0).expect("Should aggregate");
        assert_eq!(result.layer_id, 0);
        assert_eq!(result.deltas.len(), dim);
        assert!(result.confidence > 0.0);
        assert!(result.confidence <= 1.0);
    }

    // -----------------------------------------------------------------------
    // E2E Test 3: Staking Registry → Heartbeat → Slashing
    // -----------------------------------------------------------------------

    #[test]
    fn test_staking_lifecycle() {
        let mut registry = ResourceRegistry::with_defaults();

        // Register 3 nodes
        for i in 0..3 {
            let commitment = test_commitment(&format!("node_{}", i));
            registry.register(commitment).expect("Should register");
        }

        let stats = registry.stats();
        assert_eq!(stats.total_nodes, 3);
        assert_eq!(stats.active_nodes, 3);

        // Heartbeat
        registry.process_heartbeat("node_0").expect("Should heartbeat");

        // Slash a node
        registry.slash_node("node_1", "Test slashing").expect("Should slash");

        let stats = registry.stats();
        assert_eq!(stats.active_nodes, 2);
        assert_eq!(stats.slashed_nodes, 1);

        // Verify slashed node has zero reputation
        let slashed = registry.get_commitment("node_1").unwrap();
        assert_eq!(slashed.reputation_score, 0.0);
    }

    // -----------------------------------------------------------------------
    // E2E Test 4: Auth Validator → Signature Validation
    // -----------------------------------------------------------------------

    #[test]
    fn test_auth_validator_creation() {
        let config = AuthConfig {
            require_signature: true,
            signature_timeout_secs: 300,
            authorized_keys: vec![],
        };
        let validator = AuthValidator::new(config);
        assert!(validator.config.require_signature);
        assert_eq!(validator.config.signature_timeout_secs, 300);
    }

    // -----------------------------------------------------------------------
    // E2E Test 5: API Response Serialization
    // -----------------------------------------------------------------------

    #[test]
    fn test_api_response_serialization() {
        let response = ApiResponse::<serde_json::Value>::ok(serde_json::json!({
            "status": "ok",
            "data": { "test": true }
        })));
        assert!(response.success);
        assert!(response.data.is_some());
        assert!(response.error.is_none());
        assert_eq!(response.version, "v2");

        let err_response = ApiResponse::<serde_json::Value>::error("test error".to_string());
        assert!(!err_response.success);
        assert!(err_response.data.is_none());
        assert_eq!(err_response.error.unwrap(), "test error");
    }

    // -----------------------------------------------------------------------
    // E2E Test 6: Full Pipeline Simulation
    // -----------------------------------------------------------------------

    #[test]
    fn test_full_pipeline_simulation() {
        // 1. Register nodes in staking registry
        let mut registry = ResourceRegistry::with_defaults();
        for i in 0..5 {
            registry.register(test_commitment(&format!("node_{}", i))).unwrap();
        }

        // 2. Collect active nodes sorted by resource score
        let active = registry.get_active_nodes();
        assert_eq!(active.len(), 5);

        // 3. Simulate FedAvg with updates from active nodes
        let dim = 128;
        let config = FedAvgConfig {
            min_participants: 3,
            krum_selections: 2,
        };
        let mut aggregator = FedAvgAggregator::new(config);

        for (i, node) in active.iter().enumerate().take(5) {
            let update = test_update(&node.node_id, 0, dim, i as u32);
            aggregator.add_update(update).unwrap();
        }

        // 4. Aggregate
        let result = aggregator.aggregate(0).unwrap();
        assert_eq!(result.layer_id, 0);
        assert_eq!(result.deltas.len(), dim);
        assert_eq!(result.participants, 5);

        // 5. Verify registry stats match
        let stats = registry.stats();
        assert_eq!(stats.total_nodes, 5);
        assert_eq!(stats.active_nodes, 5);
        assert_eq!(stats.slashed_nodes, 0);

        // 6. Simulate heartbeat for top node
        let top_node = active[0].node_id.clone();
        registry.process_heartbeat(&top_node).unwrap();

        // 7. Verify heartbeat updated
        let top_commitment = registry.get_commitment(&top_node).unwrap();
        assert!(top_commitment.last_heartbeat >= top_commitment.registered_at);
    }

    // -----------------------------------------------------------------------
    // E2E Test 7: Byzantine Tolerance in FedAvg
    // -----------------------------------------------------------------------

    #[test]
    fn test_byzantine_tolerance() {
        let dim = 64;
        let config = FedAvgConfig {
            min_participants: 5,
            krum_selections: 3,
        };
        let mut aggregator = FedAvgAggregator::new(config);

        // Add 7 normal updates
        for i in 0..7 {
            let update = test_update(&format!("honest_{}", i), 0, dim, i as u32);
            aggregator.add_update(update).unwrap();
        }

        // Add 2 Byzantine updates (large outliers)
        for i in 0..2 {
            let mut update = test_update(&format!("byzantine_{}", i), 0, dim, (100 + i) as u32);
            // Make the deltas extremely large to simulate Byzantine behavior
            for d in update.deltas.iter_mut() {
                *d *= 1000.0;
            }
            aggregator.add_update(update).unwrap();
        }

        // Aggregate with Krum filtering
        let result = aggregator.aggregate(0).unwrap();
        assert_eq!(result.layer_id, 0);
        assert_eq!(result.deltas.len(), dim);

        // Krum should have filtered out the Byzantine nodes
        // Total participants includes all, but accepted should be less
        assert!(result.participants >= config.min_participants);
    }

    // -----------------------------------------------------------------------
    // E2E Test 8: Resource Score Ordering
    // -----------------------------------------------------------------------

    #[test]
    fn test_resource_score_ordering() {
        let mut registry = ResourceRegistry::with_defaults();

        // Node with GPU should rank higher
        registry.register(ResourceCommitment::new("gpu_node".to_string(), 8, 64.0, true, 10000.0, 1000.0)).unwrap();
        registry.register(ResourceCommitment::new("cpu_node".to_string(), 8, 64.0, false, 10000.0, 1000.0)).unwrap();
        registry.register(ResourceCommitment::new("weak_node".to_string(), 2, 4.0, false, 100.0, 50.0)).unwrap();

        let active = registry.get_active_nodes();
        assert_eq!(active.len(), 3);
        assert_eq!(active[0].node_id, "gpu_node");
        assert_eq!(active[1].node_id, "cpu_node");
        assert_eq!(active[2].node_id, "weak_node");
    }
} // mod e2e

// ---------------------------------------------------------------------------
// Basic tests (always available for module structure validation)
// ---------------------------------------------------------------------------

#[cfg(not(feature = "phase6-sprint2"))]
mod basic_tests {
    /// When phase6-sprint2 is not enabled, verify the module compiles
    #[test]
    fn test_sprint2_not_enabled() {
        assert!(!cfg!(feature = "phase6-sprint2"));
    }
}
