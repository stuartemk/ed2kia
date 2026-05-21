# ed2kIA — Public Launch Guide (Sprint19)

> **v2.1.0-sprint19** — Lanzamiento Público & Onboarding Comunitario
> 
> Guía operativa de día cero. Técnico, transparente, anti-corporativo, cero hype.

---

## Tabla de Contenidos

1. [Checklist de Lanzamiento](#1-checklist-de-lanzamiento)
2. [Flujo de Onboarding](#2-flujo-de-onboarding)
3. [Resolución de Incidentes](#3-resolución-de-incidentes-comunes)
4. [Procedimientos de Rollback](#4-procedimientos-de-rollback)
5. [Contacto de Stewards](#5-contacto-de-stewards)
6. [Cláusula Ética](#6-cláusula-ética-y-cero-lógica-financiera)
7. [Recursos](#7-recursos)

---

## 1. Checklist de Lanzamiento

### Pre-Launch (T-24h)

- [ ] `cargo check --all-targets` — PASSED
- [ ] `cargo test --lib` — 100% PASSED
- [ ] `scripts/audit-scan.sh` — 🟢 AUDIT READY
- [ ] `scripts/pre-launch-check.sh` — PASSED
- [ ] Docker images built and tagged
- [ ] Bootstrap peers configured (`launch/genesis/seed_nodes.json`)
- [ ] SCTGuard Z-axis enforcement verified
- [ ] BFTAggregator Byzantine tolerance validated
- [ ] Rate-limiting defaults configured (100 req/min)
- [ ] Monitoring activated (Prometheus + Grafana)
- [ ] Rollback plan reviewed (`launch/day1/rollback_plan.md`)

### Launch Day (T=0)

```bash
# Execute launch automation
./scripts/launch-day.sh --profile mainnet --replicas 1

# Dry run first (recommended)
./scripts/launch-day.sh --dry-run
```

**Expected output:**
```
🟢 LAUNCH SUCCESS — ed2kIA v2.1.0-sprint19 is LIVE

  Report: docs/launch-day-report-YYYYMMDD.md
  Dashboard: http://localhost:3000
  API Health: http://localhost:3000/api/health
```

### Post-Launch (T+1h)

- [ ] Verify `docs/launch-day-report-YYYYMMDD.md` generated
- [ ] Check `/api/health` returns HTTP 200
- [ ] Check `/api/metrics` returns valid JSON
- [ ] Check `/api/atlas/stats` returns valid JSON
- [ ] Public dashboard loads (`web/public-dashboard.html`)
- [ ] First community node onboarded via `ed2kia-onboard`
- [ ] SCTGuard blocking Z < 0 payloads
- [ ] BFTAggregator rejecting Byzantine gradients
- [ ] CRDT convergence verified across nodes

---

## 2. Flujo de Onboarding

### Para Nodos Voluntarios

```bash
# 1. Clone repository
git clone https://github.com/Stuartemk/ed2kIA.git
cd ed2kIA

# 2. Build onboarding binary
cargo build --bin ed2kia-onboard --features v2.1-community-onboarding --release

# 3. Run wizard
./target/release/ed2kia-onboard wizard
```

**Wizard flow:**
1. **Environment check** — CPU ≥ 2, RAM ≥ 512MB, network connectivity
2. **Node identity** — Assign unique node name
3. **Role selection** — Relay / Orchestrator / WASM Node / Auditor
4. **Port config** — Default 3000
5. **Config generation** — `ed2kia.conf` with real-time validation
6. **Bootstrap peers** — Connect to seed nodes, sync CRDTs
7. **SCTGuard verification** — Confirm Z-axis active
8. **Merit registration** — Register as Novice (0.5x voting)
9. **Diagnostic export** — `onboarding-diag.json`

**Minimum requirements:**
| Resource | Minimum | Recommended |
|----------|---------|-------------|
| CPU | 2 cores | 4+ cores |
| RAM | 512 MB | 2 GB |
| Disk | 1 GB | 10 GB |
| Network | 1 Mbps | 10 Mbps |
| OS | Linux / macOS / Windows | Linux (Ubuntu 22.04+) |

### Merit Progression

```
Novice (0.5x) → Contributor (1.0x) → Auditor (1.5x) → Steward (2.0x) → Guardian (3.0x)
```

Progression is based on:
- Uptime and reliability
- Quality of contributions (gradient validation, SAE activations)
- Community participation (RFC voting, bug reports)
- Ethical compliance (SCTGuard alignment)

---

## 3. Resolución de Incidentes Comunes

### Incidente: Nodo no se conecta a mesh

**Síntomas:** `Active Peers: 0`, healthcheck falla

**Diagnóstico:**
```bash
# Check bootstrap peers
cat ed2kia.conf | grep bootstrap_peers

# Test connectivity
curl -s http://localhost:3000/api/health

# Check logs
tail -f logs/ed2kia.log | grep -i "peer\|connect\|error"
```

**Resolución:**
1. Verify bootstrap peers are reachable
2. Check firewall rules (port 3000 TCP/UDP open)
3. Restart node: `systemctl restart ed2kia`
4. If persistent, report to stewards con `onboarding-diag.json`

### Incidente: SCTGuard rechaza todos los payloads

**Síntomas:** High rejection rate, Z-axis consistently negative

**Diagnóstico:**
```bash
# Check SCTGuard status
curl -s http://localhost:3000/api/metrics | python3 -m json.tool | grep -A5 sct
```

**Resolución:**
1. Verify model weights are correctly loaded (GGUF + QLoRA)
2. Check SCT core configuration (`sct_guard_enabled = true`)
3. Validate input data format (3D tensor {x, y, z})
4. If model is misaligned, trigger retraining with human feedback

### Incidente: CRDT divergence detectada

**Síntomas:** Version vectors diverging, merge conflicts

**Diagnóstico:**
```bash
# Check CRDT status
curl -s http://localhost:3000/api/metrics | python3 -m json.tool | grep -A5 crdt
```

**Resolución:**
1. Verify network partition status
2. Force CRDT sync: restart affected nodes
3. Check for Byzantine nodes (BFTAggregator logs)
4. Apply slashing if intentional divergence detected

### Incidente: Alta latencia de consenso

**Síntomas:** `consensus_latency_ms > 1000`

**Diagnóstico:**
```bash
# Check network metrics
curl -s http://localhost:3000/api/metrics | python3 -m json.tool
```

**Resolución:**
1. Check mesh size (should be 12-20 peers)
2. Verify BFTAggregator is active
3. Check for network congestion
4. Consider adding more relay nodes

---

## 4. Procedimientos de Rollback

### Rollback Automático

`scripts/launch-day.sh` incluye rollback automático si:
- Healthcheck falla (fase 5)
- Docker compose no inicia (fase 3)
- Cargo check falla (fase 2)

**Comando de rollback manual:**
```bash
# Stop all services
docker compose --profile mainnet -f deploy/docker-compose.yml down

# Verify stopped
docker ps | grep ed2k
```

### Rollback a Sprint Anterior

```bash
# 1. Stop current services
docker compose --profile mainnet -f deploy/docker-compose.yml down

# 2. Checkout previous version
git checkout a9e56f0  # Sprint17

# 3. Rebuild
cargo build --release

# 4. Restart with previous config
docker compose --profile mainnet -f deploy/docker-compose.yml up -d
```

### Rollback Plan Detallado

Ver [`launch/day1/rollback_plan.md`](launch/day1/rollback_plan.md) para:
- Decision tree de rollback
- RTO/RPO targets
- Comunicación a stakeholders
- Post-mortem template

---

## 5. Contacto de Stewards

### Canales de Soporte

| Canal | Uso | Respuesta |
|-------|-----|-----------|
| GitHub Issues | Bugs, RFCs, feature requests | 24-48h |
| Discord #node-operators | Soporte técnico en tiempo real | ~1h |
| Discord #governance | Discusiones de gobernanza | 24h |
| Email: stewards@ed2kIA.org | Reportes de seguridad | 4h (crítico) |

### Escalación

1. **Nivel 1:** Documentación (`docs/`, `README.md`)
2. **Nivel 2:** GitHub Issues (template: `node_operator_issue.md`)
3. **Nivel 3:** Discord #node-operators
4. **Nivel 4:** Steward directo (email)
5. **Nivel 5:** Guardian emergency call (seguridad crítica)

### Reportar Bug de Seguridad

**Proceso:**
1. NO publiques en GitHub públicamente
2. Email a `security@ed2kIA.org` con:
   - Descripción del vulnerability
   - Pasos para reproducir
   - Impacto estimado
   - Sugerencia de fix (opcional)
3. Respuesta en 4h para issues críticos
4. Bug bounty disponible (ver [`SECURITY.md`](SECURITY.md))

---

## 6. Cláusula Ética y Cero Lógica Financiera

### Principios Fundamentales

Este software se distribuye bajo **Apache 2.0 + Ethical Use Clause**:

1. **Transparencia absoluta:** Cero backdoors, código 100% auditable
2. **Cero lógica financiera:** Sin tokens, sin staking financiero, sin mecanismos de especulación
3. **Propiedad comunitaria:** La red pertenece a sus operadores, no a entidades corporativas
4. **Acceso universal:** Funciona en hardware modesto y conexiones inestables
5. **Alineación ética:** SCTGuard Z-axis enforcement obligatorio

### Uso Prohibido

- Armas, vigilancia masiva, discriminación algorítmica
- Manipulación electoral o desinformación a escala
- Explotación laboral o violación de derechos humanos
- Centralización coercitiva o captura de la red

### Mérito Criptográfico

El sistema de mérito (`v2.1-merit-system`) reemplaza los incentivos financieros:
- Basado en contribución verificable, no en capital
- Voting weight refleja compromiso demostrado, no riqueza
- Progresión transparente y auditável
- Cero transferencia de mérito entre nodos

---

## 7. Recursos

### Documentación Técnica

| Recurso | Enlace |
|---------|--------|
| Kernel Architecture | [`docs/kernel-architecture.md`](kernel-architecture.md) |
| Audit Preparation | [`docs/audit-prep.md`](audit-prep.md) |
| Governance | [`GOVERNANCE.md`](../GOVERNANCE.md) |
| Community Onboarding | [`docs/COMMUNITY_ONBOARDING.md`](COMMUNITY_ONBOARDING.md) |
| Threat Model v2.0 | [`security/threat_model_v2.0.md`](../security/threat_model_v2.0.md) |

### Scripts Operativos

| Script | Uso |
|--------|-----|
| `scripts/launch-day.sh` | Automatización de lanzamiento |
| `scripts/audit-scan.sh` | Escaneo pre-auditoría |
| `scripts/validate-node.sh` | Validador de nodos voluntarios |
| `scripts/pre-launch-check.sh` | Checklist pre-lanzamiento |

### Dashboards

| Dashboard | URL |
|-----------|-----|
| Public Observability | `web/public-dashboard.html` |
| Stewardship Dashboard | `web/stewardship-dashboard.html` |
| Atlas Visualizer | `web/atlas.html` |
| Grafana (internal) | `http://localhost:3001` |

### Leyes Estuardianas

| Ley | Principio | Módulo |
|-----|-----------|--------|
| Ley 1 | P2P Sovereignty | GossipSub mesh, partition tolerance |
| Ley 2 | Transparent Audit | SCTGuard, BFTAggregator, slashing |
| Ley 3 | Zero Waste | GGUF + QLoRA, CRDTs |
| Ley 4 | Edge Distribution | WASM micro-sharding, browser node |
| Ley 5 | Multiple Possibilities | CRDT convergence, version vectors |

---

*Guía generada para ed2kIA v2.1.0-sprint19*
*Ley 1: Diversidad Comunitaria · Ley 4: Simbiosis Existencial*
*Cero Lógica Financiera · Propiedad Comunitaria*
