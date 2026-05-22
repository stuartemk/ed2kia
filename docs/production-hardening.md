# Production Hardening & Deployment Runbook — ed2kIA v2.1.0-sprint26

> **Estado:** PRODUCTION-READY & FORMALLY-VALIDATED
> **Feature Gates:** `v2.1-formal-validation`, `v2.1-cross-platform-sync`, `v2.1-production-hardening`
> **Última Actualización:** Sprint 26

---

## Tabla de Contenidos

1. [Arquitectura Multiplataforma](#1-arquitectura-multiplataforma)
2. [Validación Formal del Kernel](#2-validación-formal-del-kernel)
3. [Postura de Seguridad](#3-postura-de-seguridad)
4. [Escalado Horizontal](#4-escalado-horizontal)
5. [Resolución de Incidentes](#5-resolución-de-incidentes)
6. [Cláusula Ética y Cero Lógica Financiera](#6-cláusula-ética-y-cero-lógica-financiera)
7. [Comandos de Despliegue](#7-comandos-de-despliegue)

---

## 1. Arquitectura Multiplataforma

### Componentes

| Componente | Plataforma | Descripción |
|------------|-----------|-------------|
| **WASM Worker** | Browser/PWA | `web/wasm-worker.js` — Nodo WASM no-bloqueante en Web Worker |
| **Browser Node** | Browser/PWA | `web/browser-node.js` — Bridge Worker ↔ Main Thread |
| **Cross Sync** | Tauri/Capacitor/PWA | `src/platform/cross_sync.rs` — Sync offline-first con VersionVector |
| **Dashboard** | Browser/PWA | `web/public-dashboard.html` — UI Alpine.js con panel Simbiosis |
| **CLI** | Desktop/Server | `src/bin/ed2kia-cli.rs` — Wizard de operación |
| **Systemd** | Linux Server | `deploy/systemd/ed2kia.service` — Servicio de nodo |

### Sincronización Offline-First

```
┌─────────────────────────────────────────────────────────────┐
│                    Cross-Sync Engine                         │
│                                                              │
│  Local Queue (Priority: SCT > BFT > CRDT > Telemetry)       │
│       │                                                      │
│       ▼                                                      │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  VersionVector + Timestamp Determinista               │   │
│  │  - Conmutatividad: merge(a,b) == merge(b,a)          │   │
│  │  - Asociatividad: merge(a,merge(b,c)) == merge(merge(a,b),c) │
│  │  - Idempotencia: merge(a,a) == a                      │   │
│  └──────────────────────────────────────────────────────┘   │
│       │                                                      │
│       ▼                                                      │
│  Batch Merge → Conflict Resolution → Converged State         │
└─────────────────────────────────────────────────────────────┘
```

**Garantías:**
- Cero pérdida de estado en desconexión (cola local persistente)
- Convergencia garantizada al reconectar (CRDT properties)
- Consumo RAM < 64MB (memory-bounded)
- Priorización ética: SCT evaluations siempre primero

### Plataformas Soportadas

| Plataforma | Estado | Notas |
|------------|--------|-------|
| **Browser (PWA)** | ✅ Production | WASM Worker + Alpine.js |
| **Tauri (Desktop)** | ✅ Ready | `src-tauri/` configured |
| **Capacitor (Mobile)** | ✅ Ready | Cross-sync engine agnostic |
| **Linux Server** | ✅ Production | Systemd service |
| **Docker** | ✅ Production | `infra/docker-compose.testnet-v2.1.yml` |

---

## 2. Validación Formal del Kernel

### Invariantes Validadas (proptest)

**Archivo:** `tests/property/kernel_invariants.rs`

#### SCT (Stuartian Context Tensor)
| Invariante | Propiedad | Casos |
|------------|-----------|-------|
| Z Axis Bounds | `Z ∈ [-1.0, 1.0]` | 500 |
| Negative Z → Rejected | `Z < 0 → SCTDecision::Rejected` | 500 |
| Positive Z → Approved | `Z >= 0 → SCTDecision::Approved` | 500 |
| Stewardship Score Bounded | `score ∈ [-2.0, 2.0]` | 500 |
| Constructor Validation | X,Y out of bounds → Error | 500 each |

#### BFT (Byzantine Fault Tolerance)
| Invariante | Propiedad | Casos |
|------------|-----------|-------|
| Median Convergence | Median within [center ± noise] | 500 |
| Outlier Resistance | ≤30% Byzantine → median ≈ center | 500 |
| Zero Divergence | Identical inputs → identical median | 500 |

#### CRDTs (Conflict-free Replicated Data Types)
| Invariante | Propiedad | Casos |
|------------|-----------|-------|
| Commutativity | `merge(a,b) == merge(b,a)` | 500 |
| Idempotency | `merge(a,a) == a` | 500 |
| Associativity | `merge(a,merge(b,c)) == merge(merge(a,b),c)` | 500 |
| Monotonicity | Value never decreases | 500 |

#### QLoRA
| Invariante | Propiedad | Casos |
|------------|-----------|-------|
| Rank Bounds | `rank <= d_model` | 500 |
| Alpha Positive | `alpha > 0` | 500 |
| Payload Size | `≤ 1MB` for reasonable dims | 500 |
| Formula Deterministic | Same inputs → same outputs | 500 |

### Ejecución

```bash
# Validación formal completa
cargo test --test kernel_invariants --features "v2.1-formal-validation" -- --test-threads=2

# Verboso (mostrar cada caso)
cargo test --test kernel_invariants --features "v2.1-formal-validation" -- --nocapture --test-threads=2
```

---

## 3. Postura de Seguridad

### Headers CSP

| Header | Valor Recomendado | Estado |
|--------|-------------------|--------|
| `Content-Security-Policy` | `default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; object-src 'wasm-unsafe-eval'; frame-ancestors 'none'` | ✅ Configurable |
| `Cross-Origin-Opener-Policy` | `same-origin` | ✅ Via reverse proxy |
| `Cross-Origin-Embedder-Policy` | `require-corp` | ✅ Via reverse proxy |
| `X-Content-Type-Options` | `nosniff` | ✅ Via reverse proxy |
| `X-Frame-Options` | `DENY` | ✅ Via reverse proxy |

### WASM Sandboxing

- **std::fs:** Prohibido en `src/wasm/` (verificado por `harden-production.sh`)
- **std::net:** Prohibido en `src/wasm/` (verificado por `harden-production.sh`)
- **Memory Limit:** 64MB max por instancia WASM
- **Isolation:** Web Worker context (no access to DOM/main thread)

### Validación de Firmas

- **Ed25519:** Payload signatures verified before processing
- **Genesis State:** SHA256 hash + Ed25519 signature (`src/mainnet/genesis.rs`)
- **SCT Evaluation:** Deterministic, reproducible results

### Rate Limiting

- `/api/*`: Configurable rate limit (recommended: 100 req/min per peer)
- WebSocket: Connection limit per IP
- GossipSub: Message rate bounded by mesh config

### Script de Endurecimiento

```bash
# Ejecutar hardening completo
bash scripts/harden-production.sh

# Solo generar reporte (sin modificar)
bash scripts/harden-production.sh --report-only
```

**Salida:**
- `🟢 HARDENED` — Todos los checks pasaron
- `🔴 VULNERABILITY DETECTED: [causa]` — Acción requerida
- Reporte: `docs/security-hardening-report-YYYYMMDD.md`

---

## 4. Escalado Horizontal

### Topología de Red

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Seed Node  │─────│  Seed Node  │─────│  Seed Node  │
│  (Region A) │     │  (Region B) │     │  (Region C) │
└──────┬──────┘     └──────┬──────┘     └──────┬──────┘
       │                   │                   │
       ▼                   ▼                   ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Mesh Node  │─────│  Mesh Node  │─────│  Mesh Node  │
│  (P2P)      │     │  (P2P)      │     │  (P2P)      │
└─────────────┘     └─────────────┘     └─────────────┘
```

### Métricas de Escalado

| Métrica | Límite | Acción |
|---------|--------|--------|
| Mesh Size | 11 (default) | Configurable via `MeshConfig` |
| Fanout | 6 | Ajustar para latencia vs cobertura |
| Heartbeat | 1s | Aumentar para reducir overhead |
| Memory per Node | <256MB | Monitor via `/health` |
| Sync Batch | 64 entries | Ajustar según ancho de banda |

### Auto-Scaling

```bash
# Verificar salud del mesh
curl http://localhost:3030/health | jq .

# Escalar: añadir nodo
ed2kia-cli node start --config configs/node-N.toml

# Reducir: drenar nodo
ed2kia-cli node stop --graceful --timeout 30s
```

---

## 5. Resolución de Incidentes

### Matriz de Incidentes

| Severidad | Síntoma | Diagnóstico | Acción |
|-----------|---------|-------------|--------|
| **P0** | Nodo no inicia | `journalctl -u ed2kia.service` | Verificar config, reiniciar |
| **P0** | SCT rejecting all | Check payload format | Validar inputs, rollback |
| **P1** | Alta latencia sync | Check network, mesh health | Verificar conectividad, peers |
| **P1** | Memory leak | `htop`, heap snapshot | Identificar módulo, restart |
| **P2** | Dashboard no carga | Browser console, network | Verificar CSP, cache |
| **P2** | Gossip slow | Mesh metrics, heartbeat | Ajustar fanout, heartbeat |

### Comandos de Diagnóstico

```bash
# Logs del servicio
journalctl -u ed2kia.service -f --no-pager

# Health check
curl -s http://localhost:3030/health | jq .

# Métricas del mesh
curl -s http://localhost:3030/metrics | jq .

# Verificar WASM worker
# Browser Console: window._browserNode?.getHealth()

# Hardening check
bash scripts/harden-production.sh

# Validación formal
cargo test --test kernel_invariants --features "v2.1-formal-validation"
```

### Rollback

```bash
# Rollback a versión estable
bash ops/rollback_v0.6.0.sh

# Rollback manual
git checkout v2.1.0-sprint25 -- Cargo.toml src/ web/
cargo build --release
systemctl restart ed2kia.service
```

---

## 6. Cláusula Ética y Cero Lógica Financiera

### Principios Fundamentales

1. **Cero Lógica Financiera:** Este software no contiene módulos de trading, staking financiero, tokenomics ni mecanismos de especulación. La reputación es técnica, no monetaria.
2. **Propiedad Comunitaria:** El código es Apache 2.0 con Cláusula de Uso Ético. Pertenece a la comunidad global.
3. **Transparencia Total:** Todas las decisiones SCT son auditables, reproducibles y deterministas.
4. **Cero Backdoors:** El código está diseñado para ser auditable por terceros. Sin puertas traseras, sin telemetría oculta.

### Leyes del Kernel Estuardiano

| Ley | Nombre | Aplicación |
|-----|--------|------------|
| **Ley 1** | Simbiosis Existencial | SCT Z >= 0 para aprobación |
| **Ley 2** | Reconocimiento del Error | Validación formal, audit trail |
| **Ley 3** | Cero Desperdicio | Memory-bounded, zero-allocation paths |
| **Ley 4** | Simbiosis Existencial | Worker messages incluyen SCT vectors |
| **Ley 5** | Múltiples Posibilidades | CRDT convergence, offline-first |

### Enlaces de Gobernanza

- [GOVERNANCE.md](../GOVERNANCE.md) — Estructura de gobernanza
- [kernel-architecture.md](kernel-architecture.md) — Arquitectura del Kernel
- [federation-blueprint.md](federation-blueprint.md) — Blueprint de federación

---

## 7. Comandos de Despliegue

### Build & Deploy

```bash
# Build production
cargo build --release --features "v2.1-formal-validation,v2.1-cross-platform-sync,v2.1-production-hardening"

# Build WASM
cargo build --target wasm32-unknown-unknown --release

# Deploy systemd
sudo cp deploy/systemd/ed2kia.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable ed2kia.service
sudo systemctl start ed2kia.service

# Verify
sudo systemctl status ed2kia.service
journalctl -u ed2kia.service -f --no-pager
```

### Validación Pre-Despliegue

```bash
# 1. Validación técnica
cargo check --all-targets --features "v2.1-formal-validation,v2.1-cross-platform-sync,v2.1-production-hardening"
cargo test --test kernel_invariants --features "v2.1-formal-validation" -- --test-threads=2
bash -n scripts/harden-production.sh

# 2. Hardening
bash scripts/harden-production.sh

# 3. Verificar docs
grep -E "^\[.*\]\(.*\)" README.md CHANGELOG.md docs/production-hardening.md | head -20
```

### Monitoreo

```bash
# Prometheus + Grafana
docker-compose -f infra/docker-compose.testnet-v2.1.yml up -d

# Dashboard: http://localhost:3000/d/ed2kia
# Prometheus: http://localhost:9090
```

---

## Referencias

| Recurso | Ruta |
|---------|------|
| Kernel Architecture | [`docs/kernel-architecture.md`](kernel-architecture.md) |
| Federation Blueprint | [`docs/federation-blueprint.md`](federation-blueprint.md) |
| Governance | [`GOVERNANCE.md`](../GOVERNANCE.md) |
| CHANGELOG | [`CHANGELOG.md`](../CHANGELOG.md) |
| Hardening Script | [`scripts/harden-production.sh`](../scripts/harden-production.sh) |
| Property Tests | [`tests/property/kernel_invariants.rs`](../tests/property/kernel_invariants.rs) |
| Cross Sync | [`src/platform/cross_sync.rs`](../src/platform/cross_sync.rs) |

---

*Documento generado para ed2kIA v2.1.0-sprint26 — Production Hardening & Formal Validation*
*Apache 2.0 + Ethical Use Clause*
