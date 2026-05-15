# Grant Submission Tracker — ed2kIA v1.8

**Última actualización:** 2026-05-15 | **Responsable:** [PLACEHOLDER: Grant Lead]

---

## Tabla de Grants Activos

| Grant | Deadline | Status | Required Docs | Submission Link | Notes |
|-------|----------|--------|---------------|-----------------|-------|
| **NSF AI Safety** | [PLACEHOLDER: Q3 2026] | Draft Complete | NSF_APP_ID, PI signatures, budget justification, institutional letter | [PLACEHOLDER: NSF FastLane URL] | $120K budget, 6mo timeline. Draft: `nsf-ai-safety-draft.md`. Needs PI/Co-PI names, institution verification. |
| **Gitcoin Quadratic Funding** | [PLACEHOLDER: Round #XX] | Draft Complete | Gitcoin_APP_ID, wallet address, Discord/Twitter links, signal post | [PLACEHOLDER: Gitcoin ALLO URL] | $5K target. Draft: `gitcoin-quadratic-funding-draft.md`. Needs wallet address, community links, forum signal post. |
| **OSSF Security Grant** | [PLACEHOLDER: Next cycle] | Draft Complete | OSSF_APP_ID, security auditor contact, threat model, SBOM | [PLACEHOLDER: OSSF Grants URL] | $40K budget. Draft: `ossf-draft.md`. Needs cryptography expert name, security auditor contact, SBOM generation. |

---

## Checklist Pre-Envío por Grant

### NSF AI Safety

- [ ] Draft revisado por PI y Co-PI
- [ ] Budget justification actualizado con rates actuales
- [ ] Institutional letter obtenida
- [ ] NSF_APP_ID registrado en FastLane
- [ ] Firmas digitales de todos los PIs
- [ ] References verificadas (DOIs, URLs)
- [ ] Impact metrics con baselines reales
- [ ] Risk assessment revisado por compliance

### Gitcoin Quadratic Funding

- [ ] Wallet address verificado (ETH mainnet)
- [ ] Discord server URL activo
- [ ] Twitter handle verificado
- [ ] Signal post publicado en Gitcoin Forum
- [ ] Community narrative con métricas reales
- [ ] Donor retention plan documentado
- [ ] Use of funds transparente
- [ ] Gitcoin_APP_ID registrado

### OSSF Security Grant

- [ ] Cryptography expert identificado
- [ ] Security auditor contactado
- [ ] Threat model actualizado
- [ ] SBOM generado (CycloneDX/SPDX)
- [ ] ZKP circuit audit scope definido
- [ ] WASM sandbox limits documentados
- [ ] async_steering.rs hardening plan
- [ ] OSSF_APP_ID registrado

---

## Workflow de Preparación

### Paso 1: Verificar Drafts

```bash
# Verificar que todos los drafts existen
test -f docs/grants/nsf-ai-safety-draft.md && echo "NSF: OK"
test -f docs/grants/gitcoin-quadratic-funding-draft.md && echo "GITCOIN: OK"
test -f docs/grants/ossf-draft.md && echo "OSSF: OK"
```

### Paso 2: Generar Package de Envío

```bash
# Ejecutar script de preparación
bash scripts/prepare_grant_submission.sh
# Output: grants-submission-v1.8.tar.gz + SHA256 checksums
```

### Paso 3: Verificar Checksums

```bash
# Verificar integridad del package
sha256sum -c grants-submission-v1.8.sha256
```

### Paso 4: Checklist Final

- [ ] Todos los drafts revisados
- [ ] Placeholders reemplazados con info real
- [ ] Firmas obtenidas donde requerido
- [ ] Checksums verificados
- [ ] Submission links actualizados
- [ ] Timeline de seguimiento establecido

---

## Timeline de Seguimiento

| Fecha | Acción | Grant | Responsable |
|-------|--------|-------|-------------|
| 2026-05-15 | Drafts completados | Todos | Auto |
| [PLACEHOLDER] | Review PI/Co-PI | NSF | [PLACEHOLDER] |
| [PLACEHOLDER] | Signal post publicado | Gitcoin | [PLACEHOLDER] |
| [PLACEHOLDER] | Security auditor contactado | OSSF | [PLACEHOLDER] |
| [PLACEHOLDER] | Envío NSF | NSF | [PLACEHOLDER] |
| [PLACEHOLDER] | Envío Gitcoin | Gitcoin | [PLACEHOLDER] |
| [PLACEHOLDER] | Envío OSSF | OSSF | [PLACEHOLDER] |

---

## Métricas de Progreso

| Métrica | Actual | Target |
|---------|--------|--------|
| Drafts completados | 3/3 | 3 |
| Placeholders resueltos | 0/15 | 15 |
| Firmas obtenidas | 0/3 | 3 |
| Grants enviados | 0/3 | 3 |
| Funding total target | $0 | $175K |

---

## Disclaimers

1. **PLACEHOLDERS:** Todos los campos `[PLACEHOLDER: ...]` requieren verificación humana antes del envío.
2. **URLS:** Los submission links son placeholders. Verificar URLs reales en los portals oficiales.
3. **MONTOS:** Los budgets son estimaciones basadas en los drafts. Confirmar con PI y compliance.
4. **TIMELINE:** Las deadlines son estimadas. Verificar con los programas de grants.

---

## Comandos Rápidos

```bash
# Verificar estado actual
grep -c "\[PLACEHOLDER" docs/grants/submission-tracker.md
# Resultado esperado: 0 antes de envío

# Generar package
bash scripts/prepare_grant_submission.sh

# Verificar checksums
sha256sum -c grants-submission-v1.8.sha256

# Contar grants listos
grep -c "Ready to Submit" docs/grants/submission-tracker.md
```

---

*Última actualización: 2026-05-15 | v1.8 Sprint 1*
