//! Alignment & Workload Scheduler Benchmarks — Sprint 69: Testnet Hardening & Distributed Alignment Workloads
//!
//! Benchmarks for:
//! - Shard distribution latency (distribute_shards)
//! - Load balance ratio computation
//! - Assignment map building
//! - Fault tolerance redistribution under node failures
//!
//! Run with: `cargo bench --features v9.5-testnet-hardening --bench alignment_benchmarks`

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::HashMap;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Replicate core structures for standalone benchmarking (no crate dependency)
// ---------------------------------------------------------------------------

pub const LATENCY_THRESHOLD_MS: u64 = 50;

pub struct NodeTier {
    pub id: String,
    pub capacity: u64,
    pub latency_ms: u64,
    pub score: f64,
}

pub struct ShardAssignment {
    pub shard_id: u32,
    pub target: String,
    pub fallback: Option<String>,
}

/// Distribute shards across nodes using weighted round-robin by score/capacity.
pub fn distribute_shards(nodes: &[NodeTier], shard_count: u32) -> Vec<ShardAssignment> {
    if nodes.is_empty() || shard_count == 0 {
        return Vec::new();
    }

    let mut assignments = Vec::with_capacity(shard_count as usize);
    let total_score: f64 = nodes.iter().map(|n| n.score * n.capacity as f64).sum();

    let mut weighted_indices: Vec<usize> = Vec::new();
    for (i, node) in nodes.iter().enumerate() {
        if total_score > 0.0 {
            let weight = (node.score * node.capacity as f64 / total_score * shard_count as f64)
                .max(1.0) as usize;
            for _ in 0..weight {
                weighted_indices.push(i);
            }
        } else {
            weighted_indices.push(i);
        }
    }

    while weighted_indices.len() < shard_count as usize {
        weighted_indices.extend(0..nodes.len());
    }

    for s in 0..shard_count {
        let idx = (s as usize) % weighted_indices.len();
        let node_idx = weighted_indices[idx];
        let target = nodes[node_idx].id.clone();
        let fallback = if nodes[node_idx].latency_ms > LATENCY_THRESHOLD_MS {
            let fallback_idx = (node_idx + 1) % nodes.len();
            Some(nodes[fallback_idx].id.clone())
        } else {
            None
        };
        assignments.push(ShardAssignment {
            shard_id: s,
            target,
            fallback,
        });
    }

    assignments
}

/// Build a shard_id → node_id map from assignments.
pub fn build_assignment_map(assignments: &[ShardAssignment]) -> HashMap<u32, &str> {
    assignments
        .iter()
        .map(|a| (a.shard_id, a.target.as_str()))
        .collect()
}

/// Compute load balance ratio: min/max assignments per node.
pub fn load_balance_ratio(assignments: &[ShardAssignment]) -> f64 {
    if assignments.is_empty() {
        return 0.0;
    }

    let mut counts: HashMap<String, u64> = HashMap::new();
    for a in assignments {
        *counts.entry(a.target.clone()).or_insert(0) += 1;
    }

    if counts.is_empty() {
        return 0.0;
    }

    let values: Vec<u64> = counts.values().cloned().collect();
    let max = *values.iter().max().unwrap();
    let min = *values.iter().min().unwrap();

    if max == 0 {
        return 0.0;
    }

    min as f64 / max as f64
}

// ---------------------------------------------------------------------------
// Benchmark helpers
// ---------------------------------------------------------------------------

fn small_cluster() -> Vec<NodeTier> {
    vec![
        NodeTier {
            id: "node1".into(),
            capacity: 150,
            latency_ms: 5,
            score: 0.95,
        },
        NodeTier {
            id: "node2".into(),
            capacity: 200,
            latency_ms: 8,
            score: 0.90,
        },
        NodeTier {
            id: "node3".into(),
            capacity: 100,
            latency_ms: 80,
            score: 0.70,
        },
    ]
}

fn large_cluster() -> Vec<NodeTier> {
    (0..50)
        .map(|i| NodeTier {
            id: format!("node-{}", i),
            capacity: 50 + (i as u64) * 10,
            latency_ms: (i % 5) * 25, // Some nodes have high latency
            score: 0.5 + (i as f64 % 50.0) / 100.0,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

fn bench_distribute_shards_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("distribute_shards/small_cluster");
    for shards in [16, 64, 256] {
        group.bench_with_input(BenchmarkId::from_parameter(shards), &shards, |b, s| {
            let nodes = small_cluster();
            b.iter(|| distribute_shards(black_box(&nodes), black_box(*s)))
        });
    }
    group.finish();
}

fn bench_distribute_shards_large(c: &mut Criterion) {
    let mut group = c.benchmark_group("distribute_shards/large_cluster");
    for shards in [64, 256, 1024, 4096] {
        group.bench_with_input(BenchmarkId::from_parameter(shards), &shards, |b, s| {
            let nodes = large_cluster();
            b.iter(|| distribute_shards(black_box(&nodes), black_box(*s)))
        });
    }
    group.finish();
}

fn bench_build_assignment_map(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_assignment_map");
    for shards in [64, 256, 1024] {
        group.bench_with_input(BenchmarkId::from_parameter(shards), &shards, |b, s| {
            let nodes = small_cluster();
            let assignments = distribute_shards(&nodes, *s);
            b.iter(|| build_assignment_map(black_box(&assignments)))
        });
    }
    group.finish();
}

fn bench_load_balance_ratio(c: &mut Criterion) {
    let mut group = c.benchmark_group("load_balance_ratio");
    for shards in [64, 256, 1024] {
        group.bench_with_input(BenchmarkId::from_parameter(shards), &shards, |b, s| {
            let nodes = small_cluster();
            let assignments = distribute_shards(&nodes, *s);
            b.iter(|| load_balance_ratio(black_box(&assignments)))
        });
    }
    group.finish();
}

fn bench_fault_tolerance_redistribution(c: &mut Criterion) {
    let mut group = c.benchmark_group("fault_tolerance/redistribution");
    let nodes = large_cluster();
    let assignments = distribute_shards(&nodes, 1024);

    // Simulate removing 10 nodes and redistributing
    group.bench_with_input("10_node_failure", &10, |b, _failures| {
        b.iter(|| {
            let surviving: Vec<&NodeTier> = nodes.iter().skip(10).collect();
            let owned: Vec<NodeTier> = surviving.iter().map(|n| (*n).clone()).collect();
            distribute_shards(&owned, 1024)
        })
    });

    group.bench_with_input("25_node_failure", &25, |b, _failures| {
        b.iter(|| {
            let surviving: Vec<&NodeTier> = nodes.iter().skip(25).collect();
            let owned: Vec<NodeTier> = surviving.iter().map(|n| (*n).clone()).collect();
            distribute_shards(&owned, 1024)
        })
    });

    group.finish();
}

fn bench_end_to_end_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("end_to_end/full_pipeline");
    for shards in [64, 256, 1024] {
        group.bench_with_input(BenchmarkId::from_parameter(shards), &shards, |b, s| {
            let nodes = large_cluster();
            b.iter(|| {
                let assignments = distribute_shards(&nodes, *s);
                let map = build_assignment_map(&assignments);
                let ratio = load_balance_ratio(&assignments);
                black_box((assignments, map, ratio))
            })
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Criterion registration
// ---------------------------------------------------------------------------

criterion_group!(
    benches,
    bench_distribute_shards_small,
    bench_distribute_shards_large,
    bench_build_assignment_map,
    bench_load_balance_ratio,
    bench_fault_tolerance_redistribution,
    bench_end_to_end_pipeline,
);
criterion_main!(benches);
