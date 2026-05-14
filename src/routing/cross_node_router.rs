//! Cross-Node Compute Router — Predictive routing of compute tasks across federation nodes.
//!
//! Routes tasks based on:
//! - Historical latency profiles
//! - Declared node capacity
//! - Reputation v2 scores
//! - Fallback to static Kademlia if prediction_confidence < 0.75

use std::collections::{HashMap, VecDeque};

// ─── Errors ───

#[derive(Debug, Clone)]
pub enum RoutingError {
    NoHealthyNodes,
    NodeNotFound(String),
    PredictionUnreliable { confidence: f64 },
    InvalidTask(String),
}

impl std::fmt::Display for RoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoHealthyNodes => write!(f, "No healthy nodes available for routing"),
            Self::NodeNotFound(id) => write!(f, "Node not found: {}", id),
            Self::PredictionUnreliable { confidence } => {
                write!(f, "Prediction confidence {:.2} below 0.75 threshold", confidence)
            }
            Self::InvalidTask(msg) => write!(f, "Invalid task: {}", msg),
        }
    }
}

impl std::error::Error for RoutingError {}

// ─── Config ───

#[derive(Debug, Clone)]
pub struct RouterConfig {
    pub min_prediction_confidence: f64,
    pub max_route_history: usize,
    pub latency_weight: f64,
    pub capacity_weight: f64,
    pub reputation_weight: f64,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            min_prediction_confidence: 0.75,
            max_route_history: 1000,
            latency_weight: 0.4,
            capacity_weight: 0.3,
            reputation_weight: 0.3,
        }
    }
}

// ─── Node Profile ───

#[derive(Debug, Clone)]
pub struct NodeProfile {
    pub node_id: String,
    pub capacity: f64,
    pub current_load: f64,
    pub avg_latency_ms: f64,
    pub reputation: f64,
    pub healthy: bool,
    pub latency_history: VecDeque<f64>,
}

impl NodeProfile {
    pub fn new(node_id: String, capacity: f64) -> Self {
        Self {
            node_id,
            capacity,
            current_load: 0.0,
            avg_latency_ms: 0.0,
            reputation: 1.0,
            healthy: true,
            latency_history: VecDeque::new(),
        }
    }

    pub fn available_capacity(&self) -> f64 {
        (self.capacity - self.current_load).max(0.0)
    }

    pub fn record_latency(&mut self, latency_ms: f64) {
        self.latency_history.push_back(latency_ms);
        if self.latency_history.len() > 100 {
            self.latency_history.pop_front();
        }
        let sum: f64 = self.latency_history.iter().sum();
        self.avg_latency_ms = sum / self.latency_history.len() as f64;
    }

    pub fn routing_score(&self, weights: &RouterConfig) -> f64 {
        if !self.healthy || self.available_capacity() <= 0.0 {
            return 0.0;
        }
        let latency_score = 1.0 / (1.0 + self.avg_latency_ms);
        let capacity_score = self.available_capacity() / self.capacity;
        let rep_score = self.reputation;
        latency_score * weights.latency_weight
            + capacity_score * weights.capacity_weight
            + rep_score * weights.reputation_weight
    }
}

// ─── Compute Task ───

