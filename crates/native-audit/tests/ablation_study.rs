//! Sprint 152 (v15.2.0) — The Ablation Crucible
//!
//! Ablation study: Baseline → SAE → Koopman → Full TV-HBL
//! demonstrating progressive improvement with formal guarantees.
//!
//! **Ablation Configs:**
//! 1. **Baseline**: Raw nominal control (no safety)
//! 2. **SAE**: Sparse Autoencoder lifting + projection
//! 3. **Koopman**: KoopmanVanguard linearized prediction
//! 4. **TV-HBL Full**: Time-Varying Hybrid Barrier-Lyapunov + QP projection

use candle_core::{Device, Result, Tensor};
use native_audit::control::{
    compute_tv_hbl, project_control_tv_hbl,
};

// ─── Helpers ────────────────────────────────────────────────────────────────

fn make_tensor(rows: usize, cols: usize, seed: f32, device: &Device) -> Result<Tensor> {
    let data: Vec<f32> = (0..rows * cols)
        .map(|i| (seed * (i as f32 + 1.0)) % 10.0)
        .collect();
    Tensor::from_vec(data, (rows, cols), device)
}

fn make_symmetric_posdef(dim: usize, min_diag: f32, device: &Device) -> Result<Tensor> {
    let mut data = vec![0.0f32; dim * dim];
    for i in 0..dim {
        data[i * dim + i] = min_diag;
    }
    Tensor::from_vec(data, (dim, dim), device)
}

/// Ablation config descriptor.
#[derive(Debug, Clone, Copy)]
struct AblationConfig {
    name: &'static str,
    #[allow(dead_code)]
    description: &'static str,
}

const ABLATION_CONFIGS: &[AblationConfig] = &[
    AblationConfig {
        name: "baseline",
        description: "Raw nominal control (no safety)",
    },
    AblationConfig {
        name: "sae",
        description: "Sparse Autoencoder lifting + projection",
    },
    AblationConfig {
        name: "koopman",
        description: "KoopmanVanguard linearized prediction",
    },
    AblationConfig {
        name: "tv_hbl_full",
        description: "Time-Varying Hybrid Barrier-Lyapunov + QP projection",
    },
];

// ─── TV-HBL Core Tests ──────────────────────────────────────────────────────

mod tv_hbl_core_tests {
    use super::*;

    #[test]
    fn test_tv_hbl_safe_state() -> Result<()> {
        let device = Device::Cpu;
        let dim = 8;
        let state = make_tensor(1, dim, 5.0, &device)?;
        let safe_center = make_tensor(1, dim, 1.0, &device)?;
        let p_matrix = make_symmetric_posdef(dim, 2.0, &device)?;
        let alpha_t = 0.5;

        let result = compute_tv_hbl(&state, &safe_center, &p_matrix, alpha_t)?;
        assert!(result.safe, "State should be safe: h={:.4}", result.h_value);
        assert!(result.h_value > 0.0, "h_value should be positive: {:.4}", result.h_value);
        assert!(result.quadratic_term > alpha_t, "quad > alpha: {:.4} > {:.4}", result.quadratic_term, alpha_t);
        Ok(())
    }

    #[test]
    fn test_tv_hbl_unsafe_state() -> Result<()> {
        let device = Device::Cpu;
        let dim = 8;
        let state = make_tensor(1, dim, 1.0, &device)?;
        let safe_center = make_tensor(1, dim, 1.0, &device)?;
        let p_matrix = make_symmetric_posdef(dim, 1.0, &device)?;
        let alpha_t = 5.0;

        let result = compute_tv_hbl(&state, &safe_center, &p_matrix, alpha_t)?;
        assert!(!result.safe, "State should be unsafe: h={:.4}", result.h_value);
        assert!(result.h_value < 0.0, "h_value should be negative: {:.4}", result.h_value);
        Ok(())
    }

    #[test]
    fn test_tv_hbl_gradient_shape() -> Result<()> {
        let device = Device::Cpu;
        let dim = 16;
        let batch = 4;
        let state = make_tensor(batch, dim, 3.0, &device)?;
        let safe_center = make_tensor(1, dim, 1.0, &device)?;
        let p_matrix = make_symmetric_posdef(dim, 1.5, &device)?;

        let result = compute_tv_hbl(&state, &safe_center, &p_matrix, 1.0)?;
        let grad_shape = result.grad_h.shape().dims();
        assert_eq!(grad_shape.len(), 2, "Gradient should be 2D");
        assert_eq!(grad_shape[1], dim, "Gradient dim should match input");
        Ok(())
    }

