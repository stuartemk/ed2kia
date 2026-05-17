# Dashboard de Votación Comunitaria — ACTIVO

**Versión:** 2.0.0-active
**Licencia:** Apache 2.0 + Cláusula de Uso Ética
**Activación:** 2026-05-17
**Integración:** RFC Process (`docs/governance/rfc-tracking.md`)

> **STATUS:** ✅ OPERATIONAL — Dashboard activo para votación comunitaria v2.1
> **NOTA:** Esta es una herramienta de seguimiento PASIVA. No ejecuta código automáticamente.

---

## 1. Tabla de Votación Activa

| RFC ID | Título | Propositor | Status | Votes Pro | Votes Contra | Votes Abstenido | Deadline | Link Discussion |
|--------|--------|------------|--------|-----------|--------------|-----------------|----------|-----------------|
| RFC-001 | Feedback Aggregation Process | Community | **Active** | 0 | 0 | 0 | 2026-06-15 | [Discussion](rfc-001-feedback-aggregation.md) |
| RFC-002 | Observability Infrastructure (Prometheus/Grafana) | Qweni | Draft | - | - | - | TBD | [Scaffold](../../src/observability/mod.rs) |
| RFC-003 | Testnet/Infra v2.1 (Docker, Systemd, CI) | Qweni | Draft | - | - | - | TBD | [Docker](../../infra/docker-compose.testnet-v2.1.yml) |

### Estados de Propuesta
- **Draft**: En desarrollo, no abierto a votación
- **Active**: Abierto a votación comunitaria
- **Under Review**: Votación cerrada, en evaluación por stewards
- **Accepted**: Aprobado, listo para implementación
- **Modified**: Aprobado con modificaciones sugeridas
- **Deferred**: Pospuesto a siguiente ciclo
- **Rejected**: Rechazado por no alcanzar quórum o mayoría

---

## 2. Reglas de Quórum y Pesos

### 2.1 Tiers de Participación

| Tier | Peso de Voto | Requisitos | Miembros Activos |
|------|--------------|------------|------------------|
| Novice | 0.5 | Registro verificado | TBD |
| Contributor | 1.0 | 1 PR mergeado o 1 RFC submitido | TBD |
| Active | 1.5 | 5 PRs mergeados o 3 meses activo | TBD |
| Steward | 2.0 | Nombrado por gobernanza actual | TBD |
| Guardian | 3.0 | 1 año activo + reputación verificada | TBD |

### 2.2 Cálculo de Quórum

```
Quórum = (Total de votos ponderados) / (Total de miembros elegibles ponderados)
Umbral de quórum = 30% (0.30)
```

### 2.3 Cálculo de Mayoría

```
Mayoría = (Votos Pro ponderados) / (Votos Pro + Votos Contra ponderados)
Umbral de aprobación = 60% (0.60)
Umbral de veto (Guardians) = 2 Guardianes en contra = Rechazo automático
```

### 2.4 Ejemplo de Cálculo

```
Propuesta: RFC-001
- 10 Novices a favor: 10 × 0.5 = 5.0
- 5 Contributors a favor: 5 × 1.0 = 5.0
- 2 Stewards a favor: 2 × 2.0 = 4.0
- 1 Contributor en contra: 1 × 1.0 = 1.0
- 1 Guardian en contra: 1 × 3.0 = 3.0

Total Pro ponderado: 14.0
Total Contra ponderado: 4.0
Mayoría: 14.0 / (14.0 + 4.0) = 77.8% ✅ APROBADO

Quórum: 18 votos ponderados / 50 miembros elegibles = 36% ✅ QUÓRUM ALCANZADO
```

---

## 3. Integración con RFC Process

### 3.1 Flujo de Votación

1. RFC submitido → Status: **Draft**
2. Steward revisa formato → Status: **Active** (abre votación)
3. Período de votación (mínimo 7 días)
4. Cierre de votación → Status: **Under Review**
5. Stewards evalúan resultados + feedback cualitativo
6. Decisión final → Status: **Accepted/Modified/Deferred/Rejected**

### 3.2 Export CSV para Tally Script

