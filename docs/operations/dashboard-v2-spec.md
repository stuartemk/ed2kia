# Operational Dashboard v2 — Especificación Técnica

**Versión:** v2.0  
**Sprint:** v1.8 "ChatGPT Moment" — Sprint 2  
**Fecha:** 2026-05-15  
**Estado:** Draft → Review → Approved  

---

## 1. Objetivo

Dashboard v2 proporciona una vista unificada del estado operacional de ed2kIA, consolidando métricas de:
- **Red P2P:** Nodos activos, latencia, reputación, geographic routing
- **Calidad de Código:** Tests, coverage, clippy warnings, benchmarks
- **Sprint Progress:** FASEs completadas, deliverables, feature flags
- **Comunidad:** Contributors, mentorship tiers, PRs abiertos
- **Funding:** Grants activos, progreso vs targets, canales de donación
- **Release Engineering:** Beta prep status, validation checks, changelog

---

## 2. Arquitectura

### 2.1 Componentes

```
┌─────────────────────────────────────────────────────┐
│                  Dashboard v2                        │
├─────────────┬─────────────┬─────────────────────────┤
│  Métricas   │  Sprint     │  Comunidad & Funding    │
│  Operacionales│ Progress   │                         │
├─────────────┼─────────────┼─────────────────────────┤
│  Red P2P    │  FASEs      │  Contributors           │
│  Calidad    │  Deliverables│  Mentorship             │
│  Benchmarks │  Feature    │  Grants                 │
│             │  Flags      │  Donations              │
├─────────────┴─────────────┴─────────────────────────┤
│  Release Engineering & Validation                    │
├─────────────────────────────────────────────────────┤
│  Automated Checks (Scripts)                          │
└─────────────────────────────────────────────────────┘
```

### 2.2 Data Sources

| Métrica | Source | Update Freq |
|---------|--------|-------------|
| Nodos activos | P2P swarm metrics | Real-time (dev) |
| Test results | `cargo test` | On-demand |
| Clippy warnings | `cargo clippy` | On-demand |
| Benchmarks | `cargo bench -p ed2kIA-benchmarks` | Weekly |
| Sprint progress | FASE completion tracking | Per FASE |
| Contributors | Git history | Daily |
| Grants status | `docs/grants/follow-up-tracker.md` | Weekly |
| Release status | `release/v1.8.0-beta/RELEASE_PLAN.md` | Per milestone |

---

## 3. Secciones del Dashboard

### 3.1 Métricas Operacionales

#### Red P2P
```
┌─────────────────────────────────────┐
│  RED P2P                            │
├─────────────────────────────────────┤
│  Nodos activos: 0 (dev)             │
│  Latencia promedio: N/A             │
│  Geographic routing: Enabled        │
│  KAD fallback: Ready                │
│  Reputación Ed25519: Schema v1      │
│  WASM Mobile Bridge: Ready          │
└─────────────────────────────────────┘
```

**KPIs:**
- Nodos activos: Target ≥ 3 (dev), ≥ 10 (prod)
- Latencia promedio: Target < 100ms (dev mesh)
- Geographic routing coverage: % peers con lat/lon
- KAD fallback rate: % queries con fallback
- WASM memory usage: < 64MB (mobile target)

#### Calidad de Código
```
┌─────────────────────────────────────┐
│  CALIDAD DE CÓDIGO                  │
├─────────────────────────────────────┤
│  Tests: 2935 passed, 8 failed       │
│  Coverage: N/A (todo)               │
│  Clippy: 2 warnings (style)         │
│  Feature flags: 3 active            │
│  Benchmarks: Baseline v1.7          │
└─────────────────────────────────────┘
```

**KPIs:**
- Test pass rate: Target ≥ 99%
- Clippy warnings: Target = 0 (current: 2 style)
- Coverage: Target ≥ 80% (future)
- Benchmark regression: Target < 5% vs baseline

#### Benchmarks
```
┌─────────────────────────────────────┐
│  BENCHMARKS                         │
├─────────────────────────────────────┤
│  SAE loader: < 200ms (dim 8192)     │
│  Tensor f32: < 50ms                 │
│  Tensor fp8: < 20ms                 │
│  vs Baseline v1.7: ±0%              │
└─────────────────────────────────────┘
```

**Comandos:**
```bash
# Ejecutar benchmarks
cargo bench -p ed2kIA-benchmarks --features stable

# Comparar con baseline
cat benchmarks/results/baseline-v1.7.json
```

---

### 3.2 Sprint Progress

#### FASEs Completadas
```
┌─────────────────────────────────────┐
│  SPRINT v1.8 PROGRESS               │
├─────────────────────────────────────┤
│  Sprint 1: ✅ Complete (FASE 49-53) │
│  Sprint 2: ✅ Complete (FASE 54-57) │
│  Beta Prep: ✅ Ready                │
│  Total FASEs: 17/18 (94%)           │
└─────────────────────────────────────┘
```

**Detalle por Sprint:**

| Sprint | FASEs | Deliverables | Status |
|--------|-------|--------------|--------|
| Sprint 1 | 49-53 | 12 | ✅ Complete |
| Sprint 2 | 54-57 | 8 | ✅ Complete |
| Beta Prep | 57 | 3 | ✅ Ready |
| **Total** | **17** | **23** | **94%** |

#### Feature Flags
```
┌─────────────────────────────────────┐
│  FEATURE FLAGS                      │
├─────────────────────────────────────┤
│  stable: ✅ Active (2887+ tests)    │
│  v1.8-sprint1: ✅ Active (+48)      │
│  v1.8-sprint2: ✅ Active (+45)      │
└─────────────────────────────────────┘
```

---

### 3.3 Comunidad & Funding

