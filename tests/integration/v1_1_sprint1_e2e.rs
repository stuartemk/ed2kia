//! E2E Integration Tests for ed2kIA v1.1.0 Sprint 1
//!
//! This module validates the complete integration flow across all v1.1-sprint1
//! modules:
//!
//! 1. **FedAvg v2** – Gradient compression + parallel aggregation pipeline
//! 2. **Gradient Compressor** – Top-K sparsity + int8 quantization roundtrip
//! 3. **WASM Sandbox v2** – Module lifecycle with profiling and resource limits
//! 4. **WASM Profiler** – Session management with threshold alerts
//! 5. **Cross-Model Router** – Adaptive routing with fallback chains
//! 6. **Capability Registry** – Model registration and schema negotiation
//!
//! All tests are gated behind `#[cfg(feature = "v1.1-sprint1")]` and can be
//! run independently with:
//!
//! ```bash
//! cargo test --features v1.1-sprint1 --test v1_1_sprint1_e2e
//! ```

#[cfg(feature = "v1.1-sprint1")]
mod imports {
    pub use ed2kia::federation::avg_aggregator::WeightUpdate;
    pub use ed2kia::federation_v2_sprint1::avg_aggregator_v2::{
        AggregationResultV2, FedAvgAggregatorV2, FedAvgConfigV2,
    };
    pub use ed2kia::federation_v2_sprint1::gradient_compressor::GradientCompressor;
    pub use ed2kia::interoperability::capability_registry::{
        CapabilityRegistry, ModelCapability,
    };
    pub use ed2kia::interoperability::cross_model_router::{
        CrossModelRouter, RoutingPriority, RoutingRequest,
    };
    pub use ed2kia::security::wasm_profiler::{ExecutionProfile, Profiler, ProfilingAlert};
    pub use ed2kia::security::wasm_sandbox_v2::{
        SandboxConfigV2, WasmSandboxV2,
    };
}

#[cfg(feature = "v1.1-sprint1")]
mod tests {
    use super::imports::*;

    // ---------------------------------------------------------------------------
    // Helper: Create a WeightUpdate with deterministic deltas
    // ---------------------------------------------------------------------------

    fn make_update(node_id: &str, layer_id: u32, dim: usize, seed: u32) -> WeightUpdate {
        let deltas: Vec<f32> = (0..dim)
            .map(|i| ((i + seed as usize) % 100) as f32 / 50.0 - 1.0)
            .collect();
        WeightUpdate::new(node_id.to_string(), layer_id, deltas, 100, 0.5)
    }

    // ---------------------------------------------------------------------------
    // Helper: Generate a valid WASM module with an exported "run" function
    // ---------------------------------------------------------------------------

    fn wasm_with_run_function() -> Vec<u8> {
        // Minimal WASM: magic + version + type + function + export + code
        [
            0x00, 0x61, 0x73, 0x6D, // magic "\0asm"
            0x01, 0x00, 0x00, 0x00, // version 1.0.0
            // Type section: 1 func type () -> ()
            0x01, 0x04, 0x01, 0x60, 0x00, 0x00,
            // Function section: 1 func referencing type 0
            0x03, 0x02, 0x01, 0x00,
            // Export section: "run" -> func 0
            0x07, 0x07, 0x01, 0x03, b'r', b'u', b'n', 0x00, 0x00,
            // Code section: 1 function body (empty)
            0x0A, 0x04, 0x01, 0x02, 0x00, 0x0B,
        ]
        .to_vec()
    }

    // =========================================================================
    // Test 1: test_fedavg_v2_full_pipeline
    // =========================================================================
    // Creates weight updates from multiple nodes, compresses via FedAvg v2,
    // aggregates, and verifies the result contains valid weights with
    // compression metrics.
    // =========================================================================

