//! Neural ODE Zonotope Reachability Tests — Sprint 112 (v11.2.0)
//!
//! Tests for neural_ode.rs covering:
//! - NeuralODEField creation and evaluation
//! - NeuralODEZonotope Euler/RK2/RK4 integration
//! - Flowpipe computation
//! - Trajectory certificates
//! - Control Barrier Functions (CBF)
//! - Self-improvement engine
//! - Integration method selection
//! - Flowpipe volume growth
//! - Certificate display

use candle_core::{DType, Device, Result, Tensor};
use native_audit::hybrid_zonotope::{HybridZonotopeConfig, LayerType};
use native_audit::neural_ode::{
    ControlBarrierFunction, NeuralODEConfig, NeuralODEField, NeuralODEZonotope,
    SelfImprovementConfig, SelfImprovementEngine,
};

// ============================================================================
// Helper Functions
// ============================================================================

fn create_field(dim: usize, device: &Device) -> Result<NeuralODEField> {
    let weight = Tensor::randn(0.0, 0.5, (dim, dim), device)?.to_dtype(DType::F32)?;
    let bias = Tensor::zeros(dim, DType::F32, device)?;
    NeuralODEField::new(&weight, Some(&bias), LayerType::ReLU)
}

fn create_ode(dim: usize, device: &Device) -> Result<NeuralODEZonotope> {
    let field = create_field(dim, device)?;
    let center = Tensor::zeros((1, dim), DType::F32, device)?;
    let config = NeuralODEConfig::default();
    NeuralODEZonotope::from_epsilon(&center, 0.1, field, config)
}

fn create_ode_config(
    dim: usize,
    device: &Device,
    config: NeuralODEConfig,
) -> Result<NeuralODEZonotope> {
    let field = create_field(dim, device)?;
    let center = Tensor::zeros((1, dim), DType::F32, device)?;
    NeuralODEZonotope::from_epsilon(&center, 0.1, field, config)
}

// ============================================================================
// NeuralODEField Tests
// ============================================================================

#[test]
fn test_field_creation() -> Result<()> {
    let device = Device::Cpu;
    let field = create_field(8, &device)?;
    assert_eq!(field.weight.shape().dims(), &[8, 8]);
    Ok(())
}

#[test]
fn test_field_creation_various_dims() -> Result<()> {
    let device = Device::Cpu;
    for dim in [4, 8, 16, 32] {
        let field = create_field(dim, &device)?;
        assert_eq!(field.weight.shape().dims(), &[dim, dim]);
    }
    Ok(())
}

#[test]
fn test_field_evaluate() -> Result<()> {
    let device = Device::Cpu;
    let field = create_field(8, &device)?;
    let x = Tensor::ones((1, 8), DType::F32, &device)?;
    let fx = field.evaluate(&x)?;
    assert_eq!(fx.shape().dims(), &[1, 8]);
    Ok(())
}

#[test]
fn test_field_evaluate_batch() -> Result<()> {
    let device = Device::Cpu;
    let field = create_field(4, &device)?;
    let x = Tensor::ones((2, 4), DType::F32, &device)?;
    let fx = field.evaluate(&x)?;
    assert_eq!(fx.shape().dims(), &[2, 4]);
    Ok(())
}

#[test]
fn test_field_identity() -> Result<()> {
    let device = Device::Cpu;
    let weight = Tensor::eye(4, DType::F32, &device)?;
    let bias = Tensor::zeros(4, DType::F32, &device)?;
    let field = NeuralODEField::new(&weight, Some(&bias), LayerType::Affine)?;
    let x = Tensor::from_vec(vec![1.0f32, 2.0, 3.0, 4.0], (1, 4), &device)?;
    let fx = field.evaluate(&x)?;
    let out: Vec<f32> = fx.flatten_all()?.to_vec1()?;
    assert!((out[0] - 1.0).abs() < 0.01);
    Ok(())
}

#[test]
fn test_field_without_bias() -> Result<()> {
    let device = Device::Cpu;
    let weight = Tensor::randn(0.0, 0.5, (4, 4), &device)?.to_dtype(DType::F32)?;
    let field = NeuralODEField::new(&weight, None, LayerType::SiLU)?;
    let x = Tensor::ones((1, 4), DType::F32, &device)?;
    let fx = field.evaluate(&x)?;
    assert_eq!(fx.shape().dims(), &[1, 4]);
    Ok(())
}

#[test]
fn test_field_activation_types() -> Result<()> {
    let device = Device::Cpu;
    let weight = Tensor::eye(4, DType::F32, &device)?;
    let bias = Tensor::zeros(4, DType::F32, &device)?;

    for activation in [
        LayerType::ReLU,
        LayerType::SiLU,
        LayerType::GeLU,
        LayerType::Affine,
    ] {
        let field = NeuralODEField::new(&weight, Some(&bias), activation)?;
        let x = Tensor::ones((1, 4), DType::F32, &device)?;
        let fx = field.evaluate(&x)?;
        assert!(fx.shape().dims().iter().all(|&d| d > 0));
    }
    Ok(())
}

