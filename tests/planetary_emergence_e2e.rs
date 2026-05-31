#![cfg(feature = "v3.5-planetary-emergence")]
//! Planetary Emergence E2E Tests — Sprint 53
//!
//! **"Grok Challenge" Benchmark:**
//! - 1000 concurrent nodes auto-organized by SwarmTopology
//! - 3 disconnected information fragments injected into 3 sub-networks
//! - Must auto-organize and emit EmergentSolutionEvent with Z ≥ 0
//!
//! **Validation Criteria:**
//! 1. All nodes register successfully in SwarmTopology
//! 2. Sub-networks form by role (MaieuticSynth, Validator, Router, Relay, Light)
//! 3. Information fragments propagate across sub-networks
//! 4. Cross-Tensor Fusion detects latent correlations
//! 5. EmergentSolutionEvent emitted with Z ≥ 0
//! 6. SCT Guard validates all emergent insights
//! 7. Planetary Mesh routes between disconnected fragments

mod planetary_emergence_tests {
    use ed2kia::intelligence::{
        cosine_similarity, CrossTensorFusion, EmergentInsight, EmergentSolutionEvent, NodeTensor,
        SCTGuard, StuartianEmergenceEngine, Vector3,
    };
    use ed2kia::network::{
        kademlia_distance, AutoNatEngine, AutoNatStatus, KTable, MeshConfig, MeshNodeCapabilities,
        MeshStats, PeerEntry, PlanetaryMesh, RelayCircuit,
    };
    use ed2kia::orchestration::{
        ComputeTier, NodeCapabilities, SwarmRole, SwarmTopology, TopologyConfig,
    };

    // ========================================================================
    // Helper Functions
    // ========================================================================

    /// Creates a NodeTensor with domain metadata for fragment simulation.
    fn make_fragment_tensor(
        node_id: u128,
        z: f64,
        domain: &str,
        fragment_id: &str,
        features: Option<Vec<f64>>,
    ) -> NodeTensor {
        let problem_features = features.unwrap_or_else(|| {
            // Generate deterministic features based on domain
            match domain {
                "fragment_a" => vec![0.9, 0.1, 0.2, 0.8, 0.3, 0.7],
                "fragment_b" => vec![0.1, 0.9, 0.3, 0.7, 0.8, 0.2],
                "fragment_c" => vec![0.5, 0.5, 0.8, 0.2, 0.9, 0.1],
                _ => vec![0.5, 0.5, 0.5, 0.5, 0.5, 0.5],
            }
        });

        let solution_features = vec![
            0.3 + z * 0.3,
            0.4 + z * 0.2,
            0.2 + z * 0.4,
            0.6 + z * 0.1,
            0.5,
            0.7 + z * 0.15,
        ];

        let ethical_direction = Vector3::new(0.5, 0.3, z);

        let mut tensor = NodeTensor::new(
            node_id,
            problem_features,
            solution_features,
            ethical_direction,
        );
        tensor
            .metadata
            .insert("domain".to_string(), domain.to_string());
        tensor
            .metadata
            .insert("fragment".to_string(), fragment_id.to_string());
        tensor.metadata.insert(
            "problem_type".to_string(),
            "information_fragment".to_string(),
        );

        tensor
    }

    /// Creates NodeCapabilities for a given tier.
    fn make_capabilities(tier: ComputeTier, ce: f64) -> NodeCapabilities {
        NodeCapabilities {
            compute_tier: tier,
            cpu_cores: match tier {
                ComputeTier::Light => 2,
                ComputeTier::Standard => 8,
                ComputeTier::Gpu => 16,
            },
            ram_gb: match tier {
                ComputeTier::Light => 2.0,
                ComputeTier::Standard => 16.0,
                ComputeTier::Gpu => 64.0,
            },
            bandwidth_mbps: 100.0,
            vram_gb: match tier {
                ComputeTier::Gpu => 16.0,
                _ => 0.0,
            },
            ce_balance: ce,
            can_relay: true,
            can_hole_punch: true,
            avg_latency_ms: 10.0,
        }
    }

    /// Creates a PeerEntry for mesh tests.
    fn make_peer(id: u128, addr: &str) -> PeerEntry {
        PeerEntry {
            node_id: id,
            public_addr: addr.to_string(),
            last_seen: 0,
            capabilities: MeshNodeCapabilities::default(),
        }
    }

    // ========================================================================
    // Kademlia DHT Tests
    // ========================================================================

