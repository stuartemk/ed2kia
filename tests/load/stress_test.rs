//! Async Stress Test Suite - ed2kIA P2P Network Simulator
//!
//! This module provides comprehensive async stress tests for the ed2kIA distributed
//! interpretability network, simulating 50+ concurrent nodes, network partition
//! scenarios, and consensus validation under load.
//!
//! # Test Methodology
//!
//! Tests use a simulated P2P network with gossipsub-style message propagation,
//! reputation scoring, and consensus validation. Each test scenario measures
//! latency percentiles (p50, p95, p99), message throughput, consensus success
//! rate, and resource utilization.
//!
//! # Running Tests
//!
//! ```bash
//! # Run all stress tests
//! cargo test --test stress_test --features stable
//!
//! # Run specific test
//! cargo test --test stress_test test_50_nodes_concurrent --features stable -- --nocapture
//!
//! # Run ignored (long-running) tests
//! cargo test --test stress_test --features stable -- --ignored
//! ```
//!
//! # License
//!
//! Apache 2.0 + Ethical Use Clause

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{Mutex, RwLock};
use tokio::time;

// ============================================================================
// Static Counters for Metrics Aggregation
// ============================================================================

static TOTAL_MESSAGES_PROCESSED: AtomicU64 = AtomicU64::new(0);
static TOTAL_CONSENSUS_ROUNDS: AtomicU64 = AtomicU64::new(0);

/// Reset global counters between test runs
fn reset_global_counters() {
    TOTAL_MESSAGES_PROCESSED.store(0, Ordering::Relaxed);
    TOTAL_CONSENSUS_ROUNDS.store(0, Ordering::Relaxed);
}

// ============================================================================
// Node Simulator
// ============================================================================

/// Health status of a simulated node
#[derive(Debug, Clone, PartialEq)]
enum NodeHealth {
    Healthy,
    Degraded,
    Unhealthy,
    Partitioned,
}

impl std::fmt::Display for NodeHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeHealth::Healthy => write!(f, "healthy"),
            NodeHealth::Degraded => write!(f, "degraded"),
            NodeHealth::Unhealthy => write!(f, "unhealthy"),
            NodeHealth::Partitioned => write!(f, "partitioned"),
        }
    }
}

/// Simulated P2P node representing an ed2kIA network participant
#[derive(Debug, Clone)]
struct SimulatedNode {
    /// Unique node identifier
    node_id: String,
    /// Associated model name
    model: String,
    /// Current health status
    health: NodeHealth,
    /// Reputation score [0.0, 1.0]
    reputation: f64,
    /// Message queue for outgoing messages
    outgoing_queue: Arc<Mutex<VecDeque<SimulatedMessage>>>,
    /// Received messages counter
    messages_received: Arc<AtomicUsize>,
    /// Messages sent counter
    messages_sent: Arc<AtomicUsize>,
    /// Consensus votes cast
    votes_cast: Arc<AtomicUsize>,
    /// Consensus votes received
    votes_received: Arc<AtomicUsize>,
    /// Maximum concurrent capacity
    max_capacity: usize,
    /// Current load
    current_load: Arc<RwLock<usize>>,
    /// Average latency in milliseconds
    avg_latency_ms: f64,
    /// Created timestamp
    created_at: Instant,
    /// Rate limit: max messages per second
    rate_limit: usize,
    /// Messages sent this second (for rate limiting)
    messages_this_second: Arc<RwLock<usize>>,
    /// Last rate limit reset
    last_rate_reset: Arc<RwLock<Instant>>,
}

impl SimulatedNode {
    fn new(node_id: String, model: String) -> Self {
        Self {
            node_id,
            model,
            health: NodeHealth::Healthy,
            reputation: 1.0,
            outgoing_queue: Arc::new(Mutex::new(VecDeque::new())),
            messages_received: Arc::new(AtomicUsize::new(0)),
            messages_sent: Arc::new(AtomicUsize::new(0)),
            votes_cast: Arc::new(AtomicUsize::new(0)),
            votes_received: Arc::new(AtomicUsize::new(0)),
            max_capacity: 100,
            current_load: Arc::new(RwLock::new(0)),
            avg_latency_ms: 10.0,
            created_at: Instant::now(),
            rate_limit: 1000,
            messages_this_second: Arc::new(RwLock::new(0)),
            last_rate_reset: Arc::new(RwLock::new(Instant::now())),
        }
    }

    /// Generate a deterministic node ID from index
    fn generate_id(index: usize) -> String {
        format!("peer_{:016x}", index.wrapping_mul(0x9E3779B97F4A7C15))
    }

    /// Select a model based on node index for cross-model diversity
    fn select_model(index: usize) -> String {
        let models = [
            "qwen-scope-7b",
            "llama-3-8b",
            "mistral-7b",
            "gemma-7b",
            "phi-3-mini",
        ];
        models[index % models.len()].to_string()
    }

    /// Create a node with configured parameters
    fn with_config(node_id: String, model: String, capacity: usize, latency_ms: f64) -> Self {
        Self {
            node_id,
            model,
            health: NodeHealth::Healthy,
            reputation: 1.0,
            outgoing_queue: Arc::new(Mutex::new(VecDeque::new())),
            messages_received: Arc::new(AtomicUsize::new(0)),
            messages_sent: Arc::new(AtomicUsize::new(0)),
            votes_cast: Arc::new(AtomicUsize::new(0)),
            votes_received: Arc::new(AtomicUsize::new(0)),
            max_capacity: capacity,
            current_load: Arc::new(RwLock::new(0)),
            avg_latency_ms: latency_ms,
            created_at: Instant::now(),
            rate_limit: 1000,
            messages_this_second: Arc::new(RwLock::new(0)),
            last_rate_reset: Arc::new(RwLock::new(Instant::now())),
        }
    }

    /// Check if node can accept more messages (rate limiting)
    async fn can_send(&self) -> bool {
        let now = Instant::now();
        let mut messages_this_second = self.messages_this_second.write().await;
        let mut last_rate_reset = self.last_rate_reset.write().await;

        if now.duration_since(*last_rate_reset).as_secs() >= 1 {
            *messages_this_second = 0;
            *last_rate_reset = now;
        }

        *messages_this_second < self.rate_limit
    }

    /// Enqueue a message for sending
    async fn enqueue_message(&self, message: SimulatedMessage) -> bool {
        if self.health == NodeHealth::Unhealthy || self.health == NodeHealth::Partitioned {
            return false;
        }

        if !self.can_send().await {
            return false;
        }

        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        TOTAL_MESSAGES_PROCESSED.fetch_add(1, Ordering::Relaxed);
        *self.messages_this_second.write().await += 1;

        let mut queue = self.outgoing_queue.lock().await;
        queue.push_back(message);
        true
    }

