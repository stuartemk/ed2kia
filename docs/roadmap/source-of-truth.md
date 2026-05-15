# Fuente de Verdad — ed2kIA Source of Truth

> **Declaración Oficial:** Este documento es la única referencia válida para fases, versiones y estado del proyecto ed2kIA.
>
> **Última Actualización:** 2026-05-15T23:11:00Z
> **Proceso de Actualización:** Solo vía PR con revisión de @ed2kIA/core-team
> **Protocolo:** Auto-Push Permanente activo

---

## 1. Declaración de Autoridad

Este documento resuelve todas las discrepancias entre:
- README.md (fase status)
- Cargo.toml (version string)
- Git tags (release markers)
- Documentación de roadmap (fase planning)
- CHANGELOG (historial de cambios)

**Cualquier discrepancia entre este documento y otros artefactos indica que el otro artefacto está desactualizado.**

---

## 2. Tabla Maestra: Fase ↔ Versión ↔ Estado ↔ Commits Clave ↔ Docs

| Fase | Versión | Estado | Commits Clave | Docs de Referencia |
|------|---------|--------|---------------|-------------------|
| **FASE 1** | v0.1.0 → v0.3.0 | ✅ Completada | N/A (pre-git history) | [`README.md`](../../README.md) §Fase 1 |
| **FASE 2** | v0.3.0 → v0.4.0 | ✅ Completada | N/A (pre-git history) | [`README.md`](../../README.md) §Fase 2 |
| **FASE 3** | v0.4.0 → v0.5.0 | ✅ Completada | N/A (pre-git history) | [`README.md`](../../README.md) §Fase 3 |
| **FASE 4** | v0.5.0 → v1.0.0 | ✅ Completada | `96e3c14` (v1.0.0-stable) | [`README.md`](../../README.md) §Fase 4 |
| **FASE 5** | v1.0.0 → v1.5.0 | ✅ Completada | `96e3c14` (v1.6.0-stable includes F5) | [`README.md`](../../README.md) §Fase 5 |
| **FASE 6** | v1.6.0 → v1.8.0-beta.1 | ✅ Completada | `b72c8bd` (audit), `1e2dbe8` (readme sync) | [`phase6-audit-mapping.md`](phase6-audit-mapping.md) |
| **FASE 7** | v1.9.0 | ✅ ACTIVE | `4d3c2f5` (v1.9 roadmap), [THIS COMMIT] (unificación) | [`phase7-v1.9-unification.md`](phase7-v1.9-unification.md) |

### FASE 6 Detail (Reconciled)

| Sub-FASE | Versión | Commit | Descripción |
|----------|---------|--------|-------------|
| FASE 59 | v1.8.0-beta.1 | `2c7ab4e` | Beta Release Execution & CI Validation |
| FASE 60 | v1.8.0-beta.1 | `c676208` | Beta Testing Coordination & Feedback Pipeline |
| FASE 61 | v1.8.0-beta.1 | `f019588` | Performance Monitoring & Bug Triage Automation |
| FASE 62 | v1.8.0-beta.1 | `4d3c2f5` | Post-Beta Retrospective & v1.9 Roadmap |
| FASE 63 | v1.8.0-beta.1 | `200eb0b` | Operational Prompt v6.0 & Long-Term Maintenance |
| FASE 64 | v1.8.0-beta.1 | `b72c8bd` | FASE 6 Audit & Feature Mapping |
| FASE 65 | v1.8.0-beta.1 | `1e2dbe8` | README & Roadmap Sync |
| FASE 66 | v1.8.0-beta.1 | `65d7a5c` | Versioning Alignment & Release Strategy |
| FASE 67 | v1.8.0-beta.1 | `e46ebbe` | Source of Truth & Final Reconciliation |

### FASE 7 Detail (Unificado v1.9)

| Sub-FASE | Versión | Commit | Descripción |
|----------|---------|--------|-------------|
| FASE 68 | v1.9.0 | [THIS COMMIT] | Unificación Estratégica FASE 7 ↔ v1.9 Roadmap |
| FASE 69 | v1.9.0 | TBD | Sprint 1 — Production Hardening & Mobile GUI Foundation |
| FASE 70 | v1.9.0 | TBD | Tracking Unificado & Dashboard v3 |
| FASE 71 | v1.9.0 | TBD | Operational Prompt v7.0 & Handover Final |

---

## 3. Version Actual del Proyecto

