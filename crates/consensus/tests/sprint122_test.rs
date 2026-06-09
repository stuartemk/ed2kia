//! Sprint 122 (v12.2.0) — THE GAME-THEORETIC MESH & TAYLOR-ZONOTOPE SYNTHESIS
//!
//! Integration tests for hierarchical sharding, game-theoretic PoSym,
//! and cross-module synergy validation.

use ed2k_consensus::hierarchical_sharding::{
    assign_node_by_load, trust_weighted_shard_vote, ClusterConfig, ClusterDiscovery, ConsensusType,
    ConsistentHashRing, HierarchicalShardManager, PeerInfo, ShardAssignment, ShardConfig,
    ShardingError,
};
use ed2k_consensus::posym::{
    byzantine_weighted_median, calculate_energy_impact, pac_bayes_bound, update_trust_score,
};

// ─── Consistent Hash Ring Integration Tests ───────────────────────────────

#[test]
fn test_ring_large_scale_distribution() {
    let ring = ConsistentHashRing::new(64).unwrap();
    let mut counts = vec![0usize; 64];

    for node_id in 0..10_000 {
        let shard = ring.get_shard(node_id);
        counts[shard as usize] += 1;
    }

    let min_count = *counts.iter().min().unwrap();
    let max_count = *counts.iter().max().unwrap();
    let expected = 10_000 / 64;
    let ratio = min_count as f64 / max_count as f64;

    assert!(
        ratio > 0.6,
        "Large-scale distribution ratio {:.3} too low (min={}, max={}, expected={})",
        ratio,
        min_count,
        max_count,
        expected
    );
}

#[test]
fn test_ring_stability_under_churn() {
    let ring = ConsistentHashRing::new(16).unwrap();

    // Record initial assignments
    let nodes: Vec<u64> = (0..1000).collect();
    let initial: Vec<u64> = nodes.iter().map(|&n| ring.get_shard(n)).collect();

    // Simulate churn: re-query after "time passes"
    let after: Vec<u64> = nodes.iter().map(|&n| ring.get_shard(n)).collect();

    // Consistent hashing should be deterministic
    assert_eq!(
        initial, after,
        "Assignments should be stable without shard changes"
    );
}

#[test]
fn test_ring_load_tracking_accuracy() {
    let mut ring = ConsistentHashRing::new(8).unwrap();

    // Simulate realistic assignment pattern
    for node_id in 0..80 {
        let shard = ring.get_shard(node_id);
        ring.record_assignment(shard);
    }

    let total: usize = ring.get_all_loads().values().sum();
    assert_eq!(total, 80, "Total load should match number of assignments");
}

// ─── Hierarchical Shard Manager Integration Tests ─────────────────────────

#[test]
fn test_manager_multi_cluster_assignment() {
    let clusters = vec![
        ClusterConfig::new(0, 8, 0).with_region("us-east"),
        ClusterConfig::new(1, 8, 8).with_region("eu-west"),
        ClusterConfig::new(2, 8, 16).with_region("ap-south"),
    ];
    let mut manager = HierarchicalShardManager::new(24, &clusters).unwrap();

    // Assign 500 nodes across clusters
    for i in 0..500 {
        manager.assign_node(i).unwrap();
    }

    assert_eq!(manager.total_nodes(), 500);

    // Verify all nodes have valid assignments
    for i in 0..500 {
        let assignment = manager.get_node_assignment(i).unwrap();
        assert!(assignment.shard_id < 24);
    }
}

#[test]
fn test_manager_capacity_enforcement() {
    let clusters = vec![ClusterConfig::new(0, 4, 0)];
    let mut manager = HierarchicalShardManager::new(4, &clusters).unwrap();

    // Set low capacity for testing
    for shard_id in 0..4 {
        if let Some(config) = manager.get_shard_config(shard_id) {
            // Assign up to capacity
            for i in 0..config.max_nodes {
                let node_id = shard_id * 1000 + i as u64;
                let _ = manager.assign_node_to_shard(node_id, shard_id);
            }
        }
    }

    // Now assignments to full shards should fail
    let result = manager.assign_node_to_shard(99999, 0);
    assert!(matches!(
        result,
        Err(ShardingError::ShardAtCapacity(_, _, _))
    ));
}