Formato esperado por `scripts/voting-tally.sh`:
```csv
rfc_id,propositor,status,vote_weight,tier,timestamp
RFC-001,@user1,pro,1.0,Contributor,2026-05-16T12:00:00Z
RFC-001,@user2,contra,2.0,Steward,2026-05-16T13:00:00Z
```

### 3.3 Validación del Script de Tally

```bash
# Validar syntax del script
bash -n scripts/voting-tally.sh && echo "✓ Syntax OK"

# Ejecutar con datos de ejemplo
./scripts/voting-tally.sh --input tests/voting/sample.csv --output results/tally.json
```

---

## 4. Timeline de Votación

| Fase | Duración | Responsable |
|------|----------|-------------|
| Draft → Active | Variable | Steward reviewer |
| Active (votación) | 7-14 días | Comunidad |
| Under Review | 3-5 días | Stewards |
| Decisión Final | 1-2 días | Stewards + Guardianes |

---

## 5. Triggers de Votación

### 5.1 Cuándo Activar Votación

| Condición | Acción |
|-----------|--------|
| RFC con status **Draft** + formato válido | Steward → **Active** |
| 7 días desde activación | Auto-cierre → **Under Review** |
| Quórum < 30% al cierre | **Deferred** (siguiente ciclo) |
| Mayoría < 60% | **Rejected** |
| 2+ Guardianes en contra | **Rejected** (veto) |
| Mayoría ≥ 60% + quórum ≥ 30% | **Accepted** |

### 5.2 Notificaciones

| Evento | Canal |
|--------|-------|
| RFC activado | GitHub Discussion + Discord |
| 3 días antes de cierre | Email + Discord reminder |
| 1 día antes de cierre | Email + Discord + GitHub notification |
| Resultado publicado | GitHub Release + Discord announcement |

---

## 6. Métricas de Participación

### 6.1 Dashboard de Métricas

| Métrica | Valor Actual | Target |
|---------|-------------|--------|
| RFCs Activos | 1 (RFC-001) | ≥ 3 |
| Total Votos (RFC-001) | 0 | ≥ 50 |
| Quórum Alcanzado | N/A | ≥ 30% |
| Participación por Tier | N/A | Todos los tiers |
| Tiempo Promedio Decisión | N/A | ≤ 14 días |

### 6.2 Historial de Votaciones

| RFC | Fecha Inicio | Fecha Cierre | Resultado | Quórum | Mayoría |
|-----|-------------|--------------|-----------|--------|---------|
| *(Sin votaciones completadas)* | - | - | - | - | - |

---

## 7. Guardrails de Gobernanza

### 7.1 Principios

1. **GOBERNANZA PASIVA:** Este dashboard es una herramienta de seguimiento. No ejecuta código automáticamente.
2. **TRANSPARENCIA:** Todos los votos son públicos y auditables.
3. **INCLUSIÓN:** Múltiples tiers permiten participación progresiva.
4. **SEGURIDAD:** Veto Guardian protege contra cambios peligrosos.
5. **APACHE 2.0 + ÉTICA:** Todas las decisiones respetan la licencia y cláusula ética.

### 7.2 Anti-Sybil

- **Rate Limiting:** Máx 1 voto por RFC por miembro
- **Verificación:** Registro verificado requerido
- **Tier Escalado:** Pesos basados en contribución histórica
- **Audit Log:** Todos los votos registrados con timestamp

---

## 8. Referencias

| Documento | Path |
|-----------|------|
| RFC Tracking | `docs/governance/rfc-tracking.md` |
| Voting Tally Script | `scripts/voting-tally.sh` |
| RFC-001 (Feedback) | `docs/community/rfc-001-feedback-aggregation.md` |
| RFC Call v2.1 | `docs/community/rfc-call-v2.1.md` |
| GOVERNANCE.md | `GOVERNANCE.md` |
| Contributor Funnel | `docs/community/contributor-funnel.md` |

---

*Dashboard activado: 2026-05-17*
*Mantenido por: Qweni (Autonomous Stewardship Loop)*
*Próxima revisión: 2026-06-15 (Deadline RFC-001)*
