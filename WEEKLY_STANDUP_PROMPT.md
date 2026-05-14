# WEEKLY STANDUP PROMPT — ed2kIA v1.7

**Propósito:** Template reutilizable para sesiones de standup semanal asistido por IA.
**Frecuencia:** Cada lunes 10:00 CST (o primer día hábil del sprint)
**Duración objetivo:** 30 minutos (automatizado)

---

## Contexto del Sprint

```
Sprint: v1.7 — RFC-001 Latency Mitigation
Versión base: v1.6.0-stable
Objetivo: Reducir latencia de tensor streaming de ~350ms a <50ms
Estrategias: Prefetching, FP8/INT4 quantization, Geographic Routing, Async Steering
```

## Tareas del Standup

### Tarea 1: Revisar PRs Abiertas
```bash
# Listar PRs abiertas
gh pr list --state open --limit 20

# PRs esperando review > 48h
gh pr list --state open --search "created:>=$(date -d '3 days ago' +%Y-%m-%d)"

# PRs con CI fallando
gh pr list --state open --json statusCheckRollup --jq '.[] | select(.statusCheckRollup[].conclusion == "FAILURE")'
```

**Acción:** Asignar reviewers, escalar bloqueos, cerrar PRs stale.

### Tarea 2: Actualizar Benchmarks
```bash
# Ejecutar benchmark suite
cargo bench -p ed2kIA-benchmarks --features stable --nopreview 2>&1 | tee benchmarks/results/latest.log

# Comparar con baseline
# (manual o script: diff benchmarks/results/baseline-v1.7.json vs latest)
```

**Acción:** Actualizar `benchmarks/results/baseline-v1.7.json` si hay mejora sostenida.

### Tarea 3: Revisar Issues Activos
```bash
# Issues abiertas por etiqueta
gh issue list --label "v1.7" --state open
gh issue list --label "good-first-issue" --state open

# Issues sin actividad > 7 días
gh issue list --state open --search "updated:<$(date -d '7 days ago' +%Y-%m-%d)"
```

**Acción:** Reasignar issues estancadas, cerrar duplicadas, crear nuevas si es necesario.

### Tarea 4: Escalar Bloqueos
- Dependencias externas pendientes (ej: Candle update, libp2p features)
- Infraestructura (CI runners, artifact storage)
- Decisiones de arquitectura pendientes (documentar en RFC)
- Recursos humanos (necesidad de mentoría, review capacity)

### Tarea 5: Planificar Next Sprint
- Revisar progreso vs objetivos de semana actual
- Ajustar prioridades para próxima semana
- Actualizar `docs/sprint-v1.7-weekly-sync.md`
- Preparar handover para próximo shift

---

## Formato de Salida JSON

```json
{
  "standup": {
    "date": "2026-05-20",
    "sprint": "v1.7-week-2",
    "presenter": "Qweni AI Assistant"
  },
  "metrics": {
    "tests_passing": 28,
    "tests_total": 28,
    "prs_open": 0,
    "prs_merged_this_week": 0,
    "issues_open": 4,
    "issues_closed_this_week": 0,
    "benchmark_fp8_mape": 0.1,
    "benchmark_int4_mape": 7.0
  },
  "diff_metrics": {
    "tests_delta": 0,
    "prs_delta": 0,
    "issues_delta": 0,
    "benchmark_delta": "N/A (baseline pending)"
  },
  "blockers": [],
  "actions": [
    {
      "task": "Ejecutar cargo bench local",
      "owner": "Core Team",
      "deadline": "2026-05-22"
    },
    {
      "task": "Crear issues vía gh CLI",
      "owner": "Maintainer",
      "deadline": "2026-05-18"
    }
  ],
  "next_sprint_priorities": [
    "Optimizar cuantización block-based para mejor compresión",
    "Integrar quantization en benchmark suite",
    "Agregar tokio-async a AsyncSteeringChannel"
  ]
}
```

---

## Protocolo de Rollback

### Si validación falla:
1. **NO PUSH** — Detener ciclo auto-push
2. Identificar commit problemático: `git log --oneline -10`
3. Opciones:
   - Fix in-place: Corregir y re-validar
   - Revert: `git revert <commit_hash>`
   - Hotfix branch: `git checkout -b hotfix/<description>`
4. Documentar en `docs/incident-log.md`
5. Notificar en Discord #dev-chat si afecta a contribuyentes

### Validación ética:
- Verificar que cambios no introducen unsafe code
- Confirmar que benchmarks no están hardcodeados
- Revisar que documentación refleja estado real del código
- Asegurar transparencia en métricas (no ocultar regresiones)

---

## Referencias

- **RFC-001:** `docs/rfc/rfc-001-latency-mitigation-v1.7.md`
- **Weekly Sync:** `docs/sprint-v1.7-weekly-sync.md`
- **Baseline:** `benchmarks/results/baseline-v1.7.json`
- **Feedback Loop:** `docs/community-feedback-loop.md`
- **Auto-Push Protocol:** `CONTRIBUTING.md` § Auto-Push Permanente
- **Day 1 Operations:** `DAY1_OPERATIONS_PROMPT.md`

---

## Cómo Usar Este Prompt

1. Copiar este archivo como referencia para cada standup semanal
2. Ejecutar comandos de diagnóstico
3. Generar JSON de salida con métricas actuales
4. Actualizar `docs/sprint-v1.7-weekly-sync.md` con resultados
5. Commit + push si validación = PASS
6. Archivar JSON en `release/reports/standup-YYYY-MM-DD.json`
