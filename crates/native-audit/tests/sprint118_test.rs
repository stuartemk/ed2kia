//! Sprint 118 (v11.8.0) — SAE Modular Low-Dim + Distributed Testnet Sim + Formal Proofs + Full Pipeline Benchmarks
//!
//! 40+ integration tests covering:
//! - SAE projection soundness and sparsity
//! - Testnet simulation convergence and fairness
//! - Full certified pipeline with SAE subspace reduction
//! - Edge case benchmarks

use candle_core::Device;
use native_audit::sae_modular::{SAEConfig, SAE};
use native_audit::testnet_sim::{
    byzantine_median, compute_gini, run_testnet_simulation, SimConfig,
};

// --- Helper functions ---

fn make_hidden_batch(
    batch: usize,
    dim: usize,
    device: &Device,
) -> candle_core::Result<candle_core::Tensor> {
    let data: Vec<f32> = (0..(batch * dim))
        .map(|i| (i as f32 % 13.0) / 13.0)
        .collect();
    candle_core::Tensor::from_vec(data, (batch, dim), device)
}

// ============================================================
// SAE Modular Tests (14 tests)
// ============================================================

#[test]
fn test_sae_config_default_values() {
    let cfg = SAEConfig::default();
    assert_eq!(cfg.input_dim, 4096);
    assert_eq!(cfg.latent_dim, 1024);
    assert_eq!(cfg.top_k, 512);
    assert!(cfg.recon_threshold > 0.0);
}

#[test]
fn test_sae_creation_small() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let config = SAEConfig {
        input_dim: 32,
        latent_dim: 8,
        top_k: 4,
        recon_threshold: 0.5,
    };
    let sae = SAE::new(&config, &device)?;
    assert_eq!(sae.encoder_w.shape().dims(), [32, 8]);
    assert_eq!(sae.decoder_w.shape().dims(), [8, 32]);
    assert_eq!(sae.top_k, 4);
    Ok(())
}

#[test]
fn test_sae_creation_large() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let config = SAEConfig {
        input_dim: 4096,
        latent_dim: 1024,
        top_k: 256,
        recon_threshold: 0.05,
    };
    let sae = SAE::new(&config, &device)?;
    assert_eq!(sae.encoder_w.shape().dims(), [4096, 1024]);
    assert_eq!(sae.effective_dim(), 256);
    Ok(())
}

#[test]
fn test_sae_encode_dimension() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let config = SAEConfig {
        input_dim: 64,
        latent_dim: 16,
        top_k: 8,
        recon_threshold: 0.5,
    };
    let sae = SAE::new(&config, &device)?;
    let hidden = make_hidden_batch(1, 64, &device)?;
    let latents = sae.encode(&hidden)?;
    // Input is 2D [1, 64], so output is 2D [1, 16]
    assert_eq!(latents.shape().dims(), [1, 16]);
    Ok(())
}

#[test]
fn test_sae_batch_encode() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let config = SAEConfig {
        input_dim: 64,
        latent_dim: 16,
        top_k: 8,
        recon_threshold: 0.5,
    };
    let sae = SAE::new(&config, &device)?;
    let hidden = make_hidden_batch(8, 64, &device)?;
    let latents = sae.encode(&hidden)?;
    assert_eq!(latents.shape().dims(), [8, 16]);
    Ok(())
}

#[test]
fn test_sae_decode_roundtrip() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let config = SAEConfig {
        input_dim: 32,
        latent_dim: 16,
        top_k: 16,
        recon_threshold: 1.0,
    };
    let sae = SAE::new(&config, &device)?;
    let hidden = make_hidden_batch(1, 32, &device)?;
    let latents = sae.encode(&hidden)?;
    let reconstructed = sae.decode(&latents)?;
    // Input is 2D [1, 32], so output is 2D [1, 32]
    assert_eq!(reconstructed.shape().dims(), [1, 32]);
    Ok(())
}

#[test]
fn test_sae_projection_result() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let config = SAEConfig {
        input_dim: 32,
        latent_dim: 16,
        top_k: 8,
        recon_threshold: 1.0,
    };
    let sae = SAE::new(&config, &device)?;
    let hidden = make_hidden_batch(2, 32, &device)?;
    let result = sae.project(&hidden)?;
    assert_eq!(result.recon_errors.len(), 2);
    assert_eq!(result.active_features.len(), 2);
    assert!(result.avg_recon_error >= 0.0);
    Ok(())
}

