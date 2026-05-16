# Grants Submission Checklist — ed2kIA v1.9

**Versión:** v1.9 Sprint 2
**Fecha:** 2026-05-16
**Responsable:** Grant Lead + Core Team
**Estado:** Preparación (NO envío)

---

## Resumen de Grants en Pipeline

| Grant | Budget | Deadline | Estado | Draft |
|-------|--------|----------|--------|-------|
| NSF AI Safety | $120K | Q3 2026 | Draft completo | `docs/grants/nsf-ai-safety-draft.md` |
| Gitcoin Quadratic Funding | $5K target | Round #XX | Draft completo | `docs/grants/gitcoin-quadratic-funding-draft.md` |
| OSSF Security Grant | $40K | Next cycle | Draft completo | `docs/grants/ossf-draft.md` |

---

## Checklist Pre-Envío por Grant

### NSF AI Safety — $120K / 6 meses

#### Documentos Requeridos
- [ ] NSF_APP_ID registrado en FastLane
- [ ] PI y Co-PI identificados con firmas
- [ ] Budget justification actualizado (rates 2026)
- [ ] Institutional letter obtenida
- [ ] References verificadas (DOIs, URLs)
- [ ] Impact metrics con baselines reales

#### Contenido Técnico
- [ ] ZKP circuit audit scope definido
- [ ] Async steering hardening plan documentado
- [ ] Security model con threat analysis
- [ ] Risk assessment revisado por compliance
- [ ] Timeline con milestones verificables

#### Validación
- [ ] Draft revisado por PI y Co-PI
- [ ] Budget aligna con scope técnico
- [ ] Impact metrics medibles
- [ ] Firmas digitales de todos los PIs

---

### Gitcoin Quadratic Funding — $5K target

#### Documentos Requeridos
- [ ] Wallet address verificado (ETH mainnet)
- [ ] Discord server URL activo
- [ ] Twitter handle verificado
- [ ] Gitcoin_APP_ID registrado
- [ ] Signal post publicado en Gitcoin Forum

#### Contenido Comunitario
- [ ] Community narrative con métricas reales
- [ ] Donor retention plan documentado
- [ ] Use of funds transparente
- [ ] Contributor spotlight examples
- [ ] Roadmap v1.9→v2.0 visible

#### Validación
- [ ] Links verificados (Discord, Twitter, GitHub)
- [ ] Métricas de comunidad actualizadas
- [ ] Signal post con engagement >50 upvotes
- [ ] Wallet address con actividad reciente

---

### OSSF Security Grant — $40K

#### Documentos Requeridos
- [ ] Cryptography expert identificado
- [ ] Security auditor contactado
- [ ] OSSF_APP_ID registrado
- [ ] SBOM generado (CycloneDX/SPDX)

#### Contenido de Seguridad
- [ ] Threat model actualizado
- [ ] ZKP circuit audit scope definido
- [ ] WASM sandbox limits documentados
- [ ] async_steering.rs hardening plan
- [ ] Dependency audit (cargo audit clean)

#### Validación
- [ ] Security expert con credenciales verificadas
- [ ] Auditor con experiencia en ZKP/cryptography
- [ ] SBOM generado y validado
- [ ] Zero critical vulnerabilities

---

## Workflow de Preparación

### Fase 1: Verificación de Drafts

```bash
# Verificar que todos los drafts existen
test -f docs/grants/nsf-ai-safety-draft.md && echo "NSF: OK"
test -f docs/grants/gitcoin-quadratic-funding-draft.md && echo "Gitcoin: OK"
test -f docs/grants/ossf-draft.md && echo "OSSF: OK"
```

### Fase 2: Review Interno

1. **PI/Co-PI review** — Revisión técnica y científica
2. **Budget review** — Verificar alignment con scope
3. **Security review** — Para OSSF y NSF
4. **Community review** — Para Gitcoin

### Fase 3: Pre-Envío

1. **Finalizar placeholders** — Reemplazar [PLACEHOLDER] con datos reales
2. **Generar SBOM** — `cargo audit --format cyclonedx > sbom.json`
3. **Verificar links** — Todos los URLs accesibles
4. **Backup** — Copia de seguridad de drafts finales

### Fase 4: Envío (cuando esté listo)

1. **Submit NSF** — FastLane portal
2. **Submit Gitcoin** — ALLO portal
3. **Submit OSSF** — Grants portal
4. **Update tracker** — `docs/grants/submission-tracker.md`

---

## Métricas de Progreso

| Grant | Docs | Review | Pre-Envío | Envío |
|-------|------|--------|-----------|-------|
| NSF | [ ] | [ ] | [ ] | [ ] |
| Gitcoin | [ ] | [ ] | [ ] | [ ] |
| OSSF | [ ] | [ ] | [ ] | [ ] |

---

## Notas Importantes

1. **NO enviar sin approval del core team** — Requiere 2/3 approvals
2. **Budget alignment** — Verificar que scope técnico justifica budget solicitado
3. **Timeline realista** — No comprometer deadlines sin capacidad verificada
4. **Transparencia** — Todos los drafts públicos en repo

---

## Próximos Pasos

- [ ] Completar review interno NSF draft
- [ ] Obtener institutional letter (NSF)
- [ ] Verificar wallet address (Gitcoin)
- [ ] Contactar security auditor (OSSF)
- [ ] Generar SBOM actualizado
- [ ] Publicar signal post Gitcoin Forum

---

*Documento de preparación — NO es envío formal*
*Última actualización: 2026-05-16*