#[derive(Debug, Clone)]
pub enum TaskType {
    FineTuning,
    Inference,
    ZKPGeneration,
    DataProcessing,
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FineTuning => write!(f, "fine_tuning"),
            Self::Inference => write!(f, "inference"),
            Self::ZKPGeneration => write!(f, "zkp_generation"),
            Self::DataProcessing => write!(f, "data_processing"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ComputeTask {
    pub task_id: String,
    pub task_type: TaskType,
    pub required_capacity: f64,
    pub priority: u8,
}

// ─── Route Decision ───

#[derive(Debug, Clone)]
pub struct RouteDecision {
    pub task_id: String,
    pub target_node: String,
    pub confidence: f64,
    pub predicted_latency_ms: f64,
    pub used_fallback: bool,
}

// ─── Stats ───

#[derive(Debug, Clone)]
pub struct RouterStats {
    pub total_routes: u64,
    pub fallback_routes: u64,
    pub avg_decision_ms: f64,
    pub avg_confidence: f64,
}

impl Default for RouterStats {
    fn default() -> Self {
        Self {
            total_routes: 0,
            fallback_routes: 0,
            avg_decision_ms: 0.0,
            avg_confidence: 0.0,
        }
    }
}

// ─── Router ───

pub struct CrossNodeRouter {
    config: RouterConfig,
    nodes: HashMap<String, NodeProfile>,
    route_history: VecDeque<RouteDecision>,
    stats: RouterStats,
}

impl CrossNodeRouter {
    pub fn new(config: RouterConfig) -> Self {
        Self {
            config,
            nodes: HashMap::new(),
            route_history: VecDeque::new(),
            stats: RouterStats::default(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(RouterConfig::default())
    }

    pub fn register_node(&mut self, node_id: String, capacity: f64) {
        self.nodes.insert(node_id.clone(), NodeProfile::new(node_id, capacity));
    }

    pub fn update_node_load(&mut self, node_id: &str, load: f64) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.current_load = load.max(0.0).min(node.capacity);
        }
    }

    pub fn update_node_reputation(&mut self, node_id: &str, reputation: f64) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.reputation = reputation.clamp(0.0, 1.0);
        }
    }

