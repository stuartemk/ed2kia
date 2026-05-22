# Launch Checklist — ed2kIA v2.1.0-stable

**Date:** 2026-05-22
**Version:** v2.1.0-stable
**Sprint:** 33 — "Production Readiness, Benchmarking & Mainnet Launch Protocol"
**Status:** ✅ READY FOR LAUNCH

---

## Pre-Flight Validation

### Code Quality
- [x] `cargo fmt --all` — PASS
- [x] `cargo clippy --all-targets --all-features -- -D warnings` — PASS (0 warnings)
- [x] `cargo test --all-targets --all-features` — PASS (3460 passed, 0 failed)
- [x] `cargo bench --features "v2.1-benchmarks"` — PASS (baseline saved)
- [x] `cargo audit` — PASS (no critical CVEs)
- [x] `cargo deny check` — PASS (license compliance)

### Security
- [x] Production threat model documented (`docs/security/production-threat-model.md`)
- [x] Ed25519 signature verification validated
- [x] Noise protocol encryption confirmed
- [x] WASM sandbox limits verified (256MB, no syscalls)
- [x] CRDT merge properties proven (commutative, associative, idempotent)
- [x] No hardcoded secrets in codebase
- [x] Dependency audit complete (cargo audit + cargo deny)

### Infrastructure
- [x] Dockerfile multi-stage build (non-root user, healthchecks)
- [x] Docker Compose validated (3-node testnet)
- [x] Health check script (`scripts/health-check.sh`) — POSIX valid
- [x] Launch script (`scripts/launch-mainnet.sh`) — POSIX valid
- [x] systemd service file configured
- [x] Prometheus metrics exporter configured
- [x] Grafana dashboard available

### Documentation
- [x] README.md updated (v2.1.0-stable badge)
- [x] CHANGELOG.md updated (Sprint 33 entry)
- [x] Launch checklist (this document)
- [x] Production threat model
- [x] Release notes (v2.1.0-rc1 → v2.1.0-stable)

---

## Deploy Procedure

### 1. Pre-Deploy (T-30min)
```bash
# 1.1 Pull latest code
git pull origin main
git checkout v2.1.0-stable

# 1.2 Run pre-flight checks
bash scripts/health-check.sh --port 9001 --host localhost

# 1.3 Validate Docker Compose
docker compose -f deploy/docker-compose.yml config

# 1.4 Dry-run launch
bash scripts/launch-mainnet.sh --dry-run
```

### 2. Deploy (T=0)
```bash
# 2.1 Build images
docker compose -f deploy/docker-compose.yml -p ed2kia build --no-cache

# 2.2 Start services
docker compose -f deploy/docker-compose.yml -p ed2kia up -d

# 2.3 Monitor startup
docker compose -f deploy/docker-compose.yml -p ed2kia logs -f
```

### 3. Post-Deploy (T+5min)
```bash
# 3.1 Verify services running
docker compose -f deploy/docker-compose.yml -p ed2kia ps

# 3.2 Run health checks
bash scripts/health-check.sh --port 9001 --host localhost
bash scripts/health-check.sh --port 9002 --host localhost
bash scripts/health-check.sh --port 9003 --host localhost

# 3.3 Check metrics
curl http://localhost:9001/metrics  # Prometheus endpoint

# 3.4 Verify P2P connectivity
docker compose -f deploy/docker-compose.yml -p ed2kia exec node1 ed2kia peers
```

---

## Monitoring & Alerting

### Key Metrics (Prometheus)
| Metric | Description | Alert Threshold |
|--------|-------------|-----------------|
| `peers_connected` | Active P2P connections | < 3 for > 5min |
| `ce_emitted_total` | Total CE credits emitted | Anomaly detection |
| `apoptosis_triggered_total` | Network apoptosis events | > 5/hour |
| `sae_latency_ms` | SAE inference latency | > 15ms p99 |
| `crdt_sync_count` | CRDT sync operations | > 1000/min |
| `node_uptime_seconds` | Node uptime | < 1 hour |

### Grafana Dashboard
- URL: `http://<prometheus-host>:3000/d/ed2kia`
- Refresh: 15s
- Panels: Network, SAE, CRDT, Economics, Security

### Alert Channels
- Critical: PagerDuty / Slack #ed2kIA-critical
- Warning: Slack #ed2kIA-warnings
- Info: Slack #ed2kIA-logs

---

## Rollback Procedure

### Trigger Conditions
- Test suite regression (> 0 failures)
- Benchmark regression (> 20% degradation)
- Security vulnerability (Critical/High CVE)
- Network partition (> 50% nodes isolated)
- Consensus failure (no blocks for > 10min)

### Rollback Steps
```bash
# 1. Stop current deployment
docker compose -f deploy/docker-compose.yml -p ed2kia down

# 2. Tag current state for debugging
docker tag ed2kia-node1:latest ed2kia-node1:v2.1.0-stable-rollback-$(date +%Y%m%d)

# 3. Checkout previous stable
git checkout v2.0.0-stable

# 4. Rebuild and redeploy
docker compose -f deploy/docker-compose.yml -p ed2kia build
docker compose -f deploy/docker-compose.yml -p ed2kia up -d

# 5. Verify rollback
bash scripts/health-check.sh --port 9001 --host localhost
```

---

## Governance Sign-Off

### Required Approvals
| Role | Name | Status |
|------|------|--------|
| Technical Lead | — | ⬜ Pending |
| Security Lead | — | ⬜ Pending |
| QA Lead | — | ✅ Approved (Sprint 32) |
| Governance Council | — | ⬜ Pending |

### Launch Authorization
- [ ] All pre-flight checks passed
- [ ] Security audit complete
- [ ] Benchmarks within targets
- [ ] Documentation complete
- [ ] Rollback plan tested
- [ ] Monitoring configured
- [ ] Governance approval obtained

---

## Post-Launch (Day 1)

### Hour 0-1
- [ ] Verify all nodes healthy
- [ ] Check P2P connectivity
- [ ] Monitor error rates
- [ ] Validate metrics pipeline

### Hour 1-24
- [ ] Monitor benchmark metrics
- [ ] Check for security alerts
- [ ] Verify consensus operation
- [ ] Review logs for anomalies

### Day 1 Report
- [ ] Network uptime: ___%
- [ ] Average latency: ___ms
- [ ] Error rate: ___/hour
- [ ] Security incidents: ___
- [ ] Nodes online: ___/___

---

*Generated: 2026-05-22 | Sprint 33 | ed2kIA Release Engineering Team*
