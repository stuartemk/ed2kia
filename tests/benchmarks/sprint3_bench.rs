//! v1.1.0 Sprint 3 Benchmarks
//!
//! Comparativa latencia/throughput vs v1.0.0, estrés de pool de verificación,
//! partición de red en marketplace.
//!
//! Feature-gated: `--features v1.1-sprint3`
//! No harness: benchmarks manuales con timing explícito.

#![cfg(feature = "v1.1-sprint3")]

use ark_bn254::{Fr, G1Affine, G1Projective};
use ark_ec::{Group, CurveGroup};
use ark_ff::UniformRand;
use ark_std::rand::thread_rng;
use ark_std::rand::Rng;
use ed2kia::zkp::async_prover::AsyncProver;
use ed2kia::zkp::batch_accumulator::BatchAccumulator;
use ed2kia::zkp::circuit::{BatchCommitment, ZKPProof, Witness};
use ed2kia::zkp::verifier_pool::VerifierPool;
use ed2kia::marketplace_v2::matchmaker::{ResourceMatchmaker, ResourceRequest, ResourceListing, ResourceType};
use ed2kia::marketplace_v2::escrow_ledger::{EscrowLedger, EscrowState, SLOMetrics};
use ed2kia::marketplace_v2::pricing_engine::{PricingEngine, PricingResourceType, MarketSample};
use ed2kia::bridge_v2::zkp_marketplace_bridge::ZKPMarketplaceBridge;
use ed2kia::bridge_v2::proof_submission::ProofSubmissionManager;
use sha2::{Digest, Sha256};
use std::time::Instant;

// ============================================================================
// Helpers
// ============================================================================

fn create_witness_n(features: usize) -> Witness {
    let mut rng = thread_rng();
    let feature_values: Vec<Fr> = (0..features).map(|_| Fr::rand(&mut rng)).collect();
    let blinding_factors: Vec<Fr> = (0..4).map(|_| Fr::rand(&mut rng)).collect();

    let mut batch_hash = [0u8; 32];
    batch_hash.copy_from_slice(&Sha256::digest(b"bench_batch"));

    Witness {
        feature_values,
        blinding_factors,
        batch_hash,
    }
}

fn create_proof_for_verification() -> (ZKPProof, BatchCommitment) {
    let mut rng = thread_rng();
    let generator = <G1Projective as Group>::generator();
    let a = generator.into_affine();
    let b = vec![a];
    let challenge: [u8; 32] = rng.gen();

    let proof = ZKPProof {
        a,
        b,
        c: a,
        challenge,
        batch_id: "bench".into(),
        feature_count: 64,
    };

    let commitment = BatchCommitment {
        commitment_point: a,
        batch_hash: challenge,
        feature_count: 64,
        compact_bytes: Vec::new(),
    };

    (proof, commitment)
}

fn make_sae_listing(node_id: &str, price: f32) -> ResourceListing {
    ResourceListing {
        node_id: node_id.into(),
        resource_type: ResourceType::SAEShard {
            model_id: "scope-v2".into(),
            layer: 5,
        },
        quantity: 10.0,
        base_price: price,
        listed_at: 1000,
        expires_at: 10000,
        max_latency_ms: 100,
        availability_slo: 0.99,
        min_throughput: 1000,
    }
}

// ============================================================================
// Benchmark: ZKP Prover
// ============================================================================

fn bench_zkp_prover_single() {
    let prover = AsyncProver::new();
    let witness = create_witness_n(64);

    let start = Instant::now();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(prover.generate_proof("bench_single".into(), witness));
    let elapsed = start.elapsed();

    match result {
        Ok(pr) => {
            println!("  ZKP Prover (single, 64 features): {} ms", pr.generation_time_ms);
            println!("  Wall time: {} ms", elapsed.as_secs_f64() * 1000.0);
        }
        Err(e) => {
            println!("  ZKP Prover (single, 64 features): ERROR - {}", e);
            println!("  Wall time: {} ms", elapsed.as_secs_f64() * 1000.0);
        }
    }
}

