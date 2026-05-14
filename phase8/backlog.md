# Phase 8 Backlog - 12 User Stories Prioritizadas

> **Fecha**: 2026-05-04  
> **VersiÃ³n**: Phase 8 Planning  
> **Sprint AsignaciÃ³n**: 4 sprints Ã 4 semanas  
> **Licencia**: Apache 2.0 + Ethical Use Clause  

---

## 1. Resumen del Backlog

| # | Story | Sprint | Prioridad | T-Shirt | EstimaciÃ³n (dÃ­as) |
|---|---|---|---|---|---|
| US-01 | Marketplace API para descubrimiento de modelos | Sprint 1 | P0 | L | 3 |
| US-02 | UI Dashboard para operadores | Sprint 1 | P0 | XL | 5 |
| US-03 | Model Registry v2 con versionado semÃ¡ntico | Sprint 1 | P1 | M | 3 |
| US-04 | Monitoreo en tiempo real (WebSockets) | Sprint 1 | P2 | M | 3 |
| US-05 | Multi-model adapter (Llama + Mistral) | Sprint 2 | P0 | XL | 6 |
| US-06 | FederaciÃ³n cross-model | Sprint 2 | P0 | L | 4 |
| US-07 | Tests de escalado (100+ nodos) | Sprint 2 | P1 | L | 3 |
| US-08 | RLHF integration con FeedbackLoop | Sprint 3 | P0 | XL | 4 |
| US-09 | Governance v2 (liquid governance) | Sprint 3 | P0 | L | 4 |
| US-10 | Alignment metrics dashboard | Sprint 3 | P1 | M | 2 |
| US-11 | SLOs + alerting + rollback automatizado | Sprint 4 | P0 | XL | 5 |
| US-12 | DocumentaciÃ³n completa + launch plan | Sprint 4 | P0 | L | 4 |

---

## 2. User Stories Detalladas

### US-01: Marketplace API para Descubrimiento de Modelos

**Sprint**: 1  
**Prioridad**: P0  
**T-Shirt**: L  
**EstimaciÃ³n**: 3 dÃ­as  

**Como** operador de nodo ed2kIA,  
**Quisiera** buscar y descargar modelos SAE desde un marketplace centralizado,  
**Para que** pueda desplegar nuevos modelos sin configuraciÃ³n manual.

**Acceptance Criteria**:
- [ ] GET /api/v3/marketplace/models retorna lista de modelos disponibles
- [ ] GET /api/v3/marketplace/models/:id retorna detalles del modelo (versiÃ³n, dimensiones, compatibilidad)
- [ ] POST /api/v3/marketplace/models permite registrar nuevos modelos
- [ ] GET /api/v3/marketplace/models/:id/download inicia descarga de pesos
- [ ] BÃºsqueda por nombre, versiÃ³n y compatibilidad
- [ ] ValidaciÃ³n de checksum SHA-256 al descargar

**Dependencias tÃ©cnicas**:
- Model Registry v2 (US-03)
- Schema Registry (Phase 7)
- CDN para distribuciÃ³n de pesos

**Tests requeridos**:
- `test_marketplace_list_models`
- `test_marketplace_get_model_details`
- `test_marketplace_register_model`
- `test_marketplace_download_with_checksum`

---

### US-02: UI Dashboard para Operadores

**Sprint**: 1  
**Prioridad**: P0  
**T-Shirt**: XL  
**EstimaciÃ³n**: 5 dÃ­as  

**Como** operador de nodo ed2kIA,  
**Quisiera** un dashboard web para monitorear el estado de mi nodo y la red,  
**Para que** pueda tomar decisiones operativas sin usar la lÃ­nea de comandos.

**Acceptance Criteria**:
- [ ] PÃ¡gina de inicio con mÃ©tricas globales (nodos activos, consenso, latency)
- [ ] PÃ¡gina de modelos con catÃ¡logo de SAEs disponibles
- [ ] PÃ¡gina de nodos con estado de nodos federados
- [ ] PÃ¡gina de governance con propuestas activas
- [ ] NavegaciÃ³n responsive (desktop + tablet)
- [ ] ActualizaciÃ³n automÃ¡tica cada 30s (polling)

**Dependencias tÃ©cnicas**:
- API v2 endpoints existentes
- Marketplace API (US-01)
- Alpine.js + Tailwind CSS (stack ligero)

