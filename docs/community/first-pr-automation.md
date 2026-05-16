# Automatización Primeros PRs — ed2kIA v1.9

**Versión:** v1.9 Sprint 2
**Fecha:** 2026-05-16
**Responsable:** Core Team + Community Maintainers

---

## Objetivo

Reducir tiempo de onboarding de contribuidores externos de 72h → 24h mediante automatización del flujo first-PR.

---

## Flujo Automatizado

```
[Fork] → [Draft PR] → [Bot Check] → [Auto-Label] → [Mentor Assign] → [Review] → [Auto-Merge]
   │          │           │            │             │            │           │
  Auto     Template    CI/CLippy    good-first   Rotating     24-48h     Score >= 0.8
  message  fill       pass check    +ready       mentor
```

---

## Componentes

### 1. PR Template Auto-Fill

GitHub auto-fills PR description desde issue linked:

```markdown
## Descripción
<!-- Auto-filled desde issue #N -->

## Cambios
- [ ] Code sigue Rust best practices (no unsafe)
- [ ] Tests cubren edge cases
- [ ] `cargo test` pasa
- [ ] `cargo clippy` sin warnings
- [ ] Documentación actualizada

## Métricas
| Benchmark | Baseline | PR | Δ |
|-----------|----------|----|---|
|           |          |    |   |

## Verificación RFC-001
- [ ] Alignment verificado
```

### 2. Bot Checks (GitHub Actions)

Trigger: `pull_request_target` on `opened`

**Checks automáticos:**
1. **Label auto-assign**: `good-first-issue` + `first-pr` + `pending-review`
2. **CI trigger**: `cargo check` + `cargo test` + `cargo clippy`
3. **Mentor assignment**: Rotating mentor desde `@ed2kia/mentors`
4. **Template validation**: Verificar PR description completa

### 3. Mentor Assignment Rotating

```
Week N:   Mentor A (P0-P1 issues)
Week N+1: Mentor B (P2-P3 issues)
Week N+2: Mentor C (docs/features)
```

**SLA mentor:**
- Ack PR: 4h
- Initial review: 24h
- Final approval: 48h

### 4. Auto-Merge Criteria

PR se auto-mergea cuando:
- [ ] CI pasa (check + test + clippy)
- [ ] 2 approvals de `@ed2kia/core-team`
- [ ] Score de calidad >= 0.8
- [ ] No tiene label `hold` o `needs-work`
- [ ] Linked issue tiene label `good-first-issue`

**Score de calidad:**
```
score = 0.4 * test_coverage + 0.3 * clippy_clean + 0.2 * doc_quality + 0.1 * benchmark_pass
```

---

## Scripts de Automatización

### Auto-Merge Script

Ubicación: `scripts/auto_merge_pr.sh`

```bash
#!/usr/bin/env bash
# auto_merge_pr.sh — Verifica y mergea PRs que cumplen criterios
# Uso: ./scripts/auto_merge_pr.sh <pr_number>
```

### PR Triage Script

Ubicación: `scripts/pr_triage.sh`

```bash
#!/usr/bin/env bash
# pr_triage.sh — Clasifica PRs entrantes y asigna mentors
# Uso: ./scripts/pr_triage.sh [--dry-run]
```

---

## Métricas de Éxito

| Métrica | Baseline | Target v1.9 |
|---------|----------|-------------|
| Time to first review | 72h | 24h |
| First-PR merge rate | 45% | 70% |
| Contributor retention (30d) | 30% | 50% |
| Auto-merge rate | 0% | 25% |

---

## Integración con Existing Tools

- **CODEOWNERS**: Actualizado con rutas v1.9 (`/src/zkp/proof_aggregation.rs`, `/src/gui/neural_steer_ui.rs`)
- **CI (ci.yml)**: Paths trigger para first-PR checks
- **Feedback loop**: `docs/feedback_integration_log.md` tracka onboarding feedback

---

## Próximos Pasos

- [ ] Configurar GitHub Actions para auto-label
- [ ] Implementar mentor rotation schedule
- [ ] Deploy auto-merge script en CI
- [ ] Monitor métricas primer mes

---

*Documento generado para FASE 74 — Automatización Primeros PRs & Onboarding Externo*