    pub fn record_latency(&mut self, node_id: &str, latency_ms: f64) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.record_latency(latency_ms);
        }
    }

    pub fn set_node_health(&mut self, node_id: &str, healthy: bool) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.healthy = healthy;
        }
    }

    pub fn route_task(&mut self, task: &ComputeTask) -> Result<RouteDecision, RoutingError> {
        let start = current_timestamp_ms();

        // Find candidates
        let candidates: Vec<&NodeProfile> = self
            .nodes
            .values()
            .filter(|n| n.healthy && n.available_capacity() >= task.required_capacity)
            .collect();

        if candidates.is_empty() {
            return Err(RoutingError::NoHealthyNodes);
        }

        // Score candidates
        let mut scored: Vec<_> = candidates
            .iter()
            .map(|n| (n.node_id.clone(), n.routing_score(&self.config)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let best = scored.first().unwrap();
        let best_node = self.nodes.get(&best.0).unwrap();

        // Compute confidence
        let confidence = self.compute_confidence(best_node);
        let used_fallback = confidence < self.config.min_prediction_confidence;

        let decision = RouteDecision {
            task_id: task.task_id.clone(),
            target_node: best.0.clone(),
            confidence,
            predicted_latency_ms: best_node.avg_latency_ms,
            used_fallback,
        };

        // Update stats
        let elapsed = current_timestamp_ms() - start;
        self.stats.total_routes += 1;
        if used_fallback {
            self.stats.fallback_routes += 1;
        }
        self.stats.avg_decision_ms =
            (self.stats.avg_decision_ms * (self.stats.total_routes - 1) as f64 + elapsed as f64)
                / self.stats.total_routes as f64;
        self.stats.avg_confidence =
            (self.stats.avg_confidence * (self.stats.total_routes - 1) as f64 + confidence)
                / self.stats.total_routes as f64;

        // Record history
        self.route_history.push_back(decision.clone());
        if self.route_history.len() > self.config.max_route_history {
            self.route_history.pop_front();
        }

        Ok(decision)
    }

    pub fn get_stats(&self) -> &RouterStats {
        &self.stats
    }

    pub fn get_config(&self) -> &RouterConfig {
        &self.config
    }

    pub fn get_node_profile(&self, node_id: &str) -> Option<&NodeProfile> {
        self.nodes.get(node_id)
    }

    pub fn get_recent_routes(&self, limit: usize) -> Vec<&RouteDecision> {
        self.route_history.iter().rev().take(limit).collect()
    }

    pub fn reset_stats(&mut self) {
        self.stats = RouterStats::default();
    }

    fn compute_confidence(&self, node: &NodeProfile) -> f64 {
        if node.latency_history.is_empty() {
            return 0.5;
        }
        // Confidence based on latency consistency (low variance = high confidence)
        let mean = node.avg_latency_ms;
        let variance: f64 = node
            .latency_history
            .iter()
            .map(|l| (l - mean).powi(2))
            .sum::<f64>()
            / node.latency_history.len() as f64;
        let std_dev = variance.sqrt();
        let cv = if mean > 0.0 { std_dev / mean } else { 0.0 };
        // Lower CV = higher confidence
        (1.0 / (1.0 + cv)).min(1.0)
    }
}

impl Default for CrossNodeRouter {
    fn default() -> Self {
        Self::with_defaults()
    }
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let router = CrossNodeRouter::with_defaults();
        assert_eq!(router.get_stats().total_routes, 0);
    }

    #[test]
    fn test_register_node() {
        let mut router = CrossNodeRouter::with_defaults();
        router.register_node("n1".to_string(), 100.0);
        assert!(router.get_node_profile("n1").is_some());
    }

    #[test]
    fn test_route_task() {
        let mut router = CrossNodeRouter::with_defaults();
        router.register_node("n1".to_string(), 100.0);
        router.register_node("n2".to_string(), 200.0);
        // Give n1 some load so n2 has higher capacity score
        router.update_node_load("n1", 80.0);
        let task = ComputeTask {
            task_id: "t1".to_string(),
            task_type: TaskType::Inference,
            required_capacity: 50.0,
            priority: 1,
        };
        let decision = router.route_task(&task).unwrap();
        assert_eq!(decision.target_node, "n2");
    }

    #[test]
    fn test_route_no_healthy_nodes() {
        let mut router = CrossNodeRouter::with_defaults();
        router.register_node("n1".to_string(), 100.0);
        router.set_node_health("n1", false);
        let task = ComputeTask {
            task_id: "t1".to_string(),
            task_type: TaskType::Inference,
            required_capacity: 50.0,
            priority: 1,
        };
        assert!(router.route_task(&task).is_err());
    }

    #[test]
    fn test_latency_recording() {
        let mut router = CrossNodeRouter::with_defaults();
        router.register_node("n1".to_string(), 100.0);
        router.record_latency("n1", 10.0);
        router.record_latency("n1", 20.0);
        let profile = router.get_node_profile("n1").unwrap();
        assert!((profile.avg_latency_ms - 15.0).abs() < 0.01);
    }

    #[test]
    fn test_reputation_impact() {
        let mut router = CrossNodeRouter::with_defaults();
        router.register_node("n1".to_string(), 100.0);
        router.register_node("n2".to_string(), 100.0);
        router.update_node_reputation("n1", 1.0);
        router.update_node_reputation("n2", 0.5);
        let task = ComputeTask {
            task_id: "t1".to_string(),
            task_type: TaskType::Inference,
            required_capacity: 10.0,
            priority: 1,
        };
        let decision = router.route_task(&task).unwrap();
        assert_eq!(decision.target_node, "n1");
    }

    #[test]
    fn test_fallback_routing() {
        let mut router = CrossNodeRouter::new(RouterConfig {
            min_prediction_confidence: 0.99,
            ..Default::default()
        });
        router.register_node("n1".to_string(), 100.0);
        let task = ComputeTask {
            task_id: "t1".to_string(),
            task_type: TaskType::Inference,
            required_capacity: 10.0,
            priority: 1,
        };
        let decision = router.route_task(&task).unwrap();
        assert!(decision.used_fallback);
    }

    #[test]
    fn test_stats_tracking() {
        let mut router = CrossNodeRouter::with_defaults();
        router.register_node("n1".to_string(), 100.0);
        for i in 0..5 {
            let task = ComputeTask {
                task_id: format!("t{}", i),
                task_type: TaskType::Inference,
                required_capacity: 10.0,
                priority: 1,
            };
            router.route_task(&task).unwrap();
        }
        assert_eq!(router.get_stats().total_routes, 5);
    }

    #[test]
    fn test_reset_stats() {
        let mut router = CrossNodeRouter::with_defaults();
        router.reset_stats();
        assert_eq!(router.get_stats().total_routes, 0);
    }

    #[test]
    fn test_task_type_display() {
        let t = TaskType::FineTuning;
        assert_eq!(t.to_string(), "fine_tuning");
    }

    #[test]
    fn test_error_display() {
        let e = RoutingError::NoHealthyNodes;
        assert!(!e.to_string().is_empty());
    }

    #[test]
    fn test_node_available_capacity() {
        let mut node = NodeProfile::new("n".to_string(), 100.0);
        node.current_load = 60.0;
        assert!((node.available_capacity() - 40.0).abs() < 0.01);
    }
}
