# Grant Execution Support Kit — ed2kIA v2.1

**Versión:** 1.0.0
**Fecha:** 2026-05-17
**Licencia:** Apache 2.0 + Cláusula de Uso Ética
**Propósito:** Soporte técnico para envíos de grant (NSF, Gitcoin, OSSF)

> **NOTA:** Este documento es SOPORTE DE EJECUCIÓN. No simula autenticación ni firmas externas.
> El envío final requiere revisión humana, firma externa y envío manual.

---

## 1. Technical Deliverables Matrix

### 1.1 Mapeo de Requisitos → Módulos v2.1

| Grant Req | Módulo v2.1 | Feature Gate | Métrica de Éxito | Status |
|-----------|-------------|--------------|------------------|--------|
| **Observable AI Systems** (NSF) | Observability scaffold | `v2.1-observability` | Prometheus metrics, Grafana dashboards | Scaffold |
| **Formal Verification** (NSF) | ZKP v3 multi-curve | `v2.1-zkp-v3` | BN254/BLS12/Pasta curves, batch verification | Scaffold |
| **Security Hardening** (OSSF) | CVE remediation | `v2.1-security-hardening` | 0 CVEs Critical/High, OSSF ≥ 9.0 | Pending |
| **Community Governance** (Gitcoin) | Voting dashboard | N/A | RFC-001 active, 30% quorum target | Active |
| **Distributed Training** (NSF) | Federation scaling | `v2.1-sprint1` | Multi-node sync, gradient compression | Scaffold |
| **Ethical AI Control** (NSF) | Neural Steer UI | `v2.1-gui` | Tauri desktop, ethical sliders | Scaffold |
| **API Transparency** (OSSF) | Enterprise APIs | `v2.1-enterprise` | OpenAPI spec, rate limiting | Scaffold |

### 1.2 Métricas de Éxito por Grant

| Grant | Métrica Principal | Target | Actual (v2.0) | Delta |
|-------|------------------|--------|---------------|-------|
| **NSF AI Safety** | Tests passing | ≥ 3500 | 3025 | +475 |
| **NSF AI Safety** | Code coverage | ≥ 85% | ≥ 80% | +5% |
| **OSSF** | OSSF Score | ≥ 9.0 | 8.5 | +0.5 |
| **OSSF** | CVEs Critical/High | 0 | 14 tracked | -14 |
| **Gitcoin** | Contributors activos | ≥ 50 | TBD | TBD |
| **Gitcoin** | PRs mergeados | ≥ 100 | TBD | TBD |

---

## 2. Budget Justification Template

### 2.1 Infraestructura CI/CD

| Item | Justificación | Costo Estimado | Periodo |
|------|--------------|----------------|---------|
| GitHub Actions minutes | CI/CD pipeline (cargo test, clippy, audit) | Included (free tier) | Ongoing |
| Docker Hub | Container registry for testnet images | $0 (free tier) | Ongoing |
| Grafana Cloud | Metrics visualization (optional) | $0 (self-hosted) | Ongoing |

### 2.2 Auditorías de Seguridad

| Item | Justificación | Costo Estimado | Periodo |
|------|--------------|----------------|---------|
| cargo audit | Weekly dependency scanning | $0 (automated) | Weekly |
| OSSF Scorecard | Open source security scoring | $0 (automated) | Monthly |
| External audit | Third-party security review | TBD (grant-funded) | Q2 2027 |

### 2.3 Hardware Testnet

| Item | Justificación | Costo Estimado | Periodo |
|------|--------------|----------------|---------|
| Testnet nodes | 5-node cluster for v2.1 testing | TBD (cloud credits) | Q2 2027 |
| Load testing | Benchmark infrastructure | $0 (local) | Ongoing |

### 2.4 Compensación Técnica

| Rol | Justificación | Compensación | Notas |
|-----|--------------|--------------|-------|
| Core Maintainers | Full-time development | TBD (grant-funded) | No tokens, cash-only |
| RFC Reviewers | Community governance | TBD (grant-funded) | Volunteer-first |
| Security Auditors | External review | TBD (grant-funded) | Contract-based |

> **PRINCIPIO:** CERO LÓGICA FINANCIERA EN CÓDIGO. Toda compensación es externa al protocolo.

---

## 3. Compliance & Ethics Checklist

### 3.1 Licencia y Ética

- [x] Apache 2.0 license en todos los archivos de código
- [x] Cláusula de Uso Ética en documentación
- [x] Cero lógica financiera en runtime
- [x] Transparencia absoluta en métricas
- [x] Gobernanza comunitaria (RFC process)

### 3.2 Verificación Técnica

```bash
# Verificar licencia en archivos Rust
grep -r "Apache" src/ --include="*.rs" | head -5

# Verificar ausencia de lógica financiera
grep -r "token\|coin\|wallet\|payment" src/ --include="*.rs" || echo "✓ No financial logic found"

# Verificar feature gates
grep "v2.1-" Cargo.toml | grep -E "feature|v2.1"
```

### 3.3 Datos y Privacidad

- [x] No collection de PII (Personally Identifiable Information)
- [x] Métricas agregadas, no individuales
- [x] Datos de testnet públicos y auditables
- [x] Cero telemetría sin consentimiento

---

## 4. Submission Workflow

### 4.1 Pasos para el Orquestador

```
1. REVISIÓN HUMANA
   - [ ] Revisar Technical Deliverables Matrix
   - [ ] Validar métricas de éxito
   - [ ] Confirmar compliance & ethics

2. FIRMA EXTERNA
   - [ ] Preparar documentos para signatario autorizado
   - [ ] Verificar identidad del signatario
   - [ ] Firma manual (NO simulada)

3. ENVÍO MANUAL
   - [ ] Submitir a portal del grant
   - [ ] Guardar confirmación de recepción
   - [ ] Actualizar submission-tracker.md

4. SEGUIMIENTO
   - [ ] Monitorear status de aplicación
   - [ ] Responder solicitudes de información
   - [ ] Actualizar follow-up-tracker.md
```

### 4.2 Guardrails de Envío

| Regla | Descripción |
|-------|-------------|
| **CERO AUTENTICACIÓN** | Nunca simular login, API keys, o firmas externas |
| **CERO FIRMAS** | Nunca generar firmas digitales para envíos |
| **REVISIÓN HUMANA** | Todo envío requiere aprobación humana explícita |
| **TRANSPARENCIA** | Documentar todos los pasos del proceso |
| **BACKUP** | Guardar copias de todos los documentos enviados |

---

## 5. Referencias

| Documento | Path |
|-----------|------|
| NSF AI Safety Draft | `docs/grants/nsf-ai-safety-draft.md` |
| Gitcoin Quadratic Funding | `docs/grants/gitcoin-quadratic-funding-draft.md` |
| OSSF Draft | `docs/grants/ossf-draft.md` |
| Submission Tracker | `docs/grants/submission-tracker.md` |
| Follow-up Tracker | `docs/grants/follow-up-tracker.md` |
| Security Audit Q1 2027 | `docs/reports/security-audit-Q1-2027.md` |
| Feature Gates | `Cargo.toml` [features] |
| Voting Dashboard | `docs/community/voting-dashboard-active.md` |

---

*Kit generado: 2026-05-17*
*Mantenido por: Qweni (Autonomous Stewardship Loop)*
*Próxima revisión: Pre-submission de cada grant*
