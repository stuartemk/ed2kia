//! Load Balancer — Dynamic load distribution across federation nodes.
//!
//! Implements weighted round-robin with health-aware routing and
//! automatic rebalancing when load skew exceeds threshold.

use std::collections::{HashMap, VecDeque};

// ─── Errors ───

#[derive(Debug, Clone)]
pub enum BalancerError {
    NoNodesAvailable,
    NodeNotFound(String),
    RebalanceFailed(String),
}

impl std::fmt::Display for BalancerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoNodesAvailable => write!(f, "No nodes available for balancing"),
            Self::NodeNotFound(id) => write!(f, "Node not found: {}", id),
            Self::RebalanceFailed(msg) => write!(f, "Rebalance failed: {}", msg),
        }
    }
}

impl std::error::Error for BalancerError {}

// ─── Config ───

#[derive(Debug, Clone)]
pub struct BalancerConfig {
    pub skew_threshold: f64,
    pub rebalance_interval_ms: u64,
    pub health_check_interval_ms: u64,
    pub max_weight: f64,
}

impl Default for BalancerConfig {
    fn default() -> Self {
        Self {
            skew_threshold: 0.3,
            rebalance_interval_ms: 5000,
            health_check_interval_ms: 1000,
            max_weight: 10.0,
        }
    }
}

// ─── Node Weight ───

#[derive(Debug, Clone)]
pub struct NodeWeight {
    pub node_id: String,
    pub weight: f64,
    pub current_load: f64,
    pub max_capacity: f64,
    pub healthy: bool,
    pub requests_served: u64,
}

impl NodeWeight {
    pub fn new(node_id: String, max_capacity: f64) -> Self {
        Self {
            node_id,
            weight: 1.0,
            current_load: 0.0,
            max_capacity,
            healthy: true,
            requests_served: 0,
        }
    }

    pub fn utilization(&self) -> f64 {
        if self.max_capacity <= 0.0 {
            return 1.0;
        }
        (self.current_load / self.max_capacity).min(1.0)
    }

    pub fn effective_weight(&self) -> f64 {
        if !self.healthy {
            return 0.0;
        }
        self.weight * (1.0 - self.utilization())
    }
}

// ─── Balance State ───

#[derive(Debug, Clone)]
pub struct BalanceState {
    pub avg_load: f64,
    pub max_load: f64,
    pub min_load: f64,
    pub skew: f64,
    pub needs_rebalance: bool,
}

// ─── Stats ───

#[derive(Debug, Clone)]
pub struct BalancerStats {
    pub total_assignments: u64,
    pub total_rebalances: u64,
    pub avg_skew: f64,
    pub last_rebalance_ms: u64,
}

impl Default for BalancerStats {
    fn default() -> Self {
        Self {
            total_assignments: 0,
            total_rebalances: 0,
            avg_skew: 0.0,
            last_rebalance_ms: 0,
        }
    }
}

// ─── Balancer ───

pub struct LoadBalancer {
    config: BalancerConfig,
    nodes: HashMap<String, NodeWeight>,
    round_robin_idx: usize,
    stats: BalancerStats,
    assignment_history: VecDeque<String>,
}

impl LoadBalancer {
    pub fn new(config: BalancerConfig) -> Self {
        Self {
            config,
            nodes: HashMap::new(),
            round_robin_idx: 0,
            stats: BalancerStats::default(),
            assignment_history: VecDeque::new(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(BalancerConfig::default())
    }

    pub fn add_node(&mut self, node_id: String, capacity: f64) {
        self.nodes
            .insert(node_id.clone(), NodeWeight::new(node_id, capacity));
    }

    pub fn remove_node(&mut self, node_id: &str) -> Result<(), BalancerError> {
        self.nodes
            .remove(node_id)
            .map(|_| ())
            .ok_or(BalancerError::NodeNotFound(node_id.to_string()))
    }

    pub fn update_load(&mut self, node_id: &str, load: f64) -> Result<(), BalancerError> {
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| BalancerError::NodeNotFound(node_id.to_string()))?;
        node.current_load = load.max(0.0).min(node.max_capacity);
        Ok(())
    }

    pub fn set_health(&mut self, node_id: &str, healthy: bool) -> Result<(), BalancerError> {
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| BalancerError::NodeNotFound(node_id.to_string()))?;
        node.healthy = healthy;
        Ok(())
    }

