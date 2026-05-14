# Integration Hooks – Fase 6 Sprint 1 → v0.5.0

**Purpose:** Define the integration points between the new `phase6-core` modules and the existing v0.5.0 stable codebase.

---

## 1. Feature Flag Isolation

```
phase6-core (Sprint 1)
    └── phase6-experimental (full Fase 6)
            └── core-only (Fases 1-5, default)
```

- **Zero impact on `core-only` builds**: The `phase6-core` modules are completely excluded when the feature is not enabled.
- **Default build**: `cargo build` (uses `core-only`) → no phase6 code compiled.
- **Sprint 1 build**: `cargo build --features "phase6-core"` → compiles adapter + aggregator.
- **Full Fase 6 build**: `cargo build --features "phase6-experimental"` → includes all Fase 6 modules.

---

## 2. Module Registration (`src/main.rs`)

```rust
// Existing v0.5.0 modules (unchanged)
mod p2p { pub mod protocol; pub mod swarm; }
mod sae { pub mod loader; pub mod router; }
// ...

// NEW: Phase 6 modules (feature gated)
#[cfg(any(feature = "phase6-core", feature = "phase6-experimental"))]
mod interoperability { pub mod adapter; pub mod schema; }

#[cfg(any(feature = "phase6-core", feature = "phase6-experimental"))]
mod federation { pub mod avg_aggregator; pub mod sync_protocol; }

#[cfg(feature = "phase6-core")]
mod phase6;  // Re-exports
```

---

## 3. Integration Points

### 3.1 TensorAdapter ↔ SAE Loader

| Component | Hook | Description |
|-----------|------|-------------|
| `sae::loader::SAEModel` | `forward(input: &Tensor) -> Tensor` | Returns hidden state tensor |
| `interoperability::TensorAdapter` | `adapt(tensor, source_model)` | Normalizes to Qwen-Scope schema |

**Flow:**
```
SAEModel::forward() → Tensor
    ↓
TensorAdapter::normalize_dtype() → Tensor (f32)
    ↓
TensorAdapter::reshape_to_qwen() → Tensor [batch, 3584]
    ↓
NormalizedHiddenState { data, source_model, ... }
```

### 3.2 FedAvgAggregator ↔ LayerRouter

| Component | Hook | Description |
|-----------|------|-------------|
| `sae::router::LayerRouter` | `request_lease()` | Assigns SAE layers to nodes |
| `federation::FedAvgAggregator` | `add_update(update)` | Receives weight deltas from nodes |

**Flow:**
```
Node A trains local SAE layer → WeightUpdate { node_id, layer_id, weight_deltas }
    ↓
FedAvgAggregator::add_update(update)
    ↓
FedAvgAggregator::aggregate(layer_id)
    ↓
AggregationResult { final_weights, confidence, ... }
    ↓
Distribute to peers via GossipSub
```

### 3.3 phase6/mod.rs ↔ External API

```rust
// Public re-exports for consumers
#[cfg(feature = "phase6-core")]
pub use crate::phase6::{
    interoperability::{TensorAdapter, AdapterError, SourceModel},
    federation::{FedAvgAggregator, AggregationResult, WeightUpdate},
};
```

---

## 4. Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                     ed2kIA v0.5.0 + phase6-core             │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────┐    Tensor     ┌──────────────┐               │
│  │ SAE Model │─────────────→│ TensorAdapter │               │
│  │ (candle)  │              │  - dtype cast │               │
│  └──────────┘              │  - reshape    │               │
│                            │  - padding    │               │
│                            └──────┬───────┘               │
│                                   │ NormalizedHiddenState  │
│                            ┌──────▼───────┐               │
│                            │  P2P Swarm   │               │
│                            │  (GossipSub) │               │
│                            └──────┬───────┘               │
│                                   │ WeightUpdate           │
│                            ┌──────▼──────────┐            │
│                            │ FedAvgAggregator│            │
│                            │  - Krum filter  │            │
│                            │  - weighted avg │            │
│                            └──────┬──────────┘            │
│                                   │ AggregationResult      │
│                            ┌──────▼──────────┐            │
│                            │  SAE Loader     │            │
│                            │  (apply weights)│            │
│                            └─────────────────┘            │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 5. Backward Compatibility

| Aspect | Status |
|--------|--------|
| v0.5.0 API | ✅ Unchanged |
| Default features | ✅ `core-only` (no phase6) |
| Binary size (no phase6) | ✅ No increase |
| Test suite (no phase6) | ✅ 76 tests pass |
| Existing CI pipeline | ✅ Compatible |

---

## 6. Migration Path

```
v0.5.0 (stable)
    ↓  add phase6-core feature
v0.5.0 + phase6-core (Sprint 1)
    ↓  add remaining Fase 6 modules
v0.6.0-dev (phase6-experimental)
    ↓  stabilize
v0.6.0 (stable, phase6 default)
```

---

## 7. Validation Commands

```bash
# Verify v0.5.0 still works
cargo build
cargo test

# Verify Sprint 1 modules
cargo build --features "phase6-core"
cargo test --features "phase6-core"

# Verify full Fase 6
cargo build --features "phase6-experimental"
cargo test --features "phase6-experimental"

# Clippy (zero warnings)
cargo clippy --features "phase6-core" -- -D warnings
```
