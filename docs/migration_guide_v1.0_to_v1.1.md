# Migration Guide: ed2kIA v1.0.0 → v1.1.0 STABLE

**Date:** May 8, 2026
**Difficulty:** Easy (Zero Breaking Changes)

---

## Summary

ed2kIA v1.1.0 is a **drop-in replacement** for v1.0.0 with zero breaking API changes. All existing modules, types, and functions remain compatible. The primary change is the consolidation of sprint feature flags into the `stable` feature.

---

## What Changed

### 1. Feature Flag Consolidation

**v1.0.0:**
```toml
# Cargo.toml
[features]
default = ["stable"]
stable = ["phase6-core", "phase6-sprint2", ...]
"v1.1-sprint1" = []
"v1.1-sprint2" = []
"v1.1-sprint3" = []
"v1.1-sprint4" = []
"v1.1-sprint5" = []
```

**v1.1.0:**
```toml
# Cargo.toml
[features]
default = ["stable"]
stable = [
    "phase6-core", "phase6-sprint2", ...,
    "v1.1-sprint1", "v1.1-sprint2", "v1.1-sprint3", "v1.1-sprint4", "v1.1-sprint5"
]
```

**Impact:** If you were using `--features stable`, you now automatically get all v1.1.0 modules. No changes required.

### 2. Version String Update

| Constant | v1.0.0 | v1.1.0 |
|----------|--------|--------|
| `VERSION` | `"1.0.0"` | `"1.1.0"` |
| `SPRINT_IDENTIFIER` | `"v1.0.0-stable"` | `"v1.1.0-stable"` |

**Impact:** Only affects version reporting. No functional changes.

### 3. Module Availability

Previously sprint-gated modules are now available under `stable`:

| Module | Old Feature Gate | New Feature Gate |
|--------|-----------------|------------------|
| `wasm_sandbox_v2` | `v1.1-sprint1` | `stable` |
| `wasm_profiler` | `v1.1-sprint1` | `stable` |
| `capability_registry` | `v1.1-sprint1` | `stable` |
| `cross_model_router` | `v1.1-sprint1` | `stable` |
| `federation_v2_sprint1` | `v1.1-sprint1` | `stable` |
| `liquid_v2` | `v1.1-sprint2` | `stable` |
| `voting_mechanism` | `v1.1-sprint2` | `stable` |
| `dynamic_engine` | `v1.1-sprint2` | `stable` |
| `contract_manager` | `v1.1-sprint2` | `stable` |
| `streaming_metrics` | `v1.1-sprint2` | `stable` |
| `realtime` (web) | `v1.1-sprint2` | `stable` |
| `async_prover` | `v1.1-sprint3` | `stable` |
| `verifier_pool` | `v1.1-sprint3` | `stable` |
| `batch_accumulator` | `v1.1-sprint3` | `stable` |
| `marketplace_v2` | `v1.1-sprint3` | `stable` |
| `bridge_v2` | `v1.1-sprint3` | `stable` |
| `loop_v2` | `v1.1-sprint4` | `stable` |
| `steering_engine` | `v1.1-sprint4` | `stable` |
| `confidence_calculator` | `v1.1-sprint4` | `stable` |
| `gradient_normalizer` | `v1.1-sprint4` | `stable` |
| `trust_sync` | `v1.1-sprint4` | `stable` |
| `cross_model_scaler` | `v1.1-sprint4` | `stable` |
| `realtime_backend` | `v1.1-sprint4` | `stable` |
| `ws_alignment_stream` | `v1.1-sprint4` | `stable` |
| `sse_metrics` | `v1.1-sprint4` | `stable` |
| `dashboard_v2` | `v1.1-sprint5` | `stable` |
| `ws_dashboard_stream` | `v1.1-sprint5` | `stable` |
| `adaptive_router_v2` | `v1.1-sprint5` | `stable` |
| `predictive_balancer` | `v1.1-sprint5` | `stable` |

---

## Migration Steps

### Step 1: Update Cargo.toml

```diff
 [dependencies]
-ed2kia = "1.0.0"
+ed2kia = "1.1.0"
```

### Step 2: Update Feature Flags (Optional)

If you were using individual sprint features:

```diff
 [dependencies]
-ed2kia = { version = "1.0.0", features = ["v1.1-sprint1", "v1.1-sprint2"] }
+ed2kia = { version = "1.1.0", features = ["stable"] }
```

