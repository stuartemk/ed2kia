//! Goliath Benchmark — Certified Koopman Guardian & Conformal Tube MPC
//!
//! Sprint 158 (v15.8.0) — Autonomous Benchmark
//!
//! Simulates 100 adversarial prompts measuring:
//! - **ASR** (Attack Success Rate)
//! - **Certifiability Rate** (% feasible QP solutions)
//! - **Lyapunov Derivative** (contraction rate)
//! - **Tube Radius** (uncertainty bound over horizon)
//!
//! Run with: `cargo bench --bench koopman_mpc_vs_baseline`

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Number of adversarial prompts per benchmark run.
const NUM_PROMPTS: usize = 100;

/// Prediction horizon for tube propagation.
const HORIZON: usize = 10;

/// Conformal delta (PAC confidence level: 1 - delta).
const CONFORMAL_DELTA: f32 = 0.05;

/// Lyapunov contraction target alpha.
const LYAPUNOV_ALPHA: f32 = 0.1;

/// CBF gamma parameter.
const CBF_GAMMA: f32 = 0.5;

// ---------------------------------------------------------------------------
// Test Data Generators
// ---------------------------------------------------------------------------

/// Generate adversarial prompt states: perturbed trajectories from safe centroid.
fn generate_adversarial_prompts(count: usize, perturbation_scale: f32) -> Vec<Vec<f32>> {
    let mut prompts = Vec::with_capacity(count);
    for i in 0..count {
        let mut state = vec![0.0f32; 8]; // 8-dim latent space
        for (j, s) in state.iter_mut().enumerate() {
            // Deterministic perturbation based on prompt index
            let phase = (i * 7 + j * 13) as f32 * 0.01;
            *s = phase * perturbation_scale;
        }
        prompts.push(state);
    }
    prompts
}

/// Generate calibration errors for conformal quantile estimation.
fn generate_calibration_errors(count: usize, noise_level: f32) -> Vec<f32> {
    (0..count)
        .map(|i| {
            let base = (i % 10) as f32 * 0.01;
            base + noise_level * ((i * 3) % 7) as f32 * 0.001
        })
        .collect()
}

/// Simulate Koopman operator norm for different stability regimes.
fn koopman_operator_norm(regime: &str) -> f32 {
    match regime {
        "stable" => 0.85,
        "marginal" => 0.95,
        "unstable" => 1.10,
        _ => 0.90,
    }
}

// ---------------------------------------------------------------------------
// Core Benchmark Functions
// ---------------------------------------------------------------------------

/// Calibrate conformal epsilon from empirical errors.
///
/// Implements `ε_robust = Q_{1-δ}({e_i})` — empirical quantile calibration.
fn calibrate_conformal_epsilon(calibration_errors: &[f32], delta: f32) -> f32 {
    if calibration_errors.is_empty() {
        return 1.0; // Conservative default
    }
    let mut sorted = calibration_errors.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    let idx = (((1.0 - delta) * (n + 1) as f32).ceil() as usize)
        .saturating_sub(1)
        .min(n - 1);
    sorted[idx]
}

/// Propagate tube with conformal margin over horizon.
///
/// Implements `Z_{k+1} = K·Z_k ⊕ W ⊖ ε_k`
fn propagate_tube_conformal(
    k_norm: f32,
    initial_radius: f32,
    disturbance_bound: f32,
    conformal_epsilon: f32,
    horizon: usize,
) -> Vec<f32> {
    let mut radii = Vec::with_capacity(horizon + 1);
    radii.push(initial_radius);

    let mut r = initial_radius;
    for _ in 0..horizon {
        // Affine propagation: r_{k+1} = K_norm * r_k
        let propagated = k_norm * r;
        // Minkowski sum with disturbance: ⊕ W
        let with_disturbance = propagated + disturbance_bound;
        // Conformal tightening: ⊖ ε
        let epsilon_factor = 1.0 - conformal_epsilon.min(0.5);
        r = with_disturbance * epsilon_factor;
        radii.push(r);
    }
    radii
}

