# Security Policy — ed2kIA

**Versión:** v1.8-prep
**Actualizado:** 2026-05-15

---

## Supported Versions

| Version | Status | Supported? |
|---------|--------|------------|
| v1.6.0-stable | Current STABLE | ✅ Yes |
| v1.5.0-stable | Previous STABLE | ✅ Yes (security only) |
| v1.4.0-stable | Legacy | ⚠️ Security critical only |
| < v1.4.0 | EOL | ❌ No |

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

### What We Do NOT Consider a Security Vulnerability

- Availability issues caused by external infrastructure (ISPs, cloud providers)
- Misconfiguration of user-deployed nodes
- Lack of features or functionality
- Usability issues

---

## Security Architecture

### Threat Model

See [`security/threat_model_v1.1.md`](security/threat_model_v1.1.md) for our comprehensive STRIDE+DREAD threat model covering:

- **Spoofing:** Node identity verification via Ed25519 signatures
- **Tampering:** Merkle tree integrity + ZKP proof verification
- **Repudiation:** Immutable reputation ledger (redb)
- **Information Disclosure:** Encrypted P2P channels (libp2p)
- **Denial of Service:** Rate limiting + reputation-based scoring
- **Elevation of Privilege:** Governance quorum + time-locked voting

### Security Controls

| Control | Implementation |
|---------|----------------|
| **Zero unsafe code** | `#![forbid(unsafe_code)]` enforced |
| **Zero telemetry** | No external network calls, no analytics |
| **Cryptographic verification** | ZKP proofs (arkworks), Merkle trees, Ed25519 signatures |
| **WASM sandbox** | wasmtime isolation with memory limits (256MB default) |
| **Reputation system** | Anti-Sybil scoring + 50%/30d decay |
| **Governance quorum** | ≥30% participation + ≥51% reputation-weighted approval |
| **Audit trail** | Immutable reputation ledger + signed proposals |

### Dependencies

We use audited Rust crates with strict version pinning:

- **libp2p** — P2P networking (audited, widely used)
- **arkworks** — ZKP circuits (academic-grade, peer-reviewed)
- **wasmtime** — WASM runtime (Bytecode Alliance, production-hardened)
- **candle-core** — ML inference (Hugging Face, open source)
- **redb** — Embedded key-value store (pure Rust, memory-safe)

Run `cargo audit` regularly to check for known vulnerabilities:

```bash
cargo install cargo-audit
cargo audit
```

### Automated Dependency Audit

Use the included audit script for comprehensive dependency analysis:

```bash
bash scripts/dependency_audit.sh
```

This script performs:
1. CVE vulnerability scan (`cargo audit`)
2. Dependency tree analysis
3. Duplicate dependency detection
4. Version pinning verification
5. Security-relevant dependency checks

Reports are saved to `docs/security/audit-reports/`.

---

## Pre-Audit Preparation

See [`docs/security/audit-prep-checklist.md`](docs/security/audit-prep-checklist.md) for the complete security audit preparation checklist covering:

- Code quality gates (clippy, tests, audit)
- Dependency audit procedures
- Cryptographic review checklist
- Memory safety verification
- Runtime security controls
- Governance & reputation security
- Data integrity measures
- Build & deployment security
- Documentation requirements

---

## Build Security

### Reproducible Builds

```bash
# Build with locked dependencies
cargo build --release --locked

# Verify checksums
sha256sum target/release/ed2kia
```

### Feature Flags

```bash
# Production build (stable features only)
cargo build --release --features stable

# Development build (all features)
cargo build --features debug,test-mocks
```

---

## Incident Response

### Severity Levels

| Level | Response Time | Example |
|-------|---------------|---------|
| **Critical** | 24h | RCE, consensus bypass |
| **High** | 72h | DoS, reputation manipulation |
| **Medium** | 7 days | Information disclosure |
| **Low** | 30 days | Minor cryptographic weakness |

### Disclosure Process

1. **Report received** → Acknowledge within 48h
2. **Triage** → Classify severity within 5 business days
3. **Fix development** → Create private fork for patching
4. **Review** → Internal + external reviewer sign-off
5. **Release** → Patched version published with CVE (if applicable)
6. **Public disclosure** → Coordinated public announcement

---

## Compliance

- **License:** Apache 2.0 + Ethical Use Clause
- **No backdoors:** Verified by community audit
- **No telemetry:** Zero external network calls
- **No unsafe code:** Memory-safe Rust throughout
- **Transparent:** All code auditable on GitHub

---

## Contact

- **Security Advisories:** [GitHub Security Tab](https://github.com/ed2kia/ed2kIA/security/advisories/new)
- **General Issues:** [GitHub Issues](https://github.com/ed2kia/ed2kIA/issues)
- **Discussions:** [GitHub Discussions](https://github.com/ed2kia/ed2kIA/discussions)

---

## Security Contacts & Resources

- **Security Advisories:** [GitHub Security Tab](https://github.com/Stuartemk/ed2kIA/security/advisories/new)
- **General Issues:** [GitHub Issues](https://github.com/Stuartemk/ed2kIA/issues)
- **Discussions:** [GitHub Discussions](https://github.com/Stuartemk/ed2kIA/discussions)
- **Audit Prep Checklist:** [`docs/security/audit-prep-checklist.md`](docs/security/audit-prep-checklist.md)
- **Dependency Audit Script:** [`scripts/dependency_audit.sh`](scripts/dependency_audit.sh)
- **Threat Model:** [`security/threat_model_v1.1.md`](security/threat_model_v1.1.md)

---

*Última actualización: 2026-05-15 (v1.8-prep)*