**Note:** The individual sprint features (`v1.1-sprint1`, etc.) still exist as sub-features of `stable` for backward compatibility, but using `stable` directly is recommended.

### Step 3: Rebuild

```bash
cargo build --release
```

### Step 4: Run Tests

```bash
cargo test --features stable
```

---

## New Module Quick Reference

### Dashboard v2

```rust
use ed2kia::ui::dashboard_v2::{DashboardState, DashboardConfig, DashboardMetric};

let mut dashboard = DashboardState::default();
dashboard.register_node("node-1".to_string());
dashboard.record_metric(DashboardMetric::CpuUsage, 0.75, None);
let snapshot = dashboard.get_snapshot().unwrap();
```

### Adaptive Router v2

```rust
use ed2kia::interoperability::adaptive_router_v2::{AdaptiveRouter, AdaptiveRouterConfig};

let mut router = AdaptiveRouter::default();
router.register_node("node-1".to_string(), "qwen".to_string());
let decision = router.route("qwen", None).unwrap();
router.record_success(&decision.target_node, 50.0).unwrap();
```

### Predictive Balancer

```rust
use ed2kia::scaling::predictive_balancer::{PredictiveBalancer, PredictiveBalancerConfig};

let mut balancer = PredictiveBalancer::default();
balancer.register_node("node-1".to_string());
balancer.record_load("node-1", 100.0, 50.0, 5.0).unwrap();
let prediction = balancer.predict_latency("node-1").unwrap();
let best = balancer.get_best_node(&["node-1".to_string()]).unwrap();
```

### Alignment Loop v2

```rust
use ed2kia::alignment::loop_v2::{AlignmentLoopV2, LoopV2Config};
use ed25519_dalek::SigningKey;

let key = SigningKey::generate(&mut rand::thread_rng());
let mut loop_v2 = AlignmentLoopV2::new(key);
// Ingest feedback, run cycles, apply steering signals
let signals = loop_v2.run_cycle().unwrap();
```

---

## Backward Compatibility

| Aspect | Status |
|--------|--------|
| Public API | ✅ 100% compatible |
| Type signatures | ✅ No changes |
| Module paths | ✅ No changes |
| Feature flags | ✅ Backward compatible |
| Configuration | ✅ No changes |
| Test suite | ✅ All v1.0.0 tests pass |

---

## Known Differences

### 1. Test Feature Requirements

Test configurations now use `required-features = ["stable"]` instead of individual sprint features:

```diff
 [[test]]
 name = "v1_1_sprint5_e2e"
 path = "tests/integration/v1_1_sprint5_e2e.rs"
-required-features = ["v1.1-sprint5"]
+required-features = ["stable"]
```

### 2. Version Reporting

```rust
// v1.0.0
assert_eq!(ed2kia::version(), "1.0.0");
assert_eq!(ed2kia::sprint_identifier(), "v1.0.0-stable");

// v1.1.0
assert_eq!(ed2kia::version(), "1.1.0");
assert_eq!(ed2kia::sprint_identifier(), "v1.1.0-stable");
```

---

## Troubleshooting

### Q: My code uses `#[cfg(feature = "v1.1-sprint1")]`

**A:** This still works. The sprint features are maintained as sub-features of `stable`. However, consider migrating to `#[cfg(feature = "stable")]` for forward compatibility.

### Q: Do I need to update my Cargo.lock?

**A:** Yes. Run `cargo update` to pull the latest dependency versions.

### Q: Will my existing configurations work?

**A:** Yes. All configuration structures remain unchanged.

### Q: Are there any performance implications?

**A:** No. This release includes zero performance regressions. All benchmarks pass within acceptable thresholds.

---

## Rollback Plan

If you need to rollback to v1.0.0:

```bash
# Update Cargo.toml
sed -i 's/ed2kia = "1.1.0"/ed2kia = "1.0.0"/' Cargo.toml

# Update lock file
cargo update

# Rebuild
cargo build --release
```

---

## Support

- **Issues:** https://github.com/ed2kia/ed2kia/issues
- **Documentation:** https://github.com/ed2kia/ed2kia/tree/main/docs
- **Community:** See `docs/CONTRIBUTING.md`

---

**Last Updated:** May 8, 2026
