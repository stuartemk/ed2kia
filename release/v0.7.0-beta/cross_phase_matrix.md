# Cross-Phase Module Matrix - v0.7.0-Beta

> **Fecha**: 2026-05-04  
> **VersiÃ³n**: v0.7.0-beta  
> **Estado**: ConsolidaciÃ³n alpha â beta  
> **Licencia**: Apache 2.0 + Ethical Use Clause  

---

## 1. PropÃ³sito

Esta matriz documenta las dependencias cruzadas entre mÃ³dulos de v0.5.0 (STABLE), Phase 6 (interoperabilidad/federaciÃ³n/staking/API v2) y Phase 7 (alignment/trust/schema). Establece los feature gates, procedimientos de fallback y rutas de migraciÃ³n para cada integraciÃ³n cross-phase.

---

## 2. Matriz de Dependencias Cross-Phase

| MÃ³dulo Phase 7 | Archivo | Dependencia v0.5.0 | Dependencia Phase 6 | Feature Gate | Fallback |
|---|---|---|---|---|---|
| **AlignmentFeedbackLoop** | `src/alignment/feedback_loop.rs` | `consciousness.rs` (SteeringSignal) | `engine.rs` (AlignmentFeedback) | `phase7-sprint2` | Desactivar loop â usar steering estÃ¡tico |
| **DynamicTrustScorer** | `src/federation/trust_scoring.rs` | `p2p/swarm.rs` (node discovery) | `bridge.rs` (TrustRecord) | `phase7-sprint2` | Trust estÃ¡tico (0.8 base) |
| **SchemaRegistry** | `src/interoperability/schema_registry.rs` | `sae/loader.rs` (tensor schema) | `adapter.rs` (NormalizedHiddenState) | `phase7-sprint2` | ValidaciÃ³n bÃ¡sica de dimensiones |
| **AlignmentScorer** | `src/alignment/engine.rs` | `bridge/consciousness.rs` | `adapter.rs` (tensor normalization) | `phase7-sprint1` | Drift neutral (0.0) |
| **FederationBridge** | `src/federation/bridge.rs` | `p2p/protocol.rs` | `sync_protocol.rs` (FederationRound) | `phase7-sprint1` | Red local (single-network) |
| **ConsciousnessBridge** | `src/bridge/consciousness.rs` | `sae/router.rs` | `adapter.rs` + `schema.rs` | `core-only` | Sin inyecciÃ³n de contexto |
| **FedAvgAggregator** | `src/federation/avg_aggregator.rs` | `sae/loader.rs` | `adapter.rs` (WeightUpdate) | `phase6-core` | Promedio simple (sin Krum) |
| **SyncProtocol** | `src/federation/sync_protocol.rs` | `p2p/swarm.rs` | `avg_aggregator.rs` | `phase6-core` | Sync sincrÃ³nico directo |
| **TensorAdapter** | `src/interoperability/adapter.rs` | `sae/loader.rs` | `schema.rs` (QwenScopeSchema) | `phase6-core` | Sin adaptaciÃ³n (passthrough) |
| **ResourceRegistry** | `src/staking/registry.rs` | `security/memory_guard.rs` | `auth.rs` (node auth) | `phase6-core` | Sin slashing |
| **AuthValidator** | `src/api/auth.rs` | `security/wasm_sandbox.rs` | `routes.rs` (API v2) | `phase6-core` | Sin validaciÃ³n de firmas |
| **WASMSandbox** | `src/security/wasm_sandbox.rs` | `security/memory_guard.rs` | `onnx_adapter.rs` | `core-only` | EjecuciÃ³n local sin sandbox |
| **MemoryGuard** | `src/security/memory_guard.rs` | N/A (core) | `wasm_sandbox.rs` | `core-only` | Sin lÃ­mites de memoria |

---

## 3. Flujos Integrados Cross-Phase

### 3.1 Flujo de AlineaciÃ³n Continua (Phase 7 â Phase 6 â Core)

```
Usuario/Anotador
    â AlignmentFeedback (phase7-sprint1)
AlignmentScorer [engine.rs]
    â compute_drift()
AlignmentFeedbackLoop [feedback_loop.rs] (phase7-sprint2)
    â apply_steering() â SteeringSignal
ConsciousnessBridge [consciousness.rs] (core)
    â inject_context()
SAE Router [router.rs] (core)
    â forward_pass con steering aplicado
Resultado alineado
```

