//! Omni-Node Scaling Benchmarks — Sprint 48
//!
//! Benchmarks for Omni-Node throughput, SCT routing latency, CE ledger concurrency,
//! and Migration Handshake scale. Baseline for v3.0.0-stable release.
//!
//! Run with: `cargo bench --features "v3.0-scaling-bench" --bench omni_node_scaling`

#![cfg(feature = "v3.0-scaling-bench")]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

// ---------------------------------------------------------------------------
// Reusable test data generators
// ---------------------------------------------------------------------------

/// Generate a sequence of CE weights for inter-pillar messages.
fn generate_ce_weights(count: usize) -> Vec<f64> {
    (0..count).map(|i| 10.0 + (i % 50) as f64).collect()
}

/// Generate simulated SCT tensor values (X, Y, Z) — all ethical (Z ≥ 0).
fn generate_ethical_tensors(count: usize) -> Vec<(f32, f32, f32)> {
    (0..count)
        .map(|i| {
            let x = 0.3 + (i % 7) as f32 * 0.1;
            let y = 0.1 + (i % 5) as f32 * 0.08;
            let z = 0.0 + (i % 10) as f32 * 0.05;
            (x, y, z, z)
        })
        .map(|(_, x, y, z)| (x, y, z))
        .collect()
}

/// Generate simulated SCT tensor values including some unethical (Z < 0).
fn generate_mixed_tensors(count: usize) -> Vec<(f32, f32, f32)> {
    (0..count)
        .map(|i| {
            let x = 0.3 + (i % 7) as f32 * 0.1;
            let y = 0.1 + (i % 5) as f32 * 0.08;
            let z = if i % 5 == 0 { -0.1 } else { 0.05 };
            (x, y, z)
        })
        .collect()
}

/// Simulate cluster handshake payloads for migration negotiation.
fn generate_handshake_payloads(count: usize) -> Vec<Vec<u8>> {
    (0..count)
        .map(|i| format!("cluster-{:04u}", i).into_bytes())
        .collect()
}

// ---------------------------------------------------------------------------
// Benchmark: Omni-Node Throughput
//
// Measures inter-pillar message routing throughput with SCT validation.
// Simulates SymbioticRouter processing messages between 4 pillars.
// ---------------------------------------------------------------------------

