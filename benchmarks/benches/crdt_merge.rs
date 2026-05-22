//! CRDT Merge Benchmark — Measures CRDT merge performance at scale
//!
//! Run with: `cargo bench -p ed2kIA-benchmarks --bench crdt_merge`
//!
//! Target metrics:
//! - CRDT merge (100 peers): < 5ms
//! - CRDT merge (1000 peers): < 10ms
//! - CRDT merge (10000 peers): < 50ms

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::BTreeMap;
use std::time::Instant;

/// Simplified GCounter CRDT for benchmarking (matches production structure)
#[derive(Debug, Clone, Default)]
struct GCounter {
    counters: BTreeMap<String, u64>,
}

impl GCounter {
    fn new() -> Self {
        Self {
            counters: BTreeMap::new(),
        }
    }

    fn increment(&mut self, node_id: &str) {
        *self.counters.entry(node_id.to_string()).or_insert(0) += 1;
    }

    fn value(&self) -> u64 {
        self.counters.values().sum()
    }

    fn merge(&mut self, other: &GCounter) {
        for (node_id, &count) in &other.counters {
            *self.counters.entry(node_id.clone()).or_insert(0) =
                (*self.counters.get(node_id).unwrap_or(&0)).max(count);
        }
    }
}

/// Simplified PNCounter CRDT for benchmarking
#[derive(Debug, Clone)]
struct PNCounter {
    plus: GCounter,
    minus: GCounter,
}

impl PNCounter {
    fn new() -> Self {
        Self {
            plus: GCounter::new(),
            minus: GCounter::new(),
        }
    }

    fn increment(&mut self, node_id: &str) {
        self.plus.increment(node_id);
    }

    fn decrement(&mut self, node_id: &str) {
        self.minus.increment(node_id);
    }

    fn value(&self) -> i64 {
        self.plus.value() as i64 - self.minus.value() as i64
    }

    fn merge(&mut self, other: &PNCounter) {
        self.plus.merge(&other.plus);
        self.minus.merge(&other.minus);
    }
}

/// Simplified ORSet CRDT for benchmarking
#[derive(Debug, Clone)]
struct ORSet {
    elements: BTreeMap<String, u64>,
    tombstones: BTreeMap<String, u64>,
    counter: u64,
}

impl ORSet {
    fn new() -> Self {
        Self {
            elements: BTreeMap::new(),
            tombstones: BTreeMap::new(),
            counter: 0,
        }
    }

    fn add(&mut self, element: &str, node_id: &str) {
        self.counter += 1;
        self.elements.insert(element.to_string(), self.counter);
        self.tombstones.remove(element);
    }

    fn remove(&mut self, element: &str) {
        if let Some(&tag) = self.elements.get(element) {
            self.tombstones.insert(element.to_string(), tag);
        }
    }

    fn len(&self) -> usize {
        self.elements.len()
    }

    fn merge(&mut self, other: &ORSet) {
        self.counter = self.counter.max(other.counter);

        for (element, &tag) in &other.elements {
            let current_tag = *self.elements.get(element).unwrap_or(&0);
            let tombstone_tag = *self.tombstones.get(element).unwrap_or(&0);

            if tag >= current_tag && tag > tombstone_tag {
                self.elements.insert(element.clone(), tag);
            }
        }

        for (element, &tag) in &other.tombstones {
            let current_tag = *self.tombstones.get(element).unwrap_or(&0);
            if tag >= current_tag {
                self.tombstones.insert(element.clone(), tag);
                if tag > *self.elements.get(element).unwrap_or(&0) {
                    self.elements.remove(element);
                }
            }
        }
    }
}

/// Benchmark GCounter merge with varying peer counts
fn benchmark_gcounter_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("crdt_gcounter_merge");

    let peer_counts = [10, 100, 1000, 10000];

    for n_peers in &peer_counts {
        // Create two counters with n_peers each
        let mut counter_a = GCounter::new();
        let mut counter_b = GCounter::new();

        for i in 0..*n_peers {
            let node_id = format!("node-{}", i);
            counter_a.increment(&node_id);
            counter_b.increment(&node_id);
            // Add some divergence
            if i % 2 == 0 {
                counter_a.increment(&node_id);
            } else {
                counter_b.increment(&node_id);
            }
        }

        group.bench_with_input(BenchmarkId::from_parameter(n_peers), &counter_a, |b, a| {
            b.iter(|| {
                let mut merged = a.clone();
                merged.merge(black_box(&counter_b));
                black_box(merged.value());
            });
        });
    }

    group.finish();
}

