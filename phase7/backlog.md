# Phase 7 Backlog: Historias Priorizadas

> **Formato**: User Stories con criterios de aceptación, dependencias y estimaciones.
> **Prioridad**: P0 (crítico) → P3 (nice-to-have)
> **Estimaciones**: T-shirt sizes (XS=<2d, S=<4d, M=<8d, L=<13d, XL=<20d)

---

## Sprint 7.1: Alineación Continua

### F7-B01: Motor de Alineación Continua [P0]
**Como** operador de nodo ed2kIA, **quiero** que el sistema detecte y corrija desviaciones éticas en tiempo real, **para** que la IA mantenga alineación con valores humanos sin intervención manual.

**Criterios de Aceptación:**
- [ ] `ContinuousAlignmentEngine` procesa feedback en <100ms
- [ ] Detecta desviaciones en: toxicidad, sesgo, alucinación, contradicción
- [ ] Genera `SteeringSignal` automáticamente cuando se detecta desviación
- [ ] Integra con `bridge/consciousness.rs` para inyección de contexto
- [ ] Expone métricas en `/api/v2/alignment/status`

**Dependencias:** `bridge/consciousness.rs`, `interpret/feature_analyzer.rs`
**Estimación:** L (10 días)
**Sprint:** 7.1 (W1-W2)

---

### F7-B02: Value Guard - Detección de Desviaciones [P0]
**Como** auditor de seguridad, **quiero** un sistema que monitoree continuamente las activaciones SAE para detectar patrones de desalineación, **para** prevenir comportamientos no deseados antes de que se manifiesten.

**Criterios de Aceptación:**
- [ ] `ValueGuard` analiza activaciones SAE en cada inference cycle
- [ ] Precisión de detección ≥95% (validado con dataset de prueba)
- [ ] False positive rate <2%
- [ ] Configurable thresholds por tipo de desviación
- [ ] Alertas en tiempo real vía metrics endpoint

**Dependencias:** F7-B01, `sae/loader.rs`
**Estimación:** M (6 días)
**Sprint:** 7.1 (W2-W3)

---

### F7-B03: Feedback Loop Integrado [P1]
**Como** humano en el loop, **quiero** que mis correcciones se integren automáticamente en el ciclo de inferencia, **para** que la IA aprenda de mis correcciones en tiempo real.

**Criterios de Aceptación:**
- [ ] `human/feedback_cli.rs` envía feedback directamente al alignment engine
- [ ] Feedback procesado y aplicado en <500ms
- [ ] Historial de feedback persistente (redb)
- [ ] Métricas de efectividad del feedback (antes/después)

**Dependencias:** F7-B01, `human/feedback_cli.rs`, `rlhf/feedback_store.rs`
**Estimación:** M (5 días)
**Sprint:** 7.1 (W3-W4)

---

### F7-B04: Métricas de Alineación en API v2 [P2]
**Como** desarrollador, **quiero** endpoints API para consultar el estado de alineación de cada nodo, **para** monitorear la salud ética de la red.

**Criterios de Aceptación:**
- [ ] `GET /api/v2/alignment/status` — Estado general de alineación
- [ ] `GET /api/v2/alignment/history` — Historial de desviaciones y correcciones
- [ ] `POST /api/v2/alignment/threshold` — Configurar umbrales (admin only)
- [ ] Documentación OpenAPI actualizada

**Dependencias:** F7-B01, `api/routes.rs`
**Estimación:** S (3 días)
**Sprint:** 7.1 (W4)

---

## Sprint 7.2: Federación Cross-Red

### F7-B05: Gateway de Federación Cross-Red [P0]
**Como** operador de red, **quiero** un gateway que permita sincronizar modelos con redes externas, **para** que ed2kIA pueda participar en ecosistemas de IA más amplios.

**Criterios de Aceptación:**
- [ ] `CrossNetGateway` con interfaz genérica para adaptadores
- [ ] Soporte para ≥2 adaptadores (IPFS, HuggingFace)
- [ ] Rate limiting configurable por red externa
- [ ] Quota management (bandwidth, storage, requests)
- [ ] Fallback a modo local si red externa no disponible

**Dependencias:** `federation/sync_protocol.rs`, `p2p/swarm.rs`
**Estimación:** XL (18 días)
**Sprint:** 7.2 (W5-W6)