    #[test]
    fn test_fedavg_v2_full_pipeline() {
        let config = FedAvgConfigV2 {
            min_participants: 2,
            krum_f: 1,
            compression_enabled: true,
            top_k_sparsity: 32,
            quantization_bits: 8,
            parallel_layers: 4,
            min_participation_fraction: 0.4,
        };
        let mut agg = FedAvgAggregatorV2::new(config);

        // Create updates from 5 federated nodes
        for i in 0..5 {
            let update = make_update(&format!("node_{}", i), 0, 100, i as u32);
            agg.add_update(update)
                .expect("Should accept valid weight update");
        }

        // Aggregate layer 0
        let result = agg.aggregate(0).expect("Aggregation should succeed");

        // Verify result structure
        assert_eq!(result.layer_id, 0, "Layer ID should match input");
        assert_eq!(result.final_weights.len(), 100, "Final weights should have correct dimension");
        assert!(
            result.accepted_updates >= 2,
            "At least min_participants should be accepted, got {}",
            result.accepted_updates
        );
        assert!(
            result.compression_ratio > 0.0 && result.compression_ratio <= 1.0,
            "Compression ratio should be in (0, 1], got {}",
            result.compression_ratio
        );
        assert!(
            result.bytes_saved > 0,
            "Bytes saved should be positive with compression enabled, got {}",
            result.bytes_saved
        );
        assert!(
            result.aggregation_latency_ms >= 0.0,
            "Aggregation latency should be non-negative, got {}",
            result.aggregation_latency_ms
        );
        assert!(
            result.confidence > 0.0 && result.confidence <= 1.0,
            "Confidence should be in (0, 1], got {}",
            result.confidence
        );
    }

    // =========================================================================
    // Test 2: test_gradient_compression_roundtrip
    // =========================================================================
    // Compresses a gradient vector using Top-K + int8 quantization, then
    // decompresses and verifies reconstruction accuracy > 99% for selected
    // indices.
    // =========================================================================

    #[test]
    fn test_gradient_compression_roundtrip() {
        // Create a gradient vector with known values
        let dim = 1000;
        let deltas: Vec<f32> = (0..dim)
            .map(|i| ((i % 100) as f32 / 50.0 - 1.0) * (1.0 + (i % 7) as f32 * 0.1))
            .collect();

        let k = 100; // Keep top 100 elements (10% sparsity)

        // Compress using static methods
        let compressed = GradientCompressor::compress_and_quantize(&deltas, k);

        assert_eq!(compressed.original_dim, dim, "Original dimension should be preserved");
        assert_eq!(compressed.data.len(), k, "Compressed data should have k elements");
        assert_eq!(compressed.indices.len(), k, "Indices should have k elements");
        assert!(
            (compressed.compression_ratio - k as f32 / dim as f32).abs() < 1e-5,
            "Compression ratio should match k/dim"
        );

        // Decompress using static method
        let reconstructed = GradientCompressor::decompress_full(&compressed);

        assert_eq!(reconstructed.len(), dim, "Reconstructed vector should have original dimension");

        // Verify accuracy for selected indices
        let mut correct = 0usize;
        let mut total_checked = 0usize;

        for &idx in &compressed.indices {
            let orig: f32 = deltas[idx];
            let recon: f32 = reconstructed[idx];
            total_checked += 1;

            // Allow small quantization error (< 2% relative error)
            if orig.abs() > 1e-6 {
                let rel_err = (orig - recon).abs() / orig.abs();
                if rel_err < 0.02 {
                    correct += 1;
                }
            } else {
                // Near-zero values should reconstruct near-zero
                if recon.abs() < 0.01 {
                    correct += 1;
                }
            }
        }

        let accuracy = correct as f32 / total_checked as f32;
        assert!(
            accuracy > 0.99,
            "Roundtrip accuracy {:.2}% should be > 99% for selected indices",
            accuracy * 100.0
        );
    }

    // =========================================================================
    // Test 3: test_wasm_sandbox_v2_lifecycle
    // =========================================================================
    // Loads a WASM module, executes a function, profiles the execution,
    // and verifies resource limits are enforced.
    // =========================================================================

    #[test]
    fn test_wasm_sandbox_v2_lifecycle() {
        let config = SandboxConfigV2 {
            memory_limit_bytes: 64 * 1024 * 1024, // 64MB
            fuel_limit: 1_000_000,
            fallback_threshold_percent: 80.0,
            max_modules: 10,
            enable_profiling: true,
        };
        let mut sandbox = WasmSandboxV2::new(config);

        // Load module
        let wasm = wasm_with_run_function();
        let module_id = sandbox
            .load_module(&wasm)
            .expect("Should load valid WASM module");

        assert_eq!(module_id.size_bytes, wasm.len(), "Module size should match input");

        // Execute function
        let result = sandbox
            .execute(&module_id.id, "run", Vec::new())
            .expect("Should execute 'run' function");

        // Verify execution result
        assert!(
            !result.fallback_triggered,
            "Fallback should not be triggered for simple module"
        );
        assert!(
            result.profile.wall_time_ms >= 0.0,
            "Wall time should be non-negative"
        );

        // Get profile
        let profile = sandbox
            .get_profile(&module_id.id)
            .expect("Profile should exist after execution");

        assert!(
            profile.wall_time_ms >= 0.0,
            "Profiled wall time should be non-negative"
        );

        // List modules
        let modules = sandbox.list_modules();
        assert_eq!(modules.len(), 1, "Should have exactly 1 loaded module");

        // Remove module
        let removed = sandbox
            .remove_module(&module_id.id);
        assert!(removed, "Should successfully remove loaded module");

        let modules = sandbox.list_modules();
        assert_eq!(modules.len(), 0, "Should have 0 modules after removal");
    }