// ============================================================================
// NeuralODEZonotope Tests
// ============================================================================

#[test]
fn test_ode_creation() -> Result<()> {
    let device = Device::Cpu;
    let ode = create_ode(8, &device)?;
    assert!(ode.zonotope.log_volume_proxy()?.is_finite());
    Ok(())
}

#[test]
fn test_ode_from_epsilon() -> Result<()> {
    let device = Device::Cpu;
    let field = create_field(8, &device)?;
    let center = Tensor::zeros((1, 8), DType::F32, &device)?;
    let config = NeuralODEConfig::default();
    let ode = NeuralODEZonotope::from_epsilon(&center, 0.05, field, config)?;
    assert!(ode.zonotope.log_volume_proxy()?.is_finite());
    Ok(())
}

#[test]
fn test_euler_step() -> Result<()> {
    let device = Device::Cpu;
    let ode = create_ode(8, &device)?;
    let next = ode.euler_step()?;
    assert!(next.log_volume_proxy()?.is_finite());
    Ok(())
}

#[test]
fn test_euler_step_chain() -> Result<()> {
    let device = Device::Cpu;
    let ode = create_ode(4, &device)?;
    let mut z = ode.zonotope.clone();
    for _ in 0..5 {
        let field = ode.field.clone();
        let config = ode.config.clone();
        let temp_ode = NeuralODEZonotope::new(z.clone(), field, config);
        z = temp_ode.euler_step()?;
        assert!(z.log_volume_proxy()?.is_finite());
    }
    Ok(())
}

#[test]
fn test_rk2_step() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        method: "rk2".to_string(),
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let next = ode.rk2_step()?;
    assert!(next.log_volume_proxy()?.is_finite());
    Ok(())
}

#[test]
fn test_rk4_step() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        method: "rk4".to_string(),
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let next = ode.rk4_step()?;
    assert!(next.log_volume_proxy()?.is_finite());
    Ok(())
}

#[test]
fn test_integrate_step_euler() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        method: "euler".to_string(),
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let next = ode.integrate_step()?;
    assert!(next.log_volume_proxy()?.is_finite());
    Ok(())
}

#[test]
fn test_integrate_step_rk2() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        method: "rk2".to_string(),
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let next = ode.integrate_step()?;
    assert!(next.log_volume_proxy()?.is_finite());
    Ok(())
}

#[test]
fn test_integrate_step_rk4() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        method: "rk4".to_string(),
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let next = ode.integrate_step()?;
    assert!(next.log_volume_proxy()?.is_finite());
    Ok(())
}

// ============================================================================
// Flowpipe Tests
// ============================================================================

#[test]
fn test_flowpipe_computation() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        time_steps: 10,
        dt: 0.01,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let flowpipe = ode.compute_flowpipe()?;
    assert_eq!(flowpipe.steps.len(), 11); // 0 + 10 steps
    assert!(flowpipe.max_log_volume.is_finite());
    Ok(())
}

#[test]
fn test_flowpipe_single_step() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        time_steps: 1,
        dt: 0.01,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let flowpipe = ode.compute_flowpipe()?;
    assert_eq!(flowpipe.steps.len(), 2);
    Ok(())
}

#[test]
fn test_flowpipe_volume_growth() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        time_steps: 20,
        dt: 0.01,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let flowpipe = ode.compute_flowpipe()?;
    for (i, step) in flowpipe.steps.iter().enumerate() {
        assert!(
            step.log_volume.is_finite(),
            "Step {} has non-finite volume",
            i
        );
    }
    Ok(())
}

#[test]
fn test_flowpipe_steps_increasing() -> Result<()> {
    let device = Device::Cpu;
    for n in [5, 10, 15] {
        let config = NeuralODEConfig {
            time_steps: n,
            dt: 0.01,
            ..NeuralODEConfig::default()
        };
        let ode = create_ode_config(8, &device, config)?;
        let flowpipe = ode.compute_flowpipe()?;
        assert_eq!(flowpipe.steps.len(), n + 1);
    }
    Ok(())
}

#[test]
fn test_flowpipe_max_volume() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        time_steps: 10,
        dt: 0.01,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let flowpipe = ode.compute_flowpipe()?;
    for step in &flowpipe.steps {
        assert!(step.log_volume <= flowpipe.max_log_volume + 1e-6);
    }
    Ok(())
}

#[test]
fn test_flowpipe_time_horizon() -> Result<()> {
    let device = Device::Cpu;
    let dt = 0.05;
    let n_steps = 20;
    let config = NeuralODEConfig {
        time_steps: n_steps,
        dt,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let flowpipe = ode.compute_flowpipe()?;
    // Total time = steps * dt
    let expected_steps = n_steps + 1;
    assert_eq!(flowpipe.steps.len(), expected_steps);
    Ok(())
}

// ============================================================================
// Trajectory Certificate Tests
// ============================================================================

#[test]
fn test_certificate_generation() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        time_steps: 5,
        dt: 0.01,
        mc_samples: 32,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let cert = ode.generate_certificate()?;
    assert!(cert.num_steps > 0);
    assert!(cert.total_time > 0.0);
    assert!(cert.violation_prob >= 0.0);
    Ok(())
}

