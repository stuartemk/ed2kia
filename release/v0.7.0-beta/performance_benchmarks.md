# Performance Benchmarks - v0.7.0-Beta

> **Fecha**: 2026-05-04  
> **VersiÃ³n**: v0.7.0-beta  
> **Estado**: DefiniciÃ³n de mÃ©tricas y umbrales de aceptaciÃ³n  
> **Licencia**: Apache 2.0 + Ethical Use Clause  

---

## 1. PropÃ³sito

Este documento define los mÃ©tricas de rendimiento objetivo, scripts de benchmark y umbrales de aceptaciÃ³n/rechazo para la versiÃ³n v0.7.0-beta. Los benchmarks validan que los mÃ³dulos de Phase 6 y Phase 7 cumplen con los requisitos de producciÃ³n antes de la promociÃ³n a v0.8.0-alpha.

---

## 2. MÃ©tricas Objetivo

### 2.1 Resumen Ejecutivo

| MÃ©trica | Objetivo | Umbral CrÃ­tico | MÃ©todo de MediciÃ³n |
|---|---|---|---|
| **SAE Latency (p50)** | â¤350ms | >500ms | `ops/benchmark_runner.sh --sae-load` |
| **Consensus Rate** | â¥88% | <80% | `ops/benchmark_runner.sh --p2p-sim` |
| **WASM Memory** | â¤180MB | >250MB | `ops/benchmark_runner.sh --sae-load --measure-memory` |
| **API v2 Throughput** | â¥500 req/s | <300 req/s | `ops/benchmark_runner.sh --api-load` |
| **Alignment Drift (p95)** | â¤0.15 | >0.30 | `ops/benchmark_runner.sh --alignment-loop` |
| **Trust Score Update** | â¤50ms/node | >100ms/node | `ops/benchmark_runner.sh --trust-scoring` |
| **Schema Validation** | â¤20ms/schema | >50ms/schema | `ops/benchmark_runner.sh --schema-registry` |

### 2.2 SAE Latency

**DefiniciÃ³n**: Tiempo de ejecuciÃ³n del forward pass del SAE (Sparse Autoencoder) incluyendo carga de pesos, inferencia y extracciÃ³n de activaciones.

| Percentil | Objetivo | Umbral CrÃ­tico | MÃ©todo |
|---|---|---|---|
| p50 | â¤350ms | >500ms | Promedio de 1000 iteraciones |
| p95 | â¤450ms | >650ms | Percentil 95 de 1000 iteraciones |
| p99 | â¤550ms | >800ms | Percentil 99 de 1000 iteraciones |

**ConfiguraciÃ³n de referencia**:
- Modelo: SAE 4096 â 16384 (expansion factor 4x)
- Batch size: 1
- Device: CPU (x86_64, 8 cores)
- Feature gate: `core-only`

**Script**:
```bash
./ops/benchmark_runner.sh --sae-load \
  --iterations 1000 \
  --batch-size 1 \
  --features core-only \
  --output results/sae_latency.jsonl
```

### 2.3 Consensus Rate

**DefiniciÃ³n**: Porcentaje de rondas de federaciÃ³n que alcanzan consenso vÃ¡lido (â¥min_participants con hash vÃ¡lido).

| MÃ©trica | Objetivo | Umbral CrÃ­tico | MÃ©todo |
|---|---|---|---|
| Consensus Rate | â¥88% | <80% | 100 rondas simuladas |
| Round Latency (p50) | â¤2s | >5s | Tiempo por ronda |
| Byzantine Tolerance | â¤20% byzantinos | >25% | Krum filter effectiveness |

**ConfiguraciÃ³n de referencia**:
- Nodos: 10 (8 honestos, 2 byzantinos)
- Min participants: 7
- Feature gate: `phase6-core`

**Script**:
```bash
./ops/benchmark_runner.sh --p2p-sim \
  --nodes 10 \
  --byzantine-ratio 0.2 \
  --rounds 100 \
  --features phase6-core \
  --output results/consensus.jsonl
```

### 2.4 WASM Memory

**DefiniciÃ³n**: Uso pÃ©ximo de memoria durante la ejecuciÃ³n del forward pass del SAE en el sandbox WASM.

| MÃ©trica | Objetivo | Umbral CrÃ­tico | MÃ©todo |
|---|---|---|---|
| Peak Memory | â¤180MB | >250MB | MemoryGuard stats |
| Memory Leak Rate | â¤1MB/1000 iter | >5MB/1000 iter | Diferencia inicial vs final |
| GC Pressure | â¤10% del tiempo | >20% | Time spent in GC |