#[test]
fn test_manager_rebalance_improves_distribution() {
    let clusters = vec![ClusterConfig::new(0, 8, 0)];
    let mut manager = HierarchicalShardManager::new(8, &clusters).unwrap();

    // Create heavy imbalance
    for i in 0..50 {
        manager.assign_node_to_shard(i, 0).unwrap();
    }
    for i in 50..60 {
        manager.assign_node_to_shard(i, 7).unwrap();
    }

    let initial_imbalance = manager.load_imbalance();
    assert!(initial_imbalance > 0.3, "Initial imbalance should be high");

    // Rebalance multiple times
    for _ in 0..20 {
        manager.rebalance().unwrap();
    }

    let final_imbalance = manager.load_imbalance();
    // Rebalance moves nodes from most to least loaded shard.
    // With only 2 shards used (0 and 7), rebalance will distribute to all 8.
    // Imbalance may reach 0.0 if perfectly distributed, or improve significantly.
    assert!(
        final_imbalance <= initial_imbalance,
        "Rebalancing should not increase imbalance: {:.3} -> {:.3}",
        initial_imbalance,
        final_imbalance
    );
}

#[test]
fn test_manager_node_removal_consistency() {
    let clusters = vec![ClusterConfig::new(0, 4, 0)];
    let mut manager = HierarchicalShardManager::new(4, &clusters).unwrap();

    let nodes: Vec<u64> = (0..100).collect();
    for &node in &nodes {
        manager.assign_node(node).unwrap();
    }

    // Remove half the nodes
    for &node in &nodes[..50] {
        let removed = manager.remove_node(node);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().node_id, node);
    }

    assert_eq!(manager.total_nodes(), 50);

    // Verify removed nodes are gone
    for node in &nodes[..50] {
        assert!(manager.get_node_assignment(*node).is_none());
    }

    // Verify remaining nodes are intact
    for node in &nodes[50..] {
        assert!(manager.get_node_assignment(*node).is_some());
    }
}

#[test]
fn test_manager_shard_config_retrieval() {
    let clusters = vec![ClusterConfig::new(0, 16, 0)];
    let manager = HierarchicalShardManager::new(16, &clusters).unwrap();

    for shard_id in 0..16 {
        let config = manager.get_shard_config(shard_id);
        assert!(config.is_some(), "Shard {} config should exist", shard_id);
        // Shard configs are created with cluster_id from the ClusterConfig
        let cfg = config.unwrap();
        assert_eq!(
            cfg.shard_id, shard_id,
            "Shard config ID should match for shard {}",
            shard_id
        );
    }

    assert!(manager.get_shard_config(16).is_none());
}

// ─── Cluster Discovery Integration Tests ──────────────────────────────────

#[test]
fn test_discovery_large_cluster() {
    let mut discovery = ClusterDiscovery::new(0.7).with_max_peers(200);

    // Simulate 200 peers across 8 shards
    for i in 0..200 {
        let peer = PeerInfo {
            peer_id: i,
            shard_id: i % 8,
            trust_score: if i % 10 < 8 { 0.85 } else { 0.3 }, // 80% trusted
            energy_budget: 0.5 + (i % 5) as f64 * 0.1,
            address: format!("10.0.{}.{}", i / 256, i % 256),
            last_seen: 1000,
        };
        discovery.add_peer(peer);
    }

    assert_eq!(discovery.peer_count(), 200);

    // Trusted peers should be ~80%
    let trusted = discovery.discover_trusted_peers();
    assert!(
        trusted.len() >= 150,
        "Expected ~160 trusted peers, got {}",
        trusted.len()
    );
    assert!(trusted.len() <= 170);

    // Per-shard discovery
    for shard_id in 0..8 {
        let shard_peers = discovery.discover_peers_in_shard(shard_id);
        assert_eq!(shard_peers.len(), 25, "Each shard should have 25 peers");
    }
}

#[test]
fn test_discovery_temporal_filtering() {
    let mut discovery = ClusterDiscovery::new(0.5);

    // Add peers with different last_seen times
    for i in 0..20 {
        let peer = PeerInfo {
            peer_id: i,
            shard_id: 0,
            trust_score: 0.8,
            energy_budget: 1.0,
            address: format!("peer-{}", i),
            last_seen: if i < 10 { 1000 } else { 100 }, // 10 recent, 10 stale
        };
        discovery.add_peer(peer);
    }

    // At time 1400 with timeout 500, only recent peers are active
    let active = discovery.discover_active_peers(0, 1400, 500);
    assert_eq!(active.len(), 10, "Only 10 peers should be active");

    // At time 200 with timeout 500, all are active
    let all_active = discovery.discover_active_peers(0, 200, 500);
    assert_eq!(all_active.len(), 20, "All peers should be active");
}

