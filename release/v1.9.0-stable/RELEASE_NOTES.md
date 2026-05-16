# Release Notes — ed2kIA v1.9.0-stable

**Release Date:** 2026-05-16
**Version:** 1.9.0-stable
**Previous Stable:** v1.6.0-stable
**Branch:** main
**Feature Gates:** `v1.9-sprint1`, `v1.9-sprint2`

---

## Overview

ed2kIA v1.9.0-stable is a major production release unifying FASE 7 strategic milestones with the v1.9 technical roadmap. This release delivers ZKP proof aggregation, neural steering UI controls, security audit compliance (OSSF 8.5/10), community scaling infrastructure, and grant preparation packages.

**Key Achievements:**
- 64 new unit tests across proof_aggregation (33) and neural_steer_ui (31)
- OSSF compliance audit completed (22 CVEs found, 0 critical in production paths)
- Community onboarding automation (first-PR pipeline, feedback processing)
- Grants submission package ready (Gitcoin, NSF, OSSF)

---

## FASE 7 Milestones Completed

| FASE | Title | Status | Commit |
|------|-------|--------|--------|
| FASE 68 | Unificación Estratégica FASE 7 ↔ v1.9 | ✅ | 6604403 |
| FASE 69 | Sprint 1 — Production Hardening & Mobile GUI | ✅ | 5921253 |
| FASE 70 | Tracking Unificado & Dashboard v3 | ✅ | fca7e7b |
| FASE 71 | Operational Prompt v7.0 & Handover | ✅ | 2f6f2c1 |
| FASE 72 | Sprint 2 — ZKP Aggregation & Neural Steer UI | ✅ | eeb5bfd |
| FASE 73 | Integración Feedback Beta & Paquete Grants | ✅ | afca75e |
| FASE 74 | Automatización Primeros PRs & Onboarding | ✅ | ba17b3d |
| FASE 75 | Weekly Cycle 5 & Operational Prompt v8.0 | ✅ | 4b134b4 |
| FASE 76 | Security Audit & OSSF Compliance | ✅ | 6751ad1 |

---

## New Features

### ZKP Proof Aggregation (`src/zkp/proof_aggregation.rs`)

Batch verification and commitment pooling for ZKP proofs:

- **AggregationBatch:** Manage proof batches with size limits, finalization, and verification
- **ProofAggregator:** Multi-batch management with creation, verification, and cleanup
- **AggregationMetrics:** Track verification times, success rates, and reduction ratios
- **Deterministic commitments:** SHA-256 based commitment hashing for reproducibility

**API:**
```rust
let mut aggregator = ProofAggregator::new(max_batch_size: 100, max_batches: 10);
let batch_id = aggregator.create_batch(current_ms)?;
aggregator.add_proof_to_batch(&batch_id, proof_id)?;
aggregator.finalize_batch(&batch_id, current_ms)?;
let verified = aggregator.verify_batch(&batch_id)?;
```

### Neural Steer UI (`src/gui/neural_steer_ui.rs`)

Ethical slider components for AI behavior control:

- **SteeringSlider:** Empathy, creativity, and safety sliders with bounds validation
- **NeuralSteerConfig:** JSON-serializable configuration with safety checks
- **SteeringSignalBridge:** Config application with rollback support
- **Safety thresholds:** Automatic violation detection and clamping

**API:**
```rust
let mut config = NeuralSteerConfig::new(empathy: 0.7, creativity: 0.5, safety: 0.9, updated_at_ms);
assert!(config.is_safe());
let signal = config.compute_signal();
let json = config.to_json()?;
let restored = NeuralSteerConfig::from_json(&json)?;
```

### Mobile GUI Foundation (`src/gui/mobile_foundation.rs`)

Tauri/React Native bridge foundation with resource management:

- **ResourceSliderConfig:** Battery, thermal, and bandwidth limits for mobile nodes
- **Node state management:** Online/offline/syncing states with transitions
- **WASM target support:** 64MB memory-limited compilation

### Security & Compliance

- **OSSF Compliance Report:** `docs/security/ossf-compliance-report.md`
- **Security Policy Updated:** `SECURITY.md` v1.9-stable with audit section
- **CVE Scan:** 22 findings documented (0 Critical, 3 High, 8 Medium, 11 Low)
- **WASM Sandbox Verification:** Cranelift-only, 256MB memory cap, minimal WASI