**Tests requeridos**:
- `test_dashboard_loads`
- `test_metrics_update`
- `test_navigation`

---

### US-03: Model Registry v2 con Versionado SemÃ¡ntico

**Sprint**: 1  
**Prioridad**: P1  
**T-Shirt**: M  
**EstimaciÃ³n**: 3 dÃ­as  

**Como** desarrollador de modelos SAE,  
**Quisiera** registrar mis modelos con versionado semÃ¡ntico y metadatos de compatibilidad,  
**Para que** los usuarios puedan encontrar la versiÃ³n correcta para su entorno.

**Acceptance Criteria**:
- [ ] Versionado semÃ¡ntico (MAJOR.MINOR.PATCH)
- [ ] Metadatos: dimensiones, dtype, modelo base, fecha, autor
- [ ] Compatibilidad backward/forward tracking
- [ ] DeprecaciÃ³n de versiones antiguas
- [ ] Ãndice de bÃºsqueda por nombre y versiÃ³n

**Dependencias tÃ©cnicas**:
- Schema Registry (Phase 7 Sprint 2)
- FeedbackStore (redb)

**Tests requeridos**:
- `test_registry_register_model`
- `test_registry_semver_validation`
- `test_registry_compatibility_check`
- `test_registry_deprecation`

---

### US-04: Monitoreo en Tiempo Real (WebSockets)

**Sprint**: 1  
**Prioridad**: P2  
**T-Shirt**: M  
**EstimaciÃ³n**: 3 dÃ­as  

**Como** operador de nodo,  
**Quisiera** ver mÃ©tricas en tiempo real en el dashboard,  
**Para que** pueda detectar problemas inmediatamente.

**Acceptance Criteria**:
- [ ] WebSocket endpoint: ws://host/api/v3/ws/metrics
- [ ] Stream de mÃ©tricas cada 5s (latency, consensus, memory, CPU)
- [ ] GrÃ¡ficas en tiempo real en el dashboard
- [ ] Reconnection automÃ¡tica con exponential backoff
- [ ] Rate limiting: mÃ¡x 100 conexiones por IP

**Dependencias tÃ©cnicas**:
- Metrics API (Phase 6)
- UI Dashboard (US-02)
- Axum WebSocket support

**Tests requeridos**:
- `test_websocket_connection`
- `test_metrics_stream`
- `test_reconnection`

---

### US-05: Multi-Model Adapter (Llama + Mistral)

**Sprint**: 2  
**Prioridad**: P0  
**T-Shirt**: XL  
**EstimaciÃ³n**: 6 dÃ­as  

**Como** usuario de ed2kIA,  
**Quisiera** usar modelos Llama y Mistral ademÃ¡s de Qwen,  
**Para que** pueda elegir el modelo que mejor se adapte a mi caso de uso.

**Acceptance Criteria**:
- [ ] Llama adapter: normaliza hidden states a Qwen-Scope format
- [ ] Mistral adapter: normaliza hidden states a Qwen-Scope format
- [ ] Generic adapter interface para futuros modelos
- [ ] ValidaciÃ³n de schema post-adaptaciÃ³n
- [ ] Performance: overhead â¤50ms por adaptaciÃ³n

**Dependencias tÃ©cnicas**:
- TensorAdapter (Phase 6)
- Schema Registry (Phase 7)
- ONNX Adapter (Phase 6)

**Tests requeridos**:
- `test_llama_adapter_normalize`
- `test_mistral_adapter_normalize`
- `test_adapter_performance`
- `test_adapter_schema_validation`

---

### US-06: FederaciÃ³n Cross-Model

**Sprint**: 2  
**Prioridad**: P0  
**T-Shirt**: L  
**EstimaciÃ³n**: 4 dÃ­as  

**Como** operador de red federada,  
**Quisiera** que nodos con diferentes modelos base puedan federarse,  
**Para que** la red sea heterogÃ©nea y mÃ¡s resiliente.

**Acceptance Criteria**:
- [ ] Model-aware routing en FederationBridge
- [ ] Compatibility negotiation durante handshake
- [ ] Adaptive aggregation (FedAvg con adaptadores)
- [ ] Cross-model trust scoring
- [ ] Fallback a same-model federation si hay incompatibilidad

