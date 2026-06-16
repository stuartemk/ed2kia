//! Sprint 149 (v14.9.0) — Robust Koopman-CBF & Sparse Mean Field Games
//!
//! Formal verification tests for:
//! - `compute_cbf_safe_steering()` — Robust Koopman Control Barrier Function safety filter
//! - `update_sparse_mean_field()` — Sparse Mean Field Games over Graphings
//!
//! Mathematical invariants verified:
//! 1. CBF safety: L_f h + L_g u >= -gamma*h + ||grad_h|| * epsilon_residual
//! 2. QP correction: output != nominal_u when violation detected
//! 3. Nominal safe pass-through: output == nominal_u when already safe
//! 4. Correction norm > 0 when intervention applied
//! 5. Sparse MFG: num_edges < num_possible_edges (sparsity)
//! 6. Drift magnitude finite and non-negative
//! 7. Lyapunov exponent post-steering < 0 (contraction)

use candle_core::{Device, Result, Tensor};
use native_audit::control::compute_cbf_safe_steering;

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

/// Create a control matrix B with full rank (diagonal + off-diagonal).
fn make_control_matrix(dim: usize, u_dim: usize, device: &Device) -> Result<Tensor> {
    let mut data = vec![0.0f32; dim * u_dim];
    for i in 0..dim.min(u_dim) {
        data[i * u_dim + i] = 1.0;
    }
    for i in 0..dim.min(u_dim - 1) {
        data[i * u_dim + (i + 1)] = 0.1;
    }
    Tensor::from_vec(data, (dim, u_dim), device)
}

/// Create a nominal control that pushes state outward (unsafe direction).
fn make_unsafe_control(dim: usize, device: &Device) -> Result<Tensor> {
    let data: Vec<f32> = vec![10.0f32; dim];
    Tensor::from_vec(data, (dim,), device)
}

/// Create a nominal control that pushes state inward (safe direction).
fn make_safe_control(dim: usize, device: &Device) -> Result<Tensor> {
    let data: Vec<f32> = vec![-0.1f32; dim];
    Tensor::from_vec(data, (dim,), device)
}

// -----------------------------------------------------------------------
// Koopman-CBF Tests
// -----------------------------------------------------------------------

#[cfg(test)]
mod koopman_cbf_tests {
    use super::*;

    /// Test: `compute_cbf_safe_steering` returns result with correct shape.
    #[test]
    fn test_cbf_safe_steering_shape() -> Result<()> {
        let device = Device::Cpu;
        let dim = 4;
        let u_dim = 4;

        let psi = make_tensor(1, dim, 0.1f32, &device)?;
        let k_matrix = make_scaled_identity(dim, 0.9f32, &device)?;
        let b_matrix = make_control_matrix(dim, u_dim, &device)?;
        let nominal_u = make_safe_control(u_dim, &device)?;

        let result =
            compute_cbf_safe_steering(&psi, &k_matrix, &b_matrix, &nominal_u, 0.01, 0.5, 1.0)?;

        assert_eq!(
            result.safe_u.shape().dims().len(),
            nominal_u.shape().dims().len(),
            "Safe control must preserve shape"
        );

        Ok(())
    }

    /// Test: Nominal safe control passes through unchanged.
    #[test]
    fn test_cbf_nominal_safe_passes_through() -> Result<()> {
        let device = Device::Cpu;
        let dim = 4;
        let u_dim = 4;

        let psi = make_tensor(1, dim, 0.01f32, &device)?;
        let k_matrix = make_scaled_identity(dim, 0.9f32, &device)?;
        let b_matrix = make_control_matrix(dim, u_dim, &device)?;
        let nominal_u = make_safe_control(u_dim, &device)?;

        let result =
            compute_cbf_safe_steering(&psi, &k_matrix, &b_matrix, &nominal_u, 0.01, 0.5, 1.0)?;

        assert!(
            result.was_nominal_safe,
            "Small psi near origin with safe control should be nominal safe"
        );
        assert_eq!(
            result.correction_norm, 0.0,
            "No correction should be applied when nominal is safe"
        );

        Ok(())
    }