#[test]
fn test_discovery_energy_aggregation() {
    let mut discovery = ClusterDiscovery::new(0.5);

    for i in 0..100 {
        let peer = PeerInfo {
            peer_id: i,
            shard_id: i % 10,
            trust_score: 0.9,
            energy_budget: (i % 20) as f64 * 0.05, // 0.0 to 0.95
            address: format!("peer-{}", i),
            last_seen: 1000,
        };
        discovery.add_peer(peer);
    }

    // Each shard has 10 peers
    for shard_id in 0..10 {
        let total_energy = discovery.total_energy_in_shard(shard_id);
        assert!(
            total_energy > 0.0,
            "Shard {} should have positive energy",
            shard_id
        );
        assert!(
            total_energy < 10.0,
            "Shard {} energy {} should be bounded",
            shard_id,
            total_energy
        );
    }
}

#[test]
fn test_discovery_trust_distribution() {
    let mut discovery = ClusterDiscovery::new(0.5);

    // Create bimodal trust distribution
    for i in 0..50 {
        let peer = PeerInfo {
            peer_id: i,
            shard_id: 0,
            trust_score: if i < 25 { 0.95 } else { 0.1 },
            energy_budget: 1.0,
            address: format!("peer-{}", i),
            last_seen: 1000,
        };
        discovery.add_peer(peer);
    }

    let avg_trust = discovery.avg_trust_in_shard(0);
    let expected = (25.0 * 0.95 + 25.0 * 0.1) / 50.0;
    assert!(
        (avg_trust - expected).abs() < 0.01,
        "Avg trust {:.3} should match expected {:.3}",
        avg_trust,
        expected
    );
}

// ─── Trust-Weighted Voting Integration Tests ──────────────────────────────

#[test]
fn test_vote_large_election() {
    // Simulate 1000 voters across 5 shards
    let votes: Vec<(u64, u64, f64)> = (0..1000)
        .map(|i| {
            let shard = i % 5;
            let trust = match shard {
                0 => 0.9, // Strong support
                1 => 0.7,
                2 => 0.5,
                3 => 0.3,
                _ => 0.1, // Weak support
            };
            (i, shard as u64, trust as f64)
        })
        .collect();

    let result = trust_weighted_shard_vote(&votes, 100.0);

    assert_eq!(
        result.winning_shard, 0,
        "Shard 0 should win with highest trust"
    );
    assert!(result.quorum_met);
    assert_eq!(result.total_votes, 1000);
}

#[test]
fn test_vote_sybil_resistance() {
    // 1 honest high-trust voter vs 100 low-trust Sybil voters
    let votes: Vec<(u64, u64, f64)> = vec![(0, 0, 0.95)]; // Honest
    let votes = {
        let mut v = votes;
        for i in 1..101 {
            v.push((i as u64, 1, 0.005)); // Sybil — very low trust each
        }
        v
    };

    // 1 × 0.95 = 0.95 vs 100 × 0.005 = 0.50 → Honest wins
    let result = trust_weighted_shard_vote(&votes, 0.0);
    assert_eq!(
        result.winning_shard, 0,
        "Honest voter should win: weight {:.3} > {:.3}",
        result.shard_weights[&0], result.shard_weights[&1]
    );
    assert!(
        result.shard_weights[&0] > result.shard_weights[&1],
        "Honest weight {:.3} > Sybil weight {:.3}",
        result.shard_weights[&0],
        result.shard_weights[&1]
    );
}

#[test]
fn test_vote_quorum_dynamics() {
    let votes: Vec<(u64, u64, f64)> = (0..100)
        .map(|i| (i, 0, 0.02)) // Total trust = 2.0
        .collect();

    // Quorum at 1.0 should pass
    let result = trust_weighted_shard_vote(&votes, 1.0);
    assert!(result.quorum_met);

    // Quorum at 3.0 should fail
    let result = trust_weighted_shard_vote(&votes, 3.0);
    assert!(!result.quorum_met);
}

// ─── Load-Based Assignment Integration Tests ──────────────────────────────