    pub fn assign_request(&mut self) -> Result<String, BalancerError> {
        let healthy: Vec<&NodeWeight> = self.nodes.values().filter(|n| n.healthy).collect();
        if healthy.is_empty() {
            return Err(BalancerError::NoNodesAvailable);
        }

        // Weighted selection
        let total_weight: f64 = healthy.iter().map(|n| n.effective_weight()).sum();
        if total_weight <= 0.0 {
            return Err(BalancerError::NoNodesAvailable);
        }

        // Round-robin starting point
        let start = self.round_robin_idx % healthy.len();
        self.round_robin_idx += 1;

        // Select node with highest effective weight
        let best_idx = healthy
            .iter()
            .enumerate()
            .skip(start)
            .chain(healthy.iter().enumerate().take(start))
            .max_by(|(_, a), (_, b)| {
                a.effective_weight()
                    .partial_cmp(&b.effective_weight())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Record
        let best = &healthy[best_idx];
        self.stats.total_assignments += 1;
        self.assignment_history.push_back(best.node_id.clone());
        if self.assignment_history.len() > 100 {
            self.assignment_history.pop_front();
        }

        Ok(best.node_id.clone())
    }

    pub fn check_balance(&self) -> BalanceState {
        let loads: Vec<f64> = self
            .nodes
            .values()
            .filter(|n| n.healthy)
            .map(|n| n.utilization())
            .collect();

        if loads.is_empty() {
            return BalanceState {
                avg_load: 0.0,
                max_load: 0.0,
                min_load: 0.0,
                skew: 0.0,
                needs_rebalance: false,
            };
        }

        let avg = loads.iter().sum::<f64>() / loads.len() as f64;
        let max = loads.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min = loads.iter().cloned().fold(f64::INFINITY, f64::min);
        let skew = max - min;

        BalanceState {
            avg_load: avg,
            max_load: max,
            min_load: min,
            skew,
            needs_rebalance: skew > self.config.skew_threshold,
        }
    }

    pub fn rebalance(&mut self) -> Result<(), BalancerError> {
        let state = self.check_balance();
        if !state.needs_rebalance {
            return Ok(());
        }

        // Adjust weights inversely to utilization
        for node in self.nodes.values_mut() {
            if node.healthy {
                let util = node.utilization();
                node.weight = ((1.0 - util) * self.config.max_weight).max(0.1);
            }
        }

        self.stats.total_rebalances += 1;
        self.stats.avg_skew = (self.stats.avg_skew * (self.stats.total_rebalances - 1) as f64
            + state.skew)
            / self.stats.total_rebalances as f64;
        self.stats.last_rebalance_ms = current_timestamp_ms();

        Ok(())
    }

    pub fn get_stats(&self) -> &BalancerStats {
        &self.stats
    }

    pub fn get_config(&self) -> &BalancerConfig {
        &self.config
    }

    pub fn get_node(&self, node_id: &str) -> Option<&NodeWeight> {
        self.nodes.get(node_id)
    }

    pub fn reset_stats(&mut self) {
        self.stats = BalancerStats::default();
    }
}

impl Default for LoadBalancer {
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
    fn test_balancer_creation() {
        let b = LoadBalancer::with_defaults();
        assert_eq!(b.get_stats().total_assignments, 0);
    }

    #[test]
    fn test_add_node() {
        let mut b = LoadBalancer::with_defaults();
        b.add_node("n1".to_string(), 100.0);
        assert!(b.get_node("n1").is_some());
    }

    #[test]
    fn test_remove_node() {
        let mut b = LoadBalancer::with_defaults();
        b.add_node("n1".to_string(), 100.0);
        b.remove_node("n1").unwrap();
        assert!(b.get_node("n1").is_none());
    }

    #[test]
    fn test_assign_request() {
        let mut b = LoadBalancer::with_defaults();
        b.add_node("n1".to_string(), 100.0);
        let target = b.assign_request().unwrap();
        assert_eq!(target, "n1");
    }

    #[test]
    fn test_assign_no_healthy_nodes() {
        let mut b = LoadBalancer::with_defaults();
        b.add_node("n1".to_string(), 100.0);
        b.set_health("n1", false).unwrap();
        assert!(b.assign_request().is_err());
    }

    #[test]
    fn test_update_load() {
        let mut b = LoadBalancer::with_defaults();
        b.add_node("n1".to_string(), 100.0);
        b.update_load("n1", 50.0).unwrap();
        let node = b.get_node("n1").unwrap();
        assert!((node.utilization() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_balance_check() {
        let mut b = LoadBalancer::with_defaults();
        b.add_node("n1".to_string(), 100.0);
        b.add_node("n2".to_string(), 100.0);
        b.update_load("n1", 10.0).unwrap();
        b.update_load("n2", 90.0).unwrap();
        let state = b.check_balance();
        assert!(state.needs_rebalance);
    }

    #[test]
    fn test_rebalance() {
        let mut b = LoadBalancer::with_defaults();
        b.add_node("n1".to_string(), 100.0);
        b.add_node("n2".to_string(), 100.0);
        b.update_load("n1", 10.0).unwrap();
        b.update_load("n2", 90.0).unwrap();
        b.rebalance().unwrap();
        assert_eq!(b.get_stats().total_rebalances, 1);
    }

    #[test]
    fn test_stats_tracking() {
        let mut b = LoadBalancer::with_defaults();
        b.add_node("n1".to_string(), 100.0);
        for _ in 0..10 {
            b.assign_request().unwrap();
        }
        assert_eq!(b.get_stats().total_assignments, 10);
    }

    #[test]
    fn test_error_display() {
        let e = BalancerError::NoNodesAvailable;
        assert!(!e.to_string().is_empty());
    }
}
