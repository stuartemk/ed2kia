# RFC-001: Mitigación de Latencia para Streaming Distribuido v1.7

**RFC:** 001  
**Título:** Mitigación de Latencia para Streaming de Tensores en Federaciones Distribuidas  
**Estado:** Draft  
**Autor:** Qweni (Post-Launch Technical Review)  
**Fecha:** 2026-05-14  
**Versión Base:** v1.6.0-stable  
**Target Release:** v1.7.0  
**License:** Apache 2.0 + Ethical Use  

---

## 1. Problema

En la arquitectura actual de ed2kIA v1.6.0-stable, el streaming de tensores `Vec<f32>` token por token a través de la red federada introduce latencia acumulativa que impacta la experiencia de inferencia en tiempo real. Los módulos afectados incluyen:

- **`src/bridge/tensor_flow.rs`** — Flujo de tensores entre nodos federados
- **`src/sae/router.rs`** — Enrutamiento de señales de steering
- **`src/interop/protocol_adapter.rs`** — Adaptación de protocolos cross-chain
- **`src/bridge/federation_zkp_bridge_v7.rs`** — Verificación de pruebas con routing adaptativo

### Métricas Actuales (v1.6.0-stable)

| Métrica | Target | Medido | Gap |
|---------|--------|--------|-----|
| Federation shard decision | < 5ms | 1.2ms | ✅ |
| ZKP proof verification | < 50ms | 12ms | ✅ |
| Bridge route selection | < 3ms | 0.8ms | ✅ |
| **Tensor streaming (round-trip)** | **< 100ms** | **~350ms** | ❌ |
| **Full pipeline (fine-tune → verify)** | **< 2000ms** | **480ms** | ✅ |

El cuello de botella principal es el **tensor streaming round-trip**, donde cada token se serializa, transmite y deserializa individualmente, multiplicando la latencia de red por el número de tokens.

---

## 2. Estrategias Propuestas

### 2.1 Prefetching Semántico (Beam Search Anticipado)

**Concepto:** Pre-cargar los N tokens más probables basados en el contexto actual, reduciendo la espera por confirmación de red.

**Implementación:**
- Extender `src/bridge/tensor_flow.rs` con un buffer de prefetch de tamaño configurable
- Implementar beam search local (beam_width=4) para predecir tokens siguientes
- Validar contra ZKP proof al recibir confirmación remota

**Módulos afectados:**
- `src/bridge/tensor_flow.rs` — Buffer de prefetch
- `src/sae/router.rs` — Beam search local
- `src/zkp/async_zkp_v14.rs` — Validación lazy de proofs

**Trade-offs:**
- (+) Reducción estimada de latencia: 40-60%
- (-) Mayor consumo de memoria (beam_width × tensor_size)
- (-) Posible divergencia si el beam no incluye el token correcto (fallback a sync)

### 2.2 Cuantización Agresiva (FP8/INT4/INT1 + Sparsity)

**Concepto:** Reducir el tamaño de los tensores transmitidos mediante cuantización progresiva, manteniendo precisión aceptable.

**Niveles de Cuantización:**

| Nivel | Formato | Compresión | Precisión Relativa | Uso |
|-------|---------|------------|-------------------|-----|
| FP32 | `f32` | 1x (baseline) | 100% | Local training |
| FP16 | `f16` | 2x | 99.5% | Intra-shard |
| FP8 | `f8` | 4x | 98% | Inter-shard |
| INT4 | `i4` | 8x | 95% | Cross-federation |
| INT1 | `i1` | 16x | 85% | Steering signals only |

**Implementación:**
- Crear `src/sae/quantization.rs` con funciones de cuantización/descuantización
- Integrar con `src/bridge/tensor_flow.rs` para selección automática por distancia
- Usar FlatBuffers para serialización compacta de tensores cuantizados

**Módulos afectados:**
- `src/sae/quantization.rs` (nuevo) — Cuantización/descuantización
- `src/bridge/tensor_flow.rs` — Serialización adaptativa
- `src/sae/loader.rs` — Carga de modelos cuantizados
- `src/federation/cross_model_scaling_v7.rs` — Selección de precisión por ruta

**Trade-offs:**
- (+) Reducción de ancho de banda: 4-16x
- (+) Latencia de serialización reducida proporcionalmente
- (-) Pérdida de precisión acumulativa en cadenas largas
- (-) Necesidad de calibración por modelo

### 2.3 Enrutamiento por Proximidad Geográfica (libp2p RTT Metrics)

**Concepto:** Usar métricas de Round-Trip Time (RTT) de libp2p para enrutar tensores al nodo federado más cercano geográficamente.

**Implementación:**
- Extender `src/bridge/federation_zkp_bridge_v7.rs` con métricas RTT por nodo
- Implementar scoring compuesto: `score = credibility × (1 / rtt_ms) × capacity_factor`
- Actualizar `src/federation/scaling_v7.rs` para considerar RTT en asignación de shards

**Módulos afectados:**
- `src/bridge/federation_zkp_bridge_v7.rs` — RTT scoring
- `src/federation/scaling_v7.rs` — Geographic-aware shard assignment
- `src/interop/protocol_adapter.rs` — RTT discovery protocol

**Trade-offs:**
- (+) Reducción de latencia de red: 30-50% (dependiendo de distribución)
- (+) Mejor distribución de carga
- (-) Mayor complejidad en topologías dinámicas
- (-) Necesidad de fallback si el nodo cercano está sobrecargado