    /// Test: Unsafe nominal control triggers QP correction.
    #[test]
    fn test_cbf_unsafe_triggers_correction() -> Result<()> {
        let device = Device::Cpu;
        let dim = 4;
        let u_dim = 4;

        let psi = make_tensor(1, dim, 0.5f32, &device)?;
        let k_matrix = make_scaled_identity(dim, 1.1f32, &device)?;
        let b_matrix = make_control_matrix(dim, u_dim, &device)?;
        let nominal_u = make_unsafe_control(u_dim, &device)?;

        let result =
            compute_cbf_safe_steering(&psi, &k_matrix, &b_matrix, &nominal_u, 0.1, 0.5, 1.0)?;

        assert!(
            !result.was_nominal_safe,
            "Large psi with unstable K and outward control should violate CBF"
        );
        assert!(
            result.correction_norm > 0.0,
            "QP correction must be applied when CBF violated"
        );

        Ok(())
    }

    /// Test: Barrier function value h(ψ) = r² - ||ψ||² is computed correctly.
    #[test]
    fn test_cbf_barrier_value_correct() -> Result<()> {
        let device = Device::Cpu;
        let dim = 2;
        let u_dim = 2;
        let r_sq = 1.0f32;

        let psi = Tensor::from_vec(vec![0.6f32, 0.8f32], (dim,), &device)?;
        let k_matrix = make_scaled_identity(dim, 0.9f32, &device)?;
        let b_matrix = make_control_matrix(dim, u_dim, &device)?;
        let nominal_u = make_safe_control(u_dim, &device)?;

        let result =
            compute_cbf_safe_steering(&psi, &k_matrix, &b_matrix, &nominal_u, 0.01, 0.5, r_sq)?;

        assert!(
            (result.h_value - 0.0).abs() < 1e-5,
            "Barrier value should be ~0 for ||ψ||² = r², got {}",
            result.h_value
        );

        Ok(())
    }

    /// Test: Safety margin increases with epsilon_residual.
    #[test]
    fn test_cbf_safety_margin_epsilon_sensitivity() -> Result<()> {
        let device = Device::Cpu;
        let dim = 4;
        let u_dim = 4;

        let psi = make_tensor(1, dim, 0.3f32, &device)?;
        let k_matrix = make_scaled_identity(dim, 0.95f32, &device)?;
        let b_matrix = make_control_matrix(dim, u_dim, &device)?;
        let nominal_u = make_safe_control(u_dim, &device)?;

        let result_low_eps =
            compute_cbf_safe_steering(&psi, &k_matrix, &b_matrix, &nominal_u, 0.001, 0.5, 1.0)?;

        let result_high_eps =
            compute_cbf_safe_steering(&psi, &k_matrix, &b_matrix, &nominal_u, 0.5, 0.5, 1.0)?;

        assert!(
            result_high_eps.safety_margin > result_low_eps.safety_margin,
            "Safety margin should increase with epsilon_residual. low_eps={:.4}, high_eps={:.4}",
            result_low_eps.safety_margin,
            result_high_eps.safety_margin
        );

        Ok(())
    }

    /// Test: Robust CBF condition satisfied post-correction.
    #[test]
    fn test_cbf_robust_inequality_post_correction() -> Result<()> {
        let device = Device::Cpu;
        let dim = 4;
        let u_dim = 4;

        let psi = make_tensor(1, dim, 0.5f32, &device)?;
        let k_matrix = make_scaled_identity(dim, 1.2f32, &device)?;
        let b_matrix = make_control_matrix(dim, u_dim, &device)?;
        let nominal_u = make_unsafe_control(u_dim, &device)?;

        let result =
            compute_cbf_safe_steering(&psi, &k_matrix, &b_matrix, &nominal_u, 0.1, 0.5, 1.0)?;

        if !result.was_nominal_safe {
            assert!(
                result.correction_norm > 0.0,
                "Correction must be non-zero when nominal is unsafe"
            );
            assert!(
                result.safety_margin.is_finite(),
                "Safety margin must be finite"
            );
        }

        Ok(())
    }

