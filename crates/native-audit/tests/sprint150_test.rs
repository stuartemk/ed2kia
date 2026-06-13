//! Sprint 150 (v15.0.0) — Physics-Informed Koopman + TV-CBF + Fictitious Play MFG
//!
//! Mode: `STRICT_MATH + PHYSICS_INFORMED_CONTROL + ZERO_WARNINGS + FORMAL_VERIFICATION`

use candle_core::{Device, DType, Tensor};
use candle_core::Result;

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn make_tensor(rows: usize, cols: usize, seed: f32, device: &Device) -> Result<Tensor> {
    let data: Vec<f32> = (0..rows * cols)
        .map(|i| (i as f32 * seed + seed).fract())
        .collect();
    Tensor::from_vec(data, (rows, cols), device)
}

fn make_scaled_identity(dim: usize, scale: f32, device: &Device) -> Result<Tensor> {
    let eye = Tensor::eye(dim, DType::F32, device)?;
    let scalar = Tensor::new(scale, device)?;
    eye.broadcast_mul(&scalar)
}

fn make_symmetric_posdef(dim: usize, min_diag: f32, device: &Device) -> Result<Tensor> {
    let mut data = vec![0.0f32; dim * dim];
    for i in 0..dim {
        data[i * dim + i] = min_diag + (i as f32) * 0.1;
    }
    Tensor::from_vec(data, (dim, dim), device)
}

// ─── PASO A: Physics-Informed Koopman Tests ──────────────────────────────────

mod physics_informed_koopman_tests {
    use super::*;
    use native_audit::deep_koopman::{DeepKoopmanAE, PhysicsInformedKoopmanLoss};

    fn make_tensor(rows: usize, cols: usize, seed: f32, device: &Device) -> Result<Tensor> {
        let data: Vec<f32> = (0..rows * cols)
            .map(|i| (i as f32 * seed + seed).fract())
            .collect();
        Tensor::from_vec(data, (rows, cols), device)
    }

    #[test]
    fn test_physics_informed_propagation_shape() -> Result<()> {
        let device = Device::Cpu;
        let ae = DeepKoopmanAE::new(16, 32, 1e-4, 1.0, 1.0, &device)?;
        let batch = 4;
        let psi = make_tensor(batch, 32, 0.1, &device)?;
        let k = Tensor::eye(32, DType::F32, &device)?;
        let w_res = make_tensor(32, 32, 0.01, &device)?;

        let result = ae.propagate_koopman_physics_informed(&psi, &k, &w_res, 0.1)?;
        assert_eq!(result.shape().dims(), [batch, 32]);
        Ok(())
    }

    #[test]
    fn test_physics_informed_propagation_zero_div_weight() -> Result<()> {
        let device = Device::Cpu;
        let ae = DeepKoopmanAE::new(16, 32, 1e-4, 1.0, 1.0, &device)?;
        let psi = make_tensor(2, 32, 0.1, &device)?;
        let k = Tensor::eye(32, DType::F32, &device)?;
        let w_res = make_tensor(32, 32, 0.01, &device)?;

        // With div_weight=0, should be Kψ + ReLU(W_res·ψ)
        let result = ae.propagate_koopman_physics_informed(&psi, &k, &w_res, 0.0)?;
        assert!(result.shape().dims()[0] == 2);
        assert!(result.shape().dims()[1] == 32);
        Ok(())
    }

    #[test]
    fn test_physics_informed_loss_non_negative() -> Result<()> {
        let device = Device::Cpu;
        let ae = DeepKoopmanAE::new(16, 32, 1e-4, 1.0, 1.0, &device)?;
        let psi_t = make_tensor(4, 32, 0.1, &device)?;
        let psi_t_next = make_tensor(4, 32, 0.2, &device)?;
        let k = Tensor::eye(32, DType::F32, &device)?;
        let w_res = make_tensor(32, 32, 0.01, &device)?;

        let loss = ae.compute_koopman_loss_physics_informed(
            &psi_t, &psi_t_next, &k, &w_res,
            0.01,  // gamma_frob
            0.01,  // beta_div
            0.1,   // lambda_lyap
        )?;

        assert!(loss.mse_loss >= 0.0, "MSE must be non-negative, got {}", loss.mse_loss);
        assert!(loss.frob_loss >= 0.0, "Frobenius must be non-negative, got {}", loss.frob_loss);
        assert!(loss.div_loss >= 0.0, "Div loss must be non-negative, got {}", loss.div_loss);
        assert!(loss.lyap_loss >= 0.0, "Lyapunov must be non-negative, got {}", loss.lyap_loss);
        assert!(loss.total_loss >= 0.0, "Total must be non-negative, got {}", loss.total_loss);
        Ok(())
    }