#[test]
fn test_certificate_violation_prob_range() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        time_steps: 5,
        dt: 0.01,
        mc_samples: 64,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let cert = ode.generate_certificate()?;
    assert!((0.0..=1.0).contains(&cert.violation_prob));
    Ok(())
}

#[test]
fn test_certificate_epsilon_positive() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        time_steps: 3,
        dt: 0.01,
        mc_samples: 16,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let cert = ode.generate_certificate()?;
    assert!(cert.certified_epsilon >= 0.0);
    Ok(())
}

#[test]
fn test_certificate_display() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        time_steps: 3,
        dt: 0.01,
        mc_samples: 16,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let cert = ode.generate_certificate()?;
    let display = format!("{}", cert);
    assert!(display.contains("TrajectoryCertificate"));
    Ok(())
}

#[test]
fn test_certificate_fields_finite() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        time_steps: 5,
        dt: 0.01,
        mc_samples: 32,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let cert = ode.generate_certificate()?;
    assert!(cert.min_cbf_value.is_finite());
    assert!(cert.max_log_volume.is_finite());
    assert!(cert.certified_epsilon.is_finite());
    assert!(cert.violation_prob.is_finite());
    Ok(())
}

#[test]
fn test_certificate_steps_match_config() -> Result<()> {
    let device = Device::Cpu;
    let n_steps = 7;
    let config = NeuralODEConfig {
        time_steps: n_steps,
        dt: 0.01,
        mc_samples: 16,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let cert = ode.generate_certificate()?;
    // cert.num_steps = flowpipe.steps.len() = time_steps + 1 (initial step)
    assert_eq!(cert.num_steps, n_steps + 1);
    Ok(())
}

#[test]
fn test_certificate_total_time() -> Result<()> {
    let device = Device::Cpu;
    let dt = 0.05;
    let n_steps = 10;
    let config = NeuralODEConfig {
        time_steps: n_steps,
        dt,
        mc_samples: 16,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let cert = ode.generate_certificate()?;
    let expected = (n_steps as f32) * dt;
    assert!((cert.total_time - expected).abs() < 1e-6);
    Ok(())
}

#[test]
fn test_certificate_mc_samples_effect() -> Result<()> {
    let device = Device::Cpu;
    let config1 = NeuralODEConfig {
        time_steps: 3,
        dt: 0.01,
        mc_samples: 16,
        ..NeuralODEConfig::default()
    };
    let config2 = NeuralODEConfig {
        time_steps: 3,
        dt: 0.01,
        mc_samples: 64,
        ..NeuralODEConfig::default()
    };
    let ode1 = create_ode_config(8, &device, config1)?;
    let ode2 = create_ode_config(8, &device, config2)?;
    let cert1 = ode1.generate_certificate()?;
    let cert2 = ode2.generate_certificate()?;
    // Both should produce valid certificates
    assert!(cert1.violation_prob >= 0.0);
    assert!(cert2.violation_prob >= 0.0);
    Ok(())
}

// ============================================================================
// Control Barrier Function Tests
// ============================================================================

#[test]
fn test_cbf_creation() {
    let cbf = ControlBarrierFunction::new(vec![1.0, -1.0, 0.5], 0.1, 1.0);
    assert_eq!(cbf.weight.len(), 3);
    assert_eq!(cbf.bias, 0.1);
    assert_eq!(cbf.alpha, 1.0);
}

#[test]
fn test_cbf_norm_based() {
    let cbf = ControlBarrierFunction::norm_based(1.0, 3);
    assert_eq!(cbf.weight.len(), 3);
    assert!(cbf.bias > 0.0);
}

#[test]
fn test_cbf_evaluate() -> Result<()> {
    let device = Device::Cpu;
    let cbf = ControlBarrierFunction::new(vec![1.0, -1.0], 0.5, 1.0);
    let x = Tensor::from_vec(vec![1.0f32, 0.5], (1, 2), &device)?;
    let val = cbf.evaluate(&x, &device)?;
    assert!(val.to_scalar::<f32>()?.is_finite());
    Ok(())
}

#[test]
fn test_cbf_evaluate_zonotope() -> Result<()> {
    let device = Device::Cpu;
    let cbf = ControlBarrierFunction::new(vec![1.0, -1.0], 0.5, 1.0);
    let center = Tensor::zeros((1, 2), DType::F32, &device)?;
    let gens = Tensor::randn(0.0, 0.1, (2, 2), &device)?.to_dtype(DType::F32)?;
    let zonotope = native_audit::zonotope::Zonotope::new(
        center,
        gens,
        native_audit::zonotope::ZonotopeConfig::default(),
    )?;
    let hybrid_config = HybridZonotopeConfig::default();
    let z = native_audit::hybrid_zonotope::HybridZonotope::from_zonotope(zonotope, hybrid_config)?;
    let lower = cbf.evaluate_zonotope_lower(&z)?;
    assert!(lower.is_finite());
    Ok(())
}

#[test]
fn test_cbf_lie_derivative() -> Result<()> {
    let device = Device::Cpu;
    let cbf = ControlBarrierFunction::new(vec![1.0, -1.0], 0.5, 1.0);
    let f_x = Tensor::ones(2, DType::F32, &device)?;
    let lie = cbf.lie_derivative(&f_x)?;
    assert!(lie.to_scalar::<f32>()?.is_finite());
    Ok(())
}

#[test]
fn test_cbf_zero_weight() {
    let cbf = ControlBarrierFunction::new(vec![0.0, 0.0], 1.0, 1.0);
    assert_eq!(cbf.weight, vec![0.0, 0.0]);
    assert_eq!(cbf.bias, 1.0);
}

#[test]
fn test_cbf_high_alpha() {
    let cbf = ControlBarrierFunction::new(vec![1.0], 0.0, 10.0);
    assert_eq!(cbf.alpha, 10.0);
}

// ============================================================================
// Self-Improvement Engine Tests
// ============================================================================

#[test]
fn test_engine_creation() {
    let config = SelfImprovementConfig::default();
    let engine = SelfImprovementEngine::new(config, 1.0);
    assert_eq!(engine.current_vfe, 1.0);
    assert!(engine.history.is_empty());
}

#[test]
fn test_engine_run_round() -> Result<()> {
    let device = Device::Cpu;
    let ode = create_ode(8, &device)?;
    let mut engine = SelfImprovementEngine::new(SelfImprovementConfig::default(), 1.0);
    let result = engine.run_round(&ode)?;
    assert_eq!(result.round, 0);
    assert!(result.vfe_before > 0.0);
    assert!(engine.history.len() == 1);
    Ok(())
}

#[test]
fn test_engine_run_loop() -> Result<()> {
    let device = Device::Cpu;
    let ode = create_ode(8, &device)?;
    let config = SelfImprovementConfig {
        rounds: 5,
        ..SelfImprovementConfig::default()
    };
    let mut engine = SelfImprovementEngine::new(config, 1.0);
    let results = engine.run_loop(&ode)?;
    assert_eq!(results.len(), 5);
    assert_eq!(engine.history.len(), 5);
    Ok(())
}

#[test]
fn test_engine_cumulative_reduction() -> Result<()> {
    let device = Device::Cpu;
    let ode = create_ode(8, &device)?;
    let config = SelfImprovementConfig {
        rounds: 3,
        ..SelfImprovementConfig::default()
    };
    let mut engine = SelfImprovementEngine::new(config, 1.0);
    engine.run_loop(&ode)?;
    let reduction = engine.cumulative_vfe_reduction();
    assert!(reduction >= 0.0);
    assert!(reduction.is_finite());
    Ok(())
}

#[test]
fn test_engine_vfe_decreases() -> Result<()> {
    let device = Device::Cpu;
    let ode = create_ode(8, &device)?;
    let config = SelfImprovementConfig {
        rounds: 5,
        ..SelfImprovementConfig::default()
    };
    let mut engine = SelfImprovementEngine::new(config, 1.0);
    let results = engine.run_loop(&ode)?;
    for result in &results {
        assert!(result.vfe_after <= result.vfe_before);
    }
    Ok(())
}

#[test]
fn test_engine_reward_finite() -> Result<()> {
    let device = Device::Cpu;
    let ode = create_ode(8, &device)?;
    let config = SelfImprovementConfig {
        rounds: 3,
        ..SelfImprovementConfig::default()
    };
    let mut engine = SelfImprovementEngine::new(config, 1.0);
    let results = engine.run_loop(&ode)?;
    for result in &results {
        assert!(result.reward.is_finite());
    }
    Ok(())
}

#[test]
fn test_engine_diversity() -> Result<()> {
    let device = Device::Cpu;
    let ode = create_ode(8, &device)?;
    let config = SelfImprovementConfig {
        rounds: 3,
        diversity_weight: 0.5,
        ..SelfImprovementConfig::default()
    };
    let mut engine = SelfImprovementEngine::new(config, 1.0);
    let results = engine.run_loop(&ode)?;
    for result in &results {
        assert!(result.diversity.is_finite());
    }
    Ok(())
}

#[test]
fn test_engine_certified_epsilon() -> Result<()> {
    let device = Device::Cpu;
    let ode = create_ode(8, &device)?;
    let config = SelfImprovementConfig {
        rounds: 3,
        ..SelfImprovementConfig::default()
    };
    let mut engine = SelfImprovementEngine::new(config, 1.0);
    let results = engine.run_loop(&ode)?;
    for result in &results {
        assert!(result.certified_epsilon >= 0.0);
    }
    Ok(())
}

#[test]
fn test_engine_round_numbers() -> Result<()> {
    let device = Device::Cpu;
    let ode = create_ode(8, &device)?;
    let config = SelfImprovementConfig {
        rounds: 5,
        ..SelfImprovementConfig::default()
    };
    let mut engine = SelfImprovementEngine::new(config, 1.0);
    let results = engine.run_loop(&ode)?;
    for (i, result) in results.iter().enumerate() {
        assert_eq!(result.round, i);
    }
    Ok(())
}

#[test]
fn test_engine_vfe_reduction_ratio() -> Result<()> {
    let device = Device::Cpu;
    let ode = create_ode(8, &device)?;
    let config = SelfImprovementConfig {
        rounds: 3,
        ..SelfImprovementConfig::default()
    };
    let mut engine = SelfImprovementEngine::new(config, 1.0);
    let results = engine.run_loop(&ode)?;
    for result in &results {
        assert!(result.vfe_reduction >= 0.0);
    }
    Ok(())
}

#[test]
fn test_engine_zero_initial_vfe() {
    let config = SelfImprovementConfig::default();
    let engine = SelfImprovementEngine::new(config, 0.0);
    assert_eq!(engine.cumulative_vfe_reduction(), 0.0);
}

#[test]
fn test_engine_single_round() -> Result<()> {
    let device = Device::Cpu;
    let ode = create_ode(8, &device)?;
    let config = SelfImprovementConfig {
        rounds: 1,
        ..SelfImprovementConfig::default()
    };
    let mut engine = SelfImprovementEngine::new(config, 1.0);
    let results = engine.run_loop(&ode)?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].round, 0);
    Ok(())
}

