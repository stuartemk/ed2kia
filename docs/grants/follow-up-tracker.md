# Grant Follow-up Tracker — ed2kIA v1.8

**Última actualización:** 2026-05-15 | **Responsable:** Grant Lead

---

## Grants en Seguimiento Post-Envío

| Grant | Submission Date | Status | Next Follow-up | Contact | Response Deadline |
|-------|----------------|--------|----------------|---------|-------------------|
| **NSF AI Safety** | [PENDING_SUBMISSION] | Pre-submission | 2026-06-01 | NSF FastLane Support | 6-8 weeks post-submission |
| **Gitcoin Quadratic Funding** | [PENDING_SUBMISSION] | Pre-submission | Round signal date | Gitcoin Forum Moderators | Round duration (~2 weeks) |
| **OSSF Security Grant** | [PENDING_SUBMISSION] | Pre-submission | 2026-06-15 | OSSF Grants Committee | 4-6 weeks post-submission |

---

## Checklist Post-Envío

### NSF AI Safety

- [ ] Confirmar recepción (FastLane acknowledgment)
- [ ] Guardar NSF_APP_ID y confirmation number
- [ ] Programar follow-up a las 4 semanas
- [ ] Preparar supplementary materials (si se solicitan)
- [ ] Notificar a PI/Co-PI sobre estado
- [ ] Actualizar submission-tracker.md con fechas reales

### Gitcoin Quadratic Funding

- [ ] Verificar que la aplicación aparece en Gitcoin Explorer
- [ ] Publicar signal post en Gitcoin Forum
- [ ] Compartir en Discord/Twitter con enlace de donación
- [ ] Monitorear donaciones diarias durante el round
- [ ] Preparar matching update para mitad de round
- [ ] Coordinar con donors clave para matching optimization

### OSSF Security Grant

- [ ] Confirmar recepción del comité
- [ ] Verificar que el SBOM fue procesado correctamente
- [ ] Programar call de seguimiento con security auditor
- [ ] Preparar threat model presentation (si se solicita)
- [ ] Actualizar threat_model_v1.1.md con hallazgos nuevos

---

## Timeline de Seguimiento

```
Week 0: Submission → Confirmar recepción
Week 1: Follow-up #1 → Verificar estado de revisión
Week 2-3: Supplementary → Preparar materiales adicionales
Week 4: Follow-up #2 → Status check con evaluadores
Week 6-8: Decision → Respuesta final
```

---

## Métricas de Éxito

| Grant | Target | Current | Gap |
|-------|--------|---------|-----|
| NSF AI Safety | $120,000 | $0 | $120,000 |
| Gitcoin QF | $5,000 | $0 | $5,000 |
| OSSF Security | $40,000 | $0 | $40,000 |
| **TOTAL** | **$165,000** | **$0** | **$165,000** |

---

## Contactos Clave

| Grant | Contact | Email/Link | Last Contact |
|-------|---------|-----------|--------------|
| NSF | FastLane Support | fastlane@nsf.gov | [PENDING] |
| Gitcoin | Forum Moderators | forum.gitcoin.co | [PENDING] |
| OSSF | Grants Committee | grants@openssf.org | [PENDING] |

---

## Automatización

```bash
# Verificar estado de grants
bash scripts/mentorship_onboarding.sh grants-status

# Generar reporte semanal de seguimiento
bash scripts/mentorship_onboarding.sh grants-report

# Actualizar tracker con fechas automáticas
bash scripts/mentorship_onboarding.sh grants-update
```

---

## Notas

- Todos los PLACEHOLDER deben ser reemplazados antes del envío real
- Actualizar este tracker semanalmente durante el ciclo de grants
- Coordinar con el equipo de community outreach para signal posts
- Mantener copies de todas las comunicaciones con evaluadores