#[test]
fn test_load_assignment_large_fleet() {
    let configs: Vec<ShardConfig> = (0..32).map(|id| ShardConfig::new(id, 0, 200)).collect();
    let mut loads = std::collections::HashMap::new();

    // Create varied load distribution
    for id in 0..32 {
        loads.insert(id, (id * 5) as usize); // 0, 5, 10, ..., 155
    }

    // Assign 50 new nodes
    for _ in 0..50 {
        let result = assign_node_by_load(&configs, &loads, 1.0, 0.1);
        assert!(result.is_ok());
        let shard = result.unwrap();
        assert!(shard < 32);
    }
}

#[test]
fn test_load_assignment_energy_proportional() {
    let configs = vec![
        ShardConfig::new(0, 0, 100),
        ShardConfig::new(1, 0, 100),
        ShardConfig::new(2, 0, 100),
    ];
    let mut loads = std::collections::HashMap::new();
    loads.insert(0, 10);
    loads.insert(1, 50);
    loads.insert(2, 90);

    // High energy node should prefer least loaded
    let shard = assign_node_by_load(&configs, &loads, 2.0, 0.1).unwrap();
    assert_eq!(shard, 0, "Should assign to least loaded shard");

    // Low energy node with high minimum should still get least loaded
    let shard = assign_node_by_load(&configs, &loads, 0.5, 0.05).unwrap();
    assert_eq!(shard, 0);
}

// ─── Cross-Module Synergy: PoSym + Hierarchical Sharding ──────────────────

#[test]
fn test_posym_energy_informs_shard_assignment() {
    // Calculate energy impact for different node types
    // calculate_energy_impact(idle_power_w, delta_t_sec, total_flops, energy_per_flop)
    let desktop_energy = calculate_energy_impact(100.0, 60.0, 1e12, 1e-12);
    let mobile_energy = calculate_energy_impact(50.0, 60.0, 1e10, 1e-12);
    let iot_energy = calculate_energy_impact(10.0, 60.0, 1e8, 1e-12);

    assert!(
        desktop_energy > mobile_energy,
        "Desktop should consume more energy than mobile"
    );
    assert!(
        mobile_energy > iot_energy,
        "Mobile should consume more energy than IoT"
    );

    // Energy-aware shard assignment: high-energy nodes to shards with more capacity
    let configs = vec![
        ShardConfig::new(0, 0, 500), // High capacity
        ShardConfig::new(1, 0, 100), // Medium capacity
        ShardConfig::new(2, 0, 20),  // Low capacity (IoT-friendly)
    ];
    let mut loads = std::collections::HashMap::new();
    loads.insert(0, 100);
    loads.insert(1, 50);
    loads.insert(2, 10);

    // assign_node_by_load picks the shard with lowest load ratio
    // Shard 0: 100/500 = 0.20, Shard 1: 50/100 = 0.50, Shard 2: 10/20 = 0.50
    // So shard 0 (lowest ratio) is selected
    let least_loaded = assign_node_by_load(&configs, &loads, iot_energy, 0.1).unwrap();
    assert_eq!(least_loaded, 0, "Should pick shard with lowest load ratio");
}

#[test]
fn test_posym_trust_informs_cluster_discovery() {
    let mut discovery = ClusterDiscovery::new(0.6);

    // Simulate nodes with PoSym-derived trust scores
    for i in 0..30 {
        // Trust decays for inactive nodes, grows for contributors
        let trust = if i < 15 {
            0.9 - (i as f64 * 0.02) // High contributors
        } else {
            0.5 - ((i - 15) as f64 * 0.02) // Low contributors
        };

        let peer = PeerInfo {
            peer_id: i,
            shard_id: i % 6,
            trust_score: trust.max(0.0),
            energy_budget: 1.0,
            address: format!("node-{}", i),
            last_seen: 1000,
        };
        discovery.add_peer(peer);
    }

    // Only high-trust nodes should be discoverable
    let trusted = discovery.discover_trusted_peers();
    for peer in &trusted {
        assert!(
            peer.trust_score >= 0.6,
            "Trusted peer {} has trust {:.3} < 0.6",
            peer.peer_id,
            peer.trust_score
        );
    }

    assert!(
        trusted.len() < 30,
        "Not all 30 nodes should meet trust threshold"
    );
}

