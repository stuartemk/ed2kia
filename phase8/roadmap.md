# Phase 8 Roadmap - v0.8.0-alpha → v1.0.0 STABLE

> **Fecha**: 2026-05-04  
> **VersiÃ³n**: Phase 8 Planning  
> **DuraciÃ³n**: 4 sprints Ã 4 semanas = 16 semanas  
> **Objetivo**: v1.0.0 STABLE (GA - General Availability)  
> **Licencia**: Apache 2.0 + Ethical Use Clause  

---

## 1. VisiÃ³n EstratÃ©gica

Phase 8 transforma ed2kIA de una plataforma beta a un producto STABLE listo para producciÃ³n a gran escala. Los pilares son:

1. **Marketplace de Modelos**: CatÃ¡logo de SAEs compatibles con descubrimiento y descarga automatizada.
2. **UI Operacional**: Dashboard web para monitoreo, gestiÃ³n de nodos y governance.
3. **Escalado Cross-Model**: Soporte para mÃºltiples modelos base (Qwen, Llama, Mistral) con adaptaciÃ³n automÃ¡tica.
4. **AlineaciÃ³n Continua**: Feedback loop integrado con RLHF para mejora continua del modelo.
5. **Governanza LÃ­quida**: DelegaciÃ³n ponderada, quÃ³rum dinÃ¡mico y votaciÃ³n continua.
6. **Estabilidad ProducciÃ³n**: SLOs definidos, monitoring completo, rollback automatizado.

---

## 2. Timeline General

```
Sprint 1 (Semanas 1-4)    Sprint 2 (Semanas 5-8)    Sprint 3 (Semanas 9-12)   Sprint 4 (Semanas 13-16)
âââ Marketplace + UI      âââ Cross-Model Scaling  âââ Continuous Alignment  âââ STABLE Preparation
âââ v0.8.0-alpha          âââ v0.9.0-rc            âââ v0.9.5-rc             âââ v1.0.0 STABLE
```

| Sprint | Semanas | Milestone | Entregables Clave |
|---|---|---|---|
| **Sprint 1** | 1-4 | v0.8.0-alpha | Marketplace API, UI dashboard bÃ¡sico, model registry v2 |
| **Sprint 2** | 5-8 | v0.9.0-rc | Cross-model adapter, multi-model federation, scaling tests |
| **Sprint 3** | 9-12 | v0.9.5-rc | Continuous alignment, RLHF integration, governance v2 |
| **Sprint 4** | 13-16 | v1.0.0 STABLE | SLOs, monitoring completo, docs, launch |

---

## 3. Sprint 1: Marketplace + UI (Semanas 1-4)

### 3.1 Objetivo

Crear el Marketplace de Modelos y el Dashboard Operacional para que los operadores puedan descubrir, descargar y gestionar SAEs de forma autÃ³noma.

### 3.2 Entregables

| ID | Entregable | Prioridad | EstimaciÃ³n | Dependencias |
|---|---|---|---|---|
| S1.1 | Marketplace API (CRUD modelos) | P0 | 3 dÃ­as | Model Registry v2 |
| S1.2 | Model Registry v2 (versionado + compatibilidad) | P0 | 3 dÃ­as | Schema Registry |
| S1.3 | UI Dashboard (React/Alpine.js) | P1 | 5 dÃ­as | API v2 |
| S1.4 | UI - Vista de nodos federados | P1 | 2 dÃ­as | Federation API |
| S1.5 | UI - Vista de governance | P1 | 2 dÃ­as | Governance API |
| S1.6 | UI - Monitoreo en tiempo real (WebSockets) | P2 | 3 dÃ­as | Metrics API |
| S1.7 | Tests E2E Marketplace | P0 | 2 dÃ­as | S1.1, S1.2 |
| S1.8 | Docs - Marketplace API reference | P1 | 1 dÃ­a | S1.1 |

### 3.3 Arquitectura

```
Marketplace API
âââ GET  /api/v3/marketplace/models          - Listar modelos
âââ GET  /api/v3/marketplace/models/:id      - Detalle de modelo
âââ POST /api/v3/marketplace/models          - Registrar modelo
âââ PUT  /api/v3/marketplace/models/:id      - Actualizar modelo
âââ DELETE /api/v3/marketplace/models/:id    - Despublicar modelo
âââ GET  /api/v3/marketplace/models/:id/download - Descargar pesos

UI Dashboard
ââââ Home: MÃ©tricas globales (nodos, consenso, latency)
âââ Models: CatÃ¡logo de SAEs disponibles
ââââ Nodes: Estado de nodos federados
ââââ Governance: Propuestas activas + votaciÃ³n
ââââ Settings: ConfiguraciÃ³n del nodo local
```

### 3.4 Criterios de AceptaciÃ³n

- [ ] Marketplace API con 5 endpoints funcionales
- [ ] UI dashboard accesible en `/dashboard`
- [ ] 10+ modelos en catÃ¡logo inicial
- [ ] Tests E2E passing (100%)
- [ ] DocumentaciÃ³n API completa

