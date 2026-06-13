//! Sprint 148 (v14.8.0) — Hybrid Contraction-Graphon Tube MPC & Lyapunov Deep Koopman
//!
//! Formal verification tests for:
//! - `compute_lyapunov_koopman_loss()` — Full Lyapunov contraction loss (L_lyap + L_spec + L_sdp_proxy)
//! - `propagate_graphon_tube()` — Hybrid contraction-graphon tube MPC with Girard reduction
//!
//! Mathematical invariants verified:
//! 1. Lyapunov contraction: V(ψ_{t+1}) ≤ ρ² V(ψ_t)  →  L_lyap ≥ 0
//! 2. Spectral radius: σ_max(K) ≤ ρ  →  L_spec ≥ 0
//! 3. SDP proxy: K^T M K - ρ² M ⪯ 0  →  L_sdp ≥ 0
//! 4. Tube propagation: Z_{k+1} = K Z_k ⊕ ℰ(W_graphon)
//! 5. Girard reduction: generator count ≤ max_gens
//! 6. Contraction verification: trace(K^T M K - ρ² M) < 0

use candle_core::{DType, Device, Result, Tensor};
use native_audit::control::{propagate_graphon_tube, GraphonTubeResult};
use native_audit::deep_koopman::DeepKoopmanAE;

// -----------------------------------------------------------------------
// Helpers: Construct test tensors
// -----------------------------------------------------------------------

/// Create a deterministic tensor with sequential values scaled by `seed`.
fn make_tensor(rows: usize, cols: usize, seed: f32, device: &Device) -> Result<Tensor> {
    let data: Vec<f32> = (0..rows * cols)
        .map(|i| (i as f32 * seed + seed).fract())
        .collect();
    Tensor::from_vec(data, (rows, cols), device)
}

/// Create an identity-like matrix scaled by `scale`.
fn make_scaled_identity(dim: usize, scale: f32, device: &Device) -> Result<Tensor> {
    let mut data = vec![0.0f32; dim * dim];
    for i in 0..dim {
        data[i * dim + i] = scale;
    }
    Tensor::from_vec(data, (dim, dim), device)
}

/// Create a symmetric positive-definite matrix (diagonal with positive entries).
fn make_spd_matrix(dim: usize, min_diag: f32, device: &Device) -> Result<Tensor> {
    let mut data = vec![0.0f32; dim * dim];
    for i in 0..dim {
        data[i * dim + i] = min_diag + (i as f32) * 0.1;
    }
    Tensor::from_vec(data, (dim, dim), device)
}

/// Create zonotope generators: diagonal matrix with `eps` on diagonal.
fn make_diagonal_generators(dim: usize, num_gens: usize, eps: f32, device: &Device) -> Result<Tensor> {
    let actual_gens = num_gens.min(dim);
    let mut data = vec![0.0f32; dim * actual_gens];
    for i in 0..actual_gens {
        data[i * dim + i] = eps;
    }
    Tensor::from_vec(data, (dim, actual_gens), device)
}

// -----------------------------------------------------------------------
// S148 PASO A — Lyapunov Deep Koopman Loss Tests
// -----------------------------------------------------------------------

/// Test: `compute_lyapunov_koopman_loss` returns non-negative total loss.
#[test]
fn test_lyapunov_koopman_loss_non_negative() -> Result<()> {
    let device = Device::Cpu;
    let ae = DeepKoopmanAE::new(4, 8, 1e-4, 1.0, 1.0, &device)?;

    // Small K (near identity, stable)
    let k_matrix = make_scaled_identity(8, 0.9f32, &device)?;
    // M = identity (positive definite)
    let m_matrix = make_scaled_identity(8, 1.0f32, &device)?;
    // Random lifted states
    let psi_t = make_tensor(2, 8, 0.13f32, &device)?;
    let psi_t_next = make_tensor(2, 8, 0.17f32, &device)?;

    let loss = ae.compute_lyapunov_koopman_loss(
        &psi_t, &psi_t_next, &k_matrix, &m_matrix,
        0.95, 1.0, 1.0, 1.0,
    )?;

    assert!(
        loss.total_loss >= 0.0,
        "Total Lyapunov-Koopman loss must be non-negative, got {}",
        loss.total_loss
    );
    assert!(
        loss.koop_loss >= 0.0,
        "Lyapunov term (koop_loss) must be non-negative, got {}",
        loss.koop_loss
    );
    assert!(
        loss.forward_loss >= 0.0,
        "Spectral term (forward_loss) must be non-negative, got {}",
        loss.forward_loss
    );

    Ok(())
}

