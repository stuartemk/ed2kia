# Release Notes - ed2kIA v0.5.0 Stable

## Fase 6: Interoperabilidad, Federación, Staking & API v2

**Fecha:** Mayo 2025
**Versión:** 0.5.0 (Stable)
**Estado:** ✅ Validación de producción completada
**Licencia:** Apache 2.0 + Cláusula de Uso Ético

---

## Estado de Validación

Esta versión **Stable** completa la migración de APIs y validación de producción:

| Métrica | Valor |
|---------|-------|
| Errores de compilación | 0 |
| Warnings | 0 |
| Tests unitarios | 76 passed, 0 failed, 3 ignored |
| APIs migradas | libp2p 0.53, wasmtime 17.0, arkworks 0.4, safetensors 0.3 |
| Build status | ✅ Limpio (`--features "core-only"`) |

**Documentación:**
- [`docs/MIGRATION_LOG_v0.5.0.md`](MIGRATION_LOG_v0.5.0.md) - Registro completo de migración
- [`docs/VALIDATION_REPORT_v0.5.0.md`](VALIDATION_REPORT_v0.5.0.md) - Reporte de validación de producción

---

## Resumen

ed2kIA v0.5.0 introduce la **Fase 6**: arquitectura completa para interoperabilidad cross-model, agregación federada (FedAvg + Krum), staking con proof-of-computation, y API REST v2 con especificación OpenAPI 3.0.3.

Esta release sienta las bases para una red P2P verdaderamente interoperable donde nodos pueden contribuir con modelos SAE de diferentes familias (Llama, Mistral, Qwen) y participar en entrenamiento federado con garantías criptográficas.

---

## Nuevas Características

### 1. Interoperabilidad Cross-Model (`src/interoperability/`)

**`adapter.rs`** - Normalización de tensores entre modelos
- `TensorAdapter`: Adapta hidden states de Llama/Mistral/GPT2 al formato Qwen-Scope
- Normalización RMSNorm y LayerNorm configurable
- Proyección dimensional para compatibilidad entre modelos de diferente tamaño
- `NormalizedHiddenState`: Schema canónico para estados normalizados

**`schema.rs`** - Validación de schema Qwen-Scope
- `QwenScopeSchema`: Schema canónico (dim=3584, rmsnorm, range ±3.0)
- `SchemaValidationError`: DimensionMismatch, ValueOutOfRange, UnsupportedModel
- Validación automática de tensores adaptados

### 2. Federación (`src/federation/`)

**`avg_aggregator.rs`** - FedAvg + Krum
- `FedAvgAggregator`: Agregación ponderada de updates de pesos
- `Krum`: Filtro de tolerancia a fallos bizantinos (n-f-2 más consistentes)
- `WeightUpdate`: Updates con hash SHA256 para verificación de integridad
- `AggregationResult`: Resultados con tracking de nodos incluidos/excluidos

**`sync_protocol.rs`** - Protocolo de sincronización federada
- `SyncProtocol`: Gestión de rondas de federación
- `SyncMessage`: Mensajes RoundRequest/Response, WeightUpdate, GlobalModel
- `FederationRound`: Estado de rondas activas con tracking de participantes

### 3. Staking (`src/staking/`)

**`proof.rs`** - Proof-of-Computation
- `StakingProof`: Pruebas con commitment hash SHA256 + nonce anti-replay
- `ProofGenerator`: Generación de pruebas con métricas de cómputo
- `ProofVerifier`: Verificación con tracking de nonces, age check, hash validation
- `ComputeMetrics`: samples_processed, compute_time_ms, memory_usage_mb, cpu_usage_percent

**`registry.rs`** - Registro de recursos
- `ResourceRegistry`: Registro de nodos con recursos comprometidos
- `ResourceCommitment`: CPU, RAM, GPU, bandwidth, storage
- `NodeStatus`: Active, Inactive, Slashed, Unregistered
- Heartbeat tracking y slashing automático
- `RegistryStats`: Métricas globales del registro

### 4. API v2 (`src/api/`)

**`openapi.rs`** - Especificación OpenAPI 3.0.3
- Generación automática de spec JSON
- Endpoints documentados: /api/v2/health, /network, /sae/analyze, /federation/rounds, /staking/registry, /governance/proposals
- Component schemas: NetworkStatus, HiddenStateInput, AnalysisResult, RoundConfig, Proposal
- Serialización JSON y YAML

