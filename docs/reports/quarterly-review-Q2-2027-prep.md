# Preparación Ciclo de Revisión Q2 2027

**Versión:** 1.0.0-draft
**Fecha:** 2026-05-17
**Licencia:** Apache 2.0 + Cláusula de Uso Ética
**Responsable:** Qweni (Autonomous Stewardship Loop)

---

## 1. Resumen Ejecutivo

Este documento establece la **línea base métrica**, **cronograma** y **lista de entregables** para el ciclo de revisión trimestral Q2 2027 (Julio - Septiembre 2027), correspondiente al sprint v2.1 post-v2.0.0-stable.

| Campo | Valor |
|-------|-------|
| **Ciclo** | Q2 2027 |
| **Período** | 2027-07-01 → 2027-09-30 |
| **Versión Objetivo** | v2.1.0-stable |
| **Feature Gates** | v2.1-sprint1, v2.1-gui, v2.1-zkp-v3, v2.1-enterprise, v2.1-observability, v2.1-security-hardening |
| **Status** | Preparación |

---

## 2. Línea Base Métrica (v2.0.0-stable)

### 2.1 Métricas de Código

| Métrica | Valor | Fuente |
|---------|-------|--------|
| **Tests** | 3025 passing (99.7%) | `cargo test --all-targets` |
| **Cobertura** | ≥80% | cargo-llvm-cov |
| **Módulos** | 80+ implementados | `src/lib.rs` |
| **Warnings** | Pre-existentes (documentados) | `cargo clippy` |
| **CVEs Identificados** | 14 | `docs/reports/security-audit-Q1-2027.md` |

### 2.2 Métricas de Infraestructura

| Métrica | Valor | Fuente |
|---------|-------|--------|
| **OSSF Score** | 8.5/10 (PASSING) | OSSF Scorecard |
| **CI/CD** | GitHub Actions (multi-stage) | `.github/workflows/` |
| **Docker** | Multi-stage build | `deploy/Dockerfile` |
| **Systemd** | Service + Env | `deploy/systemd/` |
| **Testnet** | Scaffold (placeholders) | `infra/docker-compose.testnet-v2.1.yml` |

### 2.3 Métricas de Gobernanza

| Métrica | Valor | Fuente |
|---------|-------|--------|
| **RFCs Activos** | 3 (RFC-001/002/003) | `docs/governance/rfc-tracking.md` |
| **Constitución** | Aprobada | `GOVERNANCE.md` |
| **Voting Dashboard** | Template → Active | `docs/community/voting-dashboard-template.md` |
| **Feature Flags** | 6 gates activos | `Cargo.toml` [features] |

### 2.4 Métricas de Comunidad

| Métrica | Valor | Fuente |
|---------|-------|--------|
| **Contributor Funnel** | Documentado | `docs/community/contributor-funnel.md` |
| **Badge System** | Implementado | `scripts/generate_contributor_badges.sh` |
| **Milestone Tracker** | Activo | `docs/community/milestone-tracker.md` |
| **Early Access** | Scaffold | `release/v2.0.0-stable/` |

---

## 3. Cronograma Q2 2027

### 3.1 Timeline

| Semana | Fecha | Hitos |
|--------|-------|-------|
| **W1** | 2027-07-04 | Kickoff Q2, revisión línea base |
| **W2-3** | 2027-07-11 | RFC-001 votación + decisión |
| **W4** | 2027-07-25 | Sprint 1: GUI Tauri + ZKP v3 |
| **W5-6** | 2027-08-01 | Sprint 1: Enterprise + Observability |
| **W7** | 2027-08-15 | Mid-point review + métricas intermedias |
| **W8-9** | 2027-08-22 | Sprint 2: Security hardening |
| **W10** | 2027-09-05 | Sprint 2: Testnet v2.1 real |
| **W11** | 2027-09-12 | Integración + E2E tests |
| **W12-13** | 2027-09-19 | Release engineering v2.1.0-rc |
| **W14** | 2027-09-30 | Final review + signoff v2.1.0-stable |

### 3.2 Hitos Críticos

| Hito | Fecha Límite | Dependencias |
|------|-------------|--------------|
| RFC-001 Decisión | 2027-07-18 | Votación comunitaria |
| GUI Tauri MVP | 2027-08-01 | RFC-001 approval |
| ZKP Multi-Curve v3 | 2027-08-15 | Circuit optimization |
| Security Hardening | 2027-09-01 | CVE remediation plan |
| Testnet v2.1 Real | 2027-09-15 | Docker images + infra |
| v2.1.0-rc | 2027-09-22 | All modules integrated |
| v2.1.0-stable | 2027-09-30 | Signoff + validation |

---

## 4. Checklist de Entregables

### 4.1 Módulos de Código