### 3.5 Milestone: v0.8.0-alpha

| Criterio | Umbral |
|---|---|
| Marketplace API endpoints | â¥5 |
| Modelos en catÃ¡logo | â¥10 |
| UI pÃ¡ginas funcionales | â¥4 |
| Tests E2E passing | 100% |
| API docs completas | â¥90% coverage |

---

## 4. Sprint 2: Cross-Model Scaling (Semanas 5-8)

### 4.1 Objetivo

Habilitar soporte para mÃºltiples modelos base (Qwen, Llama, Mistral) con adaptaciÃ³n automÃ¡tica y federaciÃ³n cross-model.

### 4.2 Entregables

| ID | Entregable | Prioridad | EstimaciÃ³n | Dependencias |
|---|---|---|---|---|
| S2.1 | Multi-model adapter framework | P0 | 4 dÃ­as | TensorAdapter v2 |
| S2.2 | Llama adapter implementation | P0 | 3 dÃ­as | S2.1 |
| S2.3 | Mistral adapter implementation | P0 | 3 dÃ­as | S2.1 |
| S2.4 | Cross-model federation protocol | P0 | 4 dÃ­as | Federation Bridge |
| S2.5 | Model compatibility matrix | P1 | 2 dÃ­as | Schema Registry |
| S2.6 | Scaling tests (100+ nodos) | P0 | 3 dÃ­as | S2.4 |
| S2.7 | Performance benchmarks cross-model | P1 | 2 dÃ­as | S2.2, S2.3 |
| S2.8 | Docs - Cross-model guide | P1 | 1 dÃ­a | S2.1-S2.5 |

### 4.3 Arquitectura

```
Multi-Model Adapter
âââ Qwen Adapter (existente)
âââ Llama Adapter (nuevo)
âââ Mistral Adapter (nuevo)
âââ Generic Adapter Interface
    âââ normalize() â NormalizedHiddenState
    âââ adapt() â TargetModelFormat
    âââ validate() â SchemaResult

Cross-Model Federation
âââ Model-aware routing
âââ Compatibility negotiation
âââ Adaptive aggregation
âââ Cross-model trust scoring
```

### 4.4 Criterios de AceptaciÃ³n

- [ ] 3+ modelos soportados (Qwen, Llama, Mistral)
- [ ] FederaciÃ³n cross-model funcional
- [ ] Tests de escalado con 100+ nodos
- [ ] Benchmarks cross-model dentro de umbrales
- [ ] DocumentaciÃ³n completa

### 4.5 Milestone: v0.9.0-rc

| Criterio | Umbral |
|---|---|
| Modelos soportados | â¥3 |
| FederaciÃ³n cross-model | Funcional |
| Nodos en scaling test | â¥100 |
| Consensus rate cross-model | â¥85% |
| Adapter latency overhead | â¤50ms |

---

## 5. Sprint 3: Continuous Alignment (Semanas 9-12)

### 5.1 Objetivo

Integrar RLHF con el Alignment Feedback Loop para mejora continua del modelo basada en feedback humano.

### 5.2 Entregables

| ID | Entregable | Prioridad | EstimaciÃ³n | Dependencias |
|---|---|---|---|---|
| S3.1 | RLHF integration layer | P0 | 4 dÃ­as | FeedbackLoop, TrainerLoop |
| S3.2 | Continuous alignment pipeline | P0 | 3 dÃ­as | S3.1 |
| S3.3 | Governance v2 (liquid governance) | P0 | 4 dÃ­as | Governance v1 |
| S3.4 | Weighted delegation | P1 | 3 dÃ­as | S3.3 |
| S3.5 | Dynamic quorum | P1 | 2 dÃ­as | S3.3 |
| S3.6 | Alignment metrics dashboard | P1 | 2 dÃ­as | UI Dashboard |
| S3.7 | Tests E2E continuous alignment | P0 | 2 dÃ­as | S3.1, S3.2 |
| S3.8 | Docs - Alignment guide | P1 | 1 dÃ­a | S3.1-S3.3 |

### 5.3 Arquitectura

```
Continuous Alignment Pipeline
ââââ Human Feedback (CLI/UI/API)
ââââ FeedbackStore (redb)
ââââ AlignmentFeedbackLoop
ââââ RLHF Trainer
ââââ Model Update (hot swap)
ââââ Validation + Rollback

Governance v2 (Liquid)
ââââ Weighted Delegation
ââââ Dynamic Quorum
ââââ Continuous Voting
ââââ Proposal Automation
```

### 5.4 Criterios de AceptaciÃ³n

- [ ] RLHF integrado con FeedbackLoop
- [ ] Pipeline de alineaciÃ³n continua funcional
- [ ] Governance v2 con delegaciÃ³n ponderada
- [ ] QuÃ³rum dinÃ¡mico implementado
- [ ] Tests E2E passing

### 5.5 Milestone: v0.9.5-rc

