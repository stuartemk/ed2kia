//! Tensor Serialization Benchmark — Compares serialization formats and quantization levels
//!
//! Run with: `cargo bench -p ed2kIA-benchmarks --bench tensor_serialization`

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

/// Benchmark native f32 serialization (baseline)
fn benchmark_f32_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("tensor_serialization/f32");

    let sizes = [128, 256, 512, 1024, 2048, 4096, 8192];

    for size in sizes {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, size| {
            let tensor: Vec<f32> = (0..*size).map(|i| black_box(i as f32)).collect();
            b.iter(|| {
                // Serialize to bytes
                let bytes: Vec<u8> = tensor
                    .iter()
                    .flat_map(|v| v.to_le_bytes())
                    .collect();
                black_box(&bytes);
            });
        });
    }

    group.finish();
}

/// Benchmark FP8 quantization + serialization
fn benchmark_fp8_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("tensor_serialization/fp8");

    let sizes = [128, 256, 512, 1024, 2048, 4096, 8192];

    for size in sizes {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, size| {
            let tensor: Vec<f32> = (0..*size).map(|i| black_box((i % 100) as f32 / 50.0 - 1.0)).collect();
            b.iter(|| {
                // Quantize to FP8 (simulated as u8 for benchmark)
                let max_val = tensor.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
                let scale = if max_val > 0.0 { max_val / 127.0 } else { 1.0 };
                let quantized: Vec<u8> = tensor
                    .iter()
                    .map(|v| (v / scale) as i8 as u8)
                    .collect();
                // Serialize
                let bytes = quantized.as_slice();
                black_box(bytes);
            });
        });
    }

    group.finish();
}

/// Benchmark INT4 quantization + serialization
fn benchmark_int4_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("tensor_serialization/int4");

    let sizes = [128, 256, 512, 1024, 2048, 4096, 8192];

    for size in sizes {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, size| {
            let tensor: Vec<f32> = (0..*size).map(|i| black_box((i % 100) as f32 / 50.0 - 1.0)).collect();
            b.iter(|| {
                // Quantize to INT4 (pack 2 values per byte)
                let max_val = tensor.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
                let scale = if max_val > 0.0 { max_val / 7.0 } else { 1.0 };
                let mut packed = Vec::with_capacity(size / 2 + 1);
                for chunk in tensor.chunks(2) {
                    let v0 = (chunk[0] / scale) as i8;
                    let v1 = if chunk.len() > 1 { (chunk[1] / scale) as i8 } else { 0 };
                    let packed_byte = ((v0 as u8 & 0x0F) | ((v1 as u8 & 0x0F) << 4));
                    packed.push(packed_byte);
                }
                black_box(&packed);
            });
        });
    }

    group.finish();
}

/// Benchmark JSON serialization overhead
fn benchmark_json_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("tensor_serialization/json");

    let sizes = [128, 256, 512, 1024];

    for size in sizes {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, size| {
            let tensor: Vec<f32> = (0..*size).map(|i| black_box(i as f32)).collect();
            b.iter(|| {
                let json = serde_json::to_string(&tensor).unwrap();
                black_box(&json);
            });
        });
    }

    group.finish();
}

/// Benchmark bincode serialization (compact binary)
fn benchmark_bincode_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("tensor_serialization/bincode");

    let sizes = [128, 256, 512, 1024, 2048, 4096];

    for size in sizes {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, size| {
            let tensor: Vec<f32> = (0..*size).map(|i| black_box(i as f32)).collect();
            b.iter(|| {
                let bytes = bincode::serialize(&tensor).unwrap();
                black_box(&bytes);
            });
        });
    }

    group.finish();
}

/// TODO: Add FlatBuffers benchmark once schema is defined
/// fn benchmark_flatbuffers_serialization(c: &mut Criterion) { ... }

criterion_group!(
    benches,
    benchmark_f32_serialization,
    benchmark_fp8_serialization,
    benchmark_int4_serialization,
    benchmark_json_serialization,
    benchmark_bincode_serialization,
);
criterion_main!(benches);