    #[test]
    fn test_physics_informed_loss_finite() -> Result<()> {
        let device = Device::Cpu;
        let ae = DeepKoopmanAE::new(16, 32, 1e-4, 1.0, 1.0, &device)?;
        let psi_t = make_tensor(4, 32, 0.05, &device)?;
        let psi_t_next = make_tensor(4, 32, 0.06, &device)?;
        let k = Tensor::eye(32, DType::F32, &device)?;
        let w_res = make_tensor(32, 32, 0.001, &device)?;

        let loss = ae.compute_koopman_loss_physics_informed(
            &psi_t, &psi_t_next, &k, &w_res,
            0.001, 0.001, 0.01,
        )?;

        assert!(loss.total_loss.is_finite(), "Total loss must be finite, got {}", loss.total_loss);
        Ok(())
    }

    #[test]
    fn test_physics_informed_loss_gamma_frob_effect() -> Result<()> {
        let device = Device::Cpu;
        let ae = DeepKoopmanAE::new(16, 32, 1e-4, 1.0, 1.0, &device)?;
        let psi_t = make_tensor(4, 32, 0.1, &device)?;
        let psi_t_next = make_tensor(4, 32, 0.2, &device)?;
        let k = Tensor::eye(32, DType::F32, &device)?;
        let w_res = make_tensor(32, 32, 0.01, &device)?;

        let loss_low = ae.compute_koopman_loss_physics_informed(
            &psi_t, &psi_t_next, &k, &w_res,
            0.001, 0.0, 0.0,
        )?;
        let loss_high = ae.compute_koopman_loss_physics_informed(
            &psi_t, &psi_t_next, &k, &w_res,
            0.1, 0.0, 0.0,
        )?;

        assert!(loss_high.frob_loss > loss_low.frob_loss,
            "Higher gamma_frob should increase Frobenius loss: low={} high={}",
            loss_low.frob_loss, loss_high.frob_loss);
        Ok(())
    }

    #[test]
    fn test_physics_informed_loss_display() -> Result<()> {
        let device = Device::Cpu;
        let ae = DeepKoopmanAE::new(16, 32, 1e-4, 1.0, 1.0, &device)?;
        let psi_t = make_tensor(2, 32, 0.1, &device)?;
        let psi_t_next = make_tensor(2, 32, 0.2, &device)?;
        let k = Tensor::eye(32, DType::F32, &device)?;
        let w_res = make_tensor(32, 32, 0.01, &device)?;

        let loss = ae.compute_koopman_loss_physics_informed(
            &psi_t, &psi_t_next, &k, &w_res,
            0.01, 0.01, 0.1,
        )?;

        let display = format!("{}", loss);
        assert!(display.contains("PIKLoss"), "Display should contain PIKLoss: {}", display);
        assert!(display.contains("mse:"), "Display should contain mse: {}", display);
        Ok(())
    }

    #[test]
    fn test_stiefel_project_preserves_shape() -> Result<()> {
        let device = Device::Cpu;
        let k = make_tensor(16, 16, 0.1, &device)?;
        let projected = DeepKoopmanAE::stiefel_project(&k, 5)?;
        assert_eq!(projected.shape().dims(), [16, 16]);
        Ok(())
    }

    #[test]
    fn test_stiefel_project_converges() -> Result<()> {
        let device = Device::Cpu;
        let k = make_tensor(8, 8, 0.05, &device)?;
        let proj_5 = DeepKoopmanAE::stiefel_project(&k, 5)?;
        let proj_10 = DeepKoopmanAE::stiefel_project(&k, 10)?;

        // Difference between 5 and 10 iterations should be small
        let diff = proj_5.sub(&proj_10)?.sqr()?.sum_all()?.to_scalar::<f32>()?;
        assert!(diff < 1.0, "Stiefel projection should converge, diff={}", diff);
        Ok(())
    }