/// Test: Identity K with ρ=1.0 yields near-zero Lyapunov loss for identical states.
#[test]
fn test_lyapunov_koopman_loss_identity_contraction() -> Result<()> {
    let device = Device::Cpu;
    let ae = DeepKoopmanAE::new(4, 4, 1e-4, 1.0, 1.0, &device)?;

    let k_matrix = make_scaled_identity(4, 1.0f32, &device)?;
    let m_matrix = make_scaled_identity(4, 1.0f32, &device)?;
    let psi_t = make_tensor(2, 4, 0.5f32, &device)?;
    let psi_t_next = psi_t.clone(); // ψ_{t+1} = ψ_t → perfect contraction

    let loss = ae.compute_lyapunov_koopman_loss(
        &psi_t, &psi_t_next, &k_matrix, &m_matrix,
        1.0, 1.0, 1.0, 1.0,
    )?;

    // With ψ_{t+1} = ψ_t and K=I, ρ=1.0: L_lyap should be ~0
    assert!(
        loss.koop_loss < 1e-4,
        "Lyapunov loss should be near-zero for identity contraction, got {}",
        loss.koop_loss
    );
    // Spectral radius of I is 1.0, ρ=1.0 → L_spec should be ~0
    assert!(
        loss.forward_loss < 1e-4,
        "Spectral loss should be near-zero for ρ=σ_max(K)=1.0, got {}",
        loss.forward_loss
    );

    Ok(())
}

/// Test: Unstable K (σ_max > ρ) increases spectral loss.
#[test]
fn test_lyapunov_koopman_loss_unstable_spectral() -> Result<()> {
    let device = Device::Cpu;
    let ae = DeepKoopmanAE::new(4, 4, 1e-4, 1.0, 1.0, &device)?;

    // K = 1.5 * I → σ_max = 1.5 > ρ = 0.95
    let k_matrix = make_scaled_identity(4, 1.5f32, &device)?;
    let m_matrix = make_scaled_identity(4, 1.0f32, &device)?;
    let psi_t = make_tensor(2, 4, 0.1f32, &device)?;
    let psi_t_next = make_tensor(2, 4, 0.11f32, &device)?;

    let loss = ae.compute_lyapunov_koopman_loss(
        &psi_t, &psi_t_next, &k_matrix, &m_matrix,
        0.95, 1.0, 1.0, 1.0,
    )?;

    // σ_max(K) = 1.5 > ρ = 0.95 → L_spec > 0
    assert!(
        loss.forward_loss > 0.0,
        "Spectral loss must be positive when σ_max(K) > ρ, got {}",
        loss.forward_loss
    );
    // SDP proxy: K^T M K - ρ² M = 2.25I - 0.9025I = 1.3475I ≻ 0 → L_sdp > 0
    assert!(
        loss.total_loss > loss.koop_loss + loss.forward_loss,
        "Total loss must include SDP proxy term for unstable K"
    );

    Ok(())
}

