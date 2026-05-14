# Daily Metrics Dashboard — ed2kIA v1.7+

**Version:** v1.7.0-stable+
**Sprint Activo:** v1.8 "ChatGPT Moment"
**Última Actualización:** 2026-05-14
**Frecuencia:** Daily (automated where possible)

---

## 1. Nodos Activos & Cómputo Donado

| Métrica | Target v1.8 | Actual | Δ |
|---------|-------------|--------|---|
| Nodos activos | 100 | — | — |
| Cómputo donado (GPU-hrs/day) | 500 | — | — |
| Federaciones registradas | 10 | — | — |
| Cross-model sync rate | 95% | — | — |

**Comandos de verificación:**
```bash
# Active nodes (when network is running)
curl -s http://localhost:8080/api/v1/nodes | jq '.nodes | length'

# Compute donated (from metrics endpoint)
curl -s http://localhost:8080/api/v1/metrics | jq '.compute_total_gpu_hrs'
```

---

## 2. Reputación Ed25519

| Métrica | Target v1.8 Sprint 1 | Actual | Δ |
|---------|---------------------|--------|---|
| Contribuidores verificados | 50 | — | — |
| Pruebas Ed25519 firmadas | 500 | — | — |
| Badges otorgados | 25 | — | — |
| Tier distribution | Observer:60% Contributor:30% Other:10% | — | — |

**Referencia:** [`docs/architecture/reputation-gamification.md`](../architecture/reputation-gamification.md)

---

## 3. Issues/PRs Pendientes

| Métrica | Target | Actual | SLA |
|---------|--------|--------|-----|
| Issues sin label | 0 | — | <4h |
| Issues sin assignee | 0 | — | <24h |
| PRs pending review | 0 | — | <24h |
| PRs with CI red | 0 | — | IMMEDIATE |
| Good-first-issues available | 10 | — | — |

**Comandos de verificación:**
```bash
# Issues without labels
gh issue list --label "" --json title --jq 'length'

# PRs pending review
gh pr list --review-requested "@me" --json title --jq 'length'

# Good-first-issues open
gh issue list --label "good-first-issue" --json state --jq 'length'
```

---

## 4. SLA Compliance

| SLA | Target | Compliance | Status |
|-----|--------|------------|--------|
| Issue response time | <24h | — | 🟡 |
| PR review time | <48h | — | 🟡 |
| CI pipeline time | <15min | — | 🟡 |
| Hotfix deployment | <4h | — | 🟡 |
| Weekly report | Monday 09:00 | — | 🟡 |

**Status Legend:** 🟢 On Track | 🟡 At Risk | 🔴 Breached

---

## 5. Funding Recibido & Grants Aplicados

| Canal | Target Mes 1 | Recibido | Grants | Status |
|-------|-------------|----------|--------|--------|
| GitHub Sponsors | $500 | — | N/A | 🟡 |
| Open Collective | $500 | — | N/A | 🟡 |
| Gitcoin Grants | $5K | — | Applied: 0 | 🟡 |
| Crypto Donations | $250 | — | N/A | 🟡 |
| Corporate Sponsors | $1K | — | Pipeline: 0 | 🟡 |
| **TOTAL** | **$6.25K** | **—** | **—** | **🟡** |

**Referencia:** [`SUPPORT.md`](../../SUPPORT.md), [`docs/funding-strategy.md`](../funding-strategy.md), [`docs/funding-setup-checklist.md`](../funding-setup-checklist.md)

---

## 6. Benchmarks vs Baseline

| Benchmark | Baseline v1.7 | Actual | Δ | Target |
|-----------|--------------|--------|---|--------|
| FP8 throughput | >500 MB/s | — | — | >500 MB/s |
| INT4 throughput | >200 MB/s | — | — | >200 MB/s |
| FP8 precision loss | <2% MAPE | — | — | <2% |
| INT4 precision loss | <10% MAPE | — | — | <10% |
| Async steering latency | <5ms | — | — | <5ms |
| SAE load (8192) | <50ms | — | — | <50ms |

