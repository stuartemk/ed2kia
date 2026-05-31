//! Distributed Workload Scheduler — Sprint 69: Testnet Hardening & Distributed Alignment Workloads
//!
//! **Stuartian Law 1 (Diversidad):** Distribución equitativa de shards SAE entre nodos por tiers.
//! **Stuartian Law 5 (Múltiples Posibilidades):** Fallback automático cuando latencia >50ms.
//!
//! ### Feature Gates
//! | Feature | Módulo | Descripción |
//! |---|---|---|
//! | `v9.5-testnet-hardening` | workload_scheduler | Distribución dinámica de shards, latencia <50ms, fallback local |

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Threshold for high latency (milliseconds). Nodes exceeding this trigger fallback.
pub const LATENCY_THRESHOLD_MS: u64 = 50;

/// Node tier descriptor for workload distribution.
#[derive(Debug, Clone)]
pub struct NodeTier {
    pub id: String,
    pub capacity: u64,
    pub latency_ms: u64,
    pub score: f64,
}

/// Shard assignment result with optional fallback node.
#[derive(Debug, Clone)]
pub struct ShardAssignment {
    pub shard_id: u32,
    pub target: String,
    pub fallback: Option<String>,
}

/// Scheduler state tracking distribution history.
#[derive(Debug, Clone)]
pub struct SchedulerState {
    pub assignments: Vec<ShardAssignment>,
    pub last_distribution: Option<Instant>,
    pub total_shards_distributed: u64,
}

impl SchedulerState {
    pub fn new() -> Self {
        Self {
            assignments: Vec::new(),
            last_distribution: None,
            total_shards_distributed: 0,
        }
    }

    pub fn elapsed_since_last(&self) -> Option<Duration> {
        self.last_distribution.map(|t| t.elapsed())
    }

    pub fn reset(&mut self) {
        self.assignments.clear();
        self.last_distribution = None;
        self.total_shards_distributed = 0;
    }
}

impl Default for SchedulerState {
    fn default() -> Self {
        Self::new()
    }
}

/// Distribute shards across nodes using weighted round-robin by score/capacity.
///
/// Nodes with latency > [`LATENCY_THRESHOLD_MS`] will receive a fallback assignment
/// pointing to the next available node in rotation.
pub fn distribute_shards(nodes: &[NodeTier], shard_count: u32) -> Vec<ShardAssignment> {
    if nodes.is_empty() || shard_count == 0 {
        return Vec::new();
    }

    let mut assignments = Vec::with_capacity(shard_count as usize);
    let total_score: f64 = nodes.iter().map(|n| n.score * n.capacity as f64).sum();

    // Build weighted index: each node gets slots proportional to score * capacity
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

    // Ensure we have at least shard_count entries
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

/// Build a node lookup map from assignments for O(1) shard→node queries.
pub fn build_assignment_map(assignments: &[ShardAssignment]) -> HashMap<u32, &str> {
    assignments
        .iter()
        .map(|a| (a.shard_id, a.target.as_str()))
        .collect()
}

/// Calculate the effective load balance ratio across nodes.
/// A ratio of 1.0 means perfectly balanced; lower values indicate imbalance.
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

impl std::fmt::Display for NodeTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NodeTier(id={}, capacity={}, latency_ms={}, score={})",
            self.id, self.capacity, self.latency_ms, self.score
        )
    }
}