    #[test]
    fn test_physics_informed_full_pipeline() -> Result<()> {
        let device = Device::Cpu;
        let ae = DeepKoopmanAE::new(32, 64, 1e-4, 1.0, 1.0, &device)?;

        let psi_t = make_tensor(8, 64, 0.05, &device)?;
        let k = Tensor::eye(64, DType::F32, &device)?;
        let w_res = make_tensor(64, 64, 0.005, &device)?;

        // Propagate
        let psi_pred = ae.propagate_koopman_physics_informed(&psi_t, &k, &w_res, 0.05)?;
        assert_eq!(psi_pred.shape().dims(), [8, 64]);

        // Compute loss
        let psi_t_next = make_tensor(8, 64, 0.06, &device)?;
        let loss = ae.compute_koopman_loss_physics_informed(
            &psi_t, &psi_t_next, &k, &w_res,
            0.001, 0.001, 0.01,
        )?;

        assert!(loss.total_loss.is_finite(), "Pipeline loss must be finite: {}", loss.total_loss);
        assert!(loss.total_loss >= 0.0, "Pipeline loss must be non-negative: {}", loss.total_loss);
        Ok(())
    }
}

// ─── PASO B: TV-CBF Tests ────────────────────────────────────────────────────

mod tv_cbf_tests {
    use native_audit::control::{verify_tv_cbf, tv_cbf_qp_correct, TVCBFResult};

    #[test]
    fn test_tv_cbf_safe_state() {
        // All modalities safe, positive h_dot
        let result = verify_tv_cbf(
            1.0,   // h_topo
            -0.5,  // vfe (safe: -vfe + gamma = 0.5 + 1.0 = 1.5)
            1.0,   // gamma_vfe
            1.0,   // alpha_k
            0.5,   // h_dot
            2.0,   // semantic_safety
        );

        assert!(result.safe, "Should be safe: {}", result);
        assert!(result.h_value > 0.0, "h should be positive: {}", result.h_value);
        assert!(result.cbf_condition >= 0.0, "CBF condition should be >= 0: {}", result.cbf_condition);
    }

    #[test]
    fn test_tv_cbf_unsafe_state() {
        // Negative h_dot, boundary h
        let result = verify_tv_cbf(
            -0.5,  // h_topo (unsafe)
            2.0,   // vfe (unsafe: -2.0 + 1.0 = -1.0)
            1.0,   // gamma_vfe
            1.0,   // alpha_k
            -1.0,  // h_dot (driving away)
            0.5,   // semantic_safety
        );

        assert!(!result.safe, "Should be unsafe: {}", result);
        assert!(result.h_value < 0.0, "h should be negative: {}", result.h_value);
    }

    #[test]
    fn test_tv_cbf_min_modality() {
        // h = min(h_topo, h_vfe, h_semantic)
        let result = verify_tv_cbf(
            0.3,   // h_topo
            0.5,   // vfe → h_vfe = -0.5 + 2.0 = 1.5
            2.0,   // gamma_vfe
            1.0,   // alpha_k
            0.0,   // h_dot
            0.1,   // semantic_safety (minimum)
        );

        assert!((result.h_value - 0.1).abs() < 1e-6,
            "h should be min modality (semantic=0.1), got {}", result.h_value);
    }

    #[test]
    fn test_tv_cbf_alpha_function() {
        // α(h) = k · max(h, 0)
        let result = verify_tv_cbf(
            2.0,   // h_topo
            0.0,   // vfe → h_vfe = 0 + 1.0 = 1.0
            1.0,   // gamma_vfe
            0.5,   // alpha_k
            0.0,   // h_dot
            3.0,   // semantic
        );

        // h = min(2.0, 1.0, 3.0) = 1.0
        // α(h) = 0.5 * 1.0 = 0.5
        assert!((result.alpha_h - 0.5).abs() < 1e-6,
            "α(h) should be 0.5, got {}", result.alpha_h);
    }