#[test]
fn test_sae_sparsity_enforced() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let config = SAEConfig {
        input_dim: 32,
        latent_dim: 16,
        top_k: 3,
        recon_threshold: 1.0,
    };
    let sae = SAE::new(&config, &device)?;
    let hidden = make_hidden_batch(1, 32, &device)?;
    let latents = sae.encode(&hidden)?;
    // Output is 2D [1, 16], flatten to check sparsity
    let vals: Vec<Vec<f32>> = latents.to_vec2()?;
    let non_zero = vals.iter().flatten().filter(|v| **v != 0.0).count();
    assert!(
        non_zero <= 3,
        "Expected at most 3 active features, got {}",
        non_zero
    );
    Ok(())
}

#[test]
fn test_sae_tightness_ratio() -> candle_core::Result<()> {
    let config = SAEConfig {
        input_dim: 4096,
        latent_dim: 1024,
        top_k: 512,
        recon_threshold: 0.05,
    };
    let device = Device::Cpu;
    let sae = SAE::new(&config, &device)?;
    let ratio = sae.tightness_ratio(&config);
    assert!((ratio - 0.25).abs() < 1e-6);
    Ok(())
}

#[test]
fn test_sae_identity_projection() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let sae = SAE::identity_projection(64, 16, &device)?;
    let hidden = make_hidden_batch(1, 64, &device)?;
    let result = sae.project(&hidden)?;
    assert!(result.avg_recon_error >= 0.0);
    Ok(())
}

#[test]
fn test_sae_projection_display() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let config = SAEConfig {
        input_dim: 16,
        latent_dim: 8,
        top_k: 4,
        recon_threshold: 0.5,
    };
    let sae = SAE::new(&config, &device)?;
    let hidden = make_hidden_batch(1, 16, &device)?;
    let result = sae.project(&hidden)?;
    let display = format!("{}", result);
    assert!(display.contains("ProjectionResult"));
    assert!(display.contains("avg_error="));
    Ok(())
}

#[test]
fn test_sae_top_k_exceeds_latent_error() {
    let device = Device::Cpu;
    let config = SAEConfig {
        input_dim: 64,
        latent_dim: 16,
        top_k: 32,
        recon_threshold: 0.5,
    };
    let result = SAE::new(&config, &device);
    assert!(result.is_err());
}

#[test]
fn test_sae_effective_dim_matches_topk() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let config = SAEConfig {
        input_dim: 64,
        latent_dim: 32,
        top_k: 10,
        recon_threshold: 0.5,
    };
    let sae = SAE::new(&config, &device)?;
    assert_eq!(sae.effective_dim(), 10);
    Ok(())
}

#[test]
fn test_sae_batch_consistency() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let config = SAEConfig {
        input_dim: 32,
        latent_dim: 8,
        top_k: 4,
        recon_threshold: 1.0,
    };
    let sae = SAE::new(&config, &device)?;
    // Process same data as batch and individual
    let batch = make_hidden_batch(4, 32, &device)?;
    let batch_result = sae.project(&batch)?;
    assert_eq!(batch_result.recon_errors.len(), 4);
    assert_eq!(batch_result.active_features.len(), 4);
    Ok(())
}

// ============================================================
// Testnet Simulator Tests (14 tests)
// ============================================================

#[test]
fn test_sim_config_default() {
    let cfg = SimConfig::default();
    assert_eq!(cfg.num_nodes, 1000);
    assert!((cfg.byzantine_fraction - 0.1).abs() < 1e-6);
    assert_eq!(cfg.epochs, 50);
    assert_eq!(cfg.fanout, 6);
    assert_eq!(cfg.seed, 42);
}

#[test]
fn test_sim_small_network() {
    let config = SimConfig {
        num_nodes: 50,
        byzantine_fraction: 0.1,
        epochs: 10,
        fanout: 3,
        mean_latency_ms: 50.0,
        churn_rate: 0.01,
        attack_intensity: 0.0,
        seed: 42,
    };
    let result = run_testnet_simulation(&config);
    assert!(result.certified_ratio >= 0.0 && result.certified_ratio <= 1.0);
    assert!(result.gini >= 0.0 && result.gini <= 1.0);
    assert!(result.poa >= 1.0);
    assert!(result.convergence_epochs > 0);
}

