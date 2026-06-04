# Production Hardening & Deployment Runbook â€” ed2kIA v2.1.0-sprint26

> **Estado:** PRODUCTION-READY & FORMALLY-VALIDATED
> **Feature Gates:** `v2.1-formal-validation`, `v2.1-cross-platform-sync`, `v2.1-production-hardening`
> **Ãšltima ActualizaciÃ³n:** Sprint 26

---

## Tabla de Contenidos

1. [Arquitectura Multiplataforma](#1-arquitectura-multiplataforma)
2. [ValidaciÃ³n Formal del Kernel](#2-validaciÃ³n-formal-del-kernel)
3. [Postura de Seguridad](#3-postura-de-seguridad)
4. [Escalado Horizontal](#4-escalado-horizontal)
5. [ResoluciÃ³n de Incidentes](#5-resoluciÃ³n-de-incidentes)
6. [ClÃ¡usula Ã‰tica y Cero LÃ³gica Financiera](#6-clÃ¡usula-Ã©tica-y-cero-lÃ³gica-financiera)
7. [Comandos de Despliegue](#7-comandos-de-despliegue)

---

## 1. Arquitectura Multiplataforma

### Componentes

| Componente | Plataforma | DescripciÃ³n |
|------------|-----------|-------------|
| **WASM Worker** | Browser/PWA | `web/wasm-worker.js` â€” Nodo WASM no-bloqueante en Web Worker |
| **Browser Node** | Browser/PWA | `web/browser-node.js` â€” Bridge Worker â†” Main Thread |
| **Cross Sync** | Tauri/Capacitor/PWA | `src/platform/cross_sync.rs` â€” Sync offline-first con VersionVector |
| **Dashboard** | Browser/PWA | `web/public-dashboard.html` â€” UI Alpine.js con panel Simbiosis |
| **CLI** | Desktop/Server | `src/bin/ed2kia-cli.rs` â€” Wizard de operaciÃ³n |
| **Systemd** | Linux Server | `deploy/systemd/ed2kia.service` â€” Servicio de nodo |

### SincronizaciÃ³n Offline-First

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                    Cross-Sync Engine                         â”‚
â”‚                                                              â”‚
â”‚  Local Queue (Priority: SCT > BFT > CRDT > Telemetry)       â”‚
â”‚       â”‚                                                      â”‚
â”‚       â–¼                                                      â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”‚
â”‚  â”‚  VersionVector + Timestamp Determinista               â”‚   â”‚
â”‚  â”‚  - Conmutatividad: merge(a,b) == merge(b,a)          â”‚   â”‚
â”‚  â”‚  - Asociatividad: merge(a,merge(b,c)) == merge(merge(a,b),c) â”‚
â”‚  â”‚  - Idempotencia: merge(a,a) == a                      â”‚   â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜   â”‚
â”‚       â”‚                                                      â”‚
â”‚       â–¼                                                      â”‚
â”‚  Batch Merge â†’ Conflict Resolution â†’ Converged State         â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

**GarantÃ­as:**
- Cero pÃ©rdida de estado en desconexiÃ³n (cola local persistente)
- Convergencia garantizada al reconectar (CRDT properties)
- Consumo RAM < 64MB (memory-bounded)
- PriorizaciÃ³n Ã©tica: SCT evaluations siempre primero

### Plataformas Soportadas

| Plataforma | Estado | Notas |
|------------|--------|-------|
| **Browser (PWA)** | âœ… Production | WASM Worker + Alpine.js |
| **Tauri (Desktop)** | âœ… Ready | `src-tauri/` configured |
| **Capacitor (Mobile)** | âœ… Ready | Cross-sync engine agnostic |
| **Linux Server** | âœ… Production | Systemd service |
| **Docker** | âœ… Production | `infra/docker-compose.testnet-v2.1.yml` |

---

## 2. ValidaciÃ³n Formal del Kernel

### Invariantes Validadas (proptest)

**Archivo:** `tests/property/kernel_invariants.rs`

#### SCT (Topological Context Tensor)
| Invariante | Propiedad | Casos |
|------------|-----------|-------|
| Z Axis Bounds | `Z âˆˆ [-1.0, 1.0]` | 500 |
| Negative Z â†’ Rejected | `Z < 0 â†’ SCTDecision::Rejected` | 500 |
| Positive Z â†’ Approved | `Z >= 0 â†’ SCTDecision::Approved` | 500 |
| Stewardship Score Bounded | `score âˆˆ [-2.0, 2.0]` | 500 |
| Constructor Validation | X,Y out of bounds â†’ Error | 500 each |

#### BFT (Byzantine Fault Tolerance)
| Invariante | Propiedad | Casos |
|------------|-----------|-------|
| Median Convergence | Median within [center Â± noise] | 500 |
| Outlier Resistance | â‰¤30% Byzantine â†’ median â‰ˆ center | 500 |
| Zero Divergence | Identical inputs â†’ identical median | 500 |

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
| Payload Size | `â‰¤ 1MB` for reasonable dims | 500 |
| Formula Deterministic | Same inputs â†’ same outputs | 500 |

### EjecuciÃ³n

```bash
# ValidaciÃ³n formal completa
cargo test --test kernel_invariants --features "v2.1-formal-validation" -- --test-threads=2

# Verboso (mostrar cada caso)
cargo test --test kernel_invariants --features "v2.1-formal-validation" -- --nocapture --test-threads=2
```

---

## 3. Postura de Seguridad

### Headers CSP

| Header | Valor Recomendado | Estado |
|--------|-------------------|--------|
| `Content-Security-Policy` | `default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; object-src 'wasm-unsafe-eval'; frame-ancestors 'none'` | âœ… Configurable |
| `Cross-Origin-Opener-Policy` | `same-origin` | âœ… Via reverse proxy |
| `Cross-Origin-Embedder-Policy` | `require-corp` | âœ… Via reverse proxy |
| `X-Content-Type-Options` | `nosniff` | âœ… Via reverse proxy |
| `X-Frame-Options` | `DENY` | âœ… Via reverse proxy |

### WASM Sandboxing

- **std::fs:** Prohibido en `src/wasm/` (verificado por `harden-production.sh`)
- **std::net:** Prohibido en `src/wasm/` (verificado por `harden-production.sh`)
- **Memory Limit:** 64MB max por instancia WASM
- **Isolation:** Web Worker context (no access to DOM/main thread)

### ValidaciÃ³n de Firmas

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
- `ðŸŸ¢ HARDENED` â€” Todos los checks pasaron
- `ðŸ”´ VULNERABILITY DETECTED: [causa]` â€” AcciÃ³n requerida
- Reporte: `docs/security-hardening-report-YYYYMMDD.md`

---

## 4. Escalado Horizontal

### TopologÃ­a de Red

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”     â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”     â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚  Seed Node  â”‚â”€â”€â”€â”€â”€â”‚  Seed Node  â”‚â”€â”€â”€â”€â”€â”‚  Seed Node  â”‚
â”‚  (Region A) â”‚     â”‚  (Region B) â”‚     â”‚  (Region C) â”‚
â””â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”˜     â””â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”˜     â””â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”˜
       â”‚                   â”‚                   â”‚
       â–¼                   â–¼                   â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”     â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”     â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚  Mesh Node  â”‚â”€â”€â”€â”€â”€â”‚  Mesh Node  â”‚â”€â”€â”€â”€â”€â”‚  Mesh Node  â”‚
â”‚  (P2P)      â”‚     â”‚  (P2P)      â”‚     â”‚  (P2P)      â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜     â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜     â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

### MÃ©tricas de Escalado

| MÃ©trica | LÃ­mite | AcciÃ³n |
|---------|--------|--------|
| Mesh Size | 11 (default) | Configurable via `MeshConfig` |
| Fanout | 6 | Ajustar para latencia vs cobertura |
| Heartbeat | 1s | Aumentar para reducir overhead |
| Memory per Node | <256MB | Monitor via `/health` |
| Sync Batch | 64 entries | Ajustar segÃºn ancho de banda |

### Auto-Scaling

```bash
# Verificar salud del mesh
curl http://localhost:3030/health | jq .

# Escalar: aÃ±adir nodo
ed2kia-cli node start --config configs/node-N.toml

# Reducir: drenar nodo
ed2kia-cli node stop --graceful --timeout 30s
```

---

## 5. ResoluciÃ³n de Incidentes

### Matriz de Incidentes

| Severidad | SÃ­ntoma | DiagnÃ³stico | AcciÃ³n |
|-----------|---------|-------------|--------|
| **P0** | Nodo no inicia | `journalctl -u ed2kia.service` | Verificar config, reiniciar |
| **P0** | SCT rejecting all | Check payload format | Validar inputs, rollback |
| **P1** | Alta latencia sync | Check network, mesh health | Verificar conectividad, peers |
| **P1** | Memory leak | `htop`, heap snapshot | Identificar mÃ³dulo, restart |
| **P2** | Dashboard no carga | Browser console, network | Verificar CSP, cache |
| **P2** | Gossip slow | Mesh metrics, heartbeat | Ajustar fanout, heartbeat |

### Comandos de DiagnÃ³stico

```bash
# Logs del servicio
journalctl -u ed2kia.service -f --no-pager

# Health check
curl -s http://localhost:3030/health | jq .

# MÃ©tricas del mesh
curl -s http://localhost:3030/metrics | jq .

# Verificar WASM worker
# Browser Console: window._browserNode?.getHealth()

# Hardening check
bash scripts/harden-production.sh

# ValidaciÃ³n formal
cargo test --test kernel_invariants --features "v2.1-formal-validation"
```

### Rollback

```bash
# Rollback a versiÃ³n estable
bash ops/rollback_v0.6.0.sh

# Rollback manual
git checkout v2.1.0-sprint25 -- Cargo.toml src/ web/
cargo build --release
systemctl restart ed2kia.service
```

---

## 6. ClÃ¡usula Ã‰tica y Cero LÃ³gica Financiera

### Principios Fundamentales

1. **Cero LÃ³gica Financiera:** Este software no contiene mÃ³dulos de trading, staking financiero, tokenomics ni mecanismos de especulaciÃ³n. La reputaciÃ³n es tÃ©cnica, no monetaria.
2. **Propiedad Comunitaria:** El cÃ³digo es Apache 2.0 con ClÃ¡usula de Uso Ã‰tico. Pertenece a la comunidad global.
3. **Transparencia Total:** Todas las decisiones SCT son auditables, reproducibles y deterministas.
4. **Cero Backdoors:** El cÃ³digo estÃ¡ diseÃ±ado para ser auditable por terceros. Sin puertas traseras, sin telemetrÃ­a oculta.

### Leyes del Kernel Estuardiano

| Ley | Nombre | AplicaciÃ³n |
|-----|--------|------------|
| **Ley 1** | Simbiosis Existencial | SCT Z >= 0 para aprobaciÃ³n |
| **Ley 2** | Reconocimiento del Error | ValidaciÃ³n formal, audit trail |
| **Ley 3** | Cero Desperdicio | Memory-bounded, zero-allocation paths |
| **Ley 4** | Simbiosis Existencial | Worker messages incluyen SCT vectors |
| **Ley 5** | MÃºltiples Posibilidades | CRDT convergence, offline-first |

### Enlaces de Gobernanza

- [GOVERNANCE.md](../GOVERNANCE.md) â€” Estructura de gobernanza
- [kernel-architecture.md](kernel-architecture.md) â€” Arquitectura del Kernel
- [federation-blueprint.md](federation-blueprint.md) â€” Blueprint de federaciÃ³n

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

### ValidaciÃ³n Pre-Despliegue

```bash
# 1. ValidaciÃ³n tÃ©cnica
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

*Documento generado para ed2kIA v2.1.0-sprint26 â€” Production Hardening & Formal Validation*
*Apache 2.0 + Ethical Use Clause*
