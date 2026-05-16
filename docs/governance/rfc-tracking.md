# RFC Tracking — Request for Comments Registry

**Versión:** 1.0.0
**Licencia:** Apache 2.0 + Cláusula de Uso Ética
**Última Actualización:** 2026-05-16

---

## 1. RFC Registry

| RFC ID | Título | Autor | Status | Created | Deadline | Links |
|--------|--------|-------|--------|---------|----------|-------|
| RFC-001 | Feedback Aggregation Process | Community | Discusión | 2026-05-16 | 2026-06-15 | [Feedback](../community/rfc-001-feedback-aggregation.md) |
| RFC-002 | Observability Infrastructure (Prometheus/Grafana) | Qweni | Draft | 2026-05-16 | TBD | [Scaffold](../../src/observability/mod.rs) |
| RFC-003 | Testnet/Infra v2.1 (Docker, Systemd, CI) | Qweni | Draft | 2026-05-16 | TBD | [Docker](../../infra/docker-compose.testnet-v2.1.yml) |

### Estados
- **Draft**: En desarrollo, no abierto a votación
- **Discusión**: Abierto a feedback comunitario
- **Votación**: Abierto a votación ponderada
- **Aceptado**: Aprobado, listo para implementación
- **Modificado**: Aprobado con cambios
- **Pospuesto**: Diferido a siguiente ciclo
- **Rechazado**: No alcanzó quórum/mayoría

---

## 2. Integración con Dashboard de Votación

- **Template de votación:** [`docs/community/voting-dashboard-template.md`](../community/voting-dashboard-template.md)
- **Script de tally:** [`scripts/voting-tally.sh`](../../scripts/voting-tally.sh)
- **Reglas de quórum:** 30% participación, 60% mayoría, veto Guardian (2 en contra)

---

## 3. Monitoreo de Seguridad

- **Reporte semanal:** [`docs/reports/security-monitor-weekly.md`](../reports/security-monitor-weekly.md)
- **Pipeline CI:** [`.github/workflows/security-monitor.yml`](../../.github/workflows/security-monitor.yml)
- **Script de alertas:** [`scripts/security-alert.sh`](../../scripts/security-alert.sh)
- **Plan de remediación:** [`docs/reports/dependency-remediation-plan-Q1-2027.md`](../reports/dependency-remediation-plan-Q1-2027.md)

---

## 4. Feature Gates Activos

| Feature Gate | RFC | Status | Descripción |
|--------------|-----|--------|-------------|
| `v2.1-sprint1` | RFC-001 | Scaffold | GUI, ZKP v3, Enterprise placeholders |
| `v2.1-observability` | RFC-002 | Scaffold | Prometheus/Grafana metrics collection |
| `v2.1-security-hardening` | TBD | Pending | Dependency upgrades (wasmtime, libp2p) |

---

*Registry generado: 2026-05-16*
*Mantenido por: Qweni (Autonomous Agent)*