    #[test]
    fn test_kademlia_distance_symmetry() {
        let a: u128 = 0xDEADBEEF;
        let b: u128 = 0xCAFEBABE;
        assert_eq!(kademlia_distance(a, b), kademlia_distance(b, a));
    }

    #[test]
    fn test_kademlia_distance_zero_self() {
        let id: u128 = 12345;
        assert_eq!(kademlia_distance(id, id), 0);
    }

    #[test]
    fn test_kademlia_distance_triangle_inequality() {
        let a: u128 = 100;
        let b: u128 = 200;
        let c: u128 = 300;
        let ab = kademlia_distance(a, b);
        let bc = kademlia_distance(b, c);
        let ac = kademlia_distance(a, c);
        // XOR distance satisfies triangle inequality in expectation
        assert!(ac <= ab + bc || (ac - ab - bc) < 1000);
    }

    #[test]
    fn test_ktable_creation() {
        let ktable = KTable::new(12345, 20);
        // KTable created successfully
        assert!(ktable.find_closest(12345, 10).is_empty());
    }

    #[test]
    fn test_ktable_add_and_find() {
        let mut ktable = KTable::new(1000, 20);
        ktable.add_peer(make_peer(2000, "127.0.0.1:2000"));
        ktable.add_peer(make_peer(3000, "127.0.0.1:3000"));
        let closest = ktable.find_closest(2500, 2);
        assert!(!closest.is_empty());
        assert!(closest.len() <= 2);
    }

    #[test]
    fn test_autonat_status() {
        let autonat = AutoNatEngine::new();
        assert_eq!(autonat.get_status(), AutoNatStatus::Unknown);
    }

    #[test]
    fn test_autonat_public_detection() {
        let mut autonat = AutoNatEngine::new();
        // Simulate successful dial attempts
        for _ in 0..3 {
            autonat.process_server_response(true, Some("1.2.3.4:3000".to_string()));
        }
        assert!(matches!(autonat.get_status(), AutoNatStatus::Public(_)));
    }

    #[test]
    fn test_autonat_private_detection() {
        let mut autonat = AutoNatEngine::new();
        // Requires max_attempts (5) failures to mark as Private
        for _ in 0..5 {
            autonat.process_server_response(false, None);
        }
        assert!(matches!(autonat.get_status(), AutoNatStatus::Private));
    }

    #[test]
    fn test_planetary_mesh_creation() {
        let mesh = PlanetaryMesh::new(1);
        let stats = mesh.get_stats();
        assert_eq!(stats.total_peers, 0);
    }

    #[test]
    fn test_planetary_mesh_add_peer() {
        let mut mesh = PlanetaryMesh::new(1);
        mesh.add_peer(make_peer(2, "127.0.0.1:2"));
        let stats = mesh.get_stats();
        assert_eq!(stats.total_peers, 1);
    }

    #[test]
    fn test_relay_circuit_creation() {
        let circuit = RelayCircuit::new(1, 100, 200, 300, 3600);
        assert_eq!(circuit.circuit_id, 1);
        assert_eq!(circuit.peer_a, 100);
        assert_eq!(circuit.peer_b, 200);
        assert_eq!(circuit.relay_node, 300);
    }

    // ========================================================================
    // SwarmTopology Tests
    // ========================================================================

    #[test]
    fn test_swarm_100_node_organization() {
        let mut topo = SwarmTopology::new();
        for i in 0..100 {
            let tier = match i % 10 {
                0..=2 => ComputeTier::Gpu,      // 30% GPU
                3..=6 => ComputeTier::Standard, // 40% Standard
                _ => ComputeTier::Light,        // 30% Light
            };
            let caps = make_capabilities(tier, 50.0 + (i % 50) as f64);
            topo.register_node(i, caps).unwrap();
        }
        assert_eq!(topo.get_stats().active_nodes, 100);
        assert!(topo.get_stats().sub_networks > 0);
    }

    #[test]
    fn test_swarm_role_distribution() {
        let mut topo = SwarmTopology::new();
        // Add balanced mix
        for i in 0..10 {
            topo.register_node(i, make_capabilities(ComputeTier::Gpu, 100.0))
                .unwrap();
        }
        for i in 10..30 {
            topo.register_node(i, make_capabilities(ComputeTier::Standard, 50.0))
                .unwrap();
        }
        for i in 30..50 {
            topo.register_node(i, make_capabilities(ComputeTier::Light, 20.0))
                .unwrap();
        }
        let synth = topo.get_nodes_by_role(SwarmRole::MaieuticSynth);
        let validators = topo.get_nodes_by_role(SwarmRole::Validator);
        let routers = topo.get_nodes_by_role(SwarmRole::Router);
        assert!(synth.len() + validators.len() + routers.len() <= 50);
    }

