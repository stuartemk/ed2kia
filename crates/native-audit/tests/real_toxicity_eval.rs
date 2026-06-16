//! Sprint 151 (v15.1.0) — RealToxicityPrompts Streaming Evaluator Benchmark
//!
//! Benchmarks TV-CBF steering + PI-Koopman residual on RealToxicityPrompts-style data.
//! Simulates streaming evaluation with latent-space toxicity proxies.

use candle_core::{DType, Device, Tensor};
use native_audit::{compute_pi_koopman_residual, steer_tvcbf};
use std::time::Instant;

// ─── Synthetic RealToxicityPrompts Latent Data ────────────────────────────────

/// Generate synthetic latent vectors mimicking RealToxicityPrompts token-level embeddings.
fn generate_synthetic_prompts(
    num_prompts: usize,
    hidden_dim: usize,
    toxicity_level: f32,
    device: &Device,
) -> candle_core::Result<Vec<Tensor>> {
    let mut prompts = Vec::with_capacity(num_prompts);
    for p in 0..num_prompts {
        let base = (p as f32) * 0.1 + toxicity_level * 2.0;
        let mut data = Vec::with_capacity(hidden_dim);
        for d in 0..hidden_dim {
            let val = base * (d as f32 * 0.01 + 0.5) + (p as f32 * 0.001);
            data.push(val);
        }
        let tensor = Tensor::from_vec(data, (1, hidden_dim), device)?;
        prompts.push(tensor);
    }
    Ok(prompts)
}

/// Generate safe centroid representing non-toxic reference distribution.
fn generate_safe_centroid(hidden_dim: usize, device: &Device) -> candle_core::Result<Tensor> {
    Tensor::zeros((1, hidden_dim), DType::F32, device)
}

/// Generate Koopman operator approximating one-step dynamics on latent space.
fn generate_koopman_operator(
    hidden_dim: usize,
    stability: f32,
    device: &Device,
) -> candle_core::Result<Tensor> {
    let mut data = vec![0.0f32; hidden_dim * hidden_dim];
    for i in 0..hidden_dim {
        data[i * (hidden_dim + 1)] = stability;
    }
    Tensor::from_vec(data, (hidden_dim, hidden_dim), device)
}

// ─── Streaming Evaluation Pipeline ───────────────────────────────────────────

#[derive(Debug)]
struct EvalResult {
    pub prompt_id: usize,
    pub tvcbf_intervened: bool,
    pub tvcbf_h_value: f32,
    pub tvcbf_correction: f32,
    pub koopman_residual_norm: f32,
    pub koopman_div_proxy: f32,
    pub koopman_volume_ratio: f32,
    pub latency_ms: f64,
}

impl std::fmt::Display for EvalResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Prompt {}: TV-CBF intervened={} h={:.4} corr={:.4} | Koopman res={:.4} div={:.4} vol={:.4} | {:.2}ms",
            self.prompt_id,
            self.tvcbf_intervened,
            self.tvcbf_h_value,
            self.tvcbf_correction,
            self.koopman_residual_norm,
            self.koopman_div_proxy,
            self.koopman_volume_ratio,
            self.latency_ms,
        )
    }
}

/// Run streaming evaluation pipeline on a batch of prompts.
fn stream_eval(
    prompts: &[Tensor],
    safe_centroid: &Tensor,
    k_operator: &Tensor,
    barrier_radius: f32,
    alpha_0: f32,
    k_rate: f32,
) -> candle_core::Result<Vec<EvalResult>> {
    let mut results = Vec::with_capacity(prompts.len());

    for (idx, prompt) in prompts.iter().enumerate() {
        let t = Instant::now();

        let steer = steer_tvcbf(prompt, safe_centroid, barrier_radius, alpha_0, k_rate, idx)?;
        let psi_pred = steer.steered.matmul(k_operator)?;
        let residual = compute_pi_koopman_residual(&steer.steered, &psi_pred, k_operator, None)?;

        let latency = t.elapsed().as_secs_f64() * 1000.0;

        results.push(EvalResult {
            prompt_id: idx,
            tvcbf_intervened: steer.intervened,
            tvcbf_h_value: steer.h_value,
            tvcbf_correction: steer.correction_magnitude,
            koopman_residual_norm: residual.residual_norm,
            koopman_div_proxy: residual.div_proxy,
            koopman_volume_ratio: residual.volume_ratio,
            latency_ms: latency,
        });
    }

    Ok(results)
}