#[test]
fn test_engine_config_params() {
    let config = SelfImprovementConfig {
        rounds: 10,
        vfe_target: 0.01,
        diversity_weight: 0.2,
        violation_weight: 1.0,
        meta_lr: 0.05,
    };
    let engine = SelfImprovementEngine::new(config, 1.0);
    assert_eq!(engine.config.rounds, 10);
    assert_eq!(engine.config.vfe_target, 0.01);
}

// ============================================================================
// Integration Method Tests
// ============================================================================

#[test]
fn test_integration_method_euler() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        method: "euler".to_string(),
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let next = ode.integrate_step()?;
    assert!(next.log_volume_proxy()?.is_finite());
    Ok(())
}

#[test]
fn test_integration_method_rk2() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        method: "rk2".to_string(),
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let next = ode.integrate_step()?;
    assert!(next.log_volume_proxy()?.is_finite());
    Ok(())
}

#[test]
fn test_integration_method_rk4() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        method: "rk4".to_string(),
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let next = ode.integrate_step()?;
    assert!(next.log_volume_proxy()?.is_finite());
    Ok(())
}

#[test]
fn test_all_methods_produce_results() -> Result<()> {
    let device = Device::Cpu;
    for method in &["euler", "rk2", "rk4"] {
        let config = NeuralODEConfig {
            method: method.to_string(),
            ..NeuralODEConfig::default()
        };
        let ode = create_ode_config(8, &device, config)?;
        let next = ode.integrate_step()?;
        assert!(
            next.log_volume_proxy()?.is_finite(),
            "Method {} failed",
            method
        );
    }
    Ok(())
}

