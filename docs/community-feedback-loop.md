# Community Feedback Loop — ed2kIA v1.7

**Versión:** v1.7 Sprint Active
**Fecha:** 2026-05-14
**Responsable:** Core Team + Community Maintainers

---

## Flujo de Contribución

```
Issue → Fork → Branch → Draft PR → CI/Benchmarks → Review → Merge → Contributor Spotlight
```

### 1. Issue
- Contribuyentes eligen issue de `ISSUES_BATCH_V1.7.md` o crean nueva propuesta
- Issues con etiqueta `good-first-issue` incluyen scope, acceptance criteria y RFC-001 references
- **SLA respuesta:** 48 horas para asignar mentor o solicitar clarificación

### 2. Fork & Branch
- Fork del repo `Stuartemk/ed2kIA`
- Branch naming: `<type>/<scope>-<short-desc>` (ej: `perf/quantization-simd`)
- Conventional commits: `type(scope): description`

### 3. Draft PR
- Crear Draft PR con enlace al issue (`Closes #N`)
- Incluir descripción técnica: enfoque, cambios, métricas esperadas
- **SLA review inicial:** 72 horas desde PR abierta

### 4. CI & Benchmarks
- CI pipeline (`ci.yml`) debe pasar: `cargo check` + `cargo test` + `cargo clippy`
- Benchmark workflow (`benchmarks.yml`) genera reporte automático (continue-on-error)
- Comparar métricas vs baseline `benchmarks/results/baseline-v1.7.json`
- Formato de reporte:
  ```markdown
  | Benchmark | Baseline | PR | Δ |
  |-----------|----------|----|---|
  | fp8_throughput | null | 450 MB/s | baseline |
  ```

### 5. Review
- Mínimo 2 approvers del core team
- Checklist de review:
  - [ ] Code follows Rust best practices (no unsafe)
  - [ ] Tests cubren edge cases
  - [ ] Benchmarks muestran mejora o mantienen baseline
  - [ ] Documentación actualizada
  - [ ] RFC-001 alignment verificado

### 6. Merge
- Squash merge con conventional commit message
- Actualizar CHANGELOG si aplica
- Tag de versión si es feature significativa

### 7. Contributor Spotlight
- Agregar a `CONTRIBUTORS.md` con rol y contribución
- Mención en Discord #contributor-spotlight
- Credito en release notes

---

## SLAs de Respuesta

| Etapa | SLA | Responsable |
|-------|-----|-------------|
| Issue triage | 48 horas | Maintainer on-call |
| PR review inicial | 72 horas | Core team |
| PR review follow-up | 24 horas | Reviewer asignado |
| Merge post-approval | 48 horas | Core team lead |
| Bug report (SEV-1) | 4 horas | On-call engineer |
| Bug report (SEV-2) | 24 horas | Core team |
| Feature request | 1 semana | Product lead |

---

## Criterios de Aceptación

### Code Quality
- Zero unsafe code policy
- `cargo clippy` sin warnings
- `cargo fmt` aplicado
- Tests con ≥80% coverage del módulo afectado
- Documentación en rustdoc para funciones públicas

### Benchmark Requirements
- Integrar en `benchmarks/benches/` si afecta performance
- Comparar vs baseline `baseline-v1.7.json`
- Documentar resultados en PR description

### RFC-001 Alignment
- Verificar que cambios alinean con estrategias de RFC-001:
  1. Prefetching Semántico
  2. Cuantización Agresiva (FP8/INT4)
  3. Geographic Routing
  4. Async Steering Signals

---

## Manejo de Forks

### Forks Activos
- Mantener lista de forks activos en `docs/active-forks.md`
- Sync semanal con upstream/main
- Ofrecer mentoría para forks estancados >7 días

### Forks Abandonados
- Después de 30 días sin actividad, marcar issue como `up-for-grabs`
- Preservar trabajo del fork original en branch `archive/<fork-author>-<feature>`
- Contactar al autor antes de reasignar

---

## Roles Técnicos (CONTRIBUTORS.md)

### Benchmark Maintainer
- **Responsabilidades:** Mantener benchmark suite, revisar métricas PR, actualizar baseline
- **Requisitos:** Experiencia con Criterion, Rust performance profiling
- **Permisos:** Write access a `benchmarks/`, `.github/workflows/benchmarks.yml`

### SAE Optimizer
- **Responsabilidades:** Optimizar SAE loading, cuantización, SIMD intrinsics
- **Requisitos:** Experiencia con Candle, cuantización numérica, SIMD
- **Permisos:** Write access a `src/sae/`, `src/bridge/quantization.rs`

### P2P Routing
- **Responsabilidades:** Geographic routing, libp2p integration, federation scoring
- **Requisitos:** Experiencia con libp2p, networking distribuido
- **Permisos:** Write access a `src/p2p/`, `src/bridge/federation_zkp_bridge_v*.rs`

### Docs Maintainer
- **Responsabilidades:** RFCs, migration guides, benchmark contributing guide
- **Requisitos:** Technical writing, conocimiento de arquitectura ed2kIA
- **Permisos:** Write access a `docs/`, `benchmarks/README.md`

---

## Template CONTRIBUTORS.md

```markdown
# Contribuyentes — ed2kIA

## Core Team
| Nombre | Rol | GitHub | Contribuciones |
|--------|-----|--------|----------------|
| Stuartemk | Founder/Lead | @Stuartemk | Architecture, RFC-001 |

## Benchmark Maintainers
| Nombre | GitHub | Desde |
|--------|--------|-------|
| [Tu nombre] | [@tu-github] | [Fecha] |

## SAE Optimizers
| Nombre | GitHub | Desde |
|--------|--------|-------|
| [Tu nombre] | [@tu-github] | [Fecha] |

## P2P Routing
| Nombre | GitHub | Desde |
|--------|--------|-------|
| [Tu nombre] | [@tu-github] | [Fecha] |

## Docs Maintainers
| Nombre | GitHub | Desde |
|--------|--------|-------|
| [Tu nombre] | [@tu-github] | [Fecha] |

## Contributors
| Nombre | GitHub | Contribución |
|--------|--------|--------------|
| [Tu nombre] | [@tu-github] | [Descripción breve] |
```

---

## Métricas de Salud Comunitaria

| Métrica | Target | Medición |
|---------|--------|----------|
| Time to first response | < 48h | GitHub issue timestamps |
| Time to merge (good-first) | < 7 días | PR creation → merge |
| Active contributors/mes | ≥ 5 | Commits con author único |
| Benchmark PRs con mejora | ≥ 80% | Comparación vs baseline |
| Issue resolution rate | ≥ 70%/sprint | Closed / (Open + Closed) |