**Feature gates activos**: `phase7-sprint1` + `phase7-sprint2` + `core-only`  
**Fallback**: Si `phase7-sprint2` desactivado â AlignmentScorer genera steering sin feedback loop.

### 3.2 Flujo de FederaciÃ³n Cross-Red (Phase 7 â Phase 6 â Core)

```
Nodo Local (Network A)
    â DeltaUpdate
FederationBridge [bridge.rs] (phase7-sprint1)
    â sync_delta() â TrustRecord
DynamicTrustScorer [trust_scoring.rs] (phase7-sprint2)
    â update_score() â trust_score
SyncProtocol [sync_protocol.rs] (phase6-core)
    â process_message() â FederationRound
FedAvgAggregator [avg_aggregator.rs] (phase6-core)
    â aggregate() â AggregationResult
P2P Swarm [swarm.rs] (core)
    â broadcast()
Nodos Remotos (Network B, C, ...)
```

**Feature gates activos**: `phase7-sprint1` + `phase7-sprint2` + `phase6-core`  
**Fallback**: Si `phase7-sprint2` desactivado â trust_score estÃ¡tico (0.8).

### 3.3 Flujo de ValidaciÃ³n de Esquemas (Phase 7 â Phase 6 â Core)

```
Modelo Externo (ONNX)
    â Tensor crudo
ONNX Adapter [onnx_adapter.rs] (phase6-core)
    â load_model() â Tensor
TensorAdapter [adapter.rs] (phase6-core)
    â adapt() â NormalizedHiddenState
SchemaRegistry [schema_registry.rs] (phase7-sprint2)
    â validate() â SchemaResult (compatible/incompatible)
SAE Loader [loader.rs] (core)
    â load_weights() â SAE weights validados
```

**Feature gates activos**: `phase7-sprint2` + `phase6-core`  
**Fallback**: Si `phase7-sprint2` desactivado â validaciÃ³n bÃ¡sica de dimensiones.

---

## 4. Procedimientos de Fallback

### 4.1 DesactivaciÃ³n de `phase7-sprint2`

| MÃ³dulo | Comportamiento con feature | Comportamiento sin feature | Impacto |
|---|---|---|---|
| AlignmentFeedbackLoop | Loop completo (feedback â drift â steering â rollback) | No disponible | Sin cierre de loop continuo |
| DynamicTrustScorer | Scoring dinÃ¡mico con detecciÃ³n Sybil | Trust estÃ¡tico 0.8 | Sin detecciÃ³n Sybil |
| SchemaRegistry | ValidaciÃ³n semÃ¡ntica completa | ValidaciÃ³n bÃ¡sica | Sin versionado semÃ¡ntico |

**Procedimiento**:
1. Desactivar feature: `cargo build --features "phase7-sprint1,phase6-core"`
2. Verificar compilaciÃ³n: `cargo check --features "phase7-sprint1,phase6-core"`
3. Ejecutar tests: `cargo test --features "phase7-sprint1,phase6-core"`
4. Monitorear mÃ©tricas: drift, trust_score, schema_validation_rate

### 4.2 DesactivaciÃ³n de `phase7-sprint1`

| MÃ³dulo | Comportamiento con feature | Comportamiento sin feature | Impacto |
|---|---|---|---|
| AlignmentScorer | CÃ¡lculo de drift + steering | No disponible | Sin alineaciÃ³n |
| FederationBridge | Sync cross-red con handshake | No disponible | Single-network |

**Procedimiento**:
1. Desactivar feature: `cargo build --features "phase6-core"`
2. Verificar compilaciÃ³n: `cargo check --features "phase6-core"`
3. Ejecutar tests: `cargo test --features "phase6-core"`
4. Monitorear mÃ©tricas: consensus_rate, sync_latency

### 4.3 DesactivaciÃ³n de `phase6-core`

| MÃ³dulo | Comportamiento con feature | Comportamiento sin feature | Impacto |
|---|---|---|---|
| FedAvgAggregator | AgregaciÃ³n FedAvg + Krum | No disponible | Sin federaciÃ³n |
| SyncProtocol | Sync P2P asÃ­ncrono | No disponible | Sin sync |
| TensorAdapter | AdaptaciÃ³n cross-model | No disponible | Solo Qwen |

