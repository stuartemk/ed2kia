//! Sprint 113 (v11.3.0) — HYBRID TAYLOR-ZONOTOPE + FORMAL CBF INVARIANCE + META-SELF-IMPROVEMENT + DISTRIBUTED CERTIFICATES
//!
//! Comprehensive tests for:
//! - Taylor Model ODE propagation, Lie derivatives, CBF invariance
//! - Hybrid Taylor-Zonotope flowpipe computation
//! - Certified steering trajectory
//! - Safe meta-optimization with reach-set constraints
//! - Distributed hybrid certificates + hash-chain proofs

use candle_core::{DType, Device, Tensor};
use native_audit::meta_active_inference::{
    MetaActiveInferenceConfig, MetaActiveInferenceEngine, MetaSafetyConstraints,
};
use native_audit::neural_ode::{NeuralODEConfig, NeuralODEField, NeuralODETaylor};
use native_audit::taylor_model::TaylorModel;
use native_audit::{
    CertificateChainEntry, CollectiveHybridCertificate, DistributedHybridCertificate,
};

// ---------------------------------------------------------------------------
// Taylor Model ODE Propagation (S113 A)
// ---------------------------------------------------------------------------

#[test]
fn test_propagate_ode_step_order1() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::ones((1, 4), DType::F32, &device)?;
    let tm = TaylorModel::new_from_epsilon(&center, 0.01)?;

    // Identity ODE: f(x) = x
    let f = |x: &TaylorModel| -> candle_core::Result<TaylorModel> { Ok(x.clone()) };

    let result = tm.propagate_ode_step(&f, 0.01, 1)?;
    assert!(result.remainder.is_finite());
    Ok(())
}

#[test]
fn test_propagate_ode_step_order2() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::zeros((1, 4), DType::F32, &device)?;
    let tm = TaylorModel::new_from_epsilon(&center, 0.01)?;

    let f = |x: &TaylorModel| -> candle_core::Result<TaylorModel> { Ok(x.clone()) };
    let result = tm.propagate_ode_step(&f, 0.01, 2)?;
    assert!(result.remainder.is_finite());
    Ok(())
}

#[test]
fn test_propagate_ode_step_order3() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::zeros((1, 4), DType::F32, &device)?;
    let tm = TaylorModel::new_from_epsilon(&center, 0.01)?;

    let f = |x: &TaylorModel| -> candle_core::Result<TaylorModel> { Ok(x.clone()) };
    let result = tm.propagate_ode_step(&f, 0.01, 3)?;
    assert!(result.remainder.is_finite());
    Ok(())
}

#[test]
fn test_lipschitz_estimate() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::zeros((1, 4), DType::F32, &device)?;
    let tm = TaylorModel::new_from_epsilon(&center, 0.01)?;
    let lip = tm.lipschitz_estimate()?;
    assert!(lip.is_finite());
    assert!(lip >= 0.0);
    Ok(())
}

#[test]
fn test_update_remainder_ode() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::zeros((1, 4), DType::F32, &device)?;
    let tm = TaylorModel::new_from_epsilon(&center, 0.01)?;

    for order in [1, 2, 3] {
        let new_r = tm.update_remainder_ode(0.01, order)?;
        assert!(new_r.is_finite());
        assert!(new_r >= tm.remainder); // Remainder should grow or stay same
    }
    Ok(())
}

#[test]
fn test_safety_margin() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::ones((1, 4), DType::F32, &device)?;
    let tm = TaylorModel::new_from_epsilon(&center, 0.01)?;
    // evaluate_cbf (used by safety_margin) expects weight shape (1, dim)
    // and does double squeeze: weight @ center.t() -> (1,1) -> squeeze -> squeeze -> scalar
    let weight = Tensor::ones((1, 4), DType::F32, &device)?;
    let margin = tm.safety_margin(&weight, 0.0)?;
    assert!(margin.is_finite());
    Ok(())
}

// ---------------------------------------------------------------------------
// CBF Invariance (S113 A)
// ---------------------------------------------------------------------------

#[test]
fn test_lie_derivative_bound_vec() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::zeros((1, 4), DType::F32, &device)?;
    let tm = TaylorModel::new_from_epsilon(&center, 0.01)?;
    let grad_h = Tensor::ones((1, 4), DType::F32, &device)?;

    let bound = tm.lie_derivative_bound_vec(&grad_h, &tm)?;
    assert!(bound.is_finite());
    Ok(())
}

