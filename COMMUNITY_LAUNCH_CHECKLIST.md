# Community Launch Checklist — ed2kIA v1.7.0-stable

**Fecha:** 2026-05-14
**Version:** v1.7.0-stable
**Estado:** PRE-LAUNCH

---

## Dia 0: Publicacion Inicial

### EleutherAI Discord

**Canal:** `#projects` o `#showcase`

**Mensaje:**
```
🚀 ed2kIA v1.7.0-stable — Decentralized AI Federation with Verifiable Contributions

Hi EleutherAI! We just released ed2kIA v1.7.0-stable, a Rust-based framework for
decentralized AI training with cryptographic verification.

Key features:
✅ SAE fine-tuning with cross-model gradient alignment (v7)
✅ FP8/INT4 quantization with <2% precision loss
✅ Async ZKP proof batching (v14) with adaptive routing
✅ Auto-Push CI/CD protocol for zero-friction contributions
✅ Reputation system with Ed25519 signed proofs

We're looking for contributors for our v1.8 "ChatGPT Moment" sprint:
👉 Good First Issues: https://github.com/Stuartemk/ed2kIA/blob/main/ISSUES_BATCH_V1.8.md
📖 Docs: https://github.com/Stuartemk/ed2kIA

Apache 2.0 licensed. Zero telemetry. Ethical AI focus.
```

**Checklist:**
- [ ] Mensaje publicado en EleutherAI Discord
- [ ] Respuestas monitoreadas (primeras 2h)
- [ ] Links verificados

### r/rust

**Titulo:** `ed2kIA v1.7 — Decentralized AI Federation in Rust (SAE fine-tuning, ZKP proofs, FP8 quantization)`

**Cuerpo:**
```
We released ed2kIA v1.7.0-stable, a Rust framework for decentralized AI training
with cryptographic verification. Built entirely in Rust with zero unsafe code.

Technical highlights:
- SAE Fine-Tuning v7: Distributed training with cross-model gradient alignment
- Quantization v3: FP8/INT4 with per-element scales, <2% MAPE
- Async ZKP v14: Adaptive proof batching with Merkle+VRF fallback
- Federation Scaling v7: Cross-model shard coordination with predictive load balancing

Performance targets:
- FP8 throughput: >500 MB/s
- INT4 throughput: >200 MB/s
- Async steering latency: <5ms

Looking for Rust contributors — we have good-first-issues ready:
https://github.com/Stuartemk/ed2kIA/blob/main/ISSUES_BATCH_V1.8.md

Repo: https://github.com/Stuartemk/ed2kIA
```

**Checklist:**
- [ ] Post publicado en r/rust
- [ ] AMAs respondidos
- [ ] Links a issues verificados

### Hugging Face Candle / Spaces

**Canal:** Hugging Face Discord `#community-projects` + post en HF Blog

**Mensaje:**
```
🔬 ed2kIA v1.7 — Verifiable AI Training Framework

Building transparent AI training with cryptographic proofs. ed2kIA v1.7 brings:

✅ SAE fine-tuning with distributed gradient sync
✅ FP8/INT4 quantization for efficient inference
✅ ZKP-based proof verification across federations
✅ Browser-based verification (WASM target in v1.8)

Perfect for researchers who want verifiable, reproducible ML training.

Good First Issues: https://github.com/Stuartemk/ed2kIA/blob/main/ISSUES_BATCH_V1.8.md
Repo: https://github.com/Stuartemk/ed2kIA
```

**Checklist:**
- [ ] Post en HF Discord
- [ ] Model card preparado (si aplica)
- [ ] Demo space planificado (v1.8)

### Twitter / X

**Tweet:**
```
🚀 ed2kIA v1.7.0-stable is live!

Decentralized AI training with cryptographic verification:
✅ SAE fine-tuning + cross-model alignment
✅ FP8/INT4 quantization (<2% loss)
✅ ZKP proof batching
✅ Auto-Push CI/CD

Contributing is easy — 10 good-first-issues ready:
https://github.com/Stuartemk/ed2kIA/blob/main/ISSUES_BATCH_V1.8.md

#OpenSource #AI #Rust #DecentralizedAI
```

**Checklist:**
- [ ] Tweet publicado
- [ ] Thread tecnico (opcional)
- [ ] Retweets de colaboradores

---

## Dia 1: Respuesta & Activacion

### Monitoreo de Issues

- [ ] Revisar nuevos issues/PRs
- [ ] Asignar labels a issues nuevos
- [ ] Responder a comentarios en posts
- [ ] Welcome message a primeros contribuidores

### Leaderboard Mock

- [ ] Activar leaderboard mock (documentado en reputation-gamification.md)
- [ ] Anunciar primer challenge: "First PR merged = Observer → Contributor"
- [ ] Preparar badges para primeros contribuidores

### Discord/Comunidad

- [ ] Crear canal `#v1.8-contributors` en Discord (si existe)
- [ ] Pinned message con links a issues
- [ ] AMA session (30 min)

---

## Dia 3: Primer Report de Metricas

### Metricas a Recopilar

| Metrica | Target Dia 3 |
|---------|-------------|
| GitHub Stars | +50 |
| Forks | +10 |
| Issues abiertos | 10 (batch) |
| PRs recibidos | 2+ |
| Discord miembros | +20 |
| Posts engagement | 100+ views |

### Ajuste de SLAs

- [ ] Revisar response time para issues (target: <24h)
- [ ] Ajustar capacity si hay mas/menos contribuidores
- [ ] Actualizar roadmap si hay feedback significativo

### Report

- [ ] Crear `docs/transparency/launch-week-1.md` con metricas
- [ ] Compartir en Discord + Twitter

---

## Dia 7: Revision Semanal & Escalado

### Revision

- [ ] Contar contribuidores nuevos
- [ ] Medir K-factor (invitaciones por contribuidor)
- [ ] Revisar funding recibido (si aplica)
- [ ] Evaluar readiness para v1.8 Sprint 1

### Escalado a v1.8 Sprint 1

**Criteria para iniciar Sprint 1:**
- [ ] 5+ contribuidores activos
- [ ] 2+ PRs merged
- [ ] Funding channel activo (al menos 1)
- [ ] Community engagement positivo

**Si criteria PASS:**
- [ ] Anunciar v1.8 Sprint 1 kickoff
- [ ] Activar issue assignment
- [ ] Programar weekly sync

**Si criteria FAIL:**
- [ ] Analizar bottlenecks
- [ ] Ajustar outreach strategy
- [ ] Extender Dia 0-7 por 1 semana mas

---

## Scripts de Ejecucion

Ver `scripts/post_community_launch.sh` para comandos automatizados.

---

## Contactos de Emergencia

| Rol | Contacto |
|-----|----------|
| Tech Lead | [CORE_TEAM_MEMBER_1] |
| Community Manager | [CORE_TEAM_MEMBER_2] |
| Security | security@ed2kIA.org |

---

## Sign-Off

| Item | Status |
|------|--------|
| Dia 0 posts | PENDING |
| Dia 1 monitoring | PENDING |
| Dia 3 metrics | PENDING |
| Dia 7 review | PENDING |
| v1.8 Sprint 1 ready | PENDING |

**Ready for launch:** YES (content prepared)
**Execution:** Manual (requires team member to post)