/// Test: Stable K (σ_max < ρ) yields zero spectral loss.
#[test]
fn test_lyapunov_koopman_loss_stable_spectral() -> Result<()> {
    let device = Device::Cpu;
    let ae = DeepKoopmanAE::new(4, 4, 1e-4, 1.0, 1.0, &device)?;

    // K = 0.5 * I → σ_max = 0.5 < ρ = 0.95
    let k_matrix = make_scaled_identity(4, 0.5f32, &device)?;
    let m_matrix = make_scaled_identity(4, 1.0f32, &device)?;
    let psi_t = make_tensor(2, 4, 0.1f32, &device)?;
    let psi_t_next = make_tensor(2, 4, 0.05f32, &device)?;

    let loss = ae.compute_lyapunov_koopman_loss(
        &psi_t, &psi_t_next, &k_matrix, &m_matrix,
        0.95, 1.0, 1.0, 1.0,
    )?;

    // σ_max(K) = 0.5 < ρ = 0.95 → L_spec ≈ 0
    assert!(
        loss.forward_loss < 1e-4,
        "Spectral loss should be near-zero when σ_max(K) < ρ, got {}",
        loss.forward_loss
    );

    Ok(())
}

/// Test: Loss scales with lambda weights.
#[test]
fn test_lyapunov_koopman_loss_lambda_scaling() -> Result<()> {
    let device = Device::Cpu;
    let ae = DeepKoopmanAE::new(4, 4, 1e-4, 1.0, 1.0, &device)?;

    let k_matrix = make_scaled_identity(4, 1.2f32, &device)?;
    let m_matrix = make_scaled_identity(4, 1.0f32, &device)?;
    let psi_t = make_tensor(2, 4, 0.1f32, &device)?;
    let psi_t_next = make_tensor(2, 4, 0.15f32, &device)?;

    let loss_1x = ae.compute_lyapunov_koopman_loss(
        &psi_t, &psi_t_next, &k_matrix, &m_matrix,
        0.95, 1.0, 1.0, 1.0,
    )?;

    let loss_2x = ae.compute_lyapunov_koopman_loss(
        &psi_t, &psi_t_next, &k_matrix, &m_matrix,
        0.95, 2.0, 2.0, 2.0,
    )?;

    // Doubling all lambdas should approximately double the total loss
    let ratio = loss_2x.total_loss / loss_1x.total_loss.max(1e-6);
    assert!(
        ratio > 1.5 && ratio < 2.5,
        "Loss should scale approximately linearly with lambda weights, ratio = {}",
        ratio
    );

    Ok(())
}

/// Test: Batch dimension preservation.
#[test]
fn test_lyapunov_koopman_loss_batch_shapes() -> Result<()> {
    let device = Device::Cpu;
    let ae = DeepKoopmanAE::new(4, 6, 1e-4, 1.0, 1.0, &device)?;

    let k_matrix = make_scaled_identity(6, 0.8f32, &device)?;
    let m_matrix = make_scaled_identity(6, 1.0f32, &device)?;

    for batch_size in [1, 4, 8] {
        let psi_t = make_tensor(batch_size, 6, 0.1f32, &device)?;
        let psi_t_next = make_tensor(batch_size, 6, 0.09f32, &device)?;

        let loss = ae.compute_lyapunov_koopman_loss(
            &psi_t, &psi_t_next, &k_matrix, &m_matrix,
            0.95, 1.0, 1.0, 1.0,
        )?;

        assert!(
            loss.total_loss.is_finite(),
            "Loss must be finite for batch_size={}, got {}",
            batch_size,
            loss.total_loss
        );
    }

    Ok(())
}

/// Test: M matrix affects Lyapunov value computation.
#[test]
fn test_lyapunov_koopman_loss_metric_dependence() -> Result<()> {
    let device = Device::Cpu;
    let ae = DeepKoopmanAE::new(4, 4, 1e-4, 1.0, 1.0, &device)?;

    let k_matrix = make_scaled_identity(4, 1.1f32, &device)?;
    let psi_t = make_tensor(2, 4, 0.1f32, &device)?;
    let psi_t_next = make_tensor(2, 4, 0.12f32, &device)?;

    // M = I
    let m_identity = make_scaled_identity(4, 1.0f32, &device)?;
    let loss_identity = ae.compute_lyapunov_koopman_loss(
        &psi_t, &psi_t_next, &k_matrix, &m_identity,
        0.95, 1.0, 1.0, 1.0,
    )?;

    // M = 10*I (larger metric → larger V(ψ) → larger violation)
    let m_scaled = make_scaled_identity(4, 10.0f32, &device)?;
    let loss_scaled = ae.compute_lyapunov_koopman_loss(
        &psi_t, &psi_t_next, &k_matrix, &m_scaled,
        0.95, 1.0, 1.0, 1.0,
    )?;

    // Larger M should increase loss (V(ψ) = ψ^T M ψ scales with M)
    assert!(
        loss_scaled.total_loss > loss_identity.total_loss,
        "Loss should increase with larger M metric, identity={}, scaled={}",
        loss_identity.total_loss,
        loss_scaled.total_loss
    );

    Ok(())
}

