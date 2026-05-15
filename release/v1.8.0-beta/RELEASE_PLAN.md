# v1.8.0-beta Release Plan — ed2kIA

**Fecha:** 2026-05-15 | **Estado:** Preparing | **Responsable:** Release Engineering

---

## Resumen del Release

v1.8.0-beta introduce el "ChatGPT Moment" de ed2kIA: API Explorer para visualización 3D de conceptos SAE, sistema de Reputation Proofs con firmas Ed25519, Geographic Routing para optimización P2P basada en proximidad geográfica, y WASM Mobile Bridge para despliegue en dispositivos móviles.

---

## Features Principales

### Sprint 1 — "ChatGPT Moment" Core

| Feature | Módulo | Descripción |
|---------|--------|-------------|
| **API Explorer v1** | `api/explorer_v1.rs` | REST endpoints para visualización 3D de conceptos SAE, activations, y steering signals |
| **Reputation Proof Schema** | `reputation/proof_schema.rs` | Ed25519-based reputation proofs con sistema de tiers (Bronze→Diamond) |
| **QuantConfig** | `bridge/quantization.rs` | Configuración de cuantización FP8/INT4 con clamp ranges |
| **Async Steering v1** | `protocol/async_steering.rs` | Late correction signals para pipelines tensor distribuidos |

### Sprint 2 — Geographic Routing & Mobile

| Feature | Módulo | Descripción |
|---------|--------|-------------|
| **Geographic Routing** | `p2p/geographic_routing.rs` | Haversine distance + RTT scoring para peer selection optimizada |
| **WASM Mobile Bridge** | `wasm/mobile_bridge.rs` | Adaptador P2P ligero para WASM con memory sandbox y adaptive sync |

### Developer Experience

| Feature | Archivo | Descripción |
|---------|---------|-------------|
| **Justfile** | `justfile` | Command runner con 30+ recetas de desarrollo |
| **DevTools Setup** | `devtools/setup.sh` | Setup automático del entorno de desarrollo |
| **Docker Compose Dev** | `devtools/docker-compose.yml` | 3 nodos P2P + Prometheus + Grafana |
| **Mentorship Program** | `CONTRIBUTING.md` | Programa de mentorship con 3 niveles (Seed/Sprout/Tree) |
| **Grant Follow-up** | `docs/grants/follow-up-tracker.md` | Tracker de seguimiento post-envío de grants |

---

## Validación Pre-Release

### Checklist Técnico

- [x] `cargo check --features v1.8-sprint1` — PASS
- [x] `cargo check --features v1.8-sprint2` — PASS
- [x] `cargo clippy --features v1.8-sprint1` — PASS (0 errors)
- [x] `cargo clippy --features v1.8-sprint2` — PASS (0 errors, 2 style warnings)
- [x] `cargo test --features v1.8-sprint1` — PASS (2935 passed)
- [x] `cargo test --features v1.8-sprint2` — PASS (2935 passed)
- [ ] `cargo audit` — PENDING
- [ ] Cross-compilation (wasm32-unknown-unknown) — PENDING
- [ ] Docker build validation — PENDING

### Checklist Documentación

- [x] CHANGELOG.md actualizado
- [x] README.md con sección Local Development
- [x] CONTRIBUTING.md con mentorship program
- [x] Grant follow-up tracker creado
- [ ] RELEASE_NOTES.md generado
- [ ] Migration guide v1.6→v1.8 — PENDING

### Checklist Comunidad

- [ ] Signal post en Gitcoin Forum
- [ ] Discord announcement
- [ ] Twitter/X announcement
- [ ] GitHub Release draft
- [ ] Beta tester onboarding

---

## Timeline

| Fase | Fecha | Duración | Entregable |
|------|-------|----------|------------|
| **Beta Prep** | 2026-05-15 | 1 día | RELEASE_PLAN.md, validation |
| **Beta Release** | 2026-05-16 | 1 día | GitHub Release v1.8.0-beta |
| **Beta Testing** | 2026-05-16..22 | 7 días | Feedback collection, bug fixes |
| **RC Prep** | 2026-05-23 | 1 día | Release candidate |
| **Stable Release** | 2026-05-25 | 1 día | v1.8.0-stable |

---

## Risk Assessment

| Risk | Impact | Probabilidad | Mitigación |
|------|--------|-------------|------------|
| WASM compilation failures | High | Medium | Feature-gated, no blocking |
| Geographic data privacy | Medium | Low | Optional, opt-in, no PII stored |
| Beta tester adoption | Medium | Medium | Mentorship program, onboarding automation |
| Grant submission delays | Low | High | Follow-up tracker, automated reminders |

---

## Rollback Plan

```bash
# Rollback a v1.6.0-stable
git checkout v1.6.0-stable
cargo build --release
# Redeploy con versión anterior
```

---

## Sign-off Criteria

- [ ] All validation checks PASS
- [ ] Zero critical bugs open
- [ ] Documentation complete
- [ ] Community notified
- [ ] Release artifacts generated (checksums, signatures)
- [ ] Rollback plan tested

---

## Comandos de Release

```bash
# Validación completa
just release-check

# Generar artifacts
just release-pack

# Tag release
git tag -a v1.8.0-beta -m "v1.8.0-beta: ChatGPT Moment"
git push origin v1.8.0-beta

# GitHub Release
bash scripts/github_release.sh v1.8.0-beta
```