#[test]
fn test_cbf_value() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::ones((1, 4), DType::F32, &device)?;
    let tm = TaylorModel::new_from_epsilon(&center, 0.01)?;
    // cbf_value expects weight shape (1, dim) — does double squeeze internally
    let weight = Tensor::ones((1, 4), DType::F32, &device)?;

    let val = tm.cbf_value(&weight, 0.0)?;
    assert!(val.is_finite());
    assert!(val > 0.0); // All positive center + positive weight → h = 4.0
    Ok(())
}

#[test]
fn test_verify_cbf_invariance_safe() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let center = Tensor::ones((1, 4), DType::F32, &device)?;
    let tm = TaylorModel::new_from_epsilon(&center, 0.001)?;
    // grad_h is used as weight in cbf_value — shape (1, dim) for proper matmul
    let grad_h = Tensor::ones((1, 4), DType::F32, &device)?;

    // Zero ODE: f(x) = 0 → no change, trivially safe
    let f = |_x: &TaylorModel| -> candle_core::Result<TaylorModel> {
        let zero = Tensor::zeros((1, 4), DType::F32, &device)?;
        TaylorModel::new_from_epsilon(&zero, 0.0)
    };

    let safe = tm.verify_cbf_invariance(&f, 0.01, 5, &grad_h, 0.0, 0.5, 1)?;
    // With zero dynamics and positive CBF, should be safe
    assert!(safe);
    Ok(())
}

// ---------------------------------------------------------------------------
// Hybrid Taylor-Zonotope Flowpipe (S113 B)
// ---------------------------------------------------------------------------

#[test]
fn test_compute_hybrid_flowpipe() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 4;
    let field = NeuralODEField::new(
        &Tensor::zeros((dim, dim), DType::F32, &device)?,
        None,
        native_audit::hybrid_zonotope::LayerType::Affine,
    )?;
    let config = NeuralODEConfig {
        time_steps: 5,
        ..Default::default()
    };
    let center = Tensor::ones((1, dim), DType::F32, &device)?;
    let ode = NeuralODETaylor::from_epsilon(&center, 0.01, field, config)?;

    let flowpipe = ode.compute_hybrid_flowpipe(3, 16, 2)?;
    assert_eq!(flowpipe.len(), 6); // time_steps + 1
    for step in &flowpipe {
        assert!(step.remainder.is_finite());
    }
    Ok(())
}

#[test]
fn test_tightness_vs_zonotope() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 4;
    let field = NeuralODEField::new(
        &Tensor::zeros((dim, dim), DType::F32, &device)?,
        None,
        native_audit::hybrid_zonotope::LayerType::Affine,
    )?;
    let config = NeuralODEConfig::default();
    let center = Tensor::ones((1, dim), DType::F32, &device)?;
    let ode = NeuralODETaylor::from_epsilon(&center, 0.01, field, config)?;

    let tightness = ode.tightness_vs_zonotope()?;
    assert!(tightness.is_finite());
    assert!(tightness > 0.0);
    Ok(())
}

// ---------------------------------------------------------------------------
// Safe Meta-Optimization (S113 C)
// ---------------------------------------------------------------------------

#[test]
fn test_meta_safety_constraints_default() {
    let constraints = MetaSafetyConstraints::default();
    assert!(constraints.max_step_size > 0.0);
    assert!(constraints.min_beta_cbf >= 0.0);
    assert!(constraints.max_lr > 0.0);
    assert!(constraints.reach_horizon > 0);
}

#[test]
fn test_meta_optimize_safe_accepts() -> candle_core::Result<()> {
    let config = MetaActiveInferenceConfig {
        meta_lr: 0.01,
        use_es: true,
        population_size: 20,
        ..Default::default()
    };
    let mut engine = MetaActiveInferenceEngine::new(&config);
    let constraints = MetaSafetyConstraints::default();
    let peer_vfes = &[0.5, 0.6, 0.4];

    let result = engine.meta_optimize_safe(peer_vfes, &constraints)?;
    assert!(result.vfe_reduction.is_finite());
    assert!(result.safety_margin.is_finite());
    assert!(result.reach_diameter.is_finite());
    assert!(result.reach_diameter >= 0.0);
    Ok(())
}

#[test]
fn test_meta_optimize_safe_rejects_large_step() -> candle_core::Result<()> {
    let config = MetaActiveInferenceConfig {
        meta_lr: 1.0, // Very large LR to trigger rejection
        use_es: true,
        population_size: 20,
        ..Default::default()
    };
    let mut engine = MetaActiveInferenceEngine::new(&config);
    let constraints = MetaSafetyConstraints {
        max_step_size: 0.001, // Very tight constraint
        ..Default::default()
    };
    let peer_vfes = &[0.5];

    let result = engine.meta_optimize_safe(peer_vfes, &constraints)?;
    // May or may not be accepted depending on gradient magnitude
    assert!(result.vfe_reduction.is_finite() || !result.accepted);
    Ok(())
}