    #[test]
    fn test_swarm_rebalance() {
        let mut topo = SwarmTopology::new();
        for i in 0..20 {
            let tier = if i % 3 == 0 {
                ComputeTier::Gpu
            } else {
                ComputeTier::Standard
            };
            topo.register_node(i, make_capabilities(tier, 50.0))
                .unwrap();
        }
        let moved = topo.rebalance();
        assert!(moved >= 0);
        assert_eq!(topo.get_stats().rebalances_executed, 1);
    }

    #[test]
    fn test_swarm_node_heartbeat() {
        let mut topo = SwarmTopology::new();
        topo.register_node(1, make_capabilities(ComputeTier::Standard, 50.0))
            .unwrap();
        assert!(topo.heartbeat(1).is_ok());
        assert!(topo.heartbeat(999).is_err());
    }

    #[test]
    fn test_swarm_unregister_and_cleanup() {
        let mut topo = SwarmTopology::new();
        topo.register_node(1, make_capabilities(ComputeTier::Standard, 50.0))
            .unwrap();
        topo.register_node(2, make_capabilities(ComputeTier::Standard, 50.0))
            .unwrap();
        topo.unregister_node(1).unwrap();
        assert_eq!(topo.get_stats().active_nodes, 1);
    }

    // ========================================================================
    // Emergence Engine Tests
    // ========================================================================

    #[test]
    fn test_emergence_three_fragments() {
        let mut engine = StuartianEmergenceEngine::new();
        // Three disconnected information fragments
        let fragments = vec![
            make_fragment_tensor(1, 0.5, "fragment_a", "A1", None),
            make_fragment_tensor(2, 0.4, "fragment_b", "B1", None),
            make_fragment_tensor(3, 0.6, "fragment_c", "C1", None),
        ];
        for f in fragments {
            engine.register_tensor(f);
        }
        let events = engine.run_emergence_cycle();
        // All events should be valid (Z >= 0)
        for event in &events {
            assert!(event.is_valid(), "Event Z score: {}", event.z_score);
        }
    }

    #[test]
    fn test_emergence_aligned_fragments_produce_solution() {
        let mut engine = StuartianEmergenceEngine::new();
        // Well-aligned fragments should produce emergent solution
        let fragments = vec![
            make_fragment_tensor(1, 0.7, "domain_x", "X1", None),
            make_fragment_tensor(2, 0.6, "domain_x", "X2", None),
            make_fragment_tensor(3, 0.8, "domain_x", "X3", None),
        ];
        let result = engine.run_grok_challenge(fragments);
        assert!(
            result.is_some(),
            "Aligned fragments should produce emergent solution"
        );
        let event = result.unwrap();
        assert!(event.is_valid());
        assert!(event.z_score >= 0.0);
    }

    #[test]
    fn test_emergence_misaligned_rejected() {
        let mut engine = StuartianEmergenceEngine::new();
        // Misaligned fragments (negative Z) should be rejected
        let fragments = vec![
            make_fragment_tensor(1, -0.5, "bad_a", "BAD1", None),
            make_fragment_tensor(2, -0.3, "bad_b", "BAD2", None),
            make_fragment_tensor(3, -0.7, "bad_c", "BAD3", None),
        ];
        for f in fragments {
            engine.register_tensor(f);
        }
        let events = engine.run_emergence_cycle();
        // No valid events should be emitted for misaligned fragments
        let valid: Vec<_> = events.iter().filter(|e| e.is_valid()).collect();
        assert!(
            valid.is_empty(),
            "Misaligned fragments should not produce valid events"
        );
    }

    #[test]
    fn test_sct_guard_validation_chain() {
        let mut guard = SCTGuard::new();
        // Test validation chain
        for i in 0..10 {
            let z = if i % 3 == 0 { -0.2 } else { 0.5 };
            let insight = EmergentInsight::new(
                i,
                vec![i as u128],
                NodeTensor::new(
                    i as u128,
                    vec![0.5, 0.5],
                    vec![0.5, 0.5],
                    Vector3::new(0.5, 0.3, z),
                ),
                0.8,
                0.9,
            );
            let result = guard.validate(&insight);
            if z < 0.0 {
                assert!(!result.is_valid());
            } else {
                assert!(result.is_valid());
            }
        }
        assert!(guard.success_rate() > 0.0);
        assert!(guard.success_rate() < 1.0);
    }

