# Stewardship Readiness v2.1 — ed2kIA

**Version:** v2.1-sprint1
**Last Updated:** 2026-05-17
**Status:** Pre-Activation
**Owner:** @ed2kia/maintainers

---

## 1. Pre-Activation Checklist

All items must be verified before activating `v2.1-sprint1` feature gates in production.

### Governance & RFCs

- [ ] RFC-001 (Latency Mitigation) approved and documented
- [ ] RFC-002 (Observability Scaffold) drafted
- [ ] RFC-003 (Security Hardening) drafted
- [ ] RFC Tracking updated in [`docs/governance/rfc-tracking.md`](../governance/rfc-tracking.md)
- [ ] Voting Dashboard active in [`docs/community/voting-dashboard-active.md`](../community/voting-dashboard-active.md)
- [ ] Quorum reached for v2.1 activation (≥ 60% weighted vote)

### Feature Gates

- [ ] `v2.1-sprint1` defined in `Cargo.toml`
- [ ] `v2.1-observability` defined in `Cargo.toml`
- [ ] `v2.1-security-hardening` defined in `Cargo.toml`
- [ ] `v2.1-zkp-v3` defined in `Cargo.toml`
- [ ] `v2.1-gui` defined in `Cargo.toml`
- [ ] `v2.1-enterprise` defined in `Cargo.toml`
- [ ] No v2.1 features in `default` or `stable` gate
- [ ] Feature gate check job passes in CI/CD

**Verification Command:**
```bash
# Verify feature gates exist
grep -E '^"v2\.1-' Cargo.toml

# Verify no v2.1 in default
grep -A 20 '^default = \[' Cargo.toml | grep -v "v2.1" || echo "PASS: No v2.1 in default"

# Test each feature gate
cargo check --features v2.1-observability
cargo check --features v2.1-security-hardening
```

### CI/CD Integration

- [ ] `feature-gate-check` job in `.github/workflows/ci.yml` (verifies v2.1 not in default)
- [ ] `feature-gate-tests` job in `.github/workflows/ci.yml` (tests each v2.1 feature)
- [ ] `codeowners-sync` job in `.github/workflows/ci.yml` (protected path verification)
- [ ] `alerts.yml` workflow active (push/workflow_run triggers)
- [ ] All existing CI jobs still passing (no regressions)

**Verification Command:**
```bash
# Check CI workflow contains v2.1 jobs
grep -E "feature-gate|v2\.1" .github/workflows/ci.yml

# Validate YAML syntax
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))" 2>/dev/null || echo "yml check skipped"
```

### CODEOWNERS Sync

- [ ] `/docs/governance/` → @ed2kia/governance-team
- [ ] `/docs/community/` → @ed2kia/docs-team
- [ ] `/docs/grants/` → @ed2kia/maintainers
- [ ] `/infra/` → @ed2kia/ops-team
- [ ] `/tests/integration/` → @ed2kia/core-team
- [ ] `/benchmarks/` → @ed2kia/core-team
- [ ] `CHANGELOG.md` → @ed2kia/maintainers
- [ ] `Cargo.toml` → @ed2kia/core-team
- [ ] `SECURITY.md` → @ed2kia/crypto-team

**Verification Command:**
```bash
grep -E "v2.1|governance|grants|infra|integration|benchmarks" .github/CODEOWNERS
```

### Grant Support Kit

- [ ] Grant Execution Support Kit created: [`docs/grants/grant-execution-support-kit.md`](../grants/grant-execution-support-kit.md)
- [ ] Technical Deliverables Matrix populated
- [ ] Budget Justification Template ready
- [ ] Compliance & Ethics Checklist verified
- [ ] Submission Workflow documented (human-only)

### Documentation

- [ ] CONTRIBUTING.md updated with v2.1 Ambassador Workflow
- [ ] Testnet Dry-Run Script validated: `scripts/run-v21-dryrun.sh`
- [ ] Stewardship Readiness Checklist (this document)
- [ ] Quarterly Review Q2 2027 Prep: [`docs/reports/quarterly-review-Q2-2027-prep.md`](../reports/quarterly-review-Q2-2027-prep.md)

---

## 2. Activation Protocol

When all Pre-Activation items are checked, the Orquestador executes:

### Step 1: Commit Feature Flag