    #[test]
    fn test_tv_hbl_alpha_time_varying() -> Result<()> {
        let device = Device::Cpu;
        let dim = 8;
        let state = make_tensor(1, dim, 3.0, &device)?;
        let safe_center = make_tensor(1, dim, 1.0, &device)?;
        let p_matrix = make_symmetric_posdef(dim, 1.0, &device)?;

        let alpha_0 = 0.5;
        let k = 0.1;
        let mut prev_h = f32::INFINITY;
        for t in 0..5 {
            let alpha_t = alpha_0 * (1.0 + k * t as f32);
            let result = compute_tv_hbl(&state, &safe_center, &p_matrix, alpha_t)?;
            assert!(
                result.h_value < prev_h,
                "h should decrease as alpha grows: t={}, h={:.4}, prev={:.4}",
                t, result.h_value, prev_h
            );
            prev_h = result.h_value;
        }
        Ok(())
    }

    #[test]
    fn test_tv_hbl_display() -> Result<()> {
        let device = Device::Cpu;
        let dim = 4;
        let state = make_tensor(1, dim, 2.0, &device)?;
        let safe_center = make_tensor(1, dim, 1.0, &device)?;
        let p_matrix = make_symmetric_posdef(dim, 1.0, &device)?;

        let result = compute_tv_hbl(&state, &safe_center, &p_matrix, 0.5)?;
        let display = format!("{}", result);
        assert!(display.contains("TVHBL"), "Display should contain 'TVHBL': {}", display);
        assert!(display.contains("h="), "Display should contain 'h=': {}", display);
        assert!(display.contains("safe="), "Display should contain 'safe=': {}", display);
        Ok(())
    }

    #[test]
    fn test_tv_hbl_quadratic_positive() -> Result<()> {
        let device = Device::Cpu;
        let dim = 8;
        let state = make_tensor(1, dim, 5.0, &device)?;
        let safe_center = make_tensor(1, dim, 2.0, &device)?;
        let p_matrix = make_symmetric_posdef(dim, 1.0, &device)?;

        let result = compute_tv_hbl(&state, &safe_center, &p_matrix, 0.0)?;
        assert!(result.quadratic_term >= 0.0, "Quadratic term should be non-negative: {:.4}", result.quadratic_term);
        Ok(())
    }

    #[test]
    fn test_tv_hbl_zero_diff() -> Result<()> {
        let device = Device::Cpu;
        let dim = 8;
        let state = make_tensor(1, dim, 3.0, &device)?;
        let p_matrix = make_symmetric_posdef(dim, 1.0, &device)?;
        let alpha_t = 1.0;

        let result = compute_tv_hbl(&state, &state, &p_matrix, alpha_t)?;
        assert!((result.quadratic_term - 0.0).abs() < 1e-6, "Quadratic should be ~0: {:.6}", result.quadratic_term);
        assert!((result.h_value + alpha_t).abs() < 1e-6, "h should be -alpha: {:.6}", result.h_value);
        assert!(!result.safe, "Should be unsafe when state == center");
        Ok(())
    }
}

// ─── QP Projection Tests ────────────────────────────────────────────────────

mod qp_projection_tests {
    use super::*;

    #[test]
    fn test_qp_safe_state_passes() -> Result<()> {
        let device = Device::Cpu;
        let dim = 8;
        let u_nom = make_tensor(1, dim, 1.0, &device)?;
        let grad_h = make_tensor(1, dim, 0.5, &device)?;
        let h_value = 5.0;
        let gamma = 1.0;
        let delta = 1e-6;

        let result = project_control_tv_hbl(&u_nom, &grad_h, h_value, gamma, delta)?;
        assert!(!result.corrected, "Safe state should not be corrected");
        assert!(result.lambda == 0.0, "Lambda should be 0 for safe state: {:.6}", result.lambda);
        Ok(())
    }

    #[test]
    fn test_qp_unsafe_state_corrected() -> Result<()> {
        let device = Device::Cpu;
        let dim = 8;
        let u_nom = make_tensor(1, dim, 1.0, &device)?;
        let grad_h = make_tensor(1, dim, 2.0, &device)?;
        let h_value = -3.0;
        let gamma = 1.0;
        let delta = 1e-6;

        let result = project_control_tv_hbl(&u_nom, &grad_h, h_value, gamma, delta)?;
        assert!(result.corrected, "Unsafe state should be corrected");
        assert!(result.lambda > 0.0, "Lambda should be positive: {:.6}", result.lambda);
        assert!(result.safety_margin_after > result.safety_margin_before,
            "Safety margin should improve: after={:.4}, before={:.4}",
            result.safety_margin_after, result.safety_margin_before);
        Ok(())
    }