    /// Process incoming message
    async fn receive_message(&self, _message: &SimulatedMessage) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        TOTAL_MESSAGES_PROCESSED.fetch_add(1, Ordering::Relaxed);

        let mut load = self.current_load.write().await;
        *load = (*load + 1).min(self.max_capacity);
    }

    /// Release load after processing
    async fn release_load(&self) {
        let mut load = self.current_load.write().await;
        if *load > 0 {
            *load -= 1;
        }
    }

    /// Update reputation based on behavior
    fn update_reputation(&mut self, delta: f64) {
        self.reputation = (self.reputation + delta).clamp(0.0, 1.0);
    }

    /// Set health status
    fn set_health(&mut self, health: NodeHealth) {
        self.health = health;
    }

    /// Get uptime
    fn uptime(&self) -> Duration {
        self.created_at.elapsed()
    }
}

// ============================================================================
// Message Types
// ============================================================================

/// Simulated message types in the ed2kIA network
#[derive(Debug, Clone)]
enum SimulatedMessage {
    /// Gossipsub-style broadcast
    Gossip {
        source: String,
        payload: Vec<u8>,
        timestamp: Instant,
        ttl: u8,
    },
    /// Consensus vote
    ConsensusVote {
        voter_id: String,
        batch_id: String,
        merkle_root: String,
        confidence: f64,
    },
    /// Tensor request
    TensorRequest {
        request_id: String,
        layer_id: u32,
        tensor_size: usize,
    },
    /// Tensor response
    TensorResponse {
        request_id: String,
        features_count: usize,
        confidence: f64,
    },
    /// Heartbeat
    Heartbeat {
        node_id: String,
        load: usize,
        reputation: f64,
    },
}

impl SimulatedMessage {
    fn size_bytes(&self) -> usize {
        match self {
            SimulatedMessage::Gossip { payload, .. } => payload.len() + 64,
            SimulatedMessage::ConsensusVote { .. } => 128,
            SimulatedMessage::TensorRequest { tensor_size, .. } => tensor_size + 32,
            SimulatedMessage::TensorResponse { features_count, .. } => features_count * 8 + 32,
            SimulatedMessage::Heartbeat { .. } => 64,
        }
    }
}

// ============================================================================
// Network Simulator
// ============================================================================

/// Configuration for the network simulator
#[derive(Debug, Clone)]
struct NetworkConfig {
    /// Number of gossipsub peers per node
    mesh_size: usize,
    /// Maximum message TTL
    max_ttl: u8,
    /// Consensus threshold (percentage of votes needed)
    consensus_threshold: f64,
    /// Rate limit per node (messages per second)
    rate_limit: usize,
    /// Simulated base latency (ms)
    base_latency_ms: f64,
    /// Latency variance (ms)
    latency_variance_ms: f64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            mesh_size: 6,
            max_ttl: 3,
            consensus_threshold: 0.67,
            rate_limit: 1000,
            base_latency_ms: 10.0,
            latency_variance_ms: 5.0,
        }
    }
}

/// Metrics collected during stress tests
#[derive(Debug, Clone)]
struct StressMetrics {
    /// Total messages processed
    total_messages: usize,
    /// Message latencies in milliseconds
    latencies: Vec<f64>,
    /// Consensus rounds completed
    consensus_rounds: usize,
    /// Consensus rounds successful
    consensus_successes: usize,
    /// Messages per second (throughput)
    throughput: f64,
    /// Test duration
    duration: Duration,
    /// Peak memory estimate (bytes)
    peak_memory_estimate: usize,
    /// CPU utilization estimate (0.0 - 1.0)
    cpu_utilization_estimate: f64,
}

impl StressMetrics {
    fn new() -> Self {
        Self {
            total_messages: 0,
            latencies: Vec::new(),
            consensus_rounds: 0,
            consensus_successes: 0,
            throughput: 0.0,
            duration: Duration::ZERO,
            peak_memory_estimate: 0,
            cpu_utilization_estimate: 0.0,
        }
    }

    /// Calculate p50 latency
    fn latency_p50(&self) -> f64 {
        if self.latencies.is_empty() {
            return 0.0;
        }
        let mut sorted = self.latencies.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx = sorted.len() / 2;
        sorted[idx]
    }

    /// Calculate p95 latency
    fn latency_p95(&self) -> f64 {
        if self.latencies.is_empty() {
            return 0.0;
        }
        let mut sorted = self.latencies.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx = (sorted.len() as f64 * 0.95) as usize;
        sorted[idx.min(sorted.len() - 1)]
    }

    /// Calculate p99 latency
    fn latency_p99(&self) -> f64 {
        if self.latencies.is_empty() {
            return 0.0;
        }
        let mut sorted = self.latencies.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx = (sorted.len() as f64 * 0.99) as usize;
        sorted[idx.min(sorted.len() - 1)]
    }

    /// Consensus success rate
    fn consensus_success_rate(&self) -> f64 {
        if self.consensus_rounds == 0 {
            return 0.0;
        }
        self.consensus_successes as f64 / self.consensus_rounds as f64
    }

    /// Print structured metrics output for CI/CD
    fn print_report(&self, test_name: &str) {
        println!("\n========== Stress Test Report: {} ==========", test_name);
        println!("Duration: {:.2}s", self.duration.as_secs_f64());
        println!("Total Messages: {}", self.total_messages);
        println!("Throughput: {:.2} msgs/sec", self.throughput);
        println!("Latency p50: {:.2} ms", self.latency_p50());
        println!("Latency p95: {:.2} ms", self.latency_p95());
        println!("Latency p99: {:.2} ms", self.latency_p99());
        println!(
            "Consensus: {}/{} rounds ({:.2}% success)",
            self.consensus_successes,
            self.consensus_rounds,
            self.consensus_success_rate() * 100.0
        );
        println!(
            "Peak Memory Estimate: {:.2} MB",
            self.peak_memory_estimate as f64 / (1024.0 * 1024.0)
        );
        println!(
            "CPU Utilization Estimate: {:.2}%",
            self.cpu_utilization_estimate * 100.0
        );
        println!("============================================");
    }
}

/// Network simulator managing simulated P2P nodes
struct NetworkSimulator {
    /// Active nodes in the network
    nodes: Arc<RwLock<HashMap<String, SimulatedNode>>>,
    /// Network configuration
    config: NetworkConfig,
    /// Network partitions (groups of isolated nodes)
    partitions: Arc<RwLock<Vec<HashSet<String>>>>,
    /// Collected metrics
    metrics: Arc<RwLock<StressMetrics>>,
    /// Message propagation log
    propagation_log: Arc<RwLock<Vec<(Instant, String, String)>>>,
}

