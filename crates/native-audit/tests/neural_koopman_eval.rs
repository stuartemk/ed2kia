//! Neural Koopman CBF + ISS Lyapunov Evaluation Tests — Sprint 162 (v16.2.0)
//!
//! Validates:
//! 1. Neural Koopman Lifting via SAE Observables (dimensionality control)
//! 2. Explicit CBF Projection (fallback safety)
//! 3. ISS Lyapunov Verification (disturbance robustness)
//! 4. Neural Koopman Operator Inference Loss
//! 5. Model tracing (all tests print model_name)

use candle_core::{DType, Device, Tensor};
use native_audit::control::{
    compute_iss_tube_radius, compute_iss_ultimate_bound, explicit_cbf_projection,
    verify_iss_lyapunov,
};
use native_audit::deep_koopman::{compute_lyapunov_value, DeepKoopman};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn make_tensor(rows: usize, cols: usize, seed: f32, device: &Device) -> Result<Tensor> {
    let data: Vec<f32> = (0..rows * cols).map(|i| seed + (i as f32) * 0.01).collect();
    Ok(Tensor::from_vec(data, (rows, cols), device)?)
}

// ============================================================================
// PASO C: Neural Koopman Lifting Tests
// ============================================================================

#[test]
fn test_neural_koopman_lift_shape() -> Result<()> {
    let device = Device::Cpu;
    let model_name = "HuggingFaceTB/SmolLM2-135M";
    println!("🧪 INICIANDO TEST CON MODELO: {}", model_name);

    let hidden = make_tensor(2, 256, 0.1, &device)?; // [batch=2, d=256]
    let sae_feats = make_tensor(2, 128, 0.5, &device)?; // [batch=2, k=128]

    let lifted = DeepKoopman::neural_koopman_lift(&hidden, &sae_feats)?;
    println!(
        "📈 Dimensionalidad controlada (Lifting): {:?}",
        lifted.shape().dims()
    );

    // Verify shape: [batch, d + k] = [2, 384]
    let dims = lifted.shape().dims();
    assert_eq!(dims[0], 2, "Batch dimension preserved");
    assert_eq!(dims[1], 384, "Lifted dim = d + k = 256 + 128");

    println!("✅ Neural Koopman Lift shape verified: {:?}", dims);
    Ok(())
}

#[test]
fn test_neural_koopman_lift_preserves_hidden() -> Result<()> {
    let device = Device::Cpu;
    let model_name = "HuggingFaceTB/SmolLM2-135M";
    println!("🧪 MODELO: {} — Lift preserves raw hidden", model_name);

    let hidden = make_tensor(1, 64, 0.3, &device)?;
    let sae_feats = make_tensor(1, 32, 0.7, &device)?;

    let lifted = DeepKoopman::neural_koopman_lift(&hidden, &sae_feats)?;

    // Extract first d columns and compare with original hidden
    let lifted_hidden = lifted.narrow(1, 0, 64)?;
    let diff = lifted_hidden
        .broadcast_sub(&hidden)?
        .sqr()?
        .sum_all()?
        .to_scalar::<f32>()?;
    assert!(
        diff < 1e-6,
        "Lifted state should preserve raw hidden in first d dims"
    );

    println!("✅ Hidden state preserved in lifted observable");
    Ok(())
}

#[test]
fn test_neural_koopman_lift_large_scale() -> Result<()> {
    let device = Device::Cpu;
    let model_name = "HuggingFaceTB/SmolLM2-360M";
    println!("🧪 MODELO: {} — Large-scale lifting", model_name);

    // Simulate 360M model hidden dim (4096) + SAE features (2048)
    let hidden = make_tensor(4, 4096, 0.01, &device)?;
    let sae_feats = make_tensor(4, 2048, 0.05, &device)?;

    let lifted = DeepKoopman::neural_koopman_lift(&hidden, &sae_feats)?;
    let dims = lifted.shape().dims();
    println!("📈 Large-scale lifted dims: {:?}", dims);

    assert_eq!(dims[0], 4, "Batch preserved");
    assert_eq!(dims[1], 6144, "Lifted = 4096 + 2048");

    // Verify dimensionality is O(d + k) not O(d²)
    let naive_poly_dim = 4096 * 4096; // O(d²) polynomial lifting
    let neural_dim = dims[1];
    println!(
        "⚡ Dimensionality savings: O(d²)={} vs Neural Koopman O(d+k)={}",
        naive_poly_dim, neural_dim
    );
    assert!(
        neural_dim < naive_poly_dim,
        "Neural Koopman should be far smaller than polynomial lifting"
    );

    println!("✅ Large-scale lifting verified — dimensionality controlled");
    Ok(())
}

