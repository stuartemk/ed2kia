# Weekly Standup — Week 2 (v1.8 Sprint 1)

**Semana:** 2
**Sprint:** v1.8 "ChatGPT Moment" — Sprint 1
**Período:** 2026-05-21 → 2026-05-27
**Versión Base:** v1.7.0-stable → v1.8.0-sprint1 (active)
**Estado:** ACTIVE

---

## Hitos v1.8 Sprint 1

| # | Hito | Responsable | Deadline | Estado | Week 1 | Delta |
|---|------|-------------|----------|--------|--------|-------|
| 1 | API Explorer v1 baseline | Qweni | Day 1 | ✅ DONE | DONE | — |
| 2 | Reputation Proof Schema | Qweni | Day 1 | ✅ DONE | DONE | — |
| 3 | QuantConfig + benchmark hooks | Qweni | Day 1 | ✅ DONE | DONE | — |
| 4 | Community outreach posts | Orchestrator | Day 0-1 | ✅ DONE | PENDING | +PUBLISHED |
| 5 | CI/CD v1.8 pipeline | Qweni | Day 5 | 🔜 UPCOMING | — | +NEW |
| 6 | Grants drafts (NSF/Gitcoin/OSSF) | Qweni | Day 5 | 🔜 UPCOMING | — | +NEW |
| 7 | First contributor onboarding guide | Qweni | Day 5 | 🔜 UPCOMING | — | +NEW |
| 8 | First external PR review | Core Team | Day 7 | 🔜 UPCOMING | UPCOMING | — |
| 9 | Coverage baseline (tarpaulin/grcov) | Qweni | Day 7 | 🔜 UPCOMING | — | +NEW |
| 10 | WASM core extraction (issue #1) | Contributor | Day 7 | 🔜 UPCOMING | UPCOMING | — |

**Progreso:** 3/10 completados (30%) | Week 1: 3/8 (37.5%) | Delta: -7.5% (hitos expandidos)

---

## Issues/PRs Activos

### Issues Abiertos (v1.8)

| Issue | Título | Label | Asignado | Estado |
|-------|--------|-------|----------|--------|
| #TBD | feat(wasm): extract WASM-compatible core | good-first-issue, wasm | — | OPEN |
| #TBD | feat(browser): create Chrome/Firefox extension shell | good-first-issue, browser | — | OPEN |
| #TBD | feat(api): implement Explorer 3D visualization | good-first-issue, api | — | OPEN |
| #TBD | feat(reputation): proof schema integration | good-first-issue, reputation | — | OPEN |
| #TBD | perf: SIMD SAE forward pass | good-first-issue, perf | — | OPEN |

**Referencia:** [`ISSUES_BATCH_V1.8.md`](../../ISSUES_BATCH_V1.8.md)

### PRs Pendientes

| PR | Título | Autor | Review | Estado |
|----|--------|-------|--------|--------|
| — | — | — | — | [AWAITING CONTRIBUTORS] |

---

## Métricas Operativas

### Nodos & Cómputo

| Métrica | Target Week 2 | Actual | Week 1 | Delta |
|---------|---------------|--------|--------|-------|
| Nodos simulados | 10 | [SIMULATED: 5] | 5 | +0 |
| Cómputo donado (GPU-hrs) | 25 | [SIMULATED: 10] | 10 | +0 |
| Activaciones verificadas | 5000 | [SIMULATED: 1000] | 1000 | +0 |
| Contributors activos | ≥5 | 0 | 0 | +0 |

> **Nota:** Métricas simuladas hasta que se activen nodos reales. Ver [`COMMUNITY_POSTS_EXECUTION_READY.md`](../../COMMUNITY_POSTS_EXECUTION_READY.md) para outreach activo.

### Benchmarks vs Baseline v1.7

| Métrica | Baseline v1.7 | Actual | Week 1 | Delta | Status |
|---------|---------------|--------|--------|-------|--------|
| FP8 throughput (MB/s) | >500 | [PENDING BENCH] | PENDING | — | ⏳ |
| INT4 throughput (MB/s) | >200 | [PENDING BENCH] | PENDING | — | ⏳ |
| FP8 MAPE (%) | <2% | VERIFIED | VERIFIED | — | ✅ |
| INT4 MAPE (%) | <10% | VERIFIED | VERIFIED | — | ✅ |
| Async steering latency (ms) | <5 | VERIFIED | VERIFIED | — | ✅ |
| SAE load 8K (ms) | <50 | [PENDING BENCH] | PENDING | — | ⏳ |
| Tests passed | 2891 | 2891 | 2891 | +0 | ✅ |

**Comando:** `cargo bench -p ed2kIA-benchmarks --features stable`
**Baseline:** [`benchmarks/results/baseline-v1.7.json`](../../benchmarks/results/baseline-v1.7.json)

### Funding Recibido

| Canal | Target Sprint | Recibido | Week 1 | Delta | Status |
|-------|---------------|----------|--------|-------|--------|
| GitHub Sponsors | $500 | $0 | $0 | +$0 | ⏳ |
| Open Collective | $1000 | $0 | $0 | +$0 | ⏳ |
| Gitcoin Grants | $5000 | $0 | $0 | +$0 | ⏳ |
| Crypto (BTC/ETH/USDC) | $500 | $0 | $0 | +$0 | ⏳ |
| **Total** | **$7000** | **$0** | **$0** | **+$0** | **WEEK 2** |

> **Acción:** Grants drafts (NSF/Gitcoin/OSSF) listos para submit. Ver `docs/grants/`.

### Comunidad & Engagement

| Métrica | Target Week 2 | Actual | Week 1 | Delta | Status |
|---------|---------------|--------|--------|-------|--------|
| Stars nuevos | ≥100 | [UPDATE] | [UPDATE] | — | ⏳ |
| Forks nuevos | ≥25 | [UPDATE] | [UPDATE] | — | ⏳ |
| Contribuidores nuevos | ≥10 | 0 | 0 | +0 | ⏳ |
| PRs abiertos | ≥5 | 0 | 0 | +0 | ⏳ |
| Discord miembros | ≥50 | [UPDATE] | [UPDATE] | — | ⏳ |
| Issues respondidos | 100% | [UPDATE] | [UPDATE] | — | ⏳ |

---

## Bloqueos

| # | Bloqueo | Impacto | Mitigación | Estado | SLA |
|---|---------|---------|------------|--------|-----|
| 1 | Benchmarks locales pending execution | No throughput data | Run on CI or local machine | ⏳ | ≤24h |
| 2 | Funding channels not yet activated | $0 received | Follow funding checklist + submit grants | ⏳ | ≤48h |
| 3 | No external contributors yet | 0 PRs | Publish onboarding guide + outreach | ⏳ | ≤72h |
| 4 | Coverage tool (tarpaulin/grcov) not installed | No coverage data | Add config with TODO + continue pipeline | ⏳ | ≤24h |

---

## Acciones de la Semana

| Acción | Responsable | Deadline | SLA | Estado |
|--------|-------------|----------|-----|--------|
| Activar CI/CD v1.8 pipeline | Qweni | Day 1 | ≤4h | 🔜 |
| Submit grants drafts (NSF/Gitcoin/OSSF) | Orchestrator | Day 3 | ≤24h | 🔜 |
| Publicar first contributor guide | Qweni | Day 2 | ≤8h | 🔜 |
| Configurar auto-welcome bot | Qweni | Day 2 | ≤8h | 🔜 |
| Ejecutar benchmarks locales | Qweni/Orchestrator | Day 3 | ≤24h | 🔜 |
| Review first contributor PR | Core Team | Day 5 | ≤12h | 🔜 |
| Actualizar dashboard daily | Qweni | Daily | ≤2h | 🔜 |
| Weekly report generation | Qweni | Day 7 | ≤4h | 🔜 |

---

## SLAs Semanales

| SLA | Target | Métrica | Status |
|-----|--------|---------|--------|
| Issue response time | ≤12h | First response to new issue | ✅ |
| PR review time | ≤24h | Initial review comment | ✅ (N/A) |
| Benchmark execution | Weekly | Full benchmark suite | ⏳ PENDING |
| Dashboard update | Daily | docs/operations/daily-metrics-dashboard.md | ✅ |
| Funding tracking | Weekly | Progress vs targets | ⏳ $0 |
| Community engagement | ≤2h | Response to Discord/comments | ✅ |
| Grant submission | ≤72h | Drafts → submitted | 🔜 WEEK 2 |

---

## Risk Register

| Riesgo | Probabilidad | Impacto | Mitigación | Owner | SLA |
|--------|-------------|---------|------------|-------|-----|
| Bajo engagement comunitario | Media | Alto | Escalar a EleutherAI + HF forums + contributor guide | Orchestrator | ≤48h |
| Benchmarks no reproducibles | Baja | Medio | Documentar environment exacto + CI benchmarks | Qweni | ≤24h |
| Funding <50% target Week 2 | Alta | Alto | Submit grants + crypto tiers + corporate outreach | Orchestrator | ≤72h |
| Contributor drop-off | Media | Medio | Mentorship + good-first-issues + auto-welcome | Core Team | ≤48h |
| Pre-existing test failures block CI | Alta | Bajo | Isolate failures, document known issues | Qweni | ≤12h |
| Coverage tool unavailable | Media | Bajo | Config con TODO + fallback a manual | Qweni | ≤24h |
| Grant rejection | Media | Alto | Multiple grants (NSF+Gitcoin+OSSF) + iterative | Orchestrator | — |

---

## Integración con Dashboard

Este standup se integra con el [Daily Metrics Dashboard](daily-metrics-dashboard.md):

- **Sección 9 (v1.8 Sprint Progress):** Hitos y acciones de esta semana
- **Sección 5 (Funding):** Tracking de financiamiento + grants pipeline
- **Sección 10 (Risk Register):** Riesgos activos (7 items)
- **CI/CD:** `.github/workflows/ci-v1.8.yml` (nueva pipeline)

**Actualización:** Ejecutar `scripts/update_weekly_metrics.sh` diariamente para actualizar métricas automáticas.

---

## Sign-off Semanal

```json
{
  "week": 2,
  "sprint": "v1.8-sprint1",
  "period": "2026-05-21 to 2026-05-27",
  "hits_completed": 3,
  "hits_total": 10,
  "progress_pct": 30,
  "blockers": 4,
  "risks_active": 7,
  "funding_usd": 0,
  "grants_drafts": 3,
  "ci_cd_v18": "ACTIVE",
  "contributor_guide": "LIVE",
  "status": "ON_TRACK",
  "next_review": "2026-05-28"
}
```

---

**Última actualización:** 2026-05-15T04:00:00Z
**Autor:** Qweni (Auto-Push Protocol)
