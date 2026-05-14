# Fase 6 Sprint 1 – Technical Report

**Date:** 2026-05-03
**Sprint:** Sprint 1
**Feature Flag:** `phase6-core`
**Status:** Implementation Complete – Validation Pending

---

## Executive Summary

Sprint 1 delivers the two core modules required for Fase 6 (Interoperability & Federation):

1. **TensorAdapter** (`src/interoperability/adapter.rs`) – Cross-model tensor compatibility using `candle_core::Tensor`
2. **FedAvgAggregator** (`src/federation/avg_aggregator.rs`) – Federated Averaging with Krum Byzantine filtering

Both modules are feature-gated behind `phase6-core`, ensuring zero impact on the v0.5.0 stable build.

---

## 1. TensorAdapter

### Architecture

```
Source Tensor (Llama/Mistral/ONNX)
    ↓
normalize_dtype()     → Cast to target dtype (f16/bf16/f32)
    ↓
reshape_to_qwen()     → Project to Qwen-Scope dimensionality
    ↓
apply_padding()       → Zero-pad or truncate to target shape
    ↓
validate_schema()     → Verify shape + dtype match
    ↓
NormalizedHiddenState (Qwen-Scope format)
```

### Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| `candle_core::Tensor` ops | Vectorized, GPU-ready, consistent with SAE module |
| `AdapterError` struct | Explicit error context (source, expected, got) |
| Block averaging for shrink | Preserves information better than truncation |
| Zero-padding for expand | Standard ML practice, no hallucination |

### API

```rust
pub struct TensorAdapter {
    target_dim: usize,
    target_dtype: DType,
}

impl TensorAdapter {
    pub fn new(target_dim: usize, target_dtype: DType) -> Self;
    pub fn qwen2_7b() -> Self;                    // Preset: 3584, F32
    pub fn normalize_dtype(&self, t: &Tensor) -> Result<Tensor, AdapterError>;
    pub fn reshape_to_qwen(&self, t: &Tensor) -> Result<Tensor, AdapterError>;
    pub fn apply_padding(&self, t: &Tensor, shape: &[usize]) -> Result<Tensor, AdapterError>;
    pub fn validate_schema(&self, t: &Tensor, shape: &[usize]) -> Result<(), AdapterError>;
    pub fn adapt(&self, t: &Tensor, model: SourceModel) -> Result<NormalizedHiddenState, AdapterError>;
}
```

### Tests (11 total)

| Test | Coverage |
|------|----------|
| `test_source_model_display` | Enum serialization |
| `test_adapter_error_display` | Error formatting |
| `test_normalized_hidden_state` | Data struct |
| `test_normalize_dtype_f32_to_f32` | Identity cast |
| `test_normalize_dtype_f16_to_f32` | Cross-dtype cast |
| `test_reshape_to_qwen_same_dim` | No-op projection |
| `test_reshape_to_qwen_expand` | Zero-padding |
| `test_reshape_to_qwen_shrink` | Block averaging |
| `test_apply_padding_no_change` | Identity padding |
| `test_apply_padding_expand` | Shape expansion |
| `test_validate_schema_ok/mismatch` | Schema validation |
| `test_full_adapt_pipeline` | End-to-end flow |

---

## 2. FedAvgAggregator

### Architecture

```
WeightUpdate (node_id, layer_id, weight_deltas, num_samples)
    ↓
add_update()          → Hash verification + queue
    ↓
apply_krum_filter(f)  → O(n²) euclidean distance matrix
    ↓                    Select top n-f-2 closest updates
aggregate()           → Weighted FedAvg via candle_core::Tensor
    ↓
AggregationResult { final_weights, accepted_updates, filtered_malicious, confidence }
```

### Krum Algorithm

1. **Distance Matrix:** Compute pairwise euclidean distances between all weight deltas using `candle_core::Tensor` broadcasting: `[n,1,dim] - [1,n,dim] = [n,n,dim]`
2. **Krum Scores:** For each node, sum the `n-f-2` smallest distances (excluding self)
3. **Selection:** Choose nodes with lowest scores (most consistent with peers)
4. **Weighted Average:** FedAvg with sample-weighted aggregation on selected nodes

