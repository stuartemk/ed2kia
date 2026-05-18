# Changelog — ed2kIA

All notable changes to this project will be documented in this file.

Format based on [Keep a Changelog](https://keepachangelog.com/).

---

## [v2.1.0-sprint2] — 2026-05-18

### 🎉 Sprint Summary

**v2.1.0-sprint2** delivers the **3 Web Viability Pillars** required for browser-based P2P node operation: **Relay Server** (WebRTC/Circuit Relay v2 signaling), **WASM Micro-Sharding** (tensor chunking for wasm32 peers ≤50MB), and **WASM Telemetry Bridge** (wasm-bindgen CustomEvent dispatch to browser DOM). These pillars enable reliable connectivity, memory-safe tensor processing, and real-time inference feedback for web peers.

| Metric | Value |
|--------|-------|
| **Feature Gates** | 3 new (`v2.1-relay-server`, `v2.1-wasm-micro-sharding`, `v2.1-wasm-telemetry`) + 4 inherited |
| **Tests** | +37 new (14 relay + 23 sharding) = 3089 total PASS |
| **CI Jobs** | 12 jobs (matrix extended + wasm-telemetry-check) |
| **Coverage** | ≥80% (tracking via cargo-llvm-cov) |
| **OSSF Score** | 8.5/10 (PASSING) |
| **Security** | 0 CVEs introduced, 0 unsafe code |

### Added

- **Relay Server ("El Faro")** — WebRTC/Circuit Relay v2 signaling scaffold for browser P2P connectivity ([`src/relay_server/mod.rs`](src/relay_server/mod.rs))
  - `RelayNode`, `RelayCircuit`, `RelayTransport`, `RelayConfig` structs
  - Circuit creation, validity checking, expiration cleanup
  - 14 unit tests covering full lifecycle

- **WASM Micro-Sharding** — Tensor chunking for wasm32 peers with ≤50MB size limits ([`src/sae/wasm_sharding.rs`](src/sae/wasm_sharding.rs))
  - `WasmPeerProfile`, `TensorShard`, `ShardedTensor` structs
  - `shard_tensor_for_wasm()` with candle-core slicing
  - `reconstruct_tensor()` for lossless reassembly
  - `detect_wasm_peer()` + `estimate_tensor_size_mb()` utilities
  - 23 unit tests covering sharding lifecycle

- **WASM Telemetry Bridge** — wasm-bindgen + web-sys CustomEvent dispatch from Rust to browser DOM ([`src/mvp_core/inference_bridge.rs`](src/mvp_core/inference_bridge.rs))
  - `emit_inference_complete()` function for real-time inference events
  - Browser Node HTML updated with telemetry log UI + event listener

### Changed

- **CI/CD Pipeline** — Feature gate matrix extended with `v2.1-relay-server` + `v2.1-wasm-micro-sharding`
- **CI/CD Pipeline** — New `wasm-telemetry-check` job (job #12) with wasm32 target + HTML listener verification
- **Cargo.toml** — 3 new feature gates added (NOT in default/stable)
- **lib.rs** — Relay server + WASM sharding modules conditionally compiled

### Security

- **Zero unsafe code** — `#![forbid(unsafe_code)]` enforced
- **Zero telemetry** — No external network calls, no analytics
- **0 CVEs introduced** in this sprint
- **Feature-gated isolation** — v2.1 features strictly excluded from default build

---

## [v2.1.0-sprint1] — 2026-05-17

### 🎉 Sprint Summary

**v2.1.0-sprint1** delivers the **MVP Core Loop validation**, **WASM Browser Node pipeline**, **CI/CD automation** and **activation runbook** for community stewards. This sprint focuses on operational readiness for the Discovery → Distribution → Inference → Collection cycle.

| Metric | Value |
|--------|-------|
| **Feature Gates** | 4 active (`v2.1-mvp-core`, `v2.1-wasm-browser`, `v2.1-observability`, `v2.1-security-hardening`) |
| **CI Jobs** | 11 jobs in matrix (wasm-build, mvp-core-validation, clippy, test, audit, ...) |
| **Tests** | 27 PASS (MVP Core Loop) + 3025 PASS (stable) |
| **Coverage** | ≥80% (tracking via cargo-llvm-cov) |
| **OSSF Score** | 8.5/10 (PASSING) |
| **Security** | 0 CVEs introduced, 0 unsafe code |

### Added

- **MVP Core Loop Module** — Isolated Discovery → Distribution → Inference → Collection cycle with 27 unit tests ([`src/mvp_core/`](src/mvp_core/))
- **WASM Browser Node Scaffold** — `wasm32-unknown-unknown` target for P2P in browser ([`src/wasm/`](src/wasm/))
- **Build Script: build-wasm.sh** — POSIX script for CI WASM builds with trunk ([`scripts/build-wasm.sh`](scripts/build-wasm.sh))
- **Trunk.toml** — Trunk configuration for WASM bundling
- **Browser Node HTML** — Minimal HTML page for testing WASM node in browser ([`browser-node.html`](browser-node.html))
- **Validation Script: validate-mvp-flow.sh** — Automated MVP Core Loop validation (tests, bench, check) ([`scripts/validate-mvp-flow.sh`](scripts/validate-mvp-flow.sh))
- **CI/CD: WASM Build Job** — GitHub Actions job for WASM compilation + trunk build ([`.github/workflows/ci.yml`](.github/workflows/ci.yml))
- **CI/CD: MVP Core Validation Job** — Runs `cargo test --features v2.1-mvp-core --lib mvp_core`
- **Activation Runbook** — Pre-flight, activation, post-activation, rollback procedures ([`docs/operations/activation-package-v2.1.md`](docs/operations/activation-package-v2.1.md))
- **Stewardship Readiness Doc** — Quick commands, emergency protocols, MVP/WASM pipeline ([`docs/operations/stewardship-readiness-v2.1.md`](docs/operations/stewardship-readiness-v2.1.md))
- **Adoption Manifesto** — Community-facing narrative for v2.1 features ([`docs/operations/adoption-manifesto-v2.1.md`](docs/operations/adoption-manifesto-v2.1.md))

### Changed

- **CI/CD Pipeline** — Added `wasm-build` and `mvp-core-validation` jobs to matrix (11 total jobs)
- **Feature Gates** — 4 active gates in CI matrix, NOT in default/stable
- **Cargo.toml** — Updated feature gates for `v2.1-mvp-core` and `v2.1-wasm-browser`

### Security

- **Zero unsafe code** — `#![forbid(unsafe_code)]` enforced
- **Zero telemetry** — No external network calls, no analytics
- **0 CVEs introduced** in this sprint
- **Feature-gated isolation** — v2.1 features strictly excluded from default build

### Operations

- **MVP Core Loop validated** — 27 tests PASS, 0 panics, cycle steps verified
- **Activation Runbook ready** — Human operator procedures documented
- **CI/CD automation** — WASM + MVP validation in every PR

---

## [v2.0.0-stable] — 2026-05-16

### 🎉 Release Summary

**ed2kIA v2.0.0-stable** marks the transition to **STEWARDSHIP MODE** — autonomous operations, community governance, and RFC-driven evolution. This release consolidates FASE 81-99, delivering GUI desktop (Tauri), ZKP multi-curve, observability scaffold, security monitoring pipeline, and full constitutional governance.

| Metric | Value |
|--------|-------|
| **Tests** | 3025 passing (99.7% pass rate) |
| **Coverage** | ≥80% (tracking via cargo-llvm-cov) |
| **OSSF Score** | 8.5/10 (PASSING) |
| **Modules** | 80+ implemented |
| **Security Audit** | 14 CVEs tracked, 0 critical unmitigated |
| **Mode** | STEWARDSHIP (autonomous) |

### Added

- **GUI Desktop (Tauri Scaffold)** — Neural Steering UI with ethical sliders (empathy, creativity, safety) ([`src/gui/`](src/gui/))
- **ZKP Multi-Curve Setup** — BN254, BLS12-381, Pasta curve support with adaptive selection ([`src/zkp/multi_curve_setup.rs`](src/zkp/multi_curve_setup.rs))
- **Proof Aggregation** — Batch verification + commitment pooling ([`src/zkp/proof_aggregation.rs`](src/zkp/proof_aggregation.rs))
- **Circuit Optimization** — Constraint pooling, Pedersen precomputation, benchmarks ([`src/zkp/circuit_optimization.rs`](src/zkp/circuit_optimization.rs))
- **Circuit Optimizer** — Adaptive circuit selection by statement complexity ([`src/zkp/circuit_optimizer.rs`](src/zkp/circuit_optimizer.rs))
- **WASM Mobile Bridge** — Lightweight P2P sync adapter for mobile/WASM targets ([`src/wasm/mobile_bridge.rs`](src/wasm/mobile_bridge.rs))
- **API Explorer v1** — REST endpoints for 3D concept visualization, activations, steering signals ([`src/api/explorer_v1.rs`](src/api/explorer_v1.rs))
- **API Auth v2** — Ed25519 signature validation for API endpoints ([`src/api/auth.rs`](src/api/auth.rs))
- **Async Steering v1** — Late correction signals for distributed tensor pipelines ([`src/protocol/async_steering.rs`](src/protocol/async_steering.rs))
- **Quantization v3** — Per-element FP8/INT4 for tensor payload reduction ([`src/bridge/quantization.rs`](src/bridge/quantization.rs))
- **Reputation Proof Schema** — Ed25519-based reputation proofs with tier system ([`src/reputation/proof_schema.rs`](src/reputation/proof_schema.rs))
- **Observability Scaffold** — Prometheus/Grafana metrics (feature-gated `v2.1-observability`) ([`src/observability/`](src/observability/))
- **v2.1 Structural Scaffold** — Feature-gated placeholder modules (GUI, ZKP v3, Enterprise) ([`src/v2_1/`](src/v2_1/))
- **Security Monitoring Pipeline** — Weekly `cargo audit` cron job (Mondays 03:00 UTC) ([`.github/workflows/security-monitor.yml`](.github/workflows/security-monitor.yml))
- **Testnet Infrastructure** — Docker Compose scaffold + systemd unit templates ([`infra/`](infra/))
- **Voting Dashboard Template** — Weighted tiers (Novice 0.5 → Guardian 3.0), 30% quorum, 60% majority ([`docs/community/voting-dashboard-template.md`](docs/community/voting-dashboard-template.md))
- **Voting Tally Script** — POSIX shell script for weighted vote tallying ([`scripts/voting-tally.sh`](scripts/voting-tally.sh))
- **Security Alert Script** — Parses security reports, generates Slack/webhook alerts ([`scripts/security-alert.sh`](scripts/security-alert.sh))
- **Project Constitution** — Governance charter, ethical principles, decision matrix ([`docs/governance/project-constitution.md`](docs/governance/project-constitution.md))
- **RFC Process** — Formal Request for Comments (RFC-001/002/003) ([`docs/governance/rfc-tracking.md`](docs/governance/rfc-tracking.md))
- **Milestone Tracker** — Community milestone tracking + badge generator ([`docs/community/milestone-tracker.md`](docs/community/milestone-tracker.md))
- **Autonomous Health Check** — Daily 02:00 UTC health monitoring script ([`scripts/autonomous_health_check.sh`](scripts/autonomous_health_check.sh))
- **Early Access Program** — 50 participants, 8-week duration ([`docs/early_access_program_v2.0.md`](docs/early_access_program_v2.0.md))
- **Sustainability Framework** — Partnership playbook + grant execution support

### Changed

- **README.md** — Updated to v2.0.0-stable, diplomatic/collaborative tone, real metrics (3025 tests, OSSF 8.5/10)
- **SECURITY.md** — Updated with Threat Model v2.0, CVE Matrix Q1 2027, monitoring pipeline
- **CI/CD Pipeline** — Added `feature-gate-check` and `voting-script-validation` jobs
- **Feature Gates** — Added `v2.1-sprint1`, `v2.1-gui`, `v2.1-zkp-v3`, `v2.1-enterprise`, `v2.1-observability` (NOT in default)
- **Operational Mode** — Transitioned from DEVELOPMENT to STEWARDSHIP (autonomous loop)
- **Threat Model** — Updated to v2.0 (17 threats identified and mitigated)
- **Security Audit** — Q1 2027 audit: 14 CVEs tracked, remediation plan created

### Deprecated

- **Phase-based roadmap** — Superseded by RFC-driven evolution (v2.1 → v3.0)
- **FASE 1-6 documentation** — Legacy phases completed, archived in source-of-truth

### Removed

- N/A (no breaking removals in v2.0.0-stable)

### Fixed

- **Cargo.toml versioning** — Documented discrepancy: `1.6.0-stable` (Cargo) vs `v2.0.0-stable` (operational)
- **Feature gate isolation** — v2.1-* features strictly excluded from default gate
- **Shell script validation** — Added CI job for `bash -n` syntax checks

### Security

- **14 CVEs identified** (wasmtime 17.0.3 sandbox escapes, rustls-webpki 0.101.7 TLS)
- **5 unmaintained dependencies** (mach, paste, ring 0.16, rustls-pemfile, yaml-rust)
- **1 unsound dependency** (lru 0.12.5)
- **Remediation plan** — Feature-gated upgrades under `v2.1-security-hardening` (Q2-Q3 2027)
- **OSSF Score: 8.5/10** (PASSING)
- **Zero unsafe code** — `#![forbid(unsafe_code)]` enforced
- **Zero telemetry** — No external network calls, no analytics

### Stewardship

- **Autonomous Loop** — Daily health checks, CI maintenance, weekly security audits
- **RFC Process** — RFC-001 (Feedback), RFC-002 (Observability), RFC-003 (Testnet)
- **Community Governance** — Constitution v1.0, voting tiers, quorum rules
- **Quarterly Review** — Template + autonomous watchdog workflow
- **Handover Package** — Prompt v13.0, signoff JSON, operations guide

---

## [v1.9.0-stable] — 2026-05-16

### Added

- **ZKP Aggregation** — Proof aggregation + neural steer UI foundation
- **Production Hardening** — Mobile GUI foundation, stability improvements
- **OSSF Compliance** — Score 8.5/10, security.md update
- **Release Notes & Migration Guide** — v1.8 → v1.9 migration documentation
- **Community Scaling Strategy** — Grant submission package, ambassador program
- **First-PR Automation** — CODEOWNERS update, automated PR triage

### Changed

- **CI/CD Pipeline** — Optimized for stable maintenance
- **Operational Prompt** — v9.0 with v2.0 architectural vision
- **Source of Truth** — Final reconciliation, versioning alignment

---

## [v1.8.0-beta.1] — 2026-05-16

### Added

- **Beta Release** — CI validation, tester onboarding, feedback pipeline
- **Performance Monitoring** — Bug triage automation, P0-P3 matrix
- **DevTools** — Justfile, setup.sh, docker-compose for local dev
- **Grants Tracker** — Follow-up tracker, mentorship automation

### Changed

- **README.md** — Phase 6 completion sync, roadmap alignment
- **CONTRIBUTING.md** — Updated with grants + mentorship info

---

## [v1.6.0-stable] — 2026-05-16

### Added

- **Core P2P Network** — libp2p with KAD + mDNS
- **SAE Loader** — Candle-based .safetensors loading
- **LayerRouter** — Dynamic sharding + leases
- **Tensor Flow Pipeline** — Node A → Node B tensor routing
- **ZKP Circuits** — Pedersen commitments + Fiat-Shamir (BN254)
- **WASM Sandbox** — wasmtime isolation (256MB memory limit)
- **Human Feedback CLI** — Interactive TTY + batch JSON modes
- **Governance** — Ed25519 signed proposals + time-locked voting
- **Reputation System** — Anti-Sybil scoring + 50%/30d decay
- **Web Dashboard** — Alpine.js UI + Prometheus metrics
- **RLHF Loop** — Feedback store (redb) + trainer loop + drift detection

---

## [Unreleased]

### Planned (v2.1 — Post-RFC)

- **wasmtime upgrade** — 17.0.3 → >=24.0.7 (feature-gated `v2.1-security-hardening`)
- **libp2p upgrade** — rustls-webpki >=0.103.13 (feature-gated)
- **GUI v2.1** — Full Tauri desktop app (RFC-001)
- **ZKP v3** — Recursive prover, cross-chain proof adapter
- **Enterprise** — SSO, K8s Operator, compliance reporting
- **Observability** — Full Prometheus/Grafana integration (RFC-002)
- **Testnet v2.1** — Docker Compose + systemd deployment (RFC-003)

---

**ed2kIA** — Red descentralizada de interpretabilidad de IA para el beneficio humano.

[
