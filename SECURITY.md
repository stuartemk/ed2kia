# Security Policy — ed2kIA

**Versión:** v2.0.0-stable
**Actualizado:** 2026-05-17
**Modo:** STEWARDSHIP (Autonomous)

---

## Supported Versions

| Version | Status | Supported? |
|---------|--------|------------|
| v2.0.0-stable | Current STABLE | ✅ Yes |
| v1.9.0-stable | Previous STABLE | ✅ Yes (security only) |
| v1.6.0-stable | Legacy | ⚠️ Security critical only |
| < v1.6.0 | EOL | ❌ No |

---

## Reporting a Vulnerability

### Responsible Disclosure

We follow a **responsible disclosure** policy. If you discover a security vulnerability:

1. **Do NOT open a public GitHub issue.**
2. Send a private report to:
   - **GitHub Security Advisories:** Use the "Report a vulnerability" button in the Security tab of this repository.
3. Include the following in your report:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact assessment
   - Suggested fix (if available)
4. **Response time:** We will acknowledge receipt within **48 hours**.
5. **Resolution window:** We aim to resolve critical vulnerabilities within **90 days**.

### What We Consider a Security Vulnerability

- Remote code execution (RCE)
- Denial of Service (DoS) affecting network consensus
- Cryptographic weaknesses in ZKP proofs or Merkle verification
- Privilege escalation in governance or reputation systems
- Data exfiltration through P2P message channels
- Memory safety issues (buffer overflows, use-after-free)
- WASM sandbox escapes

### What We Do NOT Consider a Security Vulnerability

- Availability issues caused by external infrastructure (ISPs, cloud providers)
- Misconfiguration of user-deployed nodes
- Lack of features or functionality
- Usability issues

---

## Security Architecture

### Threat Model v2.0

See [`security/threat_model_v2.0.md`](security/threat_model_v2.0.md) for our comprehensive STRIDE+DREAD threat model covering **17 identified threats**:

| Category | Control | Implementation |
|----------|---------|----------------|
| **Spoofing** | Node identity verification | Ed25519 signatures + reputation ledger |
| **Tampering** | Data integrity | Merkle trees + ZKP proof verification |
| **Repudiation** | Immutable audit trail | redb embedded ledger + signed proposals |
| **Information Disclosure** | Encrypted channels | libp2p encrypted transport |
| **Denial of Service** | Rate limiting + scoring | Reputation-based peer scoring + connection limits |
| **Elevation of Privilege** | Governance controls | ≥30% quorum + ≥51% reputation-weighted approval |
| **WASM Sandbox Escape** | Isolation | wasmtime (256MB memory limit, no host I/O) |
| **Sybil Attacks** | Identity flooding | Micro-PoW handshake + Anti-Sybil scoring + 50%/30d decay + Ed25519 proofs |
| **API Abuse** | Feedback spam | Per-node rate limiting (10/300s) + exponential backoff ban |

### Security Controls

| Control | Implementation | Status |
|---------|----------------|--------|
| **Zero unsafe code** | `#![forbid(unsafe_code)]` enforced | ✅ Active |
| **Zero telemetry** | No external network calls, no analytics | ✅ Active |
| **Cryptographic verification** | ZKP (arkworks), Merkle trees, Ed25519 | ✅ Active |
| **WASM sandbox** | wasmtime isolation (256MB default) | ⚠️ CVE tracked (upgrade planned) |
| **Reputation system** | Anti-Sybil + 50%/30d decay + tier proofs | ✅ Active |
| **Governance quorum** | ≥30% participation + ≥51% approval | ✅ Active |
| **Audit trail** | Immutable ledger + signed proposals | ✅ Active |
| **Automated monitoring** | Weekly cargo audit + security alerts | ✅ Active |
| **Micro-PoW Sybil Resistance** | SHA-256 challenge (~2s/nodo) + rate limiting + ban matrix | ✅ Active (v2.1-sprint9) |
| **Signed GossipSub** | `MessageAuthenticity::Signed` + libp2p 0.53 SwarmBuilder | ✅ Active (v2.1-sprint9) |
| **RLHF Rate Limiting** | Per-node FeedbackStore (10 submissions/300s window) | ✅ Active (v2.1-sprint9) |

### Sprint 9: Sybil Resistance & Federated Consensus

**v2.1.0-sprint9 "Resiliencia Absoluta"** añade tres controles de seguridad críticos:

| Control | Módulo | Mitigación |
|---------|--------|------------|
| **Micro-PoW Handshake** | `src/orchestrator/sybil.rs` | SHA-256 con 1-4 leading zeros (~2s solve), previene inundación Sybil sin barreras financieras |
| **Rate Limiting** | `src/orchestrator/sybil.rs` | 10 submissions/300s por nodo, ban exponencial (3 fallos → temp, 5 → perm) |
| **Signed Federation** | `src/orchestrator/network.rs` | `MessageAuthenticity::Signed` asegura proveniencia criptográfica en GossipSub |
| **Reputation Sync** | `src/orchestrator/network.rs` | Sincronización federada de slashing previene Sybil hopping entre clusters |
| **RLHF Rate Limit** | `src/atlas/api.rs` | `FeedbackStore` con rate limiting per-node, previene spam API |
| **Zero PII** | `web/atlas-visualizer.js` | Feedback almacenado localmente, export opt-in, cero datos personales |