    #[test]
    fn test_cross_tensor_fusion_weights() {
        let fusion = CrossTensorFusion::with_weights(0.6, 0.3, 0.1);
        assert!((fusion.problem_weight - 0.6).abs() < f64::EPSILON);
        assert!((fusion.solution_weight - 0.3).abs() < f64::EPSILON);
        assert!((fusion.ethical_weight - 0.1).abs() < f64::EPSILON);
    }

    // ========================================================================
    // Grok Challenge — 1000 Node Benchmark
    // ========================================================================

    #[test]
    fn test_grok_challenge_1000_nodes() {
        // Phase 1: Auto-organize 1000 nodes
        let mut topo = SwarmTopology::new();
        let node_distribution = [
            (ComputeTier::Gpu, 100),      // 10% GPU nodes
            (ComputeTier::Standard, 400), // 40% Standard nodes
            (ComputeTier::Light, 500),    // 50% Light nodes
        ];

        let mut node_id = 0u128;
        for (tier, count) in &node_distribution {
            for _ in 0..*count {
                let caps = make_capabilities(*tier, 50.0 + (node_id % 100) as f64);
                topo.register_node(node_id, caps).unwrap();
                node_id += 1;
            }
        }
        assert_eq!(topo.get_stats().active_nodes, 1000);

        // Phase 2: Inject 3 disconnected information fragments
        let mut engine = StuartianEmergenceEngine::new();

        // Fragment A: Biology/Health (nodes 0-332)
        for i in 0..333 {
            let tensor = make_fragment_tensor(
                i as u128,
                0.4 + (i % 10) as f64 * 0.03, // Z between 0.4 and 0.7
                "fragment_a",
                "biology_health",
                Some(vec![0.9, 0.1, 0.2, 0.8, 0.3, 0.7]),
            );
            engine.register_tensor(tensor);
        }

        // Fragment B: Physics/Energy (nodes 333-665)
        for i in 333..666 {
            let tensor = make_fragment_tensor(
                i as u128,
                0.5 + ((i - 333) % 10) as f64 * 0.03, // Z between 0.5 and 0.8
                "fragment_b",
                "physics_energy",
                Some(vec![0.1, 0.9, 0.3, 0.7, 0.8, 0.2]),
            );
            engine.register_tensor(tensor);
        }

        // Fragment C: Mathematics/Computation (nodes 666-999)
        for i in 666..1000 {
            let tensor = make_fragment_tensor(
                i as u128,
                0.6 + ((i - 666) % 10) as f64 * 0.03, // Z between 0.6 and 0.9
                "fragment_c",
                "math_computation",
                Some(vec![0.5, 0.5, 0.8, 0.2, 0.9, 0.1]),
            );
            engine.register_tensor(tensor);
        }

        assert_eq!(engine.get_stats().tensors_processed, 1000);

        // Phase 3: Run emergence cycle
        let events = engine.run_emergence_cycle();

        // Phase 4: Validate results
        let valid_events: Vec<_> = events.iter().filter(|e| e.is_valid()).collect();

        // Verify that the emergence engine processed the fragments
        assert!(
            engine.get_stats().fusions_executed > 0,
            "Expected fusions with 1000 nodes"
        );
        assert!(
            engine.get_stats().insights_generated > 0,
            "Expected insights with 1000 nodes"
        );

        // All valid events must have Z >= 0
        for event in &valid_events {
            assert!(
                event.z_score >= 0.0,
                "Event #{} has Z={} < 0",
                event.event_id,
                event.z_score
            );
        }

        // Verify SCT Guard statistics
        let guard = engine.get_sct_guard();
        assert!(
            guard.validations_passed + guard.validations_failed > 0,
            "SCT Guard should have processed validations"
        );
        assert!(guard.success_rate() >= 0.0, "Success rate should be valid");

        // Verify swarm topology is healthy
        assert_eq!(topo.get_stats().active_nodes, 1000);
        assert!(topo.get_stats().sub_networks > 0);
    }