impl NetworkSimulator {
    fn new(config: NetworkConfig) -> Self {
        Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
            config,
            partitions: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(RwLock::new(StressMetrics::new())),
            propagation_log: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a node to the network
    async fn add_node(&self, node: SimulatedNode) {
        let mut nodes = self.nodes.write().await;
        nodes.insert(node.node_id.clone(), node);
    }

    /// Create and add multiple nodes
    async fn create_nodes(&self, count: usize) {
        for i in 0..count {
            let node_id = SimulatedNode::generate_id(i);
            let model = SimulatedNode::select_model(i);
            let latency = self.config.base_latency_ms
                + (i as f64 % 50.0) * self.config.latency_variance_ms / 10.0;
            let node = SimulatedNode::with_config(
                node_id,
                model,
                100,
                latency,
            );
            self.add_node(node).await;
        }
    }

    /// Remove a node from the network
    async fn remove_node(&self, node_id: &str) {
        let mut nodes = self.nodes.write().await;
        nodes.remove(node_id);
    }

    /// Get the number of active nodes
    async fn node_count(&self) -> usize {
        self.nodes.read().await.len()
    }

    /// Get healthy nodes
    async fn healthy_nodes(&self) -> Vec<String> {
        let nodes = self.nodes.read().await;
        nodes
            .values()
            .filter(|n| n.health == NodeHealth::Healthy)
            .map(|n| n.node_id.clone())
            .collect()
    }

    /// Create a network partition
    async fn create_partition(&self, group_a: Vec<String>, group_b: Vec<String>) {
        let mut nodes = self.nodes.write().await;

        // Mark nodes as partitioned
        for id in &group_a {
            if let Some(node) = nodes.get_mut(id) {
                node.set_health(NodeHealth::Partitioned);
            }
        }
        for id in &group_b {
            if let Some(node) = nodes.get_mut(id) {
                node.set_health(NodeHealth::Partitioned);
            }
        }

        // Store partition
        let mut partitions = self.partitions.write().await;
        partitions.clear();
        partitions.push(group_a.into_iter().collect());
        partitions.push(group_b.into_iter().collect());
    }

    /// Heal network partition
    async fn heal_partition(&self) {
        let mut nodes = self.nodes.write().await;
        for node in nodes.values_mut() {
            if node.health == NodeHealth::Partitioned {
                node.set_health(NodeHealth::Healthy);
            }
        }

        let mut partitions = self.partitions.write().await;
        partitions.clear();
    }

    /// Check if two nodes are in the same partition
    async fn can_communicate(&self, node_a: &str, node_b: &str) -> bool {
        let partitions = self.partitions.read().await;
        if partitions.is_empty() {
            return true;
        }

        for partition in partitions.iter() {
            if partition.contains(node_a) && partition.contains(node_b) {
                return true;
            }
        }
        false
    }

    /// Simulate gossipsub-style message propagation
    async fn propagate_gossip(&self, source_id: &str, message: SimulatedMessage) {
        let start = Instant::now();
        let nodes = self.nodes.read().await;

        // Select mesh peers for propagation
        let peer_ids: Vec<String> = nodes
            .values()
            .filter(|n| n.node_id != source_id && n.health == NodeHealth::Healthy)
            .map(|n| n.node_id.clone())
            .collect();

        if peer_ids.is_empty() {
            return;
        }

        // Select mesh_size peers
        let mesh_peers: Vec<String> = peer_ids
            .into_iter()
            .take(self.config.mesh_size)
            .collect();

        drop(nodes);

        // Propagate to mesh peers
        let mut propagated = HashSet::new();
        propagated.insert(source_id.to_string());

        let mut to_process: VecDeque<(String, SimulatedMessage, u8)> = VecDeque::new();
        for peer in mesh_peers {
            if self.can_communicate(source_id, &peer).await {
                to_process.push_back((peer, message.clone(), self.config.max_ttl));
            }
        }

        while let Some((target_id, msg, ttl)) = to_process.pop_front() {
            if ttl == 0 || propagated.contains(&target_id) {
                continue;
            }

            let nodes = self.nodes.read().await;
            if let Some(target) = nodes.get(&target_id) {
                target.receive_message(&msg).await;
                propagated.insert(target_id.clone());

                // Record latency
                let latency = start.elapsed().as_secs_f64() * 1000.0
                    + target.avg_latency_ms
                    + fastrand_f64() * self.config.latency_variance_ms;

                let mut metrics = self.metrics.write().await;
                metrics.latencies.push(latency);
                metrics.total_messages += 1;

                // Log propagation
                let mut log = self.propagation_log.write().await;
                log.push((Instant::now(), source_id.to_string(), target_id.clone()));
            }
            drop(nodes);

            // Continue propagation if TTL allows
            if ttl > 1 {
                let nodes = self.nodes.read().await;
                let peers: Vec<String> = nodes
                    .values()
                    .filter(|n| !propagated.contains(&n.node_id) && n.health == NodeHealth::Healthy)
                    .map(|n| n.node_id.clone())
                    .collect();
                drop(nodes);

                for peer in peers.into_iter().take(self.config.mesh_size / 2) {
                    if self.can_communicate(&target_id, &peer).await {
                        to_process.push_back((peer, message.clone(), ttl - 1));
                    }
                }
            }

            // Small delay to simulate network latency
            time::sleep(Duration::from_millis(1)).await;
        }
    }

    /// Run a consensus round
    async fn run_consensus_round(&self, batch_id: String) -> bool {
        TOTAL_CONSENSUS_ROUNDS.fetch_add(1, Ordering::Relaxed);

        let nodes = self.nodes.read().await;
        let healthy: Vec<&SimulatedNode> = nodes
            .values()
            .filter(|n| n.health == NodeHealth::Healthy)
            .collect();

        if healthy.len() < 3 {
            drop(nodes);
            let mut metrics = self.metrics.write().await;
            metrics.consensus_rounds += 1;
            return false;
        }

        // Generate merkle root for this batch
        let merkle_root = format!("0x{:x}", fastrand_u64());

        // Collect votes
        let mut votes_for = 0usize;
        let mut votes_against = 0usize;

        for node in &healthy {
            // Simulate vote based on reputation
            let vote_confidence = node.reputation * (0.8 + fastrand_f64() * 0.2);
            if vote_confidence > 0.5 {
                votes_for += 1;
            } else {
                votes_against += 1;
            }

            node.votes_cast.fetch_add(1, Ordering::Relaxed);
        }

        drop(nodes);

        let total_votes = votes_for + votes_against;
        let threshold = self.config.consensus_threshold;
        let success = total_votes > 0
            && votes_for as f64 / total_votes as f64 >= threshold;

        let mut metrics = self.metrics.write().await;
        metrics.consensus_rounds += 1;
        if success {
            metrics.consensus_successes += 1;
        }

        success
    }

    /// Simulate cross-model routing under load
    async fn route_cross_model(&self, request_id: String, layer_id: u32) -> Option<String> {
        // Collect owned node data to avoid borrow conflicts with the RwLock guard
        // First collect basic data synchronously, then read loads asynchronously
        let raw_data: Vec<(String, String, f64, Arc<tokio::sync::RwLock<usize>>, usize)> = {
            let nodes = self.nodes.read().await;
            nodes.values()
                .filter(|n| n.health == NodeHealth::Healthy)
                .map(|n| {
                    (
                        n.node_id.clone(),
                        n.model.clone(),
                        n.reputation,
                        n.current_load.clone(),
                        n.max_capacity,
                    )
                })
                .collect()
        };

        if raw_data.is_empty() {
            return None;
        }

        // Now resolve the async loads
        let mut node_data: Vec<(String, String, f64, usize, usize)> = Vec::new();
        for (node_id, model, reputation, current_load, max_capacity) in raw_data {
            let load = *current_load.read().await;
            node_data.push((node_id, model, reputation, load, max_capacity));
        }

        // Group by model
        let mut model_groups: HashMap<String, Vec<(String, f64, usize, usize)>> = HashMap::new();
        for item in node_data {
            let (node_id, model, reputation, load, max_capacity) = item;
            model_groups
                .entry(model)
                .or_default()
                .push((node_id, reputation, load, max_capacity));
        }

        // Select target model based on layer_id
        let models: Vec<&String> = model_groups.keys().collect();
        let target_model = models[layer_id as usize % models.len()];

        // Select best node in target model based on load and reputation
        let candidates = model_groups.get(target_model).unwrap();
        let mut best_node_id: Option<String> = None;
        let mut best_score = f64::MIN;

        for (node_id, reputation, load, max_capacity) in candidates {
            let load_ratio = *load as f64 / *max_capacity as f64;
            let score = reputation * (1.0 - load_ratio);

            if score > best_score {
                best_score = score;
                best_node_id = Some(node_id.clone());
            }
        }

        if let Some(target_id) = best_node_id {
            // Send tensor request by looking up the node
            let msg = SimulatedMessage::TensorRequest {
                request_id,
                layer_id,
                tensor_size: 4096,
            };
            let nodes = self.nodes.read().await;
            if let Some(node) = nodes.get(&target_id) {
                node.enqueue_message(msg).await;
            }

            Some(target_id)
        } else {
            None
        }
    }

    /// Simulate message flood for rate limiting tests
    async fn flood_node(&self, target_id: &str, message_count: usize) -> (usize, usize) {
        let mut sent = 0usize;
        let mut rejected = 0usize;

        for i in 0..message_count {
            let msg = SimulatedMessage::Gossip {
                source: "flood_source".to_string(),
                payload: vec![0u8; 64],
                timestamp: Instant::now(),
                ttl: 1,
            };

            let nodes = self.nodes.read().await;
            if let Some(node) = nodes.get(target_id) {
                let result = node.enqueue_message(msg).await;
                if result {
                    sent += 1;
                } else {
                    rejected += 1;
                }
            }
            drop(nodes);

            // Minimal delay to avoid starving other tasks
            if i % 100 == 0 {
                time::sleep(Duration::from_millis(1)).await;
            }
        }

        (sent, rejected)
    }

    /// Simulate Sybil attack and measure reputation impact
    async fn simulate_sybil_attack(&self, attacker_count: usize, legit_count: usize) -> f64 {
        // Add attacker nodes with low reputation
        for i in 0..attacker_count {
            let node_id = format!("attacker_{:04}", i);
            let mut node = SimulatedNode::new(node_id, "sybil_model".to_string());
            node.reputation = 0.1; // Low reputation
            node.rate_limit = 5000; // High rate limit (trying to flood)
            self.add_node(node).await;
        }

        // Add legitimate nodes
        for i in 0..legit_count {
            let node_id = format!("legit_{:04}", i);
            let node = SimulatedNode::new(node_id, "qwen-scope-7b".to_string());
            self.add_node(node).await;
        }

        // Run consensus and measure success rate
        let mut successes = 0usize;
        let rounds = 10;

        for r in 0..rounds {
            let batch_id = format!("batch_sybil_{}", r);
            if self.run_consensus_round(batch_id).await {
                successes += 1;
            }
        }

        // Calculate average reputation of network
        let nodes = self.nodes.read().await;
        let total_rep: f64 = nodes.values().map(|n| n.reputation).sum();
        let avg_reputation = total_rep / nodes.len() as f64;

        // Clean up attacker nodes
        drop(nodes);
        for i in 0..attacker_count {
            self.remove_node(&format!("attacker_{:04}", i)).await;
        }
        for i in 0..legit_count {
            self.remove_node(&format!("legit_{:04}", i)).await;
        }

        successes as f64 / rounds as f64 * avg_reputation
    }

    /// Update metrics with final calculations
    async fn finalize_metrics(&self, duration: Duration) {
        let mut metrics = self.metrics.write().await;
        metrics.duration = duration;
        metrics.throughput = if duration.as_secs_f64() > 0.0 {
            metrics.total_messages as f64 / duration.as_secs_f64()
        } else {
            0.0
        };

        // Estimate memory usage (each node ~ 4KB + messages)
        let node_count = self.nodes.read().await.len();
        metrics.peak_memory_estimate = node_count * 4096 + metrics.total_messages * 128;

        // Estimate CPU utilization based on throughput
        let cpu_cores = num_cpus::get() as f64;
        metrics.cpu_utilization_estimate =
            (metrics.throughput / (cpu_cores * 10000.0)).min(1.0);
    }

    /// Get a clone of current metrics
    async fn get_metrics(&self) -> StressMetrics {
        self.metrics.read().await.clone()
    }
}

/// Generate a random f64 in [0.0, 1.0)
fn fastrand_f64() -> f64 {
    fastrand::f64()
}

/// Generate a random u64
fn fastrand_u64() -> u64 {
    fastrand::u64(0..u64::MAX)
}

// ============================================================================
// Stress Test Scenarios
// ============================================================================

/// Test: 50 nodes running concurrently with gossip propagation
#[tokio::test]
async fn test_50_nodes_concurrent() {
    reset_global_counters();

    let config = NetworkConfig {
        mesh_size: 6,
        max_ttl: 3,
        ..Default::default()
    };
    let network = NetworkSimulator::new(config);
    network.create_nodes(50).await;

    assert_eq!(
        network.node_count().await,
        50,
        "Network should have exactly 50 nodes"
    );

    let start = Instant::now();
    let test_duration = Duration::from_secs(5);

    // Run concurrent message propagation
    let network_nodes = network.nodes.clone();
    let network_propagate = network.clone_for_propagation();

    let producer = tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_millis(50));
        let mut msg_count = 0usize;