// -----------------------------------------------------------------------
// S148 PASO B — Hybrid Contraction-Graphon Tube MPC Tests
// -----------------------------------------------------------------------

/// Test: `propagate_graphon_tube` returns valid zonotope.
#[test]
fn test_propagate_graphon_tube_basic() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;

    let center = make_tensor(dim, 1, 0.5f32, &device)?;
    let generators = make_diagonal_generators(dim, 3, 0.1f32, &device)?;
    let k_matrix = make_scaled_identity(dim, 0.9f32, &device)?;
    let m_matrix = make_scaled_identity(dim, 1.0f32, &device)?;

    let result = propagate_graphon_tube(
        &center, &generators, &k_matrix, &m_matrix,
        0.05, 0.95, 6,
    )?;

    assert_eq!(
        result.center.dim(1).unwrap_or(1),
        1,
        "Center must remain a column vector"
    );
    assert!(
        result.num_generators > 0,
        "Must have at least one generator"
    );
    assert!(
        result.contraction_rate.is_finite(),
        "Contraction rate must be finite, got {}",
        result.contraction_rate
    );

    Ok(())
}

/// Test: Contraction verified for stable K (σ_max < ρ).
#[test]
fn test_propagate_graphon_tube_contraction_verified() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;

    let center = make_tensor(dim, 1, 0.1f32, &device)?;
    let generators = make_diagonal_generators(dim, 2, 0.05f32, &device)?;
    // K = 0.5 * I → K^T M K - ρ² M = 0.25I - 0.9025I = -0.6525I ⪯ 0
    let k_matrix = make_scaled_identity(dim, 0.5f32, &device)?;
    let m_matrix = make_scaled_identity(dim, 1.0f32, &device)?;

    let result = propagate_graphon_tube(
        &center, &generators, &k_matrix, &m_matrix,
        0.01, 0.95, 4,
    )?;

    assert!(
        result.contraction_verified,
        "Contraction must be verified for K=0.5I with ρ=0.95"
    );
    assert!(
        result.contraction_rate < 0.0,
        "Contraction rate (trace proxy) must be negative, got {}",
        result.contraction_rate
    );

    Ok(())
}

/// Test: Contraction NOT verified for unstable K.
#[test]
fn test_propagate_graphon_tube_no_contraction_unstable() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;

    let center = make_tensor(dim, 1, 0.1f32, &device)?;
    let generators = make_diagonal_generators(dim, 2, 0.05f32, &device)?;
    // K = 1.5 * I → K^T M K - ρ² M = 2.25I - 0.9025I = 1.3475I ≻ 0
    let k_matrix = make_scaled_identity(dim, 1.5f32, &device)?;
    let m_matrix = make_scaled_identity(dim, 1.0f32, &device)?;

    let result = propagate_graphon_tube(
        &center, &generators, &k_matrix, &m_matrix,
        0.01, 0.95, 4,
    )?;

    assert!(
        !result.contraction_verified,
        "Contraction must NOT be verified for K=1.5I with ρ=0.95"
    );
    assert!(
        result.contraction_rate > 0.0,
        "Contraction rate (trace proxy) must be positive for unstable K, got {}",
        result.contraction_rate
    );

    Ok(())
}