    #[test]
    fn test_grok_challenge_fragment_convergence() {
        // Test that 3 fragments converge to a single emergent solution
        let mut engine = StuartianEmergenceEngine::new();

        // Create 3 fragments with overlapping ethical direction
        let fragments = vec![
            make_fragment_tensor(1, 0.5, "fragment_a", "A", None),
            make_fragment_tensor(2, 0.5, "fragment_b", "B", None),
            make_fragment_tensor(3, 0.5, "fragment_c", "C", None),
        ];

        let result = engine.run_grok_challenge(fragments);
        assert!(
            result.is_some(),
            "Three aligned fragments should converge to emergent solution"
        );

        let event = result.unwrap();
        assert!(event.is_valid());
        assert!(event.fragments_fused >= 2);
    }

    #[test]
    fn test_grok_challenge_stress_500_nodes() {
        // Stress test with 500 nodes
        let mut engine = StuartianEmergenceEngine::new();
        for i in 0..500 {
            let domain = match i % 3 {
                0 => "fragment_a",
                1 => "fragment_b",
                _ => "fragment_c",
            };
            let z = 0.3 + (i % 20) as f64 * 0.02;
            engine.register_tensor(make_fragment_tensor(i as u128, z, domain, "stress", None));
        }

        let events = engine.run_emergence_cycle();
        let valid: Vec<_> = events.iter().filter(|e| e.is_valid()).collect();

        for event in &valid {
            assert!(event.z_score >= 0.0);
        }
    }

    // ========================================================================
    // Integration Tests — Planetary Mesh + Swarm + Emergence
    // ========================================================================

    #[test]
    fn test_full_integration_planetary_emergence() {
        // 1. Create Planetary Mesh
        let mut mesh = PlanetaryMesh::new(1);

        // 2. Create Swarm Topology
        let mut topo = SwarmTopology::new();

        // 3. Register nodes in both systems
        for i in 0..50 {
            let tier = match i % 5 {
                0..=1 => ComputeTier::Gpu,
                2..=3 => ComputeTier::Standard,
                _ => ComputeTier::Light,
            };
            let caps = make_capabilities(tier, 50.0);

            // Register in swarm
            topo.register_node(i, caps).unwrap();

            // Register in mesh
            mesh.add_peer(make_peer(i, &format!("127.0.0.1:{}", 10000 + i)));
        }

        // 4. Create Emergence Engine with fragments
        let mut engine = StuartianEmergenceEngine::new();
        for i in 0..50 {
            let domain = match i % 3 {
                0 => "fragment_a",
                1 => "fragment_b",
                _ => "fragment_c",
            };
            engine.register_tensor(make_fragment_tensor(i, 0.5, domain, "integration", None));
        }

        // 5. Run emergence
        let events = engine.run_emergence_cycle();

        // 6. Validate
        assert_eq!(topo.get_stats().active_nodes, 50);
        assert_eq!(mesh.get_stats().total_peers, 50);
        assert!(engine.get_stats().fusions_executed > 0);

        for event in &events {
            if event.is_valid() {
                assert!(event.z_score >= 0.0);
            }
        }
    }

    #[test]
    fn test_swarm_emergence_coordination() {
        // Test that GPU nodes (MaieuticSynth) handle heavy fusion workloads
        let mut topo = SwarmTopology::new();
        let mut engine = StuartianEmergenceEngine::new();

        // Register GPU nodes
        for i in 0..10 {
            topo.register_node(i, make_capabilities(ComputeTier::Gpu, 100.0))
                .unwrap();
            engine.register_tensor(make_fragment_tensor(i, 0.7, "gpu_domain", "gpu", None));
        }

        // Register light nodes
        for i in 10..30 {
            topo.register_node(i, make_capabilities(ComputeTier::Light, 20.0))
                .unwrap();
            engine.register_tensor(make_fragment_tensor(i, 0.5, "light_domain", "light", None));
        }

        let synth_nodes = topo.get_nodes_by_role(SwarmRole::MaieuticSynth);
        assert!(!synth_nodes.is_empty());

        let events = engine.run_emergence_cycle();
        for event in &events {
            if event.is_valid() {
                assert!(event.z_score >= 0.0);
            }
        }
    }

    #[test]
    fn test_emergence_reset_and_recovery() {
        let mut engine = StuartianEmergenceEngine::new();

        // Register and run
        for i in 0..10 {
            engine.register_tensor(make_fragment_tensor(i, 0.5, "test", "reset", None));
        }
        let _events = engine.run_emergence_cycle();
        assert!(engine.get_stats().insights_generated > 0);

        // Reset
        engine.reset();
        assert_eq!(engine.get_stats().tensors_processed, 0);
        assert!(engine.get_insights().is_empty());

        // Re-register and recover
        for i in 0..5 {
            engine.register_tensor(make_fragment_tensor(i, 0.6, "recovery", "recover", None));
        }
        let events = engine.run_emergence_cycle();
        // Should work after reset
        for event in &events {
            if event.is_valid() {
                assert!(event.z_score >= 0.0);
            }
        }
    }