/// Solve robust CBF-QP (simplified analytical solution).
///
/// `min_u ½||u - u_nom||² s.t. L_f h + L_g h·u + γ·h ≥ ε_robust`
fn solve_robust_cbf_qp(
    u_nom: &[f32],
    cbf_value: f32,
    lf_h: f32,
    lg_h: &[f32],
    gamma: f32,
    conformal_epsilon: f32,
) -> (Vec<f32>, bool) {
    let rhs = conformal_epsilon - lf_h - gamma * cbf_value;

    // Compute L_g h · u_nom
    let lg_dot_unom: f32 = lg_h.iter().zip(u_nom.iter()).map(|(a, b)| a * b).sum();
    let nominal_satisfies = lg_dot_unom >= rhs;

    if nominal_satisfies {
        return (u_nom.to_vec(), true);
    }

    // Compute ||L_g h||²
    let lg_norm_sq: f32 = lg_h.iter().map(|x| x * x).sum();

    if lg_norm_sq < 1e-10 {
        return (vec![0.0; u_nom.len()], false);
    }

    // Projection: u_safe = u_nom + λ·L_g h
    let lambda = (rhs - lg_dot_unom) / lg_norm_sq;
    let u_safe: Vec<f32> = u_nom
        .iter()
        .zip(lg_h.iter())
        .map(|(u, g)| u + lambda * g)
        .collect();

    (u_safe, true)
}

/// Compute Lyapunov derivative approximation.
///
/// `V(ψ) = ||ψ - ψ_safe||²`, `V̇ ≈ V(ψ_{t+1}) - V(ψ_t)`
fn compute_lyapunov_derivative(state: &[f32], next_state: &[f32], safe_centroid: &[f32]) -> f32 {
    let v_t: f32 = state
        .iter()
        .zip(safe_centroid.iter())
        .map(|(s, c)| (s - c).powi(2))
        .sum();
    let v_next: f32 = next_state
        .iter()
        .zip(safe_centroid.iter())
        .map(|(s, c)| (s - c).powi(2))
        .sum();
    v_next - v_t
}

/// Compute Attack Success Rate (ASR).
fn compute_asr(exits_count: usize, total_prompts: usize) -> f32 {
    if total_prompts == 0 {
        return 0.0;
    }
    (exits_count as f32 / total_prompts as f32) * 100.0
}

/// Compute Certifiability Rate.
fn compute_certifiability_rate(feasible_count: usize, total_count: usize) -> f32 {
    if total_count == 0 {
        return 0.0;
    }
    (feasible_count as f32 / total_count as f32) * 100.0
}

// ---------------------------------------------------------------------------
// Simulation Kernels
// ---------------------------------------------------------------------------