fn compute_stats(results: &[EvalResult]) -> (f64, f64, f64, f32, f32, f32, usize) {
    let total_latency: f64 = results.iter().map(|r| r.latency_ms).sum();
    let avg_latency = total_latency / (results.len() as f64 + 1e-8);
    let min_latency = results
        .iter()
        .map(|r| r.latency_ms)
        .fold(f64::INFINITY, f64::min);
    let max_latency = results.iter().map(|r| r.latency_ms).fold(0.0f64, f64::max);

    let avg_residual: f32 = results.iter().map(|r| r.koopman_residual_norm).sum::<f32>()
        / (results.len() as f32 + 1e-8);
    let avg_div: f32 =
        results.iter().map(|r| r.koopman_div_proxy).sum::<f32>() / (results.len() as f32 + 1e-8);
    let interventions = results.iter().filter(|r| r.tvcbf_intervened).count();

    (
        avg_latency,
        min_latency,
        max_latency,
        avg_residual,
        avg_div,
        interventions as f32 / results.len() as f32,
        interventions,
    )
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[test]
fn bench_stream_eval_small() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let hidden_dim = 128;
    let num_prompts = 50;

    let prompts = generate_synthetic_prompts(num_prompts, hidden_dim, 0.3, &device)?;
    let safe_centroid = generate_safe_centroid(hidden_dim, &device)?;
    let k_op = generate_koopman_operator(hidden_dim, 0.95, &device)?;

    let results = stream_eval(&prompts, &safe_centroid, &k_op, 1.0, 0.5, 0.05)?;
    let (avg_ms, min_ms, max_ms, avg_res, avg_div, _intervention_rate, n_interventions) =
        compute_stats(&results);

    println!("\n=== RealToxicityPrompts Streaming Eval (Small) ===");
    println!("Prompts: {} | Hidden dim: {}", num_prompts, hidden_dim);
    println!(
        "Avg latency: {:.2}ms | Min: {:.2}ms | Max: {:.2}ms",
        avg_ms, min_ms, max_ms
    );
    println!("Avg Koopman residual: {:.6}", avg_res);
    println!("Avg divergence proxy: {:.6}", avg_div);
    println!(
        "Interventions: {}/{} ({:.1}%)",
        n_interventions,
        num_prompts,
        _intervention_rate * 100.0
    );

    assert!(
        avg_ms < 100.0,
        "Average latency {:.2}ms exceeds 100ms target",
        avg_ms
    );
    assert!(avg_res.is_finite(), "Residual must be finite");
    assert!(avg_div.is_finite(), "Divergence proxy must be finite");

    Ok(())
}

#[test]
fn bench_stream_eval_large() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let hidden_dim = 2048;
    let num_prompts = 200;

    let prompts = generate_synthetic_prompts(num_prompts, hidden_dim, 0.5, &device)?;
    let safe_centroid = generate_safe_centroid(hidden_dim, &device)?;
    let k_op = generate_koopman_operator(hidden_dim, 0.98, &device)?;

    let results = stream_eval(&prompts, &safe_centroid, &k_op, 2.0, 0.5, 0.05)?;
    let (avg_ms, min_ms, max_ms, avg_res, avg_div, intervention_rate, n_interventions) =
        compute_stats(&results);

    println!("\n=== RealToxicityPrompts Streaming Eval (Large - Llama-3.2-1B scale) ===");
    println!("Prompts: {} | Hidden dim: {}", num_prompts, hidden_dim);
    println!(
        "Avg latency: {:.2}ms | Min: {:.2}ms | Max: {:.2}ms",
        avg_ms, min_ms, max_ms
    );
    println!("Avg Koopman residual: {:.6}", avg_res);
    println!("Avg divergence proxy: {:.6}", avg_div);
    println!("Intervention rate: {:.1}%", intervention_rate * 100.0);
    println!("Interventions: {}/{}", n_interventions, num_prompts);

    assert!(
        avg_ms < 500.0,
        "Average latency {:.2}ms exceeds 500ms target for large dim",
        avg_ms
    );
    assert!(avg_res.is_finite());
    assert!(avg_div.is_finite());

    Ok(())
}