### Dependencies

We use audited Rust crates with strict version pinning:

| Crate | Purpose | Audit Status |
|-------|---------|--------------|
| **libp2p** | P2P networking | ✅ Audited, widely used |
| **arkworks** | ZKP circuits | ✅ Academic-grade, peer-reviewed |
| **wasmtime** | WASM runtime | ⚠️ CVE tracked (17.0.3 → >=24.0.7 planned) |
| **candle-core** | ML inference | ✅ Hugging Face, open source |
| **redb** | Embedded KV store | ✅ Pure Rust, memory-safe |
| **axum** | HTTP server | ✅ Tokio ecosystem, audited |

Run `cargo audit` regularly to check for known vulnerabilities:

```bash
cargo install cargo-audit
cargo audit
```

---

## CVE Matrix Q1 2027

**Source:** [`docs/reports/security-audit-Q1-2027.md`](docs/reports/security-audit-Q1-2027.md)

### Critical/High Severity

| ID | Severity | Package | Version | Impact | Status |
|----|----------|---------|---------|--------|--------|
| RUSTSEC-2024-0438 | **High** | wasmtime | 17.0.3 | Windows device filename sandbox bypass | pending (v2.1-security-hardening) |

### Medium Severity

| ID | Package | Version | Impact | Status |
|----|---------|---------|--------|--------|
| RUSTSEC-2026-0020 | wasmtime | 17.0.3 | Guest-controlled resource exhaustion WASI | pending |
| RUSTSEC-2026-0021 | wasmtime | 17.0.3 | Panic adding excessive WASI fields | pending |
| RUSTSEC-2026-0085 | wasmtime | 17.0.3 | Panic lifting `flags` component | pending |
| RUSTSEC-2026-0087 | wasmtime | 17.0.3 | Segfault f64x2.splat Cranelift x86-64 | pending |
| RUSTSEC-2026-0098 | rustls-webpki | 0.101.7 | URI name constraints incorrectly accepted | pending |
| RUSTSEC-2026-0099 | rustls-webpki | 0.101.7 | Wildcard name constraints accepted | pending |
| RUSTSEC-2026-0104 | rustls-webpki | 0.101.7 | Panic in CRL parsing | pending |
| RUSTSEC-2026-0119 | hickory-proto | 0.24.4 | CPU exhaustion O(n²) name compression | pending |
| RUSTSEC-2024-0437 | protobuf | 2.28.0 | Crash uncontrolled recursion | pending |
| RUSTSEC-2025-0009 | ring | 0.16.20 | AES panic with overflow checking | pending |

### Low Severity

| ID | Package | Version | Impact | Status |
|----|---------|---------|--------|--------|
| RUSTSEC-2026-0086 | wasmtime | 17.0.3 | Host data leakage 64-bit tables Winch | pending |
| RUSTSEC-2025-0046 | wasmtime | 17.0.3 | Host panic with `fd_renumber` WASIp1 | pending |
| RUSTSEC-2025-0118 | wasmtime | 17.0.3 | Unsound shared linear memory access | pending |
| RUSTSEC-2026-0002 | lru | 0.12.5 | IterMut violates Stacked Borrows | pending (transitive) |

### Unmaintained Dependencies

| Crate | Version | Dependent Via | Replacement Plan |
|-------|---------|---------------|-----------------|
| mach | 0.3.2 | wasmtime-runtime → wasmtime | Monitor forks |
| paste | 1.0.15 | wasmtime, candle-core, ark-ff, gemm | build-script codegen (v2.1) |
| ring | 0.16.20 | rcgen → libp2p-tls → libp2p | libp2p upgrade |
| rustls-pemfile | 1.0.4 | reqwest | rustls-pemistore o pem (v2.1) |
| yaml-rust | 0.4.5 | config | yaml-rust2 o serde_yaml (v2.1) |

---

## Remediation Strategy

**Source:** [`docs/reports/dependency-remediation-plan-Q1-2027.md`](docs/reports/dependency-remediation-plan-Q1-2027.md)

### Pinning Strategy

**DECISIÓN:** NO aplicar pinning directo en v2.0.0-stable. Los upgrades de wasmtime y libp2p tienen breaking changes que requieren:

1. Feature gate `v2.1-security-hardening`
2. Testing exhaustivo con `cargo test`
3. Validación de compatibilidad P2P

### Feature-Gated Replacements

| Package | Replacement | Feature Gate | Status |
|---------|-------------|--------------|--------|
| wasmtime 17.x | wasmtime >=24.0.7 | v2.1-security-hardening | scaffold |
| libp2p (rustls-webpki 0.101) | libp2p with rustls-webpki >=0.103.13 | v2.1-security-hardening | scaffold |
| paste 1.0.15 | build-script codegen | v2.1-security-hardening | pending |
| rustls-pemfile 1.0.4 | rustls-pemistore o pem | v2.1-security-hardening | pending |
| yaml-rust 0.4.5 | yaml-rust2 o serde_yaml | v2.1-security-hardening | pending |