/// Simulate a single adversarial prompt through the full Certified Koopman Guardian pipeline.
fn simulate_single_prompt(
    prompt: &[f32],
    k_norm: f32,
    conformal_epsilon: f32,
    disturbance_bound: f32,
    safe_centroid: &[f32],
) -> (bool, bool, f32, f32) {
    // 1. Compute nominal control (gradient descent toward safe centroid)
    let u_nom: Vec<f32> = prompt
        .iter()
        .zip(safe_centroid.iter())
        .map(|(p, s)| (s - p) * 0.1)
        .collect();

    // 2. CBF value: h(x) = ||x_safe||² - ||x||² (simplified)
    let dist_sq: f32 = prompt
        .iter()
        .zip(safe_centroid.iter())
        .map(|(p, s)| (p - s).powi(2))
        .sum();
    let cbf_value = 1.0 - dist_sq; // Safety margin

    // 3. Lie derivatives (simplified proxy)
    let lf_h = -0.05; // Drift toward unsafe
    let lg_h: Vec<f32> = prompt
        .iter()
        .map(|_| 0.5 + (0.1 * prompt.len() as f32))
        .collect();

    // 4. Solve robust CBF-QP
    let (_u_safe, feasible) =
        solve_robust_cbf_qp(&u_nom, cbf_value, lf_h, &lg_h, CBF_GAMMA, conformal_epsilon);

    // 5. Propagate tube for this prompt
    let tube_radii = propagate_tube_conformal(
        k_norm,
        dist_sq.sqrt(),
        disturbance_bound,
        conformal_epsilon,
        HORIZON,
    );
    let final_radius = *tube_radii.last().unwrap_or(&1.0);

    // 6. Check if state exits tube (attack success)
    let exits_tube = final_radius > 2.0; // Threshold for "exit"

    // 7. Compute Lyapunov derivative
    let next_state: Vec<f32> = prompt
        .iter()
        .zip(safe_centroid.iter())
        .map(|(p, s)| p + (s - p) * 0.05) // Small step toward safe
        .collect();
    let v_dot = compute_lyapunov_derivative(prompt, &next_state, safe_centroid);

    (exits_tube, feasible, v_dot, final_radius)
}

/// Full Goliath Benchmark simulation: 100 adversarial prompts through Certified Koopman Guardian.
fn goliath_benchmark_simulation(perturbation_scale: f32, regime: &str) -> (f32, f32, f32, f32) {
    let prompts = generate_adversarial_prompts(NUM_PROMPTS, perturbation_scale);
    let calibration_errors = generate_calibration_errors(NUM_PROMPTS, 0.01);
    let k_norm = koopman_operator_norm(regime);

    // Conformal calibration
    let conformal_epsilon = calibrate_conformal_epsilon(&calibration_errors, CONFORMAL_DELTA);

    // Safe centroid (origin)
    let safe_centroid = vec![0.0f32; 8];

    // Simulate all prompts
    let mut exits_count = 0usize;
    let mut feasible_count = 0usize;
    let mut total_v_dot = 0.0f32;
    let mut total_radius = 0.0f32;

    for prompt in &prompts {
        let (exits, feasible, v_dot, radius) = simulate_single_prompt(
            prompt,
            k_norm,
            conformal_epsilon,
            0.05, // disturbance bound
            &safe_centroid,
        );
        if exits {
            exits_count += 1;
        }
        if feasible {
            feasible_count += 1;
        }
        total_v_dot += v_dot;
        total_radius += radius;
    }

    let asr = compute_asr(exits_count, NUM_PROMPTS);
    let cert_rate = compute_certifiability_rate(feasible_count, NUM_PROMPTS);
    let avg_v_dot = total_v_dot / NUM_PROMPTS as f32;
    let avg_radius = total_radius / NUM_PROMPTS as f32;

    (asr, cert_rate, avg_v_dot, avg_radius)
}

// ---------------------------------------------------------------------------
// Criterion Benchmarks
// ---------------------------------------------------------------------------

/// Benchmark: Conformal Epsilon Calibration
fn bench_conformal_calibration(c: &mut Criterion) {
    let mut group = c.benchmark_group("sprint158/conformal_calibration");

    let sizes = [10, 50, 100, 500, 1000];
    for size in sizes {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, sz| {
            let errors = generate_calibration_errors(*sz, 0.01);
            b.iter(|| calibrate_conformal_epsilon(black_box(&errors), black_box(CONFORMAL_DELTA)))
        });
    }
}

/// Benchmark: Tube Propagation over Horizon
fn bench_tube_propagation(c: &mut Criterion) {
    let mut group = c.benchmark_group("sprint158/tube_propagation");

    let regimes = ["stable", "marginal", "unstable"];
    for regime in regimes {
        let k_norm = koopman_operator_norm(regime);
        group.bench_with_input(BenchmarkId::from_parameter(regime), &regime, |b, _r| {
            b.iter(|| {
                propagate_tube_conformal(
                    black_box(k_norm),
                    black_box(0.5),
                    black_box(0.05),
                    black_box(0.1),
                    black_box(HORIZON),
                )
            })
        });
    }
}

