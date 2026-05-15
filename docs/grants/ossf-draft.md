# Open Source Security Foundation (OSSF) Grant Application — ed2kIA v1.8

**Program:** OSSF Security Improvement Fund / OSSF Best Practices Badge
**Project:** ed2kIA — Distributed AI Interpretability Federation
**Repository:** https://github.com/Stuartemk/ed2kIA
**License:** Apache 2.0 + Ethical Use Clause
**Date:** 2026-05-15
**Version:** Draft v1.0

> **DISCLAIMER:** This is a technical draft. All placeholders ([PLACEHOLDER: ...]) must be filled with verified information before submission. OSSF program IDs, deadlines, and funding amounts must be confirmed with the official OSSF portal.

---

## 1. Project Overview

**ed2kIA** is a Rust-based distributed AI interpretability federation that enables transparent, verifiable analysis of Large Language Models (LLMs) through Sparse Autoencoders (SAEs), Zero-Knowledge Proofs (ZKPs), and Ed25519-based reputation.

**Security-Relevant Components:**
- **Async ZKP v14:** Zero-knowledge proof generation and verification (circuit correctness critical)
- **Federation ZKP Bridge v7:** Cross-model proof verification with adaptive routing
- **Ed25519 Reputation Proof Schema:** Cryptographic signature validation for contributor reputation
- **API Explorer v1:** Rate limiting, Ed25519 proof validation, input sanitization
- **Async Steering:** Real-time correction signals with delay decay (RFC-001)

**Current Status:** v1.7.0-stable, 2891 tests passing, v1.8-sprint1 active development.

---

## 2. Security Focus Areas

### 2.1 WASM Sandbox (Planned)
**Goal:** Extract WASM-compatible core for browser-safe execution of SAE interpretation.

**Threat Model:**
- Untrusted SAE models could contain malicious code
- Browser execution requires sandbox isolation
- Memory safety critical (Rust → WASM provides guarantees)

**Plan:**
1. Identify WASM-compatible modules (no `unsafe`, no FFI)
2. Extract core SAE loader + feature analyzer to WASM target
3. Add boundary checks for all external inputs
4. Fuzz test with `cargo fuzz` (libFuzzer)

**Timeline:** Sprint 2 (Q3 2026)

### 2.2 ZKP Anti-Cheat
**Goal:** Ensure ZKP circuits cannot be exploited to forge compute proofs.

**Current Protections:**
- Ed25519 signature validation (mock in v1.8, production-ready in v1.9)
- SHA-256 compute hash verification
- Timestamp expiration (±5min tolerance)
- Node registration verification
- Replay detection (proof_id uniqueness)

**Hardening Plan:**
1. Replace mock Ed25519 with `ed25519-dalek` production signatures
2. Add proof batch aggregation (Merkle root verification)
3. Implement VRF (Verifiable Random Function) for proof sampling
4. External audit of circuit correctness (Halo2/SP1)

**Timeline:** Sprint 2-3 (Q3-Q4 2026)

### 2.3 Audit Readiness
**Goal:** Prepare codebase for external security audit.

**Current Status:**
- ✅ Zero `unsafe` blocks (enforced by ethical clause)
- ✅ Comprehensive test coverage (2891 tests)
- ✅ CI/CD pipeline (lint, test, coverage, benchmark)
- ✅ Security policy ([`SECURITY.md`](../../SECURITY.md))
- ✅ Threat model ([`security/threat_model_v1.1.md`](../../security/threat_model_v1.1.md))
- ✅ Audit prep checklist ([`security/audit_prep.md`](../../security/audit_prep.md))
- ⏳ External audit (pending funding)

**Audit Scope (Requested):**
| Component | Priority | Estimated Cost |
|-----------|----------|----------------|
| Async ZKP v14 circuits | P0 | $15,000 |
| Federation ZKP Bridge v7 | P0 | $10,000 |
| Ed25519 Reputation Proof Schema | P1 | $5,000 |
| API Explorer v1 (auth + rate limit) | P1 | $5,000 |
| Async Steering (RFC-001) | P2 | $5,000 |
| **Total** | | **$40,000** |

### 2.4 Async Steering Hardening (`async_steering.rs`)
**Goal:** Harden real-time correction signal pipeline against injection attacks.

**Current Protections:**
- Signal value bounds checking (0.0 to 1.0)
- Delay decay (older signals have less impact)
- Sequence number validation (FIFO ordering)
- Capacity limits (bounded channel)

**Hardening Plan:**
1. Add Ed25519 signature verification for steering signals
2. Implement signal rate limiting (max signals per node per minute)
3. Add anomaly detection (flag signals outside expected distribution)
4. Fuzz test signal parsing with `proptest`

**Timeline:** Sprint 2 (Q3 2026)

---

## 3. Funding Request

### 3.1 OSSF Security Improvement Fund
**Amount Requested:** $40,000
**Purpose:** External security audit of ZKP circuits + async steering hardening

| Category | Amount | Deliverable |
|----------|--------|-------------|
| ZKP Circuit Audit | $25,000 | Audit report + remediation for async_zkp_v14 + federation_zkp_bridge_v7 |
| Reputation Schema Audit | $5,000 | Audit report for proof_schema.rs + Ed25519 integration |
| API Security Audit | $5,000 | Audit report for explorer_v1.rs (auth, rate limit, input validation) |
| Async Steering Hardening | $5,000 | Implementation + tests for signature verification + rate limiting |
| **Total** | **$40,000** | |