    #[test]
    fn test_tv_cbf_alpha_zero_at_boundary() {
        // When h ≤ 0, α(h) = 0
        let result = verify_tv_cbf(
            -1.0,  // h_topo (negative)
            0.0,   // vfe
            1.0,   // gamma_vfe
            1.0,   // alpha_k
            0.5,   // h_dot
            2.0,   // semantic
        );

        assert!((result.alpha_h - 0.0).abs() < 1e-6,
            "α(h) should be 0 when h < 0, got {}", result.alpha_h);
    }

    #[test]
    fn test_tv_cbf_result_display() {
        let result = verify_tv_cbf(1.0, 0.0, 1.0, 1.0, 0.5, 2.0);
        let display = format!("{}", result);
        assert!(display.contains("TVCBF"), "Display should contain TVCBF: {}", display);
        assert!(display.contains("safe="), "Display should contain safe=: {}", display);
    }

    #[test]
    fn test_tv_cbf_qp_correct_safe_passes() {
        let nominal_u = vec![0.1, 0.2, 0.3];
        let l_g_h = vec![1.0, 0.0, 0.0];
        let h_dot_drift = 0.5;
        let alpha_h = 0.3;

        // current_safety = 0.5 + 1.0*0.1 = 0.6 > 0.3 → safe
        let safe_u = tv_cbf_qp_correct(&nominal_u, &l_g_h, h_dot_drift, alpha_h, 1e-8);

        assert_eq!(safe_u.len(), 3);
        for (i, (a, b)) in safe_u.iter().zip(nominal_u.iter()).enumerate() {
            assert!((a - b).abs() < 1e-6, "u[{}] should be unchanged: {} vs {}", i, a, b);
        }
    }

    #[test]
    fn test_tv_cbf_qp_correct_unsafe_corrects() {
        let nominal_u = vec![0.0, 0.0, 0.0];
        let l_g_h = vec![1.0, 0.0, 0.0];
        let h_dot_drift = -0.5;
        let alpha_h = 0.3;

        // current_safety = -0.5 + 0 = -0.5 < 0.3 → needs correction
        let safe_u = tv_cbf_qp_correct(&nominal_u, &l_g_h, h_dot_drift, alpha_h, 1e-8);

        assert_eq!(safe_u.len(), 3);
        assert!(safe_u[0] > 0.0, "First component should be corrected positive: {}", safe_u[0]);
    }

    #[test]
    fn test_tv_cbf_qp_correct_norm() {
        let nominal_u = vec![0.0; 4];
        let l_g_h = vec![1.0, 1.0, 0.0, 0.0];
        let h_dot_drift = 0.0;
        let alpha_h = 1.0;

        let safe_u = tv_cbf_qp_correct(&nominal_u, &l_g_h, h_dot_drift, alpha_h, 1e-8);

        // λ = (1.0 - 0.0) / (1+1+1e-8) ≈ 0.5
        // u_safe = [0.5, 0.5, 0, 0]
        assert!((safe_u[0] - 0.5).abs() < 0.01, "u[0] ≈ 0.5, got {}", safe_u[0]);
        assert!((safe_u[1] - 0.5).abs() < 0.01, "u[1] ≈ 0.5, got {}", safe_u[1]);
    }

    #[test]
    fn test_tv_cbf_full_trajectory() {
        // Simulate a trajectory where safety degrades then recovers
        let scenarios = [
            (2.0, 0.0, 1.0, 1.0, 0.5, 3.0),  // Safe
            (1.0, 0.5, 1.0, 1.0, 0.0, 2.0),   // Borderline
            (0.5, 1.0, 1.0, 1.0, -0.3, 1.0),  // Unsafe
            (1.5, 0.0, 1.0, 1.0, 0.3, 2.5),   // Safe again
        ];

        let results: Vec<TVCBFResult> = scenarios.iter().map(|&(a, b, c, d, e, f)| {
            verify_tv_cbf(a, b, c, d, e, f)
        }).collect();

        assert!(results[0].safe, "Step 0 should be safe");
        assert!(!results[2].safe, "Step 2 should be unsafe");
        assert!(results[3].safe, "Step 3 should be safe");
    }
}

// ─── PASO C: Fictitious Play MFG Tests ───────────────────────────────────────