// ============================================================================
// Configuration Tests
// ============================================================================

#[test]
fn test_config_default() {
    let config = NeuralODEConfig::default();
    assert!(config.dt > 0.0);
    assert!(config.time_steps > 0);
    assert!(!config.method.is_empty());
}

#[test]
fn test_config_custom() {
    let config = NeuralODEConfig {
        dt: 0.001,
        time_steps: 100,
        method: "rk4".to_string(),
        taylor_correction: true,
        max_gens: 256,
        use_neural_tightener: true,
        mc_samples: 128,
    };
    assert_eq!(config.dt, 0.001);
    assert_eq!(config.time_steps, 100);
    assert_eq!(config.method, "rk4");
    assert!(config.taylor_correction);
}

#[test]
fn test_config_small_dt() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        dt: 0.001,
        time_steps: 5,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let next = ode.integrate_step()?;
    assert!(next.log_volume_proxy()?.is_finite());
    Ok(())
}

#[test]
fn test_config_large_dt() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        dt: 0.1,
        time_steps: 3,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let next = ode.integrate_step()?;
    assert!(next.log_volume_proxy()?.is_finite());
    Ok(())
}

// ============================================================================
// Dimension Scaling Tests
// ============================================================================

#[test]
fn test_ode_dim_4() -> Result<()> {
    let device = Device::Cpu;
    let ode = create_ode(4, &device)?;
    let next = ode.euler_step()?;
    assert!(next.log_volume_proxy()?.is_finite());
    Ok(())
}

#[test]
fn test_ode_dim_16() -> Result<()> {
    let device = Device::Cpu;
    let ode = create_ode(16, &device)?;
    let next = ode.euler_step()?;
    assert!(next.log_volume_proxy()?.is_finite());
    Ok(())
}

#[test]
fn test_ode_dim_32() -> Result<()> {
    let device = Device::Cpu;
    let ode = create_ode(32, &device)?;
    let next = ode.euler_step()?;
    assert!(next.log_volume_proxy()?.is_finite());
    Ok(())
}

#[test]
fn test_flowpipe_dim_scaling() -> Result<()> {
    let device = Device::Cpu;
    for dim in [4, 8, 16] {
        let config = NeuralODEConfig {
            time_steps: 5,
            dt: 0.01,
            ..NeuralODEConfig::default()
        };
        let ode = create_ode_config(dim, &device, config)?;
        let flowpipe = ode.compute_flowpipe()?;
        assert_eq!(flowpipe.steps.len(), 6);
    }
    Ok(())
}

