//! Sprint 171 — DR-CBF + Conformal Tubes Empirical Benchmarks
//!
//! Benchmarks for the conformal prediction tube module and DR-CBF soft penalty steering.
//!
//! Run with:
//! ```bash
//! cargo bench --bench s171_dr_cbf_conformal
//! ```

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

use candle_core::{Device, Tensor, DType};
use ed2kia::conformal::{
    conformal_tube_radius,
    propagate_conformal_tube,
    verify_conformal_coverage,
    dr_cbf_conformal_step,
    compute_soft_vfe_pain,
    propagate_tube_horizon,
    verify_conformal_safety,
    ConformalConfig,
};

// Helper: generate synthetic calibration errors
fn generate_calibration_errors(n: usize, seed: u32) -> Vec<f32> {
    (0..n)
        .map(|i| {
            let x = ((i as f32 * 31.0 + seed as f32 * 17.0) % 100.0) / 100.0;
            x * 2.0 + 0.01 // Range [0.01, 2.01]
        })
        .collect()
}

// Helper: create tensor from vector
fn make_tensor(data: Vec<f32>) -> Tensor {
    Tensor::from_vec(data, Device::Cpu).expect("tensor creation failed")
}

fn bench_conformal_tube_radius(c: &mut Criterion) {
    let mut group = c.benchmark_group("conformal_tube_radius");

    for &n in &[10, 50, 100, 500, 1000, 5000] {
        let errors = generate_calibration_errors(n, 42);
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| conformal_tube_radius(black_box(&errors), black_box(0.05)))
        });
    }
    group.finish();
}

fn bench_propagate_conformal_tube(c: &mut Criterion) {
    let mut group = c.benchmark_group("propagate_conformal_tube");

    for &dim in &[4, 16, 64, 256, 1024] {
        let center = Tensor::zeros((dim,), DType::F32, &Device::Cpu)
            .expect("tensor creation failed");
        group.bench_with_input(BenchmarkId::from_parameter(dim), &dim, |b, _| {
            b.iter(|| {
                propagate_conformal_tube(
                    black_box(&center),
                    black_box(1.0),
                    black_box(0.95),
                    black_box(0.1),
                )
            })
        });
    }
    group.finish();
}

fn bench_verify_conformal_coverage(c: &mut Criterion) {
    let mut group = c.benchmark_group("verify_conformal_coverage");

    for &n in &[10, 100, 1000, 10000, 100000] {
        let errors = generate_calibration_errors(n, 99);
        let radius = conformal_tube_radius(&errors, 0.05);
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| verify_conformal_coverage(black_box(&errors), black_box(radius)))
        });
    }
    group.finish();
}

fn bench_dr_cbf_conformal_step(c: &mut Criterion) {
    let mut group = c.benchmark_group("dr_cbf_conformal_step");

    let cal_errors = generate_calibration_errors(200, 42);

    for &dim in &[4, 16, 64, 256] {
        let u_nom = Tensor::zeros((dim,), DType::F32, &Device::Cpu)
            .expect("tensor creation failed");
        let lg_h = Tensor::ones((dim,), DType::F32, &Device::Cpu)
            .expect("tensor creation failed");
        group.bench_with_input(BenchmarkId::from_parameter(dim), &dim, |b, _| {
            b.iter(|| {
                dr_cbf_conformal_step(
                    black_box(&u_nom),
                    black_box(0.5),
                    black_box(&lg_h),
                    black_box(&cal_errors),
                    black_box(0.05),
                    black_box(0.95),
                    black_box(0.1),
                    black_box(1.0),
                )
            })
        });
    }
    group.finish();
}

fn bench_compute_soft_vfe_pain(c: &mut Criterion) {
    let mut group = c.benchmark_group("compute_soft_vfe_pain");

    for &h_val in &[(-5.0_f32).to_bits(), (-1.0_f32).to_bits(), 0u32, 1u32, 5u32] {
        let h = f32::from_bits(h_val);
        group.bench_with_input(BenchmarkId::from_parameter(h), &h, |b, _| {
            b.iter(|| compute_soft_vfe_pain(black_box(h), black_box(1.0)))
        });
    }
    group.finish();
}

fn bench_propagate_tube_horizon(c: &mut Criterion) {
    let mut group = c.benchmark_group("propagate_tube_horizon");

    for &horizon in &[10, 50, 100, 500, 1000] {
        group.bench_with_input(BenchmarkId::from_parameter(horizon), &horizon, |b, _| {
            b.iter(|| {
                propagate_tube_horizon(
                    black_box(1.0),
                    black_box(0.95),
                    black_box(0.1),
                    black_box(horizon),
                )
            })
        });
    }
    group.finish();
}

fn bench_verify_conformal_safety(c: &mut Criterion) {
    c.bench_function("verify_conformal_safety", |b| {
        b.iter(|| verify_conformal_safety(black_box(5.0), black_box(1.0), black_box(2.0)))
    });
}

fn bench_conformal_config_edge_vs_precision(c: &mut Criterion) {
    let mut group = c.benchmark_group("conformal_config_comparison");
    let cal_errors = generate_calibration_errors(500, 42);

    // Edge fast config
    let edge_config = ConformalConfig::edge_fast();
    group.bench_function("edge_fast_radius", |b| {
        b.iter(|| conformal_tube_radius(black_box(&cal_errors), black_box(edge_config.alpha)))
    });

    // High precision config
    let precision_config = ConformalConfig::high_precision();
    group.bench_function("high_precision_radius", |b| {
        b.iter(|| conformal_tube_radius(black_box(&cal_errors), black_box(precision_config.alpha)))
    });

    group.finish();
}

fn bench_full_conformal_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_conformal_pipeline");

    for &steps in &[10, 50, 100] {
        let cal_errors = generate_calibration_errors(200, 42);
        let radius = conformal_tube_radius(&cal_errors, 0.05);
        let horizon = propagate_tube_horizon(radius, 0.95, 0.1, steps);
        let coverage = verify_conformal_coverage(&cal_errors, radius);

        group.bench_with_input(BenchmarkId::from_parameter(steps), &steps, |b, _| {
            b.iter(|| {
                let r = conformal_tube_radius(black_box(&cal_errors), black_box(0.05));
                let h = propagate_tube_horizon(black_box(r), black_box(0.95), black_box(0.1), black_box(steps));
                let c = verify_conformal_coverage(black_box(&cal_errors), black_box(r));
                black_box((r, h, c));
            })
        });
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(200);
    targets =
        bench_conformal_tube_radius,
        bench_propagate_conformal_tube,
        bench_verify_conformal_coverage,
        bench_dr_cbf_conformal_step,
        bench_compute_soft_vfe_pain,
        bench_propagate_tube_horizon,
        bench_verify_conformal_safety,
        bench_conformal_config_edge_vs_precision,
        bench_full_conformal_pipeline,
}
criterion_main!(benches);