    // =========================================================================
    // Test 4: test_wasm_profiler_thresholds
    // =========================================================================
    // Starts a profiling session, records metrics that exceed thresholds,
    // and verifies the correct alert is triggered.
    // =========================================================================

    #[test]
    fn test_wasm_profiler_thresholds() {
        let memory_limit = 1_000_000usize; // 1MB
        let fuel_limit = 100_000u64;
        let mut profiler = Profiler::new(memory_limit, fuel_limit);

        // Start session
        let session = profiler.start_session();

        // Record memory at 85% of limit (should trigger MemoryHigh)
        profiler.record_memory(&session, 850_000);
        profiler.record_fuel(&session, 50_000);
        profiler.record_time(&session, 100.0);

        // Finalize session
        let profile = profiler
            .finalize_session(&session)
            .expect("Session should finalize");

        assert_eq!(profile.memory_bytes_peak, 850_000, "Peak memory should be 850KB");
        assert_eq!(profile.fuel_consumed, 50_000, "Fuel consumed should be 50K");
        assert!((profile.wall_time_ms - 100.0).abs() < 0.01, "Wall time should be 100ms");

        // Check thresholds - should trigger MemoryHigh (> 80%)
        let alert = profiler.check_thresholds(&profile);
        match alert {
            ProfilingAlert::MemoryHigh(percent) => {
                assert!(
                    (percent - 85.0).abs() < 0.1,
                    "MemoryHigh alert should report ~85%, got {}",
                    percent
                );
            }
            other => panic!(
                "Expected MemoryHigh alert, got {:?}",
                other
            ),
        }

        // Test MemoryCritical (> 95%)
        let critical_profile = ExecutionProfile {
            memory_bytes_peak: 960_000,
            memory_bytes_current: 960_000,
            cpu_cycles: 0,
            wall_time_ms: 50.0,
            instructions_executed: 0,
            fuel_consumed: 10_000,
        };
        let alert = profiler.check_thresholds(&critical_profile);
        match alert {
            ProfilingAlert::MemoryCritical(percent) => {
                assert!(
                    (percent - 96.0).abs() < 0.1,
                    "MemoryCritical alert should report ~96%, got {}",
                    percent
                );
            }
            other => panic!(
                "Expected MemoryCritical alert, got {:?}",
                other
            ),
        }

        // Test FuelExhausted
        let fuel_profile = ExecutionProfile {
            memory_bytes_peak: 100_000,
            memory_bytes_current: 100_000,
            cpu_cycles: 0,
            wall_time_ms: 50.0,
            instructions_executed: 100_000,
            fuel_consumed: 100_000, // >= fuel_limit
        };
        let alert = profiler.check_thresholds(&fuel_profile);
        assert!(
            matches!(alert, ProfilingAlert::FuelExhausted),
            "Expected FuelExhausted alert, got {:?}",
            alert
        );

        // Test Ok (within limits)
        let ok_profile = ExecutionProfile {
            memory_bytes_peak: 500_000,
            memory_bytes_current: 500_000,
            cpu_cycles: 0,
            wall_time_ms: 10.0,
            instructions_executed: 0,
            fuel_consumed: 50_000,
        };
        let alert = profiler.check_thresholds(&ok_profile);
        assert!(
            matches!(alert, ProfilingAlert::Ok),
            "Expected Ok alert for within-limits profile, got {:?}",
            alert
        );

        // Verify stats
        let stats = profiler.get_stats();
        assert!(
            stats.total_sessions >= 1,
            "Should have at least 1 completed session"
        );
        assert!(
            stats.alerts_triggered >= 3,
            "Should have at least 3 alerts triggered, got {}",
            stats.alerts_triggered
        );
    }

    // =========================================================================
    // Test 5: test_cross_model_routing_direct
    // =========================================================================
    // Registers models with specific capabilities, routes a request, and
    // verifies the optimal model is selected based on latency and budget.
    // =========================================================================