### Rollback Plan

```bash
# Pre-upgrade backup
git tag pre-remediation-Q1-2027

# Apply changes in feature gate
cargo check --features v2.1-security-hardening
cargo test --features v2.1-security-hardening
cargo clippy --features v2.1-security-hardening -- -D warnings
cargo audit --features v2.1-security-hardening

# Rollback (if tests fail)
git reset --hard pre-remediation-Q1-2027
cargo check --all-targets  # Verify restoration
```

### Timeline

| Phase | Action | Estimated Date | Dependencies |
|-------|--------|----------------|--------------|
| 1 | Documentation (current) | 2026-05-17 | ✅ Completed |
| 2 | Feature gate `v2.1-security-hardening` | Q2 2027 | RFC-002 approval |
| 3 | wasmtime upgrade (feature-gated) | Q2 2027 | Phase 2 |
| 4 | libp2p upgrade (feature-gated) | Q2 2027 | Phase 2 |
| 5 | Unmaintained replacements | Q3 2027 | Phases 3-4 |
| 6 | Promotion to stable | Q3 2027 | Phase 5 + validation |

---

## Automated Monitoring Pipeline

### Weekly Security Scan

**Workflow:** [`.github/workflows/security-monitor.yml`](.github/workflows/security-monitor.yml)

- **Schedule:** Mondays 03:00 UTC (weekly cron)
- **Tools:** `cargo audit`, `cargo outdated`
- **Output:** Security report auto-committed to `docs/reports/`
- **Alerts:** [`scripts/security-alert.sh`](scripts/security-alert.sh) parses reports, generates Slack/webhook notifications

### Daily Health Check

**Script:** [`scripts/autonomous_health_check.sh`](scripts/autonomous_health_check.sh)

- **Schedule:** Daily 02:00 UTC
- **Checks:** Build status, test pass rate, dependency audit
- **Mode:** STEWARDSHIP (autonomous loop)

---

## Security Audit Summary

| Metric | Value |
|--------|-------|
| **Total CVEs** | 14 |
| **Critical/High** | 1 (wasmtime sandbox escape) |
| **Medium** | 10 |
| **Low** | 3 |
| **Unmaintained Deps** | 5 |
| **Unsound Deps** | 1 |
| **OSSF Score** | 8.5/10 (PASSING) |
| **Verdict** | ⚠️ REQUIRES ATTENTION — Plan de mitigación activo |

**Full Audit Report:** [`docs/reports/security-audit-Q1-2027.md`](docs/reports/security-audit-Q1-2027.md)

---

## 🔐 Sprint 8: Streaming & Merit Proofs

**Added in v2.1.0-sprint8 "El Despertar"** — New security controls for HuggingFace streaming bridge and cryptographic merit system:

| Control | Implementation | Status |
|---------|----------------|--------|
| **SHA256 Checksum Verification** | `sha2::Sha256` Digest per chunk in `stream_sae_to_shards()` | ✅ Active |
| **Progressive Ingestion** | `reqwest::bytes_stream()` prevents RAM exhaustion attacks | ✅ Active |
| **Ed25519 Proof Signing** | `ed25519-dalek` `SigningKey` for `MeritProof` generation | ✅ Active |
| **Signature Verification** | `verify_proof()` validates Ed25519 signature before tier acceptance | ✅ Active |
| **Replay Prevention** | `POST /api/merit/claim` requires unique `(node_id, audit_count, timestamp)` tuple | ✅ Active |
| **Rate Limiting** | Proof claiming gated by `record_audit()` ledger entries | ✅ Active |
| **Zero Financial Logic** | MeritProof contains no economic value — technical reputation only | ✅ Enforced |

### Threat Mitigations

- **RAM Exhaustion via HF Download**: Mitigated by streaming ingestion (`bytes_stream()`) + chunked SHA256 verification. File is never fully loaded into memory.
- **Merit Proof Forgery**: Mitigated by Ed25519 signature verification. Unsigned or tampered proofs are rejected by `verify_proof()`.
- **Replay Attacks**: Mitigated by timestamp + audit_count binding in `MeritProof`. Each proof is cryptographically unique to the node's contribution history.
- **Sybil Inflation**: Merit tiers reflect actual audit volume, not self-reported claims. The `MeritEngine` ledger is the source of truth.

---

## Compliance & Ethics

- **License:** Apache 2.0 + Cláusula de Uso Ético
- **Zero Financial Logic:** No tokens, no speculation, no profit mechanisms
- **Transparency First:** All algorithms, decisions, and processes documented
- **Safety by Design:** Safety controls built-in, not bolted-on
- **Do No Harm:** Active prevention of harmful applications

**Constitution:** [`docs/governance/project-constitution.md`](docs/governance/project-constitution.md)

---

*Security Policy updated: 2026-05-17*
*Next review: Q2 2027 (Jul-Sep)*
*Tool: cargo-audit (RUSTSEC database)*
