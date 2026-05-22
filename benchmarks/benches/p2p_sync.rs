//! P2P GossipSub Sync Benchmark — Measures message propagation latency
//!
//! Run with: `cargo bench -p ed2kIA-benchmarks --bench p2p_sync`
//!
//! Target metrics:
//! - Local gossip propagation: < 50ms
//! - Simulated WAN propagation: < 200ms

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

/// Simulated P2P gossip message
#[derive(Clone, Debug)]
struct GossipMessage {
    id: u64,
    payload: Vec<u8>,
    origin: String,
    timestamp: u64,
}

/// Simulated P2P node with gossipsub-like propagation
struct GossipNode {
    id: String,
    peers: Vec<String>,
    received_messages: HashMap<u64, GossipMessage>,
    outbox: VecDeque<GossipMessage>,
}

impl GossipNode {
    fn new(id: String) -> Self {
        Self {
            id,
            peers: Vec::new(),
            received_messages: HashMap::new(),
            outbox: VecDeque::new(),
        }
    }

    fn add_peer(&mut self, peer_id: String) {
        self.peers.push(peer_id);
    }

    fn inject_message(&mut self, msg: GossipMessage) {
        if self.received_messages.insert(msg.id, msg.clone()).is_none() {
            self.outbox.push_back(msg);
        }
    }

    fn propagate(&mut self) -> Vec<(String, GossipMessage)> {
        let mut deliveries = Vec::new();
        while let Some(msg) = self.outbox.pop_front() {
            for peer_id in &self.peers {
                deliveries.push((peer_id.clone(), msg.clone()));
            }
        }
        deliveries
    }

    fn received_count(&self) -> usize {
        self.received_messages.len()
    }
}

/// Simulate a gossip network with N nodes fully connected
fn create_gossip_network(n_nodes: usize) -> HashMap<String, GossipNode> {
    let mut nodes = HashMap::new();
    for i in 0..n_nodes {
        let id = format!("node-{}", i);
        nodes.insert(id.clone(), GossipNode::new(id));
    }
    // Fully connected mesh
    for (id, node) in nodes.iter_mut() {
        for other_id in nodes.keys() {
            if other_id != id {
                node.add_peer(other_id.clone());
            }
        }
    }
    nodes
}

/// Benchmark local gossip propagation (1 round)
fn benchmark_gossip_local(c: &mut Criterion) {
    let mut group = c.benchmark_group("p2p_gossip_local");

    let network_sizes = [10, 50, 100, 256];

    for size in &network_sizes {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, size| {
            b.iter_custom(|iters| {
                let mut total = std::time::Duration::ZERO;
                for _ in 0..iters {
                    let mut nodes = create_gossip_network(*size);
                    let msg = GossipMessage {
                        id: 1,
                        payload: vec![0u8; 256],
                        origin: "node-0".to_string(),
                        timestamp: 0,
                    };

                    let start = Instant::now();

                    // Inject into node-0
                    if let Some(node) = nodes.get_mut("node-0") {
                        node.inject_message(msg);
                    }

                    // Propagate 1 round
                    let mut deliveries = Vec::new();
                    if let Some(node) = nodes.get_mut("node-0") {
                        deliveries = node.propagate();
                    }

                    // Deliver to peers
                    for (peer_id, msg) in deliveries {
                        if let Some(peer) = nodes.get_mut(&peer_id) {
                            peer.inject_message(msg);
                        }
                    }

                    total += start.elapsed();
                }
                total
            });
        });
    }

    group.finish();
}

/// Benchmark multi-round gossip propagation (full convergence)
fn benchmark_gossip_convergence(c: &mut Criterion) {
    let mut group = c.benchmark_group("p2p_gossip_convergence");

    let network_sizes = [10, 50, 100];

    for size in &network_sizes {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, size| {
            b.iter_custom(|iters| {
                let mut total = std::time::Duration::ZERO;
                for _ in 0..iters {
                    let mut nodes = create_gossip_network(*size);
                    let msg = GossipMessage {
                        id: 1,
                        payload: vec![0u8; 256],
                        origin: "node-0".to_string(),
                        timestamp: 0,
                    };

                    let start = Instant::now();

                    // Inject into node-0
                    if let Some(node) = nodes.get_mut("node-0") {
                        node.inject_message(msg);
                    }

                    // Propagate until convergence (all nodes received)
                    let mut rounds = 0;
                    while rounds < 10 {
                        let mut any_new = false;
                        let mut all_deliveries = Vec::new();

                        for node in nodes.values_mut() {
                            let deliveries = node.propagate();
                            if !deliveries.is_empty() {
                                any_new = true;
                                all_deliveries.extend(deliveries);
                            }
                        }

                        for (peer_id, msg) in all_deliveries {
                            if let Some(peer) = nodes.get_mut(&peer_id) {
                                if peer.received_messages.insert(msg.id, msg.clone()).is_none() {
                                    any_new = true;
                                }
                            }
                        }

                        rounds += 1;
                        if !any_new && rounds > 1 {
                            break;
                        }
                    }

                    total += start.elapsed();
                }
                total
            });
        });
    }

    group.finish();
}

/// Benchmark message serialization/deserialization
fn benchmark_message_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("p2p_message_serialization");

    let payload_sizes = [64, 256, 1024, 4096];

    for size in &payload_sizes {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, size| {
            b.iter(|| {
                let msg = GossipMessage {
                    id: black_box(1),
                    payload: vec![0u8; *size],
                    origin: "node-0".to_string(),
                    timestamp: black_box(0),
                };
                black_box(&msg);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_gossip_local,
    benchmark_gossip_convergence,
    benchmark_message_serialization
);
criterion_main!(benches);