    #[test]
    fn test_cross_model_routing_direct() {
        let mut registry = CapabilityRegistry::new();

        // Register models with different latencies
        registry
            .register(ModelCapability::new(
                "fast_model".to_string(),
                "Qwen-Scope-7B".to_string(),
                "1.0.0".to_string(),
                vec!["sae_forward".to_string(), "embedding".to_string()],
                4096,
                22528,
                32,
                8.0,   // latency p50
                20.0,  // latency p99
                256,   // memory MB
            ))
            .expect("Should register fast_model");

        registry
            .register(ModelCapability::new(
                "slow_model".to_string(),
                "Llama-3-8B".to_string(),
                "1.0.0".to_string(),
                vec!["sae_forward".to_string()],
                4096,
                22528,
                32,
                25.0,  // latency p50
                60.0,  // latency p99
                512,   // memory MB
            ))
            .expect("Should register slow_model");

        let mut router = CrossModelRouter::new(registry);

        // Route request with generous budget
        let request = RoutingRequest::new(
            "sae_forward".to_string(),
            "1.0.0".to_string(),
            50.0,   // latency budget
            1024,   // memory budget
            RoutingPriority::Normal,
        );

        let result = router
            .route(request)
            .expect("Routing should succeed");

        assert_eq!(
            result.selected_model, "fast_model",
            "Should select fast_model (lowest latency within budget)"
        );
        assert!(
            result.estimated_latency_ms <= 50.0,
            "Estimated latency should be within budget"
        );
        assert!(
            result.fallback_chain.contains(&"slow_model".to_string()),
            "Fallback chain should include slow_model"
        );

        // Verify routing stats
        let stats = router.get_routing_stats();
        assert_eq!(stats.total_requests, 1, "Should have 1 total request");
    }

    // =========================================================================
    // Test 6: test_cross_model_routing_fallback
    // =========================================================================
    // Registers multiple models, exhausts the primary model (by setting
    // runtime status to exceed budget), and verifies fallback to secondary.
    // =========================================================================

    #[test]
    fn test_cross_model_routing_fallback() {
        let mut registry = CapabilityRegistry::new();

        // Register primary model (low latency)
        registry
            .register(ModelCapability::new(
                "primary".to_string(),
                "Primary-Model".to_string(),
                "1.0.0".to_string(),
                vec!["inference".to_string()],
                4096,
                22528,
                32,
                5.0,
                10.0,
                128,
            ))
            .unwrap();

        // Register secondary model (higher latency)
        registry
            .register(ModelCapability::new(
                "secondary".to_string(),
                "Secondary-Model".to_string(),
                "1.0.0".to_string(),
                vec!["inference".to_string()],
                4096,
                22528,
                32,
                15.0,
                30.0,
                256,
            ))
            .unwrap();

        // Register tertiary model (highest latency)
        registry
            .register(ModelCapability::new(
                "tertiary".to_string(),
                "Tertiary-Model".to_string(),
                "1.0.0".to_string(),
                vec!["inference".to_string()],
                4096,
                22528,
                32,
                30.0,
                60.0,
                512,
            ))
            .unwrap();

        let mut router = CrossModelRouter::new(registry);

        // Simulate primary model becoming slow (runtime status update)
        router.update_model_status("primary", 100.0, 2048);

        // Route with tight budget that excludes primary
        let request = RoutingRequest::new(
            "inference".to_string(),
            "1.0.0".to_string(),
            50.0,   // latency budget (excludes primary at 100ms)
            1024,   // memory budget (excludes primary at 2048MB)
            RoutingPriority::Normal,
        );

        let result = router
            .route(request.clone())
            .expect("Should fallback to secondary model");

        assert_eq!(
            result.selected_model, "secondary",
            "Should select secondary model after primary exceeds budget"
        );

        // Test route_with_fallback
        let result_fallback = router
            .route_with_fallback(request, 3)
            .expect("Fallback routing should succeed");

        assert!(
            !result_fallback.fallback_chain.is_empty(),
            "Fallback chain should not be empty"
        );
    }

    // =========================================================================
    // Test 7: test_capability_schema_negotiation
    // =========================================================================
    // Registers models with different schema versions, negotiates a common
    // schema, and verifies the most common version is selected.
    // =========================================================================