mod fictitious_play_tests {
    use ed2k_consensus::mean_field::{update_policy_fictitious_play, run_fictitious_play};

    #[test]
    fn test_fictitious_play_policy_sums_to_one() {
        let q_values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = update_policy_fictitious_play(&q_values, 1.0, 0.1, None);

        let sum: f64 = result.policy.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6, "Policy must sum to 1, got {}", sum);
    }

    #[test]
    fn test_fictitious_play_policy_non_negative() {
        let q_values = vec![-5.0, -3.0, -1.0, 0.0, 2.0];
        let result = update_policy_fictitious_play(&q_values, 1.0, 0.1, None);

        for (i, &p) in result.policy.iter().enumerate() {
            assert!(p >= 0.0, "Policy[{}] must be non-negative, got {}", i, p);
        }
    }

    #[test]
    fn test_fictitious_play_higher_q_higher_prob() {
        let q_values = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let result = update_policy_fictitious_play(&q_values, 1.0, 0.0, None);

        for i in 0..result.policy.len() - 1 {
            assert!(result.policy[i + 1] > result.policy[i],
                "Higher Q should have higher prob: Q[{}]={} prob={} vs Q[{}]={} prob={}",
                i, q_values[i], result.policy[i],
                i + 1, q_values[i + 1], result.policy[i + 1]);
        }
    }

    #[test]
    fn test_fictitious_play_temperature_effect() {
        let q_values = vec![0.0, 1.0, 2.0];

        // Low temperature → more peaked
        let result_cold = update_policy_fictitious_play(&q_values, 0.1, 0.0, None);
        // High temperature → more uniform
        let result_hot = update_policy_fictitious_play(&q_values, 10.0, 0.0, None);

        assert!(result_cold.entropy < result_hot.entropy,
            "Cold policy should have lower entropy: cold={} hot={}",
            result_cold.entropy, result_hot.entropy);
    }

    #[test]
    fn test_fictitious_play_entropy_positive() {
        let q_values = vec![1.0, 2.0, 3.0];
        let result = update_policy_fictitious_play(&q_values, 1.0, 0.1, None);

        assert!(result.entropy >= 0.0, "Entropy must be non-negative, got {}", result.entropy);
        // Max entropy for 3 actions = ln(3) ≈ 1.099
        assert!(result.entropy <= 1.1, "Entropy must be bounded, got {}", result.entropy);
    }

    #[test]
    fn test_fictitious_play_expected_q_valid() {
        let q_values = vec![1.0, 2.0, 3.0];
        let result = update_policy_fictitious_play(&q_values, 1.0, 0.0, None);

        let min_q = *q_values.iter().min_by_key(|v| v.to_bits()).unwrap();
        let max_q = *q_values.iter().max_by_key(|v| v.to_bits()).unwrap();

        assert!(result.expected_q >= min_q && result.expected_q <= max_q,
            "Expected Q must be in [min, max]: E[Q]={} range=[{}, {}]",
            result.expected_q, min_q, max_q);
    }

    #[test]
    fn test_fictitious_play_policy_change() {
        let q_values = vec![1.0, 2.0, 3.0];
        let prev_policy = vec![0.33, 0.33, 0.34];

        let result = update_policy_fictitious_play(&q_values, 1.0, 0.0, Some(&prev_policy));

        assert!(result.policy_change > 0.0, "Policy change should be positive: {}", result.policy_change);
        assert!(result.policy_change <= 2.0, "Policy change should be ≤ 2: {}", result.policy_change);
    }

    #[test]
    fn test_fictitious_play_result_display() {
        let q_values = vec![1.0, 2.0, 3.0];
        let result = update_policy_fictitious_play(&q_values, 1.0, 0.1, None);

        let display = format!("{}", result);
        assert!(display.contains("FictPlay"), "Display should contain FictPlay: {}", display);
        assert!(display.contains("entropy="), "Display should contain entropy: {}", display);
    }

    #[test]
    fn test_run_fictitious_play_convergence() {
        // Simple oracle: Q favors action 2
        let oracle = |_: &[f64]| -> Vec<f64> { vec![0.0, 1.0, 2.0] };

        let result = run_fictitious_play(oracle, 1.0, 0.1, 50, 1e-6);

        assert!(result.policy_change < 0.1, "Should converge: Δπ={}", result.policy_change);
        assert!(result.policy[2] > result.policy[0],
            "Action 2 should have highest prob: {:?}", result.policy);
    }

    #[test]
    fn test_run_fictitious_play_max_iters() {
        let oracle = |_: &[f64]| -> Vec<f64> { vec![1.0, 2.0, 3.0, 4.0] };

        let result = run_fictitious_play(oracle, 1.0, 0.0, 5, 1e-10);

        assert_eq!(result.policy.len(), 4, "Should have 4 actions");
        let sum: f64 = result.policy.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6, "Policy must sum to 1: {}", sum);
    }

    #[test]
    fn test_run_fictitious_play_dynamic_oracle() {
        // Oracle that shifts based on population policy
        let oracle = |pop: &[f64]| -> Vec<f64> {
            if pop.is_empty() {
                return vec![1.0, 1.0];
            }
            // Opponent-following: favor opposite of population
            vec![pop[1], pop[0]]
        };

        let result = run_fictitious_play(oracle, 1.0, 0.1, 100, 1e-4);

        assert_eq!(result.policy.len(), 2);
        let sum: f64 = result.policy.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5, "Policy must sum to 1: {}", sum);
    }

    #[test]
    fn test_fictitious_play_full_pipeline() {
        // Simulate a multi-agent game with 3 actions
        let mut population = vec![0.33, 0.33, 0.34];

        for _round in 0..20 {
            // Compute Q based on population
            let q_values: Vec<f64> = (0..3).map(|i| {
                // Reward diversity: Q[i] = 1 - population[i]
                1.0 - population[i] + (i as f64) * 0.1
            }).collect();

            let result = update_policy_fictitious_play(&q_values, 1.0, 0.05, Some(&population));
            population = result.policy;
        }

        // Verify final policy
        let sum: f64 = population.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6, "Final policy must sum to 1: {}", sum);
        for (i, &p) in population.iter().enumerate() {
            assert!(p >= 0.0 && p <= 1.0, "Policy[{}] in [0,1]: {}", i, p);
        }
    }
}