| Campo | Valor | Fuente |
|-------|-------|--------|
| **Versión Cargo.toml** | `1.6.0-stable` | [`Cargo.toml`](../../Cargo.toml) (line 3) — Legacy, no actualizar sin core-team approval |
| **Último Tag Git** | `v1.8.0-beta.1` | `git tag -l 'v*' \| sort -V \| tail -1` |
| **Versión README Badge** | `1.8.0_BETA` | [`README.md`](../../README.md) (line 9) — Actualizada en FASE 65 |
| **Versión Operacional** | `v1.8.0-beta.1` | Este documento — **FUENTE DE VERDAD** |

**Nota:** `Cargo.toml` mantiene `1.6.0-stable` como versión base estable. La versión operacional real es `v1.8.0-beta.1` según git tag. Esta discrepancia es intencional: Cargo.toml representa la última versión stable, mientras que git tags representan el estado actual de desarrollo.

---

## 4. Documentos de Referencia

### Documentos Primarios

| Documento | Path | Propósito |
|-----------|------|-----------|
| **Source of Truth** | `docs/roadmap/source-of-truth.md` | Este documento — Referencia maestra |
| **FASE 6 Audit** | `docs/roadmap/phase6-audit-mapping.md` | Mapeo FASE 6 items → implementaciones reales |
| **Versioning Alignment** | `docs/roadmap/versioning-alignment.md` | Matriz Fase↔Versión, feature gates, branching |
| **v1.9 Roadmap** | `docs/roadmap/v1.9-roadmap-draft.md` | Planificación v1.9 "Production Ready" |
| **FASE 7 Unification** | `docs/roadmap/phase7-v1.9-unification.md` | Unificación FASE 7 = v1.9, Sprint mapping, governance |

### Documentos Secundarios

| Documento | Path | Propósito |
|-----------|------|-----------|
| **GOVERNANCE.md** | `GOVERNANCE.md` | Gobernanza del proyecto |
| **CONTRIBUTING.md** | `CONTRIBUTING.md` | Guía de contribución + versioning |
| **DAY1_OPERATIONS_PROMPT.md** | `DAY1_OPERATIONS_PROMPT.md` | Prompt operacional v6.0 |
| **README.md** | `README.md` | Documentación pública principal |
| **Changelog** | `release/changelog.md` | Historial de cambios |

### Documentos Beta

| Documento | Path | Propósito |
|-----------|------|-----------|
| **Release Notes** | `release/v1.8.0-beta.1/RELEASE_NOTES.md` | Notas de release beta |
| **Tester Onboarding** | `docs/beta/tester-onboarding.md` | Guía para testers beta |
| **Feedback Tracker** | `docs/beta/feedback-tracker.md` | Tracking de feedback beta |
| **Bug Triage Matrix** | `docs/operations/bug-triage-matrix.md` | Matriz de triage P0-P3 |
| **Retrospective** | `docs/retrospectives/beta-v1.8-retro.md` | Retrospectiva post-beta |

---

## 5. Proceso de Actualización

### Cómo Actualizar Este Documento

1. **Identificar cambio:** Nueva fase completada, versión release, o discrepancia detectada
2. **Crear PR:** Modificar `docs/roadmap/source-of-truth.md` con cambios
3. **Requerir revisión:** Al menos 1 reviewer de @ed2kIA/core-team
4. **Validar:** Verificar que todos los enlaces funcionan (`test -f` para cada referencia)
5. **Merge → Auto-Push:** Commit message: `docs(roadmap): update source of truth — [reason]`

### Qué Actualizar

| Evento | Actualización Requerida |
|--------|------------------------|
| Nueva fase completada | Agregar fila a Tabla Maestra §2 |
| Nuevo release tag | Actualizar §3 (Version Actual) |
| Nuevo documento de referencia | Agregar a §4 (Documentos de Referencia) |
| Discrepancia detectada | Corregir + documentar en changelog |

---

## 6. Validación Automática

### Comandos de Verificación

```bash
# Verificar existencia de documentos primarios
test -f docs/roadmap/source-of-truth.md && echo "✓ Source of Truth"
test -f docs/roadmap/phase6-audit-mapping.md && echo "✓ FASE 6 Audit"
test -f docs/roadmap/versioning-alignment.md && echo "✓ Versioning Alignment"
test -f docs/roadmap/v1.9-roadmap-draft.md && echo "✓ v1.9 Roadmap"
test -f docs/roadmap/phase7-v1.9-unification.md && echo "✓ FASE 7 Unification"

# Verificar git tag actual
git describe --tags --abbrev=0

# Verificar última versión en tabla
grep -c "FASE.*Completada\|En desarrollo" docs/roadmap/source-of-truth.md
```