    #[test]
    fn test_capability_schema_negotiation() {
        let mut registry = CapabilityRegistry::new();

        // Register 2 models with schema v1.0.0
        registry
            .register(ModelCapability::new(
                "model_a".to_string(),
                "Model-A".to_string(),
                "1.0.0".to_string(),
                vec!["task_x".to_string()],
                4096,
                22528,
                32,
                10.0,
                20.0,
                256,
            ))
            .unwrap();

        registry
            .register(ModelCapability::new(
                "model_b".to_string(),
                "Model-B".to_string(),
                "1.0.0".to_string(),
                vec!["task_x".to_string()],
                4096,
                22528,
                32,
                12.0,
                24.0,
                384,
            ))
            .unwrap();

        // Register 1 model with schema v2.0.0
        registry
            .register(ModelCapability::new(
                "model_c".to_string(),
                "Model-C".to_string(),
                "2.0.0".to_string(),
                vec!["task_x".to_string()],
                4096,
                22528,
                32,
                8.0,
                16.0,
                128,
            ))
            .unwrap();

        // Get all models and negotiate schema
        let all_models = registry.get_all();
        let models: Vec<&ModelCapability> = all_models.into_iter().collect();

        let negotiated = registry
            .negotiate_schema(&models)
            .expect("Should negotiate a common schema");

        assert_eq!(
            negotiated, "1.0.0",
            "Should select v1.0.0 as it is the most common schema (2 vs 1)"
        );

        // Verify registry stats
        let stats = registry.stats();
        assert_eq!(stats.total_models, 3, "Should have 3 registered models");
        assert_eq!(stats.schema_versions, 2, "Should have 2 unique schema versions");
        assert_eq!(stats.unique_tasks, 1, "Should have 1 unique task");
    }

    // =========================================================================
    // Test 8: test_p2p_to_fedavg_to_routing
    // =========================================================================
    // Full integration flow: simulate P2P weight updates -> FedAvg v2
    // aggregation -> cross-model routing with aggregated results.
    // =========================================================================

    #[test]
    fn test_p2p_to_fedavg_to_routing() {
        // Phase 1: Simulate P2P weight collection
        let num_nodes = 5;
        let layer_id = 0;
        let dim = 200;

        let mut agg_config = FedAvgConfigV2::default();
        agg_config.compression_enabled = true;
        agg_config.top_k_sparsity = 50;
        agg_config.krum_f = 1;
        let mut fedavg = FedAvgAggregatorV2::new(agg_config.clone());

        // Collect updates from P2P nodes
        for i in 0..num_nodes {
            let update = make_update(&format!("p2p_node_{}", i), layer_id, dim, i as u32);
            fedavg
                .add_update(update)
                .expect("Should accept P2P node update");
        }

        // Phase 2: FedAvg v2 aggregation
        let agg_result = fedavg
            .aggregate(layer_id)
            .expect("FedAvg aggregation should succeed");

        assert_eq!(agg_result.layer_id, layer_id, "Aggregated layer should match");
        assert_eq!(
            agg_result.final_weights.len(),
            dim,
            "Aggregated weights should have correct dimension"
        );
        assert!(
            agg_result.compression_ratio < 1.0,
            "Compression ratio should be < 1.0 with compression enabled"
        );

        // Phase 3: Register aggregated model in capability registry
        let mut registry = CapabilityRegistry::new();

        registry
            .register(ModelCapability::new(
                "aggregated_model".to_string(),
                "FedAvg-Aggregated".to_string(),
                "1.0.0".to_string(),
                vec!["sae_forward".to_string(), "inference".to_string()],
                dim,
                dim,
                32,
                agg_result.aggregation_latency_ms,
                agg_result.aggregation_latency_ms * 2.0,
                512,
            ))
            .expect("Should register aggregated model");

        // Register a secondary model for routing comparison
        registry
            .register(ModelCapability::new(
                "baseline_model".to_string(),
                "Baseline".to_string(),
                "1.0.0".to_string(),
                vec!["sae_forward".to_string()],
                dim,
                dim,
                32,
                50.0,
                100.0,
                256,
            ))
            .expect("Should register baseline model");

        // Phase 4: Cross-model routing
        let mut router = CrossModelRouter::new(registry);

        let request = RoutingRequest::new(
            "sae_forward".to_string(),
            "1.0.0".to_string(),
            1000.0, // generous latency budget
            2048,   // generous memory budget
            RoutingPriority::Normal,
        );

        let route_result = router
            .route(request)
            .expect("Routing should succeed after aggregation");

        // The aggregated model should be selected (lower latency from aggregation)
        assert_eq!(
            route_result.selected_model, "aggregated_model",
            "Should route to aggregated model (lowest latency)"
        );

        // Verify full pipeline stats
        let routing_stats = router.get_routing_stats();
        assert_eq!(routing_stats.total_requests, 1, "Should have 1 routed request");
    }

