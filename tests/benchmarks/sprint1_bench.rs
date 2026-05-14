//! Benchmarks for ed2kIA v1.1.0 Sprint 1
//!
//! This module provides performance benchmarks comparing v1.0.0 vs v1.1.0-sprint1
//! across all new modules:
//!
//! | Benchmark | Description | v1.0.0 Baseline | v1.1.0 Target |
//! |-----------|-------------|-----------------|---------------|
//! | `bench_fedavg_v1_aggregation` | v1.0.0 FedAvg with n nodes | Baseline | N/A |
//! | `bench_fedavg_v2_aggregation` | v1.1.0 FedAvg v2 with n nodes | N/A | Compare vs v1 |
//! | `bench_gradient_compression_topk` | Top-K sparsity compression | N/A | New feature |
//! | `bench_gradient_quantization_int8` | int8 quantization | N/A | New feature |
//! | `bench_wasm_sandbox_v2_execute` | WASM v2 execution overhead | N/A | New feature |
//! | `bench_cross_model_routing` | Cross-model routing with n models | N/A | New feature |
//! | `bench_capability_lookup` | Capability registry lookup | N/A | New feature |
//!
//! # Running Benchmarks
//!
//! ```bash
//! # Run all benchmarks
//! cargo bench --features v1.1-sprint1 --test sprint1_bench
//!
//! # Run specific benchmark
//! cargo bench --features v1.1-sprint1 --test sprint1_bench fedavg
//! ```
//!
//! # Methodology
//!
//! - Each benchmark reports latency (ms), throughput (ops/sec), and memory usage
//! - v1 vs v2 comparisons are done within the same test run for consistency
//! - Long-running benchmarks are marked `#[ignore]` and require `--ignored` flag
//! - Uses criterion-style manual timing with `std::time::Instant`

#[cfg(feature = "v1.1-sprint1")]
mod imports {
    pub use ed2kia::federation::avg_aggregator::{FedAvgAggregator, FedAvgConfig, WeightUpdate};
    pub use ed2kia::federation_v2_sprint1::avg_aggregator_v2::{
        FedAvgAggregatorV2, FedAvgConfigV2,
    };
    pub use ed2kia::federation_v2_sprint1::gradient_compressor::GradientCompressor;
    pub use ed2kia::interoperability::capability_registry::{
        CapabilityRegistry, ModelCapability,
    };
    pub use ed2kia::interoperability::cross_model_router::{
        CrossModelRouter, RoutingPriority, RoutingRequest,
    };
    pub use ed2kia::security::wasm_sandbox_v2::{SandboxConfigV2, WasmSandboxV2};
}

#[cfg(feature = "v1.1-sprint1")]
mod benchmarks {
    use super::imports::*;
    use std::time::Instant;

    // ---------------------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------------------

    fn make_update(node_id: &str, layer_id: u32, dim: usize, seed: u32) -> WeightUpdate {
        let deltas: Vec<f32> = (0..dim)
            .map(|i| ((i + seed as usize) % 100) as f32 / 50.0 - 1.0)
            .collect();
        WeightUpdate::new(node_id.to_string(), layer_id, deltas, 100, 0.5)
    }

    fn make_capability(id: &str, name: &str, tasks: &[&str], latency: f64, memory: usize) -> ModelCapability {
        ModelCapability::new(
            id.to_string(),
            name.to_string(),
            "1.0.0".to_string(),
            tasks.iter().map(|s| s.to_string()).collect(),
            4096,
            22528,
            32,
            latency,
            latency * 2.5,
            memory,
        )
    }

    fn wasm_with_run_function() -> Vec<u8> {
        [
            0x00, 0x61, 0x73, 0x6D,
            0x01, 0x00, 0x00, 0x00,
            0x01, 0x04, 0x01, 0x60, 0x00, 0x00,
            0x03, 0x02, 0x01, 0x00,
            0x07, 0x07, 0x01, 0x03, b'r', b'u', b'n', 0x00, 0x00,
            0x0A, 0x04, 0x01, 0x02, 0x00, 0x0B,
        ]
        .to_vec()
    }

    fn report(name: &str, iterations: usize, elapsed_ms: f64, memory_bytes: usize) {
        let throughput = (iterations as f64) / (elapsed_ms / 1000.0);
        println!(
            "BENCH {} | iterations={} | latency={:.3}ms | throughput={:.0} ops/sec | memory={} bytes",
            name, iterations, elapsed_ms, throughput, memory_bytes
        );
    }