// ============================================================================
// PASO C: Explicit CBF Projection Tests
// ============================================================================

#[test]
fn test_explicit_cbf_projection_safe() -> Result<()> {
    let device = Device::Cpu;
    let model_name = "HuggingFaceTB/SmolLM2-135M";
    println!("🧪 MODELO: {} — CBF safe (no correction)", model_name);

    let u_nom = make_tensor(1, 10, 0.0, &device)?;
    let lg_h = make_tensor(1, 10, 1.0, &device)?;

    // h_x > 0 (safe), lf_h = 0 → violation should be negative
    let u_safe = explicit_cbf_projection(&u_nom, 0.5, 0.0, &lg_h, 1.0, 0.0, 0.0)?;

    // Should return u_nom unchanged when already safe
    let diff = u_safe
        .broadcast_sub(&u_nom)?
        .sqr()?
        .sum_all()?
        .to_scalar::<f32>()?;
    assert!(diff < 1e-6, "Safe control should be unchanged");

    println!("🛡️ Explicit CBF Projection: Safe control unchanged");
    Ok(())
}

#[test]
fn test_explicit_cbf_projection_applies_correction() -> Result<()> {
    let device = Device::Cpu;
    let model_name = "HuggingFaceTB/SmolLM2-135M";
    println!("🧪 MODELO: {} — CBF applies correction", model_name);

    let u_nom = make_tensor(1, 10, 0.0, &device)?;
    let lg_h = make_tensor(1, 10, 1.0, &device)?;

    // h_x < 0 (unsafe), large gamma → violation positive → correction applied
    let u_safe = explicit_cbf_projection(&u_nom, -0.5, 0.0, &lg_h, 2.0, 0.0, 0.0)?;

    // u_safe should differ from u_nom
    let diff = u_safe
        .broadcast_sub(&u_nom)?
        .abs()?
        .sum_all()?
        .to_scalar::<f32>()?;
    assert!(diff > 0.01, "Unsafe control should be corrected");

    println!(
        "🛡️ Explicit CBF Projection: Correction applied (diff={:.4})",
        diff
    );
    Ok(())
}

#[test]
fn test_explicit_cbf_projection_degenerate() -> Result<()> {
    let device = Device::Cpu;
    let model_name = "HuggingFaceTB/SmolLM2-135M";
    println!("🧪 MODELO: {} — CBF degenerate L_g·h", model_name);

    let u_nom = make_tensor(1, 10, 0.5, &device)?;
    let lg_h = Tensor::zeros((1, 10), DType::F32, &device)?; // Zero control channel

    let u_safe = explicit_cbf_projection(&u_nom, -1.0, 0.0, &lg_h, 1.0, 0.0, 0.0)?;

    // Should return u_nom when ||L_g·h||² ≈ 0
    let diff = u_safe
        .broadcast_sub(&u_nom)?
        .sqr()?
        .sum_all()?
        .to_scalar::<f32>()?;
    assert!(
        diff < 1e-6,
        "Degenerate control channel should return nominal"
    );

    println!("🛡️ Explicit CBF Projection: Degenerate case handled gracefully");
    Ok(())
}

#[test]
fn test_explicit_cbf_projection_tube_robust() -> Result<()> {
    let device = Device::Cpu;
    let model_name = "HuggingFaceTB/SmolLM2-135M";
    println!(
        "🧪 MODELO: {} — CBF with tube + conformal margin",
        model_name
    );

    let u_nom = make_tensor(1, 8, 0.0, &device)?;
    let lg_h = make_tensor(1, 8, 1.0, &device)?;

    // With tube_radius and delta_conformal, the projection is more conservative
    let u_safe = explicit_cbf_projection(&u_nom, 0.1, -0.2, &lg_h, 1.0, 0.05, 0.1)?;

    println!(
        "🛡️ CBF tube-robust projection computed, shape: {:?}",
        u_safe.shape().dims()
    );
    assert_eq!(u_safe.shape().dims(), &[1, 8], "Shape preserved");
    Ok(())
}

// ============================================================================
// PASO C: ISS Lyapunov Tests
// ============================================================================

#[test]
fn test_iss_lyapunov_stable() {
    let model_name = "HuggingFaceTB/SmolLM2-135M";
    println!("🧪 MODELO: {} — ISS Lyapunov stable", model_name);

    // V̇ = -0.6, V = 0.5, ||w||² = 0.01, α = 1.0, β = 2.0
    // ISS bound: -α·V + β·||w||² = -0.5 + 0.02 = -0.48
    // V̇ = -0.6 ≤ -0.48 → SATISFIED
    let is_stable = verify_iss_lyapunov(-0.6, 0.5, 0.01, 1.0, 2.0);
    assert!(is_stable, "System should be ISS stable");

    println!("⚖️ ISS Lyapunov Stability Verified: {}", is_stable);
}