**Dependencias tÃ©cnicas**:
- FederationBridge (Phase 7 Sprint 1)
- Multi-model adapter (US-05)
- DynamicTrustScorer (Phase 7 Sprint 2)

**Tests requeridos**:
- `test_cross_model_handshake`
- `test_cross_model_aggregation`
- `test_compatibility_fallback`

---

### US-07: Tests de Escalado (100+ Nodos)

**Sprint**: 2  
**Prioridad**: P1  
**T-Shirt**: L  
**EstimaciÃ³n**: 3 dÃ­as  

**Como** arquitecto de sistemas,  
**Quisiera** validar que el sistema escala a 100+ nodos,  
**Para que** pueda garantizar estabilidad en producciÃ³n a gran escala.

**Acceptance Criteria**:
- [ ] SimulaciÃ³n de 100 nodos federados
- [ ] Consensus rate â¥85% con 100 nodos
- [ ] Round latency â¤5s con 100 nodos
- [ ] Memory usage â¤2GB con 100 nodos
- [ ] Reporte de resultados con grÃ¡ficas

**Dependencias tÃ©cnicas**:
- SyncProtocol (Phase 6)
- FedAvgAggregator (Phase 6)
- Benchmark runner (ops/benchmark_runner.sh)

**Tests requeridos**:
- `test_100_node_consensus`
- `test_100_node_latency`
- `test_100_node_memory`

---

### US-08: RLHF Integration con FeedbackLoop

**Sprint**: 3  
**Prioridad**: P0  
**T-Shirt**: XL  
**EstimaciÃ³n**: 4 dÃ­as  

**Como** cientÃ­fico de IA,  
**Quisiera** integrar RLHF con el Alignment Feedback Loop,  
**Para que** el modelo mejore continuamente basado en feedback humano.

**Acceptance Criteria**:
- [ ] RLHF integration layer entre FeedbackStore y AlignmentFeedbackLoop
- [ ] Pipeline: feedback â drift â RLHF training â model update â validation
- [ ] Hot swap de modelos sin downtime
- [ ] Rollback automÃ¡tico si drift aumenta post-update
- [ ] Audit trail de todos los updates

**Dependencias tÃ©cnicas**:
- AlignmentFeedbackLoop (Phase 7 Sprint 2)
- FeedbackStore (Phase 6)
- TrainerLoop (RLHF)

**Tests requeridos**:
- `test_rlhf_pipeline`
- `test_hot_swap`
- `test_rollback_on_degradation`

---

### US-09: Governance v2 (Liquid Governance)

**Sprint**: 3  
**Prioridad**: P0  
**T-Shirt**: L  
**EstimaciÃ³n**: 4 dÃ­as  

**Como** participante de la red,  
**Quisiera** delegar mi voto a expertos de confianza con capacidad de revocaciÃ³n,  
**Para que** la governance sea mÃ¡s efectiva y participativa.

**Acceptance Criteria**:
- [ ] Weighted delegation: delegar voto con peso personalizado
- [ ] Dynamic quorum: quÃ³rum se ajusta basado en participaciÃ³n
- [ ] Continuous voting: votaciÃ³n continua (no por perÃ­odos fijos)
- [ ] RevocaciÃ³n instantÃ¡nea de delegaciÃ³n
- [ ] Audit trail de todas las delegaciones y votos

**Dependencias tÃ©cnicas**:
- Governance v1 (Phase 6)
- ReputationScorer (Phase 6)
- Staking Registry (Phase 6)

**Tests requeridos**:
- `test_weighted_delegation`
- `test_dynamic_quorum`
- `test_continuous_voting`
- `test_delegation_revocation`

---

### US-10: Alignment Metrics Dashboard

**Sprint**: 3  
**Prioridad**: P1  
**T-Shirt**: M  
**EstimaciÃ³n**: 2 dÃ­as  

**Como** cientÃ­fico de IA,  
**Quisiera** ver mÃ©tricas de alineaciÃ³n en el dashboard,  
**Para que** pueda monitorear la efectividad del feedback loop.

**Acceptance Criteria**:
- [ ] GrÃ¡fica de drift por layer (tiempo real)
- [ ] Historial de steering adjustments
- [ ] Rate de rollback
- [ ] Feedback volume por concepto
- [ ] Efectividad de RLHF (drift reduction %)