**ConfiguraciÃ³n de referencia**:
- MÃ³dulo WASM: SAE forward pass
- Iteraciones: 1000
- MemoryGuard limit: 512MB
- Feature gate: `core-only`

**Script**:
```bash
./ops/benchmark_runner.sh --sae-load \
  --iterations 1000 \
  --measure-memory \
  --memory-limit 512 \
  --features core-only \
  --output results/wasm_memory.jsonl
```

### 2.5 API v2 Throughput

**DefiniciÃ³n**: Requests por segundo procesados por la API v2 (endpoints /api/v2/*).

| Endpoint | Objetivo | Umbral CrÃ­tico | MÃ©todo |
|---|---|---|---|
| GET /api/v2/health | â¥1000 req/s | <500 req/s | wrk/ab |
| POST /api/v2/sae/analyze | â¥500 req/s | <300 req/s | wrk/ab |
| POST /api/v2/federation/round | â¥200 req/s | <100 req/s | wrk/ab |
| POST /api/v2/governance/proposal | â¥150 req/s | <80 req/s | wrk/ab |

**ConfiguraciÃ³n de referencia**:
- Concurrency: 50
- Duration: 60s
- Feature gate: `phase6-core`

**Script**:
```bash
./ops/benchmark_runner.sh --api-load \
  --concurrency 50 \
  --duration 60 \
  --features phase6-core \
  --output results/api_throughput.jsonl
```

### 2.6 Alignment Drift

**DefiniciÃ³n**: DesviaciÃ³n promedio entre activaciones actuales y deseadas despuÃ©s de aplicar steering.

| MÃ©trica | Objetivo | Umbral CrÃ­tico | MÃ©todo |
|---|---|---|---|
| Drift (p50) | â¤0.10 | >0.20 | AlignmentScorer.compute_drift() |
| Drift (p95) | â¤0.15 | >0.30 | Percentil 95 |
| Rollback Rate | â¤5% | >15% | AlignmentFeedbackLoop |

**ConfiguraciÃ³n de referencia**:
- Feedback entries: 100
- Layer: SAE layer 0
- Feature gate: `phase7-sprint1` + `phase7-sprint2`

**Script**:
```bash
./ops/benchmark_runner.sh --alignment-loop \
  --feedback-count 100 \
  --features phase7-sprint1,phase7-sprint2 \
  --output results/alignment_drift.jsonl
```

### 2.7 Trust Score Update

**DefiniciÃ³n**: Tiempo de actualizaciÃ³n del trust score por nodo en el DynamicTrustScorer.

| MÃ©trica | Objetivo | Umbral CrÃ­tico | MÃ©todo |
|---|---|---|---|
| Update Time (p50) | â¤50ms/node | >100ms/node | DynamicTrustScorer.update_score() |
| Sybil Detection (p50) | â¤200ms | >500ms | DynamicTrustScorer.detect_sybil() |
| Cross-net Propagation | â¤100ms | >200ms | DynamicTrustScorer.propagate_cross_net() |

**ConfiguraciÃ³n de referencia**:
- Nodos: 100
- Redes: 3
- Feature gate: `phase7-sprint2`

**Script**:
```bash
./ops/benchmark_runner.sh --trust-scoring \
  --nodes 100 \
  --networks 3 \
  --features phase7-sprint2 \
  --output results/trust_scoring.jsonl
```

### 2.8 Schema Validation

**DefiniciÃ³n**: Tiempo de validaciÃ³n de un esquema en el SchemaRegistry.

| MÃ©trica | Objetivo | Umbral CrÃ­tico | MÃ©todo |
|---|---|---|---|
| Register Time | â¤15ms | >30ms | SchemaRegistry.register() |
| Validate Time | â¤20ms/schema | >50ms/schema | SchemaRegistry.validate() |
| Compatible Query | â¤10ms | >25ms | SchemaRegistry.get_compatible() |

**ConfiguraciÃ³n de referencia**:
- Esquemas registrados: 50
- Feature gate: `phase7-sprint2`

**Script**:
```bash
./ops/benchmark_runner.sh --schema-registry \
  --schemas 50 \
  --features phase7-sprint2 \
  --output results/schema_validation.jsonl
```

---

## 3. Umbrales de AceptaciÃ³n/Rechazo

### 3.1 Criterios de PromociÃ³n a v0.8.0-alpha

| Criterio | Umbral | Estado |
|---|---|---|
| SAE Latency p50 â¤350ms | â / â | Pendiente |
| Consensus Rate â¥88% | â / â | Pendiente |
| WASM Memory â¤180MB | â / â | Pendiente |
| API v2 Throughput â¥500 req/s | â / â | Pendiente |
| Alignment Drift p95 â¤0.15 | â / â | Pendiente |
| Trust Score Update â¤50ms/node | â / â | Pendiente |
| Schema Validation â¤20ms/schema | â / â | Pendiente |
| 0 errores de seguridad crÃ­ticos | â / â | Pendiente |
| 0 warnings de clippy | â / â | Pendiente |
| 100% tests passing | â / â | Pendiente |

**Requisito**: Todos los criterios deben ser â para promociÃ³n.

### 3.2 Procedimiento de Rechazo

Si cualquier mÃ©trica excede el umbral crÃ­tico:
1. Registrar hallazgo en `release/v0.7.0-beta/security_audit_prep.md`
2. Crear issue con prioridad P0
3. Asignar al equipo responsable
4. Establecer SLA de remediaciÃ³n (48h para P0, 72h para P1)
5. Re-ejecutar benchmarks despuÃ©s de remediaciÃ³n

---

## 4. Hardware de Referencia

### 4.1 MÃ¡quina de Benchmark

| Componente | EspecificaciÃ³n |
|---|---|
| CPU | AMD Ryzen 9 5950X (16 cores / 32 threads) |
| RAM | 64GB DDR4-3200 |
| Storage | NVMe SSD (Samsung 980 PRO) |
| OS | Ubuntu 24.04 LTS (WSL2) |
| Rust | 1.85.0 (stable) |
| Feature Flags | `phase7-sprint1` + `phase7-sprint2` + `phase6-core` |

### 4.2 CI/CD (GitHub Actions)

| Componente | EspecificaciÃ³n |
|---|---|
| Runner | ubuntu-latest (GitHub-hosted) |
| CPU | 2 cores |
| RAM | 7GB |
| Storage | 14GB SSD |

**Nota**: Los umbrales en CI pueden ser 2x mÃ¡s relajados que en hardware de referencia.

---

## 5. Formato de Salida JSONL

Cada benchmark exporta resultados en formato JSONL:

```jsonl
{"timestamp":"2026-05-04T13:00:00Z","benchmark":"sae_latency","metric":"p50_ms","value":342.5,"unit":"ms","status":"pass"}
{"timestamp":"2026-05-04T13:00:00Z","benchmark":"sae_latency","metric":"p95_ms","value":438.2,"unit":"ms","status":"pass"}
{"timestamp":"2026-05-04T13:00:00Z","benchmark":"sae_latency","metric":"p99_ms","value":521.7,"unit":"ms","status":"pass"}
```

**Campos**:
- `timestamp`: ISO 8601 UTC
- `benchmark`: Nombre del benchmark
- `metric`: Nombre de la mÃ©trica
- `value`: Valor numÃ©rico
- `unit`: Unidad de mediciÃ³n
- `status`: "pass" | "warn" | "fail"

---

## 6. Historial de Benchmarks

| VersiÃ³n | Fecha | SAE Latency | Consensus | WASM Mem | API TPS | Estado |
|---|---|---|---|---|---|---|
| v0.5.0 | 2026-03-15 | 320ms | 91% | 165MB | N/A | â STABLE |
| v0.6.0-RC | 2026-04-01 | 335ms | 89% | 172MB | 520 | â RC |
| v0.7.0-alpha | 2026-05-01 | 348ms | 88% | 178MB | 505 | â Alpha |
| v0.7.0-beta | 2026-05-04 | Pendiente | Pendiente | Pendiente | Pendiente | Pendiente |

---

## 7. Contactos

| Rol | Contacto | Responsabilidad |
|---|---|---|
| Performance Architect | `@ed2kia/perf-team` | DiseÃ±o de benchmarks, anÃ¡lisis de resultados |
| Release Engineer | `@ed2kia/release-team` | EjecuciÃ³n de benchmarks, validaciÃ³n de umbrales |
| SAE Team | `@ed2kia/sae-team` | OptimizaciÃ³n de latencia SAE |
| Federation Team | `@ed2kia/fed-team` | OptimizaciÃ³n de consenso |

---

*Documento generado para v0.7.0-beta. PrÃ³xima revisiÃ³n: v0.8.0-alpha.*