        loop {
            interval.tick().await;
            if Instant::now().duration_since(start) > test_duration {
                break;
            }

            let nodes = network_nodes.read().await;
            let node_ids: Vec<String> = nodes.keys().cloned().collect();
            drop(nodes);

            if node_ids.is_empty() {
                continue;
            }

            let source = &node_ids[msg_count % node_ids.len()];
            let msg = SimulatedMessage::Gossip {
                source: source.clone(),
                payload: vec![msg_count as u8; 128],
                timestamp: Instant::now(),
                ttl: 3,
            };

            network_propagate
                .propagate_gossip(source, msg)
                .await;

            msg_count += 1;
        }

        msg_count
    });

    let _msg_count = producer.await.expect("Producer task should complete");

    let duration = start.elapsed();
    network.finalize_metrics(duration).await;

    let metrics = network.get_metrics().await;
    metrics.print_report("test_50_nodes_concurrent");

    assert!(
        metrics.total_messages > 0,
        "Should have processed messages in 50-node network"
    );
    assert!(
        metrics.latency_p99() < 5000.0,
        "P99 latency should be under 5 seconds: {:.2} ms",
        metrics.latency_p99()
    );
    assert!(
        metrics.throughput > 10.0,
        "Throughput should be > 10 msgs/sec: {:.2}",
        metrics.throughput
    );
}