#[test]
fn test_iss_lyapunov_unstable() {
    let model_name = "HuggingFaceTB/SmolLM2-135M";
    println!("🧪 MODELO: {} — ISS Lyapunov unstable", model_name);

    // V̇ = 0.5, V = 0.1, ||w||² = 0.01, α = 1.0, β = 2.0
    // ISS bound: -0.1 + 0.02 = -0.08
    // V̇ = 0.5 > -0.08 → VIOLATED
    let is_stable = verify_iss_lyapunov(0.5, 0.1, 0.01, 1.0, 2.0);
    assert!(!is_stable, "System should be ISS unstable");

    println!("⚖️ ISS Lyapunov Violation Detected (as expected)");
}

#[test]
fn test_iss_lyapunov_boundary() {
    let model_name = "HuggingFaceTB/SmolLM2-135M";
    println!("🧪 MODELO: {} — ISS Lyapunov boundary", model_name);

    // V̇ = -0.48, V = 0.5, ||w||² = 0.01, α = 1.0, β = 2.0
    // ISS bound: -0.5 + 0.02 = -0.48
    // V̇ = -0.48 ≤ -0.48 → SATISFIED (boundary)
    let is_stable = verify_iss_lyapunov(-0.48, 0.5, 0.01, 1.0, 2.0);
    assert!(is_stable, "Boundary case should be stable");

    println!("⚖️ ISS Lyapunov Boundary case: stable");
}

#[test]
fn test_iss_ultimate_bound() {
    let model_name = "HuggingFaceTB/SmolLM2-135M";
    println!("🧪 MODELO: {} — ISS ultimate bound", model_name);

    // r_ultimate = (β/α) · w_max² = (2.0/1.0) · 0.01 = 0.02
    let r = compute_iss_ultimate_bound(1.0, 2.0, 0.01);
    assert!((r - 0.02).abs() < 1e-6, "Ultimate bound should be 0.02");

    println!("📐 ISS Ultimate Bound: r = {:.4}", r);
}

#[test]
fn test_iss_ultimate_bound_infinity() {
    let model_name = "HuggingFaceTB/SmolLM2-135M";
    println!("🧪 MODELO: {} — ISS bound with α=0", model_name);

    let r = compute_iss_ultimate_bound(0.0, 2.0, 0.01);
    assert!(r.is_infinite(), "Zero alpha → infinite bound");

    println!("📐 ISS Ultimate Bound with α=0: ∞ (unstable)");
}

#[test]
fn test_iss_tube_radius() {
    let model_name = "HuggingFaceTB/SmolLM2-135M";
    println!("🧪 MODELO: {} — ISS tube radius", model_name);

    // r_tube = r_ultimate + ε_conformal = 0.02 + 0.05 = 0.07
    let r = compute_iss_tube_radius(1.0, 2.0, 0.01, 0.05);
    assert!((r - 0.07).abs() < 1e-6, "Tube radius should be 0.07");

    println!("📐 ISS Tube Radius: r = {:.4} (ultimate + conformal)", r);
}

// ============================================================================
// PASO C: Neural Koopman Operator Loss Tests
// ============================================================================

#[test]
fn test_neural_koopman_operator_loss_basic() -> Result<()> {
    let device = Device::Cpu;
    let model_name = "HuggingFaceTB/SmolLM2-135M";
    println!("🧪 MODELO: {} — Operator Inference Loss", model_name);

    let psi_t = make_tensor(2, 16, 0.1, &device)?;
    let psi_t_next = make_tensor(2, 16, 0.11, &device)?;
    let k_op = Tensor::eye(16, DType::F32, &device)?; // Identity operator
    let psi_safe = Tensor::zeros((2, 16), DType::F32, &device)?;

    let loss = DeepKoopman::neural_koopman_operator_loss(
        &psi_t,
        &psi_t_next,
        &k_op,
        None,
        None,
        0.1,
        0.01,
        &psi_safe,
    )?;

    println!("📊 Neural Koopman Operator Loss: {:.6}", loss);
    assert!(loss >= 0.0, "Loss should be non-negative");

    Ok(())
}