fn benchmark_omni_node_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("omni_node/throughput");

    let sizes = [100, 500, 1_000, 5_000, 10_000];

    for size in sizes {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, size| {
            let ce_weights = generate_ce_weights(*size);
            let tensors = generate_ethical_tensors(*size);
            b.iter(|| {
                let mut approved = 0u32;
                let mut rejected = 0u32;
                for (i, &ce) in ce_weights.iter().enumerate() {
                    let (x, y, z) = tensors[i];
                    // Simulate SCT Guard: Z ≥ 0
                    let ethical = z >= 0.0;
                    // Simulate CE check
                    let sufficient_ce = ce >= 0.5;
                    if ethical && sufficient_ce {
                        approved += 1;
                    } else {
                        rejected += 1;
                    }
                }
                black_box((approved, rejected));
            });
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark: SCT Routing Latency
//
// Measures p50/p95 latency of Z ≥ 0 validation under concurrent load.
// Simulates SymbiosisValidator processing tensors.
// ---------------------------------------------------------------------------

fn benchmark_sct_routing_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("omni_node/sct_latency");

    let batch_sizes = [10, 100, 500, 1_000];

    for size in batch_sizes {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("validation", size), &size, |b, size| {
            let tensors = generate_mixed_tensors(*size);
            b.iter(|| {
                let mut results = Vec::with_capacity(*size);
                for (x, y, z) in &tensors {
                    // Simulate full SCT evaluation: sigmoid(Z) → decision
                    let z_clamped = z.clamp(-5.0, 5.0);
                    let sigmoid = 1.0 / (1.0 + (-z_clamped).exp());
                    let approved = sigmoid >= 0.5; // Z ≥ 0 threshold
                    let stewardship = (1.0 - (*x / (*x + *y + 1e-6))) * z.signum().max(0.0);
                    results.push((approved, stewardship));
                }
                black_box(results);
            });
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark: CE Ledger Concurrency
//
// Measures operations/sec for deposit/withdraw in ExistentialCreditLedger.
// Simulates concurrent pillar CE tracking.
// ---------------------------------------------------------------------------

fn benchmark_ce_ledger_concurrency(c: &mut Criterion) {
    let mut group = c.benchmark_group("omni_node/ce_ledger");

    let operations = [100, 1_000, 5_000, 10_000];

    for size in operations {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("deposit_withdraw", size), &size, |b, size| {
            let amounts = generate_ce_weights(*size);
            b.iter(|| {
                // Simulate 4 pillars with independent CE ledgers
                let mut ledgers = [0.0f64; 4];
                let mut successes = 0u32;
                let mut failures = 0u32;
                for (i, &amount) in amounts.iter().enumerate() {
                    let pillar = (i % 4);
                    // Deposit
                    ledgers[pillar] += amount;
                    // Withdraw half
                    let withdraw_amount = amount * 0.5;
                    if ledgers[pillar] >= withdraw_amount {
                        ledgers[pillar] -= withdraw_amount;
                        successes += 1;
                    } else {
                        failures += 1;
                    }
                }
                black_box((ledgers, successes, failures));
            });
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark: Migration Handshake Scale
//
// Measures negotiation throughput for cluster onboarding.
// Simulates MigrationNegotiator processing handshakes.
// ---------------------------------------------------------------------------

fn benchmark_migration_handshake_scale(c: &mut Criterion) {
    let mut group = c.benchmark_group("omni_node/migration_handshake");

    let cluster_counts = [10, 50, 100, 500];

    for count in cluster_counts {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::new("negotiation", count), &count, |b, count| {
            let payloads = generate_handshake_payloads(*count);
            let tensors = generate_ethical_tensors(*count);
            let max_capacity = 10_000_000u64;
            b.iter(|| {
                let mut accepted = 0u32;
                let mut rejected_capacity = 0u32;
                let mut rejected_ethics = 0u32;
                let mut total_capacity = 0u64;

                for (i, payload) in payloads.iter().enumerate() {
                    let cluster_size = (payload.len() as u64) * 1_000;
                    let (_, _, z) = tensors[i];

                    // Capacity check
                    if total_capacity + cluster_size > max_capacity {
                        rejected_capacity += 1;
                        continue;
                    }

                    // SCT validation
                    if z < 0.0 {
                        rejected_ethics += 1;
                        continue;
                    }

                    // Accept cluster
                    total_capacity += cluster_size;
                    accepted += 1;
                }

                black_box((accepted, rejected_capacity, rejected_ethics, total_capacity));
            });
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark: Full Symbiotic Ignition Cycle
//
// End-to-end simulation: Migration → Hypothesis → Exchange → Route
// ---------------------------------------------------------------------------

fn benchmark_full_ignition_cycle(c: &mut Criterion) {
    let mut group = c.benchmark_group("omni_node/full_ignition_cycle");

    let cycle_sizes = [50, 200, 500];

    for size in cycle_sizes {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("cycle", size), &size, |b, size| {
            let ce_weights = generate_ce_weights(*size);
            let tensors = generate_ethical_tensors(*size);
            let payloads = generate_handshake_payloads(*size);
            b.iter(|| {
                let mut total_score = 0.0f64;

                // Phase 1: Migration handshake validation
                for (i, payload) in payloads.iter().enumerate() {
                    let (_, _, z) = tensors[i];
                    if z >= 0.0 {
                        total_score += payload.len() as f64;
                    }
                }

                // Phase 2: CE ledger operations
                let mut ledger = 0.0f64;
                for &ce in &ce_weights {
                    ledger += ce;
                    if ledger >= 50.0 {
                        ledger -= 25.0;
                        total_score += 1.0;
                    }
                }

                // Phase 3: SCT batch validation
                for (x, y, z) in &tensors {
                    let stewardship = if *z >= 0.0 {
                        1.0 - (*x / (*x + *y + 1e-6))
                    } else {
                        0.0
                    };
                    total_score += stewardship as f64;
                }

                black_box(total_score);
            });
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Criterion Main
// ---------------------------------------------------------------------------

criterion_group! {
    name = omni_node_benchmarks;
    config = Criterion::default()
        .sample_size(100)
        .warm_up_time(std::time::Duration::from_secs(1));
    targets =
        benchmark_omni_node_throughput,
        benchmark_sct_routing_latency,
        benchmark_ce_ledger_concurrency,
        benchmark_migration_handshake_scale,
        benchmark_full_ignition_cycle,
}

criterion_main!(omni_node_benchmarks);