**Procedimiento**:
1. Desactivar feature: `cargo build --features "core-only"`
2. Verificar compilaciÃ³n: `cargo check --features "core-only"`
3. Ejecutar tests: `cargo test --features "core-only"`
4. Monitorear mÃ©tricas: sae_latency, memory_usage

---

## 5. Feature Gates Resumen

| Feature | Fase | MÃ³dulos Incluidos | Estado v0.7.0-beta |
|---|---|---|---|
| `core-only` | Base | SAE, P2P, Security, Bridge, Interpret, ZKP, Human, Scaling, RLHF, Web, Monitoring, Governance, Reputation, Ecosystem, Bootstrap | â STABLE |
| `phase6-core` | Phase 6 | Interoperability (adapter, onnx, schema), Federation (avg_aggregator, sync_protocol), Staking (registry, proof), API (routes, auth, openapi) | â STABLE |
| `phase7-sprint1` | Phase 7 Sprint 1 | Alignment (engine), Federation (bridge), Phase7 mod | â STABLE |
| `phase7-sprint2` | Phase 7 Sprint 2 | Alignment (feedback_loop), Federation (trust_scoring), Interoperability (schema_registry) | â BETA |

---

## 6. ValidaciÃ³n Cross-Phase

### 6.1 Comandos de ValidaciÃ³n

```bash
# ValidaciÃ³n completa (todos los features)
cargo check --all-features
cargo clippy --all-features -- -D warnings
cargo test --all-features

# ValidaciÃ³n Phase 7 solo
cargo check --features "phase7-sprint1,phase7-sprint2,phase6-core"
cargo test --features "phase7-sprint1,phase7-sprint2,phase6-core"

# ValidaciÃ³n Phase 6 solo
cargo check --features "phase6-core"
cargo test --features "phase6-core"

# ValidaciÃ³n Core solo
cargo check --features "core-only"
cargo test --features "core-only"
```

### 6.2 Tests E2E Cross-Phase

| Test | Archivo | Features Requeridos | DescripciÃ³n |
|---|---|---|---|
| `test_feedback_loop_integration` | `tests/integration/phase7_e2e.rs` | phase7-sprint2 | Feedback â Loop â Steering |
| `test_trust_scoring_integration` | `tests/integration/phase7_e2e.rs` | phase7-sprint2 | Trust â Sybil â Cross-net |
| `test_schema_registry_integration` | `tests/integration/phase7_e2e.rs` | phase7-sprint2 | Register â Validate â Compatible |
| `test_full_pipeline_simulation` | `tests/integration/phase6_e2e.rs` | phase6-core | ONNX â Adapter â FedAvg â Staking |
| `test_alignment_bridge_flow` | `tests/integration/phase7_e2e.rs` | phase7-sprint1 + phase7-sprint2 | Engine â Bridge â Loop |

---

## 7. Matriz de Compatibilidad de Versiones

| VersiÃ³n | Feature Gates | MÃ³dulos Activos | Estado | Soporte |
|---|---|---|---|---|
| v0.5.0 | `core-only` | Core (16 mÃ³dulos) | STABLE | LTS hasta v1.0.0 |
| v0.6.0-RC | `phase6-core` | Core + Phase 6 (12 mÃ³dulos) | RC | Canary rollout |
| v0.7.0-alpha | `phase7-sprint1` + `phase7-sprint2` | Core + P6 + P7 (6 mÃ³dulos) | Alpha | ValidaciÃ³n E2E |
| v0.7.0-beta | `phase7-sprint1` + `phase7-sprint2` | Core + P6 + P7 (6 mÃ³dulos) | Beta | AuditorÃ­a + Benchmarks |

---

## 8. Contactos y EscalaciÃ³n

| Rol | Contacto | Responsabilidad |
|---|---|---|
| Release Engineer | `@ed2kia/release-team` | ConsolidaciÃ³n beta, validaciÃ³n cross-phase |
| Security Auditor | `@ed2kia/security-team` | AuditorÃ­a de seguridad, STRIDE |
| Performance Architect | `@ed2kia/perf-team` | Benchmarks, optimizaciÃ³n |
| Phase 8 Lead | `@ed2kia/phase8-team` | Roadmap, backlog, investigaciÃ³n |

---

*Documento generado para v0.7.0-beta. PrÃ³xima revisiÃ³n: v0.8.0-alpha.*
