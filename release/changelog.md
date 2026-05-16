# Changelog ed2kIA

Todas las cambios notables en este proyecto están documentados en este archivo.

El formato sigue [Semantic Versioning](https://semver.org/) y [Keep a Changelog](https://keepachangelog.com/).

---

## [Unreleased]

## [v1.9.0-stable] - 2026-05-16 (FASE 7 Complete — ZKP Aggregation, Neural Steer UI, OSSF Compliance, Community Scaling)

### ✨ Agregado

#### ZKP Proof Aggregation (FASE 72)
- Batch verification y commitment pooling para ZKP proofs
- AggregationBatch con límites de tamaño, finalización y verificación
- ProofAggregator multi-batch con cleanup automático
- AggregationMetrics: tiempos verificación, success rate, reduction ratio
- 33 tests unitarios

#### Neural Steer UI (FASE 72)
- Sliders éticos: empathy, creativity, safety con validación de bounds
- NeuralSteerConfig con serialización JSON y safety checks
- SteeringSignalBridge con soporte rollback
- Safety thresholds con detección automática de violaciones
- 31 tests unitarios

#### Security Audit & OSSF Compliance (FASE 76)
- Reporte OSSF completo: `docs/security/ossf-compliance-report.md`
- CVE scan: 22 findings (0 Critical, 3 High, 8 Medium, 11 Low)
- WASM sandbox verification: Cranelift-only, 256MB cap, minimal WASI
- OSSF Scorecard: 8.5/10 (Passing)
- SECURITY.md actualizado v1.9-stable

#### Community Infrastructure (FASE 73-74)
- First PR automation pipeline
- Feedback processing automation
- Auto-merge PR scripts
- Contributor funnel documentation

### 🔧 Cambios

- Feature flags: `v1.9-sprint1`, `v1.9-sprint2`
- FASE 68-76 completadas (8 commits)
- 64 tests nuevos agregados

### 🔒 Seguridad

- OSSF compliance audit completado
- 22 CVEs documentados con mitigaciones
- WASM sandbox verificado

---

## [v1.8.0-beta] - 2026-05-15 (ChatGPT Moment — API Explorer, Reputation Proofs, Geographic Routing, WASM Mobile)

### ✨ Agregado

#### API Explorer v1 (Sprint 1)
- REST endpoints para visualización 3D de conceptos SAE
- Activation records con tracking temporal
- Steering signals con ordenamiento por prioridad
- Rate limiting per-node con token bucket
- Ed25519 proof validation para API auth

#### Reputation Proof Schema (Sprint 1)
- Ed25519-based reputation proofs con 6 tiers (Bronze→Diamond)
- Proof batching con batch verification
- Anti-Sybil rate limiting por ventana temporal
- Metadata-attached proofs con content hashing

#### QuantConfig (Sprint 1)
- FP8/INT4 quantization config con clamp ranges
- Per-element y per-block quantization modes
- Payload reduction ratio tracking

#### Async Steering v1 (Sprint 1)
- Late correction signals para pipelines tensor distribuidos
- Priority-based signal scheduling
- Drop rate tracking con metrics

#### Geographic Routing (Sprint 2)
- Haversine distance calculation para peer proximity scoring
- RTT measurement con EMA smoothing
- Composite routing score (base - RTT penalty - distance penalty)
- KAD fallback cuando geo data es insuficiente
- Stale peer detection & cleanup

#### WASM Mobile Bridge (Sprint 2)
- Memory-limited WASM compilation (64MB default)
- Priority background task scheduler
- Adaptive P2P sync con battery awareness
- Sync timeout detection & recovery

#### Developer Experience
- Justfile con 30+ recetas de desarrollo
- devtools/setup.sh para setup automático
- devtools/docker-compose.yml (3 nodos + Prometheus + Grafana)
- Programa de mentorship (Seed/Sprout/Tree)
- Grant follow-up tracker & automation

### 🔧 Cambios

- Feature flags: `v1.8-sprint1`, `v1.8-sprint2`
- README.md con sección Local Development completa
- CONTRIBUTING.md con mentorship program

---

## [v1.6.0-stable] - 2026-05-14 (Fase 7-11: Release Preparation & GitHub Handover)

### ✨ Agregado

#### SAE Fine-Tuning v7
- Cross-model gradient alignment v5 con adaptive normalization
- Adaptive LR decay con convergence detection (patience-based)
- LZ4 checkpoint compression con incremental deltas
- Distributed fine-tuning engine con multi-model coordination
- 187 tests passing (160 unit + 27 E2E + 13 stress)

#### Cross-Model Federation Scaling v7
- Multi-model shard coordination con predictive load balancing
- Real-time metrics collection con adaptive thresholds
- Cross-model assignment validation
- Predictive capacity planning

#### Async ZKP v14
- Adaptive proof batching con parallel verification
- Merkle+VRF fallback verification
- Priority-based proof scheduling (critical/high/normal/background)
- Multi-federation load distribution

#### Federation ZKP Bridge v7
- Adaptive routing con credibility scoring
- Proof fallback verification
- Cross-federation proof coordination
- Consensus verification

#### UI Dashboard v7
- WebSocket streaming con real-time metrics
- Pool metrics visualization
- Federation health monitoring

#### GitHub Release Infrastructure
- CHANGELOG.md con Keep-a-Changelog format
- Issue/PR templates con ethical alignment checklist
- Labels & auto-management configuration
- Git release preparation scripts

### 🔧 Cambios

#### Versión y Features
- Versión normalizada a 1.6.0-stable en Cargo.toml, src/lib.rs, README.md
- Feature flags consolidados: v1.6-sprint[1-3] → stable
- Zero-breaking-changes migration desde v1.5.0

### 🔒 Security

- Zero unsafe code, zero telemetry, zero financial logic
- Apache 2.0 + Ethical Use Clause
- Responsible disclosure policy en SECURITY.md
- CODEOWNERS configurado para auto-assignment

### 🏛️ Governance

- CODEOWNERS configurado con auto-assignment patterns
- SECURITY.md con responsible disclosure (90-day window)
- PR templates con governance checklist
- Issue templates con ethical alignment requirements

### 📚 Documentation

- docs/architecture_v1.6.0.md - Architecture document
- docs/migration_guide_v1.5_to_v1.6.md - Migration guide
- docs/official_launch_announcement_v1.6.md - Launch announcement
- release/v1.6.0-stable/launch_preparation_signoff.json - JSON sign-off

---

## [v0.5.0] - 2024-XX-XX (Fase 5: Bootstrap, Gobernanza, Reputación & Ecosistema)

### ✨ Agregado

#### Gobernanza Descentralizada
- Sistema de propuestas P2P con firma criptográfica Ed25519 (`src/governance/proposal.rs`)
- Votación con time-lock de 72h, quórum ≥30% y ejecución automática (`src/governance/voting.rs`)
- CLI `govern`: propose, list, vote, result, stats
- Documentación completa en `docs/GOVERNANCE.md`

#### Reputación por Cómputo Verificado
- Ledger inmutable de contribuciones con redb (`src/reputation/ledger.rs`)
- Scoring con decay exponencial (50%/30d), multiplicador ZKP y anti-Sybil (`src/reputation/scoring.rs`)
- CLI `reputation`: status, leaderboard, decay
- Integración con sistema de gobernanza (reputación ≥0.7 para votar)

#### Integración con Ecosistema
- Sincronización con Hugging Face y ModelScope (`src/ecosystem/hf_sync.rs`)
- Registro local de modelos con versionado, checksums y rollback (`src/ecosystem/model_registry.rs`)
- CLI `sync`: download, list, verify, export
- Cache local con rate limiting y verificación SHA-256

#### Bootstrap de Red
- Seed registry con descubrimiento DNS + hardcoded + validación de salud (`src/bootstrap/seed_registry.rs`)
- Inicialización determinista con modo genesis (`src/bootstrap/network_init.rs`)
- CLI `bootstrap`: genesis, join, status, migrate
- Documentación en `docs/NETWORK_BOOTSTRAP.md`

#### Release & Distribución
- Script multiplataforma `release/packager.sh` (tar.gz, zip, checksums, firma Ed25519)
- Changelog semántico `release/changelog.md`
- Soporte para 7 targets: Linux (gnu/musl), Windows, macOS (amd64/arm64)

#### Documentación
- `docs/GOVERNANCE.md` - Carta de gobernanza completa
- `docs/CONTRIBUTING.md` - Guía para voluntarios
- `docs/NETWORK_BOOTSTRAP.md` - Procedimiento de lanzamiento

### 🔧 Cambios Técnicos
- Agregadas dependencias: `ed25519-dalek 2.1`, `reqwest 0.11`, `chrono 0.4`, `fastrand 2.1`
- CLI extendido con comandos: `govern`, `reputation`, `sync`, `bootstrap`, `release`
- Integración `tracing` para logs de gobernanza y reputación

### 🔒 Seguridad
- Firmas Ed25519 para propuestas de gobernanza
- Hashes SHA-256 para integridad de ledger y modelos
- Protección anti-Sybil: límite por IP/ASN + deduplicación por batch
- Verificación de checksums en descargas de ecosistema

---

## [v0.4.0] - 2024-XX-XX (Fase 4: Escalado, RLHF, Web UI & Producción)

### ✨ Agregado
- Peer Manager con scoring dinámico y límites de conexión adaptativos
- Bootstrap Manager con descubrimiento DNS + AutoNAT
- Feedback Store con redb + export JSONL (llama.cpp/vLLM compatible)
- Trainer Loop con drift detection semántico
- Web Server con axum + tower-http
- Dashboard Web UI con Alpine.js
- Métricas Prometheus (counters/gauges/histograms)
- Health checks pluggable
- CI/CD pipeline (test, cross-compile, Docker, release, audit)

---

## [v0.3.0] - 2024-XX-XX (Fase 3: Seguridad, ZKP, Human-in-the-Loop & Deploy)

### ✨ Agregado
- WASM Sandbox con wasmtime (límites de memoria, detección de escapes)
- ZKP con Pedersen commitments + Fiat-Shamir en BN254
- Human Feedback CLI (modo interactivo TTY + batch JSON)
- Concept Updater con quorum de votación
- Docker multi-stage build + docker-compose
- Systemd service templates + install script

---

## [v0.2.0] - 2024-XX-XX (Fase 2: Interpretación, Feedback & Consenso)

### ✨ Agregado
- Feature Analyzer (análisis SAE + detección anomalías)
- Semantic Map (mapeo feature→concepto Qwen-Scope)
- Merkle tree (generación/verificación de raíces)
- Consensus Validator (validación asíncrona + umbrales)
- Consciousness Bridge (agregación + conflictos + steering)
- GossipSub en swarm P2P

---

## [v0.1.0] - 2024-XX-XX (Fase 1: Core P2P + SAE Loader + Tensor Routing)

### ✨ Agregado
- Estructura de proyecto Rust
- CLI con Clap (join, status, exit)
- Swarm P2P con libp2p (KAD + mDNS)
- Protocolo de mensajes (TensorRequest/Response, Leases, Steering)
- SAE Loader con Candle (.safetensors)
- LayerRouter con sharding dinámico y leases
- Tensor Flow Pipeline (Node A → Node B)

---

## Próximas Versiones

### [v0.6.0] - Fase 6 (Planificado)
- Interoperabilidad cross-model (SAEs entre diferentes LLMs)
- Alignment continuo con feedback humano automatizado
- Federación con otras redes voluntarias
- Dashboard web de gobernanza
- Health checks reales con reqwest async
- Integración con datasets library de Hugging Face
- Benchmarks de rendimiento P2P

---

[v0.5.0]: https://github.com/ed2kia/ed2kIA/compare/v0.4.0...v0.5.0
[v0.4.0]: https://github.com/ed2kia/ed2kIA/compare/v0.3.0...v0.4.0
[v0.3.0]: https://github.com/ed2kia/ed2kIA/compare/v0.2.0...v0.3.0
[v0.2.0]: https://github.com/ed2kia/ed2kIA/compare/v0.1.0...v0.2.0
[v0.1.0]: https://github.com/ed2kia/ed2kIA/releases/tag/v0.1.0