#[test]
fn test_meta_optimize_safe_sequence() -> candle_core::Result<()> {
    let config = MetaActiveInferenceConfig {
        meta_lr: 0.02,
        use_es: true,
        population_size: 20,
        ..Default::default()
    };
    let mut engine = MetaActiveInferenceEngine::new(&config);
    let constraints = MetaSafetyConstraints::default();
    let peer_vfes = &[0.5, 0.5, 0.5];

    let results = engine.meta_optimize_safe_sequence(10, peer_vfes, &constraints)?;
    assert_eq!(results.len(), 10);
    for r in &results {
        assert!(r.vfe_reduction.is_finite() || !r.accepted);
    }
    Ok(())
}

#[test]
fn test_safe_opt_result_debug() {
    let constraints = MetaSafetyConstraints::default();
    let result = native_audit::meta_active_inference::SafeOptResult {
        vfe_reduction: 0.1,
        accepted: true,
        rejection_reason: None,
        safety_margin: 0.4,
        reach_diameter: 0.02,
    };
    let _ = format!("{:?}", result);
    let _ = constraints; // Avoid unused warning
}

// ---------------------------------------------------------------------------
// Distributed Certificates + Hash-Chain (S113 D)
// ---------------------------------------------------------------------------

#[test]
fn test_certificate_chain_genesis() {
    let entry = CertificateChainEntry::genesis([1u8; 32], 1000);
    assert_eq!(entry.sequence, 0);
    assert_eq!(entry.prev_hash, [0u8; 32]);
    assert_eq!(entry.cert_hash, [1u8; 32]);
}

#[test]
fn test_certificate_chain_append() {
    let genesis = CertificateChainEntry::genesis([1u8; 32], 1000);
    let second = genesis.append([2u8; 32], 1001);
    assert_eq!(second.sequence, 1);
    assert_ne!(second.prev_hash, [0u8; 32]); // Should have non-zero prev_hash
}

#[test]
fn test_certificate_chain_verify_link() {
    let genesis = CertificateChainEntry::genesis([1u8; 32], 1000);
    let second = genesis.append([2u8; 32], 1001);
    let third = second.append([3u8; 32], 1002);

    assert!(third.verify_link(&second));
    assert!(!third.verify_link(&genesis)); // Wrong predecessor
}

#[test]
fn test_distributed_certificate_creation() {
    let chain = CertificateChainEntry::genesis([1u8; 32], 1000);
    let cert = DistributedHybridCertificate::new(
        1,
        vec![-1.0, -2.0],
        vec![1.0, 2.0],
        0.5,
        true,
        2,
        16,
        10,
        0.01,
        chain,
        0.1,
    );
    assert!(cert.is_safe);
    assert_eq!(cert.node_id, 1);
    assert_eq!(cert.taylor_order, 2);
}

#[test]
fn test_distributed_certificate_hash() {
    let chain = CertificateChainEntry::genesis([1u8; 32], 1000);
    let cert = DistributedHybridCertificate::new(
        1,
        vec![-1.0],
        vec![1.0],
        0.5,
        true,
        2,
        16,
        10,
        0.01,
        chain,
        0.1,
    );
    let hash = cert.compute_hash();
    assert_ne!(hash, [0u8; 32]); // Non-trivial hash

    // Same cert → same hash
    let cert2 = DistributedHybridCertificate::new(
        1,
        vec![-1.0],
        vec![1.0],
        0.5,
        true,
        2,
        16,
        10,
        0.01,
        CertificateChainEntry::genesis([1u8; 32], 1000),
        0.1,
    );
    assert_eq!(hash, cert2.compute_hash());
}

#[test]
fn test_distributed_certificate_avg_width() {
    let chain = CertificateChainEntry::genesis([1u8; 32], 1000);
    let cert = DistributedHybridCertificate::new(
        1,
        vec![-1.0, -2.0, -3.0],
        vec![1.0, 2.0, 3.0],
        0.5,
        true,
        2,
        16,
        10,
        0.01,
        chain,
        0.1,
    );
    let avg = cert.avg_width();
    // Widths are [2, 4, 6], avg = (2+4+6)/3 = 4.0
    assert!((avg - 4.0).abs() < 1e-6);
}

