# Integration Matrix — v0.9.0-alpha.1 (Phase 9 Sprint 1)

## Módulos y Dependencias

| Módulo | Dependencias Internas | Dependencias Externas | Feature Flag |
|--------|----------------------|----------------------|--------------|
| `governance/liquid.rs` | Ninguna | `thiserror`, `uuid` | `phase9-sprint1` |
| `ui/realtime.rs` | Ninguna | `axum/ws`, `dashmap`, `serde_json`, `tokio`, `thiserror`, `uuid` | `phase9-sprint1` |
| `federation/async_zkp.rs` | Ninguna | `sha2`, `hex`, `num_cpus`, `thiserror` | `phase9-sprint1` |
| `phase9/mod.rs` | liquid, realtime, async_zkp | Ninguna | `phase9-sprint1` |

## Matriz de Compatibilidad Cross-Phase

| Phase 9 Sprint 1 ↓ / Phase → | Phase 6 | Phase 7 | Phase 8 Sprint 1 | Phase 8 Sprint 2 |
|-------------------------------|---------|---------|-------------------|-------------------|
| **LiquidGovernance** | ✅ Compatible | ✅ Compatible | ✅ Compatible | ✅ Compatible |
| **RealtimeUIBackend** | ✅ Compatible | ✅ Compatible | ⚠️ Coexiste con `ui/backend.rs` | ✅ Compatible |
| **AsyncZKPFederation** | ✅ Compatible | ✅ Compatible | ✅ Compatible | ✅ Compatible |

**Nota**: `ui/realtime.rs` coexiste con `ui/backend.rs` (Phase 8 Sprint 1) bajo feature flags diferentes. No hay conflicto de nombres porque:
- Phase 8 Sprint 1: `#[cfg(feature = "phase8-sprint1")] mod ui { pub mod backend; }`
- Phase 9 Sprint 1: `#[cfg(feature = "phase9-sprint1")] mod ui { pub mod realtime; }`

## Matriz de Feature Flags

| Feature Flag | Incluye | Excluye |
|--------------|---------|---------|
| `default` | Fases 1-5 core | Phase 6-9 |
| `phase6-core` | Interoperability, FedAvg | Phase 7-9 |
| `phase6-sprint2` | Staking, API v2, ONNX | Phase 7-9 |
| `phase7-sprint1` | Alignment Engine, Federation Bridge | Phase 6, 8-9 |
| `phase7-sprint2` | Feedback Loop, Trust Scoring, Schema Registry | Phase 6, 8-9 |
| `phase8-sprint1` | Marketplace, UI Backend, SLO Engine | Phase 6-7, 9 |
| `phase8-sprint2` | Cross-Model Scaling, Continuous Alignment, SLA Enforcer | Phase 6-7, 9 |
| `phase9-sprint1` | Liquid Governance, Realtime UI, Async ZKP | Phase 6-8 |

## Puntos de Integración

### 1. LiquidGovernance ↔ Fase 5 Governance
- **Compatibilidad**: `governance/liquid.rs` extiende `governance/proposal.rs` y `governance/voting.rs`
- **Migración**: Los proposals existentes pueden ser promovidos a liquid governance con delegación
- **Riesgo**: Bajo (módulo independiente, misma namespace)

### 2. RealtimeUIBackend ↔ Phase 8 UI Backend
- **Compatibilidad**: Coexistencia vía feature flags
- **Migración**: `ui/backend.rs` (REST) + `ui/realtime.rs` (WebSocket) = API completa
- **Riesgo**: Medio (requiere axum ws feature)

### 3. AsyncZKPFederation ↔ Fase 3 ZKP
- **Compatibilidad**: `federation/async_zkp.rs` usa `ark-bn254` (ya en dependencies)
- **Migración**: Extiende `zkp/circuit.rs` con batch proofs
- **Riesgo**: Bajo (módulo independiente)

## Validación de Integración

```bash
# Compilación individual por feature
cargo check --features phase9-sprint1
cargo clippy --features phase9-sprint1
cargo test --features phase9-sprint1

# Compilación combinada (future: v1.0.0)
cargo check --features phase6-sprint2,phase7-sprint2,phase8-sprint2,phase9-sprint1
```

## Checklist de Validación

- [ ] `cargo check --features phase9-sprint1` — 0 errores, 0 warnings
- [ ] `cargo clippy --features phase9-sprint1` — 0 errores, 0 warnings
- [ ] `cargo test --features phase9-sprint1` — Todos pasan
- [ ] Tests de integración: `tests/integration/phase9_sprint1_e2e.rs`
- [ ] Documentación completa
- [ ] Changelog actualizado
- [ ] Pipeline CI/CD configurado
