# Plan de Consolidación v1.0.0-stable

## Resumen

Este documento describe la estrategia de consolidación progresiva de todas las fases (6-9) en un único release estable `v1.0.0`. El objetivo es unificar los feature flags, eliminar código duplicado y proporcionar una API coherente sin downtime.

## Estrategia: Migración Progresiva sin Downtime

### Fase A: Auditoría y Compatibilidad (Semanas 1-2)

1. **Inventario de Feature Flags**
   ```
   phase6-core, phase6-sprint2, phase6-experimental
   phase7-sprint1, phase7-sprint2
   phase8-sprint1, phase8-sprint2
   phase9-sprint1
   ```

2. **Matriz de Conflictos**
   | Conflicto | Resolución |
   |-----------|-----------|
   | `mod ui` en phase8-sprint1 y phase9-sprint1 | Merge: `ui/backend.rs` + `ui/realtime.rs` |
   | `mod slo` en phase8-sprint1 y phase8-sprint2 | Merge: `slo/engine.rs` + `slo/enforcer.rs` |
   | `mod federation` en phase6 y phase9 | Merge: `federation/avg_aggregator.rs` + `federation/async_zkp.rs` |
   | `mod governance` en phase6-experimental y phase9 | Merge: `governance/proposal.rs` + `governance/liquid.rs` |

3. **Análisis de Dependencias Cruzadas**
   - Verificar que ningún módulo de Phase 9 dependa de módulos de Phase 6-8
   - Validar que las firmas de API sean compatibles

### Fase B: Unificación de Módulos (Semanas 3-5)

1. **Merge de Módulos Duplicados**
   ```rust
   // src/ui/mod.rs (unificado)
   pub mod backend;    // Phase 8 Sprint 1: REST API
   pub mod realtime;   // Phase 9 Sprint 1: WebSocket
   
   // src/federation/mod.rs (unificado)
   pub mod avg_aggregator;  // Phase 6: FedAvg
   pub mod sync_protocol;   // Phase 6: P2P Sync
   pub mod async_zkp;       // Phase 9: ZKP Federation
   pub mod bridge;          // Phase 7: Federation Bridge
   pub mod trust_scoring;   // Phase 7: Dynamic Trust
   
   // src/governance/mod.rs (unificado)
   pub mod proposal;    // Phase 5: Basic proposals
   pub mod voting;      // Phase 5: Basic voting
   pub mod liquid;      // Phase 9: Liquid democracy
   
   // src/slo/mod.rs (unificado)
   pub mod engine;      // Phase 8 Sprint 1: SLO tracking
   pub mod enforcer;    // Phase 8 Sprint 2: SLA enforcement
   ```

2. **Eliminación de Feature Flags**
   - Crear feature flag unificado: `full` (incluye todo)
   - Deprecar flags individuales con warnings
   - Mantener flags individuales durante 2 releases para compatibilidad

3. **Consolidación de `src/main.rs`**
   ```rust
   // ANTES (fragmentado)
   #[cfg(feature = "phase8-sprint1")]
   mod ui { pub mod backend; }
   
   #[cfg(feature = "phase9-sprint1")]
   mod ui { pub mod realtime; }
   
   // DESPUÉS (unificado)
   mod ui {
       pub mod backend;
       pub mod realtime;
   }
   ```

### Fase C: Validación y Hardening (Semanas 6-7)

1. **Suite de Tests de Regresión**
   - Ejecutar todos los tests sin feature flags
   - Validar compatibilidad backward con APIs existentes
   - Tests de integración cross-phase

2. **Benchmark de Performance**
   - Medir overhead de unificación
   - Validar que no haya regresiones >5%

3. **Security Audit**
   - Revisar superficie de ataque expandida
   - Validar que los módulos de seguridad (wasm_sandbox, memory_guard) funcionen con todos los módulos activos

### Fase D: Release v1.0.0 (Semana 8)

1. **Pre-Release**
   - `v1.0.0-rc.1`: Feature freeze, testing intensivo
   - `v1.0.0-rc.2`: Bug fixes, documentation final
   - `v1.0.0`: Release estable

2. **Deprecación de Feature Flags**
   ```toml
   [features]
   default = ["full"]
   full = []  # Incluye todo
   # Legacy flags (deprecated, removed in v1.1.0)
   phase6-core = ["full"]
   phase6-sprint2 = ["full"]
   phase7-sprint1 = ["full"]
   phase7-sprint2 = ["full"]
   phase8-sprint1 = ["full"]
   phase8-sprint2 = ["full"]
   phase9-sprint1 = ["full"]
   ```

3. **Documentación**
   - Migration guide: v0.9.0 → v1.0.0
   - API reference unificada
   - Architecture diagram actualizado

## Timeline

| Semana | Actividad | Deliverable |
|--------|-----------|-------------|
| 1 | Auditoría de feature flags | Matriz de conflictos |
| 2 | Análisis de dependencias cruzadas | Reporte de compatibilidad |
| 3 | Merge de módulos `ui/` | `src/ui/mod.rs` unificado |
| 4 | Merge de módulos `federation/` | `src/federation/mod.rs` unificado |
| 5 | Merge de módulos `governance/` + `slo/` | Módulos unificados |
| 6 | Suite de tests de regresión | 100% tests passing sin flags |
| 7 | Security audit + benchmarks | Reporte de seguridad |
| 8 | Release v1.0.0 | Binarios + docs + changelog |

## Riesgos y Mitigación

| Riesgo | Probabilidad | Impacto | Mitigación |
|--------|-------------|---------|-----------|
| Conflictos de namespace en módulos merge | Alta | Medio | Plan de merge detallado por módulo |
| Regresiones de performance | Media | Alto | Benchmarks pre/post merge |
| Breaking changes en API pública | Media | Alto | Semantic versioning + migration guide |
| Feature flags legacy rompen build | Baja | Bajo | Tests de compatibilidad backward |

## Criterios de Éxito

- [ ] `cargo build` sin feature flags compila todo
- [ ] `cargo test` sin feature flags: 100% passing
- [ ] `cargo clippy`: 0 warnings
- [ ] Zero breaking changes en API pública
- [ ] Documentación completa y actualizada
- [ ] Security audit sin findings críticos
- [ ] Benchmarks dentro de ±5% del baseline