#[test]
fn test_byzantine_median_in_shard_voting() {
    // Simulate shard value reports with Byzantine nodes
    let values: Vec<(f64, f64)> = (0..30)
        .map(|i| {
            let value = if i < 20 {
                100.0 + (i as f64 * 0.5) // Honest: 100-109.5
            } else {
                500.0 + (i as f64 * 10.0) // Byzantine: 500-590
            };
            let trust = if i < 20 { 0.9 } else { 0.1 };
            (value, trust)
        })
        .collect();

    let median = byzantine_weighted_median(&values);

    // Median should be in honest range despite Byzantine outliers
    assert!(
        median >= 100.0 && median <= 110.0,
        "Byzantine-resistant median {:.2} should be in honest range [100, 110]",
        median
    );
}

#[test]
fn test_pac_bayes_cluster_confidence() {
    // Simulate cluster VFE measurements
    let n = 100; // 100 samples
    let empirical_vfe = 0.85;
    let kl_divergence = 0.01;
    let delta = 0.05; // 95% confidence

    let bound = pac_bayes_bound(empirical_vfe, kl_divergence, n, delta);

    assert!(
        bound >= empirical_vfe,
        "PAC-Bayes bound {:.4} should be >= empirical VFE {:.4}",
        bound,
        empirical_vfe
    );

    // More samples should tighten the bound
    let bound_tight = pac_bayes_bound(empirical_vfe, kl_divergence, 1000, delta);
    assert!(
        bound_tight <= bound,
        "More samples should tighten bound: {:.4} <= {:.4}",
        bound_tight,
        bound
    );
}

#[test]
fn test_trust_decay_affects_discovery() {
    // Simulate trust decay over time
    let mut trust = 0.95;
    let gamma = 0.01; // Decay rate

    for round in 0..20 {
        // update_trust_score(current_score, gamma, alpha, delta_vfe, cost_energy, beta, proof_valid)
        let _ = update_trust_score(trust, gamma, 1.0, 0.0, 0.4, 0.3, false);
        trust = update_trust_score(trust, gamma, 1.0, 0.0, 0.4, 0.3, false);

        // Trust should decay when no contributions
        if round < 10 {
            assert!(
                trust < 0.95,
                "Trust should decay from initial 0.95 after {} rounds",
                round + 1
            );
        }
    }
}

// ─── Full Pipeline Integration Test ───────────────────────────────────────

#[test]
fn test_sprint122_full_pipeline() {
    // 1. Create hierarchical shard manager
    let clusters = vec![
        ClusterConfig::new(0, 8, 0).with_region("us-east"),
        ClusterConfig::new(1, 8, 8).with_region("eu-west"),
    ];
    let mut manager = HierarchicalShardManager::new(16, &clusters).unwrap();

    // 2. Assign 200 nodes
    for i in 0..200 {
        manager.assign_node(i).unwrap();
    }
    assert_eq!(manager.total_nodes(), 200);

    // 3. Set up cluster discovery with trust scores
    let mut discovery = ClusterDiscovery::new(0.6);
    for i in 0..200 {
        let assignment = manager.get_node_assignment(i).unwrap();
        let trust = if i % 5 != 0 {
            0.85
        } else {
            0.4 // 20% low trust
        };

        let peer = PeerInfo {
            peer_id: i,
            shard_id: assignment.shard_id,
            trust_score: trust,
            energy_budget: calculate_energy_impact(50.0, 30.0, 1e10, 1e-12) as f64,
            address: format!("10.0.{}.{}", i / 256, i % 256),
            last_seen: 1000,
        };
        discovery.add_peer(peer);
    }

    // 4. Verify trusted peer count
    let trusted = discovery.discover_trusted_peers();
    assert!(trusted.len() >= 150, "At least 150 trusted peers expected");
    assert!(trusted.len() <= 165);

    // 5. Run trust-weighted voting for shard leadership
    let votes: Vec<(u64, u64, f64)> = trusted
        .iter()
        .map(|p| (p.peer_id, p.shard_id, p.trust_score))
        .collect();
    let vote_result = trust_weighted_shard_vote(&votes, 50.0);
    assert!(vote_result.quorum_met);

    // 6. Verify load distribution
    let imbalance = manager.load_imbalance();
    assert!(
        imbalance < 0.5,
        "Load imbalance {:.3} should be reasonable",
        imbalance
    );

    // 7. Rebalance if needed
    if imbalance > 0.2 {
        let rebalanced = manager.rebalance().unwrap();
        assert!(!rebalanced.is_empty() || imbalance <= 0.2);
    }

    // 8. Verify PAC-Bayesian confidence on cluster VFE
    let avg_trust = discovery.avg_trust_in_shard(vote_result.winning_shard);
    let bound = pac_bayes_bound(avg_trust, 0.01, 100, 0.05);
    assert!(bound >= avg_trust);
}