---

## 7. Resolución de Discrepancias

### Procedimiento

1. **Detectar:** Discrepancia entre este documento y otro artefacto
2. **Verificar:** Este documento es la fuente de verdad
3. **Corregir:** Actualizar el artefacto desactualizado para alinearse
4. **Documentar:** Registrar la discrepancia y corrección en el commit message
5. **Notificar:** Si afecta a usuarios externos, actualizar release notes

### Discrepancias Conocidas (Resueltas)

| Discrepancia | Artefacto | Resolución | FASE |
|--------------|-----------|-----------|------|
| FASE 6 marcada como "Próximo" | README.md | Cambiada a "✅ Completada en v1.7/v1.8" | FASE 65 |
| Cargo.toml version = 1.6.0-stable | Cargo.toml | Intencional — representa última stable | Documentado §3 |
| Sin mapeo FASE 6 → commits | N/A | Creado phase6-audit-mapping.md | FASE 64 |
| Sin política de versioning | N/A | Creado versioning-alignment.md | FASE 66 |
| FASE 7 sin unificación con v1.9 | N/A | Creado phase7-v1.9-unification.md | FASE 68 |

---

## 8. Sign-off

### FASE 67 Completion Checklist

- [x] `docs/roadmap/source-of-truth.md` creado con declaración de autoridad
- [x] Tabla maestra Fase ↔ Versión ↔ Estado ↔ Commits ↔ Docs completa
- [x] Proceso de actualización definido (PR + core-team review)
- [x] Enlaces a phase6-audit-mapping.md, versioning-alignment.md, v1.9-roadmap-draft.md
- [x] `DAY1_OPERATIONS_PROMPT.md` actualizado a v6.1 con referencia a source-of-truth.md
- [x] Validación: `test -f docs/roadmap/source-of-truth.md` → EXISTS
- [x] Validación: `grep -c "source-of-truth\|phase\|version\|commit"` → ≥4

### FASE 68 Completion Checklist

- [x] `docs/roadmap/phase7-v1.9-unification.md` creado con declaración FASE 7 = v1.9
- [x] Tabla maestra §2 actualizada: FASE 7+ → FASE 7 ACTIVE
- [x] FASE 7 Detail section agregado con FASE 68-71
- [x] §4 Documentos Primarios: phase7-v1.9-unification.md agregado
- [x] §6 Validación: test -f phase7-v1.9-unification.md agregado
- [x] §7 Discrepancias: FASE 7 unificación resuelta
- [x] Validación: `test -f docs/roadmap/phase7-v1.9-unification.md` → EXISTS
- [x] Validación: `findstr /c:unificación /c:FASE 7 /c:v1.9 /c:sprint phase7-v1.9-unification.md` → ≥4

### Firmas

| Rol | Nombre | Fecha |
|-----|--------|-------|
| Ingeniero en Jefe | Roberto Estuardo Celis Hernández (RECH) | 2026-05-15 |
| AI Assistant | Qwen (FASE 64-67 Execution) | 2026-05-15 |
| AI Assistant | Qwen (FASE 68 - Unificación) | 2026-05-15 |

---

## 9. Referencias Cruzadas

- [`phase6-audit-mapping.md`](phase6-audit-mapping.md) — FASE 6 reconciliation
- [`versioning-alignment.md`](versioning-alignment.md) — Versioning policy
- [`v1.9-roadmap-draft.md`](v1.9-roadmap-draft.md) — Next version planning
- [`phase7-v1.9-unification.md`](phase7-v1.9-unification.md) — FASE 7 = v1.9 unification
- [`DAY1_OPERATIONS_PROMPT.md`](../../DAY1_OPERATIONS_PROMPT.md) — Operational prompt v6.1
- [`GOVERNANCE.md`](../../GOVERNANCE.md) — Project governance
- [`CONTRIBUTING.md`](../../CONTRIBUTING.md) — Contribution guide
- [`README.md`](../../README.md) — Public documentation

---

*Este documento se mantiene como parte del protocolo de reconciliación. Actualizaciones requieren PR con revisión de @ed2kIA/core-team.*