### 3.2 OSSF Best Practices Badge
**Current Status:**
- ✅ LICENSE file present
- ✅ Security policy (SECURITY.md)
- ✅ OpenSsf-scorecard (pending setup)
- ✅ Signed commits (Git GPG)
- ⏳ Dependency update tool (Dependabot/Renovate — pending)
- ⏳ SBOM (Software Bill of Materials — pending)

**Plan to Achieve Badge:**
1. Enable OpenSSF Scorecard action
2. Set up Dependabot for automatic dependency updates
3. Generate SBOM with `cargo audit` + `cargo-deny`
4. Achieve "Passing" → "Silver" → "Gold" badge levels

---

## 4. Security Metrics

| Metric | Current | Target (Post-Audit) | Measurement |
|--------|---------|---------------------|-------------|
| Unsafe blocks | 0 | 0 | `grep -r "unsafe" src/` |
| Test coverage | ~80% | ≥85% | Codecov |
| Critical vulnerabilities | 0 | 0 | `cargo audit` |
| ZKP circuit tests | 200+ | 300+ | `cargo test` |
| Fuzz test cases | 0 | ≥50 | `cargo fuzz` |
| Security incidents | 0 | 0 | Incident log |
| OpenSSF Scorecard | N/A | ≥8/10 | Scorecard action |

---

## 5. Threat Model Summary

### 5.1 Assets
- SAE model weights (intellectual property)
- ZKP proofs (integrity critical)
- Reputation ledger (anti-sybil critical)
- Steering signals (safety critical)

### 5.2 Threats
| Threat | Likelihood | Impact | Mitigation |
|--------|------------|--------|------------|
| Malicious SAE model | Medium | High | WASM sandbox, input validation |
| ZKP proof forgery | Low | Critical | Circuit audit, Ed25519 signatures |
| Reputation sybil attack | Medium | High | Anti-sybil tracking, contribution velocity limits |
| Steering signal injection | Medium | Critical | Signature verification, rate limiting, anomaly detection |
| DDoS on API Explorer | Medium | Medium | Rate limiting (token bucket), capacity limits |
| Dependency vulnerability | Medium | Medium | `cargo audit`, Dependabot, pinned versions |

### 5.3 Trust Boundaries
- **External → API:** Rate limiter → Ed25519 proof validator → Route handlers
- **Node → Federation:** ZKP proof submission → Circuit verification → Reputation update
- **Browser → WASM:** Sandbox isolation → Memory-safe Rust → No FFI

---

## 6. Team Security Expertise

| Role | Handle | Security Expertise |
|------|--------|-------------------|
| Lead Developer | @Stuartemk | Rust memory safety, ZKP circuits, Ed25519 |
| Cryptography | [PLACEHOLDER] | Halo2/SP1 circuits, VRF, Merkle proofs |
| Security Auditor | [PLACEHOLDER] | External audit (pending engagement) |
| Community | [PLACEHOLDER] | Vulnerability disclosure, security policy |

---

## 7. Timeline

| Month | Milestone | Deliverable |
|-------|-----------|-------------|
| **M1** | Audit scoping | Signed NDA, audit scope document |
| **M2** | ZKP circuit audit | Audit report + remediation PRs |
| **M3** | Reputation + API audit | Audit reports + remediation |
| **M4** | Async steering hardening | Implementation + tests + fuzz |
| **M5** | OpenSSF Scorecard | Passing badge + SBOM |
| **M6** | Final report | Security whitepaper + v1.8.0-stable |

---

## 8. Ethical Considerations

- **Ethical Use Clause:** Prohibits harmful applications (surveillance, manipulation, weapons)
- **Zero Unsafe Code:** No `unsafe` blocks in entire codebase
- **Zero Telemetry:** No data collection, no tracking, no analytics
- **Zero Financial Logic:** No tokens, no staking, no financial incentives
- **Transparency:** All code, audits, and security decisions public

---

## 9. References

- Security Policy — [`SECURITY.md`](../../SECURITY.md)
- Threat Model v1.1 — [`security/threat_model_v1.1.md`](../../security/threat_model_v1.1.md)
- Audit Prep — [`security/audit_prep.md`](../../security/audit_prep.md)
- RFC-001: Latency Mitigation — [`docs/rfc/rfc-001-latency-mitigation-v1.7.md`](../rfc/rfc-001-latency-mitigation-v1.7.md)
- Async Steering — [`src/protocol/async_steering.rs`](../../src/protocol/async_steering.rs)
- ZKP v14 — [`src/zkp/async_zkp_v14.rs`](../../src/zkp/async_zkp_v14.rs)
- Reputation Proof Schema — [`src/reputation/proof_schema.rs`](../../src/reputation/proof_schema.rs)

---

## 10. Submission Checklist

- [ ] Fill all [PLACEHOLDER] fields with verified information
- [ ] Confirm OSSF program and deadline
- [ ] Set up OpenSSF Scorecard action
- [ ] Enable Dependabot for dependency updates
- [ ] Run `cargo audit` and fix vulnerabilities
- [ ] Generate SBOM with `cargo-deny`
- [ ] Engage external auditor (signed NDA + scope)
- [ ] Save application ID: [PLACEHOLDER: OSSF_APP_ID]

---

*OSSF Security Grant Draft v1.0 — ed2kIA v1.8*
*Generated: 2026-05-15*
*Status: DRAFT — Requires review and placeholder completion before submission*