**Comando de ejecución:**
```bash
cargo bench -p ed2kIA-benchmarks --features stable
```

**Referencia:** [`benchmarks/results/baseline-v1.7.json`](../../benchmarks/results/baseline-v1.7.json)

---

## 7. Latencia Async Steering

| Métrica | Target | Actual | Status |
|---------|--------|--------|--------|
| P50 latency | <2ms | — | 🟡 |
| P95 latency | <5ms | — | 🟡 |
| P99 latency | <10ms | — | 🟡 |
| Signal drop rate | <0.1% | — | 🟡 |
| Correction accuracy | >95% | — | 🟡 |

---

## 8. Comunidad & Engagement

| Métrica | Target Sprint 1 | Actual | Δ |
|---------|----------------|--------|---|
| GitHub Stars | 100 | — | — |
| Forks | 25 | — | — |
| Discord Members | 50 | — | — |
| Weekly Active Contributors | 10 | — | — |
| K-factor (invitations/user) | 0.8 | — | — |

---

## 9. v1.8 Sprint Progress

| Sprint Phase | Status | Deliverables | Due |
|--------------|--------|-------------|-----|
| Sprint 1: Foundation | 🟡 Planning | WASM core, browser extension | Week 2 |
| Sprint 2: Gamification | ⏸️ Queued | Badge system, leaderboard | Week 4 |
| Sprint 3: Scale | ⏸️ Queued | Mobile services, impact dashboard | Week 6 |
| Sprint 4: Launch | ⏸️ Queued | Public launch, 100K DAV target | Week 8 |

**Referencia:** [`docs/roadmap/v1.8-chatgpt-moment.md`](../roadmap/v1.8-chatgpt-moment.md)

---

## 10. Risk Register

| Risk | Severity | Probability | Mitigation | Owner |
|------|----------|-------------|------------|-------|
| Low contributor adoption | High | Medium | Good-first-issues + funnel | Community |
| Funding shortfall | High | Medium | Multi-channel strategy | Core Team |
| Performance regression | Medium | Low | Automated benchmarks | Engineering |
| Security vulnerability | Critical | Low | Security audit + monitoring | Security |
| Scope creep v1.8 | Medium | Medium | Strict sprint boundaries | Orchestrator |

---

## Automated Checks

```bash
# Run all validations
echo "=== Funding Channels ==="
bash scripts/verify_funding_channels.sh

echo "=== CI Status ==="
cargo check --features stable 2>&1 | tail -1

echo "=== Test Count ==="
cargo test --features stable 2>&1 | grep "test result"

echo "=== Open Issues ==="
gh issue list --json state --jq 'length' 2>/dev/null || echo "gh not available"

echo "=== Open PRs ==="
gh pr list --json state --jq 'length' 2>/dev/null || echo "gh not available"
```

---

## Weekly Report Template

Cada lunes, generar `release/reports/weekly-YYYY-MM-DD.md`:

```markdown
# Weekly Report — YYYY-MM-DD

## Summary
- Contributors: [N] (+[Δ] vs last week)
- Issues closed: [N]
- PRs merged: [N]
- Funding received: $[N]

## Key Achievements
1. ...
2. ...
3. ...

## Blockers
- ...

## Next Week Priorities
1. ...
2. ...
3. ...

## Metrics Dashboard
[Link to updated dashboard]
```

---

## Integration with Operations Prompts

- **Daily:** [`DAY1_OPERATIONS_PROMPT.md`](../../DAY1_OPERATIONS_PROMPT.md) v2.0
- **Weekly:** `WEEKLY_STANDUP_PROMPT.md`
- **Funding:** [`docs/funding-setup-checklist.md`](../funding-setup-checklist.md)
- **Community:** [`COMMUNITY_LAUNCH_CHECKLIST.md`](../../COMMUNITY_LAUNCH_CHECKLIST.md)