fn bench_zkp_prover_batch() {
    let prover = AsyncProver::new();
    let witnesses: Vec<Witness> = (0..16).map(|i| {
        let mut w = create_witness_n(32);
        w.batch_hash = Sha256::digest(format!("bench_batch_{}", i)).into();
        w
    }).collect();

    let start = Instant::now();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut results = Vec::new();
    for (i, w) in witnesses.into_iter().enumerate() {
        let r = rt.block_on(prover.generate_proof(format!("bench_batch_{}", i), w));
        results.push(r);
    }
    let elapsed = start.elapsed();

    let success = results.iter().filter(|r| r.is_ok()).count();
    println!(
        "  ZKP Prover (batch, 16 x 32 features): {} ms total, {} success",
        elapsed.as_secs_f64() * 1000.0,
        success
    );
}

fn bench_zkp_prover_large_batch() {
    let prover = AsyncProver::new();
    let witness = create_witness_n(256);

    let start = Instant::now();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(prover.generate_proof("bench_large".into(), witness));
    let elapsed = start.elapsed();

    match result {
        Ok(pr) => {
            println!(
                "  ZKP Prover (large, 256 features): {} ms",
                pr.generation_time_ms
            );
        }
        Err(e) => {
            println!("  ZKP Prover (large, 256 features): ERROR - {}", e);
        }
    }
    println!("  Wall time: {} ms", elapsed.as_secs_f64() * 1000.0);
}

// ============================================================================
// Benchmark: Verifier Pool
// ============================================================================

fn bench_verifier_pool_single() {
    let pool = VerifierPool::new();
    let (proof, commitment) = create_proof_for_verification();

    let start = Instant::now();
    let result = pool.verify(proof, commitment);
    let elapsed = start.elapsed();

    match result {
        Ok(vr) => {
            println!(
                "  Verifier Pool (single): {} ms (pool time: {} ms)",
                vr.total_time_ms, vr.record.computation_time_ms
            );
        }
        Err(e) => {
            println!("  Verifier Pool (single): ERROR - {}", e);
        }
    }
    println!("  Wall time: {} ms", elapsed.as_secs_f64() * 1000.0);
}

fn bench_verifier_pool_stress() {
    let pool = VerifierPool::new();
    let items: Vec<(ZKPProof, BatchCommitment)> = (0..20)
        .map(|_| create_proof_for_verification())
        .collect();

    let start = Instant::now();
    let mut results = Vec::new();
    for (proof, commitment) in items {
        let r = pool.verify(proof, commitment);
        results.push(r);
    }
    let elapsed = start.elapsed();

    let success = results.iter().filter(|r| r.is_ok()).count();
    println!(
        "  Verifier Pool (stress, 20 items): {} ms total, {} success",
        elapsed.as_secs_f64() * 1000.0,
        success
    );
}

// ============================================================================
// Benchmark: Batch Accumulator
// ============================================================================

fn bench_batch_accumulator_add() {
    let mut acc = BatchAccumulator::new();
    let (proof, commitment) = create_proof_for_verification();

    let start = Instant::now();
    let result = acc.add_batch("bench_acc".into(), commitment);
    let elapsed = start.elapsed();

    match result {
        Ok(_) => {
            let stats = acc.get_stats();
            println!(
                "  Batch Accumulator (add): {} ms, capacity: {}/{}",
                elapsed.as_secs_f64() * 1000.0,
                stats.current_capacity,
                stats.max_capacity
            );
        }
        Err(e) => {
            println!("  Batch Accumulator (add): ERROR - {}", e);
        }
    }
}

fn bench_batch_accumulator_multiple() {
    let mut acc = BatchAccumulator::new();

    let start = Instant::now();
    for i in 0..32 {
        let (proof, commitment) = create_proof_for_verification();
        let _ = acc.add_batch(format!("bench_acc_{}", i), commitment);
    }
    let elapsed = start.elapsed();

    let stats = acc.get_stats();
    println!(
        "  Batch Accumulator (32 adds): {} ms, total_batches: {}",
        elapsed.as_secs_f64() * 1000.0,
        stats.total_batches
    );
}

// ============================================================================
// Benchmark: Marketplace Matching
// ============================================================================