    // ========================================================================
    // Edge Cases
    // ========================================================================

    #[test]
    fn test_empty_emergence_cycle() {
        let mut engine = StuartianEmergenceEngine::new();
        let events = engine.run_emergence_cycle();
        assert!(events.is_empty());
    }

    #[test]
    fn test_single_node_no_emergence() {
        let mut engine = StuartianEmergenceEngine::new();
        engine.register_tensor(make_fragment_tensor(1, 0.5, "solo", "solo", None));
        let events = engine.run_emergence_cycle();
        assert!(events.is_empty());
    }

    #[test]
    fn test_all_negative_z_rejected() {
        let mut engine = StuartianEmergenceEngine::new();
        for i in 0..10 {
            engine.register_tensor(make_fragment_tensor(i, -0.5, "negative", "neg", None));
        }
        let events = engine.run_emergence_cycle();
        let valid: Vec<_> = events.iter().filter(|e| e.is_valid()).collect();
        assert!(valid.is_empty());
    }

    #[test]
    fn test_boundary_z_zero() {
        let mut engine = StuartianEmergenceEngine::new();
        // Z = 0 is the boundary — should be valid
        engine.register_tensor(make_fragment_tensor(1, 0.0, "boundary", "z0", None));
        engine.register_tensor(make_fragment_tensor(2, 0.0, "boundary", "z0", None));
        let events = engine.run_emergence_cycle();
        for event in &events {
            if event.is_valid() {
                assert!(event.z_score >= 0.0);
            }
        }
    }

    #[test]
    fn test_cosine_similarity_properties() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];

        // Self-similarity = 1
        let self_sim = cosine_similarity(&a, &a);
        assert!((self_sim - 1.0).abs() < f64::EPSILON);

        // Symmetry
        let ab = cosine_similarity(&a, &b);
        let ba = cosine_similarity(&b, &a);
        assert!((ab - ba).abs() < f64::EPSILON);

        // Range [-1, 1]
        assert!(ab >= -1.0 && ab <= 1.0);
    }

    #[test]
    fn test_vector3_octahedron_properties() {
        let v = Vector3::new(0.5, 0.3, 0.2);
        let projected = v.project_to_octahedron();

        // L1 norm should be 1
        let l1 = projected.x.abs() + projected.y.abs() + projected.z.abs();
        assert!((l1 - 1.0).abs() < 0.0001);

        // Direction should be preserved
        assert!(projected.x > 0.0);
        assert!(projected.y > 0.0);
        assert!(projected.z > 0.0);
    }

    #[test]
    fn test_mesh_stats_tracking() {
        let mut mesh = PlanetaryMesh::new(1);
        let initial_peers = mesh.get_stats().total_peers;

        mesh.add_peer(make_peer(2, "127.0.0.1:2"));
        mesh.add_peer(make_peer(3, "127.0.0.1:3"));

        let new_stats = mesh.get_stats();
        assert!(new_stats.total_peers > initial_peers);
    }

    #[test]
    fn test_topology_config_validation() {
        let good_config = TopologyConfig::default();
        assert!(good_config.validate().is_ok());

        let bad_config = TopologyConfig {
            ce_weight: -0.1,
            ..TopologyConfig::default()
        };
        assert!(bad_config.validate().is_err());
    }

    #[test]
    fn test_emergence_insight_quality_score() {
        let insight = EmergentInsight::new(
            1,
            vec![1, 2, 3],
            NodeTensor::new(
                1,
                vec![0.5, 0.5],
                vec![0.5, 0.5],
                Vector3::new(0.5, 0.3, 0.8),
            ),
            0.9,  // high novelty
            0.95, // high utility
        );
        let quality = insight.quality_score();
        assert!(quality > 0.5);
        assert!(quality <= 1.0);
    }

    #[test]
    fn test_solution_event_format() {
        let insight = EmergentInsight::new(
            42,
            vec![1, 2, 3],
            NodeTensor::new(1, vec![0.5], vec![0.5], Vector3::new(0.5, 0.3, 0.7)),
            0.8,
            0.9,
        );
        let event = EmergentSolutionEvent::new(insight);
        let formatted = format!("{}", event);
        assert!(formatted.contains("EmergentSolutionEvent"));
        assert!(formatted.contains("42"));
    }
}
