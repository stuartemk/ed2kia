# ed2kIA v1.7.0-stable — Release Checklist

**Version:** 1.7.0-stable
**Fecha:** 2026-05-14
**Estado:** PRE-LAUNCH VALIDATION

---

## Pre-Release Validation

### CI/CD

- [x] **CI Green:** All tests passing (unit + e2e + stress)
- [x] **Cargo Check:** `cargo check --features stable` — 0 errors, 0 warnings
- [x] **Cargo Clippy:** `cargo clippy --features stable -- -D warnings` — CLEAN
- [x] **Cargo Test:** `cargo test --features stable` — All PASS
- [x] **Benchmarks:** `cargo bench -p ed2kIA-benchmarks --features stable` — Baseline established

### Documentation

- [x] **README.md:** Public narrative section updated (FASE 29)
- [x] **SUPPORT.md:** Funding infrastructure complete (FASE 30)
- [x] **CONTRIBUTING.md:** Contributor funnel + v1.8 resources (FASE 34)
- [x] **SECURITY.md:** Security disclosure policy active
- [x] **LICENSE:** Apache 2.0 present
- [x] **docs/funding-strategy.md:** Grant pipeline documented (FASE 30)
- [x] **docs/architecture/reputation-gamification.md:** Spec complete (FASE 31)
- [x] **docs/architecture/mobile-browser-expansion.md:** WASM strategy documented (FASE 32)
- [x] **docs/roadmap/v1.8-chatgpt-moment.md:** Vision documented (FASE 33)
- [x] **docs/community/contributor-funnel.md:** Journey mapped (FASE 34)
- [x] **release/v1.7.0-stable/RELEASE_NOTES.md:** Release notes complete

### Issues & Planning

- [x] **ISSUES_BATCH_V1.7.md:** v1.7 issues documented
- [x] **ISSUES_BATCH_V1.8.md:** v1.8 issues documented (FASE 35)
- [x] **scripts/create_issues.sh:** v1.7 creation script ready
- [x] **scripts/create_issues_v1.8.sh:** v1.8 creation script ready (FASE 35)

### Funding & Community

- [x] **SUPPORT.md:** Funding paths documented (GitHub Sponsors, Open Collective, Crypto, Gitcoin)
- [x] **docs/funding-strategy.md:** Grant targets + revenue streams defined
- [x] **COMMUNITY_POSTS_READY.md:** Community posts prepared

### Git & Versioning

- [x] **Branch:** main
- [x] **Last Commit:** `a56f0c1` — docs(issues): add v1.8 issues batch + creation script
- [x] **Tag:** `v1.7.0-stable` — PENDING CREATION
- [x] **Release Notes:** release/v1.7.0-stable/RELEASE_NOTES.md — COMPLETE

---

## Release Commands

```bash
# 1. Ensure clean working tree
git status
git stash  # if needed

# 2. Create annotated tag
git tag -a v1.7.0-stable -m "ed2kIA v1.7.0-stable: Latency PoC & Auto-Push Active"

# 3. Push tag to remote
git push origin v1.7.0-stable

# 4. Verify tag exists locally
git tag -l | grep v1.7.0-stable

# 5. Verify release notes exist
test -f release/v1.7.0-stable/RELEASE_NOTES.md && echo "RELEASE_NOTES: OK" || echo "RELEASE_NOTES: MISSING"
```

---

## Post-Release Actions

### Immediate (FASE 37-40)

1. **FASE 37:** Execute v1.8 issue batch via GitHub CLI
2. **FASE 38:** Activate funding channels (GitHub Sponsors, Open Collective, Gitcoin)
3. **FASE 39:** Community launch & outreach execution
4. **FASE 40:** Operations dashboard & prompt v2.0

### Within 24h

- [ ] Monitor first issue responses
- [ ] Verify funding channel activation
- [ ] Post launch announcement on community platforms
- [ ] Update transparency report

### Within 7 Days

- [ ] First weekly metrics report
- [ ] Review contributor funnel metrics
- [ ] Adjust SLAs based on initial data
- [ ] Scale to v1.8 Sprint 1 execution

---

## Rollback Plan

Si se detecta un problema critico post-release:

1. **Identificar:** Verificar CI logs y reports de la comunidad
2. **Evaluar:** Determinar si requiere hotfix o rollback completo
3. **Hotfix:** Crear branch `hotfix/v1.7.1` desde tag `v1.7.0-stable`
4. **Rollback:** Si critico, `git revert <commit_problematico>` en main
5. **Comunicar:** Actualizar transparency report y notificar a la comunidad

---

## Sign-Off Checklist

| Item | Status |
|------|--------|
| CI Green | ✅ |
| Docs Synced | ✅ |
| Issues Batch Ready | ✅ |
| Funding Paths Active | ✅ (documented) |
| Release Notes Complete | ✅ |
| Tag Creation | PENDING |
| Tag Push | PENDING |

**Ready for tag creation:** YES
**Ready for community launch:** PENDING (FASE 37-40)