impl std::fmt::Display for ShardAssignment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fallback_str = self
            .fallback
            .as_deref()
            .map(|fb| format!(", fallback={}", fb))
            .unwrap_or_default();
        write!(
            f,
            "ShardAssignment(shard={}, target={}{})",
            self.shard_id, self.target, fallback_str
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_nodes() -> Vec<NodeTier> {
        vec![
            NodeTier {
                id: "n1".into(),
                capacity: 100,
                latency_ms: 30,
                score: 0.9,
            },
            NodeTier {
                id: "n2".into(),
                capacity: 80,
                latency_ms: 60,
                score: 0.7,
            },
            NodeTier {
                id: "n3".into(),
                capacity: 120,
                latency_ms: 20,
                score: 0.95,
            },
        ]
    }

    #[test]
    fn test_distribution_basic() {
        let nodes = test_nodes();
        let assigns = distribute_shards(&nodes, 6);
        assert_eq!(assigns.len(), 6);
        for (i, a) in assigns.iter().enumerate() {
            assert_eq!(a.shard_id, i as u32);
        }
    }

    #[test]
    fn test_distribution_fallback() {
        let nodes = vec![
            NodeTier {
                id: "n1".into(),
                capacity: 100,
                latency_ms: 30,
                score: 0.9,
            },
            NodeTier {
                id: "n2".into(),
                capacity: 80,
                latency_ms: 60,
                score: 0.7,
            },
        ];
        let assigns = distribute_shards(&nodes, 4);
        // n2 has latency > 50ms, so assignments targeting n2 should have fallback
        let n2_assigns: Vec<&ShardAssignment> =
            assigns.iter().filter(|a| a.target == "n2").collect();
        for a in n2_assigns {
            assert!(a.fallback.is_some(), "High latency node must have fallback");
        }
    }

    #[test]
    fn test_empty_nodes() {
        let assigns = distribute_shards(&[], 10);
        assert!(assigns.is_empty());
    }

    #[test]
    fn test_zero_shards() {
        let nodes = test_nodes();
        let assigns = distribute_shards(&nodes, 0);
        assert!(assigns.is_empty());
    }

    #[test]
    fn test_single_node() {
        let nodes = vec![NodeTier {
            id: "solo".into(),
            capacity: 50,
            latency_ms: 10,
            score: 1.0,
        }];
        let assigns = distribute_shards(&nodes, 5);
        assert_eq!(assigns.len(), 5);
        for a in &assigns {
            assert_eq!(a.target, "solo");
            assert!(a.fallback.is_none());
        }
    }

    #[test]
    fn test_load_balance_ratio_perfect() {
        let assigns = vec![
            ShardAssignment {
                shard_id: 0,
                target: "a".into(),
                fallback: None,
            },
            ShardAssignment {
                shard_id: 1,
                target: "b".into(),
                fallback: None,
            },
        ];
        let ratio = load_balance_ratio(&assigns);
        assert!((ratio - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_load_balance_ratio_imbalanced() {
        let assigns = vec![
            ShardAssignment {
                shard_id: 0,
                target: "a".into(),
                fallback: None,
            },
            ShardAssignment {
                shard_id: 1,
                target: "a".into(),
                fallback: None,
            },
            ShardAssignment {
                shard_id: 2,
                target: "a".into(),
                fallback: None,
            },
            ShardAssignment {
                shard_id: 3,
                target: "b".into(),
                fallback: None,
            },
        ];
        let ratio = load_balance_ratio(&assigns);
        // a=3, b=1 → min=1, max=3 → ratio = 1/3 ≈ 0.333
        assert!((ratio - 1.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_load_balance_ratio_empty() {
        let ratio = load_balance_ratio(&[]);
        assert!((ratio - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_build_assignment_map() {
        let assigns = vec![
            ShardAssignment {
                shard_id: 0,
                target: "n1".into(),
                fallback: None,
            },
            ShardAssignment {
                shard_id: 1,
                target: "n2".into(),
                fallback: Some("n1".into()),
            },
        ];
        let map = build_assignment_map(&assigns);
        assert_eq!(map.get(&0), Some(&"n1"));
        assert_eq!(map.get(&1), Some(&"n2"));
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_scheduler_state_new() {
        let state = SchedulerState::new();
        assert!(state.assignments.is_empty());
        assert!(state.last_distribution.is_none());
        assert_eq!(state.total_shards_distributed, 0);
    }

    #[test]
    fn test_scheduler_state_reset() {
        let mut state = SchedulerState::new();
        state.assignments.push(ShardAssignment {
            shard_id: 0,
            target: "n1".into(),
            fallback: None,
        });
        state.last_distribution = Some(Instant::now());
        state.total_shards_distributed = 5;
        state.reset();
        assert!(state.assignments.is_empty());
        assert!(state.last_distribution.is_none());
        assert_eq!(state.total_shards_distributed, 0);
    }

    #[test]
    fn test_scheduler_state_elapsed() {
        let state = SchedulerState::new();
        assert!(state.elapsed_since_last().is_none());
    }

    #[test]
    fn test_scheduler_state_default() {
        let state = SchedulerState::default();
        assert!(state.assignments.is_empty());
    }

    #[test]
    fn test_node_tier_display() {
        let node = NodeTier {
            id: "test".into(),
            capacity: 100,
            latency_ms: 25,
            score: 0.8,
        };
        let s = format!("{}", node);
        assert!(s.contains("test"));
        assert!(s.contains("100"));
    }

    #[test]
    fn test_shard_assignment_display() {
        let a = ShardAssignment {
            shard_id: 42,
            target: "n1".into(),
            fallback: Some("n2".into()),
        };
        let s = format!("{}", a);
        assert!(s.contains("42"));
        assert!(s.contains("n1"));
        assert!(s.contains("n2"));
    }

    #[test]
    fn test_shard_assignment_display_no_fallback() {
        let a = ShardAssignment {
            shard_id: 1,
            target: "n1".into(),
            fallback: None,
        };
        let s = format!("{}", a);
        assert!(!s.contains("fallback"));
    }

    #[test]
    fn test_distribution_all_high_latency() {
        let nodes = vec![
            NodeTier {
                id: "slow1".into(),
                capacity: 50,
                latency_ms: 100,
                score: 0.5,
            },
            NodeTier {
                id: "slow2".into(),
                capacity: 50,
                latency_ms: 100,
                score: 0.5,
            },
        ];
        let assigns = distribute_shards(&nodes, 4);
        for a in &assigns {
            assert!(
                a.fallback.is_some(),
                "All high-latency nodes must have fallbacks"
            );
        }
    }

    #[test]
    fn test_distribution_weighted_towards_higher_score() {
        let nodes = vec![
            NodeTier {
                id: "high".into(),
                capacity: 100,
                latency_ms: 10,
                score: 1.0,
            },
            NodeTier {
                id: "low".into(),
                capacity: 100,
                latency_ms: 10,
                score: 0.1,
            },
        ];
        let assigns = distribute_shards(&nodes, 20);
        let high_count = assigns.iter().filter(|a| a.target == "high").count();
        let low_count = assigns.iter().filter(|a| a.target == "low").count();
        assert!(
            high_count > low_count,
            "Higher score node should receive more shards"
        );
    }

    #[test]
    fn test_latency_threshold_constant() {
        assert_eq!(LATENCY_THRESHOLD_MS, 50);
    }
}