---

### F7-B06: Adaptador IPFS [P1]
**Como** operador de nodo, **quiero** sincronizar modelos y pesos vía IPFS, **para** tener un almacenamiento descentralizado y resistente a censura.

**Criterios de Aceptación:**
- [ ] Upload/download de modelos SAE a/from IPFS
- [ ] Hash verification para integridad de datos
- [ ] Pinning automático de modelos activos
- [ ] Rate limiting para respetar límites de gateway IPFS

**Dependencias:** F7-B05
**Estimación:** M (7 días)
**Sprint:** 7.2 (W6-W7)

---

### F7-B07: Puente HuggingFace [P1]
**Como** investigador, **quiero** importar/exportar modelos desde/hacia HuggingFace Hub, **para** facilitar la adopción y colaboración con la comunidad ML.

**Criterios de Aceptación:**
- [ ] Download de modelos SAE desde HuggingFace Hub
- [ ] Upload de modelos federados a HuggingFace Hub
- [ ] Conversión automática de formatos (safetensors ↔ ed2kIA)
- [ ] Autenticación vía HF token

**Dependencias:** F7-B05, `ecosystem/hf_sync.rs`
**Estimación:** M (6 días)
**Sprint:** 7.2 (W7-W8)

---

### F7-B08: Validación Cross-Net [P2]
**Como** auditor, **quiero** que los modelos importados de redes externas sean validados antes de integrarse, **para** prevenir la introducción de modelos maliciosos o corruptos.

**Criterios de Aceptación:**
- [ ] Schema validation para modelos importados
- [ ] ZKP verification si disponible
- [ ] Reputation check del origen
- [ ] Quarantine para modelos sospechosos

**Dependencias:** F7-B05, `zkp/verifier.rs`, `security/memory_guard.rs`
**Estimación:** S (4 días)
**Sprint:** 7.2 (W8)

---

## Sprint 7.3: UI Avanzada + Marketplace

### F7-B09: Dashboard Web en Tiempo Real [P0]
**Como** operador de nodo, **quiero** un dashboard web que muestre métricas en tiempo real, **para** monitorear la salud de mi nodo y la red sin herramientas externas.

**Criterios de Aceptación:**
- [ ] Dashboard con: CPU/GPU, memoria, consenso, federación, staking
- [ ] Actualización en tiempo real vía WebSockets
- [ ] Gráficos históricos (24h, 7d, 30d)
- [ ] Responsive design (desktop + mobile)
- [ ] Dark/light theme

**Dependencias:** `monitoring/metrics.rs`, `web/server.rs`
**Estimación:** XL (15 días)
**Sprint:** 7.3 (W9-W10)

---

### F7-B10: Ledger de Recursos del Marketplace [P0]
**Como** proveedor de recursos, **quiero** listar mis recursos computacionales disponibles para leasing, **para** generar ingresos de mi infraestructura ociosa.

**Criterios de Aceptación:**
- [ ] `ResourceLedger` con CRUD de listings
- [ ] Tipos de recurso: GPU time, CPU time, storage, bandwidth
- [ ] Colateral vía staking registry
- [ ] Smart contract para escrow de pagos

**Dependencias:** `staking/registry.rs`, `reputation/ledger.rs`
**Estimación:** L (12 días)
**Sprint:** 7.3 (W10-W11)

---

### F7-B11: Motor de Precios Dinámico [P1]
**Como** el sistema, **quiero** calcular precios basados en demanda, reputación y disponibilidad, **para** equilibrar el marketplace de recursos.

**Criterios de Aceptación:**
- [ ] Precio base ajustado por demanda (supply/demand ratio)
- [ ] Descuento por reputación alta del proveedor
- [ ] Premium por recursos escasos (GPU específica)
- [ ] Circuit breakers para prevenir manipulación

**Dependencias:** F7-B10, `reputation/scoring.rs`
**Estimación:** M (6 días)
**Sprint:** 7.3 (W11)

---

### F7-B12: UI del Marketplace [P1]
**Como** usuario, **quiero** una interfaz web para explorar, listar y alquilar recursos, **para** participar en el marketplace sin línea de comandos.

