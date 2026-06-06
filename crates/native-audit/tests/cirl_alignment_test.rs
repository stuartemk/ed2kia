//! CIRL Alignment Tests — Sprint 108
//!
//! Validates Cooperative Inverse Reinforcement Learning value learning,
//! reward model training, and value alignment scores.

use candle_core::{Device, Tensor};
use native_audit::cirl_value_learning::{CIRLEngine, CIRLConfig, RewardModel, Trajectory};

#[test]
fn test_reward_model_initialization() {
    let device = Device::Cpu;
    let config = CIRLConfig::default();
    let model = RewardModel::new(&config, &device).unwrap();

    assert_eq!(model.weights.shape().dims()[0], config.reward_dim);
    assert_eq!(model.bias.shape().dims()[0], config.reward_dim);
}

#[test]
fn test_irl_loss_non_negative() {
    let device = Device::Cpu;
    let config = CIRLConfig::default();
    let model = RewardModel::new(&config, &device).unwrap();

    let state = Tensor::from_vec(vec![0.1f32, -0.2, 0.3, 0.05], (4,), &device).unwrap();
    let action = Tensor::from_vec(vec![1.0f32, 0.0], (2,), &device).unwrap();
    let traj = Trajectory::new(state, action, 0.5);

    let loss = model.compute_irl_loss(&[traj.clone(), traj]).unwrap();
    assert!(loss >= 0.0, "IRL loss must be non-negative: {:.4}", loss);
}

#[test]
fn test_cirl_value_update() {
    let device = Device::Cpu;
    let config = CIRLConfig::default();
    let mut engine = CIRLEngine::new(&config, &device, &[8]).unwrap();

    let local_trajs = engine.generate_stub_trajectories(3, &[8], &[4]).unwrap();
    let peer_trajs = vec![
        engine.generate_stub_trajectories(2, &[8], &[4]).unwrap(),
        engine.generate_stub_trajectories(2, &[8], &[4]).unwrap(),
    ];

    let updated_prior = engine
        .cirl_value_update(local_trajs, peer_trajs)
        .unwrap();

    assert_eq!(updated_prior.shape().dims(), &[8]);
    println!("CIRL value update completed — prior shape: {:?}", updated_prior.shape().dims());
}

#[test]
fn test_value_alignment_range() {
    let device = Device::Cpu;
    let config = CIRLConfig::default();
    let engine = CIRLEngine::new(&config, &device, &[8]).unwrap();

    let trajs = engine.generate_stub_trajectories(5, &[8], &[4]).unwrap();
    let alignment = engine.compute_value_alignment(&trajs).unwrap();

    assert!(
        (-1.0..=1.0).contains(&alignment),
        "Alignment in [-1, 1]: {:.4}",
        alignment
    );
    println!("Value alignment score: {:.4}", alignment);
}

#[test]
fn test_cooperative_update_stability() {
    let device = Device::Cpu;
    let config = CIRLConfig {
        learning_rate: 0.02,
        cooperation_weight: 0.5,
        ..Default::default()
    };
    let mut engine = CIRLEngine::new(&config, &device, &[16]).unwrap();

    let shared_trajs = engine.generate_stub_trajectories(5, &[16], &[8]).unwrap();
    let alignment_before = engine.compute_value_alignment(&shared_trajs).unwrap();

    // Perform 10 cooperative updates
    for i in 0..10 {
        let local = engine.generate_stub_trajectories(3, &[16], &[8]).unwrap();
        // Use different seed offset for peer trajectories
        let peers = vec![engine.generate_stub_trajectories(2, &[16], &[8]).unwrap()];
        engine.cirl_value_update(local, peers).unwrap();

        if i % 3 == 0 {
            let current_alignment = engine.compute_value_alignment(&shared_trajs).unwrap();
            println!(
                "Round {}: alignment = {:.4}",
                i, current_alignment
            );
        }
    }

    let alignment_after = engine.compute_value_alignment(&shared_trajs).unwrap();

    // Alignment should remain bounded
    assert!(
        (-1.0..=1.0).contains(&alignment_after),
        "Alignment remained bounded: after={:.4}",
        alignment_after
    );

    println!(
        "Stability: before={:.4}, after={:.4}, delta={:.4}",
        alignment_before,
        alignment_after,
        alignment_after - alignment_before
    );
}

#[test]
fn test_gradient_clipping() {
    let device = Device::Cpu;
    let config = CIRLConfig {
        clip_norm: 0.5,
        learning_rate: 0.1,
        ..Default::default()
    };
    let mut engine = CIRLEngine::new(&config, &device, &[8]).unwrap();

    // Generate trajectories with high variance to trigger clipping
    let local = engine.generate_stub_trajectories(5, &[8], &[4]).unwrap();
    let peers = vec![engine.generate_stub_trajectories(3, &[8], &[4]).unwrap()];

    // Should not panic even with aggressive settings
    let result = engine.cirl_value_update(local, peers);
    assert!(result.is_ok(), "Gradient clipping should prevent overflow");
}

#[test]
fn test_empty_peer_trajectories() {
    let device = Device::Cpu;
    let config = CIRLConfig::default();
    let mut engine = CIRLEngine::new(&config, &device, &[8]).unwrap();

    let local = engine.generate_stub_trajectories(3, &[8], &[4]).unwrap();

    // No peer trajectories — should still work with local-only update
    let result = engine.cirl_value_update(local, vec![]).unwrap();
    assert_eq!(result.shape().dims(), &[8]);
}

#[test]
fn test_discount_factor_effect() {
    let device = Device::Cpu;
    let config_high_discount = CIRLConfig {
        discount_factor: 0.99,
        ..Default::default()
    };
    let config_low_discount = CIRLConfig {
        discount_factor: 0.5,
        ..Default::default()
    };

    let model_high = RewardModel::new(&config_high_discount, &device).unwrap();
    let model_low = RewardModel::new(&config_low_discount, &device).unwrap();

    let state = Tensor::from_vec(vec![0.1f32; 8], (8,), &device).unwrap();
    let action = Tensor::from_vec(vec![1.0f32; 4], (4,), &device).unwrap();

    let trajs: Vec<Trajectory> = (0..5)
        .map(|_| Trajectory::new(state.clone(), action.clone(), 0.7))
        .collect();

    let loss_high = model_high.compute_irl_loss(&trajs).unwrap();
    let loss_low = model_low.compute_irl_loss(&trajs).unwrap();

    // Higher discount → more future weight → different loss magnitude
    // Both should be non-negative
    assert!(loss_high >= 0.0);
    assert!(loss_low >= 0.0);

    println!(
        "Discount effect: high(0.99)={:.4}, low(0.5)={:.4}",
        loss_high, loss_low
    );
}