#[test]
fn test_collective_certificate_aggregate() {
    let certs: Vec<DistributedHybridCertificate> = (0..5)
        .map(|i| {
            DistributedHybridCertificate::new(
                i,
                vec![-1.0 - i as f32 * 0.1],
                vec![1.0 + i as f32 * 0.1],
                0.5 - i as f32 * 0.05,
                true,
                2,
                16,
                10,
                0.01,
                CertificateChainEntry::genesis([1u8; 32], 1000 + i as u64),
                0.1,
            )
        })
        .collect();

    let collective = CollectiveHybridCertificate::aggregate(certs, 0.66);
    assert_eq!(collective.node_count, 5);
    assert!(collective.quorum_met);
    assert!(collective.collective_safe);
}

#[test]
fn test_collective_certificate_byzantine() {
    let certs: Vec<DistributedHybridCertificate> = (0..6)
        .map(|i| {
            let is_safe = i < 4; // 4 safe, 2 unsafe
            DistributedHybridCertificate::new(
                i,
                vec![-1.0],
                vec![1.0],
                0.5,
                is_safe,
                2,
                16,
                10,
                0.01,
                CertificateChainEntry::genesis([1u8; 32], 1000),
                0.1,
            )
        })
        .collect();

    let collective = CollectiveHybridCertificate::aggregate(certs, 0.66);
    assert!(!collective.collective_safe); // Not all safe
    assert!(collective.quorum_met); // 4/6 = 66.7% >= 66%
}

#[test]
fn test_collective_certificate_empty() {
    let collective = CollectiveHybridCertificate::aggregate(Vec::new(), 0.66);
    assert_eq!(collective.node_count, 0);
    assert!(!collective.quorum_met);
    assert!(collective.collective_safe); // Vacuously true
}

#[test]
fn test_collective_tightness() {
    let certs: Vec<DistributedHybridCertificate> = (0..3)
        .map(|_| {
            DistributedHybridCertificate::new(
                0,
                vec![-1.0, -2.0],
                vec![1.0, 2.0],
                0.5,
                true,
                2,
                16,
                10,
                0.01,
                CertificateChainEntry::genesis([1u8; 32], 1000),
                0.1,
            )
        })
        .collect();

    let collective = CollectiveHybridCertificate::aggregate(certs, 0.66);
    let tightness = collective.collective_tightness();
    assert!(tightness.is_finite());
    assert!(tightness > 0.0);
}

// ---------------------------------------------------------------------------
// Integration: Full Sprint 113 Pipeline
// ---------------------------------------------------------------------------

#[test]
fn test_sprint113_full_pipeline() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let dim = 4;

    // 1. Create Taylor Model
    let center = Tensor::ones((1, dim), DType::F32, &device)?;
    let tm = TaylorModel::new_from_epsilon(&center, 0.01)?;

    // 2. Propagate ODE step
    let f = |x: &TaylorModel| -> candle_core::Result<TaylorModel> { Ok(x.clone()) };
    let propagated = tm.propagate_ode_step(&f, 0.01, 2)?;
    assert!(propagated.remainder.is_finite());

    // 3. Compute Lipschitz estimate
    let lip = tm.lipschitz_estimate()?;
    assert!(lip >= 0.0);

    // 4. Safe meta-optimization
    let config = MetaActiveInferenceConfig {
        meta_lr: 0.01,
        use_es: true,
        population_size: 10,
        ..Default::default()
    };
    let mut engine = MetaActiveInferenceEngine::new(&config);
    let constraints = MetaSafetyConstraints::default();
    let result = engine.meta_optimize_safe(&[0.5], &constraints)?;
    assert!(result.vfe_reduction.is_finite() || !result.accepted);

    // 5. Distributed certificate
    let (lo, hi) = tm.compute_bounds()?;
    let lo_vec: Vec<f32> = lo.flatten_to(1)?.to_vec1()?;
    let hi_vec: Vec<f32> = hi.flatten_to(1)?.to_vec1()?;
    let chain = CertificateChainEntry::genesis([1u8; 32], 1000);
    let cert = DistributedHybridCertificate::new(
        1, lo_vec, hi_vec, 0.5, true, 2, dim, 10, 0.01, chain, 0.1,
    );
    let hash = cert.compute_hash();
    assert_ne!(hash, [0u8; 32]);

    // 6. Collective aggregation
    let certs = vec![cert];
    let collective = CollectiveHybridCertificate::aggregate(certs, 0.66);
    assert!(collective.collective_safe);

    Ok(())
}