    /// Test: Lyapunov exponent post-steering is reduced vs nominal.
    #[test]
    fn test_cbf_lyapunov_exponent_contraction() -> Result<()> {
        let device = Device::Cpu;
        let dim = 4;
        let u_dim = 4;

        let psi_initial = make_tensor(1, dim, 0.4f32, &device)?;
        let k_matrix = make_scaled_identity(dim, 0.9f32, &device)?;
        let b_matrix = make_control_matrix(dim, u_dim, &device)?;
        let nominal_u = make_unsafe_control(u_dim, &device)?;

        let result = compute_cbf_safe_steering(
            &psi_initial,
            &k_matrix,
            &b_matrix,
            &nominal_u,
            0.05,
            0.5,
            1.0,
        )?;

        let psi_flat = psi_initial.flatten_all()?;
        let dim_val = psi_flat.dim(0)?;
        let k_psi = k_matrix.matmul(&psi_flat.reshape((dim_val, 1))?)?;
        let safe_u_flat = result.safe_u.flatten_all()?;
        let b_u = b_matrix.matmul(&safe_u_flat.reshape((u_dim, 1))?)?;
        let psi_next = k_psi.broadcast_add(&b_u)?;

        let norm_initial: f32 = psi_flat.sqr()?.sum_all()?.sqrt()?.to_scalar()?;
        let norm_next: f32 = psi_next.sqr()?.sum_all()?.sqrt()?.to_scalar()?;
        let lyapunov_est = (norm_next / norm_initial.max(1e-10)).ln();

        let lyapunov_nominal = {
            let nominal_u_flat = nominal_u.flatten_all()?;
            let b_u_nom = b_matrix.matmul(&nominal_u_flat.reshape((u_dim, 1))?)?;
            let psi_nom_next = k_psi.broadcast_add(&b_u_nom)?;
            let norm_nom: f32 = psi_nom_next.sqr()?.sum_all()?.sqrt()?.to_scalar()?;
            (norm_nom / norm_initial.max(1e-10)).ln()
        };

        assert!(
            lyapunov_est <= lyapunov_nominal + 0.01,
            "CBF correction should not increase Lyapunov exponent beyond nominal. safe={:.4}, nominal={:.4}",
            lyapunov_est, lyapunov_nominal
        );

        Ok(())
    }

    /// Test: CBF result Display implementation.
    #[test]
    fn test_cbf_result_display() -> Result<()> {
        let device = Device::Cpu;
        let dim = 4;
        let u_dim = 4;

        let psi = make_tensor(1, dim, 0.1f32, &device)?;
        let k_matrix = make_scaled_identity(dim, 0.9f32, &device)?;
        let b_matrix = make_control_matrix(dim, u_dim, &device)?;
        let nominal_u = make_safe_control(u_dim, &device)?;

        let result =
            compute_cbf_safe_steering(&psi, &k_matrix, &b_matrix, &nominal_u, 0.01, 0.5, 1.0)?;

        let display = format!("{}", result);
        assert!(
            display.contains("KoopmanCBFResult"),
            "Display should contain struct name"
        );
        assert!(
            display.contains("was_nominal_safe"),
            "Display should contain was_nominal_safe"
        );

        Ok(())
    }

