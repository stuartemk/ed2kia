# Phase 6 Sprint 2 – Integration Matrix

**Date:** 2026-05-03
**Version:** 0.6.0-alpha.2

---

## Module Dependencies

```
┌─────────────────────────────────────────────────────────────┐
│                    Phase 6 Sprint 2                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐     ┌──────────────┐     ┌────────────┐  │
│  │  ONNX        │────▶│  Tensor      │────▶│  FedAvg    │  │
│  │  Adapter     │     │  Adapter     │     │  Aggregator│  │
│  │  (onnx_)     │     │  (adapter)   │     │  (avg_)    │  │
│  └──────────────┘     └──────────────┘     └─────┬──────┘  │
│                                                   │         │
│  ┌──────────────┐     ┌──────────────┐            │         │
│  │  Auth        │────▶│  API v2      │◀───────────┘         │
│  │  Validator   │     │  Routes      │                       │
│  │  (auth)      │     │  (routes)    │                       │
│  └──────────────┘     └──────────────┘                       │
│                                                               │
│  ┌──────────────┐     ┌──────────────┐                       │
│  │  Staking     │────▶│  Registry    │                       │
│  │  Proof       │     │  (registry)  │                       │
│  │  (proof)     │     └──────────────┘                       │
│  └──────────────┘                                             │
│                                                               │
│  ┌─────────────────────────────────────────────────┐         │
│  │  phase6/mod.rs – Feature-gated re-exports       │         │
│  └─────────────────────────────────────────────────┘         │
└─────────────────────────────────────────────────────────────┘
```

## Integration Points

| Source | Target | Mechanism | Feature Flag |
|--------|--------|-----------|--------------|
| `onnx_adapter` | `adapter` | `NormalizedHiddenState` | `phase6-sprint2` |
| `adapter` | `avg_aggregator` | `WeightUpdate` | `phase6-core` |
| `auth` | `routes` | Middleware | `phase6-sprint2` |
| `registry` | `routes` | Callback fn | `phase6-sprint2` |
| `proof` | `registry` | `StakingProof` | `phase6-sprint2` |
| `phase6/mod` | All | Re-exports | `phase6-core` / `phase6-sprint2` |

## Data Flow: ONNX → FedAvg

```
1. ONNX Model (.onnx file)
   │
   ▼
2. OnnxAdapter.load_model()
   │  Validates file, extracts weights
   │  Returns candle_core::Tensor<f32>
   │
   ▼
3. OnnxAdapter.extract_hidden_states()
   │  Extracts hidden states from target layer
   │
   ▼
4. OnnxAdapter.convert_to_qwen_scope()
   │  Normalizes to QwenScopeSchema (hidden_size=3584)
   │
   ▼
5. TensorAdapter.normalize_dtype()
   │  Ensures f32 dtype
   │
   ▼
6. TensorAdapter.reshape_to_qwen()
   │  Reshapes to target dimensions
   │
   ▼
7. NormalizedHiddenState
   │  Ready for FedAvg aggregation
   │
   ▼
8. FedAvgAggregator.add_update()
   │  Registers weight update from node
   │
   ▼
9. FedAvgAggregator.aggregate()
   │  Applies Krum filtering
   │  Returns AggregationResult
```

## Data Flow: Staking → API v2

```
1. Node Registration
   │
   ▼
2. ResourceCommitment::new()
   │  CPU, RAM, GPU, bandwidth, storage
   │
   ▼
3. ResourceRegistry.register()
   │  Validates uniqueness
   │  Stores commitment
   │
   ▼
4. StakingProof generation
   │  Proof of resource commitment
   │
   ▼
5. ResourceRegistry.verify_proof()
   │  Validates proof
   │  Updates reputation
   │
   ▼
6. API v2 Endpoint: GET /api/v2/staking/registry
   │  Returns registry stats via callback
   │
   ▼
7. AuthValidator.validate_signature()
   │  Ed25519 signature check
   │  X-Node-Signature header
   │
   ▼
8. ApiResponse<T> serialization
   │  JSON response with status + data
```

## Feature Flag Matrix

| Module | `core-only` | `phase6-core` | `phase6-sprint2` | `phase6-experimental` |
|--------|:-----------:|:-------------:|:----------------:|:---------------------:|
| `adapter.rs` | ❌ | ✅ | ✅ | ✅ |
| `schema.rs` | ❌ | ✅ | ✅ | ✅ |
| `onnx_adapter.rs` | ❌ | ❌ | ✅ | ✅ |
| `avg_aggregator.rs` | ❌ | ✅ | ✅ | ✅ |
| `sync_protocol.rs` | ❌ | ✅ | ✅ | ✅ |
| `proof.rs` | ❌ | ❌ | ✅ | ✅ |
| `registry.rs` | ❌ | ❌ | ✅ | ✅ |
| `auth.rs` | ❌ | ❌ | ✅ | ✅ |
| `routes.rs` | ❌ | ❌ | ✅ | ✅ |
| `phase6/mod.rs` | ❌ | ✅ | ✅ | ✅ |

## Test Coverage Matrix

| Test Suite | Unit | Integration | E2E | Total |
|------------|------|-------------|-----|-------|
| `onnx_adapter` | 11 | 1 | 1 | 13 |
| `auth` | 10 | 1 | 1 | 12 |
| `registry` | 9 | 2 | 2 | 13 |
| `routes` | 11 | 0 | 1 | 12 |
| `phase6/mod` | 5 | 0 | 0 | 5 |
| **Total** | **46** | **4** | **5** | **55** |

## Error Propagation

| Error Type | Source | Handler | Recovery |
|------------|--------|---------|----------|
| `OnnxError` | `onnx_adapter` | Return `Result<Tensor, OnnxError>` | Log + fallback |
| `AuthError` | `auth` | Return `StatusCode::UNAUTHORIZED` | Reject request |
| `AdapterError` | `adapter` | Return `Result<NormalizedHiddenState>` | Skip node |
| `anyhow::Error` | `registry` | Return `Result<()>` | Log + continue |

## Performance Characteristics

| Operation | Expected Latency | Throughput |
|-----------|-----------------|------------|
| ONNX load (placeholder) | <1ms | N/A |
| ONNX load (real) | 100-500ms | 1 model/s |
| Tensor normalization | <1ms | 1000s/s |
| Ed25519 verification | <1ms | 1000s/s |
| FedAvg aggregation | <10ms | 100s/s |
| Registry operations | <1ms | 10000s/s |