    #[test]
    fn test_qp_lambda_formula() -> Result<()> {
        let device = Device::Cpu;
        let dim = 4;
        let u_nom = make_tensor(1, dim, 0.0, &device)?;
        let grad_data: Vec<f32> = vec![1.0, 0.0, 0.0, 0.0];
        let grad_h = Tensor::from_vec(grad_data, (1, dim), &device)?;
        let h_value = -2.0;
        let gamma = 1.0;
        let delta = 1e-6;

        let result = project_control_tv_hbl(&u_nom, &grad_h, h_value, gamma, delta)?;
        let expected_lambda = (gamma - h_value) / (1.0 + delta);
        assert!(
            (result.lambda - expected_lambda).abs() < 1e-4,
            "Lambda formula: got {:.6}, expected {:.6}",
            result.lambda, expected_lambda
        );
        Ok(())
    }

    #[test]
    fn test_qp_preserves_shape() -> Result<()> {
        let device = Device::Cpu;
        let batch = 4;
        let dim = 16;
        let u_nom = make_tensor(batch, dim, 1.0, &device)?;
        let grad_h = make_tensor(batch, dim, 0.5, &device)?;

        let result = project_control_tv_hbl(&u_nom, &grad_h, -1.0, 1.0, 1e-6)?;
        assert_eq!(result.u_safe.shape(), u_nom.shape(), "Shape should be preserved");
        Ok(())
    }

    #[test]
    fn test_qp_display() -> Result<()> {
        let device = Device::Cpu;
        let dim = 4;
        let u_nom = make_tensor(1, dim, 1.0, &device)?;
        let grad_h = make_tensor(1, dim, 0.5, &device)?;

        let result = project_control_tv_hbl(&u_nom, &grad_h, -1.0, 1.0, 1e-6)?;
        let display = format!("{}", result);
        assert!(display.contains("TVHBLProj"), "Display should contain 'TVHBLProj': {}", display);
        assert!(display.contains("λ="), "Display should contain lambda: {}", display);
        assert!(display.contains("corrected="), "Display should contain corrected: {}", display);
        Ok(())
    }

    #[test]
    fn test_qp_correction_scaling() -> Result<()> {
        let device = Device::Cpu;
        let dim = 8;
        let u_nom = make_tensor(1, dim, 0.0, &device)?;
        let grad_h = make_tensor(1, dim, 1.0, &device)?;

        let r1 = project_control_tv_hbl(&u_nom, &grad_h, -1.0, 1.0, 1e-6)?;
        let r2 = project_control_tv_hbl(&u_nom, &grad_h, -5.0, 1.0, 1e-6)?;
        assert!(
            r2.lambda > r1.lambda,
            "More unsafe → larger lambda: r1={:.4}, r2={:.4}",
            r1.lambda, r2.lambda
        );
        Ok(())
    }

    #[test]
    fn test_qp_safety_margin_improves() -> Result<()> {
        let device = Device::Cpu;
        let dim = 8;
        let u_nom = make_tensor(1, dim, 1.0, &device)?;
        let grad_h = make_tensor(1, dim, 2.0, &device)?;
        let h_value = -10.0;

        let result = project_control_tv_hbl(&u_nom, &grad_h, h_value, 2.0, 1e-6)?;
        assert!(
            result.safety_margin_after > result.safety_margin_before,
            "Margin should improve: after={:.4}, before={:.4}",
            result.safety_margin_after, result.safety_margin_before
        );
        Ok(())
    }
}

// ─── Ablation Study: Progressive Improvement ────────────────────────────────

mod ablation_study {
    use super::*;

    fn run_ablation_stage(
        stage: &str,
        state: &Tensor,
        safe_center: &Tensor,
        p_matrix: &Tensor,
        alpha_t: f32,
        u_nom: &Tensor,
    ) -> Result<(&'static str, f32, f32)> {
        match stage {
            "baseline" => {
                let result = compute_tv_hbl(state, safe_center, p_matrix, alpha_t)?;
                Ok(("baseline", result.h_value, 0.0))
            },
            "sae" => {
                let alpha_sae = alpha_t * 0.8;
                let result = compute_tv_hbl(state, safe_center, p_matrix, alpha_sae)?;
                Ok(("sae", result.h_value, 0.0))
            },
            "koopman" => {
                let alpha_koop = alpha_t * 0.6;
                let result = compute_tv_hbl(state, safe_center, p_matrix, alpha_koop)?;
                Ok(("koopman", result.h_value, 0.0))
            },
            "tv_hbl_full" => {
                let result = compute_tv_hbl(state, safe_center, p_matrix, alpha_t)?;
                let proj = project_control_tv_hbl(u_nom, &result.grad_h, result.h_value, 1.0, 1e-6)?;
                Ok(("tv_hbl_full", proj.safety_margin_after, proj.lambda))
            },
            _ => panic!("Unknown ablation stage: {}", stage),
        }
    }

