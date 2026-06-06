//! Collective Inference Tests — Sprint 107
//!
//! Tests multi-agent active inference, trust-weighted aggregation,
//! and Byzantine detection in the Noosphere gossip protocol.

use candle_core::{Device, Tensor};
use native_audit::TensorAudit;

#[test]
fn test_collective_active_inference() {
    let device = Device::Cpu;
    let audit = TensorAudit::load_smollm2(&device, vec![6]).expect("Fallo al cargar modelo");

    let prompt = "Write a helpful response to assist the user.";

    // Extract hidden states from multiple "agents" using same prompt for matching shapes
    let local_hidden = audit.forward_extract(prompt).expect("Fallo forward local");
    let peer1 = audit.forward_extract(prompt).expect("Fallo forward peer1");
    let peer2 = audit.forward_extract(prompt).expect("Fallo forward peer2");

    let peer_contributions = vec![peer1, peer2];
    let peer_trusts = vec![0.6, 0.4]; // Trust scores (Existential Credits)

    // Safe prior = average of all contributions
    let safe_prior = local_hidden.clone();

    let start = std::time::Instant::now();
    let collective_steered = audit
        .collective_steer(
            &local_hidden,
            peer_contributions,
            peer_trusts.clone(),
            &safe_prior,
            10,
            0.05,
            10.0,
            0.5,
            0.1,
            0.05,
            0.01,
        )
        .expect("Fallo collective_steer");
    let latency = start.elapsed().as_secs_f64() * 1000.0;

    println!("\n   Collective Active Inference:");
    println!("   Agents: 3");
    println!("   Peer Trusts: {:?}", peer_trusts);
    println!("   Collective Latency: {:.2} ms", latency);
    println!("   Steered Shape: {:?}", collective_steered.shape());

    // Verify output is valid
    assert!(
        collective_steered.shape().dims() == local_hidden.shape().dims(),
        "Output shape must match input shape"
    );
    assert!(
        latency < 5000.0,
        "Collective steering should complete in reasonable time"
    );
}

#[test]
fn test_trust_weighted_aggregation() {
    let device = Device::Cpu;

    // Create test tensors with known values [4,]
    let t1 = Tensor::from_vec(vec![1.0f32; 4], (4,), &device).unwrap();
    let t2 = Tensor::from_vec(vec![2.0f32; 4], (4,), &device).unwrap();
    let t3 = Tensor::from_vec(vec![3.0f32; 4], (4,), &device).unwrap();

    let contributions = vec![t1, t2, t3];
    let trusts = vec![0.1, 0.7, 0.2];

    let average = native_audit::symbolic_fusion::CollectiveInference::trust_weighted_average(
        &contributions,
        &trusts,
        &device,
    )
    .expect("Fallo trust_weighted_average");

    let values: Vec<f32> = average.to_vec1().expect("Fallo to_vec1");

    // Expected: 0.1*1 + 0.7*2 + 0.2*3 = 0.1 + 1.4 + 0.6 = 2.1
    let expected = 2.1f32;
    for v in &values {
        assert!(
            (*v - expected).abs() < 0.01,
            "Expected ~{}, got {}",
            expected,
            v
        );
    }

    println!("\n   Trust-Weighted Aggregation:");
    println!("   Trusts: {:?}", trusts);
    println!("   Weighted Average: {:.4}", values[0]);
    println!("   Expected: {:.4}", expected);
}

#[test]
fn test_byzantine_detection() {
    let normal_sig = native_audit::TopologicalSignature {
        betti_numbers: vec![3, 1, 0],
        persistence_intervals: vec![(0.0, 1.0)],
    };
    let byzantine_sig = native_audit::TopologicalSignature {
        betti_numbers: vec![100, 50, 25],
        persistence_intervals: vec![(10.0, 100.0)],
    };

    // Network: 5 normal nodes + 1 Byzantine
    let all_sigs = vec![
        normal_sig.clone(),
        normal_sig.clone(),
        normal_sig.clone(),
        normal_sig.clone(),
        normal_sig.clone(),
        byzantine_sig.clone(),
    ];

    let byzantine =
        native_audit::symbolic_fusion::NoosphereGossip::detect_byzantine(&all_sigs, 5.0);

    println!("\n   Byzantine Detection:");
    println!("   Total Nodes: {}", all_sigs.len());
    println!("   Detected Byzantine: {:?}", byzantine);
    println!("   Expected: [5]");

    assert!(
        byzantine.contains(&5),
        "Byzantine node at index 5 should be detected"
    );
    assert!(
        byzantine.len() == 1,
        "Only the Byzantine node should be flagged"
    );
}

