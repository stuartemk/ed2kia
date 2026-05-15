# ed2kIA v1.8.0-beta.1 — Release Notes

**Fecha:** 2026-05-15
**Tag:** `v1.8.0-beta.1`
**Codebase:** `2bc3c44` (main)
**License:** Apache 2.0 + Ethical Use Clause

---

## Resumen

v1.8.0-beta.1 es el primer beta oficial del sprint v1.8 "ChatGPT Moment". Introduce API Explorer para visualización 3D de conceptos SAE, Geographic Routing para optimización P2P, WASM Mobile Bridge para despliegue móvil, y herramientas DX (Justfile, Docker Compose dev).

**Tests:** 2935 passing | **Feature Flags:** 3 active | **Commits:** 50+ desde v1.6.0-stable

---

## Feature Gates

| Feature | Flag | Status | Tests |
|---------|------|--------|-------|
| Stable core | `--features stable` | ✅ Active | 2887+ |
| Sprint 1 (API Explorer, Reputation, QuantConfig, Async Steering) | `--features v1.8-sprint1` | ✅ Active | +48 |
| Sprint 2 (Geographic Routing, WASM Bridge) | `--features v1.8-sprint2` | ✅ Active | +45 |

### Activar Features

```bash
# Stable only
cargo test --features stable

# Sprint 1 features
cargo test --features v1.8-sprint1

# Sprint 2 features
cargo test --features v1.8-sprint2

# All features
cargo test --all-features
```

---

## Nuevas Features

### Sprint 1 — "ChatGPT Moment" Core

- **API Explorer v1** — REST endpoints para visualización 3D de conceptos SAE, activations, steering signals
- **Reputation Proof Schema** — Ed25519-based reputation proofs con 6 tiers (Bronze→Diamond), anti-Sybil
- **QuantConfig v3** — FP8/INT4 quantization config con clamp ranges, per-element/per-block modes
- **Async Steering v1** — Late correction signals para pipelines tensor distribuidos

### Sprint 2 — Geographic Routing & Mobile

- **Geographic Routing** — Haversine distance + RTT EMA scoring, KAD fallback, stale detection
- **WASM Mobile Bridge** — Memory-limited WASM (64MB), priority task queue, adaptive sync con battery awareness

### Developer Experience

- **Justfile** — 30+ recetas: `just build`, `just test`, `just validate`, `just docker-compose`
- **Docker Compose Dev** — 3 nodos P2P + Prometheus + Grafana
- **Mentorship Program** — 3 tiers (Seed/Sprout/Tree), onboarding automation

---

## Benchmark Deltas

| Metric | Baseline v1.7 | Beta v1.8 | Delta |
|--------|--------------|-----------|-------|
| SAE loader (dim 8192) | < 200ms | TBD (beta) | — |
| Tensor f32 serialization | < 50ms | TBD (beta) | — |
| Tensor fp8 serialization | < 20ms | TBD (beta) | — |

> **Nota:** Benchmarks reales se recolectarán durante el ciclo beta. Baseline en `benchmarks/results/baseline-v1.7.json`.

---

## Known Limitations

1. **8 pre-existing test failures** — Documentados desde v1.6.0, no regresión
2. **2 clippy style warnings** — `manual_range_contains` en `geographic_routing.rs`, no funcional
3. **WASM cross-compilation** — Requiere `rustup target add wasm32-unknown-unknown` (no incluido por defecto)
4. **Geographic routing** — Requiere datos lat/lon de peers; sin datos usa KAD fallback
5. **Coverage** — Target ≥80% pendiente de tooling (todo en CI script)

---

## Instalación

### Quick Start

```bash
# Clone
git clone https://github.com/Stuartemk/ed2kIA.git
cd ed2kIA

# Checkout beta tag
git checkout v1.8.0-beta.1

# Build
cargo build --features v1.8-sprint2

# Test
cargo test --features v1.8-sprint2

# Dev environment (optional)
just setup --full
just docker-compose up -d
```

### Requisitos

- Rust 1.70+ (`rustup install stable`)
- Cargo (incluido con Rust)
- Git 2.30+
- Docker + Docker Compose (opcional, dev environment)
- Just (opcional, command runner): `cargo install just`

---

## Rollback Instructions

### Rollback a v1.6.0-stable

```bash
# 1. Checkout stable tag
git checkout v1.6.0-stable

# 2. Rebuild
cargo build --features stable

# 3. Verify
cargo test --features stable

# 4. Revert feature flags en Cargo.toml si necesario
#    Eliminar features v1.8-sprint1 y v1.8-sprint2
```

### Rollback Individual Feature

```bash
# Desactivar feature problemático
# En Cargo.toml, comentar la feature en el list:
# [features]
# v1.8-sprint2 = []  # COMMENTED OUT

# Rebuild sin feature
cargo build --features stable
```

### Git Revert

```bash
# Revert commit específico
git revert <commit_hash> -m "revert: <original_message>"

# Push revert
git push origin main
```

---

## Validación CI

```bash
# Ejecutar validación completa
bash scripts/beta_ci_validation.sh

# Dry-run (sin push)
bash scripts/beta_ci_validation.sh --dry-run
```

---

## Canales de Feedback

- **Bug Report:** GitHub Issues → Label `beta-bug`
- **Feature Request:** GitHub Discussions → `beta-feedback`
- **Security:** SECURITY.md (disclosure responsable)
- **Soporte:** Discord #beta-testing

---

## Sign-off

| Role | Name | Status |
|------|------|--------|
| Release Engineer | Qweni | ✅ |
| Orchestrator | @Stuartemk | ⏳ Pending |

---

*v1.8.0-beta.1 — Geographic Routing, WASM Bridge & Async Steering*
*Generated: 2026-05-15*
