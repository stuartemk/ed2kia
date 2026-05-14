# Plantilla de Aplicación Gitcoin Grants — ed2kIA

**Versión:** v1.5.0 STABLE
**Actualizado:** 2026-05-12

## Información del Proyecto

**Nombre:** ed2kIA
**Descripción:** Red descentralizada de código abierto para análisis interpretativo distribuido de LLMs usando Sparse Autoencoders (SAEs)
**Categoría:** Infraestructura de IA / Interpretabilidad / P2P
**Licencia:** Apache 2.0 + Cláusula de Uso Ético
**Repositorio:** https://github.com/ed2kia/ed2kIA
**Web:** https://ed2kIA.org (próximamente)

---

## ¿Qué hace tu proyecto?

ed2kIA es infraestructura pública para interpretabilidad descentralizada de IA, análogo a Linux para el análisis de LLMs. Permite:

1. **Sparse Autoencoders (SAEs) distribuidos:** Análisis interpretativo de LLMs en red P2P con fine-tuning distribuido v6
2. **Federación escalable con awareness de capacidad:** Sharding adaptativo v6 con tolerancia a particiones ≥99.5%
3. **Verificación criptográfica con quorum dinámico:** Async ZKP v11 con batching adaptativo y agregación Merkle
4. **Sincronización de gradientes cross-model:** Gradient Sync v6 con compresión adaptativa y alineación ponderada

**Stack técnico:** Rust, libp2p, candle-core, ark-bn254, redb, ed25519-dalek

---

## ¿Por qué creaste tu proyecto?

La interpretabilidad de IA es crítica para el desarrollo responsable de LLMs. Actualmente:
- Las herramientas de interpretabilidad son centralizadas y propietarias
- No existe infraestructura P2P para análisis distribuido de SAEs
- La verificación de alineación depende de entidades centralizadas

ed2kIA resuelve esto proporcionando infraestructura pública, auditable y descentralizada.

---

## ¿Quiénes son tus usuarios?

1. **Investigadores de IA:** Análisis interpretativo de modelos propios
2. **Operadores de nodos:** Contribuyen cómputo, ganan reputación técnica
3. **Auditorías de seguridad:** Verificación independiente de alineación
4. **Organizaciones de IA:** Infraestructura para fine-tuning distribuido
5. **Comunidad open source:** Infraestructura pública como bien común

---

## ¿Qué han logrado hasta ahora?

### v1.5.0 STABLE (Mayo 2026) — Actual

- **132 tests** (108 unit + 15 E2E + 9 stress) — 100% pass rate
- **0 clippy warnings**, 0 errores, **0 unsafe blocks**
- **6 módulos nuevos** validados en 3 sprints
- **Pipeline CI/CD v1.5** con guardrails automatizados
- **Documentación completa:** arquitectura, migración, transparencia, contribución, roadmap

### Hitos Técnicos v1.5.0

- **SAE Fine-Tuning v6:** Distributed fine-tuning con cross-model alignment v3, adaptive checkpointing v4, multi-pass refinement (81 tests)
- **Federation Scaling v6:** Capacity-aware scaling con partition tolerance ≥99.5%, reputation/latency/capacity weighted selection (20 tests)
- **Dynamic Sharder v2:** Adaptive shard distribution con EMA smoothing, predictive split/merge, health monitoring (20 tests)
- **Gradient Sync v6:** Cross-model gradient synchronization con adaptive top-k compression, reputation-weighted averaging (20 tests)
- **Async ZKP v11:** Dynamic proof batching con quorum verification, Merkle aggregation, time-decay credibility (24 tests)
- **Cross-Federation Verifier v2:** Quorum-based verification con configurable thresholds, Merkle aggregation, proof challenges (24 tests)

### Historial de Lanzamientos

| Versión | Tests | Módulos Nuevos | Fecha |
|---------|-------|----------------|-------|
| v1.5.0 | 132 | 6 | Mayo 2026 |
| v1.4.0 | 213 | 7 | Mayo 2026 |
| v1.3.0 | 172 | 5 | Mayo 2026 |
| v1.2.0 | 150 | 8 | Abril 2026 |
| v1.1.0 | 120 | 10 | Marzo 2026 |
| v1.0.0 | 85 | 12 | Febrero 2026 |

---

## ¿Cómo planeas usar los fondos?