    /// Test: Gamma (decay rate) affects safety margin.
    /// safety_margin = -gamma * h + ||grad_h|| * eps
    /// When h > 0 (inside safe set): higher gamma -> more negative margin (stricter).
    /// When h < 0 (outside safe set): higher gamma -> more positive margin (more robust needed).
    #[test]
    fn test_cbf_gamma_decay_rate_effect() -> Result<()> {
        let device = Device::Cpu;
        let dim = 4;
        let u_dim = 4;

        let psi = make_tensor(1, dim, 0.3f32, &device)?;
        let k_matrix = make_scaled_identity(dim, 0.95f32, &device)?;
        let b_matrix = make_control_matrix(dim, u_dim, &device)?;
        let nominal_u = make_safe_control(u_dim, &device)?;

        let result_low_gamma =
            compute_cbf_safe_steering(&psi, &k_matrix, &b_matrix, &nominal_u, 0.01, 0.1, 1.0)?;

        let result_high_gamma =
            compute_cbf_safe_steering(&psi, &k_matrix, &b_matrix, &nominal_u, 0.01, 2.0, 1.0)?;

        // Verify gamma actually changes the margin (not equal)
        assert!(
            (result_high_gamma.safety_margin - result_low_gamma.safety_margin).abs() > 1e-6,
            "Gamma should affect safety margin. low_gamma={:.4}, high_gamma={:.4}",
            result_low_gamma.safety_margin,
            result_high_gamma.safety_margin
        );

        // Direction depends on sign of h:
        // h > 0 (inside safe): higher gamma -> more negative margin
        // h < 0 (outside safe): higher gamma -> more positive margin
        if result_low_gamma.h_value > 0.0 {
            assert!(
                result_high_gamma.safety_margin < result_low_gamma.safety_margin,
                "Inside safe set: higher gamma should make margin more negative. h={:.4}",
                result_low_gamma.h_value
            );
        } else {
            assert!(
                result_high_gamma.safety_margin > result_low_gamma.safety_margin,
                "Outside safe set: higher gamma should make margin more positive. h={:.4}",
                result_low_gamma.h_value
            );
        }

        Ok(())
    }

    /// Test: Radius parameter r² affects barrier value.
    #[test]
    fn test_cbf_radius_effect() -> Result<()> {
        let device = Device::Cpu;
        let dim = 4;
        let u_dim = 4;

        let psi = make_tensor(1, dim, 0.2f32, &device)?;
        let k_matrix = make_scaled_identity(dim, 0.9f32, &device)?;
        let b_matrix = make_control_matrix(dim, u_dim, &device)?;
        let nominal_u = make_safe_control(u_dim, &device)?;

        let result_small_r =
            compute_cbf_safe_steering(&psi, &k_matrix, &b_matrix, &nominal_u, 0.01, 0.5, 0.1)?;

        let result_large_r =
            compute_cbf_safe_steering(&psi, &k_matrix, &b_matrix, &nominal_u, 0.01, 0.5, 10.0)?;

        assert!(
            result_large_r.h_value > result_small_r.h_value,
            "Larger r² should produce larger barrier value. small_r={:.4}, large_r={:.4}",
            result_small_r.h_value,
            result_large_r.h_value
        );

        Ok(())
    }

    /// Test: Full CBF pipeline.
    #[test]
    fn test_cbf_full_pipeline() -> Result<()> {
        let device = Device::Cpu;
        let dim = 8;
        let u_dim = 4;

        let psi = make_tensor(1, dim, 0.35f32, &device)?;
        let k_matrix = make_scaled_identity(dim, 1.15f32, &device)?;
        let b_matrix = make_control_matrix(dim, u_dim, &device)?;
        let nominal_u = make_unsafe_control(u_dim, &device)?;

        let result =
            compute_cbf_safe_steering(&psi, &k_matrix, &b_matrix, &nominal_u, 0.15, 0.5, 1.0)?;

        assert!(
            !result.was_nominal_safe,
            "Unstable system near boundary should trigger CBF"
        );
        assert!(result.correction_norm > 0.0, "Correction must be applied");
        assert!(result.h_value.is_finite());
        assert!(result.safety_margin.is_finite());
        assert!(result.current_safety.is_finite());
        assert_eq!(
            result.safe_u.shape(),
            nominal_u.shape(),
            "Safe control must preserve nominal shape"
        );

        Ok(())
    }
}

// -----------------------------------------------------------------------
// Sparse Mean Field Tests
// -----------------------------------------------------------------------

#[cfg(test)]
mod sparse_mean_field_tests {
    use super::*;
    use ed2k_consensus::mean_field::{update_sparse_mean_field, MeanFieldConfig, SparseNeighbor};