    // =========================================================================
    // Benchmark 1: bench_fedavg_v1_aggregation(n)
    // =========================================================================
    // Benchmarks v1.0.0 FedAvg aggregation with n nodes.
    // This is the baseline for comparison with v2.
    // =========================================================================

    #[test]
    #[ignore]
    fn bench_fedavg_v1_aggregation() {
        println!("\n=== bench_fedavg_v1_aggregation ===");

        let node_counts = vec![5, 10, 20, 50];
        let dim = 500;
        let iterations = 10;

        for &n in &node_counts {
            let mut total_ms = 0.0;

            for _ in 0..iterations {
                let config = FedAvgConfig {
                    min_participants: 3,
                    krum_f: 1,
                    min_participation_fraction: 0.4,
                };
                let mut agg = FedAvgAggregator::new(config);

                for i in 0..n {
                    let update = make_update(&format!("node_{}", i), 0, dim, i as u32);
                    agg.add_update(update).unwrap();
                }

                let start = Instant::now();
                agg.aggregate(0).unwrap();
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                total_ms += elapsed;
            }

            let avg_ms = total_ms / iterations as f64;
            let memory_bytes = n * dim * std::mem::size_of::<f32>();
            report(
                &format!("fedavg_v1_aggregation(n={})", n),
                iterations,
                avg_ms,
                memory_bytes,
            );
        }
    }

    // =========================================================================
    // Benchmark 2: bench_fedavg_v2_aggregation(n)
    // =========================================================================
    // Benchmarks v1.1.0 FedAvg v2 aggregation with n nodes.
    // Compares against v1 baseline for the same node count.
    // =========================================================================

    #[test]
    #[ignore]
    fn bench_fedavg_v2_aggregation() {
        println!("\n=== bench_fedavg_v2_aggregation ===");

        let node_counts = vec![5, 10, 20, 50];
        let dim = 500;
        let iterations = 10;

        for &n in &node_counts {
            // --- V2 with compression ---
            let config_v2 = FedAvgConfigV2 {
                min_participants: 3,
                krum_f: 1,
                compression_enabled: true,
                top_k_sparsity: 64,
                quantization_bits: 8,
                parallel_layers: 4,
                min_participation_fraction: 0.4,
            };

            let mut total_v2_ms = 0.0;

            for _ in 0..iterations {
                let mut agg = FedAvgAggregatorV2::new(config_v2.clone());

                for i in 0..n {
                    let update = make_update(&format!("node_{}", i), 0, dim, i as u32);
                    agg.add_update(update).unwrap();
                }

                let start = Instant::now();
                agg.aggregate(0).unwrap();
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                total_v2_ms += elapsed;
            }

            let avg_v2_ms = total_v2_ms / iterations as f64;
            let memory_bytes = n * dim * std::mem::size_of::<f32>();
            report(
                &format!("fedavg_v2_aggregation_compressed(n={})", n),
                iterations,
                avg_v2_ms,
                memory_bytes,
            );

            // --- V2 without compression (direct comparison with v1) ---
            let config_v2_nocomp = FedAvgConfigV2 {
                compression_enabled: false,
                ..config_v2.clone()
            };

            let mut total_v2_nocomp_ms = 0.0;

            for _ in 0..iterations {
                let mut agg = FedAvgAggregatorV2::new(config_v2_nocomp.clone());

                for i in 0..n {
                    let update = make_update(&format!("node_{}", i), 0, dim, i as u32);
                    agg.add_update(update).unwrap();
                }

                let start = Instant::now();
                agg.aggregate(0).unwrap();
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                total_v2_nocomp_ms += elapsed;
            }

            let avg_v2_nocomp_ms = total_v2_nocomp_ms / iterations as f64;
            report(
                &format!("fedavg_v2_aggregation_nocomp(n={})", n),
                iterations,
                avg_v2_nocomp_ms,
                memory_bytes,
            );
        }
    }

    // =========================================================================
    // Benchmark 3: bench_gradient_compression_topk(dim, k)
    // =========================================================================
    // Benchmarks Top-K sparsity compression for various dimensions and k values.
    // =========================================================================