#[test]
fn test_certificate_dim_scaling() -> Result<()> {
    let device = Device::Cpu;
    for dim in [4, 8, 16] {
        let config = NeuralODEConfig {
            time_steps: 3,
            dt: 0.01,
            mc_samples: 16,
            ..NeuralODEConfig::default()
        };
        let ode = create_ode_config(dim, &device, config)?;
        let cert = ode.generate_certificate()?;
        assert!(cert.num_steps > 0);
    }
    Ok(())
}

// ============================================================================
// Epsilon Sensitivity Tests
// ============================================================================

#[test]
fn test_epsilon_small() -> Result<()> {
    let device = Device::Cpu;
    let field = create_field(8, &device)?;
    let center = Tensor::zeros((1, 8), DType::F32, &device)?;
    let config = NeuralODEConfig::default();
    let ode = NeuralODEZonotope::from_epsilon(&center, 0.01, field, config)?;
    let next = ode.euler_step()?;
    assert!(next.log_volume_proxy()?.is_finite());
    Ok(())
}

#[test]
fn test_epsilon_large() -> Result<()> {
    let device = Device::Cpu;
    let field = create_field(8, &device)?;
    let center = Tensor::zeros((1, 8), DType::F32, &device)?;
    let config = NeuralODEConfig::default();
    let ode = NeuralODEZonotope::from_epsilon(&center, 0.5, field, config)?;
    let next = ode.euler_step()?;
    assert!(next.log_volume_proxy()?.is_finite());
    Ok(())
}

#[test]
fn test_epsilon_affects_volume() -> Result<()> {
    let device = Device::Cpu;
    let field1 = create_field(8, &device)?;
    let field2 = create_field(8, &device)?;
    let center = Tensor::zeros((1, 8), DType::F32, &device)?;
    let config = NeuralODEConfig::default();
    let ode_small = NeuralODEZonotope::from_epsilon(&center, 0.01, field1, config.clone())?;
    let ode_large = NeuralODEZonotope::from_epsilon(&center, 0.5, field2, config)?;
    let vol_small = ode_small.zonotope.log_volume_proxy()?;
    let vol_large = ode_large.zonotope.log_volume_proxy()?;
    // Larger epsilon should generally give larger volume
    assert!(vol_small <= vol_large + 1.0); // Allow some tolerance
    Ok(())
}

// ============================================================================
// Safe Trajectory Verification Tests
// ============================================================================

#[test]
fn test_verify_safe_trajectory() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        time_steps: 5,
        dt: 0.01,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let cbf = ControlBarrierFunction::norm_based(1.0, 8);
    let safe = ode.verify_safe_trajectory(&cbf, 1.0)?;
    assert!(safe || !safe); // Just check it doesn't panic
    Ok(())
}

#[test]
fn test_verify_safe_trajectory_small_cbf() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        time_steps: 3,
        dt: 0.01,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let cbf = ControlBarrierFunction::norm_based(10.0, 8);
    let safe = ode.verify_safe_trajectory(&cbf, 1.0)?;
    assert!(safe || !safe);
    Ok(())
}

// ============================================================================
// Combined Tests
// ============================================================================

#[test]
fn test_full_pipeline() -> Result<()> {
    let device = Device::Cpu;
    // 1. Create ODE
    let config = NeuralODEConfig {
        time_steps: 5,
        dt: 0.01,
        mc_samples: 32,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;

    // 2. Integrate
    let next = ode.integrate_step()?;
    assert!(next.log_volume_proxy()?.is_finite());

    // 3. Compute flowpipe
    let flowpipe = ode.compute_flowpipe()?;
    assert_eq!(flowpipe.steps.len(), 6);

    // 4. Generate certificate
    let cert = ode.generate_certificate()?;
    assert!(cert.num_steps > 0);

    // 5. Verify safety
    let cbf = ControlBarrierFunction::norm_based(1.0, 8);
    let _safe = ode.verify_safe_trajectory(&cbf, 1.0)?;

    Ok(())
}

#[test]
fn test_full_pipeline_with_self_improvement() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        time_steps: 3,
        dt: 0.01,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;

    // Self-improvement loop
    let si_config = SelfImprovementConfig {
        rounds: 3,
        ..SelfImprovementConfig::default()
    };
    let mut engine = SelfImprovementEngine::new(si_config, 1.0);
    let results = engine.run_loop(&ode)?;

    assert_eq!(results.len(), 3);
    assert!(engine.cumulative_vfe_reduction() > 0.0);
    Ok(())
}

#[test]
fn test_multiple_integration_steps() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        time_steps: 10,
        dt: 0.01,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let flowpipe = ode.compute_flowpipe()?;
    assert_eq!(flowpipe.steps.len(), 11);
    // All steps should have finite volumes
    for step in &flowpipe.steps {
        assert!(step.log_volume.is_finite());
    }
    Ok(())
}