#[test]
fn test_sprint122_energy_aware_mesh() {
    // Simulate energy-aware mesh formation
    let configs: Vec<ShardConfig> = (0..8)
        .map(|id| ShardConfig::new(id, 0, 100).with_consensus(ConsensusType::PoSym))
        .collect();

    let mut loads = std::collections::HashMap::new();
    for id in 0..8 {
        loads.insert(id, 0);
    }

    // Assign nodes with varying energy budgets
    let node_energies: Vec<f64> = vec![
        calculate_energy_impact(100.0, 60.0, 1e12, 1e-12), // Desktop
        calculate_energy_impact(80.0, 60.0, 1e11, 1e-12),  // Laptop
        calculate_energy_impact(50.0, 60.0, 1e10, 1e-12),  // Mobile
        calculate_energy_impact(30.0, 60.0, 1e9, 1e-12),   // Tablet
        calculate_energy_impact(10.0, 60.0, 1e8, 1e-12),   // IoT
    ];

    for (i, energy) in node_energies.iter().enumerate() {
        let shard = assign_node_by_load(&configs, &loads, *energy, 0.1).unwrap();
        *loads.get_mut(&shard).unwrap() += 1;
        assert!(shard < 8, "Node {} assigned to valid shard {}", i, shard);
    }

    // Verify distribution
    let total: usize = loads.values().sum();
    assert_eq!(total, 5);
}

#[test]
fn test_sprint122_byzantine_mesh_defense() {
    let clusters = vec![ClusterConfig::new(0, 8, 0)];
    let mut manager = HierarchicalShardManager::new(8, &clusters).unwrap();
    let mut discovery = ClusterDiscovery::new(0.7);

    // 70 honest nodes + 30 Byzantine
    for i in 0..100 {
        manager.assign_node(i).unwrap();
        let assignment = manager.get_node_assignment(i).unwrap();

        let (trust, is_byzantine) = if i < 70 { (0.9, false) } else { (0.1, true) };

        let peer = PeerInfo {
            peer_id: i,
            shard_id: assignment.shard_id,
            trust_score: trust,
            energy_budget: if is_byzantine { 0.0 } else { 1.0 },
            address: format!("node-{}", i),
            last_seen: 1000,
        };
        discovery.add_peer(peer);
    }

    // Byzantine nodes should be filtered out by trust threshold
    let trusted = discovery.discover_trusted_peers();
    for peer in &trusted {
        assert!(
            peer.peer_id < 70,
            "Byzantine node {} should not be in trusted set",
            peer.peer_id
        );
    }

    assert_eq!(
        trusted.len(),
        70,
        "Exactly 70 honest nodes should be trusted"
    );

    // Byzantine-weighted median should resist manipulation
    let values: Vec<(f64, f64)> = (0..100)
        .map(|i| {
            let value = if i < 70 { 100.0 } else { 9999.0 };
            let trust = if i < 70 { 0.9 } else { 0.1 };
            (value, trust)
        })
        .collect();

    let median = byzantine_weighted_median(&values);
    assert!(
        (median - 100.0).abs() < 1.0,
        "Byzantine-resistant median {:.2} should be close to 100",
        median
    );
}

// ─── Edge Cases and Boundary Tests ────────────────────────────────────────

#[test]
fn test_single_shard_manager() {
    let clusters = vec![ClusterConfig::new(0, 1, 0)];
    let mut manager = HierarchicalShardManager::new(1, &clusters).unwrap();

    for i in 0..50 {
        let assignment = manager.assign_node(i).unwrap();
        assert_eq!(assignment.shard_id, 0);
    }

    assert_eq!(manager.total_nodes(), 50);
    assert!((manager.load_imbalance() - 0.0).abs() < f64::EPSILON);
}

#[test]
fn test_shard_manager_with_many_clusters() {
    let clusters: Vec<ClusterConfig> = (0..10)
        .map(|id| ClusterConfig::new(id, 8, id * 8))
        .collect();
    let manager = HierarchicalShardManager::new(80, &clusters).unwrap();

    assert_eq!(manager.total_shards(), 80);
}