    #[test]
    #[ignore]
    fn bench_gradient_compression_topk() {
        println!("\n=== bench_gradient_compression_topk ===");

        let compressor = GradientCompressor;

        let configs: Vec<(usize, usize)> = vec![
            (1_000, 100),      // 10% sparsity
            (10_000, 1_000),   // 10% sparsity
            (100_000, 10_000), // 10% sparsity
            (1_000_000, 100_000), // 10% sparsity
        ];
        let iterations = 50;

        for &(dim, k) in &configs {
            let deltas: Vec<f32> = (0..dim)
                .map(|i| ((i % 100) as f32 / 50.0 - 1.0))
                .collect();

            let start = Instant::now();
            for _ in 0..iterations {
                let (_values, _indices) = compressor.compress_top_k(&deltas, k);
            }
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;

            let memory_bytes = dim * std::mem::size_of::<f32>();
            let compressed_bytes = k * std::mem::size_of::<f32>() + k * std::mem::size_of::<usize>();
            let ratio = compressed_bytes as f32 / memory_bytes as f32;

            println!(
                "BENCH topk(dim={}, k={}) | iterations={} | latency={:.3}ms | throughput={:.0} ops/sec | memory={} bytes | ratio={:.2}",
                dim, k, iterations, elapsed, iterations as f64 / (elapsed / 1000.0), memory_bytes, ratio
            );
        }
    }

    // =========================================================================
    // Benchmark 4: bench_gradient_quantization_int8(dim)
    // =========================================================================
    // Benchmarks int8 quantization + dequantization for various dimensions.
    // =========================================================================

    #[test]
    #[ignore]
    fn bench_gradient_quantization_int8() {
        println!("\n=== bench_gradient_quantization_int8 ===");

        let compressor = GradientCompressor;

        let dims = vec![1_000, 10_000, 100_000, 1_000_000];
        let iterations = 50;

        for dim in dims {
            let deltas: Vec<f32> = (0..dim)
                .map(|i| ((i % 100) as f32 / 50.0 - 1.0))
                .collect();

            // Quantize
            let start = Instant::now();
            for _ in 0..iterations {
                let (_quantized, _scale) = compressor.quantize_int8(&deltas);
            }
            let quantize_ms = start.elapsed().as_secs_f64() * 1000.0;

            // Full roundtrip
            let start = Instant::now();
            for _ in 0..iterations {
                let (quantized, scale) = compressor.quantize_int8(&deltas);
                let _reconstructed = compressor.dequantize_int8(&quantized, scale);
            }
            let roundtrip_ms = start.elapsed().as_secs_f64() * 1000.0;

            let original_bytes = dim * std::mem::size_of::<f32>();
            let quantized_bytes = dim * std::mem::size_of::<i8>();

            println!(
                "BENCH quantize_int8(dim={}) | iterations={} | quantize={:.3}ms | roundtrip={:.3}ms | throughput={:.0} ops/sec | memory={}->{} bytes ({}x)",
                dim, iterations, quantize_ms, roundtrip_ms,
                iterations as f64 / (quantize_ms / 1000.0),
                original_bytes, quantized_bytes,
                original_bytes / quantized_bytes
            );
        }
    }

    // =========================================================================
    // Benchmark 5: bench_wasm_sandbox_v2_execute(iterations)
    // =========================================================================
    // Benchmarks WASM v2 sandbox execution overhead including profiling.
    // =========================================================================

    #[test]
    #[ignore]
    fn bench_wasm_sandbox_v2_execute() {
        println!("\n=== bench_wasm_sandbox_v2_execute ===");

        let iteration_counts = vec![10, 50, 100];

        for &num_executions in &iteration_counts {
            let config = SandboxConfigV2 {
                memory_limit_bytes: 64 * 1024 * 1024,
                fuel_limit: 1_000_000,
                fallback_threshold_percent: 80.0,
                max_modules: 10,
                enable_profiling: true,
            };
            let mut sandbox = WasmSandboxV2::new(config);

            let wasm = wasm_with_run_function();
            let module_id = sandbox.load_module(&wasm).expect("Should load WASM module");

            let start = Instant::now();
            for _ in 0..num_executions {
                let _result = sandbox
                    .execute(&module_id.id, "run", Vec::new())
                    .expect("Should execute");
            }
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;

            let avg_ms = elapsed / num_executions as f64;
            let throughput = (num_executions as f64) / (elapsed / 1000.0);

            println!(
                "BENCH wasm_v2_execute(iter={}) | total={:.3}ms | avg={:.3}ms/exec | throughput={:.0} exec/sec",
                num_executions, elapsed, avg_ms, throughput
            );
        }
    }

