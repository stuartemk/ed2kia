# Dashboard de Votación Comunitaria — Template

**Versión:** 1.0.0
**Licencia:** Apache 2.0 + Cláusula de Uso Ética
**Integración:** RFC Process (`docs/community/rfc-process.md`)

---

## 1. Estructura de Tabla de Votación

| RFC ID | Título | Propositor | Status | Votes Pro | Votes Contra | Votes Abstenido | Deadline | Link Discussion |
|--------|--------|------------|--------|-----------|--------------|-----------------|----------|-----------------|
| RFC-001 | Feedback Aggregation Process | @contributor | Active | 0 | 0 | 0 | 2026-06-15 | [Discussion](rfc-001-feedback-aggregation.md) |
| RFC-002 | Observability Infrastructure | @contributor | Draft | - | - | - | TBD | [Discussion](TBD) |

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

| Tier | Peso de Voto | Requisitos |
|------|--------------|------------|
| Novice | 0.5 | Registro verificado |
| Contributor | 1.0 | 1 PR mergeado o 1 RFC submitido |
| Active | 1.5 | 5 PRs mergeados o 3 meses activo |
| Steward | 2.0 | Nombrado por gobernanza actual |
| Guardian | 3.0 | 1 año activo + reputación verificada |

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

---

## 4. Timeline de Votación

| Fase | Duración | Responsable |
|------|----------|-------------|
| Draft → Active | Variable | Steward reviewer |
| Active (votación) | 7-14 días | Comunidad |
| Under Review | 3-5 días | Stewards |
| Decisión final | 48h | Stewards + Guardians |

---

*Template generado: 2026-05-16*
*Última actualización: 2026-05-16*