/// Benchmark PNCounter merge with varying peer counts
fn benchmark_pncounter_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("crdt_pncounter_merge");

    let peer_counts = [10, 100, 1000];

    for n_peers in &peer_counts {
        let mut counter_a = PNCounter::new();
        let mut counter_b = PNCounter::new();

        for i in 0..*n_peers {
            let node_id = format!("node-{}", i);
            counter_a.increment(&node_id);
            counter_b.increment(&node_id);
            if i % 3 == 0 {
                counter_a.decrement(&node_id);
            }
            if i % 5 == 0 {
                counter_b.decrement(&node_id);
            }
        }

        group.bench_with_input(BenchmarkId::from_parameter(n_peers), &counter_a, |b, a| {
            b.iter(|| {
                let mut merged = a.clone();
                merged.merge(black_box(&counter_b));
                black_box(merged.value());
            });
        });
    }

    group.finish();
}

/// Benchmark ORSet merge with varying element counts
fn benchmark_orset_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("crdt_orset_merge");

    let element_counts = [100, 1000, 5000, 10000];

    for n_elements in &element_counts {
        let mut set_a = ORSet::new();
        let mut set_b = ORSet::new();

        for i in 0..*n_elements {
            let element = format!("elem-{}", i);
            set_a.add(&element, "node-a");
            if i % 2 == 0 {
                set_b.add(&element, "node-b");
            }
            if i % 10 == 0 {
                set_a.remove(&element);
            }
            if i % 7 == 0 {
                set_b.remove(&element);
            }
        }

        group.bench_with_input(BenchmarkId::from_parameter(n_elements), &set_a, |b, a| {
            b.iter(|| {
                let mut merged = a.clone();
                merged.merge(black_box(&set_b));
                black_box(merged.len());
            });
        });
    }

    group.finish();
}

/// Benchmark multi-node convergence (sequential merges)
fn benchmark_multi_node_convergence(c: &mut Criterion) {
    let mut group = c.benchmark_group("crdt_convergence");

    let node_counts = [10, 100, 1000];

    for n_nodes in &node_counts {
        // Create n_nodes counters
        let mut counters: Vec<GCounter> = Vec::with_capacity(*n_nodes);
        for i in 0..*n_nodes {
            let mut counter = GCounter::new();
            let node_id = format!("node-{}", i);
            // Each node increments its own counter 100 times
            for _ in 0..100 {
                counter.increment(&node_id);
            }
            counters.push(counter);
        }

        group.bench_with_input(BenchmarkId::from_parameter(n_nodes), &counters, |b, counters| {
            b.iter(|| {
                let mut result = GCounter::new();
                for counter in counters {
                    result.merge(counter);
                }
                black_box(result.value());
            });
        });
    }

    group.finish();
}

/// Benchmark merge latency with timing measurement
fn benchmark_merge_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("crdt_merge_latency");

    let sizes = [100, 1000, 10000];

    for size in &sizes {
        let mut counter_a = GCounter::new();
        let mut counter_b = GCounter::new();

        for i in 0..*size {
            let node_id = format!("node-{}", i);
            counter_a.increment(&node_id);
            counter_b.increment(&node_id);
        }

        group.bench_with_input(BenchmarkId::from_parameter(size), &counter_a, |b, a| {
            b.iter_custom(|iters| {
                let start = Instant::now();
                for _ in 0..iters {
                    let mut merged = a.clone();
                    merged.merge(black_box(&counter_b));
                    black_box(merged.value());
                }
                start.elapsed()
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_gcounter_merge,
    benchmark_pncounter_merge,
    benchmark_orset_merge,
    benchmark_multi_node_convergence,
    benchmark_merge_latency
);
criterion_main!(benches);