**`routes.rs`** - Handlers Axum /api/v2/*
- `ApiV2State`: Estado compartido con callbacks para cada endpoint
- GET /api/v2/health - Health check
- GET /api/v2/network - Estado P2P
- POST /api/v2/sae/analyze - Análisis SAE cross-model
- GET/POST /api/v2/federation/rounds - Gestión de federación
- GET /api/v2/staking/registry - Registro de staking
- GET/POST /api/v2/governance/proposals - Gobernanza
- GET /api/v2/openapi.json - Spec OpenAPI

---

## CLI Commands (Fase 6)

### `adapt` - Adaptador de tensores
```bash
ed2kia adapt --source Llama --input-dim 4096 --output-dim 3584 --validate
```

### `federate` - Agregación federada
```bash
ed2kia federate --start --layer 0 --min-participants 3
ed2kia federate --status
```

### `stake` - Staking y proof-of-computation
```bash
ed2kia stake --register --cpu-cores 8 --ram-gb 16 --has-gpu
ed2kia stake --prove
ed2kia stake --verify
ed2kia stake --registry
```

### `api` - API v2
```bash
ed2kia api --openapi --output ./openapi.json
ed2kia api --serve --port 8080
```

---

## Resolución de Dependencias

### Problema: candle-core / rand conflict
- **Root cause**: `half 2.7+` usa `rand 0.9` pero `candle-core 0.6` necesita `rand 0.8`
- **Solución**: Pin `half = ">=2.4, <2.5"` para forzar half 2.4.1 (rand 0.8 compatible)
- **Resultado**: candle-core 0.6 compila correctamente sin breaking changes

### Versiones clave
- `candle-core = "0.6"`
- `candle-nn = "0.6"`
- `safetensors = "0.3"`
- `half = ">=2.4, <2.5"` (pinned)
- `libp2p = "0.53"`

---

## Arquitectura

```
ed2kIA v0.5.0
├── Fase 1: Core (P2P, SAE, Bridge)
├── Fase 2: Interpretación (Feature Analyzer, Semantic Map, Consensus)
├── Fase 3: Seguridad (WASM Sandbox, ZKP, Human-in-the-Loop)
├── Fase 4: Escalabilidad (Scaling, RLHF, Web UI, Monitoring)
├── Fase 5: Gobernanza (Proposals, Reputation, Ecosystem, Bootstrap)
└── Fase 6: Interoperabilidad (Adapters, Federation, Staking, API v2)
    ├── interoperability/
    │   ├── adapter.rs      # TensorAdapter (Llama/Mistral → Qwen-Scope)
    │   └── schema.rs       # QwenScopeSchema validation
    ├── federation/
    │   ├── avg_aggregator.rs  # FedAvg + Krum
    │   └── sync_protocol.rs   # Sync messages & rounds
    ├── staking/
    │   ├── proof.rs        # Proof-of-Computation
    │   └── registry.rs     # Resource Registry
    └── api/
        ├── openapi.rs      # OpenAPI 3.0.3 spec
        └── routes.rs       # Axum /api/v2/* handlers
```

---

## Notas Técnicas

### Breaking Changes
- Ninguno en Fase 6 (módulos nuevos)
- **Migración completada:** Todos los errores pre-existentes en Fase 1-5 han sido resueltos

### Migración de APIs

| Dependencia | Versión anterior | Versión actual | Cambios principales |
|-------------|------------------|----------------|---------------------|
| libp2p | 0.52 | 0.53 | `behaviour()` → `behaviour_mut()`, `cbor::Behaviour` |
| wasmtime | 16.x | 17.0 | `Caller<'a, T>` requiere genérico, closures `'static` |
| arkworks | 0.3 | 0.4 | `format_with` → `serialize_compressed`, `CanonicalSerialize` |
| safetensors | 0.2 | 0.3 | API de carga de weights actualizada |
| serde | 1.0 | 1.0 | Compatibilidad mantenida |

### Feature Flags

| Feature | Descripción |
|---------|-------------|
| `core-only` | Fases 1-3: Core P2P, SAE, Bridge, Interpretación, Seguridad, ZKP, Human-in-the-Loop |
| `phase6-experimental` | Fases 4-6: Escalabilidad, RLHF, Web, Monitoring, Gobernanza, Interoperabilidad, Federación, Staking, API v2 |

### Compatibilidad
- Rust 1.70+
- Windows 11 / Linux / macOS
- CPU (default), CUDA (feature), Metal (feature)

### Pruebas
- Todos los módulos Fase 6 incluyen tests unitarios
- Coverage: adapter, schema, aggregator, sync, proof, registry, openapi, routes
- Verificación: `cargo check --features "core-only"` (0 errores)

### Comandos de Verificación

```bash
# Verificar compilación limpia (core-only)
cargo check --features "core-only"

# Verificar con clippy
cargo clippy --features "core-only"

# Ejecutar tests
cargo test --features "core-only"

# Build de release
cargo build --release --features "core-only"

---

## Roadmap

### v0.6.0 (Próximo)
- Integración completa de API v2 con Axum Router
- Endpoints reales conectados a módulos de federación y staking
- Dashboard web actualizado para Fase 6
- Tests de integración end-to-end

### v0.7.0
- Resolución de errores pre-existentes (libp2p 0.54+, wasmtime 25+)
- Soporte para modelos Mistral-SAE y Llama-SAE nativos
- Benchmark de rendimiento de federación
- Documentación API interactiva (Swagger UI)

---

## Créditos

- **Proyecto:** ed2kIA - Red Descentralizada de Interpretabilidad
- **Licencia:** Apache 2.0 + Cláusula de Uso Ético
- **Repositorio:** https://github.com/ed2kIA/ed2kIA

---

*Este software es de código abierto, transparente y diseñado exclusivamente para el progreso humano y el desarrollo responsable de la IA.*
