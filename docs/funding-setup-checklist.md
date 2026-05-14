# Funding Setup Checklist — ed2kIA v1.7+

**Fecha:** 2026-05-14
**Estado:** PRE-ACTIVATION
**Referencia:** [`SUPPORT.md`](../../SUPPORT.md), [`docs/funding-strategy.md`](funding-strategy.md)

---

## 1. GitHub Sponsors

### Pasos de Activación

1. Navegar a: https://github.com/sponsors/Stuartemk
2. Click en **"Set up Sponsors"** (si es la primera vez)
3. Configurar tiers:
   - **$5/mes:** Supporter — Name in SUPPORT.md
   - **$25/mes:** Backer — Badge in README + Discord role
   - **$100/mes:** Sponsor — Logo in README + priority issue review
   - **$500/mes:** Enterprise — Custom feature consultation
4. Verificar que el badge funcione:
   ```markdown
   [![Sponsor](https://img.shields.io/github/sponsors/Stuartemk)](https://github.com/sponsors/Stuartemk)
   ```
5. Link desde README.md ya configurado

### Validación

- [ ] Página de sponsors accesible
- [ ] Badge muestra contador correcto
- [ ] Tiers configurados y visibles
- [ ] Email de notificación activado

---

## 2. Open Collective

### Creación de Organización

1. Navegar a: https://opencollective.com/
2. Click **"Create a Collective"**
3. Tipo: **Open Source**
4. Nombre: **ed2kIA**
5. URL: `https://opencollective.com/ed2kIA`
6. Descripción: "Decentralized AI federation for transparent, verifiable machine learning"
7. Tags: `ai`, `machine-learning`, `decentralized`, `rust`, `open-source`

### Configuración de Multisig 2-of-3

**Requerimiento critico:** Todas las salidas requieren aprobacion de 2 de 3 signers.

1. Ir a **Settings → Financials → Host**
2. Configurar **multisig wallet**:
   - Signer 1: [CORE_TEAM_MEMBER_1] — Placeholder, requiere verificacion
   - Signer 2: [CORE_TEAM_MEMBER_2] — Placeholder, requiere verificacion
   - Signer 3: [CORE_TEAM_MEMBER_3] — Placeholder, requiere verificacion
3. Roles:
   - **Admin:** Aprobacion de gastos + gestion de signers
   - **Accountant:** Reportes financieros + transparencia
   - **Member:** Propuesta de gastos

### Disclaimer de Seguridad

> **IMPORTANTE:** Los signers del multisig deben ser miembros verificados del core team con identidad confirmada. Nunca compartir claves privadas. Usar hardware wallets (Ledger/Trezor) para las cuentas asociadas.

### Validacion

- [ ] Organizacion creada y verificada
- [ ] Multisig 2-of-3 configurado
- [ ] Roles asignados
- [ ] Link desde SUPPORT.md actualizado

---

## 3. Crypto Wallets

### Wallets Configuracion

**Disclaimer:** Las direcciones de wallet son placeholders. Reemplazar con direcciones reales verificadas antes de activar.

| Currency | Network | Address (PLACEHOLDER) | Status |
|----------|---------|----------------------|--------|
| BTC | Bitcoin | `bc1q...[VERIFICAR]` | PENDING |
| ETH | Ethereum | `0x...[VERIFICAR]` | PENDING |
| USDC | Ethereum (ERC-20) | `0x...[VERIFICAR]` | PENDING |

### Pasos de Verificacion

1. Crear wallets en hardware wallet (Ledger/Trezor recomendado)
2. Generar direcciones publicas separadas para cada currency
3. Verificar que las direcciones son validas (formato correcto)
4. Actualizar SUPPORT.md con direcciones reales
5. Hacer una transaccion de prueba (dust) para confirmar recepcion
6. Documentar en `docs/funding-strategy.md` la politica de gestion de fondos

### Seguridad

- [x] Usar hardware wallet para almacenamiento
- [x] Multisig 2-of-3 para salidas
- [x] Nunca compartir seed phrases
- [x] Backup de configuracion en lugar seguro
- [ ] Direcciones verificadas y publicadas

---

## 4. Gitcoin Grants

### Aplicacion a Rounds