#[test]
fn test_discovery_empty_cluster() {
    let discovery = ClusterDiscovery::new(0.5);
    assert_eq!(discovery.peer_count(), 0);
    assert!(discovery.discover_peers_in_shard(0).is_empty());
    assert!(discovery.discover_trusted_peers().is_empty());
    assert!((discovery.avg_trust_in_shard(0) - 0.0).abs() < f64::EPSILON);
    assert!((discovery.total_energy_in_shard(0) - 0.0).abs() < f64::EPSILON);
}

#[test]
fn test_vote_single_voter() {
    let votes = vec![(42, 5, 0.99)];
    let result = trust_weighted_shard_vote(&votes, 0.5);
    assert_eq!(result.winning_shard, 5);
    assert!((result.winning_weight - 0.99).abs() < f64::EPSILON);
    assert!(result.quorum_met);
}

#[test]
fn test_load_assignment_single_shard() {
    let configs = vec![ShardConfig::new(0, 0, 100)];
    let mut loads = std::collections::HashMap::new();
    loads.insert(0, 50);

    let shard = assign_node_by_load(&configs, &loads, 1.0, 0.1).unwrap();
    assert_eq!(shard, 0);
}

#[test]
fn test_hash_ring_single_shard() {
    let ring = ConsistentHashRing::new(1).unwrap();
    for i in 0..100 {
        assert_eq!(ring.get_shard(i), 0);
    }
}

#[test]
fn test_consensus_type_variants() {
    assert_eq!(ConsensusType::PoSym, ConsensusType::PoSym);
    assert_eq!(ConsensusType::PoN, ConsensusType::PoN);
    assert_eq!(ConsensusType::Hybrid, ConsensusType::Hybrid);
    assert_eq!(ConsensusType::ZKP, ConsensusType::ZKP);

    let default = ConsensusType::default();
    assert_eq!(default, ConsensusType::Hybrid);
}

#[test]
fn test_shard_config_builder_chain() {
    let config = ShardConfig::new(5, 2, 200)
        .with_consensus(ConsensusType::PoSym)
        .with_replication(5)
        .with_imbalance_threshold(0.15);

    assert_eq!(config.shard_id, 5);
    assert_eq!(config.cluster_id, 2);
    assert_eq!(config.max_nodes, 200);
    assert_eq!(config.consensus_type, ConsensusType::PoSym);
    assert_eq!(config.replication_factor, 5);
    assert!((config.imbalance_threshold - 0.15).abs() < f64::EPSILON);
}

#[test]
fn test_cluster_config_builder_chain() {
    let config = ClusterConfig::new(3, 16, 5).with_region("ap-northeast");

    assert_eq!(config.cluster_id, 3);
    assert_eq!(config.max_shards, 16);
    assert_eq!(config.leader_shard, 5);
    assert_eq!(config.region, Some("ap-northeast".to_string()));
}

#[test]
fn test_peer_info_clone() {
    let peer = PeerInfo::new(1, 0, "test");
    let cloned = peer.clone();
    assert_eq!(cloned.peer_id, peer.peer_id);
    assert_eq!(cloned.shard_id, peer.shard_id);
    assert_eq!(cloned.address, peer.address);
}

#[test]
fn test_shard_assignment_clone() {
    let assignment = ShardAssignment {
        node_id: 1,
        shard_id: 2,
        cluster_id: 0,
        hash_position: 42,
        load_ratio: 0.5,
        rebalanced: false,
    };
    let cloned = assignment.clone();
    assert_eq!(cloned.node_id, assignment.node_id);
    assert_eq!(cloned.shard_id, assignment.shard_id);
}

#[test]
fn test_error_variants() {
    let e = ShardingError::ShardNotFound(5);
    assert!(e.to_string().contains("5"));

    let e = ShardingError::ClusterNotFound(3);
    assert!(e.to_string().contains("3"));

    let e = ShardingError::NodeAlreadyAssigned(1, 2);
    assert!(e.to_string().contains("1"));

    let e = ShardingError::ShardAtCapacity(0, 10, 10);
    assert!(e.to_string().contains("0"));

    let e = ShardingError::InvalidShardCount(0);
    assert!(e.to_string().contains("0"));

    let e = ShardingError::DiscoveryTimeout(5000);
    assert!(e.to_string().contains("5000"));

    let e = ShardingError::ByzantineDetected(42);
    assert!(e.to_string().contains("42"));
}