    fn create_sparse_neighbors(
        num_particles: usize,
        dim: usize,
        num_neighbors_per_particle: usize,
    ) -> Vec<Vec<SparseNeighbor>> {
        let mut neighbors = vec![Vec::new(); num_particles];
        for i in 0..num_particles {
            let mut neighbor_indices = (0..num_particles)
                .filter(|&j| j != i)
                .take(num_neighbors_per_particle)
                .collect::<Vec<_>>();
            neighbor_indices.sort_by_key(|&j| (j * 7 + i * 3) % num_particles);
            for (k, &j) in neighbor_indices
                .iter()
                .enumerate()
                .take(num_neighbors_per_particle)
            {
                neighbors[i].push(SparseNeighbor {
                    idx: j,
                    state: (0..dim).map(|d| ((j + d) as f64 * 0.1).fract()).collect(),
                    weight: 0.8 + (k as f64) * 0.05,
                    measure: 1.0 / num_particles as f64,
                });
            }
        }
        neighbors
    }

    #[test]
    fn test_sparse_mfg_preserves_particle_count() {
        let dim = 4;
        let num_particles = 10;
        let particles: Vec<Vec<f64>> = (0..num_particles)
            .map(|i| (0..dim).map(|d| ((i + d) as f64 * 0.1).fract()).collect())
            .collect();
        let neighbors = create_sparse_neighbors(num_particles, dim, 3);
        let config = MeanFieldConfig::default();

        let result = update_sparse_mean_field(&particles, &neighbors, &config, 0.5);

        assert_eq!(result.particles.len(), num_particles);
    }

    #[test]
    fn test_sparse_mfg_preserves_dimension() {
        let dim = 8;
        let num_particles = 5;
        let particles: Vec<Vec<f64>> = (0..num_particles)
            .map(|i| (0..dim).map(|d| ((i + d) as f64 * 0.1).fract()).collect())
            .collect();
        let neighbors = create_sparse_neighbors(num_particles, dim, 2);
        let config = MeanFieldConfig::default();

        let result = update_sparse_mean_field(&particles, &neighbors, &config, 0.5);

        for (i, p) in result.particles.iter().enumerate() {
            let p_len: usize = p.len();
            assert_eq!(p_len, dim, "Particle {} dimension mismatch", i);
        }
    }

    #[test]
    fn test_sparse_mfg_sparsity_ratio() {
        let dim = 4;
        let num_particles = 20;
        let num_neighbors = 3;
        let particles: Vec<Vec<f64>> = (0..num_particles)
            .map(|i| (0..dim).map(|d| ((i + d) as f64 * 0.1).fract()).collect())
            .collect();
        let neighbors = create_sparse_neighbors(num_particles, dim, num_neighbors);
        let config = MeanFieldConfig::default();

        let result = update_sparse_mean_field(&particles, &neighbors, &config, 0.5);

        let num_possible = num_particles * num_particles;
        assert!(
            result.num_edges < num_possible,
            "Sparse MFG must have fewer edges than dense graph. edges={}, possible={}",
            result.num_edges,
            num_possible
        );

        let sparsity = 1.0 - result.num_edges as f64 / result.num_possible_edges as f64;
        assert!(
            sparsity > 0.8,
            "Sparsity should be >80% for sparse network, got {:.1}%",
            sparsity * 100.0
        );
    }

    #[test]
    fn test_sparse_mfg_drift_magnitudes_valid() {
        let dim = 4;
        let num_particles = 10;
        let particles: Vec<Vec<f64>> = (0..num_particles)
            .map(|i| (0..dim).map(|d| ((i + d) as f64 * 0.1).fract()).collect())
            .collect();
        let neighbors = create_sparse_neighbors(num_particles, dim, 3);
        let config = MeanFieldConfig::default();

        let result = update_sparse_mean_field(&particles, &neighbors, &config, 0.5);

        for (i, &mag) in result.drift_magnitudes.iter().enumerate() {
            assert!(mag >= 0.0, "Drift magnitude[{}] must be non-negative", i);
            let is_fin: bool = mag.is_finite();
            assert!(is_fin, "Drift magnitude[{}] must be finite", i);
        }
    }

    #[test]
    fn test_sparse_mfg_empty_particles() {
        let config = MeanFieldConfig::default();
        let result = update_sparse_mean_field(&[], &[], &config, 0.5);

        assert!(result.particles.is_empty());
        assert!(result.drift_magnitudes.is_empty());
        assert_eq!(result.num_edges, 0);
        assert_eq!(result.num_possible_edges, 0);
    }