/// Benchmark: Robust CBF-QP Solver
fn bench_cbf_qp_solver(c: &mut Criterion) {
    let mut group = c.benchmark_group("sprint158/cbf_qp_solver");

    let u_nom = vec![0.1f32; 4];
    let lg_h = vec![0.5f32; 4];

    group.bench_function("feasible_nominal", |b| {
        b.iter(|| {
            solve_robust_cbf_qp(
                black_box(&u_nom),
                black_box(0.8),
                black_box(-0.05),
                black_box(&lg_h),
                black_box(CBF_GAMMA),
                black_box(0.1),
            )
        })
    });

    group.bench_function("infeasible_nominal", |b| {
        b.iter(|| {
            solve_robust_cbf_qp(
                black_box(&u_nom),
                black_box(-0.5),
                black_box(-0.1),
                black_box(&lg_h),
                black_box(CBF_GAMMA),
                black_box(0.3),
            )
        })
    });
}

/// Benchmark: Full Goliath Simulation — Stable Regime
fn bench_goliath_stable(c: &mut Criterion) {
    c.bench_function("sprint158/goliath_stable_100prompts", |b| {
        b.iter(|| goliath_benchmark_simulation(black_box(0.5), black_box("stable")))
    });
}

/// Benchmark: Full Goliath Simulation — Marginal Regime
fn bench_goliath_marginal(c: &mut Criterion) {
    c.bench_function("sprint158/goliath_marginal_100prompts", |b| {
        b.iter(|| goliath_benchmark_simulation(black_box(1.0), black_box("marginal")))
    });
}

/// Benchmark: Full Goliath Simulation — Unstable Regime
fn bench_goliath_unstable(c: &mut Criterion) {
    c.bench_function("sprint158/goliath_unstable_100prompts", |b| {
        b.iter(|| goliath_benchmark_simulation(black_box(2.0), black_box("unstable")))
    });
}

/// Benchmark: Lyapunov Derivative Computation
fn bench_lyapunov_derivative(c: &mut Criterion) {
    let mut group = c.benchmark_group("sprint158/lyapunov_derivative");

    let dims = [4, 8, 16, 32, 64];
    for dim in dims {
        group.throughput(Throughput::Elements(dim as u64));
        group.bench_with_input(BenchmarkId::from_parameter(dim), &dim, |b, d| {
            let state = vec![0.1f32; *d];
            let next_state = vec![0.09f32; *d];
            let safe = vec![0.0f32; *d];
            b.iter(|| {
                compute_lyapunov_derivative(
                    black_box(&state),
                    black_box(&next_state),
                    black_box(&safe),
                )
            })
        });
    }
}

/// Benchmark: ASR Comparison across Perturbation Scales
fn bench_asr_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("sprint158/asr_comparison");

    let scales = [0.1, 0.5, 1.0, 2.0, 5.0];
    for scale in scales {
        group.bench_with_input(BenchmarkId::from_parameter(scale), &scale, |b, s| {
            b.iter(|| {
                let (asr, _, _, _) =
                    goliath_benchmark_simulation(black_box(*s), black_box("stable"));
                asr
            })
        });
    }
}

// ---------------------------------------------------------------------------
// Benchmark Registration
// ---------------------------------------------------------------------------

criterion_group! {
    name = goliath_benchmarks;
    config = Criterion::default().sample_size(200);
    targets =
        bench_conformal_calibration,
        bench_tube_propagation,
        bench_cbf_qp_solver,
        bench_goliath_stable,
        bench_goliath_marginal,
        bench_goliath_unstable,
        bench_lyapunov_derivative,
        bench_asr_comparison,
}

criterion_main!(goliath_benchmarks);