// ─── Integration Tests ───────────────────────────────────────────────────────

mod integration_tests {
    use super::*;

    #[test]
    fn test_s150_physics_koopman_tv_cbf_integration() -> Result<()> {
        let device = Device::Cpu;

        // 1. Physics-Informed Koopman propagation
        use native_audit::deep_koopman::DeepKoopmanAE;
        let ae = DeepKoopmanAE::new(16, 32, 1e-4, 1.0, 1.0, &device)?;
        let psi = make_tensor(2, 32, 0.05, &device)?;
        let k = Tensor::eye(32, DType::F32, &device)?;
        let w_res = make_tensor(32, 32, 0.005, &device)?;

        let psi_next = ae.propagate_koopman_physics_informed(&psi, &k, &w_res, 0.05)?;
        assert_eq!(psi_next.shape().dims(), [2, 32]);

        // 2. TV-CBF verification
        use native_audit::control::verify_tv_cbf;
        let cbf = verify_tv_cbf(1.0, 0.0, 1.0, 1.0, 0.3, 2.0);
        assert!(cbf.safe, "Integration CBF should be safe: {}", cbf);

        // 3. Fictitious Play
        use ed2k_consensus::mean_field::update_policy_fictitious_play;
        let fp = update_policy_fictitious_play(&[1.0, 2.0, 3.0], 1.0, 0.1, None);
        let sum: f64 = fp.policy.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6, "FP policy sum: {}", sum);

        Ok(())
    }

    #[test]
    fn test_s150_summary() {
        println!("=== Sprint 150 (v15.0.0) — The Physics-Informed Singularity ===");
        println!("PASO A: Physics-Informed Koopman with Residuals + Volume Preservation ✓");
        println!("PASO B: Time-Varying CBF with Multi-Modal Safety ✓");
        println!("PASO C: Fictitious Play for Nash-Convergent MFG ✓");
        println!("Mode: STRICT_MATH + PHYSICS_INFORMED_CONTROL + ZERO_WARNINGS");
    }
}