    // =========================================================================
    // Benchmark 6: bench_cross_model_routing(n_models)
    // =========================================================================
    // Benchmarks cross-model routing with varying numbers of registered models.
    // =========================================================================

    #[test]
    #[ignore]
    fn bench_cross_model_routing() {
        println!("\n=== bench_cross_model_routing ===");

        let model_counts = vec![5, 10, 20, 50, 100];
        let iterations = 100;

        for &n_models in &model_counts {
            let mut registry = CapabilityRegistry::new();

            for i in 0..n_models {
                let cap = make_capability(
                    &format!("model_{}", i),
                    &format!("Model-{}", i),
                    &["sae_forward", "embedding", "inference"],
                    5.0 + (i % 20) as f64,
                    128 + i * 10,
                );
                registry.register(cap).unwrap();
            }

            let mut router = CrossModelRouter::new(registry);

            let request = RoutingRequest::new(
                "sae_forward".to_string(),
                "1.0.0".to_string(),
                100.0,
                2048,
                RoutingPriority::Normal,
            );

            let start = Instant::now();
            for _ in 0..iterations {
                let _result = router.route(request.clone());
            }
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;

            let avg_ms = elapsed / iterations as f64;
            let throughput = (iterations as f64) / (elapsed / 1000.0);

            println!(
                "BENCH cross_model_routing(n={}) | iterations={} | total={:.3}ms | avg={:.3}ms/route | throughput={:.0} routes/sec",
                n_models, iterations, elapsed, avg_ms, throughput
            );
        }
    }

    // =========================================================================
    // Benchmark 7: bench_capability_lookup(n_models)
    // =========================================================================
    // Benchmarks capability registry lookup (find_by_task) with varying
    // numbers of registered models.
    // =========================================================================

    #[test]
    #[ignore]
    fn bench_capability_lookup() {
        println!("\n=== bench_capability_lookup ===");

        let model_counts = vec![5, 10, 20, 50, 100, 200];
        let iterations = 500;

        for &n_models in &model_counts {
            let mut registry = CapabilityRegistry::new();

            for i in 0..n_models {
                let tasks = if i % 3 == 0 {
                    vec!["sae_forward".to_string(), "embedding".to_string()]
                } else if i % 3 == 1 {
                    vec!["feature_extraction".to_string(), "embedding".to_string()]
                } else {
                    vec!["inference".to_string(), "sae_forward".to_string()]
                };

                let cap = ModelCapability::new(
                    format!("model_{}", i),
                    format!("Model-{}", i),
                    "1.0.0".to_string(),
                    tasks,
                    4096,
                    22528,
                    32,
                    5.0 + (i % 20) as f64,
                    10.0 + (i % 20) as f64 * 2.0,
                    128 + i * 5,
                );
                registry.register(cap).unwrap();
            }

            // Benchmark find_by_task
            let start = Instant::now();
            for _ in 0..iterations {
                let _results = registry.find_by_task("sae_forward");
            }
            let task_ms = start.elapsed().as_secs_f64() * 1000.0;

            // Benchmark find_by_schema
            let start = Instant::now();
            for _ in 0..iterations {
                let _results = registry.find_by_schema("1.0.0");
            }
            let schema_ms = start.elapsed().as_secs_f64() * 1000.0;

            // Benchmark find_optimal
            let start = Instant::now();
            for _ in 0..iterations {
                let _result = registry.find_optimal("sae_forward", 50.0, 2048);
            }
            let optimal_ms = start.elapsed().as_secs_f64() * 1000.0;

            let memory_bytes = n_models * std::mem::size_of::<ModelCapability>() * 2; // rough estimate

            println!(
                "BENCH capability_lookup(n={}) | iterations={} | find_by_task={:.3}ms | find_by_schema={:.3}ms | find_optimal={:.3}ms | memory={} bytes",
                n_models, iterations, task_ms, schema_ms, optimal_ms, memory_bytes
            );
        }
    }
}

fn main() {
    println!("ed2kIA v1.1.0 Sprint 1 Benchmark Suite");
    println!("Run with: cargo bench --features v1.1-sprint1 --test sprint1_bench");
    println!("For ignored (long) benchmarks: cargo bench --features v1.1-sprint1 --test sprint1_bench -- --ignored");
}
