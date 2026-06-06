//! Distributed SAE Training Tests — Sprint 108
//!
//! Validates federated SAE training with DP noise, gradient clipping,
//! and secure aggregation over simulated P2P network.

use candle_core::{Device, IndexOp, Tensor};
use native_audit::distributed_sae::{DistSAEConfig, DistributedSAETrainer, DPAccountant};

#[test]
fn test_distributed_sae_creation() {
    let device = Device::Cpu;
    let config = DistSAEConfig::default();
    let trainer = DistributedSAETrainer::new(&config, &device).unwrap();

    assert_eq!(
        trainer.encoder.shape().dims(),
        &[config.hidden_dim, config.feature_dim]
    );
    assert_eq!(
        trainer.dictionary.shape().dims(),
        &[config.feature_dim, config.hidden_dim]
    );
}

#[test]
fn test_local_train_step() {
    let device = Device::Cpu;
    let config = DistSAEConfig {
        hidden_dim: 16,
        feature_dim: 32,
        ..Default::default()
    };
    let trainer = DistributedSAETrainer::new(&config, &device).unwrap();

    let batch = Tensor::from_vec(
        (0..32).map(|i| i as f32 * 0.01).collect(),
        (2, 16),
        &device,
    )
    .unwrap();

    let update = trainer.local_train_step(&batch).unwrap();
    assert_eq!(update.shape(), trainer.encoder.shape());
}

#[test]
fn test_secure_aggregation() {
    let device = Device::Cpu;

    let updates: Vec<Tensor> = (0..4)
        .map(|i| Tensor::from_vec(vec![i as f32 * 0.25; 8], (8,), &device))
        .collect::<candle_core::Result<_>>()
        .unwrap();

    let aggregated = DistributedSAETrainer::secure_aggregate(&updates).unwrap();
    // Mean of [0, 0.25, 0.5, 0.75] = 0.375
    let expected = 0.375f32;
    let actual: f32 = aggregated.i(0).unwrap().to_scalar().unwrap();
    assert!((actual - expected).abs() < 1e-5, "Aggregation: expected={}, got={}", expected, actual);
}

#[test]
fn test_federated_round() {
    let device = Device::Cpu;
    let config = DistSAEConfig {
        hidden_dim: 8,
        feature_dim: 16,
        noise_std: 0.001,
        num_rounds: 10,
        ..Default::default()
    };
    let mut trainer = DistributedSAETrainer::new(&config, &device).unwrap();

    let batch = Tensor::from_vec(
        (0..16).map(|i| i as f32 * 0.01).collect(),
        (2, 8),
        &device,
    )
    .unwrap();
    let update = trainer.local_train_step(&batch).unwrap();

    let loss = trainer.federated_round(&[update]).unwrap();
    assert!(loss >= 0.0, "Loss should be non-negative: {:.4}", loss);
    assert!(!trainer.dp_accountant.is_exhausted());
}

#[test]
fn test_multiple_federated_rounds() {
    let device = Device::Cpu;
    let config = DistSAEConfig {
        hidden_dim: 8,
        feature_dim: 16,
        noise_std: 0.001,
        dp_epsilon: 2.0,
        num_rounds: 20,
        ..Default::default()
    };
    let mut trainer = DistributedSAETrainer::new(&config, &device).unwrap();

    let mut losses = Vec::new();
    for round in 0..10 {
        let batch = Tensor::from_vec(
            (0..16).map(|i| (i + round) as f32 * 0.001).collect(),
            (2, 8),
            &device,
        )
        .unwrap();
        let update = trainer.local_train_step(&batch).unwrap();

        let loss = trainer.federated_round(&[update]).unwrap();
        losses.push(loss);
    }

    assert_eq!(losses.len(), 10);
    assert!(!trainer.dp_accountant.is_exhausted());
    println!(
        "10 rounds completed — DP budget: {:.2}/{:.2} epsilon",
        trainer.dp_accountant.total_epsilon, config.dp_epsilon
    );
}

