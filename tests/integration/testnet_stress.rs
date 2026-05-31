//! Testnet Stress Tests — Sprint 69: Testnet Hardening & Distributed Alignment Workloads
//!
//! Validates fault tolerance, shard redistribution, and workload scheduler behavior
//! under simulated node failures and high-latency conditions.

#[path = "../../src/network/workload_scheduler.rs"]
mod scheduler;

use scheduler::{
    build_assignment_map, distribute_shards, load_balance_ratio, NodeTier, SchedulerState,
    ShardAssignment, LATENCY_THRESHOLD_MS,
};

// ---------------------------------------------------------------------------
// Helper: Create test nodes mimicking testnet topology
// ---------------------------------------------------------------------------

fn testnet_nodes() -> Vec<NodeTier> {
    vec![
        NodeTier {
            id: "testnet-node1".into(),
            capacity: 150,
            latency_ms: 5,
            score: 0.95,
        },
        NodeTier {
            id: "testnet-node2".into(),
            capacity: 200,
            latency_ms: 8,
            score: 0.90,
        },
        NodeTier {
            id: "testnet-node3".into(),
            capacity: 100,
            latency_ms: 80, // High latency → triggers fallback
            score: 0.70,
        },
        NodeTier {
            id: "testnet-node4".into(),
            capacity: 150,
            latency_ms: 12,
            score: 0.85,
        },
        NodeTier {
            id: "testnet-node5".into(),
            capacity: 50,
            latency_ms: 120, // Very high latency → triggers fallback
            score: 0.50,
        },
    ]
}

// ---------------------------------------------------------------------------
// Test: Full testnet shard distribution
// ---------------------------------------------------------------------------

#[test]
fn test_testnet_shard_distribution() {
    let nodes = testnet_nodes();
    let assignments = distribute_shards(&nodes, 64);

    assert_eq!(assignments.len(), 64);
    for (i, a) in assignments.iter().enumerate() {
        assert_eq!(a.shard_id, i as u32);
        assert!(nodes.iter().any(|n| n.id == a.target));
    }
}

// ---------------------------------------------------------------------------
// Test: High-latency nodes receive fallback assignments
// ---------------------------------------------------------------------------

#[test]
fn test_high_latency_fallback() {
    let nodes = testnet_nodes();
    let assignments = distribute_shards(&nodes, 64);

    let high_latency_assignments: Vec<&ShardAssignment> = assignments
        .iter()
        .filter(|a| {
            let node = nodes.iter().find(|n| n.id == a.target).unwrap();
            node.latency_ms > LATENCY_THRESHOLD_MS
        })
        .collect();

    for a in high_latency_assignments {
        assert!(
            a.fallback.is_some(),
            "Node {} has latency {}ms > {}ms but no fallback assigned",
            a.target,
            nodes.iter().find(|n| n.id == a.target).unwrap().latency_ms,
            LATENCY_THRESHOLD_MS
        );
        // Fallback should point to a different node
        assert!(
            a.fallback.as_ref().unwrap() != &a.target,
            "Fallback cannot be the same as target"
        );
    }
}

// ---------------------------------------------------------------------------
// Test: Load balance ratio under testnet conditions
// ---------------------------------------------------------------------------

#[test]
fn test_testnet_load_balance() {
    let nodes = testnet_nodes();
    let assignments = distribute_shards(&nodes, 64);
    let ratio = load_balance_ratio(&assignments);

    assert!(ratio > 0.0, "Load balance ratio should be positive");
    assert!(
        ratio <= 1.0,
        "Load balance ratio should not exceed 1.0 (perfect balance)"
    );
}

// ---------------------------------------------------------------------------
// Test: Assignment map correctness
// ---------------------------------------------------------------------------

#[test]
fn test_assignment_map_coverage() {
    let nodes = testnet_nodes();
    let assignments = distribute_shards(&nodes, 64);
    let map = build_assignment_map(&assignments);

    assert_eq!(map.len(), 64);
    for (shard_id, node_id) in &map {
        assert!(
            nodes.iter().any(|n| n.id == *node_id),
            "Shard {} maps to unknown node {}",
            shard_id,
            node_id
        );
    }
}

// ---------------------------------------------------------------------------
// Test: Node failure simulation — redistribute after removing a node
// ---------------------------------------------------------------------------

#[test]
fn test_node_failure_redistribution() {
    let mut nodes = testnet_nodes();
    let original = distribute_shards(&nodes, 64);
    assert_eq!(original.len(), 64);

    // Simulate node3 failure (high latency node)
    nodes.retain(|n| n.id != "testnet-node3");
    let redistributed = distribute_shards(&nodes, 64);

    assert_eq!(redistributed.len(), 64);
    for a in &redistributed {
        assert!(
            nodes.iter().any(|n| n.id == a.target),
            "Shard {} assigned to failed node {}",
            a.shard_id,
            a.target
        );
    }
}

// ---------------------------------------------------------------------------
// Test: Multiple node failures — cascade redistribution
// ---------------------------------------------------------------------------