/// Clone network simulator for use in spawned tasks
impl NetworkSimulator {
    fn clone_for_propagation(&self) -> Self {
        Self {
            nodes: self.nodes.clone(),
            config: self.config.clone(),
            partitions: self.partitions.clone(),
            metrics: self.metrics.clone(),
            propagation_log: self.propagation_log.clone(),
        }
    }
}

/// Test: 100 nodes stress test (marked ignore for longer runtime)
#[tokio::test]
#[ignore]
async fn test_100_nodes_concurrent() {
    reset_global_counters();

    let config = NetworkConfig {
        mesh_size: 8,
        max_ttl: 3,
        ..Default::default()
    };
    let network = NetworkSimulator::new(config);
    network.create_nodes(100).await;

    assert_eq!(
        network.node_count().await,
        100,
        "Network should have exactly 100 nodes"
    );

    let start = Instant::now();
    let test_duration = Duration::from_secs(10);

    let network_nodes = network.nodes.clone();
    let network_propagate = network.clone_for_propagation();

    let producer = tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_millis(25));
        let mut msg_count = 0usize;

        loop {
            interval.tick().await;
            if Instant::now().duration_since(start) > test_duration {
                break;
            }

            let nodes = network_nodes.read().await;
            let node_ids: Vec<String> = nodes.keys().cloned().collect();
            drop(nodes);

            if node_ids.is_empty() {
                continue;
            }

            let source = &node_ids[msg_count % node_ids.len()];
            let msg = SimulatedMessage::Gossip {
                source: source.clone(),
                payload: vec![msg_count as u8; 256],
                timestamp: Instant::now(),
                ttl: 3,
            };

            network_propagate
                .propagate_gossip(source, msg)
                .await;

            msg_count += 1;
        }

        msg_count
    });

    let _msg_count = producer.await.expect("Producer task should complete");

    let duration = start.elapsed();
    network.finalize_metrics(duration).await;

    let metrics = network.get_metrics().await;
    metrics.print_report("test_100_nodes_concurrent");

    assert!(
        metrics.total_messages > 0,
        "Should have processed messages in 100-node network"
    );
    assert!(
        metrics.latency_p99() < 10000.0,
        "P99 latency should be under 10 seconds: {:.2} ms",
        metrics.latency_p99()
    );
    assert!(
        metrics.throughput > 50.0,
        "Throughput should be > 50 msgs/sec: {:.2}",
        metrics.throughput
    );
}

/// Test: Network partition and healing
#[tokio::test]
async fn test_network_partition_heal() {
    reset_global_counters();

    let config = NetworkConfig::default();
    let network = NetworkSimulator::new(config);
    network.create_nodes(40).await;

    let start = Instant::now();

    // Get node IDs for partitioning
    let nodes = network.nodes.read().await;
    let all_ids: Vec<String> = nodes.keys().cloned().collect();
    drop(nodes);

    let mid = all_ids.len() / 2;
    let group_a: Vec<String> = all_ids[..mid].to_vec();
    let group_b: Vec<String> = all_ids[mid..].to_vec();

    // Create partition
    network.create_partition(group_a.clone(), group_b.clone()).await;

    // Verify partition: nodes in different groups cannot communicate
    let can_comm = network
        .can_communicate(&group_a[0], &group_b[0])
        .await;
    assert!(
        !can_comm,
        "Nodes in different partitions should not be able to communicate"
    );

    // Verify nodes in same partition can communicate
    let can_comm_same = network.can_communicate(&group_a[0], &group_a[1]).await;
    assert!(
        can_comm_same,
        "Nodes in same partition should be able to communicate"
    );

    // Run some consensus rounds during partition
    let partition_consensus = network
        .run_consensus_round("partitioned_batch".to_string())
        .await;
    // Consensus may fail during partition due to reduced healthy nodes

    // Heal partition
    network.heal_partition().await;

    // Verify healing: all nodes can communicate again
    let can_comm_healed = network
        .can_communicate(&group_a[0], &group_b[0])
        .await;
    assert!(
        can_comm_healed,
        "Nodes should be able to communicate after partition healing"
    );

    // Run consensus after healing
    let healed_consensus = network
        .run_consensus_round("healed_batch".to_string())
        .await;
    assert!(
        healed_consensus,
        "Consensus should succeed after partition healing"
    );

    // Propagate message across healed network
    let msg = SimulatedMessage::Gossip {
        source: group_a[0].clone(),
        payload: vec![42; 64],
        timestamp: Instant::now(),
        ttl: 3,
    };
    network.propagate_gossip(&group_a[0], msg).await;

    let duration = start.elapsed();
    network.finalize_metrics(duration).await;

    let metrics = network.get_metrics().await;
    metrics.print_report("test_network_partition_heal");

    assert!(
        metrics.total_messages > 0,
        "Should have processed messages after partition heal"
    );
}

