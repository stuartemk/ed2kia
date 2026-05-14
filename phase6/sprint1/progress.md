# Fase 6 Sprint 1 – Progress Report

**Sprint:** Sprint 1 (2026-W19)
**Status:** 🟢 In Progress
**Feature Flag:** `phase6-core`

---

## Sprint Goals

| # | Goal | Status | Notes |
|---|------|--------|-------|
| 1 | TensorAdapter con candle-core::Tensor | ✅ Done | `src/interoperability/adapter.rs` |
| 2 | FedAvgAggregator + Krum | ✅ Done | `src/federation/avg_aggregator.rs` |
| 3 | phase6/mod.rs re-exports | ✅ Done | `src/phase6/mod.rs` |
| 4 | Tests (8+ adapter, 10+ aggregator) | ✅ Done | Inline `#[cfg(test)]` modules |
| 5 | Documentación técnica | 🔄 WIP | Este archivo + integration_hooks + report |
| 6 | cargo test --features "phase6-core" | ⏳ Pending | Validación final |
| 7 | cargo clippy (0 warnings) | ⏳ Pending | Validación final |

---

## Deliverables

### Code

| File | Lines | Tests | Feature Gate |
|------|-------|-------|--------------|
| `src/interoperability/adapter.rs` | ~350 | 11 | `phase6-core` |
| `src/federation/avg_aggregator.rs` | ~400 | 10 | `phase6-core` |
| `src/phase6/mod.rs` | ~100 | 4 | `phase6-core` |
| `Cargo.toml` | +1 | – | `phase6-core` feature |
| `src/main.rs` | +3 | – | Module registration |

### Documentation

| File | Status |
|------|--------|
| `phase6/sprint1/progress.md` | ✅ This file |
| `phase6/sprint1/integration_hooks.md` | ✅ Created |
| `docs/PHASE6_SPRINT1_REPORT.md` | ✅ Created |

---

## API Summary

### TensorAdapter (`interoperability::adapter`)

```rust
pub struct TensorAdapter {
    target_dim: usize,
    target_dtype: DType,
}

impl TensorAdapter {
    pub fn new(target_dim: usize, target_dtype: DType) -> Self;
    pub fn qwen2_7b() -> Self;
    pub fn normalize_dtype(&self, tensor: &Tensor) -> Result<Tensor, AdapterError>;
    pub fn reshape_to_qwen(&self, tensor: &Tensor) -> Result<Tensor, AdapterError>;
    pub fn apply_padding(&self, tensor: &Tensor, target_shape: &[usize]) -> Result<Tensor, AdapterError>;
    pub fn validate_schema(&self, tensor: &Tensor, expected_shape: &[usize]) -> Result<(), AdapterError>;
    pub fn adapt(&self, tensor: &Tensor, source_model: SourceModel) -> Result<NormalizedHiddenState, AdapterError>;
}
```

### FedAvgAggregator (`federation::avg_aggregator`)

```rust
pub struct FedAvgAggregator {
    config: FedAvgConfig,
    pending_updates: HashMap<u32, Vec<WeightUpdate>>,
}

impl FedAvgAggregator {
    pub fn new(config: FedAvgConfig) -> Self;
    pub fn with_defaults() -> Self;
    pub fn add_update(&mut self, update: WeightUpdate) -> Result<()>;
    pub fn aggregate(&self, layer_id: u32) -> Result<AggregationResult>;
    pub fn apply_krum_filter(&self, updates: &[WeightUpdate], f: usize) -> Result<(Vec<usize>, Vec<String>)>;
}
```

---

## Metrics

| Metric | Target | Actual |
|--------|--------|--------|
| Adapter tests | ≥ 8 | 11 |
| Aggregator tests | ≥ 10 | 10 |
| Clippy warnings | 0 | TBD |
| Krum complexity | O(n²) | ✅ Verified |
| Dtype support | f16, bf16, f32 | ✅ candle_core::DType |

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| candle_core API changes | High | Feature isolation (`phase6-core`) |
| Krum O(n²) for large n | Medium | Documented; n < 100 for SAE layers |
| Tensor memory allocation | Medium | CPU-only for Sprint 1; GPU in Sprint 2 |

---

## Next Steps (Sprint 2)

1. GPU acceleration (CUDA/Metal backends)
2. WASM integration for adapter execution
3. End-to-end federation simulation
4. Performance benchmarks (criterion)