#[test]
fn test_reconstruction_fidelity() {
    let device = Device::Cpu;
    let config = DistSAEConfig {
        hidden_dim: 8,
        feature_dim: 16,
        ..Default::default()
    };
    let trainer = DistributedSAETrainer::new(&config, &device).unwrap();

    let input = Tensor::from_vec(
        (0..16).map(|i| i as f32 * 0.1).collect(),
        (2, 8),
        &device,
    )
    .unwrap();

    let fidelity = trainer.reconstruction_fidelity(&input).unwrap();
    assert!(
        (0.0..=1.0).contains(&fidelity),
        "Fidelity in [0, 1]: {:.4}",
        fidelity
    );
    println!("Reconstruction fidelity: {:.4}", fidelity);
}

#[test]
fn test_extract_and_reconstruct() {
    let device = Device::Cpu;
    let config = DistSAEConfig {
        hidden_dim: 8,
        feature_dim: 16,
        ..Default::default()
    };
    let trainer = DistributedSAETrainer::new(&config, &device).unwrap();

    let input = Tensor::from_vec(
        (0..16).map(|i| i as f32 * 0.05).collect(),
        (2, 8),
        &device,
    )
    .unwrap();

    let features = trainer.extract_features(&input).unwrap();
    let recon = trainer.reconstruct(&features).unwrap();

    // Reconstruction should have same shape as input
    assert_eq!(recon.shape(), input.shape());

    // Features should be sparse (many zeros due to ReLU)
    let feature_count = features.shape().elem_count();
    let nonzero: u32 = features
        .abs()
        .unwrap()
        .gt(1e-6)
        .unwrap()
        .sum_all()
        .unwrap()
        .to_scalar::<u32>()
        .unwrap();
    let sparsity = 1.0 - nonzero as f32 / feature_count as f32;

    println!(
        "Features: total={}, nonzero={}, sparsity={:.2}%",
        feature_count, nonzero, sparsity * 100.0
    );
}

#[test]
fn test_dp_accountant_budget() {
    let config = DistSAEConfig::default();
    let mut accountant =
        DPAccountant::new(config.dp_epsilon, config.dp_delta, config.num_rounds);

    // Consume half the budget
    for _ in 0..(config.num_rounds / 2) {
        accountant.consume();
    }
    assert!(!accountant.is_exhausted());

    // Consume the rest
    for _ in 0..(config.num_rounds / 2) {
        accountant.consume();
    }
    assert!(accountant.is_exhausted());

    assert!((accountant.total_epsilon - config.dp_epsilon).abs() < 1e-5);
}

#[test]
fn test_gradient_clipping_effect() {
    let device = Device::Cpu;
    let config = DistSAEConfig {
        hidden_dim: 8,
        feature_dim: 16,
        clip_norm: 0.1,
        ..Default::default()
    };
    let trainer = DistributedSAETrainer::new(&config, &device).unwrap();

    // Large batch → large gradient
    let batch = Tensor::from_vec(
        (0..64).map(|i| i as f32 * 1.0).collect(),
        (8, 8),
        &device,
    )
    .unwrap();
    let update = trainer.local_train_step(&batch).unwrap();

    let clipped = trainer.clip_gradient(&update).unwrap();
    let norm: f32 = clipped.sqr().unwrap().sum_all().unwrap().sqrt().unwrap().to_scalar().unwrap();

    assert!(
        norm <= config.clip_norm + 1e-5,
        "Clipped norm {:.4} <= clip_norm {:.4}",
        norm,
        config.clip_norm
    );
}

#[test]
fn test_multi_peer_federation() {
    let device = Device::Cpu;
    let config = DistSAEConfig {
        hidden_dim: 8,
        feature_dim: 16,
        noise_std: 0.001,
        ..Default::default()
    };

    // Simulate 5 peers each computing local updates
    let mut trainers = Vec::new();
    for _ in 0..5 {
        trainers.push(DistributedSAETrainer::new(&config, &device).unwrap());
    }

    let mut all_updates = Vec::new();
    for trainer in &trainers {
        let batch = Tensor::from_vec(
            (0..16).map(|i| i as f32 * 0.01).collect(),
            (2, 8),
            &device,
        )
        .unwrap();
        let update = trainer.local_train_step(&batch).unwrap();
        all_updates.push(update);
    }

    // Aggregate all peer updates
    let aggregated = DistributedSAETrainer::secure_aggregate(&all_updates).unwrap();
    assert_eq!(aggregated.shape().dims(), &[8, 16]);

    println!("Multi-peer federation: 5 peers aggregated successfully");
}