```bash
# Add v2.1-sprint1 to stable features (if approved)
# OR keep feature-gated for opt-in only
git commit -m "feat(v2.1): activate v2.1-sprint1 feature gates"
git push origin main
```

### Step 2: Notify Stewards

- Post to `#stewards` channel (Discord/Matrix)
- Create GitHub Issue with label `v2.1-activation`
- Update Voting Dashboard status to "Active"

### Step 3: Monitor Alerts

- Verify `.github/workflows/alerts.yml` triggers on push
- Check `feature-gate-tests` job passes for all v2.1 features
- Monitor for any regression in existing tests

### Step 4: Operational Handover

- Update `DAY1_OPERATIONS_PROMPT.md` with v2.1 context
- Verify `scripts/autonomous_health_check.sh` includes v2.1 checks
- Confirm handover package is complete

---

## 3. Rollback & Emergency Procedures

### Desactivar Feature Gates

```bash
# Revert feature gate activation
git revert <commit-hash>
git push origin main

# OR manually remove from default features in Cargo.toml
# Ensure v2.1-* features remain defined but not enabled by default
```

### Revertir CI/CD Cambios

```bash
# Restore previous CI workflow
git checkout HEAD~1 -- .github/workflows/ci.yml
git commit -m "revert: restore previous CI/CD config"
git push origin main
```

### Escalar CVEs Críticos

1. **Detectar:** `cargo audit` o GitHub Dependabot alert
2. **Clasificar:** Critical/High → SEV-1, Medium → SEV-2
3. **Contener:** Desactivar feature gate afectado si aplica
4. **Parchear:** Actualizar dependency pin en `Cargo.toml`
5. **Verificar:** `cargo audit` clean + tests passing
6. **Documentar:** Update `SECURITY.md` + `CHANGELOG.md`
7. **Notificar:** Stewards + comunidad vía GitHub Issue

**Contactos de Emergencia:**
- Security Lead: @ed2kia/crypto-team
- Maintainers: @ed2kia/maintainers
- Disclosure: security@ed2kIA.org

---

## 4. Next Milestones (Q2 2027)

### Targets

| Milestone | Target Date | Status |
|-----------|-------------|--------|
| Ambassador Onboarding v2.1 | Q2 Week 1-2 | Pending |
| Testnet Live (non-production) | Q2 Week 3-4 | Pending |
| OSSF Audit v2.1 | Q2 Week 5-6 | Pending |
| Feature Gate Promotion (opt-in → default) | Q2 Week 7-8 | Pending |
| OSSF Score ≥ 9.0 | Q2 End | Target |

### Ambassador Onboarding

- Review [`CONTRIBUTING.md`](../../CONTRIBUTING.md) v2.1 section
- Complete feature gate tutorial
- Run first dry-run: `bash scripts/run-v21-dryrun.sh --report-only`
- Submit first PR against v2.1 feature gate

### Testnet Live Prep

- Validate `infra/docker-compose.testnet-v2.1.yml` with real images
- Configure Prometheus + Grafana dashboards
- Set up monitoring alerts
- Document node operator guide v2.1

### OSSF Audit v2.1

- Run `cargo audit` — 0 Critical/High CVEs
- Verify OSSF Scorecard ≥ 9.0
- Update `SECURITY.md` with audit results
- Publish transparency report

---

## 5. References

- **Project Constitution:** [`docs/governance/project-constitution.md`](../governance/project-constitution.md)
- **GOVERNANCE.md:** [`GOVERNANCE.md`](../../GOVERNANCE.md)
- **SECURITY.md:** [`SECURITY.md`](../../SECURITY.md)
- **CHANGELOG.md:** [`CHANGELOG.md`](../../CHANGELOG.md)
- **Quarterly Review Q2 2027:** [`docs/reports/quarterly-review-Q2-2027-prep.md`](../reports/quarterly-review-Q2-2027-prep.md)
- **Grant Support Kit:** [`docs/grants/grant-execution-support-kit.md`](../grants/grant-execution-support-kit.md)
- **Voting Dashboard:** [`docs/community/voting-dashboard-active.md`](../community/voting-dashboard-active.md)
- **RFC Tracking:** [`docs/governance/rfc-tracking.md`](../governance/rfc-tracking.md)

---

*Document maintained by @ed2kia/maintainers*
*License: Apache 2.0 + Ethical Use Clause*
