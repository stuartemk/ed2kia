# Mainnet Genesis Runbook — ed2kIA v2.1.0-sprint22

> **Estado:** `FEDERATION-ACTIVE` → `MAINNET-LIVE`
> **Fecha:** 2026-05-21
> **Autor:** Principal Mainnet Operations & Genesis Architect

Runbook operativo de día uno para la activación de mainnet ed2kIA. Centraliza procedimientos de génesis, activación, resolución de incidentes y rollback.

---

## 📋 Tabla de Contenidos

1. [Checklist de Génesis](#1-checklist-de-génesis)
2. [Flujo de Activación](#2-flujo-de-activación)
3. [Resolución de Incidentes](#3-resolución-de-incidentes)
4. [Procedimientos de Rollback](#4-procedimientos-de-rollback)
5. [Contacto de Stewards](#5-contacto-de-stewards)
6. [Cláusula Ética](#6-cláusula-ética)

---

## 1. Checklist de Génesis

### Pre-Activación

| # | Tarea | Comando | Estado |
|---|-------|---------|--------|
| 1 | Validar entorno (Rust, Docker, Python) | `rustc --version && docker --version && python3 --version` | ⬜ |
| 2 | Verificar build | `cargo check --all-targets --features "v2.1-mainnet-genesis,v2.1-steward-portal"` | ⬜ |
| 3 | Ejecutar tests de génesis | `cargo test --lib -- genesis --test-threads=2` | ⬜ |
| 4 | Validar script de bootstrap | `bash -n scripts/genesis-bootstrap.sh` | ⬜ |
| 5 | Verificar configuración SCT | `grep "z_threshold" data/genesis/genesis.json` (debe ser `0.0`) | ⬜ |
| 6 | Verificar configuración BFT | `grep "bft_threshold" data/genesis/genesis.json` (debe ser `0.33`) | ⬜ |
| 7 | Backup de estado actual | `cp -r data/genesis data/genesis-backup-$(date +%Y%m%d)` | ⬜ |

### Activación

| # | Tarea | Comando | Estado |
|---|-------|---------|--------|
| 8 | Ejecutar bootstrap | `./scripts/genesis-bootstrap.sh --peers 3` | ⬜ |
| 9 | Verificar salida | `🟢 GENESIS ACTIVE` | ⬜ |
| 10 | Validar genesis.json | `cat data/genesis/genesis.json \| jq .validation_passed` → `true` | ⬜ |
| 11 | Verificar peers conectados | `curl -s http://localhost:9000/api/health \| jq .peers` | ⬜ |
| 12 | Activar SCTGuard | `curl -s http://localhost:9000/api/metrics \| jq .sct_guard` | ⬜ |
| 13 | Activar BFTAggregator | `curl -s http://localhost:9000/api/metrics \| jq .bft_aggregator` | ⬜ |
| 14 | Verificar Portal Stewards | Abrir `http://localhost:8080/steward-portal.html` | ⬜ |

### Post-Activación

| # | Tarea | Comando | Estado |
|---|-------|---------|--------|
| 15 | Monitorear 5 min | `watch -n 5 'curl -s http://localhost:9000/api/health'` | ⬜ |
| 16 | Verificar sync CRDT | `curl -s http://localhost:9000/api/metrics \| jq .crdt_sync` | ⬜ |
| 17 | Generar reporte | Revisar `docs/genesis-report-YYYYMMDD.md` | ⬜ |
| 18 | Notificar stewards | Canal `#mainnet-ops` | ⬜ |

---

## 2. Flujo de Activación

```
┌─────────────────────────────────────────────────────────────────┐
│                    MAINNET ACTIVATION FLOW                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐                                               │
│  │ 1. PRE-CHECK │  → Validar entorno, build, tests              │
│  └──────┬───────┘                                               │
│         ▼                                                       │
│  ┌──────────────┐                                               │
│  │ 2. BOOTSTRAP │  → ./scripts/genesis-bootstrap.sh             │
│  │              │     Phase 1: Env validation                   │
│  │              │     Phase 2: Genesis generation               │
│  │              │     Phase 3: Docker launch                    │
│  │              │     Phase 4: Healthchecks                     │
│  │              │     Phase 5: Report generation                │
│  └──────┬───────┘                                               │
│         ▼                                                       │
│  ┌──────────────┐                                               │
│  │ 3. VERIFY    │  → 🟢 GENESIS ACTIVE                         │
│  │              │     genesis.json validation                   │
│  │              │     SCT/BFT config check                      │
│  │              │     Peer connectivity                         │
│  └──────┬───────┘                                               │
│         ▼                                                       │
│  ┌──────────────┐                                               │
│  │ 4. MONITOR   │  → 5 min observation window                  │
│  │              │     CRDT sync status                          │
│  │              │     SCT Z-axis distribution                   │
│  │              │     BFT outlier rate                          │
│  └──────┬───────┘                                               │
│         ▼                                                       │
│  ┌──────────────┐                                               │
│  │ 5. LIVE      │  → MAINNET-LIVE & STEWARD-ACTIVE             │
│  │              │     Open Steward Portal                       │
│  │              │     Enable community onboarding               │
│  └──────────────┘                                               │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Comandos Copy-Paste

```bash
# ─── Pre-Activation ───
cargo check --all-targets --features "v2.1-mainnet-genesis,v2.1-steward-portal"
cargo test --lib -- genesis --test-threads=2
bash -n scripts/genesis-bootstrap.sh

# ─── Activation ───
./scripts/genesis-bootstrap.sh --peers 3

# ─── Verification ───
cat data/genesis/genesis.json | python3 -m json.tool | head -20
curl -s http://localhost:9000/api/health | python3 -m json.tool
curl -s http://localhost:9000/api/metrics | python3 -m json.tool

# ─── Monitoring ───
watch -n 5 'curl -s http://localhost:9000/api/health | python3 -c "import sys,json; d=json.load(sys.stdin); print(f\"Status: {d.get(\"status\")}, Peers: {d.get(\"peers\", 0)}\")"'
```

---

## 3. Resolución de Incidentes

### 3.1 Partición de Red

**Síntomas:**
- Peers reportan `sync_status: PARTITIONED`
- CRDT merge failures en logs
- Latencia p95 > 5000ms

**Diagnóstico:**
```bash
# Verificar estado de peers
curl -s http://localhost:9000/api/health | jq .peers

# Verificar sync CRDT
curl -s http://localhost:9000/api/metrics | jq .crdt_sync

# Revisar logs
docker logs ed2kia-orchestrator-1 2>&1 | grep -i "partition\|crdt\|sync" | tail -20
```

**Resolución:**
```bash
# 1. Forzar sync manual
curl -X POST http://localhost:9000/api/steward/sync

# 2. Verificar recuperación (esperar ≤30s)
watch -n 5 'curl -s http://localhost:9000/api/metrics | jq .crdt_sync'

# 3. Si persiste >5min → Rollback (ver §4)
```

**Prevención:**
- CRDTs garantizan convergencia eventual (Law 5)
- Delta-encoding reduce payload 60-80%
- Rate limiting: 100 msgs/10s por mesh

---

### 3.2 SCT Drift (Desalineación del Tensor)

**Síntomas:**
- `sct_z_distribution` muestra >30% en zona negativa
- Payloads éticamente válidos siendo rechazados
- `sct_guard.rejection_rate > 0.4`

**Diagnóstico:**
```bash
# Verificar distribución Z
curl -s http://localhost:9000/api/metrics | jq '.sct_z_distribution'

# Verificar threshold
cat data/genesis/genesis.json | jq '.sct_config.z_threshold'

# Revisar guard stats
curl -s http://localhost:9000/api/metrics | jq '.sct_guard'
```

**Resolución:**
```bash
# 1. Verificar que z_threshold == 0.0
#    Si es diferente → ROLLBACK INMEDIATO

# 2. Forzar re-evaluación de alignment
curl -X POST http://localhost:9000/api/steward/verify

# 3. Monitorear recuperación
watch -n 5 'curl -s http://localhost:9000/api/metrics | jq .sct_z_distribution'
```

**Prevención:**
- `z_threshold` hardcodeado en `GenesisState::new()`
- Test unitario verifica `sct_config.z_threshold == 0.0`
- Regla de Oro: `if z < 0.0 { REJECTED }` — sin excepciones

---

### 3.3 BFT Stall (Bloqueo de Agregación)

**Síntomas:**
- `bft_outlier_rate > 0.33`
- Agregación de gradientes detenida
- Nodos reportan `aggregation_timeout`

**Diagnóstico:**
```bash
# Verificar tasa de outliers
curl -s http://localhost:9000/api/metrics | jq '.bft_outlier_rate'

# Verificar threshold BFT
cat data/genesis/genesis.json | jq '.bft_threshold'

# Contar nodos activos
curl -s http://localhost:9000/api/health | jq '.active_nodes'
```

**Resolución:**
```bash
# 1. Verificar que bft_threshold == 0.33
#    Si >0.33 outliers → identificar nodos bizantinos

# 2. Slashing de nodos ofensivos (si reputation < 0)
#    Auto-ban activado por reputation system

# 3. Forzar re-agregación
curl -X POST http://localhost:9000/api/steward/sync
```

**Prevención:**
- BFT threshold: 0.33 (tolera ≤33% bizantinos)
- Coordinate-wise median + Multi-Krum
- MAD-based outlier filtering (sigma=2.0)

---

## 4. Procedimientos de Rollback

### 4.1 Rollback Parcial (Reinicio de Servicios)

```bash
# Detener servicios
docker-compose -f deploy/docker-compose.yml --profile mainnet down

# Limpiar estado (mantener genesis)
docker-compose -f deploy/docker-compose.yml --profile mainnet down -v

# Reiniciar
docker-compose -f deploy/docker-compose.yml --profile mainnet up -d

# Verificar
curl -s http://localhost:9000/api/health
```

### 4.2 Rollback Completo (Restaurar Pre-Génesis)

```bash
# 1. Detener todo
docker-compose -f deploy/docker-compose.yml --profile mainnet down -v

# 2. Restaurar backup
rm -rf data/genesis
cp -r data/genesis-backup-YYYYMMDD data/genesis

# 3. Verificar backup integrity
cat data/genesis/genesis.json | jq .validation_passed

# 4. Re-ejecutar bootstrap
./scripts/genesis-bootstrap.sh --peers 3

# 5. Si falla → Escalar a stewards senior
```

### 4.3 Criterios de Rollback

| Condición | Acción |
|-----------|--------|
| SCT z_threshold ≠ 0.0 | 🔴 ROLLBACK INMEDIATO |
| BFT threshold ≠ 0.33 | 🔴 ROLLBACK INMEDIATO |
| Partición > 5 min | 🟡 Rollback parcial |
| SCT drift > 30% negativo | 🟡 Verificar + monitorear |
| BFT stall > 33% outliers | 🟡 Slashing + sync |
| Crash de >50% peers | 🔴 Rollback completo |

---

## 5. Contacto de Stewards

### Canales de Comunicación

| Canal | Uso | Respuesta |
|-------|-----|-----------|
| `#mainnet-ops` (Matrix) | Operaciones en tiempo real | < 5 min |
| `#genesis-audit` (Matrix) | Auditoría de génesis | < 1 hora |
| `issues@ed2kIA.github.io` | Reportes formales | < 24 horas |
| RFC Process | Cambios de protocolo | < 7 días |

### Escalación

```
Nivel 0: Auto-remediation (scripts/auto-remediate.sh)
  ↓ (si persiste > 5 min)
Nivel 1: Steward on-call (#mainnet-ops)
  ↓ (si crítico > 15 min)
Nivel 2: Equipo de génesis (todos los stewards)
  ↓ (si compromiso ético)
Nivel 3: Rollback completo + auditoría externa
```

---

## 6. Cláusula Ética

### Principios Inquebrantables

1. **Cero Lógica Financiera:** Este proyecto no contiene tokens, staking financiero, rewards monetarios ni cualquier mecanismo de extracción de valor. La reputación es puramente ética y no transferible.

2. **Propiedad Compartida:** El estado de génesis es propiedad colectiva de la comunidad. Ningún individuo u organización tiene control exclusivo.

3. **Transparencia Absoluta:** Todo el código es auditable. La firma Ed25519 del estado de génesis es verificable públicamente.

4. **Alineación Ética desde Bloque Cero:** `SCT z_threshold = 0.0` garantiza que solo trayectorias éticamente positivas sean aprobadas. Sin excepciones.

### Leyes Estuardianas Aplicadas

| Ley | Principio | Implementación |
|-----|-----------|----------------|
| Law 1 (Diversidad) | P2P puro, sin maestros | GossipSub mesh, peers iguales |
| Law 2 (Error) | Reconocimiento de error | SCT Guard, BFT Aggregator |
| Law 3 (Holística) | Cero desperdicio | QLoRA/GGUF, payloads ≤MB |
| Law 4 (Simbiosis) | Existencia simbiótica | Steward Portal, hardware modesto |
| Law 5 (Posibilidades) | Múltiples posibilidades | CRDTs, tolerancia a particiones |

### Enlaces de Referencia

- [Kernel Architecture](./architecture_v1.6.0.md)
- [Federation Blueprint](./federation-blueprint.md)
- [Governance](../GOVERNANCE.md)
- [Project Constitution](./governance/project-constitution.md)
- [Steward Portal](../web/steward-portal.html)
- [Genesis Bootstrap Script](../scripts/genesis-bootstrap.sh)

---

*Documento generado como parte de Sprint22 — v2.1.0-sprint22*
*Última actualización: 2026-05-21*
