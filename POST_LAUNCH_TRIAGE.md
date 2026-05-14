# Post-Launch Triage Protocol — ed2kIA v1.6.0-stable

**Versión:** 1.0
**Fecha:** 2026-05-14
**Propósito:** Establecer protocolo estandarizado para triaje de incidentes post-lanzamiento.

---

## 1. Ventanas de Monitoreo

| Ventana | Duración | Enfoque | Responsable |
|---------|----------|---------|-------------|
| **Critical** | 0-48h | Estabilidad del sistema, detección de crashes, escaneo de seguridad | On-Call + Core Team |
| **Stabilization** | 48h-7d | Regresión de performance, triaje de feedback de usuarios | Core Team |
| **Observation** | 7d-30d | Tendencias a largo plazo, adopción de features, actividad de gobernanza | Full Team |

---

## 2. SLAs por Severidad

| Severidad | Response Time | Resolution Target | Ejemplo |
|-----------|--------------|-------------------|---------|
| **SEV-1** | 2h | 4h | Network partition, data loss, critical security flaw |
| **SEV-2** | 12h | 24h | Major feature broken, performance degradation >50% |
| **SEV-3** | 48h | 72h | Minor feature issue, documentation error |
| **SEV-4** | 7d | Next sprint | Enhancement request, cosmetic issue |

---

## 3. Routing de Issues

| Label | Team | Channel | Escalation |
|-------|------|---------|------------|
| `bug` | @ed2kIA/core-team | Discord #bugs | Core Team Lead |
| `docs` | @ed2kIA/docs-team | Discord #documentation | Docs Lead |
| `security` | @ed2kIA/security | SECURITY.md channel | Security Lead → Immediate |
| `governance` | @ed2kIA/governance | Discord #governance | Governance Council |
| `sae` | @ed2kIA/sae-team | Discord #dev-chat | SAE Lead |
| `p2p` | @ed2kIA/p2p-team | Discord #dev-chat | P2P Lead |
| `zkr` | @ed2kIA/zkp-team | Discord #dev-chat | ZKP Lead |
| `enhancement` | @ed2kIA/core-team | GitHub Discussions | Product Lead |
| `good first issue` | Community | Discord #contributors | Community Manager |

---

## 4. Checklist Primeros 72h

### Hora 0-4 (Lanzamiento)

- [ ] CI status verde (`cargo test --features stable` = 187 passed)
- [ ] Tag `v1.6.0-stable` visible en GitHub
- [ ] Release draft creado con artifacts adjuntos
- [ ] Announce en Discord #announcements
- [ ] On-call rotation activado

### Hora 4-24 (Critical Window)

- [ ] Dashboard health verificado (Prometheus + Grafana)
- [ ] Primeras métricas de node uptime (≥ 99.5%)
- [ ] ZKP verification time ≤ 200ms p95
- [ ] P2P sync latency ≤ 500ms p95
- [ ] Zero SEV-1 incidents

### Hora 24-48 (Stabilization Start)

- [ ] First PR review completada (si hay contributions)
- [ ] Community onboarding verificado (nuevos node operators)
- [ ] Issue triage: todos los issues con label + assignee
- [ ] Performance baseline establecido

### Hora 48-72 (Observation Prep)

- [ ] Primer reporte de métricas generado
- [ ] Feedback de primeros usuarios recopilado
- [ ] Backlog de bugs priorizado
- [ ] Plan de hotfix preparado (si necesario)

---

## 5. Flujo de Triage

```
1. DETECTAR
   ↓
   Fuente: CI alert / User report / Monitoring dashboard
   ↓
2. CLASIFICAR
   ↓
   Asignar: Severity (SEV-1 a SEV-4) + Label + Team
   ↓
3. NOTIFICAR
   ↓
   Channel: Discord # según routing table
   SLA: Timer iniciado
   ↓
4. DIAGNOSTICAR
   ↓
   Repro steps → Root cause → Fix proposal
   ↓
5. RESOLVER
   ↓
   Hotfix PR → Fast-track review → Deploy
   ↓
6. VERIFICAR
   ↓
   Monitoring confirma resolución
   Issue cerrado con fix version
   ↓
7. POSTMORTEM (SEV-1/2)
   ↓
   Dentro de 48h → Action items → Public update
```

---

## 6. Templates de Respuesta

### Acknowledgment (SEV-1/2)

```
🚨 Acknowledged: [Issue Title]
Severity: SEV-[1/2]
Assigned to: @team
ETA Diagnosis: [time]
ETA Fix: [time]
War Room: [link]
```

### Resolution

```
✅ Resolved: [Issue Title]
Fix: [PR #]
Verification: [monitoring link]
Rollout: [percentage/timeline]
Postmortem: [link si SEV-1/2]
```

### Escalation

```
⬆️ Escalated: [Issue Title]
From: @person
To: @escalation-target
Reason: [SLA breach / complexity / cross-team]
New ETA: [time]
```

---

## 7. Escalation Matrix

| Level | Role | Contact | Trigger |
|-------|------|---------|---------|
| L1 | On-Call Engineer | Discord @here | Issue detected |
| L2 | Team Lead | Discord DM | SLA 50% consumed |
| L3 | Core Team Lead | Phone/Signal | SLA breach imminent |
| L4 | Governance Council | Emergency meeting | SEV-1 > 4h unresolved |

---

## 8. Metrics de Éxito del Triage

| Metric | Target | Measurement |
|--------|--------|-------------|
| Mean Time to Acknowledge (MTTA) | ≤ 30 min | Issue creation → First response |
| Mean Time to Resolve (MTTR) | ≤ SLA per severity | Issue creation → Resolution |
| SLA Compliance | ≥ 95% | Issues resolved within SLA |
| Escalation Rate | ≤ 10% | Issues escalated / Total issues |
| Customer Satisfaction | ≥ 4.0/5.0 | Post-resolution survey |

---

## 9. Integración con GitHub

### Auto-Assignment Rules

```yaml
# .github/CODEOWNERS patterns
/src/sae/     @ed2kIA/sae-team
/src/p2p/     @ed2kIA/p2p-team
/src/zkp/     @ed2kIA/zkp-team
/src/governance/ @ed2kIA/governance
/docs/        @ed2kIA/docs-team
```

### Issue Template Labels

| Template | Auto-Labels |
|----------|-------------|
| bug_report.md | `bug`, `triage` |
| feature_request.md | `enhancement`, `triage` |
| node_operator_issue.md | `p2p`, `triage` |

---

*Post-Launch Triage Protocol — ed2kIA v1.6.0-stable*
*Generated: 2026-05-14*