1. Navegar a: https://gitcoin.co/grants
2. Click **"Apply to Round"**
3. Completar aplicacion:
   - **Project Name:** ed2kIA
   - **Tagline:** Decentralized AI federation with verifiable contributions
   - **Description:** Construir sobre la transparencia de Linux para IA distribuida
   - **Website:** https://github.com/Stuartemk/ed2kIA
   - **Socials:** Discord, Twitter (si aplican)
   - **Wallet:** Conectar wallet Ethereum verificada
   - **Categories:** `AI/ML`, `Infrastructure`, `Open Source`

### Quadratic Funding Setup

1. Configurar **matching cap:** $10,000 (recomendado para primer round)
2. Preparar **narrativa de impacto:**
   - Contribuidores verificados con Ed25519
   - Transparencia total en uso de fondos
   - Benchmarks publicos + audit trail
3. Activar **campaign:**
   - Posts en Discord/Twitter
   - Colaborar con otros proyectos en el round
   - Maximizar numero de donantes unicos (quadratic formula)

### Validacion

- [ ] Aplicacion enviada
- [ ] Wallet conectada y verificada
- [ ] Matching cap configurado
- [ ] Campaign activa

---

## 5. Corporate Sponsorship

### Tiers Disponibles

| Tier | Inversion | Beneficios |
|------|-----------|------------|
| Bronze | $1K/mes | Logo en SUPPORT.md + mentions |
| Silver | $5K/mes | Logo en README + issue priority |
| Gold | $10K/mes | Consultation + custom features |
| Platinum | $25K/mes | Strategic partnership + governance seat |

### Prospecting

- [ ] Identificar 10 empresas potenciales (AI/ML infra, cloud providers, crypto funds)
- [ ] Personalizar pitch por empresa
- [ ] Enviar emails de contacto
- [ ] Agendar calls de discovery

---

## 6. Verificacion Final

### Comandos de Validacion

```bash
# Verificar SUPPORT.md existe y contiene keywords criticas
grep -c "GitHub Sponsors\|Open Collective\|Gitcoin" SUPPORT.md
# Expected: >= 3

# Verificar funding-strategy.md existe
test -f docs/funding-strategy.md && echo "STRATEGY: OK" || echo "STRATEGY: MISSING"

# Verificar funding-setup-checklist.md existe
test -f docs/funding-setup-checklist.md && echo "CHECKLIST: OK" || echo "CHECKLIST: MISSING"

# Verificar script de verificacion
bash -n scripts/verify_funding_channels.sh && echo "SCRIPT: VALID" || echo "SCRIPT: INVALID"
```

### Checklist de Completitud

- [ ] GitHub Sponsors activado
- [ ] Open Collective creado + multisig configurado
- [ ] Crypto wallets verificadas (al menos BTC + ETH)
- [ ] Gitcoin application enviada
- [ ] SUPPORT.md actualizado con links reales
- [ ] Badges funcionando en README
- [ ] Script de verificacion pasa

---

## 7. Metricas de Exito

| Metrica | Target (Mes 1) | Target (Mes 3) |
|---------|----------------|----------------|
| GitHub Sponsors | 10 backers | 50 backers |
| Open Collective | $500 raised | $5K raised |
| Gitcoin Grants | Applied | $10K matched |
| Corporate Sponsors | 1 Bronze | 3+ tiers |
| Total Monthly | $1K | $10K |

---

## 8. Gobernanza & Transparencia

### Reportes Financieros

- **Semanal:** Dashboard interno con ingresos/gastos
- **Mensual:** Public report en `docs/transparency/` con breakdown
- **Trimestral:** Town hall + Q&A con la comunidad

### Uso de Fondos

| Categoria | % Budget | Descripcion |
|-----------|----------|-------------|
| Developer Compensation | 40% | Stipends para contribuidores verificados |
| Infrastructure | 25% | Hosting, CI/CD, tooling |
| Grants & Bounties | 20% | Programas de incentivos |
| Community | 10% | Eventos, outreach, educacion |
| Reserve | 5% | Emergencias + buffer |

### Aprobacion de Gastos

- **< $500:** 1 approver
- **$500 - $5K:** 2 approvers (multisig)
- **> $5K:** Governance proposal + community vote