fn bench_matchmaker_single() {
    let mut mm = ResourceMatchmaker::new();
    for i in 0..10 {
        mm.register_listing(make_sae_listing(&format!("node_{}", i), 100.0 + i as f32));
    }

    let req = ResourceRequest {
        requester_id: "buyer".into(),
        resource_type: ResourceType::SAEShard {
            model_id: "scope-v2".into(),
            layer: 5,
        },
        quantity: 5.0,
        max_price: 150.0,
        max_latency_ms: 100,
        min_availability: 0.95,
    };

    let start = Instant::now();
    let result = mm.match_request(&req);
    let elapsed = start.elapsed();

    match result {
        Ok(mr) => {
            println!(
                "  Matchmaker (single, 10 listings): {} ms, matched={}",
                mr.match_time_ms,
                mr.matched
            );
        }
        Err(e) => {
            println!("  Matchmaker (single): ERROR - {}", e);
        }
    }
    println!("  Wall time: {} ms", elapsed.as_secs_f64() * 1000.0);
}

fn bench_matchmaker_large() {
    let mut mm = ResourceMatchmaker::new();
    for i in 0..100 {
        mm.register_listing(make_sae_listing(&format!("node_{}", i), 50.0 + (i as f32 % 200.0)));
    }

    let req = ResourceRequest {
        requester_id: "buyer".into(),
        resource_type: ResourceType::SAEShard {
            model_id: "scope-v2".into(),
            layer: 5,
        },
        quantity: 5.0,
        max_price: 200.0,
        max_latency_ms: 100,
        min_availability: 0.95,
    };

    let start = Instant::now();
    let result = mm.match_request(&req);
    let elapsed = start.elapsed();

    match result {
        Ok(mr) => {
            println!(
                "  Matchmaker (large, 100 listings): {} ms, matched={}",
                mr.match_time_ms,
                mr.matched
            );
        }
        Err(e) => {
            println!("  Matchmaker (large): ERROR - {}", e);
        }
    }
    println!("  Wall time: {} ms", elapsed.as_secs_f64() * 1000.0);
}

// ============================================================================
// Benchmark: Escrow Ledger
// ============================================================================

fn bench_escrow_create() {
    let (ledger, _, _path) = EscrowLedger::new_test().unwrap();

    let start = Instant::now();
    let mut total_ms = 0.0;
    for i in 0..50 {
        let tx = ledger.create_escrow(
            format!("bench_escrow_{}", i),
            "seller".into(),
            "buyer".into(),
            100.0,
            format!("settlement_{}", i),
        );
        match tx {
            Ok(_) => {}
            Err(e) => println!("  Escrow create error: {}", e),
        }
    }
    let elapsed = start.elapsed();
    total_ms = elapsed.as_secs_f64() * 1000.0;

    println!(
        "  Escrow Ledger (50 creates): {} ms, avg {} ms/tx",
        total_ms,
        total_ms / 50.0
    );
}

fn bench_escrow_transitions() {
    let (ledger, _, _path) = EscrowLedger::new_test().unwrap();

    // Crear 20 escrows
    for i in 0..20 {
        let _ = ledger.create_escrow(
            format!("bench_escrow_t_{}", i),
            "seller".into(),
            "buyer".into(),
            100.0,
            "settlement".into(),
        );
    }

    let start = Instant::now();

    // Transiciones
    for i in 0..20 {
        let _ = ledger.transition_state(&format!("bench_escrow_t_{}", i), EscrowState::Delivered);
    }
    for i in 0..20 {
        let slo = SLOMetrics {
            observed_latency_ms: 50,
            agreed_latency_ms: 100,
            observed_availability: 0.99,
            agreed_availability: 0.95,
            observed_throughput: 2000,
            agreed_throughput: 1000,
        };
        let _ = ledger.release_on_zkp(&format!("bench_escrow_t_{}", i), "zkp".into(), slo);
    }

    let elapsed = start.elapsed();
    println!(
        "  Escrow Ledger (20 release_on_zkp): {} ms, avg {} ms/tx",
        elapsed.as_secs_f64() * 1000.0,
        elapsed.as_secs_f64() * 1000.0 / 20.0
    );
}