#[test]
fn test_cascade_node_failures() {
    let mut nodes = testnet_nodes();

    // Remove 3 nodes — only 2 remain
    nodes.retain(|n| n.id == "testnet-node1" || n.id == "testnet-node2");
    let assignments = distribute_shards(&nodes, 64);

    assert_eq!(assignments.len(), 64);
    let ratio = load_balance_ratio(&assignments);
    assert!(ratio > 0.0);
    assert!(ratio <= 1.0);
}

// ---------------------------------------------------------------------------
// Test: Single node survival — all shards on remaining node
// ---------------------------------------------------------------------------

#[test]
fn test_single_node_survival() {
    let nodes = testnet_nodes();
    let survivor = &nodes[0];
    let assignments = distribute_shards(std::slice::from_ref(survivor), 64);

    assert_eq!(assignments.len(), 64);
    for a in &assignments {
        assert_eq!(a.target, survivor.id);
        assert!(a.fallback.is_none(), "Single node should have no fallback");
    }
    let ratio = load_balance_ratio(&assignments);
    assert!(
        (ratio - 1.0).abs() < 1e-9,
        "Single node should have perfect balance ratio (1.0)"
    );
}

// ---------------------------------------------------------------------------
// Test: Scheduler state tracks distribution
// ---------------------------------------------------------------------------

#[test]
fn test_scheduler_state_tracks_distribution() {
    let mut state = SchedulerState::new();
    assert!(state.assignments.is_empty());
    assert_eq!(state.total_shards_distributed, 0);

    let nodes = testnet_nodes();
    let assignments = distribute_shards(&nodes, 64);
    state.assignments = assignments;
    state.total_shards_distributed = 64;
    state.last_distribution = Some(std::time::Instant::now());

    assert_eq!(state.assignments.len(), 64);
    assert_eq!(state.total_shards_distributed, 64);
    assert!(state.last_distribution.is_some());
}

// ---------------------------------------------------------------------------
// Test: Scheduler reset clears state
// ---------------------------------------------------------------------------

#[test]
fn test_scheduler_reset() {
    let mut state = SchedulerState::new();
    state.assignments = vec![ShardAssignment {
        shard_id: 0,
        target: "test".into(),
        fallback: None,
    }];
    state.total_shards_distributed = 1;

    state.reset();
    assert!(state.assignments.is_empty());
    assert_eq!(state.total_shards_distributed, 0);
    assert!(state.last_distribution.is_none());
}

// ---------------------------------------------------------------------------
// Test: Latency threshold constant is sensible
// ---------------------------------------------------------------------------

#[test]
fn test_latency_threshold_value() {
    assert!(
        LATENCY_THRESHOLD_MS > 0,
        "Latency threshold must be positive"
    );
    assert!(
        LATENCY_THRESHOLD_MS < 1000,
        "Latency threshold should be reasonable (< 1s)"
    );
}

// ---------------------------------------------------------------------------
// Test: Empty node list produces no assignments
// ---------------------------------------------------------------------------

#[test]
fn test_empty_nodes_no_assignment() {
    let assignments = distribute_shards(&[], 64);
    assert!(assignments.is_empty());
}

// ---------------------------------------------------------------------------
// Test: Zero shards produces no assignments
// ---------------------------------------------------------------------------

#[test]
fn test_zero_shards_no_assignment() {
    let nodes = testnet_nodes();
    let assignments = distribute_shards(&nodes, 0);
    assert!(assignments.is_empty());
}

// ---------------------------------------------------------------------------
// Test: Large shard count scales correctly
// ---------------------------------------------------------------------------

#[test]
fn test_large_shard_count() {
    let nodes = testnet_nodes();
    let assignments = distribute_shards(&nodes, 1024);

    assert_eq!(assignments.len(), 1024);
    let ratio = load_balance_ratio(&assignments);
    assert!(ratio > 0.0);
    assert!(ratio <= 1.0);
}

// ---------------------------------------------------------------------------
// Test: Fallback nodes are valid (exist in node list)
// ---------------------------------------------------------------------------

#[test]
fn test_fallback_nodes_are_valid() {
    let nodes = testnet_nodes();
    let assignments = distribute_shards(&nodes, 64);

    for a in &assignments {
        if let Some(ref fallback_id) = a.fallback {
            assert!(
                nodes.iter().any(|n| n.id == *fallback_id),
                "Fallback node {} does not exist in node list",
                fallback_id
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Distribution is deterministic for same inputs
// ---------------------------------------------------------------------------

#[test]
fn test_deterministic_distribution() {
    let nodes = testnet_nodes();
    let a1 = distribute_shards(&nodes, 64);
    let a2 = distribute_shards(&nodes, 64);

    assert_eq!(a1.len(), a2.len());
    for (x, y) in a1.iter().zip(a2.iter()) {
        assert_eq!(x.shard_id, y.shard_id);
        assert_eq!(x.target, y.target);
        assert_eq!(x.fallback, y.fallback);
    }
}