### Complexity

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| Distance matrix | O(n² × d) | Vectorized with Tensor |
| Krum scoring | O(n² log n) | Sort per node |
| Weighted avg | O(n × d) | Vectorized with Tensor |
| **Total** | **O(n² × d)** | d = weight dimension |

### API

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

### Tests (10 total)

| Test | Coverage |
|------|----------|
| `test_weight_update_hash` | SHA-256 integrity |
| `test_weight_update_dimension` | Dimension query |
| `test_aggregation_result_fields` | Result struct |
| `test_fedavg_config_default` | Config defaults |
| `test_add_update` | Queue insertion |
| `test_reject_invalid_hash` | Hash validation |
| `test_fedavg_aggregation` | Basic FedAvg |
| `test_krum_filter_excludes_outlier` | Byzantine detection |
| `test_insufficient_participants` | Min threshold |
| `test_pending_layers` | Layer management |
| `test_confidence_score` | Confidence calc |
| `test_clear_layer` | Cleanup |
| `test_distance_tensor_computation` | Tensor ops |
| `test_weighted_avg_tensor` | Vectorized avg |

---

## 3. Feature Flag Strategy

```toml
[features]
default = ["cpu", "core-only"]
core-only = []                  # Fases 1-5 (default)
phase6-core = []                # Sprint 1: adapter + aggregator
phase6-experimental = ["phase6-core"]  # Full Fase 6
```

| Build Command | Modules Compiled | Use Case |
|--------------|-----------------|----------|
| `cargo build` | core-only | Production v0.5.0 |
| `cargo build --features "phase6-core"` | core + adapter + aggregator | Sprint 1 testing |
| `cargo build --features "phase6-experimental"` | core + all Fase 6 | Full development |

---

## 4. Files Modified/Created

### Created
- `src/interoperability/adapter.rs` – TensorAdapter implementation
- `src/federation/avg_aggregator.rs` – FedAvg + Krum implementation
- `src/phase6/mod.rs` – Feature-gated re-exports

### Modified
- `Cargo.toml` – Added `phase6-core` feature
- `src/main.rs` – Module registration with `#[cfg]` gates

### Documentation
- `phase6/sprint1/progress.md` – Sprint tracking
- `phase6/sprint1/integration_hooks.md` – v0.5.0 integration points
- `docs/PHASE6_SPRINT1_REPORT.md` – This report

---

## 5. Validation Checklist

- [x] TensorAdapter implements `normalize_dtype()`, `reshape_to_qwen()`, `apply_padding()`, `validate_schema()`
- [x] FedAvgAggregator implements `add_update()`, `aggregate()`, `apply_krum_filter(f)`
- [x] Uses `candle_core::Tensor` for vectorized operations
- [x] Supports f16, bf16, f32 dtype conversion
- [x] Krum: O(n²) complexity, selects top n-f-2 closest
- [x] `AdapterError` struct with `source`, `expected_shape`, `got`
- [x] `AggregationResult` with `accepted_updates`, `filtered_malicious`, `final_weights`, `confidence`
- [x] Feature flag `#[cfg(feature = "phase6-core")]` on all new code
- [x] 8+ adapter tests
- [x] 10+ aggregator tests
- [ ] `cargo test --features "phase6-core"` – All tests pass
- [ ] `cargo clippy --features "phase6-core" -- -D warnings` – Zero warnings

---

## 6. Known Limitations

| Limitation | Sprint | Resolution |
|------------|--------|-----------|
| CPU-only Tensor ops | Sprint 2 | CUDA/Metal backends |
| No WASM integration | Sprint 3 | wasmtime execution |
| No end-to-end federation | Sprint 2 | Full simulation |
| Krum O(n²) | Sprint 3 | Approximate Krum for n > 100 |

---

## 7. Next Sprint (Sprint 2)

1. **GPU Acceleration:** Enable CUDA/Metal backends for Tensor operations
2. **Federation Simulation:** End-to-end test with multiple nodes
3. **Performance Benchmarks:** Criterion benchmarks for adapter + aggregator
4. **WASM Integration:** Execute adapter in wasmtime sandbox

---

*Report generated: 2026-05-03T18:00Z*
*ed2kIA Fase 6 Sprint 1 – Interoperability & Federation Core*