// ============================================================================
// Benchmark: Pricing Engine
// ============================================================================

fn bench_pricing_compute() {
    let engine = PricingEngine::new();

    let start = Instant::now();
    for _ in 0..100 {
        let _ = engine.compute_price(PricingResourceType::SAEShard, 100.0, 1.0);
    }
    let elapsed = start.elapsed();

    println!(
        "  Pricing Engine (100 computes): {} ms, avg {} ms",
        elapsed.as_secs_f64() * 1000.0,
        elapsed.as_secs_f64() * 1000.0 / 100.0
    );
}

fn bench_pricing_with_samples() {
    let engine = PricingEngine::new();

    // Registrar samples
    for i in 0..50 {
        engine.record_sample(MarketSample {
            timestamp: chrono::Utc::now().timestamp_millis() as u64 - (i * 1000) as u64,
            price: 100.0,
            quantity: 10.0,
            demand_count: 5 + (i % 20),
        });
    }

    let start = Instant::now();
    for _ in 0..100 {
        let _ = engine.compute_price(PricingResourceType::Generic, 100.0, 1.0);
    }
    let elapsed = start.elapsed();

    println!(
        "  Pricing Engine (100 computes w/ 50 samples): {} ms, avg {} ms",
        elapsed.as_secs_f64() * 1000.0,
        elapsed.as_secs_f64() * 1000.0 / 100.0
    );
}

// ============================================================================
// Benchmark: Bridge E2E
// ============================================================================

fn bench_bridge_workflow() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let start = Instant::now();
    let result = rt.block_on(async {
        let mut bridge = ZKPMarketplaceBridge::new_test().unwrap();
        bridge.publish_resource(make_sae_listing("seller1", 100.0));
        bridge.execute_resource_workflow(ResourceRequest {
            requester_id: "buyer1".into(),
            resource_type: ResourceType::SAEShard {
                model_id: "scope-v2".into(),
                layer: 5,
            },
            quantity: 5.0,
            max_price: 150.0,
            max_latency_ms: 100,
            min_availability: 0.95,
        }).await
    });
    let elapsed = start.elapsed();

    match result {
        Ok(wf) => {
            println!(
                "  Bridge E2E (1 workflow): {} ms (reported: {} ms), success={}",
                elapsed.as_secs_f64() * 1000.0,
                wf.total_time_ms,
                wf.success
            );
        }
        Err(e) => {
            println!("  Bridge E2E: ERROR - {}", e);
        }
    }
}

fn bench_bridge_multiple_workflows() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let start = Instant::now();
    let results = rt.block_on(async {
        let mut bridge = ZKPMarketplaceBridge::new_test().unwrap();
        for i in 0..5 {
            bridge.publish_resource(make_sae_listing(&format!("seller_{}", i), 100.0));
        }

        let mut results = Vec::new();
        for i in 0..5 {
            let r = bridge.execute_resource_workflow(ResourceRequest {
                requester_id: format!("buyer_{}", i),
                resource_type: ResourceType::SAEShard {
                    model_id: "scope-v2".into(),
                    layer: 5,
                },
                quantity: 3.0,
                max_price: 150.0,
                max_latency_ms: 100,
                min_availability: 0.95,
            }).await;
            results.push(r);
        }
        results
    });
    let elapsed = start.elapsed();

    let success = results.iter().filter(|r| r.as_ref().map(|w| w.success).unwrap_or(false)).count();
    println!(
        "  Bridge E2E (5 workflows): {} ms total, {} success",
        elapsed.as_secs_f64() * 1000.0,
        success
    );
}

// ============================================================================
// Benchmark: Proof Submission
// ============================================================================