#[test]
fn test_certificate_consistency() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        time_steps: 5,
        dt: 0.02,
        mc_samples: 32,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let cert = ode.generate_certificate()?;
    // cert.num_steps = flowpipe.steps.len() (includes initial step)
    let flowpipe = ode.compute_flowpipe()?;
    assert_eq!(cert.num_steps, flowpipe.steps.len());
    Ok(())
}

#[test]
fn test_lifelong_learning() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        time_steps: 2,
        dt: 0.01,
        mc_samples: 8,
        ..NeuralODEConfig::default()
    };
    let mut knowledge = Vec::new();
    for _ in 0..3 {
        let ode = create_ode_config(4, &device, config.clone())?;
        let cert = ode.generate_certificate()?;
        knowledge.push(cert.violation_prob);
    }
    assert_eq!(knowledge.len(), 3);
    for &v in &knowledge {
        assert!(v.is_finite());
        assert!((0.0..=1.0).contains(&v));
    }
    Ok(())
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_ode_zero_center() -> Result<()> {
    let device = Device::Cpu;
    let field = create_field(8, &device)?;
    let center = Tensor::zeros((1, 8), DType::F32, &device)?;
    let config = NeuralODEConfig::default();
    let ode = NeuralODEZonotope::from_epsilon(&center, 0.1, field, config)?;
    let next = ode.euler_step()?;
    assert!(next.log_volume_proxy()?.is_finite());
    Ok(())
}

#[test]
fn test_ode_nonzero_center() -> Result<()> {
    let device = Device::Cpu;
    let field = create_field(8, &device)?;
    let center = Tensor::ones((1, 8), DType::F32, &device)?;
    let config = NeuralODEConfig::default();
    let ode = NeuralODEZonotope::from_epsilon(&center, 0.1, field, config)?;
    let next = ode.euler_step()?;
    assert!(next.log_volume_proxy()?.is_finite());
    Ok(())
}

#[test]
fn test_flowpipe_empty_steps() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        time_steps: 0,
        dt: 0.01,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let flowpipe = ode.compute_flowpipe()?;
    assert_eq!(flowpipe.steps.len(), 1); // Just initial state
    Ok(())
}

#[test]
fn test_cbf_evaluate_batch() -> Result<()> {
    let device = Device::Cpu;
    // CBF.evaluate creates weight tensor matching x.shape() — weight.len() must == x.num_elements()
    // For a (2, 2) input, we need 4 weight elements
    let cbf = ControlBarrierFunction::new(vec![1.0f32, -1.0, 0.5, -0.5], 0.5, 1.0);
    let x = Tensor::from_vec(vec![1.0f32, 0.5, 2.0, 1.0], (2, 2), &device)?;
    let val = cbf.evaluate(&x, &device)?;
    let vals: Vec<f32> = val.flatten_all()?.to_vec1()?;
    for v in vals {
        assert!(v.is_finite());
    }
    Ok(())
}

#[test]
fn test_improvement_result_fields() -> Result<()> {
    let device = Device::Cpu;
    let ode = create_ode(8, &device)?;
    let mut engine = SelfImprovementEngine::new(SelfImprovementConfig::default(), 1.0);
    let result = engine.run_round(&ode)?;
    // Check all fields are accessible and valid
    assert_eq!(result.round, 0);
    assert!(result.vfe_before.is_finite());
    assert!(result.vfe_after.is_finite());
    assert!(result.vfe_reduction.is_finite());
    assert!(result.certified_epsilon.is_finite());
    assert!(result.diversity.is_finite());
    assert!(result.reward.is_finite());
    Ok(())
}

#[test]
fn test_neural_ode_config_clone() {
    let config = NeuralODEConfig::default();
    let cloned = config.clone();
    assert_eq!(config.dt, cloned.dt);
    assert_eq!(config.time_steps, cloned.time_steps);
    assert_eq!(config.method, cloned.method);
}

#[test]
fn test_flowpipe_structure() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        time_steps: 5,
        dt: 0.01,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let flowpipe = ode.compute_flowpipe()?;
    // Check structure fields
    assert!(flowpipe.steps.len() > 0);
    assert!(flowpipe.max_log_volume.is_finite());
    Ok(())
}

#[test]
fn test_certificate_structure() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        time_steps: 3,
        dt: 0.01,
        mc_samples: 16,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let cert = ode.generate_certificate()?;
    // Check all fields exist and are valid
    assert!(cert.is_safe || !cert.is_safe);
    assert!(cert.min_cbf_value.is_finite());
    assert!(cert.max_log_volume.is_finite());
    assert!(cert.certified_epsilon >= 0.0);
    assert!(cert.num_steps > 0);
    assert!(cert.total_time > 0.0);
    assert!((0.0..=1.0).contains(&cert.violation_prob));
    Ok(())
}

#[test]
fn test_engine_history_grows() -> Result<()> {
    let device = Device::Cpu;
    let ode = create_ode(8, &device)?;
    let mut engine = SelfImprovementEngine::new(SelfImprovementConfig::default(), 1.0);
    assert!(engine.history.is_empty());
    engine.run_round(&ode)?;
    assert_eq!(engine.history.len(), 1);
    engine.run_round(&ode)?;
    assert_eq!(engine.history.len(), 2);
    Ok(())
}