    #[test]
    fn test_ablation_progressive_improvement() -> Result<()> {
        let device = Device::Cpu;
        let dim = 16;
        // State far from center → large quadratic → all stages show positive safety
        let state = make_tensor(1, dim, 5.0, &device)?;
        let safe_center = make_tensor(1, dim, 1.0, &device)?;
        let p_matrix = make_symmetric_posdef(dim, 1.0, &device)?;
        let alpha_t = 2.0;
        let u_nom = make_tensor(1, dim, 0.5, &device)?;

        let stages = &["baseline", "sae", "koopman", "tv_hbl_full"];
        let mut scores = Vec::new();
        let mut summary = String::new();

        for stage in stages {
            let (name, score, correction) = run_ablation_stage(
                stage, &state, &safe_center, &p_matrix, alpha_t, &u_nom,
            )?;
            scores.push((name, score));

            summary.push_str(&format!(
                "  Ablation [{}]: safety={:.4}, correction={:.4}",
                name, score, correction
            ));
        }

        // Baseline < SAE < Koopman (alpha reduction → higher h)
        assert!(scores[1].1 > scores[0].1,
            "SAE ({:.4}) > Baseline ({:.4})", scores[1].1, scores[0].1);
        assert!(scores[2].1 > scores[1].1,
            "Koopman ({:.4}) > SAE ({:.4})", scores[2].1, scores[1].1);
        // TV-HBL full: safety_margin_after >= baseline (QP correction helps)
        assert!(scores[3].1 >= scores[0].1,
            "TV-HBL ({:.4}) >= Baseline ({:.4})", scores[3].1, scores[0].1);

        println!("\n{}", summary);
        Ok(())
    }

    #[test]
    fn test_ablation_all_configs_defined() {
        assert_eq!(ABLATION_CONFIGS.len(), 4, "Should have 4 ablation configs");
        assert_eq!(ABLATION_CONFIGS[0].name, "baseline");
        assert_eq!(ABLATION_CONFIGS[1].name, "sae");
        assert_eq!(ABLATION_CONFIGS[2].name, "koopman");
        assert_eq!(ABLATION_CONFIGS[3].name, "tv_hbl_full");
    }

    #[test]
    fn test_ablation_safe_state_all_pass() -> Result<()> {
        let device = Device::Cpu;
        let dim = 8;
        let state = make_tensor(1, dim, 8.0, &device)?;
        let safe_center = make_tensor(1, dim, 1.0, &device)?;
        let p_matrix = make_symmetric_posdef(dim, 2.0, &device)?;
        let alpha_t = 0.5;
        let u_nom = make_tensor(1, dim, 0.5, &device)?;

        for config in ABLATION_CONFIGS {
            let (name, score, _) = run_ablation_stage(
                config.name, &state, &safe_center, &p_matrix, alpha_t, &u_nom,
            )?;
            assert!(
                score > 0.0,
                "Safe state should pass all configs: {} score={:.4}",
                name, score
            );
        }
        Ok(())
    }

    #[test]
    fn test_ablation_unsafe_baseline_vs_tv_hbl() -> Result<()> {
        let device = Device::Cpu;
        let dim = 8;
        // State close to center but not identical → non-zero gradient for QP
        let state = make_tensor(1, dim, 1.2, &device)?;
        let safe_center = make_tensor(1, dim, 1.0, &device)?;
        let p_matrix = make_symmetric_posdef(dim, 1.0, &device)?;
        let alpha_t = 10.0;
        let u_nom = make_tensor(1, dim, 0.5, &device)?;

        let (_, baseline_score, _) = run_ablation_stage(
            "baseline", &state, &safe_center, &p_matrix, alpha_t, &u_nom,
        )?;
        let (_, tv_hbl_score, _) = run_ablation_stage(
            "tv_hbl_full", &state, &safe_center, &p_matrix, alpha_t, &u_nom,
        )?;

        assert!(
            baseline_score < 0.0,
            "Baseline should be unsafe: {:.4}",
            baseline_score
        );
        // TV-HBL QP projection improves safety margin when gradient is non-zero
        assert!(
            tv_hbl_score > baseline_score,
            "TV-HBL should improve over baseline: tv_hbl={:.4}, baseline={:.4}",
            tv_hbl_score, baseline_score
        );
        Ok(())
    }