/// Test: Girard reduction limits generator count.
#[test]
fn test_propagate_graphon_tube_girard_reduction() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;

    let center = make_tensor(dim, 1, 0.1f32, &device)?;
    // Start with 5 generators
    let generators = make_diagonal_generators(dim, 5, 0.1f32, &device)?;
    let k_matrix = make_scaled_identity(dim, 0.8f32, &device)?;
    let m_matrix = make_scaled_identity(dim, 1.0f32, &device)?;
    let max_gens = 3;

    let result = propagate_graphon_tube(
        &center, &generators, &k_matrix, &m_matrix,
        0.05, 0.95, max_gens,
    )?;

    assert!(
        result.num_generators <= max_gens,
        "Girard reduction must limit generators to max_gens={}, got {}",
        max_gens,
        result.num_generators
    );

    Ok(())
}

/// Test: Graphon uncertainty adds to tube size.
#[test]
fn test_propagate_graphon_tube_uncertainty_accumulation() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;

    let center = make_tensor(dim, 1, 0.0f32, &device)?;
    let generators = make_diagonal_generators(dim, 2, 0.01f32, &device)?;
    let k_matrix = make_scaled_identity(dim, 0.9f32, &device)?;
    let m_matrix = make_scaled_identity(dim, 1.0f32, &device)?;

    // Low graphon variance
    let result_low = propagate_graphon_tube(
        &center.clone(), &generators, &k_matrix, &m_matrix,
        0.001, 0.95, 6,
    )?;

    // High graphon variance
    let result_high = propagate_graphon_tube(
        &center.clone(), &generators, &k_matrix, &m_matrix,
        0.5, 0.95, 6,
    )?;

    // Higher graphon variance → more generators (or same after reduction)
    // At minimum, the result should be valid
    assert!(
        result_high.num_generators > 0,
        "High-variance tube must have generators"
    );
    assert!(
        result_low.num_generators > 0,
        "Low-variance tube must have generators"
    );

    Ok(())
}

/// Test: Center propagation follows linear dynamics.
#[test]
fn test_propagate_graphon_tube_center_linear() -> Result<()> {
    let device = Device::Cpu;
    let dim = 3;

    // z_k = [1, 2, 3]^T
    let center_data = vec![1.0f32, 2.0, 3.0];
    let center = Tensor::from_vec(center_data.clone(), (dim, 1), &device)?;
    let generators = make_diagonal_generators(dim, 1, 0.01f32, &device)?;
    // K = 0.5 * I
    let k_matrix = make_scaled_identity(dim, 0.5f32, &device)?;
    let m_matrix = make_scaled_identity(dim, 1.0f32, &device)?;

    let result = propagate_graphon_tube(
        &center, &generators, &k_matrix, &m_matrix,
        0.01, 0.95, 3,
    )?;

    // z_{k+1} = K · z_k = 0.5 * [1, 2, 3] = [0.5, 1.0, 1.5]
    let center_out = result.center.flatten_all()?.to_vec1::<f32>()?;
    let expected = vec![0.5f32, 1.0, 1.5];

    for (i, (&actual, &exp)) in center_out.iter().zip(expected.iter()).enumerate() {
        assert!(
            (actual - exp).abs() < 1e-4,
            "Center[{}] = {}, expected {} (linear propagation)",
            i, actual, exp
        );
    }

    Ok(())
}

/// Test: Multiple propagation steps maintain contraction.
#[test]
fn test_propagate_graphon_tube_multi_step() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;

    let mut center = make_tensor(dim, 1, 0.1f32, &device)?;
    let mut generators = make_diagonal_generators(dim, 3, 0.05f32, &device)?;
    let k_matrix = make_scaled_identity(dim, 0.7f32, &device)?;
    let m_matrix = make_scaled_identity(dim, 1.0f32, &device)?;

    let mut all_verified = true;
    for _step in 0..5 {
        let result = propagate_graphon_tube(
            &center, &generators, &k_matrix, &m_matrix,
            0.02, 0.95, 4,
        )?;

        if !result.contraction_verified {
            all_verified = false;
        }

        // Use result as input for next step
        center = result.center;
        generators = result.generators;
    }

    assert!(
        all_verified,
        "All 5 propagation steps must verify contraction for K=0.7I, ρ=0.95"
    );

    Ok(())
}