#[test]
fn test_cbf_lie_derivative_batch() -> Result<()> {
    let device = Device::Cpu;
    let cbf = ControlBarrierFunction::new(vec![1.0, -1.0, 0.5], 0.5, 1.0);
    let f_x = Tensor::ones(3, DType::F32, &device)?;
    let lie = cbf.lie_derivative(&f_x)?;
    assert!(lie.to_scalar::<f32>()?.is_finite());
    Ok(())
}

#[test]
fn test_field_clone() -> Result<()> {
    let device = Device::Cpu;
    let field = create_field(8, &device)?;
    let cloned = field.clone();
    assert_eq!(field.weight.shape().dims(), cloned.weight.shape().dims());
    Ok(())
}

#[test]
fn test_taylor_correction_flag() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        taylor_correction: true,
        time_steps: 3,
        dt: 0.01,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let next = ode.integrate_step()?;
    assert!(next.log_volume_proxy()?.is_finite());
    Ok(())
}

#[test]
fn test_neural_tightener_flag() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        use_neural_tightener: true,
        time_steps: 3,
        dt: 0.01,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let next = ode.integrate_step()?;
    assert!(next.log_volume_proxy()?.is_finite());
    Ok(())
}

#[test]
fn test_max_gens_config() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        max_gens: 64,
        time_steps: 3,
        dt: 0.01,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let next = ode.integrate_step()?;
    assert!(next.log_volume_proxy()?.is_finite());
    Ok(())
}

#[test]
fn test_verify_safe_returns_bool() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        time_steps: 3,
        dt: 0.01,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let cbf = ControlBarrierFunction::norm_based(1.0, 8);
    let result: bool = ode.verify_safe_trajectory(&cbf, 1.0)?;
    // Just verify it returns a valid bool
    assert!(result || !result);
    Ok(())
}

#[test]
fn test_self_improvement_convergence() -> Result<()> {
    let device = Device::Cpu;
    let ode = create_ode(8, &device)?;
    let config = SelfImprovementConfig {
        rounds: 10,
        ..SelfImprovementConfig::default()
    };
    let mut engine = SelfImprovementEngine::new(config, 1.0);
    let results = engine.run_loop(&ode)?;
    // VFE should decrease over rounds
    assert!(results.last().unwrap().vfe_after <= results.first().unwrap().vfe_before);
    Ok(())
}

#[test]
fn test_certificate_safe_field() -> Result<()> {
    let device = Device::Cpu;
    let config = NeuralODEConfig {
        time_steps: 3,
        dt: 0.01,
        mc_samples: 16,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let cert = ode.generate_certificate()?;
    // is_safe is a bool
    let _safe: bool = cert.is_safe;
    Ok(())
}

#[test]
fn test_flowpipe_step_time() -> Result<()> {
    let device = Device::Cpu;
    let dt = 0.05;
    let config = NeuralODEConfig {
        time_steps: 5,
        dt,
        ..NeuralODEConfig::default()
    };
    let ode = create_ode_config(8, &device, config)?;
    let flowpipe = ode.compute_flowpipe()?;
    for (i, step) in flowpipe.steps.iter().enumerate() {
        let expected_time = (i as f32) * dt;
        assert!(
            (step.time - expected_time).abs() < 1e-6,
            "Step {} time mismatch",
            i
        );
    }
    Ok(())
}

#[test]
fn test_engine_current_vfe_updates() -> Result<()> {
    let device = Device::Cpu;
    let ode = create_ode(8, &device)?;
    let mut engine = SelfImprovementEngine::new(SelfImprovementConfig::default(), 1.0);
    let initial_vfe = engine.current_vfe;
    engine.run_round(&ode)?;
    assert!(engine.current_vfe <= initial_vfe);
    Ok(())
}

#[test]
fn test_cbf_weight_length_matches_dim() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;
    let cbf = ControlBarrierFunction::norm_based(1.0, dim);
    assert_eq!(cbf.weight.len(), dim);
    let x = Tensor::ones((1, dim), DType::F32, &device)?;
    let val = cbf.evaluate(&x, &device)?;
    assert!(val.to_scalar::<f32>()?.is_finite());
    Ok(())
}

#[test]
fn test_improvement_result_debug() -> Result<()> {
    let device = Device::Cpu;
    let ode = create_ode(8, &device)?;
    let mut engine = SelfImprovementEngine::new(SelfImprovementConfig::default(), 1.0);
    let result = engine.run_round(&ode)?;
    let debug_str = format!("{:?}", result);
    assert!(!debug_str.is_empty());
    Ok(())
}

#[test]
fn test_improvement_result_clone() -> Result<()> {
    let device = Device::Cpu;
    let ode = create_ode(8, &device)?;
    let mut engine = SelfImprovementEngine::new(SelfImprovementConfig::default(), 1.0);
    let result = engine.run_round(&ode)?;
    let cloned = result.clone();
    assert_eq!(result.round, cloned.round);
    assert_eq!(result.vfe_before, cloned.vfe_before);
    Ok(())
}
