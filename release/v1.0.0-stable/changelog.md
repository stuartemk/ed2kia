# Changelog — ed2kIA v1.0.0 STABLE

All notable changes to this project will be documented in this file.

## [1.0.0] - 2026-05-05

### Summary
Consolidación oficial de Fases 1-9 en un único release estable. Unificación de feature flags, API coherente y 142+ tests validados.

### Added
- **Fase 1**: P2P (libp2p), SAE (Candle), Bridge (tensor/consciousness)
- **Fase 2**: Interpretación (FeatureAnalyzer, SemanticMap), Consenso (Merkle, Validator)
- **Fase 3**: Seguridad (WASM Sandbox, MemoryGuard), ZKP (ark-bn254), Human-in-the-loop
- **Fase 4**: Scaling (PeerManager, Bootstrap), RLHF, Web UI, Monitoring (Prometheus)
- **Fase 5**: Gobernanza (Proposal, Voting), Reputación (Ledger, Scoring), Ecosistema (HuggingFace sync), Bootstrap
- **Fase 6**: Interoperabilidad (TensorAdapter, Schema), Federación (FedAvg, Sync), Staking (Proof, Registry), API v2 (OpenAPI)
- **Fase 7**: Alignment Engine, Federation Bridge, Dynamic Trust Scoring, Schema Registry
- **Fase 8**: Marketplace (ResourceMatching), UI Backend (Axum), SLO Engine, Cross-Model Scaling, Continuous Alignment, SLA Enforcer
- **Fase 9**: Liquid Governance (delegación ponderada, anti-Sybil), Realtime UI (WebSocket/SSE), Async ZKP Federation (batch proofs, Merkle fallback)
- `src/lib.rs` con re-exports unificados y feature gating limpio
- Feature `stable` como default (incluye todos los módulos validados)
- Pipeline CI/CD unificado (`ci_cd_stable.yml`)

### Changed
- Version bump: `0.5.0` → `1.0.0`
- Feature flags estandarizados: `default = ["stable"]`
- Legacy flags (`phase6-experimental`, `phase8-sprint1`, etc.) mapeados a `stable` para compatibilidad
- `src/main.rs` consolidado con `#[cfg(feature = "stable")]` unificado
- CLI about text actualizado a v1.0.0 STABLE

### Deprecated
- `phase6-core`, `phase6-sprint2`, `phase6-experimental` → alias a `stable` (remover en v2.0.0)
- `phase7-sprint1`, `phase7-sprint2` → alias a `stable` (remover en v2.0.0)
- `phase8-sprint1`, `phase8-sprint2` → alias a `stable` (remover en v2.0.0)
- `phase9-sprint1` → alias a `stable` (remover en v2.0.0)
- `core-only` → alias a `stable` (remover en v2.0.0)

### Security
- WASM Sandbox con límites de memoria y detección de imports peligrosos
- MemoryGuard con detección de buffer overflow y leak tracking
- ZKP Verification con ark-bn254 y Merkle proof fallback
- Anti-Sybil detection en Liquid Governance (crypto signature + ASN/IP + voting history)
- Rate limiting en UI Backend (50 msg/s por sesión)
- Auth endpoints en API v2

### Ethics
- Licencia Apache 2.0 + Cláusula de Uso Ético
- Mandato de transparencia y auditabilidad
- Prohibición de backdoors y uso malicioso
- Compromiso con el progreso humano y desarrollo responsable de IA

### Performance
- 142 tests passing, 0 warnings, 0 errores
- Profile release: opt-level 3, LTO, codegen-units 1, strip
- Cross-compilation: x86_64, aarch64, Linux, macOS, Windows
- Docker multi-arch: linux/amd64, linux/arm64

---

## [0.9.0-alpha.1] - 2026-04-20
- Fase 9 Sprint 1: Liquid Governance, Realtime UI, Async ZKP Federation

## [0.8.0-alpha.2] - 2026-04-10
- Fase 8 Sprint 2: Cross-Model Scaling, Continuous Alignment, SLA Enforcer

## [0.8.0-alpha.1] - 2026-04-01
- Fase 8 Sprint 1: Marketplace, UI Backend, SLO Engine

## [0.7.0-alpha.2] - 2026-03-20
- Fase 7 Sprint 2: Feedback Loop, Dynamic Trust, Schema Registry

## [0.7.0-alpha.1] - 2026-03-10
- Fase 7 Sprint 1: Continuous Alignment, Cross-Net Federation Bridge

## [0.6.0-rc] - 2026-02-20
- Fase 6 Sprint 2: Staking, API v2, ONNX Adapter

## [0.5.0] - 2026-02-01
- Fase 6 Sprint 1: Interoperabilidad, FedAvg Aggregator

## [0.1.0] - 2026-01-01
- Proyecto inicial: P2P, SAE, Consenso básico

[1.0.0]: https://github.com/ed2kia/ed2kia/releases/tag/v1.0.0