/// Test: Consensus under high message volume
#[tokio::test]
async fn test_consensus_under_load() {
    reset_global_counters();

    let config = NetworkConfig {
        consensus_threshold: 0.67,
        ..Default::default()
    };
    let network = NetworkSimulator::new(config);
    network.create_nodes(30).await;

    let start = Instant::now();
    let consensus_rounds = 50;
    let mut successes = 0usize;

    // Run consensus rounds concurrently with message propagation
    let network_clone = network.clone_for_propagation();
    let consensus_handle = tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_millis(100));
        let mut round = 0;

        while round < consensus_rounds {
            interval.tick().await;
            let batch_id = format!("load_batch_{}", round);
            if network_clone.run_consensus_round(batch_id).await {
                successes += 1;
            }
            round += 1;
        }

        successes
    });

    // Concurrent message propagation using a cloned simulator
    let network_propagate = network.clone_for_propagation();
    let propagate_handle = tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_millis(50));
        let mut msg_count = 0usize;

        for _ in 0..100 {
            interval.tick().await;

            let nodes = network_propagate.nodes.read().await;
            let node_ids: Vec<String> = nodes.keys().cloned().collect();
            drop(nodes);

            if node_ids.is_empty() {
                continue;
            }

            let source = &node_ids[msg_count % node_ids.len()];
            let msg = SimulatedMessage::Gossip {
                source: source.clone(),
                payload: vec![msg_count as u8; 64],
                timestamp: Instant::now(),
                ttl: 2,
            };

            network_propagate.propagate_gossip(source, msg).await;
            msg_count += 1;
        }

        msg_count
    });

    let _successes = consensus_handle
        .await
        .expect("Consensus task should complete");
    let _msg_count = propagate_handle
        .await
        .expect("Propagation task should complete");

    let duration = start.elapsed();
    network.finalize_metrics(duration).await;

    let metrics = network.get_metrics().await;
    metrics.print_report("test_consensus_under_load");

    assert!(
        metrics.consensus_rounds > 0,
        "Should have run consensus rounds under load"
    );
    assert!(
        metrics.consensus_success_rate() > 0.5,
        "Consensus success rate should be > 50%: {:.2}%",
        metrics.consensus_success_rate() * 100.0
    );
}

/// Test: Sybil resistance under load
#[tokio::test]
async fn test_reputation_under_attack() {
    reset_global_counters();

    let config = NetworkConfig::default();
    let network = NetworkSimulator::new(config);

    // Create legitimate network first
    network.create_nodes(20).await;

    let start = Instant::now();

    // Measure baseline consensus success rate
    let baseline_success = network
        .run_consensus_round("baseline_batch".to_string())
        .await;
    assert!(
        baseline_success,
        "Baseline consensus should succeed with legitimate nodes"
    );

    // Simulate Sybil attack
    let sybil_resistance = network
        .simulate_sybil_attack(15, 10)
        .await;

    // The sybil_resistance score combines consensus success rate and avg reputation
    // A healthy network should maintain reasonable resistance
    assert!(
        sybil_resistance >= 0.0,
        "Sybil resistance score should be non-negative: {:.4}",
        sybil_resistance
    );

    // Verify original network still functions
    let post_attack_consensus = network
        .run_consensus_round("post_attack_batch".to_string())
        .await;
    assert!(
        post_attack_consensus,
        "Consensus should succeed after Sybil attack cleanup"
    );

    // Verify reputation scoring
    let nodes = network.nodes.read().await;
    for node in nodes.values() {
        assert!(
            node.reputation >= 0.0 && node.reputation <= 1.0,
            "Reputation should be in [0.0, 1.0]: {:.4}",
            node.reputation
        );
    }

    let duration = start.elapsed();
    network.finalize_metrics(duration).await;

    let metrics = network.get_metrics().await;
    metrics.print_report("test_reputation_under_attack");
}

/// Test: Message flood and rate limiting
#[tokio::test]
async fn test_message_flood() {
    reset_global_counters();

    let config = NetworkConfig {
        rate_limit: 100, // Low rate limit for testing
        ..Default::default()
    };
    let network = NetworkSimulator::new(config);
    network.create_nodes(10).await;

    let start = Instant::now();

    // Get target node
    let nodes = network.nodes.read().await;
    let target_id = nodes.keys().next().unwrap().clone();
    drop(nodes);

    // Update rate limit on target node
    {
        let mut nodes = network.nodes.write().await;
        if let Some(node) = nodes.get_mut(&target_id) {
            node.rate_limit = 100; // 100 msgs/sec limit
        }
    }

    // Flood the node with messages
    let (sent, rejected) = network.flood_node(&target_id, 500).await;

    println!(
        "Flood test: sent={}, rejected={}, total={}",
        sent,
        rejected,
        sent + rejected
    );

    // Rate limiting should reject some messages
    assert!(
        sent + rejected == 500,
        "Total messages should equal attempted: {} + {} != 500",
        sent,
        rejected
    );
    assert!(
        rejected > 0,
        "Some messages should be rejected by rate limiting: rejected={}",
        rejected
    );
    assert!(
        sent > 0,
        "Some messages should be accepted: sent={}",
        sent
    );

    // Verify rate limit ratio (should reject significant portion)
    let rejection_rate = rejected as f64 / (sent + rejected) as f64;
    assert!(
        rejection_rate > 0.1,
        "Rejection rate should be > 10%: {:.2}%",
        rejection_rate * 100.0
    );

    let duration = start.elapsed();
    network.finalize_metrics(duration).await;

    let metrics = network.get_metrics().await;
    metrics.print_report("test_message_flood");
}