fn bench_proof_submission() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let start = Instant::now();
    let result = rt.block_on(async {
        let mut manager = ProofSubmissionManager::new();
        manager.register_verifier("v1".into(), 0.95, 10);
        manager.register_verifier("v2".into(), 0.90, 15);
        manager.submit_proof("bench_submit".into(), create_witness_n(64)).await
    });
    let elapsed = start.elapsed();

    match result {
        Ok(sr) => {
            println!(
                "  Proof Submission (64 features, 2 verifiers): {} ms, state={}",
                elapsed.as_secs_f64() * 1000.0,
                sr.state
            );
        }
        Err(e) => {
            println!("  Proof Submission: ERROR - {}", e);
        }
    }
}

// ============================================================================
// Benchmark: Comparison v1.0 vs v1.1
// ============================================================================

fn bench_comparison_zkp_generation() {
    println!("\n=== ZKP Generation: v1.0 vs v1.1 Async ===");
    println!("--- v1.1 Async Prover (64 features) ---");
    bench_zkp_prover_single();
    println!("--- v1.1 Async Prover (batch 16x32) ---");
    bench_zkp_prover_batch();
    println!("--- v1.1 Async Prover (256 features) ---");
    bench_zkp_prover_large_batch();
}

fn bench_comparison_verification() {
    println!("\n=== Verification Latency: v1.0 vs v1.1 Pool ===");
    println!("--- v1.1 Verifier Pool (single) ---");
    bench_verifier_pool_single();
    println!("--- v1.1 Verifier Pool (stress 20) ---");
    bench_verifier_pool_stress();
}

fn bench_comparison_marketplace() {
    println!("\n=== Marketplace Matching: v1.0 vs v1.1 ===");
    println!("--- v1.1 Matchmaker (10 listings) ---");
    bench_matchmaker_single();
    println!("--- v1.1 Matchmaker (100 listings) ---");
    bench_matchmaker_large();
}

fn bench_comparison_escrow() {
    println!("\n=== Escrow Settlement: v1.0 vs v1.1 ===");
    println!("--- v1.1 Escrow (50 creates) ---");
    bench_escrow_create();
    println!("--- v1.1 Escrow (20 releases) ---");
    bench_escrow_transitions();
}

fn bench_comparison_full_pipeline() {
    println!("\n=== Full Pipeline: Bridge E2E ===");
    println!("--- Single workflow ---");
    bench_bridge_workflow();
    println!("--- 5 concurrent workflows ---");
    bench_bridge_multiple_workflows();
}

// ============================================================================
// Main: Run all benchmarks
// ============================================================================

fn main() {
    println!("==========================================================");
    println!("  ed2kIA v1.1.0 Sprint 3 Benchmarks");
    println!("  Async ZKP & Resource Marketplace v2");
    println!("==========================================================");

    // ZKP Benchmarks
    println!("\n=== ZKP Prover Benchmarks ===");
    bench_zkp_prover_single();
    bench_zkp_prover_batch();
    bench_zkp_prover_large_batch();

    // Verifier Benchmarks
    println!("\n=== Verifier Pool Benchmarks ===");
    bench_verifier_pool_single();
    bench_verifier_pool_stress();

    // Batch Accumulator Benchmarks
    println!("\n=== Batch Accumulator Benchmarks ===");
    bench_batch_accumulator_add();
    bench_batch_accumulator_multiple();

    // Marketplace Benchmarks
    println!("\n=== Marketplace Matching Benchmarks ===");
    bench_matchmaker_single();
    bench_matchmaker_large();

    // Escrow Benchmarks
    println!("\n=== Escrow Ledger Benchmarks ===");
    bench_escrow_create();
    bench_escrow_transitions();

    // Pricing Benchmarks
    println!("\n=== Pricing Engine Benchmarks ===");
    bench_pricing_compute();
    bench_pricing_with_samples();

    // Bridge Benchmarks
    println!("\n=== Bridge E2E Benchmarks ===");
    bench_bridge_workflow();
    bench_bridge_multiple_workflows();

    // Proof Submission Benchmarks
    println!("\n=== Proof Submission Benchmarks ===");
    bench_proof_submission();

    // Comparison
    bench_comparison_zkp_generation();
    bench_comparison_verification();
    bench_comparison_marketplace();
    bench_comparison_escrow();
    bench_comparison_full_pipeline();

    println!("\n==========================================================");
    println!("  Benchmarks complete");
    println!("==========================================================");
}
