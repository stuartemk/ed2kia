# ed2kIA Benchmarks

Performance benchmark suite for ed2kIA v1.7.0, focused on tensor serialization, SAE loading, and quantization overhead.

## 📊 Benchmarks

| Benchmark | Description | Target |
|-----------|-------------|--------|
| `sae_loader` | Model loading time by latent dimension | < 50ms for 8192 latent |
| `tensor_serialization` | Serialization throughput (f32, fp8, int4, JSON, bincode) | > 100MB/s for f32 |

## 🚀 Execution

```bash
# Run all benchmarks
cargo bench -p ed2kIA-benchmarks

# Run specific benchmark
cargo bench -p ed2kIA-benchmarks --bench sae_loader
cargo bench -p ed2kIA-benchmarks --bench tensor_serialization

# Filter specific test
cargo bench -p ed2kIA-benchmarks "tensor_serialization/f32"
```

## 📈 Metrics Objective

| Metric | v1.6.0 Baseline | v1.7.0 Target |
|--------|-----------------|---------------|
| Tensor streaming round-trip | ~350ms | < 50ms |
| FP8 serialization throughput | N/A | > 500MB/s |
| INT4 compression ratio | N/A | 8x |
| SAE load (8192 latent) | ~120ms | < 50ms |

## 🛠 Contributing

### Adding a New Benchmark

1. Create `benches/your_benchmark.rs`
2. Add `[[bench]]` entry in `Cargo.toml`
3. Use `criterion` for statistical accuracy
4. Include baseline measurements in comments

### Reporting Results

```markdown
| Benchmark | Before | After | Δ |
|-----------|--------|-------|---|
| f32 serialize | 1.2MB/s | 1.5MB/s | +25% |
```

### SIMD/CUDA Optimizations

- Profile with `perf` or `vtune` before optimizing
- Target AVX2/AVX-512 for CPU paths
- Use `candle-core` CUDA backend for GPU paths
- Document portability requirements

## 🔧 Dependencies

- `criterion` — Statistical benchmarking framework
- `candle-core` — ML tensor operations
- `flatbuffers` — Zero-copy serialization (TODO)
- `bincode` — Compact binary serialization

## 📚 References

- [RFC-001: Latency Mitigation](../docs/rfc/rfc-001-latency-mitigation-v1.7.md)
- [Architecture v1.6.0](../docs/architecture_v1.6.0.md)
- [v1.7 Roadmap](../docs/v1.7-roadmap-placeholder.md)