**Dependencias tÃ©cnicas**:
- UI Dashboard (US-02)
- AlignmentFeedbackLoop (Phase 7 Sprint 2)
- Metrics API

**Tests requeridos**:
- `test_alignment_metrics_display`
- `test_drift_chart`

---

### US-11: SLOs + Alerting + Rollback Automatizado

**Sprint**: 4  
**Prioridad**: P0  
**T-Shirt**: XL  
**EstimaciÃ³n**: 5 dÃ­as  

**Como** SRE,  
**Quisiera** SLOs definidos con alertas y rollback automatizado,  
**Para que** pueda mantener la disponibilidad del servicio.

**Acceptance Criteria**:
- [ ] SLOs definidos para 6 mÃ©tricas (disponibilidad, latency, consensus, throughput, error rate, memory)
- [ ] Alertas configuradas (PagerDuty/Opsgenie)
- [ ] Rollback automatizado cuando SLO se viola por >15min
- [ ] Runbook de incidentes
- [ ] Post-mortem template

**Dependencias tÃ©cnicas**:
- Metrics API
- CI/CD pipeline
- Monitoring stack (Grafana + Prometheus)

**Tests requeridos**:
- `test_slo_evaluation`
- `test_alert_firing`
- `test_automated_rollback`

---

### US-12: DocumentaciÃ³n Completa + Launch Plan

**Sprint**: 4  
**Prioridad**: P0  
**T-Shirt**: L  
**EstimaciÃ³n**: 4 dÃ­as  

**Como** nuevo usuario de ed2kIA,  
**Quisiera** documentaciÃ³n completa y guÃ­as de inicio rÃ¡pido,  
**Para que** pueda empezar a usar la plataforma sin barreras.

**Acceptance Criteria**:
- [ ] API reference documentation (OpenAPI 3.0)
- [ ] Getting started guide (5 min setup)
- [ ] Architecture documentation
- [ ] Security whitepaper
- [ ] Node operator guide
- [ ] Governance guide
- [ ] Launch plan con checklist
- [ ] Community announcement

**Dependencias tÃ©cnicas**:
- Todos los mÃ³dulos de Phase 8
- OpenAPI spec generator

**Tests requeridos**:
- `test_docs_build`
- `test_api_spec_valid`

---

## 3. Matriz de PriorizaciÃ³n

| Prioridad | Criterio | Stories |
|---|---|---|
| **P0** | CrÃ­tico para v1.0.0 STABLE | US-01, US-02, US-05, US-06, US-08, US-09, US-11, US-12 |
| **P1** | Importante pero no bloqueante | US-03, US-07, US-10 |
| **P2** | Nice-to-have | US-04 |

---

## 4. Velocidad Estimada

| Sprint | Stories | DÃ­as de trabajo | Velocidad estimada |
|---|---|---|---|
| Sprint 1 | US-01, US-02, US-03, US-04 | 14 dÃ­as | ~3.5 dÃ­as/story |
| Sprint 2 | US-05, US-06, US-07 | 13 dÃ­as | ~4.3 dÃ­as/story |
| Sprint 3 | US-08, US-09, US-10 | 10 dÃ­as | ~3.3 dÃ­as/story |
| Sprint 4 | US-11, US-12 | 9 dÃ­as | ~4.5 dÃ­as/story |

**Total**: 46 dÃ­as de trabajo / ~11.5 dÃ­as promedio por sprint (2 engineers en paralelo)

---

## 5. Contactos

| Story | Owner | Reviewer |
|---|---|---|
| US-01, US-03 | `@ed2kia/backend-team` | `@ed2kia/architect` |
| US-02, US-04, US-10 | `@ed2kia/frontend-team` | `@ed2kia/ux-team` |
| US-05, US-06, US-07 | `@ed2kia/federation-team` | `@ed2kia/perf-team` |
| US-08, US-09 | `@ed2kia/alignment-team` | `@ed2kia/governance-team` |
| US-11 | `@ed2kia/sre-team` | `@ed2kia/security-team` |
| US-12 | `@ed2kia/docs-team` | `@ed2kia/community-team` |

---

*Documento generado para Phase 8 Planning. PrÃ³xima revisiÃ³n: Sprint 1 refinement.*