#[test]
fn bench_tvcbf_time_varying_trajectory() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let hidden_dim = 256;

    let x = Tensor::ones((1, hidden_dim), DType::F32, &device)?;
    let x_safe = Tensor::zeros((1, hidden_dim), DType::F32, &device)?;

    let alpha_0 = 0.5;
    let k = 0.05;
    let r = 1.0;

    let mut alphas = Vec::new();

    for t in 0..20 {
        let result = steer_tvcbf(&x, &x_safe, r, alpha_0, k, t)?;
        alphas.push(result.alpha_t);
    }

    for i in 1..alphas.len() {
        assert!(
            alphas[i] > alphas[i - 1],
            "α should increase over time: α({})={:.4} <= α({})={:.4}",
            i,
            alphas[i],
            i - 1,
            alphas[i - 1]
        );
    }

    println!("\n=== TV-CBF Time-Varying Trajectory ===");
    println!(
        "α(0) = {:.4}, α(19) = {:.4}",
        alphas.first().unwrap(),
        alphas.last().unwrap()
    );
    println!(
        "α growth factor: {:.2}x",
        alphas.last().unwrap() / alphas.first().unwrap()
    );

    Ok(())
}

#[test]
fn bench_koopman_volume_preservation() -> candle_core::Result<()> {
    let device = Device::Cpu;
    let hidden_dim = 512;

    let stabilities = [0.8, 0.9, 0.95, 0.98, 1.0];
    let mut volume_ratios = Vec::new();

    for &stab in &stabilities {
        let psi_curr = Tensor::ones((8, hidden_dim), DType::F32, &device)?;
        let k_op = generate_koopman_operator(hidden_dim, stab, &device)?;
        let psi_obs = psi_curr.matmul(&k_op)?;

        let result = compute_pi_koopman_residual(&psi_curr, &psi_obs, &k_op, None)?;
        volume_ratios.push(result.volume_ratio);
    }

    println!("\n=== Koopman Volume Preservation ===");
    for (i, &stab) in stabilities.iter().enumerate() {
        println!("  K(ρ={:.2}): vol_ratio={:.6}", stab, volume_ratios[i]);
    }

    for (i, &vr) in volume_ratios.iter().enumerate() {
        assert!(
            vr > 0.0 && vr.is_finite(),
            "Volume ratio for stability {} must be positive and finite",
            stabilities[i]
        );
    }

    Ok(())
}

#[test]
fn bench_streaming_eval_display() {
    let result = EvalResult {
        prompt_id: 42,
        tvcbf_intervened: true,
        tvcbf_h_value: -0.5,
        tvcbf_correction: 0.3,
        koopman_residual_norm: 0.1,
        koopman_div_proxy: 0.05,
        koopman_volume_ratio: 0.98,
        latency_ms: 12.5,
    };

    let display = format!("{}", result);
    assert!(display.contains("Prompt 42"));
    assert!(display.contains("intervened=true"));
    println!("Display: {}", display);
}

#[test]
fn test_s151_bench_summary() {
    println!("\n=== Sprint 151: RealToxicityPrompts Streaming Evaluator ===");
    println!("Target: Llama-3.2-1B (hidden=2048, RoPE, vocab=128k)");
    println!("Pipeline: TV-CBF steer → PI-Koopman residual → Aggregate stats");
    println!("Latency target: <100ms/prompt (small), <500ms/prompt (large)");
    println!("Mode: STRICT_MATH + FRONTIER_SCALE_UP + STREAMING_EVAL");
}
