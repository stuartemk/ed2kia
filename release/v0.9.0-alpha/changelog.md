# Changelog v0.9.0-alpha.1 — Phase 9 Sprint 1

## Fecha
2026-05-04

## Versión
0.9.0-alpha.1

## Feature Flag
`phase9-sprint1`

---

## 🆕 Nuevas Funcionalidades

### 1. Gobernanza Líquida (`src/governance/liquid.rs`)
- **LiquidGovernance**: Motor de gobernanza con delegación ponderada
- `delegate_weight()`: Delegación de voto con cadena de confianza (`trust_score × staking_credits × uptime_history`)
- `cast_vote()`: Votación con resolución automática de cadena de delegación
- `execute_proposal()`: Ejecución con time-lock de 24h y verificación de quórum
- `detect_sybil_cluster()`: Detección anti-Sybil por ASN/IP + historial de votación + trust score
- **GovernanceResult**: `{ executed, quorum_met, delegation_chain, sybil_flag }`
- Umbral de detección Sybil: `cluster_trust < 0.45`
- **Tests**: 22+ tests unitarios

### 2. UI Real-Time (`src/ui/realtime.rs`)
- **RealtimeUIBackend**: Backend WebSocket para eventos en tiempo real
- `upgrade_to_ws()`: Upgrade HTTP → WebSocket vía `axum::extract::ws`
- `broadcast_event()`: Broadcast a sesiones activas con rate limiting
- `sync_state()`: Snapshot de estado por sesión
- `rate_limit_session()`: Rate limiting 50 msg/s por sesión
- **Event Types**: `governance_vote`, `alignment_drift`, `federation_sync`, `slo_breach`, `marketplace_trade`
- Gestión de sesiones con `dashmap` (thread-safe)
- **WsResult**: `{ session_id, messages_sent, rate_limited, active_sessions }`
- **Tests**: 18+ tests unitarios

### 3. Federación Async ZKP (`src/federation/async_zkp.rs`)
- **AsyncZKPFederation**: Batch proof generation con fallback Merkle+VRF
- `batch_proofs()`: Agrupación de 10-50 deltas por batch
- `generate_light_proof()`: ZKP optimizado para SAE forward pass (`ark-ec`/`ark-bn254`)
- `verify_async()`: Verificación asíncrona de batches
- `fallback_to_merkle()`: Fallback automático cuando `gas_used > threshold` o CPU < 4 cores
- **ZKPResult**: `{ proof_hash, verified, fallback_triggered, batch_size }`
- **MerkleProof**: Fallback con VRF nonce para verificación ligera
- **Tests**: 22+ tests unitarios

### 4. Módulo Phase 9 (`src/phase9/mod.rs`)
- Re-exports unificados para Sprint 1
- Metadata: `sprint_identifier()`, `version()`, `enabled_features()`
- Feature flag: `#[cfg(feature = "phase9-sprint1")]`

---

## 🔧 Cambios Técnicos

### Dependencias Nuevas
- `dashmap = "6.0"`: Concurrent HashMap para gestión de sesiones WebSocket
- `axum` features: `"ws"` añadido para soporte WebSocket

### Cargo.toml
- Feature `phase9-sprint1` añadido a `[features]`
- Aislamiento estricto: NO modifica fases anteriores

---

## 📊 Métricas de Calidad

| Métrica | Valor |
|---------|-------|
| Tests Unitarios | 63+ |
| Cobertura de Módulos | 3/3 (100%) |
| Feature Flag Isolation | ✅ |
| Cargo Check | Pendiente |
| Cargo Clippy | Pendiente |
| Cargo Test | Pendiente |

---

## ⚠️ Notas Alpha

- **ZKP Real**: Los proofs actuales son simulaciones SHA-256. Integración real con `ark-ec` circuits pendiente para v0.9.0-beta
- **WebSocket**: `upgrade_to_ws()` requiere servidor Axum activo. Tests unitarios validan lógica sin servidor
- **Rate Limiting**: Implementación basada en ventanas deslizantes de 1s. Optimización pendiente para alta concurrencia
- **Sybil Detection**: Heurística actual usa ASN/IP + voting similarity. Integración con proof-of-personhood pendiente

---

## 📋 Roadmap v0.9.0

| Sprint | Contenido | Estado |
|--------|-----------|--------|
| Sprint 1 | Liquid Governance + Realtime UI + Async ZKP | ✅ Alpha |
| Sprint 2 | Proof-of-Personhood + SSE Streams + ZKP Circuit Real | Pendiente |
| Sprint 3 | Cross-Chain Governance + UI Dashboard + Gas Optimization | Pendiente |
| v0.9.0-beta | Consolidación Sprint 1-2 | Pendiente |
| v0.9.0-rc | Hardening + Security Audit | Pendiente |
| v0.9.0 | Release estable | Pendiente |

---

## 🔗 Enlaces

- [Phase 9 Roadmap](../../phase9/roadmap.md)
- [Phase 9 Backlog](../../phase9/backlog.md)
- [Research Notes](../../phase9/research_notes.md)
- [Sprint 1 Progress](../../phase9/sprint1/progress.md)
- [Architecture v2](../../phase9/sprint1/architecture_v2.md)