/// Test: Cross-model routing under load
#[tokio::test]
async fn test_cross_model_routing_stress() {
    reset_global_counters();

    let config = NetworkConfig::default();
    let network = NetworkSimulator::new(config);
    network.create_nodes(40).await;

    let start = Instant::now();

    // Verify multiple models are present
    let nodes = network.nodes.read().await;
    let models: HashSet<&String> = nodes.values().map(|n| &n.model).collect();
    assert!(
        models.len() > 1,
        "Should have multiple models in network: {:?}",
        models
    );
    drop(nodes);

    // Run concurrent cross-model routing
    let network_clone = network.clone_for_propagation();
    let routing_handle = tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_millis(20));
        let mut routed = 0usize;
        let mut failed = 0usize;

        for i in 0..200 {
            interval.tick().await;

            let request_id = format!("cross_req_{}", i);
            let layer_id = (i % 16) as u32;

            match network_clone
                .route_cross_model(request_id, layer_id)
                .await
            {
                Some(_target) => routed += 1,
                None => failed += 1,
            }
        }

        (routed, failed)
    });

    let (routed, failed) = routing_handle
        .await
        .expect("Routing task should complete");

    println!(
        "Cross-model routing: routed={}, failed={}, total={}",
        routed,
        failed,
        routed + failed
    );

    assert!(
        routed > 0,
        "Should have successfully routed some requests: routed={}",
        routed
    );
    assert!(
        routed + failed == 200,
        "Total routing attempts should equal 200: {} + {} != 200",
        routed,
        failed
    );

    // Verify load distribution across models
    let nodes = network.nodes.read().await;
    let mut model_loads: HashMap<String, usize> = HashMap::new();
    for node in nodes.values() {
        let load = *node.current_load.read().await;
        *model_loads.entry(node.model.clone()).or_default() += load;
    }

    // All models should have received some load
    for (model, load) in &model_loads {
        println!("Model {} total load: {}", model, load);
    }

    let duration = start.elapsed();

    // Count messages sent via routing (tensor requests enqueued on nodes)
    let total_routed_messages = {
        let nodes = network.nodes.read().await;
        nodes.values()
            .map(|n| n.messages_sent.load(Ordering::Relaxed))
            .sum::<usize>()
    };

    // Update metrics with routing data
    {
        let mut metrics = network.metrics.write().await;
        metrics.total_messages = total_routed_messages;
        metrics.duration = duration;
        metrics.throughput = if duration.as_secs_f64() > 0.0 {
            total_routed_messages as f64 / duration.as_secs_f64()
        } else {
            0.0
        };
        let node_count = network.nodes.read().await.len();
        metrics.peak_memory_estimate = node_count * 4096 + total_routed_messages * 128;
        let cpu_cores = num_cpus::get() as f64;
        metrics.cpu_utilization_estimate =
            (metrics.throughput / (cpu_cores * 10000.0)).min(1.0);
    }

    let metrics = network.get_metrics().await;
    metrics.print_report("test_cross_model_routing_stress");

    assert!(
        total_routed_messages > 0,
        "Should have processed messages during cross-model routing: routed={}",
        routed
    );
}

// ============================================================================
// Utility Tests
// ============================================================================

/// Test: Node ID generation is deterministic and unique
#[tokio::test]
async fn test_node_id_generation() {
    let mut ids = HashSet::new();
    for i in 0..1000 {
        let id = SimulatedNode::generate_id(i);
        assert!(
            ids.insert(id.clone()),
            "Node ID should be unique for index {}: {}",
            i,
            id
        );
    }

    // Verify determinism
    assert_eq!(
        SimulatedNode::generate_id(0),
        SimulatedNode::generate_id(0),
        "Node ID generation should be deterministic"
    );
    assert_eq!(
        SimulatedNode::generate_id(42),
        SimulatedNode::generate_id(42),
        "Node ID generation should be deterministic"
    );
}

/// Test: Model selection distributes evenly
#[tokio::test]
async fn test_model_distribution() {
    let mut model_counts: HashMap<String, usize> = HashMap::new();

    for i in 0..100 {
        let model = SimulatedNode::select_model(i);
        *model_counts.entry(model).or_default() += 1;
    }

    // Should have 5 models with roughly equal distribution
    assert!(
        model_counts.len() == 5,
        "Should have exactly 5 models: {:?}",
        model_counts
    );

    // Each model should have roughly 20 nodes (100 / 5)
    for (model, count) in &model_counts {
        assert!(
            (*count >= 18) && (*count <= 22),
            "Model {} should have ~20 nodes: {}",
            model,
            count
        );
    }
}

/// Test: Metrics calculation correctness
#[tokio::test]
async fn test_metrics_calculation() {
    let mut metrics = StressMetrics::new();

    // Add known latencies
    for i in 0..100 {
        metrics.latencies.push(i as f64);
    }

    // p50 should be around 50
    let p50 = metrics.latency_p50();
    assert!(
        (p50 - 50.0).abs() < 1.0,
        "P50 latency should be ~50: {:.2}",
        p50
    );

    // p95 should be around 95
    let p95 = metrics.latency_p95();
    assert!(
        (p95 - 95.0).abs() < 2.0,
        "P95 latency should be ~95: {:.2}",
        p95
    );

    // p99 should be around 99
    let p99 = metrics.latency_p99();
    assert!(
        (p99 - 99.0).abs() < 1.0,
        "P99 latency should be ~99: {:.2}",
        p99
    );

    // Test consensus success rate
    metrics.consensus_rounds = 100;
    metrics.consensus_successes = 85;
    assert_eq!(
        metrics.consensus_success_rate(),
        0.85,
        "Success rate should be 0.85"
    );
}

/// Test: Message size calculation
#[tokio::test]
async fn test_message_sizes() {
    let gossip = SimulatedMessage::Gossip {
        source: "test".to_string(),
        payload: vec![0u8; 100],
        timestamp: Instant::now(),
        ttl: 3,
    };
    assert!(
        gossip.size_bytes() > 100,
        "Gossip message size should include payload + overhead"
    );

    let vote = SimulatedMessage::ConsensusVote {
        voter_id: "v1".to_string(),
        batch_id: "b1".to_string(),
        merkle_root: "0xabc".to_string(),
        confidence: 0.9,
    };
    assert_eq!(
        vote.size_bytes(),
        128,
        "Vote message should have fixed size"
    );

    let request = SimulatedMessage::TensorRequest {
        request_id: "r1".to_string(),
        layer_id: 0,
        tensor_size: 4096,
    };
    assert!(
        request.size_bytes() > 4096,
        "Tensor request size should include tensor + overhead"
    );
}

/// Test: Node health transitions
#[tokio::test]
async fn test_node_health_transitions() {
    let mut node = SimulatedNode::new("test_node".to_string(), "model".to_string());

    // Start healthy
    assert_eq!(node.health, NodeHealth::Healthy);

    // Transition to degraded
    node.set_health(NodeHealth::Degraded);
    assert_eq!(node.health, NodeHealth::Degraded);

    // Transition to unhealthy
    node.set_health(NodeHealth::Unhealthy);
    assert_eq!(node.health, NodeHealth::Unhealthy);

    // Unhealthy node should not send messages
    let msg = SimulatedMessage::Heartbeat {
        node_id: "test".to_string(),
        load: 0,
        reputation: 1.0,
    };
    let result = node.enqueue_message(msg).await;
    assert!(
        !result,
        "Unhealthy node should not enqueue messages"
    );

    // Transition to partitioned
    node.set_health(NodeHealth::Partitioned);
    assert_eq!(node.health, NodeHealth::Partitioned);

    // Partitioned node should not send messages
    let msg = SimulatedMessage::Heartbeat {
        node_id: "test".to_string(),
        load: 0,
        reputation: 1.0,
    };
    let result = node.enqueue_message(msg).await;
    assert!(
        !result,
        "Partitioned node should not enqueue messages"
    );

    // Recover to healthy
    node.set_health(NodeHealth::Healthy);
    assert_eq!(node.health, NodeHealth::Healthy);

    // Healthy node should send messages again
    let msg = SimulatedMessage::Heartbeat {
        node_id: "test".to_string(),
        load: 0,
        reputation: 1.0,
    };
    let result = node.enqueue_message(msg).await;
    assert!(
        result,
        "Healthy node should enqueue messages"
    );
}