### Community Infrastructure

- **First PR Automation:** `docs/community/first-pr-automation.md`
- **Feedback Processing:** `scripts/process_feedback.sh`
- **Auto-merge PRs:** `scripts/auto_merge_pr.sh`
- **Onboarding Pipeline:** Contributor funnel documentation

---

## Feature Gates

### Cargo.toml Features

```toml
[features]
v1.9-sprint1 = [
    "mobile-foundation",
    "async-steering-hardening",
    "zkp-circuit-optimization"
]
v1.9-sprint2 = [
    "proof-aggregation",
    "neural-steer-ui"
]
```

### Enabling Features

```bash
# Build with Sprint 1 features
cargo build --release --features v1.9-sprint1

# Build with Sprint 2 features
cargo build --release --features v1.9-sprint2

# Build with all v1.9 features
cargo build --release --features "v1.9-sprint1,v1.9-sprint2"
```

---

## Breaking Changes

### None

v1.9.0-stable maintains full backward compatibility with v1.6.0-stable API. All new modules are additive.

---

## Performance Metrics

| Metric | v1.6.0 | v1.9.0 | Change |
|--------|--------|--------|--------|
| Unit tests | 450+ | 514+ | +64 |
| ZKP batch verification | N/A | ~2ms/batch | New |
| Neural steer config serialization | N/A | <100µs | New |
| OSSF Score | N/A | 8.5/10 | New |

---

## Validation Summary

| Check | Result | Details |
|-------|--------|---------|
| `cargo check` | ✅ PASS | 0 errors |
| `cargo clippy` | ✅ PASS | 3 style warnings (non-blocking) |
| `cargo test` | ✅ PASS | 514/514 tests passing |
| `cargo audit` | ⚠️ 22 findings | 0 critical in production paths |
| OSSF Scorecard | ✅ 8.5/10 | Passing threshold |

---

## Rollback Procedure

If issues are discovered in v1.9.0-stable:

1. **Immediate rollback:**
   ```bash
   git checkout v1.6.0-stable
   cargo build --release --features stable
   ```

2. **Partial rollback (disable feature gate):**
   ```bash
   # Disable v1.9 Sprint 2 features
   cargo build --release --features stable --no-default-features
   ```

3. **Report issues:**
   - Open GitHub Issue with severity classification
   - Follow `SECURITY.md` incident response process
   - Critical: 24h response, High: 72h response

---

## Upgrade Path

### From v1.6.0-stable

```bash
git pull origin main
cargo build --release
```

No configuration changes required. See [`docs/migration/v1.8-to-v1.9.md`](../../docs/migration/v1.8-to-v1.9.md) for detailed migration steps.

### From v1.8.0-beta

```bash
git pull origin main
# Feature flags updated: v1.8-sprint1/2 → v1.9-sprint1/2
cargo build --release --features "v1.9-sprint1,v1.9-sprint2"
```

Update Cargo.toml feature references from `v1.8-*` to `v1.9-*`.

---

## Known Issues

| Issue | Severity | Workaround |
|-------|----------|------------|
| wasmtime 17.0 CVEs (15 findings) | Medium | Minimal WASI config avoids attack surfaces |
| lru 0.12.5 stacked borrows | Medium | Transitive only, not exposed |
| WSL bash unavailable on Windows | Low | Scripts documented for POSIX environments |

Full security details: [`docs/security/ossf-compliance-report.md`](../../docs/security/ossf-compliance-report.md)

---

## Contributors

- ed2kIA Core Team
- Community contributors (see GitHub)

---

## Checksums

```
# Generate after build
sha256sum target/release/ed2kia > ed2kia-v1.9.0-stable.sha256
```

---

## Links

- **Security Policy:** [`SECURITY.md`](../../SECURITY.md)
- **OSSF Report:** [`docs/security/ossf-compliance-report.md`](../../docs/security/ossf-compliance-report.md)
- **Migration Guide:** [`docs/migration/v1.8-to-v1.9.md`](../../docs/migration/v1.8-to-v1.9.md)
- **Changelog:** [`release/changelog.md`](../changelog.md)
- **Source:** https://github.com/ed2kia/ed2kIA

---

*Released: 2026-05-16 | ed2kIA v1.9.0-stable*