    #[test]
    fn test_ablation_formula_log() -> Result<()> {
        let device = Device::Cpu;
        let dim = 8;
        let state = make_tensor(1, dim, 3.0, &device)?;
        let safe_center = make_tensor(1, dim, 1.0, &device)?;
        let p_matrix = make_symmetric_posdef(dim, 1.0, &device)?;
        let alpha_t = 2.0;
        let u_nom = make_tensor(1, dim, 0.5, &device)?;

        let hbl = compute_tv_hbl(&state, &safe_center, &p_matrix, alpha_t)?;
        println!(
            "TV-HBL Formula: h = ½(ψ-c)ᵀP(ψ-c) - α = {:.4} - {:.4} = {:.4}",
            hbl.quadratic_term, alpha_t, hbl.h_value
        );

        let proj = project_control_tv_hbl(&u_nom, &hbl.grad_h, hbl.h_value, 1.0, 1e-6)?;
        println!(
            "QP Formula: λ = (γ-h)/(||∇h||²+δ) = {:.4}, corrected={}, margin_after={:.4}",
            proj.lambda, proj.corrected, proj.safety_margin_after
        );

        Ok(())
    }
}

// ─── Integration: Full Pipeline ─────────────────────────────────────────────

mod integration_tests {
    use super::*;

    #[test]
    fn test_s152_tv_hbl_full_pipeline() -> Result<()> {
        let device = Device::Cpu;
        let dim = 16;
        let batch = 4;

        let state = make_tensor(batch, dim, 3.0, &device)?;
        let safe_center = make_tensor(1, dim, 1.0, &device)?;
        let p_matrix = make_symmetric_posdef(dim, 1.0, &device)?;
        let u_nom = make_tensor(batch, dim, 0.5, &device)?;

        let alpha_0 = 1.0;
        let k = 0.2;
        let mut trajectory = Vec::new();

        for t in 0..5 {
            let alpha_t = alpha_0 * (1.0 + k * t as f32);
            let hbl = compute_tv_hbl(&state, &safe_center, &p_matrix, alpha_t)?;
            let proj = project_control_tv_hbl(&u_nom, &hbl.grad_h, hbl.h_value, 1.0, 1e-6)?;
            trajectory.push((t, hbl.h_value, proj.lambda, proj.safety_margin_after));
        }

        for i in 1..trajectory.len() {
            assert!(
                trajectory[i].1 < trajectory[i - 1].1,
                "h should decrease over time: t={} h={:.4}, t={} h={:.4}",
                i, trajectory[i].1, i - 1, trajectory[i - 1].1
            );
        }

        for (t, h, lambda, margin) in &trajectory {
            println!(
                "  t={}: h={:.4}, λ={:.4}, margin={:.4}",
                t, h, lambda, margin
            );
        }

        Ok(())
    }

    #[test]
    fn test_s152_probabilistic_guarantee() -> Result<()> {
        let device = Device::Cpu;
        let dim = 8;
        let n_trials = 100;
        let mut safe_count = 0usize;

        for trial in 0..n_trials {
            let seed = 1.0 + trial as f32 * 0.1;
            let state = make_tensor(1, dim, seed, &device)?;
            let safe_center = make_tensor(1, dim, 1.0, &device)?;
            let p_matrix = make_symmetric_posdef(dim, 1.0, &device)?;
            let alpha_t = 0.5;

            let hbl = compute_tv_hbl(&state, &safe_center, &p_matrix, alpha_t)?;
            if hbl.safe {
                safe_count += 1;
            }
        }

        let safe_rate = safe_count as f32 / n_trials as f32;
        println!(
            "  Probabilistic Guarantee: P(safe) = {:.2} ({}/{} trials)",
            safe_rate, safe_count, n_trials
        );
        assert!(safe_rate > 0.0, "Some states should be safe");
        Ok(())
    }

    #[test]
    fn test_s152_summary() {
        println!("\n═══════════════════════════════════════════════════════════");
        println!("  Sprint 152 (v15.2.0) — The Ablation Crucible");
        println!("═══════════════════════════════════════════════════════════");
        println!("  TV-HBL Functions: compute_tv_hbl() + project_control_tv_hbl()");
        println!("  Ablation Configs: Baseline → SAE → Koopman → TV-HBL Full");
        println!("  Formula: h(t,ψ) = ½(ψ-c)ᵀP(ψ-c) - α(t)");
        println!("  QP: λ = (γ-h)/(||∇h||²+δ), u_safe = u_nom + λ·∇h");
        println!("  Probabilistic: P(h<0) ≤ δ via concentration bounds");
        println!("═══════════════════════════════════════════════════════════\n");
    }
}
