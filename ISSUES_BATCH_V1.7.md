# ISSUES BATCH v1.7 — Good First Issues

**Sprint:** v1.7 — Latency Mitigation (RFC-001)
**Fecha:** 2026-05-14
**Estado:** Listos para crear via `gh issue create`

---

## Issue 1: perf: optimize FP8 quantization with SIMD intrinsics

### Scope
`src/bridge/quantization.rs` currently uses scalar f32 operations for FP8/INT4 quantization. Introduce SIMD intrinsics (AVX2 on x86_64, NEON on aarch64) to process multiple elements per instruction, targeting 2-4x throughput improvement for tensor quantization/dequantization.

### Acceptance Criteria
- [ ] Implement `quantize_f32_to_fp8_simd()` using `std::arch::x86_64::_mm256_*` intrinsics (feature-gated `target_arch = "x86_64"`)
- [ ] Implement `quantize_f32_to_int4_simd()` with same SIMD approach
- [ ] Fallback to scalar path for non-SIMD targets (existing code)
- [ ] Add benchmark in `benchmarks/benches/tensor_serialization.rs` comparing scalar vs SIMD
- [ ] Maintain precision: FP8 MAPE <2%, INT4 MAPE <10% (match existing tests)
- [ ] All 15 existing quantization tests must still PASS
- [ ] Document SIMD requirements in `benchmarks/README.md`

### References
- RFC-001 §2.2: Aggressive Quantization targets
- `src/bridge/quantization.rs:67` — `quantize_f32_to_fp8()`
- `src/bridge/quantization.rs:160` — `quantize_f32_to_int4()`
- `benchmarks/benches/tensor_serialization.rs` — existing benchmark harness

### Labels
`performance`, `good-first-issue`, `simd`, `quantization`, `v1.7`

### How to Run
```bash
# Validate existing tests
cargo test --features v1.7-sprint1 quantization

# Run benchmark
cargo bench -p ed2kIA-benchmarks --bench tensor_serialization

# Check SIMD support
rustc --print cfg | grep target_arch
```

---

## Issue 2: feat: implement geographic routing priority in LayerRouter

### Scope
Add RTT-based geographic routing to the P2P layer so that tensor requests are routed to the nearest federation node by network latency. Currently, federation selection uses credibility + capacity scoring only. Integrate libp2p RTT metrics into routing decisions per RFC-001 §2.3.

### Acceptance Criteria
- [ ] Add `rtt_ms: f64` field to `FederationNodeV7` in `src/bridge/federation_zkp_bridge_v7.rs`
- [ ] Update `routing_score()` to include RTT component: `score = credibility * (1 / (1 + rtt_ms/100)) * capacity_factor`
- [ ] Add `update_rtt(&mut self, rtt_ms: f64, alpha: f64)` with EMA smoothing
- [ ] Create `select_lowest_rtt_federation(&self, excluded: &[String]) -> Option<&FederationNodeV7>`
- [ ] Add 5+ unit tests covering RTT scoring, updates, and selection edge cases
- [ ] Update `FederationZKPBridgeV7Config` with `rtt_weight: f64` (default 0.3)
- [ ] Document routing formula in module docstring

### References
- RFC-001 §2.3: Geographic Routing via libp2p RTT metrics
- `src/bridge/federation_zkp_bridge_v7.rs:190` — `routing_score()`
- `src/bridge/federation_zkp_bridge_v7.rs:169` — `FederationNodeV7` struct
- libp2p docs: https://docs.rs/libp2p/latest/libp2p/swarm/delay_measurement/

### Labels
`feature`, `good-first-issue`, `p2p`, `routing`, `v1.7`

### How to Run
```bash
# Validate bridge module
cargo check --features v1.7-sprint1

# Run bridge tests
cargo test --features v1.7-sprint1 federation_zkp_bridge_v7

# Run integration tests
cargo test --test v1_6_sprint3_e2e
```

---

## Issue 3: docs: add benchmark contribution guide & criterion setup

### Scope
Create comprehensive documentation for contributors who want to add new benchmarks or run existing benchmark suite. Include Criterion setup, result interpretation, baseline comparison workflow, and CI integration details.

### Acceptance Criteria
- [ ] Create `docs/BENCHMARK_CONTRIBUTING.md` with:
  - Prerequisites (Rust stable, cargo-criterion)
  - How to run full benchmark suite
  - How to add new benchmark (step-by-step with example)
  - Interpreting Criterion output (mean, median, R², sample size)
  - Baseline comparison workflow (before/after table format)
  - CI benchmark integration (`.github/workflows/benchmarks.yml`)
- [ ] Update `benchmarks/README.md` with link to contribution guide
- [ ] Include example benchmark template in `docs/BENCHMARK_CONTRIBUTING.md`
- [ ] Add troubleshooting section (common errors, platform differences)
- [ ] Cross-reference RFC-001 performance targets

### References
- RFC-001: Performance targets table
- `benchmarks/README.md` — existing benchmark docs
- `benchmarks/Cargo.toml` — benchmark dependencies
- Criterion docs: https://bheisler.github.io/criterion.rs/book/

### Labels
`documentation`, `good-first-issue`, `benchmarks`, `v1.7`

### How to Run
```bash
# Verify docs build (if using mdbook)
# Or just validate markdown syntax
markdownlint docs/BENCHMARK_CONTRIBUTING.md

# Run benchmarks to verify commands work
cargo bench -p ed2kIA-benchmarks
```

---

## Issue 4: test: expand async steering edge cases (timeout, backpressure)

### Scope
`src/protocol/async_steering.rs` currently has 13 tests covering basic functionality. Expand test coverage to include edge cases: channel timeout, backpressure when channel is full, signal ordering under concurrent sends, and context window boundary conditions.

### Acceptance Criteria
- [ ] Add `test_steering_channel_timeout` — verify behavior when signal exceeds max delay
- [ ] Add `test_steering_backpressure` — verify graceful handling when channel buffer is full
- [ ] Add `test_steering_signal_ordering` — verify signals are applied in sequence order even if received out of order
- [ ] Add `test_steering_empty_window` — verify error when applying correction to empty context
- [ ] Add `test_steering_boundary_values` — test value = -1.0, 0.0, 1.0 exactly
- [ ] Add `test_steering_large_delay` — test with delay_ms > u32::MAX
- [ ] Add `test_steering_concurrent_sources` — multiple sources sending to same channel
- [ ] Minimum 7 new tests (total 20+ tests in module)
- [ ] All tests must PASS with `cargo test --features v1.7-sprint1 async_steering`

### References
- RFC-001 §2.4: Async Steering Signals for late correction
- `src/protocol/async_steering.rs` — module to extend
- `src/protocol/async_steering.rs:15` — `AsyncSteeringError` variants
- `src/protocol/async_steering.rs:46` — `SteeringSignal` struct

### Labels
`test`, `good-first-issue`, `async`, `steering`, `v1.7`

### How to Run
```bash
# Run async_steering tests
cargo test --features v1.7-sprint1 async_steering

# Run with verbose output
cargo test --features v1.7-sprint1 async_steering -- --nocapture

# Check test coverage (if using tarpaulin)
cargo tarpaulin --features v1.7-sprint1 --out Html
```

---

## Creación Automática

Ejecutar `scripts/create_issues.sh` para crear los 4 issues vía GitHub CLI:

```bash
chmod +x scripts/create_issues.sh
./scripts/create_issues.sh
```

O crear manualmente copiando cada sección a `gh issue create --title "..." --body "..." --label "..."`