    // =========================================================================
    // Test 9: test_concurrent_aggregation
    // =========================================================================
    // Aggregates multiple layers in parallel using aggregate_parallel()
    // and verifies all results are valid.
    // =========================================================================

    #[tokio::test]
    async fn test_concurrent_aggregation() {
        let config = FedAvgConfigV2 {
            min_participants: 3,
            krum_f: 0,
            compression_enabled: false,
            parallel_layers: 4,
            min_participation_fraction: 0.4,
            top_k_sparsity: 64,
            quantization_bits: 8,
        };
        let mut agg = FedAvgAggregatorV2::new(config);

        // Add updates for 4 layers
        let num_layers = 4;
        let dim = 100;

        for layer in 0..num_layers {
            for node in 0..5 {
                let update = make_update(
                    &format!("node_{}", node),
                    layer,
                    dim,
                    (layer * 10 + node) as u32,
                );
                agg.add_update(update)
                    .expect("Should accept update for layer");
            }
        }

        // Aggregate all layers in parallel
        let layer_ids: Vec<u32> = (0..num_layers).collect();
        let results = agg.aggregate_parallel(&layer_ids);

        assert_eq!(results.len(), num_layers as usize, "Should have results for all layers");

        for (i, result) in results.iter().enumerate() {
            let result: &AggregationResultV2 = result
                .as_ref()
                .expect(&format!("Layer {} aggregation should succeed", i));

            assert_eq!(
                result.layer_id,
                i as u32,
                "Layer ID should match for result {}",
                i
            );
            assert_eq!(
                result.final_weights.len(),
                dim,
                "Layer {} should have {} weights",
                i, dim
            );
            assert!(
                result.accepted_updates >= 3,
                "Layer {} should have at least 3 accepted updates",
                i
            );
        }
    }

    // =========================================================================
    // Test 10: test_byzantine_rejection_v2
    // =========================================================================
    // Submits malicious (outlier) weight updates and verifies Krum v2
    // filters them out during aggregation.
    // =========================================================================

    #[test]
    fn test_byzantine_rejection_v2() {
        let config = FedAvgConfigV2 {
            min_participants: 3,
            krum_f: 2, // Tolerate up to 2 Byzantine nodes
            compression_enabled: false,
            parallel_layers: 1,
            min_participation_fraction: 0.4,
            top_k_sparsity: 64,
            quantization_bits: 8,
        };
        let mut agg = FedAvgAggregatorV2::new(config);

        // Add 5 honest nodes with similar updates (seed=10)
        for i in 0..5 {
            let update = make_update(&format!("honest_{}", i), 0, 100, 10);
            agg.add_update(update)
                .expect("Should accept honest update");
        }

        // Add 2 Byzantine nodes with extreme outlier updates (seed=9999)
        for i in 0..2 {
            let update = make_update(&format!("byzantine_{}", i), 0, 100, 9999);
            agg.add_update(update)
                .expect("Should accept byzantine update (validation passes)");
        }

        // Aggregate
        let result = agg
            .aggregate(0)
            .expect("Aggregation should succeed despite byzantine nodes");

        // Verify byzantine nodes were filtered
        assert!(
            result.filtered_malicious >= 1,
            "At least 1 byzantine node should be filtered, got {}",
            result.filtered_malicious
        );

        // Verify byzantine nodes are in excluded list
        let byzantine_excluded: usize = result
            .excluded_nodes
            .iter()
            .filter(|n: &&String| n.starts_with("byzantine_"))
            .count();
        assert!(
            byzantine_excluded > 0,
            "At least 1 byzantine node should be in excluded list, got {}",
            byzantine_excluded
        );

        // Verify honest nodes are mostly included
        let honest_included: usize = result
            .included_nodes
            .iter()
            .filter(|n: &&String| n.starts_with("honest_"))
            .count();
        assert!(
            honest_included >= 3,
            "At least 3 honest nodes should be included, got {}",
            honest_included
        );

        // Verify confidence is reasonable (Krum v2 selects n-f-2 out of n)
        assert!(
            result.confidence > 0.3,
            "Confidence should be reasonable with majority honest nodes, got {}",
            result.confidence
        );
    }
}