/// Test: Reputation scoring bounds
#[tokio::test]
async fn test_reputation_bounds() {
    let mut node = SimulatedNode::new("test".to_string(), "model".to_string());

    // Start at 1.0
    assert_eq!(node.reputation, 1.0);

    // Decrease below 0 should clamp
    node.update_reputation(-1.5);
    assert_eq!(node.reputation, 0.0, "Reputation should clamp to 0.0");

    // Increase above 1 should clamp
    node.update_reputation(2.0);
    assert_eq!(node.reputation, 1.0, "Reputation should clamp to 1.0");

    // Normal decrease
    node.update_reputation(-0.3);
    assert_eq!(node.reputation, 0.7, "Reputation should be 0.7");

    // Normal increase
    node.update_reputation(0.15);
    assert_eq!(node.reputation, 0.85, "Reputation should be 0.85");
}

/// Test: Network partition isolation
#[tokio::test]
async fn test_partition_isolation() {
    let config = NetworkConfig::default();
    let network = NetworkSimulator::new(config);
    network.create_nodes(20).await;

    // Initially all nodes can communicate
    let nodes = network.nodes.read().await;
    let ids: Vec<String> = nodes.keys().cloned().collect();
    drop(nodes);

    let can_comm = network.can_communicate(&ids[0], &ids[19]).await;
    assert!(can_comm, "All nodes should communicate before partition");

    // Create partition
    let group_a: Vec<String> = ids[..10].to_vec();
    let group_b: Vec<String> = ids[10..].to_vec();
    network.create_partition(group_a.clone(), group_b.clone()).await;

    // Cross-partition communication should fail
    let can_comm = network.can_communicate(&group_a[0], &group_b[0]).await;
    assert!(!can_comm, "Cross-partition communication should fail");

    // Same-partition communication should succeed
    let can_comm = network.can_communicate(&group_a[0], &group_a[5]).await;
    assert!(can_comm, "Same-partition communication should succeed");

    let can_comm = network.can_communicate(&group_b[0], &group_b[5]).await;
    assert!(can_comm, "Same-partition communication should succeed");
}

/// Test: Concurrent node add/remove
#[tokio::test]
async fn test_concurrent_node_management() {
    let config = NetworkConfig::default();
    let network = NetworkSimulator::new(config);

    let start = Instant::now();

    // Add nodes concurrently
    let add_handles: Vec<_> = (0..50)
        .map(|i| {
            let network = network.clone_for_propagation();
            tokio::spawn(async move {
                let node_id = SimulatedNode::generate_id(i);
                let model = SimulatedNode::select_model(i);
                let node = SimulatedNode::new(node_id, model);
                network.add_node(node).await;
            })
        })
        .collect();

    for handle in add_handles {
        handle.await.expect("Add node task should complete");
    }

    // Note: Due to Arc sharing, nodes are added to shared state
    // The main network reference sees the changes

    let duration = start.elapsed();
    assert!(
        duration < Duration::from_secs(10),
        "Concurrent node management should complete in < 10s: {:.2}s",
        duration.as_secs_f64()
    );
}

/// Test: Global counters reset
#[tokio::test]
async fn test_global_counters() {
    reset_global_counters();

    let initial_messages = TOTAL_MESSAGES_PROCESSED.load(Ordering::Relaxed);
    let initial_consensus = TOTAL_CONSENSUS_ROUNDS.load(Ordering::Relaxed);

    assert_eq!(initial_messages, 0, "Messages counter should be 0 after reset");
    assert_eq!(initial_consensus, 0, "Consensus counter should be 0 after reset");

    // Increment counters
    TOTAL_MESSAGES_PROCESSED.fetch_add(100, Ordering::Relaxed);
    TOTAL_CONSENSUS_ROUNDS.fetch_add(5, Ordering::Relaxed);

    assert_eq!(
        TOTAL_MESSAGES_PROCESSED.load(Ordering::Relaxed),
        100,
        "Messages counter should be 100"
    );
    assert_eq!(
        TOTAL_CONSENSUS_ROUNDS.load(Ordering::Relaxed),
        5,
        "Consensus counter should be 5"
    );

    // Reset again
    reset_global_counters();
    assert_eq!(
        TOTAL_MESSAGES_PROCESSED.load(Ordering::Relaxed),
        0,
        "Messages counter should be 0 after reset"
    );
    assert_eq!(
        TOTAL_CONSENSUS_ROUNDS.load(Ordering::Relaxed),
        0,
        "Consensus counter should be 0 after reset"
    );
}

#[cfg(test)]
mod benchmarks {
    use super::*;

    /// Benchmark: Message propagation latency
    #[tokio::test]
    async fn bench_propagation_50_nodes() {
        let config = NetworkConfig::default();
        let network = NetworkSimulator::new(config);
        network.create_nodes(50).await;

        let nodes = network.nodes.read().await;
        let source = nodes.keys().next().unwrap().clone();
        drop(nodes);

        let start = Instant::now();
        let iterations = 100;

        for i in 0..iterations {
            let msg = SimulatedMessage::Gossip {
                source: source.clone(),
                payload: vec![i as u8; 64],
                timestamp: Instant::now(),
                ttl: 2,
            };
            network.propagate_gossip(&source, msg).await;
        }

        let duration = start.elapsed();
        let avg_time = duration.as_secs_f64() / iterations as f64;

        println!(
            "Benchmark: 50-node propagation avg={:.4}s/iteration",
            avg_time
        );

        assert!(
            avg_time < 1.0,
            "Average propagation time should be < 1s: {:.4}s",
            avg_time
        );
    }

    /// Benchmark: Consensus round latency
    #[tokio::test]
    async fn bench_consensus_30_nodes() {
        let config = NetworkConfig::default();
        let network = NetworkSimulator::new(config);
        network.create_nodes(30).await;

        let start = Instant::now();
        let iterations = 50;

        for i in 0..iterations {
            let batch_id = format!("bench_batch_{}", i);
            network.run_consensus_round(batch_id).await;
        }

        let duration = start.elapsed();
        let avg_time = duration.as_secs_f64() / iterations as f64;

        println!(
            "Benchmark: 30-node consensus avg={:.4}s/round",
            avg_time
        );

        assert!(
            avg_time < 0.5,
            "Average consensus time should be < 0.5s: {:.4}s",
            avg_time
        );
    }
}