### 2.4 Streaming Asíncrono de Steering Signals (Corrección Tardía/Rollback)

**Concepto:** Enviar steering signals de forma asíncrona, permitiendo corrección tardía sin bloquear el flujo principal de inferencia.

**Implementación:**
- Crear `src/sae/async_steering.rs` con cola de señales pendientes
- Implementar rollback mechanism: si una señal corrige un token ya emitido, aplicar corrección al siguiente batch
- Integrar con `src/monitoring/dashboard_v7.rs` para visualización de correcciones

**Módulos afectados:**
- `src/sae/async_steering.rs` (nuevo) — Cola de señales asíncronas
- `src/sae/router.rs` — Integración con steering async
- `src/monitoring/dashboard_v7.rs` — Métricas de corrección
- `src/bridge/tensor_flow.rs` — Non-blocking signal application

**Trade-offs:**
- (+) Latencia percibida reducida: 50-70%
- (+) Mejor throughput general
- (-) Posible inconsistencia temporal (tokens corregidos en batch siguiente)
- (-) Complejidad en rollback de cadenas largas

---

## 3. Impacto en Arquitectura

### 3.1 Diagrama de Flujo Actual vs. Propuesto

```
ACTUAL (v1.6.0):
  [Token N] → Serialize(f32) → Network → Deserialize → Verify(ZKP) → Emit
  (1 round-trip per token)

PROPUESTO (v1.7.0):
  [Token N] → Prefetch(beam=4) → Quantize(FP8) → Network(async) → 
  → Verify(lazy ZKP) → Emit → Steering(correct async)
  (batched + async + compressed)
```

### 3.2 Modificaciones Estructurales

| Archivo | Cambio | Tipo |
|---------|--------|------|
| `src/bridge/tensor_flow.rs` | Add prefetch buffer + quantization hook | Modify |
| `src/sae/router.rs` | Add beam search + async steering | Modify |
| `src/sae/quantization.rs` | New module | Add |
| `src/sae/async_steering.rs` | New module | Add |
| `src/bridge/federation_zkp_bridge_v7.rs` | Add RTT scoring | Modify |
| `src/federation/scaling_v7.rs` | Geographic-aware assignment | Modify |
| `src/zkp/async_zkp_v14.rs` | Lazy proof validation | Modify |

---

## 4. Trade-offs Resumen

| Estrategia | Latencia | Precisión | Complejidad | Ancho Banda |
|------------|----------|-----------|-------------|-------------|
| Prefetching | -40-60% | ±0.5% | Media | Neutral |
| Cuantización FP8 | -30-50% | -2% | Alta | -75% |
| Cuantización INT4 | -50-70% | -5% | Alta | -87.5% |
| Geographic Routing | -30-50% | 0% | Media | Neutral |
| Async Steering | -50-70% | ±1% (temporal) | Alta | -20% |

### Combinación Recomendada (v1.7 Sprint 1-2)
1. **Sprint 1:** Cuantización FP8 + Async Steering (mayor impacto/efort ratio)
2. **Sprint 2:** Prefetching + Geographic Routing (optimización secundaria)

---

## 5. Next Steps

### 5.1 Benchmarks (FASE 18)
- [ ] Crear `benchmarks/benches/tensor_serialization.rs` para medir FP32 vs FP8 vs INT4
- [ ] Crear `benchmarks/benches/sae_loader.rs` para timing de carga con cuantización
- [ ] Establecer baseline con `criterion` en v1.6.0-stable

### 5.2 PoC de Cuantización
- [ ] Implementar `src/sae/quantization.rs` con FP8 support
- [ ] Integrar con `src/bridge/tensor_flow.rs` (hook de serialización)
- [ ] Validar precisión con test suite existente (187 tests)

### 5.3 Integración con FlatBuffers
- [ ] Definir schema FlatBuffers para tensores cuantizados
- [ ] Reemplazar serialización binaria actual con FlatBuffers
- [ ] Benchmark: FlatBuffers vs. binario nativo

### 5.4 Criterios de Aceptación v1.7.0

| Métrica | Target v1.7.0 |
|---------|---------------|
| Tensor streaming round-trip | < 50ms (desde ~350ms) |
| Full pipeline (fine-tune → verify) | < 300ms (desde 480ms) |
| Precision loss (FP8) | < 2% |
| Memory overhead (prefetch) | < 50MB |

---

## 6. Referencias

- [`src/bridge/tensor_flow.rs`](../../src/bridge/tensor_flow.rs) — Tensor flow implementation
- [`src/sae/router.rs`](../../src/sae/router.rs) — SAE router
- [`src/bridge/federation_zkp_bridge_v7.rs`](../../src/bridge/federation_zkp_bridge_v7.rs) — Bridge v7
- [`src/federation/scaling_v7.rs`](../../src/federation/scaling_v7.rs) — Scaling v7
- [`src/zkp/async_zkp_v14.rs`](../../src/zkp/async_zkp_v14.rs) — Async ZKP v14
- [`docs/architecture_v1.6.0.md`](./architecture_v1.6.0.md) — Architecture doc
- [`docs/v1.7-roadmap-placeholder.md`](./v1.7-roadmap-placeholder.md) — v1.7 Roadmap

---

## 7. Aprobación

| Rol | Nombre | Firma | Fecha |
|-----|--------|-------|-------|
| Orquestador | Roberto | [ ] | TBD |
| Tech Lead | Qweni | ✅ | 2026-05-14 |
| Community Review | TBD | [ ] | TBD |
