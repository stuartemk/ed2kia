# FASE 7 ↔ v1.9 Unificación Estratégica

> **Declaración Oficial:** FASE 7 es el marco estratégico del Roadmap v1.9. Roadmap y FASE son el mismo ciclo de desarrollo. No existen estructuras paralelas.
>
> **Generado:** 2026-05-15T22:59:00Z
> **FASE:** 68 — Unificación Estratégica FASE 7 ↔ v1.9
> **Estado:** ACTIVE

---

## 1. Declaración de Unificación

**FASE 7 = v1.9 Roadmap.** Esta unificación elimina la duplicidad entre:
- **FASE 7** (hitos estratégicos, milestones, gobernanza)
- **v1.9 Roadmap** (sprints técnicos, features, deliverables)

Ambos son el mismo ciclo visto desde dos perspectivas:
- **Estratégica (FASE 7):** ¿Qué logramos? → Production Ready, Community Scale, Funding Sustainability
- **Táctica (v1.9):** ¿Cómo lo hacemos? → Sprint 1 (Hardening), Sprint 2 (DX), Sprint 3 (Network Growth)

### Regla de Actualización

Solo dos documentos son referencia válida para FASE 7 / v1.9:
1. [`source-of-truth.md`](source-of-truth.md) — Estado oficial del proyecto
2. Este documento (`phase7-v1.9-unification.md`) — Mapeo FASE ↔ Sprint

Cualquier otro documento que haga claims sobre FASE 7 o v1.9 debe referenciar estos dos como fuente.

---

## 2. Mapeo: FASE 7 Hitos ↔ v1.9 Sprints ↔ Features

| Hito FASE 7 | Sprint v1.9 | Features Clave | Estado |
|-------------|-------------|----------------|--------|
| **H1: Production Hardening** | Sprint 1 | Fix P0/P1, test failures, clippy warnings, WASM CI, mobile GUI foundation, ZKP optimization | 🔄 En ejecución |
| **H2: Developer Experience** | Sprint 2 | Docker Compose multi-node, API docs, migration guide, quick start, CLI scaffolding | ⏳ Planificado |
| **H3: Network Growth** | Sprint 3 | Seed node expansion, geographic routing v2, mobile bridge production, onboarding automation | ⏳ Planificado |
| **H4: Performance** | Sprint 3 | SIMD SAE forward, tensor quantization benchmark, geographic routing benchmark | ⏳ Planificado |
| **H5: Funding & Sustainability** | Sprint 1-3 | Grant follow-up (NSF/Gitcoin/OSSF), Open Collective, GitHub Sponsors | 🔄 Continuo |

---

## 3. Sprint 1 Detail — Production Hardening & Mobile GUI Foundation

### Scope Técnico

| Módulo | Archivo | Descripción | Tests |
|--------|---------|-------------|-------|
| **Mobile GUI Foundation** | `src/gui/mobile_foundation.rs` | Tauri/React Native bridge (mock + WASM target), ResourceSliderConfig, node state, thermal/battery limits | Config serialization, limit validation |
| **Async Steering Hardening** | `src/protocol/async_steering.rs` | Timeout hardening, exponential backoff retry, P95/P99 latency metrics | Timeout simulation, backoff sequence |
| **ZKP Circuit Optimization** | `src/zkp/circuit_optimization.rs` | Constraint pooling, Pedersen precomputation, benchmark hooks | Constraint count, precomputation validity |

### Feature Gate

```toml
# Cargo.toml
"v1.9-sprint1" = []
```

### Criterios de Aceptación

- [x] `src/gui/mobile_foundation.rs` creado con tests
- [x] `src/protocol/async_steering.rs` hardening aplicado
- [x] `src/zkp/circuit_optimization.rs` creado con benchmark hooks
- [x] `cargo test --features v1.9-sprint1` → PASS
- [x] `cargo clippy --features v1.9-sprint1` → 0 warnings
- [x] Métricas documentadas en `phase7-tracking.md`

---

## 4. Gobernanza de la Unificación

### Decisiones

| Decisión | Rationale | Impacto |
|----------|-----------|---------|
| FASE 7 = v1.9 | Elimina duplicidad, reduce confusión | Un solo ciclo de tracking |
| Source of Truth como referencia | Unifica versionado y fases | Discrepancias resueltas |
| Sprint-based execution | Iteración rápida, feedback continuo | Releases más frecuentes |

### Roles

| Rol | Responsabilidad | Referencia |
|-----|-----------------|------------|
| **Orquestador** | Validación final, decisiones de release | `DAY1_OPERATIONS_PROMPT.md` v7.0 |
| **Core Team** | Code review, merge approval | `GOVERNANCE.md` |
| **Community** | Feedback, testing, issues | `docs/beta/tester-onboarding.md` |

---

## 5. Timeline

| Fecha | Evento | FASE |
|-------|--------|------|
| 2026-05-15 | FASE 68: Unificación estratégica | FASE 68 |
| 2026-05-15 | FASE 69: Sprint 1 ejecución | FASE 69 |
| 2026-05-15 | FASE 70: Tracking unificado & Dashboard v3 | FASE 70 |
| 2026-05-15 | FASE 71: Prompt v7.0 & Handover | FASE 71 |
| 2026-Q3 | v1.9.0-stable target | FASE 7 Complete |

---

## 6. Referencias

- [`source-of-truth.md`](source-of-truth.md) — Fuente de verdad oficial
- [`v1.9-roadmap-draft.md`](v1.9-roadmap-draft.md) — Roadmap técnico v1.9
- [`phase6-audit-mapping.md`](phase6-audit-mapping.md) — FASE 6 reconciliación
- [`versioning-alignment.md`](versioning-alignment.md) — Política de versionado
- [`DAY1_OPERATIONS_PROMPT.md`](../../DAY1_OPERATIONS_PROMPT.md) — Prompt operacional v7.0

---

*Este documento se mantiene como parte de la unificación FASE 7 ↔ v1.9. Actualizaciones requieren PR con revisión de @ed2kIA/core-team.*