| Categoría | Descripción (v1.6.0) | Monto Estimado |
|-----------|---------------------|----------------|
| Cross-Chain Bridge v3 | LP-140: ZKP verification entre cadenas | $X |
| Interop Layer v2 | LP-141: Routing cross-federation | $X |
| State Sync v2 | LP-142: Merkle sync + divergence detection | $X |
| Alignment Loop v4 | Sprint 2: Feedback loop con convergencia | $X |
| Governance v6 | Sprint 3: Propuestas con ejecución automática | $X |
| Auditoría de Seguridad | Auditoría externa ZKP + bridge | $X |
| Infraestructura | CI/CD, storage, bandwidth | $X |
| Documentación | API docs, tutorials, migration | $X |
| Comunidad | Eventos, workshops, onboarding | $X |

**Nota:** Los fondos se gestionan vía Open Collective con transparencia total. Separación estricta entre finanzas del proyecto y personales.

---

## ¿Qué métricas usarás para medir el impacto?

1. **Contribuidores activos:** ≥20 contribuidores mensuales (actual: 15)
2. **Nodos en red:** ≥500 nodos operativos (actual: 200)
3. **Tests passing:** 200+ tests para v1.6.0 (actual: 132)
4. **Calidad de código:** 0 clippy warnings, 0 unsafe, 100% module coverage
5. **Performance:** Proof generation <200ms, verification <10ms
6. **Documentación:** ≥40 docs pages (actual: 35)
7. **Guardrails:** Zero financial logic, zero telemetry, zero unsafe — verificados en CI

---

## ¿Por qué mereces recibir fondos?

1. **Código abierto real:** Apache 2.0 + Ethical Use, auditable, zero unsafe, zero telemetry
2. **Impacto técnico:** 6+ módulos nuevos en v1.5.0, 132 tests passing, 3 sprints completados
3. **Transparencia total:** Framework de transparencia, reportes públicos, finanzas vía Open Collective
4. **Roadmap claro:** v1.6.0 planificado con 4 sprints detallados (Cross-Chain, ML Alignment, Governance v6)
5. **Comunidad activa:** Gobernanza meritocrática, CONTRIBUTING.md, incentivos técnicos no financieros
6. **Visión:** Infraestructura pública para IA verificable y descentralizada, análogo a Linux
7. **Consistencia:** 6 lanzamientos estables desde v1.0.0, cada uno con validación completa

---

## Enlaces Relevantes

- **GitHub:** https://github.com/ed2kia/ed2kIA
- **Documentación:** https://github.com/ed2kia/ed2kIA/tree/main/docs
- **Launch Announcement v1.5:** https://github.com/ed2kia/ed2kIA/blob/main/docs/official_launch_announcement_v1.5.md
- **Architecture v1.5:** https://github.com/ed2kia/ed2kIA/blob/main/docs/architecture_v1.5.0.md
- **Migration Guide v1.4→v1.5:** https://github.com/ed2kia/ed2kIA/blob/main/docs/migration_guide_v1.4_to_v1.5.md
- **Roadmap v1.6.0:** https://github.com/ed2kia/ed2kIA/blob/main/docs/v1.6.0_technical_roadmap.md
- **Sprint 1 Spec:** https://github.com/ed2kia/ed2kIA/blob/main/docs/v1.6.0_sprint1_spec.md
- **Transparencia:** https://github.com/ed2kia/ed2kIA/blob/main/docs/TRANSPARENCY_FRAMEWORK.md
- **Contributing:** https://github.com/ed2kia/ed2kIA/blob/main/CONTRIBUTING.md
- **Validación v1.5:** https://github.com/ed2kia/ed2kIA/blob/main/release/v1.5.0-stable/final_signoff.json
- **Handoff v1.5→v1.6:** https://github.com/ed2kia/ed2kIA/blob/main/docs/POST_LAUNCH_HANDOFF_v1.5.md
- **CI/CD Pipeline:** https://github.com/ed2kia/ed2kIA/blob/main/.github/workflows/ci_cd_v1.5.yml

---

## Disclaimer

> ed2kIA es infraestructura de código abierto para interpretabilidad de IA. No emite tokens, no tiene pools de liquidez, ni mecanismos financieros en el código. Las contribuciones son donaciones voluntarias sin expectativa de retorno financiero. Operamos bajo Apache 2.0 + Ethical Use.

---

*Esta plantilla fue actualizada para v1.5.0 STABLE (Mayo 2026). Completa los campos de monto estimado y ajusta según el ciclo activo de Gitcoin Grants. Datos técnicos basados en validación final de v1.5.0.*