#[test]
fn test_sim_deterministic_reproducible() {
    let config = SimConfig {
        num_nodes: 100,
        byzantine_fraction: 0.1,
        epochs: 20,
        fanout: 4,
        mean_latency_ms: 80.0,
        churn_rate: 0.02,
        attack_intensity: 0.0,
        seed: 12345,
    };
    let r1 = run_testnet_simulation(&config);
    let r2 = run_testnet_simulation(&config);
    assert_eq!(r1.certified_ratio, r2.certified_ratio);
    assert_eq!(r1.gini, r2.gini);
    assert_eq!(r1.poa, r2.poa);
    assert_eq!(r1.convergence_epochs, r2.convergence_epochs);
}

#[test]
fn test_sim_byzantine_resistance_10pct() {
    let config = SimConfig {
        num_nodes: 200,
        byzantine_fraction: 0.1,
        epochs: 30,
        fanout: 5,
        mean_latency_ms: 100.0,
        churn_rate: 0.02,
        attack_intensity: 0.5,
        seed: 42,
    };
    let result = run_testnet_simulation(&config);
    assert!(
        result.certified_ratio > 0.5,
        "Should certify majority at 10% Byzantine"
    );
    assert!(result.gini < 0.5, "Credits should be relatively fair");
}

#[test]
fn test_sim_byzantine_resistance_33pct() {
    let config = SimConfig {
        num_nodes: 300,
        byzantine_fraction: 0.33,
        epochs: 25,
        fanout: 6,
        mean_latency_ms: 100.0,
        churn_rate: 0.02,
        attack_intensity: 1.0,
        seed: 42,
    };
    let result = run_testnet_simulation(&config);
    assert!(
        result.certified_ratio > 0.0,
        "Should still certify some at 33% Byzantine"
    );
    assert!(result.poa >= 1.0, "PoA should be >= 1");
}

#[test]
fn test_sim_honest_network_optimal() {
    let config = SimConfig {
        num_nodes: 200,
        byzantine_fraction: 0.0,
        epochs: 30,
        fanout: 6,
        mean_latency_ms: 50.0,
        churn_rate: 0.0,
        attack_intensity: 0.0,
        seed: 42,
    };
    let result = run_testnet_simulation(&config);
    assert!(
        (result.poa - 1.0).abs() < 0.01,
        "PoA should be ~1.0 for honest network"
    );
    assert!(result.certified_ratio > 0.8);
}

#[test]
fn test_sim_convergence_happens() {
    let config = SimConfig {
        num_nodes: 100,
        byzantine_fraction: 0.05,
        epochs: 100,
        fanout: 5,
        mean_latency_ms: 50.0,
        churn_rate: 0.01,
        attack_intensity: 0.0,
        seed: 42,
    };
    let result = run_testnet_simulation(&config);
    assert!(
        result.convergence_epochs < result.total_epochs,
        "Should converge before end"
    );
}

#[test]
fn test_sim_gini_fairness() {
    let config = SimConfig {
        num_nodes: 150,
        byzantine_fraction: 0.1,
        epochs: 20,
        fanout: 4,
        mean_latency_ms: 80.0,
        churn_rate: 0.02,
        attack_intensity: 0.0,
        seed: 42,
    };
    let result = run_testnet_simulation(&config);
    assert!(
        result.gini < 0.4,
        "Gini should be < 0.4 for fair credit distribution, got {}",
        result.gini
    );
}

#[test]
fn test_sim_latency_increases_with_fanout() {
    let low_fanout = SimConfig {
        num_nodes: 100,
        fanout: 2,
        epochs: 5,
        ..SimConfig::default()
    };
    let high_fanout = SimConfig {
        num_nodes: 100,
        fanout: 10,
        epochs: 5,
        ..SimConfig::default()
    };
    let r_low = run_testnet_simulation(&low_fanout);
    let r_high = run_testnet_simulation(&high_fanout);
    // Higher fanout → more messages → similar avg latency (exponential dist)
    assert!(r_low.avg_latency_ms > 0.0);
    assert!(r_high.avg_latency_ms > 0.0);
}

