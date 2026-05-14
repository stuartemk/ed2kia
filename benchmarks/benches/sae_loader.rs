//! SAE Loader Benchmark — Measures model loading time and memory usage
//!
//! Run with: `cargo bench -p ed2kIA-benchmarks --bench sae_loader`

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

/// Simulate SAE model loading with different latent dimensions
fn benchmark_sae_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("sae_loader");

    // Test different latent dimensions (matching SAE v7 config)
    let dimensions = [2048, 4096, 8192, 16384];

    for dim in dimensions {
        group.bench_with_input(BenchmarkId::from_parameter(dim), &dim, |b, dim| {
            b.iter(|| {
                // Simulate weight allocation and initialization
                let weights: Vec<f32> = (0..(*dim * 128)).map(|_| black_box(0.0f32)).collect();
                black_box(&weights);
            });
        });
    }

    group.finish();
}

/// Benchmark memory profiling for SAE model loading
fn benchmark_sae_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("sae_memory");

    group.bench_function("allocate_8192_latent", |b| {
        b.iter(|| {
            // 8192 latent × 128 input_dim = 1M weights
            let weights: Vec<f32> = vec![0.0f32; 8192 * 128];
            black_box(&weights);
        });
    });

    group.finish();
}

/// TODO: Integrate with actual Candle loader once workspace path is resolved
/// use ed2kia::sae::loader::SAELoader;
/// fn benchmark_candle_load(c: &mut Criterion) { ... }

criterion_group!(benches, benchmark_sae_load, benchmark_sae_memory);
criterion_main!(benches);