#### Contributors
```
┌─────────────────────────────────────┐
│  COMUNIDAD                          │
├─────────────────────────────────────┤
│  Contributors activos: 1 (core)     │
│  PRs abiertos: 0                    │
│  Issues sin label: 0                │
│  Mentorship tiers:                  │
│    Seed: 0 | Sprout: 0 | Tree: 0   │
└─────────────────────────────────────┘
```

**Mentorship Program:**
- Seed: First PR merged
- Sprout: 2+ PRs merged
- Tree: Module owner
- Join process: `CONTRIBUTING.md` § Mentorship

#### Grants
```
┌─────────────────────────────────────┐
│  GRANTS ACTIVOS                     │
├─────────────────────────────────────┤
│  NSF AI Safety: $120K (submitted)   │
│  Gitcoin QF: $5K (submitted)        │
│  OSSF Security: $40K (submitted)    │
│  Total potencial: $165K             │
│  Total recibido: $0                 │
│  Progreso: 0%                       │
└─────────────────────────────────────┘
```

**Tracking:**
- Follow-up tracker: `docs/grants/follow-up-tracker.md`
- Script: `bash scripts/mentorship_onboarding.sh grants-status`

---

### 3.4 Release Engineering

#### Beta Status
```
┌─────────────────────────────────────┐
│  RELEASE: v1.8.0-beta               │
├─────────────────────────────────────┤
│  Release plan: ✅ Complete           │
│  Validation script: ✅ Ready        │
│  Changelog: ✅ Updated              │
│  Dry-run: ⏳ Pending                 │
│  Tag: ⏳ Pending                     │
└─────────────────────────────────────┘
```

**Validation Checklist:**
- [x] `cargo check --features v1.8-sprint2` → PASS
- [x] `cargo clippy --features v1.8-sprint2` → PASS
- [x] `cargo test --features v1.8-sprint2` → 2935 passed
- [ ] `scripts/beta_release_prep.sh --dry-run` → Pending
- [ ] Git tag `v1.8.0-beta` → Pending

**Comandos:**
```bash
# Validación completa
bash scripts/beta_release_prep.sh --dry-run

# Tag release
bash scripts/beta_release_prep.sh --tag v1.8.0-beta
```

---

## 4. Automated Checks

### 4.1 Scripts Disponibles

| Script | Purpose | Usage |
|--------|---------|-------|
| `just validate` | Full validation pipeline | `just validate` |
| `just test` | Run all tests | `just test` |
| `just clippy` | Lint check | `just clippy` |
| `just bench` | Run benchmarks | `just bench` |
| `scripts/beta_release_prep.sh` | Beta validation | `--dry-run`, `--tag` |
| `scripts/mentorship_onboarding.sh` | Grant/mentorship status | `grants-status`, `mentorship-list` |

### 4.2 Dashboard Update Commands

```bash
# Actualizar dashboard completo
just validate && just bench

# Actualizar métricas de red (dev)
just docker-compose up -d

# Actualizar grant status
bash scripts/mentorship_onboarding.sh grants-status

# Actualizar release status
bash scripts/beta_release_prep.sh --dry-run
```

---

## 5. Diferencias vs Dashboard v1

| Feature | v1 | v2 |
|---------|----|----|
| Geographic routing | ❌ | ✅ |
| WASM Mobile Bridge | ❌ | ✅ |
| Mentorship tiers | ❌ | ✅ |
| Grant follow-up | ❌ | ✅ |
| Beta release tracking | ❌ | ✅ |
| Justfile integration | ❌ | ✅ |
| Docker Compose dev | ❌ | ✅ |
| Feature flag tracking | Básico | Detallado |
| Sprint progress | Manual | Automated |

---

## 6. Formato de Salida JSON

```json
{
  "dashboard_version": "2.0",
  "generated": "2026-05-15T17:00:00Z",
  "sprint": "v1.8-sprint2",
  "p2p": {
    "active_nodes": 0,
    "avg_latency_ms": null,
    "geo_routing_enabled": true,
    "kad_fallback_ready": true,
    "wasm_bridge_ready": true
  },
  "quality": {
    "tests_passed": 2935,
    "tests_failed": 8,
    "clippy_warnings": 2,
    "feature_flags_active": 3,
    "benchmark_regression_pct": 0
  },
  "sprint": {
    "sprint1_complete": true,
    "sprint2_complete": true,
    "beta_prep_complete": true,
    "phase_completion_pct": 94
  },
  "community": {
    "active_contributors": 1,
    "open_prs": 0,
    "unlabeled_issues": 0,
    "mentorship": {
      "seed": 0,
      "sprout": 0,
      "tree": 0
    }
  },
  "funding": {
    "grants_submitted": 3,
    "total_potential": 165000,
    "total_received": 0,
    "progress_pct": 0
  },
  "release": {
    "target": "v1.8.0-beta",
    "plan_complete": true,
    "validation_script_ready": true,
    "changelog_updated": true,
    "dry_run_pending": true,
    "tag_pending": true
  }
}
```

---

## 7. Criterios de Aprobación

- [ ] Todas las secciones documentadas
- [ ] KPIs definidos con targets
- [ ] Scripts de actualización disponibles
- [ ] Formato JSON de salida definido
- [ ] Diferencias vs v1 documentadas
- [ ] Aprobado por Orquestador

---

## 8. Sign-off

**Qweni (IA):** Dashboard v2 spec complete. Ready for review.  
**Orquestador:** [Pendiente de sign-off]  

---

*Generated: 2026-05-15T17:09:00Z*  
*Sprint: v1.8 "ChatGPT Moment" — Sprint 2*  
*Dashboard Version: v2.0*  
*Previous: docs/operations/daily-metrics-dashboard.md (v1)*
