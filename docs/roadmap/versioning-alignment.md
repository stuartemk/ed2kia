# Versioning Alignment & Release Strategy

> **Generated:** 2026-05-15T22:30:00Z
> **Purpose:** Establish official mapping between Fases, Versions, Feature Gates, and Release Policy
> **Status:** ACTIVE — Reference for all future releases

---

## 1. Phase ↔ Version Matrix

| Fase | Descripción | Versión | Estado | Feature Gate |
|------|-------------|---------|--------|--------------|
| **FASE 1** | Core P2P + SAE Loader + Tensor Routing | v0.1.0 → v0.3.0 | ✅ Completada | N/A (legacy) |
| **FASE 2** | Interpretación, Feedback & Consenso | v0.3.0 → v0.4.0 | ✅ Completada | N/A (legacy) |
| **FASE 3** | Seguridad, ZKP, Human-in-the-Loop & Deploy | v0.4.0 → v0.5.0 | ✅ Completada | N/A (legacy) |
| **FASE 4** | Escalado, RLHF, Web UI & Producción | v0.5.0 → v1.0.0 | ✅ Completada | `stable` |
| **FASE 5** | Bootstrap, Gobernanza, Reputación & Ecosistema | v1.0.0 → v1.5.0 | ✅ Completada | `stable` |
| **FASE 6** | Integración y Producción | v1.6.0 → v1.8.0-beta.1 | ✅ Completada | `stable` + `v1.8-sprint1` + `v1.8-sprint2` |
| **FASE 7+** | Production Ready (v1.9+) | v1.9.0+ | 🔄 En desarrollo | `v1.9-sprint*` (future) |

### FASE 6 Evolution Detail

FASE 6 was completed through iterative sprints rather than a single phase delivery:

| Sprint | Versión | Entregables | Feature Gate |
|--------|---------|-------------|--------------|
| v1.6.0 Sprint 1 | v1.6.0 | Cross-Chain Interoperability Foundation | `v1.6-sprint1` |
| v1.6.0 Sprint 2 | v1.6.0 | Federation Scaling v7, Async ZKP v13, UI Dashboard v7 | `v1.6-sprint2` |
| v1.6.0 Sprint 3 | v1.6.0-stable | SAE Fine-Tuning v7, Cross-Model Scaling v7, Async ZKP v14 | `v1.6-sprint3` |
| v1.7.0 | v1.7.0-stable | Benchmarks, CI pipeline, Community onboarding | `stable` |
| v1.8.0 Sprint 1 | v1.8.0-beta.1 | API Explorer, Reputation Proof Schema, QuantConfig | `v1.8-sprint1` |
| v1.8.0 Sprint 2 | v1.8.0-beta.1 | Async Steering, Geographic Routing, WASM Mobile Bridge | `v1.8-sprint2` |

---

## 2. Feature Gate Policy

### Current Feature Gates (Cargo.toml)

```toml
# Production (stable)
stable = [
    "phase6-core", "phase6-sprint2", "phase6-experimental",
    "phase7-sprint1", "phase7-sprint2", "phase8-sprint1", "phase8-sprint2", "phase9-sprint1",
    "v1.1-sprint1" ... "v1.6-sprint3",
]

# Beta (v1.8.0-beta.1)
v1.8-sprint1 = []   # API Explorer, Reputation Proof, QuantConfig
v1.8-sprint2 = []   # Async Steering, Geographic Routing, WASM Bridge

# Hardware
cpu = []
cuda = ["candle-core/cuda", "candle-nn/cuda"]
metal = ["candle-core/metal", "candle-nn/metal"]
```

### Feature Gate Lifecycle

```
EXPERIMENTAL → SPRINT → BETA → STABLE → LEGACY → REMOVED
     │            │        │         │          │          │
     │            │        │         │          │          └─ v2.0.0+
     │            │        │         │          └─ Deprecation warning
     │            │        │         └─ Included in `stable` feature
     │            │        └─ Available via `vX.Y-sprintN` feature
     │            └─ Implemented in sprint, gated behind feature flag
     └─ Research/POC stage, not in Cargo.toml
```

### Rules

1. **Stable-only for production:** `cargo build --features stable` is the production build
2. **Beta features explicit:** Beta features require explicit `--features "v1.8-sprint1"` flag
3. **No implicit beta in stable:** Beta features are NOT included in `stable` until promoted
4. **Hardware features additive:** `cuda`/`metal` can be combined with any feature gate
5. **Legacy deprecation:** Legacy flags (`phase6-core`, `v1.1-sprint*`) marked for removal in v2.0.0

---

## 3. Branching Strategy

### Current Model: Trunk-Based with Feature Gates

```
main (trunk)
├── v1.8.0-beta.1 ← Current beta tag
├── v1.7.0-stable ← Last stable tag
├── v1.6.0-stable ← Previous stable tag
└── ...
```

### Branch Types

| Type | Pattern | Lifecycle | Example |
|------|---------|-----------|---------|
| **Main** | `main` | Permanent | Trunk branch |
| **Feature** | `feature/<scope>/<name>` | Sprint duration | `feature/api/explorer-v1` |
| **Hotfix** | `hotfix/<issue-id>` | Until merge | `hotfix/p0-signature-validation` |
| **Release** | Tags only (no long-lived branches) | N/A | `v1.8.0-beta.1` |