| Criterio | Umbral |
|---|---|
| RLHF integration | Funcional |
| Alignment pipeline | End-to-end |
| Governance v2 | DelegaciÃ³n + quÃ³rum |
| Drift reduction (post-RLHF) | â¥20% improvement |
| Governance participation | â¥60% de nodos |

---

## 6. Sprint 4: STABLE Preparation (Semanas 13-16)

### 6.1 Objetivo

Preparar la versiÃ³n v1.0.0 STABLE con SLOs definidos, monitoring completo, documentaciÃ³n y plan de lanzamiento.

### 6.2 Entregables

| ID | Entregable | Prioridad | EstimaciÃ³n | Dependencias |
|---|---|---|---|---|
| S4.1 | SLO definition + monitoring | P0 | 3 dÃ­as | Metrics API |
| S4.2 | Alerting rules (PagerDuty/Opsgenie) | P0 | 2 dÃ­as | S4.1 |
| S4.3 | Automated rollback | P0 | 3 dÃ­as | CI/CD pipeline |
| S4.4 | Security audit (external) | P0 | 5 dÃ­as | Codebase completo |
| S4.5 | Performance audit + optimization | P1 | 3 dÃ­as | Benchmarks |
| S4.6 | Complete documentation | P0 | 4 dÃ­as | Todos los mÃ³dulos |
| S4.7 | Launch plan + runbook | P0 | 2 dÃ­as | S4.1-S4.6 |
| S4.8 | Community announcement | P1 | 1 dÃ­a | S4.7 |

### 6.3 SLOs Definidos

| MÃ©trica | SLO | Umbral Alerta | Umbral CrÃ­tico |
|---|---|---|---|
| Disponibilidad | â¥99.9% | <99.5% | <99% |
| SAE Latency p50 | â¤350ms | >400ms | >500ms |
| Consensus Rate | â¥88% | <85% | <80% |
| API v2 Throughput | â¥500 req/s | <400 req/s | <300 req/s |
| Error Rate | â¤0.1% | >0.5% | >1% |
| Memory Usage | â¤180MB | >220MB | >250MB |

### 6.4 Criterios de AceptaciÃ³n

- [ ] SLOs definidos y monitoreados
- [ ] Alertas configuradas
- [ ] Rollback automatizado probado
- [ ] AuditorÃ­a de seguridad externa completada
- [ ] DocumentaciÃ³n completa
- [ ] Plan de lanzamiento aprobado

### 6.5 Milestone: v1.0.0 STABLE

| Criterio | Umbral |
|---|---|
| SLOs definidos | â¥6 mÃ©tricas |
| AuditorÃ­a externa | 0 hallazgos P0/P1 |
| DocumentaciÃ³n | â¥95% coverage |
| Tests passing | 100% |
| Benchmarks | Dentro de todos los umbrales |
| Launch plan | Aprobado |

---

## 7. Riesgos y MitigaciÃ³n

| Riesgo | Probabilidad | Impacto | MitigaciÃ³n |
|---|---|---|---|
| Delay en Marketplace | Media | Alto | MVP con API solo, UI en sprint 2 |
| Cross-model incompatibilidad | Media | Alto | Fallback a Qwen-only, adaptadores incrementales |
| RLHF convergence issues | Baja | CrÃ­tico | Neutral alignment como fallback |
| Security audit findings | Media | CrÃ­tico | Buffer de 2 semanas para remediaciÃ³n |
| Community adoption lenta | Media | Medio | Early access program, docs en mÃºltiples idiomas |

---

## 8. Recursos Requeridos

| Rol | Cantidad | Sprint Asignado |
|---|---|---|
| Rust Backend Engineer | 2 | Todos |
| Frontend Engineer | 1 | Sprint 1, 3 |
| Security Engineer | 1 | Sprint 4 |
| DevOps Engineer | 1 | Todos |
| Technical Writer | 1 | Sprint 2, 4 |
| QA Engineer | 1 | Todos |

---

## 9. Dependencias Externas

| Dependencia | Sprint | PropÃ³sito |
|---|---|---|
| AuditorÃ­a de seguridad externa | Sprint 4 | ValidaciÃ³n independiente |
| Hosting para Marketplace | Sprint 1 | CatÃ¡logo pÃºblico |
| Monitoring service (Grafana/PagerDuty) | Sprint 4 | SLO monitoring |
| CDN para modelos | Sprint 1 | DistribuciÃ³n de pesos |

---

## 10. Exit Criteria para Phase 8

Phase 8 se considera completada cuando:

- [ ] v1.0.0 STABLE publicado
- [ ] Todos los SLOs cumplidos por 7 dÃ­as consecutivos
- [ ] 0 vulnerabilidades crÃ­ticas/altas
- [ ] DocumentaciÃ³n completa y revisada
- [ ] Community launch exitoso
- [ ] Post-launch review completado

---

*Documento generado para Phase 8 Planning. PrÃ³xima revisiÃ³n: Sprint 1 kickoff.*