#[test]
fn test_collective_vfe_reduction() {
    // Simulate VFE values from multiple agents
    let local_vfe = 70.0f32;
    let peer_vfes = vec![65.0, 60.0, 55.0, 50.0];
    let trusts = vec![0.4, 0.3, 0.2, 0.1];

    let reduction = native_audit::symbolic_fusion::CollectiveInference::collective_vfe_reduction(
        local_vfe, &peer_vfes, &trusts,
    );

    println!("\n   Collective VFE Reduction:");
    println!("   Local VFE: {:.2}", local_vfe);
    println!("   Peer VFEs: {:?}", peer_vfes);
    println!("   Trusts: {:?}", trusts);
    println!("   Collective Reduction: {:.2}%", reduction);

    assert!(reduction > 0.0, "Collective VFE should show improvement");
    assert!(reduction < 100.0, "Reduction should be bounded");
}

#[test]
fn test_symbolic_graph_centrality() {
    use native_audit::sae_integration::FeatureCategory;

    let names = vec![
        "harm_intent".to_string(),
        "helpfulness".to_string(),
        "safety_check".to_string(),
        "deception".to_string(),
        "cooperation".to_string(),
    ];
    let categories = vec![
        FeatureCategory::Harmful,
        FeatureCategory::Helpful,
        FeatureCategory::Safety,
        FeatureCategory::Deception,
        FeatureCategory::Helpful,
    ];

    let graph = native_audit::symbolic_fusion::SymbolicGraph::from_features(&names, &categories);
    let centrality = graph.centrality();

    println!("\n   Symbolic Graph Centrality:");
    println!("   Nodes: {}", graph.nodes.len());
    println!("   Relations: {}", graph.relations.len());
    println!("   Coherence: {:.4}", graph.coherence());
    println!("   Top 3 Central Nodes:");
    for (idx, score) in centrality.iter().take(3) {
        println!("     {}: {} (score: {:.4})", idx, graph.nodes[*idx], score);
    }

    assert_eq!(graph.nodes.len(), 5);
    assert!(!graph.relations.is_empty());
    assert!(graph.coherence() > 0.0);
}

#[test]
fn test_fusion_engine_peer_trust() {
    use native_audit::sae_integration::FeatureCategory;

    let safe_names = vec![
        "helpfulness".to_string(),
        "safety_check".to_string(),
        "cooperation".to_string(),
    ];
    let safe_cats = vec![
        FeatureCategory::Helpful,
        FeatureCategory::Safety,
        FeatureCategory::Helpful,
    ];
    let safe_graph =
        native_audit::symbolic_fusion::SymbolicGraph::from_features(&safe_names, &safe_cats);

    // Honest peer: similar to safe graph
    let honest_graph = safe_graph.clone();

    // Dishonest peer: very different
    let dishonest_names = vec![
        "harm_intent".to_string(),
        "deception".to_string(),
        "violence".to_string(),
    ];
    let dishonest_cats = vec![
        FeatureCategory::Harmful,
        FeatureCategory::Deception,
        FeatureCategory::Harmful,
    ];
    let dishonest_graph = native_audit::symbolic_fusion::SymbolicGraph::from_features(
        &dishonest_names,
        &dishonest_cats,
    );

    let engine =
        native_audit::symbolic_fusion::FusionEngine::new(safe_graph.clone(), safe_graph, 0.3);

    let honest_trust = engine.peer_trust_score(&honest_graph);
    let dishonest_trust = engine.peer_trust_score(&dishonest_graph);

    println!("\n   Fusion Engine Peer Trust:");
    println!("   Honest Peer Trust: {:.4}", honest_trust);
    println!("   Dishonest Peer Trust: {:.4}", dishonest_trust);

    assert!(
        honest_trust > dishonest_trust,
        "Honest peer should have higher trust than dishonest peer"
    );
    assert!(honest_trust > 0.5, "Honest peer trust should be high");
}