### Rules

1. **No release branches:** Releases are tagged directly from `main`
2. **Feature flags control scope:** Features are gated, not branched
3. **Main is always buildable:** `cargo check --features stable` must pass on main
4. **PR → main:** All changes go through PR with CI validation

---

## 4. Tagging Policy

### Semantic Versioning + Pre-release Identifiers

Format: `vMAJOR.MINOR.PATCH[-PRERELEASE]`

| Component | Meaning | Example |
|-----------|---------|---------|
| MAJOR | Breaking changes | `v2.0.0` |
| MINOR | New features (backward compatible) | `v1.8.0` |
| PATCH | Bug fixes | `v1.8.1` |
| PRERELEASE | Alpha/Beta/RC | `v1.8.0-beta.1` |

### Pre-release Order

```
alpha.1 → alpha.2 → ... → beta.1 → beta.2 → ... → rc.1 → rc.2 → ... → STABLE
```

### Tag Commands

```bash
# Annotated tag for release
git tag -a v1.8.0-beta.1 -m "Release v1.8.0-beta.1: Beta Launch"

# Push tag
git push origin v1.8.0-beta.1

# Delete tag (emergency)
git tag -d v1.8.0-beta.1
git push origin :refs/tags/v1.8.0-beta.1
```

---

## 5. Rollback Policy

### Rollback Procedure

1. **Identify stable tag:** Find last known-good tag (`git tag -l 'v*' | sort -V`)
2. **Revert commits:** `git revert HEAD~N..HEAD` (preferred over reset)
3. **Emergency tag:** Create patch version with fix
4. **Update feature gate:** Disable problematic feature in `Cargo.toml`
5. **Notify:** Update `docs/beta/feedback-tracker.md` + Discord/Mattermost

### Rollback by Severity

| Severity | Action | SLA |
|----------|--------|-----|
| P0 (Critical) | Immediate tag + hotfix branch | 2 hours |
| P1 (High) | Feature gate disable + patch | 12 hours |
| P2 (Medium) | Next patch release | 48 hours |
| P3 (Low) | Next minor release | 7 days |

---

## 6. Release Artifacts

### Release Directory Structure

```
release/
├── changelog.md                    # Master changelog
├── packager.sh                     # Multi-platform build script
├── v1.6.0-stable/
│   ├── final_signoff.json          # Validation report
│   └── ...
├── v1.7.0-stable/
│   ├── RELEASE_NOTES.md
│   └── RELEASE_CHECKLIST.md
└── v1.8.0-beta.1/
    ├── RELEASE_NOTES.md
    └── monitor-report.md           # Beta monitoring output
```

### Required Artifacts per Release

| Artifact | Location | Purpose |
|----------|----------|---------|
| Release Notes | `release/vX.Y.Z/RELEASE_NOTES.md` | User-facing changes |
| Validation Report | `release/vX.Y.Z/final_signoff.json` or `validation_report.json` | CI/test results |
| Git Tag | `vX.Y.Z` | Version marker |
| Changelog Entry | `release/changelog.md` | Cumulative history |

---

## 7. CHANGELOG Reference

The master changelog is maintained at [`release/changelog.md`](../../release/changelog.md).

### Changelog Format (Keep a Changelog)

```markdown
## [Unreleased]
### Added
### Changed
### Deprecated
### Removed
### Fixed
### Security

## [1.8.0-beta.1] - 2026-05-15
### Added
- API Explorer v1: 3D concept visualization
- Reputation Proof Schema: Ed25519-based proofs
- Async Steering v1: Late correction signals
### Changed
- Quantization v3: Per-element FP8/INT4
```

---

## 8. Version Promotion Path

```
┌─────────────────────────────────────────────────────────────┐
│                    VERSION PROMOTION                         │
│                                                             │
│  Sprint Complete → Feature Gate → Beta Tag → Validation    │
│       ↓                                                         │
│  Beta Testing → Feedback → Hotfixes → RC Tag               │
│       ↓                                                         │
│  RC Validation → Stable Tag → Release Notes → Announce     │
│       ↓                                                         │
│  Production → Monitoring → Patch (if needed)               │
└─────────────────────────────────────────────────────────────┘
```

### Current Status

| Version | Stage | Date | Next Step |
|---------|-------|------|-----------|
| v1.6.0-stable | Production | 2026-04 | Maintained |
| v1.7.0-stable | Production | 2026-05 | Maintained |
| v1.8.0-beta.1 | Beta Testing | 2026-05-15 | → RC → Stable |
| v1.9.0 | Planning | Q3 2026 | Roadmap draft |

---

## 9. References

- [`Cargo.toml`](../../Cargo.toml) — Feature gate definitions
- [`release/changelog.md`](../../release/changelog.md) — Master changelog
- [`GOVERNANCE.md`](../../GOVERNANCE.md) — Release approval process
- [`phase6-audit-mapping.md`](phase6-audit-mapping.md) — FASE 6 reconciliation
- [`v1.9-roadmap-draft.md`](v1.9-roadmap-draft.md) — Next version planning
- [`source-of-truth.md`](source-of-truth.md) — Master reference

---

*This document is maintained as part of the versioning alignment process. Updates require PR with @ed2kIA/core-team review.*