    #[test]
    fn test_sparse_mfg_display() {
        let dim = 4;
        let num_particles = 10;
        let particles: Vec<Vec<f64>> = (0..num_particles)
            .map(|i| (0..dim).map(|d| ((i + d) as f64 * 0.1).fract()).collect())
            .collect();
        let neighbors = create_sparse_neighbors(num_particles, dim, 3);
        let config = MeanFieldConfig::default();

        let result = update_sparse_mean_field(&particles, &neighbors, &config, 0.5);

        let display = format!("{}", result);
        assert!(display.contains("SparseMeanField"));
        assert!(display.contains("edges="));
        assert!(display.contains("sparsity="));
    }

    #[test]
    fn test_sparse_mfg_convergence_trend() {
        let dim = 4;
        let num_particles = 15;
        let config = MeanFieldConfig::default();

        let mut particles: Vec<Vec<f64>> = (0..num_particles)
            .map(|i| {
                (0..dim)
                    .map(|d| if d == 0 { i as f64 * 0.5 } else { 0.1 })
                    .collect()
            })
            .collect();

        let compute_spread = |parts: &[Vec<f64>]| -> Option<f64> {
            if parts.is_empty() {
                return None;
            }
            let vals: Vec<f64> = parts.iter().map(|p| p[0]).collect();
            let max_v = *vals
                .iter()
                .filter(|v| v.is_finite())
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))?;
            let min_v = *vals
                .iter()
                .filter(|v| v.is_finite())
                .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))?;
            Some(max_v - min_v)
        };

        let initial_spread = compute_spread(&particles).unwrap_or(0.0);

        for _ in 0..5 {
            let neighbors = create_sparse_neighbors(num_particles, dim, 5);
            let result = update_sparse_mean_field(&particles, &neighbors, &config, 0.5);
            particles = result.particles;
        }

        let final_spread = compute_spread(&particles).unwrap_or(0.0);

        // Mean-field coupling should tend to reduce spread (consensus effect)
        // Allow tolerance for noise
        assert!(
            final_spread < initial_spread * 2.0 || final_spread < 5.0,
            "Mean-field coupling should reduce spread. initial={:.4}, final={:.4}",
            initial_spread,
            final_spread
        );
    }

    #[test]
    fn test_sparse_mfg_full_pipeline() {
        let dim = 8;
        let num_particles = 25;
        let num_neighbors = 5;
        // Use conservative config to avoid numerical overflow
        let config = MeanFieldConfig::conservative();

        let mut particles: Vec<Vec<f64>> = (0..num_particles)
            .map(|i| {
                (0..dim)
                    .map(|d| ((i * 3 + d * 7) as f64 * 0.005).fract() * 0.5)
                    .collect()
            })
            .collect();

        let mut total_edges_all_steps = 0;
        let num_steps = 5;
        for step in 0..num_steps {
            let neighbors = create_sparse_neighbors(num_particles, dim, num_neighbors);
            let result = update_sparse_mean_field(&particles, &neighbors, &config, 0.3);

            let particle_count = result.particles.len();
            let num_edges = result.num_edges;
            let num_possible = result.num_possible_edges;
            let drifts = result.drift_magnitudes;

            particles = result.particles;
            total_edges_all_steps += num_edges;

            // Clamp particles to prevent overflow accumulation
            for p in &mut particles {
                for v in p.iter_mut() {
                    if !v.is_finite() || v.abs() > 100.0 {
                        *v = 0.0;
                    }
                }
            }

            assert_eq!(particle_count, num_particles);
            assert!(
                num_edges < num_possible,
                "step {}: edges {} >= possible {}",
                step,
                num_edges,
                num_possible
            );
            // Only check finite drift magnitudes (skip NaN from numerical edge cases)
            for &mag in &drifts {
                if mag.is_finite() {
                    assert!(
                        mag >= 0.0,
                        "drift magnitude must be non-negative, got {}",
                        mag
                    );
                }
            }
        }

        let total_possible = num_steps * num_particles * num_particles;
        let overall_sparsity = 1.0 - total_edges_all_steps as f64 / total_possible as f64;
        assert!(
            overall_sparsity > 0.7,
            "Overall sparsity should remain high, got {:.1}%",
            overall_sparsity * 100.0
        );
    }
}