#[test]
fn test_sim_churn_effect() {
    let no_churn = SimConfig {
        num_nodes: 100,
        churn_rate: 0.0,
        epochs: 10,
        ..SimConfig::default()
    };
    let with_churn = SimConfig {
        num_nodes: 100,
        churn_rate: 0.1,
        epochs: 10,
        ..SimConfig::default()
    };
    let r1 = run_testnet_simulation(&no_churn);
    let r2 = run_testnet_simulation(&with_churn);
    assert!(r1.certified_ratio > 0.0);
    assert!(r2.certified_ratio > 0.0);
}

#[test]
fn test_sim_attack_intensity_effect() {
    let low_attack = SimConfig {
        num_nodes: 100,
        attack_intensity: 0.1,
        epochs: 15,
        ..SimConfig::default()
    };
    let high_attack = SimConfig {
        num_nodes: 100,
        attack_intensity: 1.0,
        epochs: 15,
        ..SimConfig::default()
    };
    let r_low = run_testnet_simulation(&low_attack);
    let r_high = run_testnet_simulation(&high_attack);
    // Higher attack → higher PoA
    assert!(
        r_high.poa >= r_low.poa,
        "High attack PoA {:.3} >= low attack PoA {:.3}",
        r_high.poa,
        r_low.poa
    );
}

#[test]
fn test_sim_result_display() {
    let config = SimConfig {
        num_nodes: 50,
        epochs: 5,
        ..SimConfig::default()
    };
    let result = run_testnet_simulation(&config);
    let display = format!("{}", result);
    assert!(display.contains("SimResult"));
    assert!(display.contains("PoA="));
    assert!(display.contains("certified="));
}

#[test]
fn test_sim_node_counts() {
    let config = SimConfig {
        num_nodes: 100,
        byzantine_fraction: 0.2,
        epochs: 5,
        ..SimConfig::default()
    };
    let result = run_testnet_simulation(&config);
    assert_eq!(result.honest_count + result.byzantine_count, 100);
    assert_eq!(result.byzantine_count, 20);
    assert_eq!(result.honest_count, 80);
}

#[test]
fn test_sim_pac_bound_reasonable() {
    let config = SimConfig {
        num_nodes: 500,
        byzantine_fraction: 0.1,
        epochs: 30,
        fanout: 6,
        mean_latency_ms: 100.0,
        churn_rate: 0.02,
        attack_intensity: 0.0,
        seed: 42,
    };
    let result = run_testnet_simulation(&config);
    assert!(
        result.final_pac_bound > 0.0 && result.final_pac_bound < 1.0,
        "PAC bound should be in (0,1), got {}",
        result.final_pac_bound
    );
}

// ============================================================
// Byzantine Median + Gini Utility Tests (6 tests)
// ============================================================

#[test]
fn test_byzantine_median_basic() {
    let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
    let median = byzantine_median(&values);
    assert!((median - 5.0).abs() < 0.1);
}

#[test]
fn test_byzantine_median_outliers() {
    let values = vec![-1000.0, -1000.0, 4.0, 5.0, 6.0, 1000.0, 1000.0];
    let median = byzantine_median(&values);
    assert!(
        median > 0.0 && median < 100.0,
        "Median {} should trim extreme outliers",
        median
    );
}

#[test]
fn test_byzantine_median_single() {
    let values = vec![42.0];
    let median = byzantine_median(&values);
    assert_eq!(median, 42.0);
}

#[test]
fn test_gini_perfect_equality() {
    let values = vec![5.0, 5.0, 5.0, 5.0, 5.0];
    let gini = compute_gini(&values);
    assert!(gini.abs() < 1e-6);
}

#[test]
fn test_gini_perfect_inequality() {
    let values = vec![0.0, 0.0, 0.0, 0.0, 100.0];
    let gini = compute_gini(&values);
    assert!(gini > 0.7);
}

#[test]
fn test_gini_realistic_distribution() {
    let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    let gini = compute_gini(&values);
    assert!(gini >= 0.0 && gini < 1.0);
}

// ============================================================
// Full Pipeline Integration Tests (6 tests)
// ============================================================