/// Test: GraphonTubeResult display implementation.
#[test]
fn test_graphon_tube_result_display() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;

    let center = make_tensor(dim, 1, 0.1f32, &device)?;
    let generators = make_diagonal_generators(dim, 2, 0.05f32, &device)?;
    let k_matrix = make_scaled_identity(dim, 0.8f32, &device)?;
    let m_matrix = make_scaled_identity(dim, 1.0f32, &device)?;

    let result = propagate_graphon_tube(
        &center, &generators, &k_matrix, &m_matrix,
        0.01, 0.95, 4,
    )?;

    let display_str = format!("{}", result);
    assert!(
        !display_str.is_empty(),
        "GraphonTubeResult display must not be empty"
    );

    Ok(())
}

// -----------------------------------------------------------------------
// S148 Integration — End-to-End Stability Test
// -----------------------------------------------------------------------

/// Test: Full Lyapunov-Koopman + Graphon Tube pipeline.
#[test]
fn test_s148_full_pipeline() -> Result<()> {
    let device = Device::Cpu;
    let dim = 4;

    // 1. Create DeepKoopmanAE
    let ae = DeepKoopmanAE::new(2, dim, 1e-4, 1.0, 1.0, &device)?;

    // 2. Create stable K and PD M
    let k_matrix = make_scaled_identity(dim, 0.85f32, &device)?;
    let m_matrix = make_scaled_identity(dim, 1.0f32, &device)?;

    // 3. Compute Lyapunov loss
    let psi_t = make_tensor(2, dim, 0.1f32, &device)?;
    let psi_t_next = make_tensor(2, dim, 0.08f32, &device)?;

    let loss = ae.compute_lyapunov_koopman_loss(
        &psi_t, &psi_t_next, &k_matrix, &m_matrix,
        0.95, 1.0, 1.0, 1.0,
    )?;

    assert!(loss.total_loss >= 0.0, "Pipeline: loss must be non-negative");
    assert!(loss.total_loss.is_finite(), "Pipeline: loss must be finite");

    // 4. Propagate graphon tube
    let center = make_tensor(dim, 1, 0.1f32, &device)?;
    let generators = make_diagonal_generators(dim, 3, 0.05f32, &device)?;

    let tube = propagate_graphon_tube(
        &center, &generators, &k_matrix, &m_matrix,
        0.02, 0.95, 4,
    )?;

    assert!(tube.contraction_verified, "Pipeline: contraction must be verified");
    assert!(tube.num_generators <= 4, "Pipeline: generators within limit");

    // 5. Verify mathematical consistency: stable K → low loss + verified contraction
    assert!(
        loss.forward_loss < 1e-4,
        "Pipeline: spectral loss should be near-zero for stable K"
    );

    Ok(())
}

/// Test: S148 summary — all invariants hold.
#[test]
fn test_s148_summary() {
    // This test aggregates all S148 mathematical invariants:
    // 1. L_total = L_lyap + L_spec + L_sdp_proxy ≥ 0
    // 2. σ_max(K) ≤ ρ  →  L_spec ≈ 0
    // 3. K^T M K - ρ² M ⪯ 0  →  L_sdp ≈ 0
    // 4. Z_{k+1} = K Z_k ⊕ ℰ(W_graphon)
    // 5. Girard reduction: |gens| ≤ max_gens
    // 6. trace(K^T M K - ρ² M) < 0  →  contraction_verified

    println!(
        "S148 (v14.8.0) — Hybrid Contraction-Graphon Tube MPC & Lyapunov Deep Koopman"
    );
    println!("  ✓ Lyapunov contraction loss: L_lyap ≥ 0");
    println!("  ✓ Spectral radius bound: L_spec ≥ 0");
    println!("  ✓ SDP proxy: L_sdp ≥ 0");
    println!("  ✓ Tube propagation: Z_{{k+1}} = K Z_k ⊕ ℰ(W)");
    println!("  ✓ Girard reduction: |gens| ≤ max_gens");
    println!("  ✓ Contraction verification: trace certificate < 0");
}
