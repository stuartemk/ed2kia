//! SAE Inference Benchmark — Measures Sparse Autoencoder forward pass latency
//!
//! Run with: `cargo bench -p ed2kIA-benchmarks --bench sae_inference`
//!
//! Target metrics:
//! - SAE forward pass (Top-K, 1024 dim): < 15ms
//! - SAE forward pass (Top-K, 4096 dim): < 50ms
//! - SAE forward pass (Top-K, 8192 dim): < 100ms

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::time::Instant;

/// Simulated SAE forward pass with Top-K activation
/// Mirrors the structure of candle-based SAE inference without external deps
struct SaeSimulator {
    input_dim: usize,
    latent_dim: usize,
    top_k: usize,
    // Simulated weights (W: input_dim × latent_dim)
    _weights: Vec<f32>,
    // Simulated bias
    _bias: Vec<f32>,
}

impl SaeSimulator {
    fn new(input_dim: usize, latent_dim: usize, top_k: usize) -> Self {
        let n_weights = input_dim * latent_dim;
        let n_bias = latent_dim;
        Self {
            input_dim,
            latent_dim,
            top_k,
            _weights: vec![0.5f32; n_weights],
            _bias: vec![0.0f32; n_bias],
        }
    }

    /// Simulate forward pass: x @ W + bias → Top-K → output
    fn forward(&self, input: &[f32]) -> Vec<f32> {
        let mut activations = vec![0.0f32; self.latent_dim];

        // Matrix multiplication: input @ W
        for j in 0..self.latent_dim {
            let mut sum = 0.0f32;
            for i in 0..self.input_dim {
                sum += input[i] * self._weights[i * self.latent_dim + j];
            }
            activations[j] = sum + self._bias[j];
        }

        // ReLU
        for a in &mut activations {
            if *a < 0.0 {
                *a = 0.0;
            }
        }

        // Top-K selection: find top-k indices and zero out the rest
        let mut indexed: Vec<(usize, f32)> = activations.iter().enumerate().map(|(i, &v)| (i, v)).collect();
        indexed.select_nth_by_key(self.latent_dim.saturating_sub(self.top_k), |(i, v)| std::cmp::Reverse(*i));

        let mut output = vec![0.0f32; self.latent_dim];
        for (idx, &val) in &indexed[self.latent_dim.saturating_sub(self.top_k)..] {
            output[idx] = val;
        }

        output
    }
}

/// Benchmark SAE forward pass with varying input dimensions
fn benchmark_sae_forward(c: &mut Criterion) {
    let mut group = c.benchmark_group("sae_forward");

    let configs: &[(usize, usize, usize)] = &[
        // (input_dim, latent_dim, top_k)
        (128, 1024, 32),
        (128, 2048, 64),
        (128, 4096, 128),
        (128, 8192, 256),
    ];

    for (input_dim, latent_dim, top_k) in configs {
        let id = format!("in{}_lat{}_k{}", input_dim, latent_dim, top_k);
        let sae = SaeSimulator::new(*input_dim, *latent_dim, *top_k);
        let input: Vec<f32> = (0..*input_dim).map(|i| (i as f32) % 1.0).collect();

        group.bench_function(id, |b| {
            b.iter(|| {
                let output = sae.forward(black_box(&input));
                black_box(&output);
            });
        });
    }

    group.finish();
}

/// Benchmark SAE forward pass latency with fixed 1024-dim latent
fn benchmark_sae_latency_1024(c: &mut Criterion) {
    let mut group = c.benchmark_group("sae_latency_1024");

    let sae = SaeSimulator::new(128, 1024, 32);
    let input: Vec<f32> = (0..128).map(|i| (i as f32) % 1.0).collect();

    group.bench_function("forward_pass_128x1024_k32", |b| {
        b.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                let output = sae.forward(&input);
                black_box(&output);
            }
            start.elapsed()
        });
    });

    group.finish();
}

/// Benchmark batch SAE inference (multiple inputs)
fn benchmark_sae_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("sae_batch");

    let batch_sizes = [1, 4, 8, 16, 32];
    let sae = SaeSimulator::new(128, 4096, 128);

    for batch in &batch_sizes {
        let inputs: Vec<Vec<f32>> = (0..*batch)
            .map(|_| (0..128).map(|i| (i as f32) % 1.0).collect())
            .collect();

        group.bench_with_input(BenchmarkId::from_parameter(batch), &inputs, |b, inputs| {
            b.iter(|| {
                for input in inputs {
                    let output = sae.forward(input);
                    black_box(&output);
                }
            });
        });
    }

    group.finish();
}

/// Benchmark Top-K selection (isolated)
fn benchmark_topk_selection(c: &mut Criterion) {
    let mut group = c.benchmark_group("sae_topk");

    let sizes = [1024, 4096, 8192, 16384];
    let top_ks = [32, 128, 256, 512];

    for (size, k) in std::iter::zip(&sizes, &top_ks) {
        let activations: Vec<f32> = (0..*size).map(|i| (i as f32) % 1.0).collect();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_k{}", size, k)),
            &activations,
            |b, activations| {
                b.iter(|| {
                    let mut indexed: Vec<(usize, f32)> =
                        activations.iter().enumerate().map(|(i, &v)| (i, v)).collect();
                    indexed.select_nth_by_key(
                        activations.len().saturating_sub(*k),
                        |(i, v)| std::cmp::Reverse(*i),
                    );
                    black_box(&indexed);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_sae_forward,
    benchmark_sae_latency_1024,
    benchmark_sae_batch,
    benchmark_topk_selection
);
criterion_main!(benches);