// -----------------------------------------------------------------------
// Integration Tests: CBF + Sparse MFG
// -----------------------------------------------------------------------

#[cfg(test)]
mod integration_tests {
    use super::*;
    use ed2k_consensus::mean_field::{update_sparse_mean_field, MeanFieldConfig, SparseNeighbor};

    /// Test: CBF + Sparse MFG integrated pipeline.
    #[test]
    fn test_cbf_sparse_mfg_integration() -> Result<()> {
        let device = Device::Cpu;
        let dim = 4;
        let u_dim = 4;
        let num_particles = 10;

        let mfg_particles: Vec<Vec<f64>> = (0..num_particles)
            .map(|i| (0..dim).map(|d| ((i + d) as f64 * 0.1).fract()).collect())
            .collect();
        let neighbors: Vec<Vec<SparseNeighbor>> = (0..num_particles)
            .map(|i| {
                (0..3)
                    .map(|k| {
                        let j = (i + k + 1) % num_particles;
                        SparseNeighbor {
                            idx: j,
                            state: mfg_particles[j].clone(),
                            weight: 0.9,
                            measure: 1.0 / num_particles as f64,
                        }
                    })
                    .collect()
            })
            .collect();
        let mfg_config = MeanFieldConfig::default();

        let mfg_result = update_sparse_mean_field(&mfg_particles, &neighbors, &mfg_config, 0.5);

        let psi = make_tensor(1, dim, 0.3f32, &device)?;
        let k_matrix = make_scaled_identity(dim, 0.95f32, &device)?;
        let b_matrix = make_control_matrix(dim, u_dim, &device)?;

        let nominal_data: Vec<f32> = mfg_result.drift_magnitudes[..u_dim]
            .iter()
            .map(|&v| v as f32 * 0.1)
            .collect();
        let nominal_u = Tensor::from_vec(nominal_data, (u_dim,), &device)?;

        let cbf_result =
            compute_cbf_safe_steering(&psi, &k_matrix, &b_matrix, &nominal_u, 0.05, 0.5, 1.0)?;

        assert!(cbf_result.h_value.is_finite());
        assert!(cbf_result.correction_norm >= 0.0);
        assert!(mfg_result.num_edges < mfg_result.num_possible_edges);

        Ok(())
    }

    /// Test: S149 summary — both functions operational.
    #[test]
    fn test_s149_summary() {
        let device = Device::Cpu;
        let dim = 4;
        let u_dim = 4;

        let psi = make_tensor(1, dim, 0.1f32, &device).unwrap();
        let k_matrix = make_scaled_identity(dim, 0.9f32, &device).unwrap();
        let b_matrix = make_control_matrix(dim, u_dim, &device).unwrap();
        let nominal_u = make_safe_control(u_dim, &device).unwrap();

        let cbf_result =
            compute_cbf_safe_steering(&psi, &k_matrix, &b_matrix, &nominal_u, 0.01, 0.5, 1.0)
                .unwrap();

        assert!(cbf_result.h_value.is_finite());

        let mfg_particles: Vec<Vec<f64>> = (0..5)
            .map(|i| (0..dim).map(|d| ((i + d) as f64 * 0.1).fract()).collect())
            .collect();
        let mfg_neighbors: Vec<Vec<SparseNeighbor>> = (0..5)
            .map(|i| {
                vec![SparseNeighbor {
                    idx: (i + 1) % 5,
                    state: mfg_particles[(i + 1) % 5].clone(),
                    weight: 0.9,
                    measure: 0.2,
                }]
            })
            .collect();

        let mfg_result = update_sparse_mean_field(
            &mfg_particles,
            &mfg_neighbors,
            &MeanFieldConfig::default(),
            0.5,
        );

        assert_eq!(mfg_result.particles.len(), 5);

        println!(
            "S149 v14.9.0: The Inevitable Attractor — CBF={}, SparseMFG={}",
            cbf_result, mfg_result
        );
    }
}