**Criterios de Aceptación:**
- [ ] Catálogo de recursos disponibles con filtros
- [ ] Detalle de listing (specs, precio, reputación)
- [ ] Flujo de leasing (seleccionar → confirmar → activar)
- [ ] Historial de transacciones

**Dependencias:** F7-B09, F7-B10, F7-B11
**Estimación:** L (10 días)
**Sprint:** 7.3 (W11-W12)

---

## Sprint 7.4: Gobernanza Líquida + v1.0.0

### F7-B13: Gobernanza Líquida con Delegación Ponderada [P0]
**Como** participante de la red, **quiero** delegar mi poder de voto a expertos en temas específicos, **para** mejorar la calidad de las decisiones de gobernanza.

**Criterios de Aceptación:**
- [ ] Delegación ponderada por stake y reputación
- [ ] Delegación temporal (con expiry)
- [ ] Delegación por categoría (técnica, ética, económica)
- [ ] Prevención de ciclos de delegación
- [ ] Quórum dinámico basado en participación

**Dependencias:** `governance/voting.rs`, `reputation/scoring.rs`, `staking/registry.rs`
**Estimación:** XL (18 días)
**Sprint:** 7.4 (W13-W14)

---

### F7-B14: Propuestas Automáticas Basadas en Métricas [P1]
**Como** la red, **quiero** que se generen propuestas automáticamente cuando las métricas superan umbrales, **para** responder proactivamente a problemas operativos.

**Criterios de Aceptación:**
- [ ] Monitoreo continuo de métricas críticas
- [ ] Generación automática de propuestas cuando:
  - Consensus < 80% por >1h
  - Error rate > 1% por >30min
  - Resource utilization > 90% por >2h
- [ ] Validación humana antes de publicación
- [ ] Template de propuestas configurable

**Dependencias:** F7-B13, `monitoring/health.rs`, `governance/proposal.rs`
**Estimación:** M (7 días)
**Sprint:** 7.4 (W14-W15)

---

### F7-B15: Integración Phase 7 + Documentación [P0]
**Como** mantenedor del proyecto, **quiero** todos los módulos de Phase 7 integrados y documentados, **para** lanzar v1.0.0 con calidad de producción.

**Criterios de Aceptación:**
- [ ] Todos los módulos Phase 7 compilados con feature gates
- [ ] 500+ tests pasando, 0 clippy warnings
- [ ] Documentación completa:
  - API v3 OpenAPI spec
  - Migration guide v0.6.0 → v1.0.0
  - Architecture decision records (ADRs)
  - Security audit report
- [ ] Benchmarks publicados
- [ ] Release notes v1.0.0

**Dependencias:** Todos los items anteriores
**Estimación:** L (10 días)
**Sprint:** 7.4 (W15-W16)

---

### F7-B16: Programa de Onboarding para Operadores [P2]
**Como** nuevo operador, **quiero** una guía paso a paso para desplegar un nodo con Phase 7, **para** unirme a la red sin conocimientos avanzados.

**Criterios de Aceptación:**
- [ ] Guía de instalación con screenshots
- [ ] Video tutorial (≤15 min)
- [ ] FAQ con problemas comunes
- [ ] Script de validación post-instalación
- [ ] Canal de soporte dedicado

**Dependencias:** F7-B15
**Estimación:** S (4 días)
**Sprint:** 7.4 (W16)

---

## Resumen por Sprint

| Sprint | Historias | Estimación Total | Prioridad P0 |
|---|---|---|---|
| 7.1 | F7-B01 a F7-B04 | ~24 días | 2 |
| 7.2 | F7-B05 a F7-B08 | ~35 días | 1 |
| 7.3 | F7-B09 a F7-B12 | ~43 días | 2 |
| 7.4 | F7-B13 a F7-B16 | ~39 días | 2 |
| **Total** | **16 historias** | **~141 días** | **7** |

---

## Leyenda

| Campo | Descripción |
|---|---|
| P0 | Crítico — Bloquea release si no se completa |
| P1 | Importante — Debe completarse en el sprint asignado |
| P2 | Deseable — Puede moverse a sprint siguiente si hay bloqueos |
| P3 | Nice-to-have — Backlog general, no asignado a sprint |

---

*Backlog sujeto a revisión cada sprint. Prioridades pueden ajustarse basándose en feedback de la comunidad y hallazgos técnicos.*