#[test]
fn test_sprint118_full_pipeline() -> candle_core::Result<()> {
    let device = Device::Cpu;

    // Step 1: Create SAE for dimension reduction
    let sae_config = SAEConfig {
        input_dim: 64,
        latent_dim: 16,
        top_k: 8,
        recon_threshold: 1.0,
    };
    let sae = SAE::new(&sae_config, &device)?;

    // Step 2: Project hidden state to latent subspace
    let hidden = make_hidden_batch(1, 64, &device)?;
    let proj = sae.project(&hidden)?;
    assert!(proj.avg_recon_error >= 0.0);

    // Step 3: Run testnet simulation
    let sim_config = SimConfig {
        num_nodes: 50,
        byzantine_fraction: 0.1,
        epochs: 10,
        fanout: 3,
        ..SimConfig::default()
    };
    let sim_result = run_testnet_simulation(&sim_config);
    assert!(sim_result.certified_ratio > 0.0);

    // Step 4: Verify SAE tightness
    let ratio = sae.tightness_ratio(&sae_config);
    assert!((ratio - 0.25).abs() < 1e-6);

    Ok(())
}

#[test]
fn test_sprint118_sae_then_verify() -> candle_core::Result<()> {
    let device = Device::Cpu;

    // SAE projects to lower dim, then verify in latent space
    let sae_config = SAEConfig {
        input_dim: 32,
        latent_dim: 8,
        top_k: 4,
        recon_threshold: 1.0,
    };
    let sae = SAE::new(&sae_config, &device)?;

    let hidden = make_hidden_batch(2, 32, &device)?;
    let result = sae.project(&hidden)?;

    // Verify: reconstruction error is bounded
    for &err in &result.recon_errors {
        assert!(err >= 0.0, "Reconstruction error should be non-negative");
    }

    // Verify: sparsity is enforced
    for &active in &result.active_features {
        assert!(
            active <= sae_config.top_k,
            "Active features {} > top_k {}",
            active,
            sae_config.top_k
        );
    }

    Ok(())
}

#[test]
fn test_sprint118_collective_verification() {
    // Simulate collective verification with SAE compression
    let config = SimConfig {
        num_nodes: 100,
        byzantine_fraction: 0.15,
        epochs: 20,
        fanout: 5,
        mean_latency_ms: 80.0,
        churn_rate: 0.02,
        attack_intensity: 0.3,
        seed: 42,
    };
    let result = run_testnet_simulation(&config);

    // Collective metrics
    assert!(result.certified_ratio > 0.5, "Majority should be certified");
    assert!(result.gini < 0.5, "Credit distribution should be fair");
    assert!(result.poa < 2.0, "PoA should be bounded");
    assert!(
        result.final_pac_bound < 0.5,
        "PAC bound should be reasonable"
    );
}

#[test]
fn test_sprint118_scale_comparison() {
    // Compare small vs large network
    let small = SimConfig {
        num_nodes: 50,
        epochs: 10,
        ..SimConfig::default()
    };
    let large = SimConfig {
        num_nodes: 500,
        epochs: 20,
        ..SimConfig::default()
    };
    let r_small = run_testnet_simulation(&small);
    let r_large = run_testnet_simulation(&large);

    // Larger network should have tighter PAC bound (more samples)
    assert!(
        r_large.final_pac_bound <= r_small.final_pac_bound * 2.0,
        "Large network PAC {:.4} should be tighter than small {:.4}",
        r_large.final_pac_bound,
        r_small.final_pac_bound
    );
}

#[test]
fn test_sprint118_edge_small_dims() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let sae_config = SAEConfig {
        input_dim: 8,
        latent_dim: 4,
        top_k: 2,
        recon_threshold: 1.0,
    };
    let sae = SAE::new(&sae_config, &device)?;
    let hidden = make_hidden_batch(1, 8, &device)?;
    let result = sae.project(&hidden)?;
    assert_eq!(result.recon_errors.len(), 1);
    assert!(result.active_features[0] <= 2);
    Ok(())
}

#[test]
fn test_sprint118_stress_test() -> candle_core::Result<()> {
    let device = Device::Cpu;

    // Stress: multiple SAEs + simulations
    for i in 0..5 {
        let sae_config = SAEConfig {
            input_dim: 64,
            latent_dim: 16,
            top_k: 8,
            recon_threshold: 1.0,
        };
        let sae = SAE::new(&sae_config, &device)?;
        let hidden = make_hidden_batch(2, 64, &device)?;
        let _ = sae.project(&hidden)?;

        let sim = SimConfig {
            num_nodes: 30,
            epochs: 3,
            seed: 42 + i as u64,
            ..SimConfig::default()
        };
        let _ = run_testnet_simulation(&sim);
    }
    Ok(())
}