- [ ] `src/gui/tauri_scaffold.rs` — Real Tauri app (no scaffold)
- [ ] `src/gui/neural_steer_ui.rs` — Ethical sliders + bridge
- [ ] `src/zkp/multi_curve_setup.rs` — BN254/BLS12/Pasta curves
- [ ] `src/zkp/proof_aggregation.rs` — Batch verification
- [ ] `src/zkp/circuit_optimization.rs` — Constraint pooling
- [ ] `src/observability/mod.rs` — Prometheus metrics
- [ ] `src/enterprise/mod.rs` — Enterprise APIs
- [ ] `src/security/hardening.rs` — CVE remediation

### 4.2 Infraestructura

- [ ] `infra/docker-compose.testnet-v2.1.yml` — Real images
- [ ] `infra/testnet-metrics-simulated.json` → Real metrics
- [ ] `scripts/testnet-dryrun.sh` → Production dry-run
- [ ] `deploy/Dockerfile` — Multi-stage v2.1
- [ ] `monitoring/prometheus.yml` — Config real
- [ ] `monitoring/grafana/` — Dashboards v2.1

### 4.3 Gobernanza

- [ ] RFC-001 votación completada
- [ ] RFC-002 observability implementada
- [ ] RFC-003 testnet hardening
- [ ] Dashboard de votación activo
- [ ] Quarterly review Q2 completada

### 4.4 Seguridad

- [ ] 14 CVEs remediados (o mitigados)
- [ ] `wasmtime` ≥ 24.0.7 (feature-gated)
- [ ] `rustls-webpki` ≥ 0.102.0 (feature-gated)
- [ ] OSSF Score ≥ 9.0
- [ ] Security audit Q2 2027

### 4.5 Documentación

- [ ] `CHANGELOG.md` v2.1.0-stable
- [ ] `SECURITY.md` Q2 2027 update
- [ ] `README.md` v2.1 features
- [ ] Migration guide v2.0 → v2.1
- [ ] API docs v2.1

---

## 5. Riesgos y Mitigación

| Riesgo | Probabilidad | Impacto | Mitigación |
|--------|-------------|---------|------------|
| RFC-001 rechazado | Media | Alto | Plan B: feedback manual |
| CVEs sin fix upstream | Alta | Medio | Feature-gated workarounds |
| Delay GUI Tauri | Media | Medio | Scaffold → iterative |
| Testnet infra costs | Baja | Bajo | Local-first, cloud optional |
| Quórum votación < 30% | Media | Alto | Ambassador program push |

---

## 6. Criterios de Éxito

### 6.1 Técnicos

- [ ] ≥ 3500 tests passing (99.5%+)
- [ ] ≥ 85% code coverage
- [ ] OSSF Score ≥ 9.0
- [ ] 0 CVEs Critical/High unresolved
- [ ] All 6 feature gates implemented

### 6.2 Gobernanza

- [ ] RFC-001 votación con ≥ 30% quórum
- [ ] ≥ 50 participantes en votación
- [ ] Dashboard activo con datos reales
- [ ] RFC triage Q2 completada

### 6.3 Comunidad

- [ ] ≥ 20 contributors activos
- [ ] ≥ 50 PRs mergeados
- [ ] Badge system con ≥ 100 badges emitidos
- [ ] Early access ≥ 50 nodos

---

## 7. Recursos y Dependencias

### 7.1 Herramientas

| Herramienta | Versión | Propósito |
|-------------|---------|-----------|
| Rust | ≥ 1.75.0 | Compiler |
| Cargo | ≥ 1.75.0 | Build system |
| Docker | ≥ 24.0 | Containerization |
| Tauri | ≥ 2.0 | Desktop GUI |
| Prometheus | ≥ 2.48 | Metrics |
| Grafana | ≥ 10.0 | Visualization |

### 7.2 Dependencias Críticas

| Paquete | Versión Actual | Versión Objetivo | Feature Gate |
|---------|----------------|------------------|--------------|
| wasmtime | 17.0.3 | ≥ 24.0.7 | v2.1-security-hardening |
| rustls-webpki | 0.101.7 | ≥ 0.102.0 | v2.1-security-hardening |
| libp2p | 0.53 | ≥ 0.54 | v2.1-sprint1 |
| ark-bn254 | 0.4 | ≥ 0.5 | v2.1-zkp-v3 |

---

## 8. Referencias

| Documento | Path |
|-----------|------|
| CHANGELOG v2.0.0-stable | `CHANGELOG.md` |
| Security Audit Q1 2027 | `docs/reports/security-audit-Q1-2027.md` |
| Remediation Plan Q1 2027 | `docs/reports/dependency-remediation-plan-Q1-2027.md` |
| RFC Tracking | `docs/governance/rfc-tracking.md` |
| Voting Dashboard Template | `docs/community/voting-dashboard-template.md` |
| Testnet Dry-Run | `docs/reports/testnet-dryrun-v2.1.md` |
| Feature Gates | `Cargo.toml` [features] |
| Final Signoff v2.0.0 | `release/v2.0.0-stable/final-signoff.json` |

---

*Documento generado: 2026-05-17*
*Mantenido por: Qweni (Autonomous Stewardship Loop)*
*Próxima revisión: 2027-07-04 (Kickoff Q2)*