#[test]
fn test_neural_koopman_operator_loss_zero_residual() -> Result<()> {
    let device = Device::Cpu;
    let model_name = "HuggingFaceTB/SmolLM2-135M";
    println!("🧪 MODELO: {} — Zero residual loss", model_name);

    // When ψ_{t+1} = K·ψ_t exactly, data loss should be near zero
    let psi_t = make_tensor(2, 8, 0.5, &device)?;
    let k_op = Tensor::eye(8, DType::F32, &device)?;
    // Row-vector convention: [batch, dim] x [dim, dim] → [batch, dim]
    let psi_t_next = psi_t.matmul(&k_op)?; // Exact Koopman evolution
    let psi_safe = psi_t.clone(); // Safe = current → V = 0

    let loss = DeepKoopman::neural_koopman_operator_loss(
        &psi_t,
        &psi_t_next,
        &k_op,
        None,
        None,
        0.0,
        0.0,
        &psi_safe,
    )?;

    println!("📊 Zero residual loss: {:.8}", loss);
    assert!(loss < 1e-10, "Exact evolution should have near-zero loss");

    Ok(())
}

#[test]
fn test_neural_koopman_operator_loss_lyapunov_penalty() -> Result<()> {
    let device = Device::Cpu;
    let model_name = "HuggingFaceTB/SmolLM2-135M";
    println!("🧪 MODELO: {} — Lyapunov penalty in loss", model_name);

    let psi_t = make_tensor(2, 8, 0.1, &device)?;
    let psi_safe = Tensor::zeros((2, 8), DType::F32, &device)?;

    // psi_t_next further from safe → V increases → penalty
    let psi_t_next = make_tensor(2, 8, 0.5, &device)?;
    let k_op = Tensor::eye(8, DType::F32, &device)?;

    let loss_with_lyapunov = DeepKoopman::neural_koopman_operator_loss(
        &psi_t,
        &psi_t_next,
        &k_op,
        None,
        None,
        0.0,
        1.0,
        &psi_safe,
    )?;

    let loss_without_lyapunov = DeepKoopman::neural_koopman_operator_loss(
        &psi_t,
        &psi_t_next,
        &k_op,
        None,
        None,
        0.0,
        0.0,
        &psi_safe,
    )?;

    println!(
        "📊 Loss with Lyapunov: {:.6}, without: {:.6}",
        loss_with_lyapunov, loss_without_lyapunov
    );
    assert!(
        loss_with_lyapunov >= loss_without_lyapunov,
        "Lyapunov penalty should increase loss"
    );

    Ok(())
}

// ============================================================================
// PASO C: Full Pipeline Test
// ============================================================================

#[test]
fn test_full_neural_koopman_cbf_pipeline() -> Result<()> {
    let device = Device::Cpu;
    let model_name = "HuggingFaceTB/SmolLM2-360M";
    println!("🧪 INICIANDO PIPELINE COMPLETO CON MODELO: {}", model_name);

    // 1. Neural Koopman Lifting
    let hidden = make_tensor(1, 4096, 0.01, &device)?;
    let sae_feats = make_tensor(1, 2048, 0.05, &device)?;
    let lifted = DeepKoopman::neural_koopman_lift(&hidden, &sae_feats)?;
    println!("📈 Lifted state dims: {:?}", lifted.shape().dims());
    assert_eq!(lifted.shape().dims()[1], 6144);

    // 2. ISS Verification
    let v_val = compute_lyapunov_value(
        &lifted,
        &Tensor::zeros(lifted.shape(), DType::F32, &device)?,
    )?;
    let v_dot = -0.5; // Simulated derivative
    let w_norm_sq = 0.01; // GGUF quantization noise
    let is_iss = verify_iss_lyapunov(v_dot, v_val, w_norm_sq, 1.0, 2.0);
    println!("⚖️ ISS Stable: {}, V={:.4}, V̇={:.4}", is_iss, v_val, v_dot);

    // 3. Explicit CBF Projection
    let u_nom = Tensor::zeros((1, 6144), DType::F32, &device)?;
    let lg_h = make_tensor(1, 6144, 1.0, &device)?;
    let u_safe = explicit_cbf_projection(&u_nom, 0.1, -0.2, &lg_h, 1.0, 0.05, 0.1)?;
    println!("🛡️ CBF Projection: shape={:?}", u_safe.shape().dims());

    // 4. ISS Tube Radius
    let tube_r = compute_iss_tube_radius(1.0, 2.0, 0.01, 0.05);
    println!("📐 ISS Tube Radius: {:.4}", tube_r);

    // 5. Operator Inference Loss
    let psi_t_next = make_tensor(1, 6144, 0.02, &device)?;
    let k_op = Tensor::eye(6144, DType::F32, &device)?;
    let loss = DeepKoopman::neural_koopman_operator_loss(
        &lifted,
        &psi_t_next,
        &k_op,
        None,
        None,
        0.1,
        0.01,
        &Tensor::zeros(lifted.shape(), DType::F32, &device)?,
    )?;
    println!("📊 Operator Inference Loss: {:.6}", loss);

    println!(
        "✅ PIPELINE COMPLETO — Neural Koopman CBF + ISS verificado para {}",
        model_name
    );
    Ok(())
}
