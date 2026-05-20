# ed2kIA - Red Descentralizada de Interpretabilidad

> **Imagina que los LLMs son sistemas complejos que requieren auditoría colaborativa. ed2kIA es una red de verificadores voluntarios distribuidos por todo el mundo que trabajan juntos para hacer estos sistemas más comprensibles, uno a uno, de forma transparente y verificable.**

> **Red descentralizada de código abierto para análisis interpretativo distribuido de LLMs usando Sparse Autoencoders (Qwen-Scope)**

[![License](https://img.shields.io/badge/License-Apache%202.0%20%2B%20Ethical-blue)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2021-orange)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/Version-2.1.0-sprint16.4-yellowgreen)](CHANGELOG.md)
[![Deploy](https://img.shields.io/badge/GitHub%20Pages-Active-brightgreen)](https://ed2kia.github.io/ed2kIA)
[![Tests](https://img.shields.io/badge/Tests-3216_passing-success)](CHANGELOG.md)
[![Qwen-Scope](https://img.shields.io/badge/Qwen--Scope--SAE-Integrated-brightgreen)](src/sae/qwen_scope_sae.rs)
[![Coverage](https://img.shields.io/badge/Coverage-≥80%25-tracking)](release/v2.0.0-stable/final-signoff.json)
[![OSSF](https://img.shields.io/badge/OSSF-8.5%2F10-passing)](security/audit_v2.0_sprint2.md)
[![Mode](https://img.shields.io/badge/Mode-STEWARDSHIP-blueviolet)](docs/governance/project-constitution.md)
[![CI](https://github.com/ed2kia/ed2kIA/actions/workflows/ci.yml/badge.svg)](https://github.com/ed2kia/ed2kIA/actions)

## 🌍 Mandato Ético

Este proyecto es de código abierto, transparente y diseñado exclusivamente para el progreso humano y el desarrollo responsable de la IA. Todo el código es auditable, libre de backdoors y compatible con infraestructura voluntaria global.

**Licencia:** Apache 2.0 + Cláusula de Uso Ético (bienestar humano/IA)

## 🌍 La Misión de ed2kIA: Alinear a los Gigantes mediante Verificación Comunitaria

Hoy en día, organizaciones como Google, OpenAI o Meta desarrollan las Inteligencias Artificiales más potentes del planeta. Estos sistemas son complejos: la comunidad científica busca comprender cómo toman decisiones, en qué se basan y cómo garantizar su alineación ética. **ed2kIA contribuye a este esfuerzo colectivo.**

### 🤝 ¿Cómo democratizamos el acceso a la interpretabilidad?
* **Un Supercomputador Colaborativo:** Unimos las computadoras y teléfonos de miles de personas en todo el mundo para crear una red de verificación distribuida y transparente.
* **Puente de Transparencia:** Nuestro software analiza las matemáticas complejas de la IA y las traduce a información que cualquier investigador puede auditar. Es un puente de transparencia y alineación verificable.
* **Acceso Abierto para Todos:** Al ser un proyecto 100% abierto y comunitario, democratizamos el acceso a la interpretabilidad. Cualquier estudiante o ciudadano podrá participar en la auditoría colaborativa de sistemas de IA para promover la seguridad, equidad y transparencia.

No necesitas ser un científico para contribuir al futuro. Al compartir un poco de la potencia de tu PC o tu teléfono, te conviertes en parte de una red global de verificación colaborativa.

## 📐 Arquitectura

### Decisiones Arquitectónicas Fijas

| Decisión | Implementación |
|----------|----------------|
| **Multiplataforma** | Windows/Linux/macOS desde Fase 1 |
| **Sharding** | Dinámico con Leases (5-10 min) gestionado por `LayerRouter` |
| **Comunicación** | Feedback Asincrónico + Steering Signals síncronos ligeros |
| **ZKP/WASM** | Implementación completa con multi-curve (BN254, BLS12-381, Pasta) |
| **Red P2P** | `libp2p` con KAD + mDNS para descubrimiento |
| **ML Engine** | `candle-core` + `candle-nn` + `safetensors` |
| **Serialización** | Prost (Protobuf) para metadatos, FlatBuffers para tensores |
| **GUI Desktop** | Tauri scaffold con Neural Steering UI |
| **Observabilidad** | Prometheus/Grafana metrics (feature-gated `v2.1-observability`) |

### Feature Gates v2.1 (Post-RFC)

| Feature Gate | Módulo | Status |
|--------------|--------|--------|
| `v2.1-mvp-core` | MVP Core Loop — Discovery → Distribution → Inference → Collection | ✅ Implementado (27 tests) |
| `v2.1-wasm-browser` | Browser Node — WASM P2P para navegadores | ✅ Pipeline CI/CD |
| `v2.1-wasm-workers` | Web Worker Offloading — Async inference dispatch sin bloquear UI | ✅ Implementado (2 tests) |
| `v2.1-webrtc-relay` | WebRTC + Relay Transport — libp2p WASM con Circuit Relay v2 | ✅ Implementado (13 tests) |
| `v2.1-wasm-telemetry` | WASM Telemetry Bridge — wasm-bindgen CustomEvent → DOM (4 eventos) | ✅ Implementado |
| `v2.1-relay-server` | Relay Server ("El Faro") — WebRTC/Circuit Relay v2 signaling | ✅ Implementado (14 tests) |
| `v2.1-wasm-micro-sharding` | WASM Micro-Sharding — Tensor chunking ≤50MB para wasm32 | ✅ Implementado (23 tests) |
| `v2.1-qwen-scope-sae` | Qwen Scope SAE — Top-k Sparse Autoencoder (4-tensor) | ✅ Implementado (10 tests) |
| `v2.1-qwen-scope-loader` | Safetensors Loader + WASM Micro-Sharding | ✅ Implementado (12 tests) |
| `v2.1-audit-payloads` | Audit Payloads — bincode serialization for P2P audit | ✅ Implementado (14 tests) |
| `v2.1-orchestrator` | Orchestrator Node — Native orchestrator with libp2p swarm + mpsc queues | ✅ Implementado (5 tests) |
| `v2.1-task-manager` | Task Manager — Dispatch/aggregation with timeout retry + progress events | ✅ Implementado (9 tests) |
| `v2.1-docker-deploy` | Docker Deploy — Multi-stage Dockerfile + orchestrator-node in docker-compose | ✅ Implementado |
| `v2.1-semantic-graph` | Semantic Graph — petgraph + dashmap for token↔feature mapping | ✅ Implementado (9 tests) |
| `v2.1-rosetta-api` | Rosetta API — axum HTTP endpoints for graph queries | ✅ Implementado |
| `v2.1-atlas-ui` | Atlas UI — 3D force-graph visualizer (WebGL) | ✅ Implementado |
| `v2.1-task-redundancy` | N-Node Dispatch — Configurable replication_factor for redundant task assignment | ✅ Validado (E2E, 5 tests) |
| `v2.1-consensus-engine` | Consensus Engine — O(N) index-hash grouping + epsilon-tolerant majority rule | ✅ Validado (E2E, 10 tests) |
| `v2.1-reputation-system` | Reputation Matrix — DashMap scores (+1/-50) + auto-ban on negative | ✅ Validado (E2E, 13 tests) |
| `v2.1-hf-bridge` | HuggingFace Streaming Bridge — Progressive .safetensors ingestion via reqwest bytes_stream + SHA256 | ✅ Validado (11 tests) |
| `v2.1-merit-system` | Cryptographic Merit System — Ed25519-signed proofs, ethical recognition, zero financial logic | ✅ Validado (24 tests) |
| `v2.1-portal-prod` | Production Portal — Alpine.js dashboard, browser node connection, real-time stats, merit badges | ✅ Validado |
| `v2.1-sybil-micropow` | Ethical Sybil Resistance — SHA-256 Micro-PoW handshake, rate limiting, exponential backoff | ✅ Validado (12 tests) |
| `v2.1-orchestrator-federation` | GossipSub Federation — libp2p 0.53 `MessageAuthenticity::Signed`, multi-node orchestrator mesh | ✅ Validado (9 tests) |
| `v2.1-rlhf-bridge` | RLHF Feedback Bridge — Human-in-the-loop semantic alignment via REST API + interactive UI | ✅ Validado (11 tests) |
| `v2.1-observability` | Prometheus Metrics — Ed2kMetrics registry, 5 categories (consensus/reputation/network/rlhf/wasm), 12 tests | ✅ Implementado (12 tests) |
| `v2.1-stewardship` | Stewardship Dashboard — Alpine.js governance dashboard (Network/Governance/Audit panels) | ✅ Implementado |
| `v2.1-rfc-pipeline` | RFC Triage Workflow — Auto-label, milestone assign, voting guide comments | ✅ Implementado |
| `v2.1-mainnet-bootstrap` | Mainnet Bootstrap — Docker Compose launch + healthchecks + pre-launch validation | ✅ Implementado |
| `v2.1-load-testing` | Load Testing — Concurrent WASM node stress tests + metrics capture (p95, throughput, CPU, memory) | ✅ Implementado |
| `v2.1-fuzzing` | Property-Based Fuzzing — proptest for consensus/reputation/sybil invariants | ✅ Implementado |
| `v2.1-tauri-bridge` | Tauri Desktop Bridge — Cross-platform client (WASM ↔ Tauri IPC ↔ Rust) | ✅ Implementado |
| `v2.1-federated-agg` | Federated Aggregation — FedAvg + differential privacy (ε=1.0, δ=1e-5), Ed25519 verification | ✅ Implementado |
| `v2.1-sae-training` | SAE Training Pipeline — Distributed training loop with candle-core, checkpointing, hooks | ✅ Implementado |
| `v2.1-ethical-audit` | Ethical Compliance Audit — Automated verification of ethical clause, zero financial logic | ✅ Implementado |
| `v2.1-chaos-engine` | Chaos Engine — Async fault injection (WASM failure, partition, latency, malicious votes, queue saturation) | ✅ Implementado |
| `v2.1-operator-cli` | Operator CLI Wizard — Standalone TUI (clap + dialoguer) for guided node setup | ✅ Implementado |
| `v2.1-auto-remediation` | Auto-Remediation Pipeline — Automated incident response with monitoring, restart, rollback, reporting | ✅ Implementado |
| `v2.1-governance` | CODEOWNERS + GOVERNANCE §§12-13 — Observability transparency & Pre-Launch Validation | ✅ Implementado |
| `v2.1-launch-readiness` | Pre-Launch Checklist — Automated 5-phase validation script + readiness report | ✅ Implementado |
| `v2.1-agg-committees` | Hierarchical Committees — Reputation-based + VRF-based selectors, LocalAggregator, GlobalMesh | ✅ Implementado (14 tests) |
| `v2.1-staleness-aware` | Staleness-Aware Weighting — Exponential decay `w=1/(1+tau)^alpha`, StalenessConfig | ✅ Implementado (18 tests) |
| `v2.1-bft-aggregation` | BFT Aggregation — Coordinate-wise median, Multi-Krum, MAD-based outlier filtering | ✅ Implementado (16 tests) |
| `v2.1-qlora-gguf` | QLoRA/GGUF — GGUF mmap loader, QLoRA forward pass, zstd P2P payloads (Law 3) | ✅ Implementado (33 tests) |
| `v2.1-proof-of-comprehension` | Proof of Comprehension — Cryptographic proof of useful work via SAE activations (Law 2) | 🏗️ Scaffold (Sprint16) |
| `v2.1-stuartian-filter` | Stuartian Filter — Deterministic alignment filter with KL divergence detection (Law 2) | 🏗️ Scaffold (Sprint16) |
| `v2.1-async-gossip-crdt` | Async Gossip with CRDTs — Partition-tolerant GossipSub with conflict-free convergence (Law 5) | 🏗️ Scaffold (Sprint16) |
| `v2.1-security-hardening` | wasmtime ≥24.0.7, rustls-webpki ≥0.103.13 | Planificado Q2-Q3 2027 |
| `v2.1-gui` | GUI Bridge, Mobile, 3D Visualizer | Draft |
| `v2.1-zkp-v3` | ZKP v3, Recursive Prover, Cross-Chain | Draft |
| `v2.1-enterprise` | SSO, K8s Operator, Compliance | Draft |

> **Nota:** Los feature gates `v2.1-*` NO están incluidos en `default = ["stable"]`. Requieren activación explícita vía RFC comunitario.

## 🧬 Kernel Estuardiano & Arquitectura v2.1 (Sprint16)

**ed2kIA v2.1.0-sprint16** internaliza el Kernel Estuardiano como ley base: 5 leyes estuardianas mapeadas directamente a decisiones técnicas a través de 4 módulos feature-gated.

### Leyes Estuardianas → Decisiones Técnicas

| Ley | Principio | Decisión Técnica |
|-----|-----------|------------------|
| Law 1 (Diversidad) | P2P puro, sin maestros | GossipSub mesh dinámico, sin nodos maestros |
| Law 2 (Error) | Reconocimiento de error | SAEs, validación de gradientes, auditoría transparente |
| Law 3 (Holística) | Cero desperdicio computacional | QLoRA/GGUF, payloads ≤MB, eficiencia termodinámica |
| Law 4 (Simbiosis) | Existencia simbiótica | WASM en navegador, hardware modesto, fricción cero |
| Law 5 (Posibilidades) | Múltiples posibilidades | Async, tolerancia a particiones, CRDTs, eventual consistency |

### Flujo Arquitectónico

```
┌─────────────────────────────────────────────────────────────────┐
│                     Kernel Estuardiano v2.1                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────┐    ┌──────────────────┐    ┌──────────────┐  │
│  │ QLoRA/GGUF   │───▶│ Stuartian Filter │───▶│ Async Gossip │  │
│  │ (Law 3)      │    │ (Law 2)          │    │ + CRDTs      │  │
│  │              │    │                  │    │ (Law 5)      │  │
│  │ GGUF base    │    │ KL divergence    │    │ GossipSub    │  │
│  │ QLoRA diff   │    │ Alignment check  │    │ Offline cache│  │
│  │ ≤MB payload  │    │ Reputation slash │    │ CRDT merge   │  │
│  └──────────────┘    └──────────────────┘    └──────────────┘  │
│         │                    │                    │             │
│         ▼                    ▼                    ▼             │
│  ┌────────────────────────────────────────────────────────┐    │
│  │         Proof of Comprehension (Law 2)                 │    │
│  │  SAE activation batches → Gradient validation → Proof │    │
│  └────────────────────────────────────────────────────────┘    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Módulos v2.1

| Módulo | Feature Gate | Archivos | Estado |
|--------|-------------|----------|--------|
| QLoRA/GGUF | `v2.1-qlora-gguf` | `loader.rs`, `adapter.rs`, `payload.rs` | ✅ Implementado |
| Proof of Comprehension | `v2.1-proof-of-comprehension` | `task.rs`, `verifier.rs` | 🏗️ Scaffold |
| Stuartian Filter | `v2.1-stuartian-filter` | `divergence.rs`, `slashing.rs` | 🏗️ Scaffold |
| SCT (Stuartian Context Tensor) | `v2.1-sct-core`, `v2.1-sct-reward`, `v2.1-sct-guard` | `sct_core.rs`, `sct_reward.rs`, `sct_guard.rs` | ✅ Implementado (37 tests) |
| Async Gossip + CRDTs | `v2.1-async-gossip-crdt` | `mesh.rs`, `cache.rs`, `crdt.rs` | 🏗️ Scaffold |

### 🎯 Stuartian Context Tensor (SCT) — Sprint16.3

El SCT reemplaza la alineación 2D (RLHF) con un tensor tridimensional `(X, Y, Z)`:

| Eje | Nombre | Rango | Activación | Significado |
|-----|--------|-------|------------|-------------|
| X | Beneficio | `[0, 1]` | Sigmoid | Utilidad percibida del output |
| Y | Costo/Fricción | `[0, 1]` | Sigmoid | Esfuerzo requerido del usuario |
| Z | Foco Estuardiano | `[-1, 1]` | Tanh | Alineación ética (Superior +1 ↔ Inferior -1) |

**Regla de Oro:** `if Z < 0 → REJECTED` (rechazo determinista, sin excepciones)

```bash
# Ejecutar tests SCT
cargo test --lib --features "v2.1-sct-core,v2.1-sct-reward,v2.1-sct-guard" -- sct
```

> **Nota:** Los módulos se encuentran en fase de scaffold (estructura pura, cero lógica de negocio). La implementación módulo por módulo se realizará en sprints subsiguientes (Sprint16.1 → Sprint16.4).

## ⚡ Hardening & Cross-Platform (Sprint13)

**ed2kIA v2.1.0-sprint13** introduce infraestructura de hardening para escalabilidad y resiliencia de mainnet:

### Load Testing — `v2.1-load-testing`

Pruebas de estrés concurrentes con nodos WASM simulados:

```bash
# Ejecutar stress tests con control de recursos
cargo test --test stress_test --features "v2.1-load-testing" -- --test-threads=4
```

- **Métricas:** p95 latencia, throughput (tasks/s), footprint de memoria, uso de CPU, tasa de slashing
- **Control de recursos:** `--test-threads=4`, límites de iteración para CI, `tokio::time::timeout`

### Property-Based Fuzzing — `v2.1-fuzzing`

Fuzzing basado en propiedades con `proptest` para invariantes criptográficos:

```bash
# Ejecuar fuzz tests (1000 casos, single-threaded)
cargo test --test consensus_fuzz --features "v2.1-fuzzing,v2.1-consensus-engine,v2.1-reputation-system,v2.1-sybil-micropow,v1.7-sprint1" -- --test-threads=1
```

- **Consensus:** Determinismo, tolerancia Byzantine (≤f/3 maliciosos)
- **Reputation:** Monotonía de scores, ban persistente sin unban explícito
- **Sybil:** Rechazo de nonces inválidos, rate-limiting activo

### Tauri Desktop Bridge — `v2.1-tauri-bridge`

Cliente desktop cross-platform (WASM ↔ Tauri IPC ↔ Rust):

```
┌──────────────────────────────────────────────────┐
│  Frontend (web/)                                 │
│  ┌─────────────┐  ┌──────────────────────────┐  │
│  │ Atlas 3D    │  │ Stewardship Dashboard    │  │
│  │ Visualizer  │  │ (Alpine.js)              │  │
│  └──────┬──────┘  └────────────┬─────────────┘  │
│         │                      │                 │
│         └──────────┬───────────┘                 │
│                    │ Tauri IPC                    │
├────────────────────┼─────────────────────────────┤
│  Backend (Rust)    │                              │
│  ┌─────────────────▼────────────────────────┐   │
│  │ Commands: start_worker, stop_worker,     │   │
│  │         sync_atlas, get_merit_proof      │   │
│  └──────────────────────────────────────────┘   │
└──────────────────────────────────────────────────┘
```

- **Sandboxed:** Sin telemetría externa, permisos mínimos
- **Multi-plataforma:** Windows, Linux, macOS
- **Build:** Multi-stage Docker, bundles nativos

## 🧠 Aprendizaje Federado & Alineación Continua (Sprint14)

**ed2kIA v2.1.0-sprint14** introduce infraestructura de aprendizaje federado con privacidad diferencial y auditoría ética automatizada:

### Secure Gradient Aggregation — `v2.1-federated-agg`

Agregación segura de gradientes con FedAvg ponderado por reputación y privacidad diferencial:

```bash
# Verificar compilación del agregador federado
cargo check --features "v2.1-federated-agg"
```

- **FedAvg adaptado:** Promedio ponderado por `reputation_score`, compresión INT8/FP8
- **Privacidad diferencial:** Ruido Gaussiano (ε=1.0, δ=1e-5) calibrado por sensibilidad L-infinito
- **Verificación Ed25519:** Firmas criptográficas para cada actualización de gradiente
- **Anti-poisoning:** Rechazo por umbral de divergencia respecto a la media actual

### Distributed SAE Training Pipeline — `v2.1-sae-training`

Pipeline de entrenamiento distribuido compatible con candle-core:

```bash
# Verificar compilación del pipeline de entrenamiento
cargo check --features "v2.1-sae-training"
```

- **Fases:** forward pass → sparsity mask → backward pass → gradient clipping → compresión INT8
- **Checkpointing automático:** Cada N pasos con estado completo (epoch, step, loss, best_loss)
- **Hooks de validación:** `on_step`, `on_epoch`, `on_convergence`
- **Early stopping:** Patiencia configurable + tolerancia de convergencia

### Automated Ethical Compliance — `v2.1-ethical-audit`

Auditoría automatizada de cumplimiento ético:

```bash
# Ejecutar auditoría ética
bash scripts/verify-ethical-compliance.sh
```

- **Cláusula ética:** Validación de patrones éticos en LICENSE
- **Zero financial logic:** Escaneo de patrones financieros prohibidos en src/
- **Zero telemetry:** Validación de ausencia de telemetría externa
- **Reporte:** Generación automática de `docs/ethical-compliance-report.md`
- **Salida:** 🟢 ÉTICA VALIDADA o 🔴 BLOQUEADO: [infracciones]

```
┌─────────────────────────────────────────────────────┐
│  Federated Learning Architecture                    │
│                                                      │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐      │
│  │ WASM Node│    │ WASM Node│    │ WASM Node│      │
│  │ (Train)  │    │ (Train)  │    │ (Train)  │      │
│  └────┬─────┘    └────┬─────┘    └────┬─────┘      │
│       │ gradients     │ gradients     │ gradients   │
│       │ (Ed25519)     │ (Ed25519)     │ (Ed25519)   │
│       └───────────────┼───────────────┘             │
│                      │                              │
│             ┌────────▼────────┐                     │
│             │ FederatedAgg    │                     │
│             │ FedAvg + DP     │                     │
│             │ (ε=1.0, δ=1e-5) │                     │
│             └────────┬────────┘                     │
│                      │ aggregated weights            │
│             ┌────────▼────────┐                     │
│             │ SAE Model       │                     │
│             │ (Community)     │                     │
│             └─────────────────┘                     │
└─────────────────────────────────────────────────────┘
```

## 🛠️ Resiliencia Operativa & Automatización de Respuesta (Sprint15)

**ed2kIA v2.1.0-sprint15** introduce infraestructura de resiliencia operativa para transparencia y respuesta automatizada:

### Chaos Engine — `v2.1-chaos-engine`

Motor asíncrono (tokio) para inyección controlada de fallos en local/testnet:

```bash
# Verificar compilación del Chaos Engine
cargo check --features "v2.1-chaos-engine"
```

- **Escenarios:** WASM node failure, network partition (GossipSub isolation), artificial latency, malicious vote injection, task queue saturation
- **Control estricto:** Solo activo con `--chaos-mode`, duración limitada, rollback automático, logs detallados
- **API async:** `activate()`, `rollback()`, `status()`, `shutdown()`
- **Seguridad:** Cero inyección sin flag explícito, cooldown entre escenarios, historial de eventos

### Operator CLI Wizard — `v2.1-operator-cli`

Binario standalone con interfaz TUI para configuración guiada de nodos:

```bash
# Construir y ejecutar el wizard
cargo build --bin ed2kia-cli --features "v2.1-operator-cli"
./target/debug/ed2kia-cli wizard
```

- **Selección de rol:** Relay, Orchestrator, WASM Node, Auditor
- **Generación de config:** Validación en tiempo real, formato TOML/JSON
- **Verificación de entorno:** Rust toolchain, espacio en disco
- **Health checks:** Verificación contra endpoint API
- **Export de logs:** Filtrado por rango temporal

### Auto-Remediation Pipeline — `v2.1-auto-remediation`

Script de respuesta automatizada a incidentes:

```bash
# Ejecutar monitoreo continuo
bash scripts/auto-remediate.sh --monitor

# Rollback manual a checkpoint
bash scripts/auto-remediate.sh --rollback

# Verificación de salud
bash scripts/auto-remediate.sh --health
```

- **Monitoreo activo:** `/api/health`, `/api/metrics`, consenso, slashing/partición
- **Acciones automáticas:** Restart graceful, rollback a checkpoint, generación de reportes
- **Notificaciones:** Webhooks opcionales (Slack, Discord, etc.)
- **Configuración:** Variables de entorno (`ED2KIA_API`, `ED2KIA_MAX_RESTARTS`, etc.)

```
┌─────────────────────────────────────────────────────┐
│  Operational Resilience Architecture                 │
│                                                       │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────┐ │
│  │ Chaos Engine │  │ CLI Wizard   │  │ Auto-     │ │
│  │ (Fault      │  │ (Setup &    │  │ Remediate)│ │
│  │  Injection) │  │  Validate)  │  │           │ │
│  └──────┬───────┘  └──────┬───────┘  └─────┬─────┘ │
│         │                 │                │       │
│         └─────────────────┼────────────────┘       │
│                    │                                │
│         ┌──────────▼──────────┐                    │
│         │  ed2kIA Node       │                    │
│         │  (Monitored &      │                    │
│         │   Resilient)       │                    │
│         └────────────────────┘                    │
└─────────────────────────────────────────────────────┘
```

## ️ Sistema Inmunológico — Defensa contra Data Poisoning

**ed2kIA v2.1.0-sprint7** introduce el Sistema Inmunológico, una capa defensiva de tres fases contra Data Poisoning en redes permissionless:

### Fase 1: Redundancia de Tareas (N-Node Dispatch)

El mismo task de auditoría se envía a **N nodos distintos** (configurable vía `replication_factor`), eliminando puntos únicos de fallo:

```rust
let manager = TaskManager::new(timeout, 3)
    .with_replication(3);  // Dispatch to 3 distinct peers
```

### Fase 2: Consenso Determinista

Motor de consenso O(N) que agrupa resultados por hash de índices y aplica regla de mayoría con tolerancia epsilon para f32:

```rust
let winner = validate_consensus(results, 1e-4);  // Some(result) o None
```

### Fase 3: Matriz de Reputación (Slashing & Banning)

Sistema de reputación concurrente con penalización asimétrica: **+1** por acierto, **-50** por fallo, auto-ban cuando score < 0:

```rust
let engine = ReputationEngine::new();
if engine.update_score("peer-42".to_string(), matched: false) {
    println!("Peer banned automatically!");
}
```

### 🚀 Quickstart: Dry-Run E2E

Valida la Secuencia de Ignición completa (Relay → Orchestrator → WASM Nodes → Consensus/Reputation → Atlas 3D) en tu máquina local:

```bash
# 1. Generar Dummy SAE (~129 KB)
python scripts/generate_dummy_sae.py

# 2. Ejecutar tests E2E (consenso + reputación)
cargo test --features "v2.1-reputation-system v2.1-task-manager" --test e2e_consensus_test

# 3. Bootstrap completo (opcional, requiere Rust + Python)
bash scripts/ignite-local-testnet.sh
```

**Resultados esperados:** 5/5 tests PASS, 0 warnings, consenso determinista validado con peers mock (2 honestos, 1 malicioso).

---

## 🛡️ Resiliencia & Alineación Humana — Defensa de Tercer Orden

**ed2kIA v2.1.0-sprint9** introduce la capa de **Resiliencia Absoluta**, un flujo de defensa de tercer orden que protege la integridad semántica de la red mediante resistencia computacional ética, federación criptográfica y corrección humana continua:

```
Micro-PoW handshake → GossipSub federado → Feedback RLHF → Corrección comunitaria de sesgos
```

### Fase 4: Resistencia Sybil Ética (Micro-PoW)

Challenge SHA-256 con dificultad configurable (1–4 leading zeros, ~2s/nodo) que previene inundación de identidades sin barreras financieras:

```rust
let engine = SybilEngine::with_difficulty(2).unwrap();
let challenge = engine.generate_challenge();
let solution = solve_challenge(challenge.nonce, challenge.difficulty);
assert!(engine.verify(&challenge, &solution, "node-42").is_ok());
```

**Cero staking, cero KYC** — Resistencia puramente computacional, accesible desde cualquier dispositivo.

### Fase 5: Federación GossipSub (Orchestrator Mesh)

Coordinación multi-nodo vía `libp2p 0.53` con `MessageAuthenticity::Signed` para proveniencia criptográfica en topics ATLAS_SYNC y REPUTATION_SYNC:

```rust
let swarm = build_federation_swarm(peer_id, listen_addr).await?;
// ATLAS_SYNC: Propagación de deltas semánticos
// REPUTATION_SYNC: Sincronización federada de slashing
```

### Fase 6: Puente RLHF (Alineación Humana Continua)

Corrección comunitaria de sesgos semánticos vía API REST + UI interactiva. Cualquier usuario puede corregir activaciones erróneas en el Atlas Semántico:

```bash
# Submit human correction
curl -X POST http://localhost:3030/api/feedback \
  -H "Content-Type: application/json" \
  -d '{"node_id":"user-1","token":"justicia","feature":"feat-42","decision":"correct","note":"Se refiere a equity, no legal justice"}'

# Export feedback for training pipeline
curl http://localhost:3030/api/feedback/export
```

**Principios éticos:** Cero PII, almacenamiento local, export opt-in, gobernanza distribuida.

---

## 🚀 Launch & Demo — Prueba en <30s

**ed2kIA v2.1.0-sprint10** incluye infraestructura de lanzamiento de cero fricción para que cualquier hacker pueda probar un browser node y ver el "Aha! Moment" en menos de 30 segundos:

### Demo en Vivo (GitHub Pages)

Accede al portal de demostración directamente desde tu navegador — sin instalación, sin configuración:

```
👉 https://ed2kia.github.io/ed2kIA   (se activa con primer push a main)
```

### Script de Tráfico de Demo (15s)

Genera tráfico simulado para grabar videos de demostración con un solo comando:

```bash
# Tráfico demo de 15s (nodos → auditorías → RLHF feedback)
bash scripts/simulate_traffic.sh

# Personalizar puerto y duración
ED2KIA_PORT=8080 DEMO_DURATION=30 bash scripts/simulate_traffic.sh
```

**Fases del demo:**
1. **0-3s:** Conexión de nodos WASM simulados
2. **3-10s:** Inyección de tareas de auditoría
3. **10-13s:** Feedback RLHF (correcciones humanas)
4. **Final:** Estadísticas finales en Atlas 3D

### Kit de Lanzamiento Comunitario

Copywriting listo para publicar en comunidades técnicas:

| Plataforma | Archivo | Enfoque |
|------------|---------|---------|
| Hacker News | [`docs/launch-kit/show-hn.md`](docs/launch-kit/show-hn.md) | Técnico, humilde, disruptivo |
| Reddit | [`docs/launch-kit/reddit-ml-rust.md`](docs/launch-kit/reddit-ml-rust.md) | r/machinelearning, r/rust, r/open_source |
| Twitter/X | [`docs/launch-kit/x-thread.md`](docs/launch-kit/x-thread.md) | Hilo de 5 tweets (problema → solución → arquitectura → ética → CTA) |

---

## 📊 Observabilidad & Gobernanza — Preparación para Mainnet

**ed2kIA v2.1.0-sprint11** introduce la infraestructura de **Operational Readiness**, el conjunto de herramientas de observabilidad, gobernanza y validación automática requeridas antes del despliegue en mainnet:

```
Prometheus Metrics → Grafana Dashboard → Pre-Launch Validation → Mainnet Ready
```

### Métricas Prometheus (5 categorías, 12+ métricas)

Registro ligero de métricas de salud de red con namespace `ed2kia_`, cero telemetría externa, cero lógica financiera:

| Categoría | Métricas |
|-----------|----------|
| **Consensus** | `ed2kia_consensus_votes_total`, `ed2kia_consensus_rounds_total`, `ed2kia_consensus_round_latency_seconds` |
| **Reputation** | `ed2kia_reputation_slashing_total`, `ed2kia_reputation_banned_peers`, `ed2kia_reputation_score_sum` |
| **Network** | `ed2kia_network_peers_active`, `ed2kia_network_bytes_received_total`, `ed2kia_network_bytes_sent_total`, `ed2kia_network_gossipsub_messages_total` |
| **RLHF** | `ed2kia_rlhf_feedback_total`, `ed2kia_rlhf_corrections_accepted`, `ed2kia_rlhf_corrections_rejected` |
| **WASM Worker** | `ed2kia_wasm_worker_cpu_ms`, `ed2kia_wasm_worker_sae_inference_latency_ms`, `ed2kia_wasm_worker_active_workers` |

### Dashboard Grafana

Panel de visualización en tiempo real con 5 filas de paneles (Network, Consensus, Reputation, RLHF, WASM) incluyendo histogramas p50/p95/p99, gauges y timeseries:

```
📊 Dashboard: prometheus/grafana-dashboard.json (UID: ed2kia-dashboard-v21)
```

### Validación Pre-Launch Automatizada

Script de validación de 5 fases con reporte de readiness automático:

```bash
# Ejecutar checklist pre-launch
bash scripts/pre-launch-check.sh

# Reporte: docs/launch-readiness-report.md
# Resultado: GREEN READY FOR MAINNET o RED BLOCKED
```

**Fases de validación:**
1. `cargo check --all-targets` — Compilación limpia
2. `cargo test --lib` — Tests unitarios PASS
3. Verificación de archivos críticos (Cargo.toml, LICENSE, README.md, etc.)
4. Validación JSON (grafana-dashboard.json)
5. Integridad documental (CHANGELOG.md, README.md)

### CODEOWNERS & Gobernanza

Propiedad modular para revisiones de PR obligatorias — cada módulo tiene owner asignado en [`CODEOWNERS`](CODEOWNERS). Gobernanza operativa documentada en [`GOVERNANCE.md`](GOVERNANCE.md) §§12-13.

---

## 🌐 Nodo en el Navegador — Participación sin Barreras

**ed2kIA v2.1 introduce el primer nodo P2P que funciona directamente en tu navegador web**, sin instalaciones, sin software adicional.

### ¿Qué significa para la comunidad?

- **Cero fricción de entrada:** Cualquier persona con un navegador moderno puede unirse a la red como verificador, sin instalar Rust, Docker o herramientas de desarrollo.
- **Participación global instantánea:** Estudiantes, investigadores y ciudadanos de cualquier país pueden contribuir con capacidad de cómputo desde su dispositivo actual.
- **Transparencia verificable:** El nodo WASM ejecuta Sparse Autoencoders (SAE) directamente en el cliente, permitiendo auditoría visual del proceso de interpretabilidad.
- **Arquitectura P2P real:** Usa WebRTC y WebSockets a través de `libp2p` para descubrimiento de pares KAD y comunicación descentralizada.

### MVP Core Loop — Ciclo Básico Validado

El ciclo operativo mínimo (Discovery → Distribution → Inference → Collection) está aislado y validado con **27 tests unitarios**, permitiendo iteración rápida sin depender de módulos avanzados (ZKP, Gobernanza, Reputación) que permanecen detrás de feature gates separados.

```
Navegador ──→ [WASM Node] ──→ [KAD Discovery] ──→ [Tensor Distribution]
                                                    ↓
                                            [SAE Inference] ──→ [Result Collection]
                                                    ↓
                                            Red P2P Global
```

> **Ética primero:** Toda participación es voluntaria, auditable y compatible con la [Constitución del Proyecto](docs/governance/project-constitution.md).

### 3 Pilares Críticos de Viabilidad Web (v2.1-sprint4)

La viabilidad del nodo WASM en navegador se sustenta en **3 pilares técnicos** implementados y validados:

| Pilar | Módulo | Función | Tests |
|-------|--------|---------|-------|
| **Web Workers** | [`browser_node/worker`](src/browser_node/worker.rs) | Async inference offloading sin bloquear UI (postMessage/onmessage) | 2 |
| **WebRTC + Relay** | [`browser_node/webrtc_transport`](src/browser_node/webrtc_transport.rs) | libp2p WASM transport con Circuit Relay v2 | 13 |
| **Telemetría Reactiva** | [`mvp_core/inference_bridge`](src/mvp_core/inference_bridge.rs) | Rust → JS CustomEvent → DOM (4 eventos: task, inference, peer, error) | integrado |

```
Navegador ──→ [Web Worker] ──→ [postMessage] ──→ [Audit Task Dispatch]
                                                    ↓
Navegador ──→ [WebRTC Relay] ──→ [Circuit Relay v2] ──→ [Peer Discovery]
                                                    ↓
Navegador ──→ [Telemetry Bridge] ──→ [CustomEvent] ──→ [DOM Reactive Update]
```

> **Conecta tu navegador, únete a la red P2P real y visualiza tu impacto en tiempo real. Fricción cero, transparencia total.** Activa los feature gates `v2.1-wasm-workers`, `v2.1-webrtc-relay` y `v2.1-wasm-telemetry` para probar localmente. Contribuye vía [CONTRIBUTING.md](CONTRIBUTING.md).

### Qwen Scope SAE Integration (v2.1-sprint3)

**Audita modelos de clase mundial de forma descentralizada.** Tu navegador procesa fragmentos seguros de Sparse Autoencoders y devuelve transparencia verificable.

La integración Qwen Scope SAE proporciona la arquitectura completa para auditoría descentralizada de LLMs:

| Componente | Módulo | Función | Tests |
|------------|--------|---------|-------|
| **Top-k SAE** | [`sae/qwen_scope_sae`](src/sae/qwen_scope_sae.rs) | Arquitectura 4-tensor con forward pass exacto `f(x) = TopK(W_enc @ (x - b_dec) + b_enc)` | 10 |
| **Safetensors Loader** | [`sae/qwen_scope_loader`](src/sae/qwen_scope_loader.rs) | Carga de pesos Qwen Scope + micro-sharding WASM ≤50MB | 12 |
| **Audit Payloads** | [`protocol/audit_payloads`](src/protocol/audit_payloads.rs) | Serialización bincode para flujos P2P de auditoría | 14 |
| **Inference Bridge** | [`mvp_core/inference_bridge`](src/mvp_core/inference_bridge.rs) | `execute_audit_task()` — ciclo completo P2P | integrado |

```
Audit Task ──→ [Deserialize bincode] ──→ [QwenScopeSAE::forward()] ──→ [Sparse Features]
                                                                         ↓
                                                            [Serialize Result] ──→ P2P Network
```

> **Ética primero:** Toda auditoría es voluntaria, transparente y compatible con la [Constitución del Proyecto](docs/governance/project-constitution.md). Cero lógica financiera, máxima interpretabilidad.

## 📦 Estructura del Proyecto

```
ed2kIA/
├── Cargo.toml              # Dependencias versionadas + feature flags
├── README.md               # Este archivo
├── LICENSE                 # Apache 2.0 + Ethical Use Clause
├── .github/                # Fase 4: CI/CD
│   └── workflows/
│       └── ci.yml          # Test, cross-compile, Docker, release, audit
├── deploy/                 # Fase 3: Scripts de despliegue
│   ├── Dockerfile          # Multi-stage build (rust → debian)
│   ├── docker-compose.yml  # Red de 3 nodos de prueba
│   └── systemd/            # Service templates systemd
│       ├── ed2kia.service  # Unit file systemd
│       ├── ed2kia.env      # Configuración de entorno
│       └── install.sh      # Script de instalación
├── docs/                   # Fase 5: Documentación
│   ├── GOVERNANCE.md       # Sistema de gobernanza
│   ├── CONTRIBUTING.md     # Guía de contribución
│   └── NETWORK_BOOTSTRAP.md # Procedimiento de lanzamiento
├── release/                # Fase 5: Paquetes de release
│   ├── packager.sh         # Multi-platform build script
│   └── changelog.md        # Semantic versioning changelog
├── web/                    # Fase 4: Dashboard Web UI
│   ├── index.html          # Alpine.js dashboard
│   └── assets/
│       ├── style.css       # Estilos dashboard
│       └── app.js          # Lógica Alpine.js
├── src/
│   ├── main.rs             # Orquestador principal + CLI Fase 5
│   ├── p2p/
│   │   ├── swarm.rs        # libp2p + GossipSub (Fase 2)
│   │   └── protocol.rs     # Schema + FeatureBatch/ConsensusVote (Fase 2)
│   ├── sae/
│   │   ├── loader.rs       # Candle-based SAE loader (.safetensors)
│   │   ├── router.rs       # LayerRouter (dynamic sharding + leases)
│   │   ├── qwen_scope_sae.rs    # Qwen Scope Top-k SAE (4-tensor arch)
│   │   ├── qwen_scope_loader.rs # Safetensors ingestion + WASM sharding
│   │   └── wasm_sharding.rs     # Tensor chunking ≤50MB for wasm32
│   ├── bridge/
│   │   ├── tensor_flow.rs  # Node A → Node B tensor pipeline
│   │   └── consciousness.rs # Agregación + conflictos + steering (Fase 2)
│   ├── protocol/           # Protocolos P2P + Auditoría
│   │   ├── audit_payloads.rs    # AuditTask/AuditResult con bincode
│   │   └── async_steering.rs    # Steering signals para pipelines tensor
│   ├── interpret/          # Fase 2: Motor de Interpretación
│   │   ├── feature_analyzer.rs  # Análisis SAE + detección anomalías
│   │   └── semantic_map.rs      # Mapeo feature→concepto (Qwen-Scope)
│   ├── consensus/          # Fase 2: Consenso & Merkle
│   │   ├── validator.rs    # Validación asíncrona + Merkle + ZKP (Fase 3)
│   │   └── merkle.rs       # Generación/verificación raíces Merkle
│   ├── security/           # Fase 3: Sandbox WASM
│   │   ├── wasm_sandbox.rs # Ejecución aislada SAE forward (wasmtime)
│   │   └── memory_guard.rs # Límites de memoria + detección escapes
│   ├── zkp/                # Fase 3: Zero-Knowledge Proofs
│   │   ├── circuit.rs      # Circuitos aritméticos + Pedersen commitments
│   │   └── verifier.rs     # Verificación ZKP/Merkle/VRF + reputación
│   ├── human/              # Fase 3: Human-in-the-loop
│   │   ├── feedback_cli.rs # CLI interactivo para labeling (TTTY/JSON)
│   │   └── concept_updater.rs # Updates seguros del semantic_map
│   ├── scaling/            # Fase 4: Escalado P2P
│   │   ├── peer_manager.rs # Gestión peers + scoring dinámico + límites
│   │   └── bootstrap.rs    # Descubrimiento DNS + AutoNAT + protocolo
│   ├── rlhf/               # Fase 4: RLHF Loop
│   │   ├── feedback_store.rs # Almacenamiento redb + export JSONL
│   │   └── trainer_loop.rs # Batches + drift detection + export training
│   ├── web/                # Fase 4: Servidor Web
│   │   ├── server.rs       # Axum HTTP + static files
│   │   └── routes.rs       # API endpoints (/api/status, /api/network, ...)
│   └── monitoring/         # Fase 4: Observabilidad
│       ├── metrics.rs      # Prometheus counters/gauges/histograms
│       └── health.rs       # Health checks + uptime + resource monitoring
│   ├── governance/         # Fase 5: Gobernanza
│       ├── proposal.rs     # Propuestas firmadas Ed25519
│       └── voting.rs       # Votación time-locked + quorum + auto-exec
│   ├── reputation/         # Fase 5: Reputación
│       ├── ledger.rs       # Registro inmutable contribuciones (redb)
│       └── scoring.rs      # Créditos + decay + anti-Sybil
│   ├── ecosystem/          # Fase 5: Ecosistema
│       ├── hf_sync.rs      # Hugging Face/ModelScope sync
│       └── model_registry.rs # Registry local + checksums + rollback
│   └── bootstrap/          # Fase 5: Bootstrap
│       ├── seed_registry.rs # Descubrimiento seeds + health validation
│       └── network_init.rs  # Genesis mode + migration + fallback
└──

## 💻 Desarrollo Local

### Setup Rápido

```bash
# Instalar herramienta de desarrollo
cargo install just

# Setup automático del entorno
bash devtools/setup.sh

# Setup completo (incluye tooling opcional)
bash devtools/setup.sh --full
```

### Comandos de Desarrollo

| Comando | Descripción |
|---------|-------------|
| `just build` | Build debug (CPU) |
| `just build-release` | Build release (CPU) |
| `just build-cuda` | Build con CUDA GPU |
| `just build-metal` | Build con Metal (Apple Silicon) |
| `just build-wasm` | Build para WASM |
| `just check` | Syntax check |
| `just clippy` | Lint con Clippy |
| `just test` | Ejecutar tests |
| `just test-sprint2` | Tests v1.8 Sprint 2 |
| `just dev` | Ejecutar nodo local |
| `just docker-compose` | Entorno completo Docker |

### Entorno Docker Completo

```bash
# Iniciar 3 nodos P2P + Prometheus + Grafana
just docker-compose

# Acceder a dashboards:
#   - Node 1 API: http://localhost:9000
#   - Node 2 API: http://localhost:9002
#   - Node 3 API: http://localhost:9004
#   - Prometheus: http://localhost:9090
#   - Grafana: http://localhost:3000 (admin/admin)

# Detener entorno
just docker-compose-down
```

### Estructura de DevTools

```
devtools/
├── setup.sh            # Setup automático del entorno
└── docker-compose.yml  # Entorno local completo (3 nodos + monitoring)
```

## 🛠️ Requisitos

- **Rust 1.75+** (edición 2021)
- **Cargo** (incluido con Rust)
- **Opcional:** CUDA toolkit (para GPU acceleration)
- **Opcional:** Metal (para Apple Silicon)

### Instalar Rust

```bash
# Windows
winget install Rustlang.Rust

# Linux/macOS
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.sh | sh
```

## 🚀 Compilación

### Build básico (CPU)

```bash
cargo build
```

### Build con GPU CUDA

```bash
cargo build --features cuda
```

### Build con Metal (Apple Silicon)

```bash
cargo build --features metal
```

### Build de producción

```bash
cargo build --release
```

## ✅ Verificación

### Syntax check

```bash
cargo check
```

### Tests unitarios

```bash
cargo test
```

### Lint

```bash
cargo clippy
```

## 💻 Uso

### CLI Commands (Fase 4)

```bash
# Ver ayuda
cargo run -- --help

# ─── Comandos P2P (Fase 1-2) ─────────────────────────────
# Unirse a la red
cargo run -- join

# Unirse con bootstrap peers
cargo run -- --bootstrap "/ip4/127.0.0.1/tcp/4001/p2p/PEER_ID" join

# Unirse con SAE cargado
cargo run -- --sae-path ./models/qwen2-7b-sae.safetensors join

# Ver estado del nodo
cargo run -- status

# Salir de la red
cargo run -- exit

# ─── Comandos Fase 3: Seguridad ──────────────────────────
# Ejecutar módulo WASM en sandbox aislado
cargo run -- sandbox ./models/sae_module.wasm ./input.bin

# Verificar batch con ZKP
cargo run -- verify <batch_id> --features "f1,f2,f3" --verifier "node_1"

# ─── Comandos Fase 3: Human-in-the-loop ──────────────────
# Modo interactivo (TTY)
cargo run -- feedback interactive

# Modo batch (JSON)
cargo run -- feedback batch --file requests.json

# Estadísticas de labeling
cargo run -- feedback stats

# ─── Comandos Fase 3: Deploy ─────────────────────────────
# Build Docker
cargo run -- deploy docker

# Info systemd
cargo run -- deploy systemd

# Cross-compilation info
cargo run -- deploy cross

# ─── Comandos Fase 3: Red ────────────────────────────────
# Info de red
cargo run -- network

# Reputación criptográfica
cargo run -- network crypto-reputation <node_id>

# ─── Comandos Fase 4: Web UI ─────────────────────────────
# Iniciar servidor web (dashboard)
cargo run -- web --port 3000

# ─── Comandos Fase 4: Escalado ───────────────────────────
# Estado de escalado (peers, límites, scoring)
cargo run -- scale --status

# ─── Comandos Fase 4: RLHF ───────────────────────────────
# Exportar feedback a JSONL (formato training)
cargo run -- rlhf --export --path ./training_data.jsonl

# ─── Comandos Fase 4: Health ─────────────────────────────
# Ejecutar health checks
cargo run -- health --check

# ─── Comandos Fase 5: Gobernanza ─────────────────────────
# Crear propuesta firmada
cargo run -- govern propose --type "protocol" --title "Title" --payload "Payload"

# Listar propuestas
cargo run -- govern list

# Votar propuesta
cargo run -- govern vote <proposal_id> --direction "approve"

# ─── Comandos Fase 5: Reputación ─────────────────────────
# Ver reputación del nodo
cargo run -- reputation status

# Leaderboard de reputación
cargo run -- reputation leaderboard

# Aplicar decay
cargo run -- reputation decay

# ─── Comandos Fase 5: Sync Ecosistema ────────────────────
# Descargar modelo desde Hugging Face
cargo run -- sync download --repo "org/model" --file "model.safetensors"

# Listar modelos registrados
cargo run -- sync list

# ─── Comandos Fase 5: Bootstrap ──────────────────────────
# Inicializar red genesis
cargo run -- bootstrap genesis --data-dir ./data --p2p-port 9000 --http-port 3000

# Unirse a red existente
cargo run -- bootstrap join --data-dir ./data

# Ver estado de bootstrap
cargo run -- bootstrap status

# ─── Comandos Fase 5: Release ────────────────────────────
# Crear paquete de release
cargo run -- release package --version "0.5.0"
```

### Parámetros

| Parámetro | Descripción | Default |
|-----------|-------------|---------|
| `-n, --node-id <ID>` | ID del nodo | Auto-generado |
| `-p, --port <PORT>` | Puerto P2P | 0 (auto) |
| `--bootstrap <ADDRS>` | Bootstrap peers (coma separado) | None |
| `--sae-path <PATH>` | Path al archivo SAE (.safetensors) | None |
| `--function <NAME>` | Función WASM a ejecutar (sandbox) | "sae_forward" |
| `--features <LIST>` | Features sparse para verificación | Required |
| `--verifier <ID>` | ID del nodo verificador | Auto |
| `--file <PATH>` | Archivo JSON para feedback batch | Required |

## 🛡️ Fase 3 - Seguridad y Verificación

### WASM Sandbox (wasmtime)

Ejecución aislada de forward passes SAE en entornos WASM con:
- **Límite de memoria:** 256MB configurable
- **I/O de host deshabilitado:** Sin acceso al filesystem/red
- **Optimización Cranelift Speed:** Máximo rendimiento JIT
- **Detección de escapes:** Monitoreo en tiempo real

```bash
# Ejecutar módulo WASM en sandbox
cargo run -- sandbox ./models/sae_module.wasm ./input.bin --function "sae_forward"
```

### Zero-Knowledge Proofs (arkworks)

Verificación criptográfica de batches con:
- **Pedersen commitments** en curva BN254
- **Fiat-Shamir heuristic** para challenges
- **Merkle inclusion proofs** como fallback
- **Reputación criptográfica** por nodo

```bash
# Verificar batch con ZKP
cargo run -- verify <batch_id> --features "f1,f2,f3" --verifier "node_1"
```

### Human-in-the-Loop

Sistema de labeling interactivo con:
- **Modo interactivo (TTY):** Aprobación/rechazo/corrección en tiempo real
- **Modo batch (JSON):** Procesamiento masivo desde archivos
- **Quorum de votación:** Actualización segura del semantic_map
- **Rollback:** Reversión de cambios problemáticos

```bash
# Modo interactivo
cargo run -- feedback interactive

# Modo batch
cargo run -- feedback batch --file requests.json

# Estadísticas
cargo run -- feedback stats
```

### Deployment

#### Docker

```bash
# Build imagen
docker build -t ed2kia:latest -f deploy/Dockerfile .

# Ejecutar contenedor
docker run -p 9000:9000 ed2kia:latest run --config config.toml

# Red de 3 nodos con docker-compose
cd deploy && docker-compose up -d
```

#### Systemd

```bash
# Instalar servicio
sudo bash deploy/systemd/install.sh

# Configurar
sudo nano /etc/ed2kia/ed2kia.env

# Iniciar
sudo systemctl start ed2kia
sudo systemctl enable ed2kia

# Logs
sudo journalctl -u ed2kia -f
```

#### Cross-compilation

```bash
# Compilar para Linux desde Windows
cargo build --target x86_64-unknown-linux-gnu --release

# Compilar para ARM64
cargo build --target aarch64-unknown-linux-gnu --release
```

## 🔄 Flujo de Tensores: Nodo A → Nodo B

### Paso a Paso

```
┌─────────────────────────────────────────────────────────────────────┐
│                     FLUJO DE TENSORES ed2kIA                        │
└─────────────────────────────────────────────────────────────────────┘

┌──────────────────┐         ┌──────────────────┐         ┌──────────────────┐
│     NODO A       │         │    RED P2P       │         │     NODO B       │
│  (Extractor)     │         │   (libp2p)       │         │  (Inferencer)    │
└──────────────────┘         └──────────────────┘         └──────────────────┘

 1. Extraer Hidden States
    └── LLM genera hidden states [batch, seq_len, hidden_dim]
    └── Tensor se planariza a Vec<f32>
        │
        │ 2. Serializar
        │    └── TensorPayload::serialize()
        │    └── Header: shape + stride + dtype + device
        │    └── Data: bytes f32 compactos
        │
        ▼
 3. Crear TensorRequest
    └── request_id: UUID v4
    └── layer_id: ID de capa SAE destino
    └── tensor_data: Vec<f32>
    └── tensor_shape: Vec<usize>
        │
        │ 4. Enviar via libp2p
        │    └── swarm.send_tensor_request(&peer_id, request)
        │    └── CBOR serialization (Phase 1)
        │    └── Protobuf/FlatBuffers (Phase 2)
        │
        │  ──────────────────────────────────────────────── │
        │                                                   │
        ▼                                                   ▼
 5. Recepción en Nodo B                           6. Deserializar
    └── swarm.handle_message()                      └── TensorPayload::deserialize()
    └── Ed2kMessage::TensorRequest                  └── Reconstruir tensor
                                                        │
                                                        ▼
                                                    7. Forward Pass SAE
                                                       └── SAEModel::forward(input)
                                                       └── encoded = W_enc @ x + b_enc
                                                       └── sparse = TopK(encoded)
                                                       └── confidence = calculate_confidence()
                                                        │
                                                        ▼
                                                    8. Crear TensorResponse
                                                       └── sparse_features: Vec<SparseFeature>
                                                       └── confidence_score: f64
                                                       └── error: Option<String>
                                                        │
        │ 9. Enviar Respuesta                              │
        │    └── swarm.send_response(&peer_a, response)    │
        │                                                   │
        ────────────────────────────────────────────────────
        │
        ▼
10. Recepción en Nodo A
    └── pipeline.process_response(response)
    └── Actualizar PipelineState → Received
        │
        ▼
11. Agregación
    └── ConsciousnessBridge::add_features()
    └── Agregar features de múltiples nodos
        │
        ▼
12. Inyección como Contexto
    └── ConsciousnessBridge::inject_context()
    └── Steering signals para LLM downstream
    └── TODO: Phase 3 - Implementación completa
```

### Placeholders Documentados

| Módulo | Fase | Descripción |
|--------|------|-------------|
| `SteeringSignal` | 2 | Señales de steering síncronas ligeras |
| `PubSub` | 2 | FloodSub para señales asincrónicas |

## 📊 Roadmap

### Fase 1 - Core P2P + SAE Loader + Tensor Routing ✅
- ✅ Estructura de proyecto Rust
- ✅ CLI con Clap (join, status, exit)
- ✅ Swarm P2P con libp2p (KAD + mDNS)
- ✅ Protocolo de mensajes (TensorRequest/Response, Leases, Steering)
- ✅ SAE Loader con Candle (.safetensors)
- ✅ LayerRouter con sharding dinámico y leases
- ✅ Tensor Flow Pipeline (Node A → Node B)
- ✅ Placeholders para ZKP, WASM, ConsensusValidator

### Fase 2 - Interpretación, Feedback & Consenso ✅
- ✅ Feature Analyzer (análisis SAE + detección anomalías)
- ✅ Semantic Map (mapeo feature→concepto Qwen-Scope)
- ✅ Merkle tree (generación/verificación de raíces)
- ✅ Consensus Validator (validación asíncrona + umbrales)
- ✅ Consciousness Bridge (agregación + conflictos + steering)
- ✅ GossipSub en swarm P2P
- ✅ Protocolo extendido (FeatureBatch, ConsensusVote)
- ✅ CLI extendido (analyze, interpret, consensus)

### Fase 3 - Seguridad, ZKP, Human-in-the-Loop & Deploy ✅
- ✅ WASM Sandbox (wasmtime) con límites de memoria y detección de escapes
- ✅ Memory Guard (tracking de allocations + análisis de patrones)
- ✅ ZKP Circuit (Pedersen commitments + Fiat-Shamir en BN254)
- ✅ ZKP Verifier (verificación ZKP/Merkle/VRF + reputación cripto)
- ✅ Human Feedback CLI (modo interactivo TTY + batch JSON)
- ✅ Concept Updater (updates seguros del semantic_map con quorum)
- ✅ Qwen-Scope integration (load_from_qwen_scope + learn_concept)
- ✅ ZKP + reputación cripto en ConsensusValidator
- ✅ CLI Fase 3 (sandbox, verify, feedback, deploy, network)
- ✅ Docker multi-stage build
- ✅ Docker Compose (red de 3 nodos de prueba)
- ✅ Systemd service templates + install script

### Fase 4 - Escalado, RLHF, Web UI & Producción ✅
- ✅ Peer Manager (scoring dinámico + límites de conexión + adaptive gossipsub)
- ✅ Bootstrap Manager (descubrimiento DNS + AutoNAT + protocolo ed2k/0.4.0)
- ✅ Feedback Store (redb embedded + export JSONL para llama.cpp/vLLM)
- ✅ Trainer Loop (batches + drift detection semántico + export training)
- ✅ Web Server (axum + tower-http + static files)
- ✅ API Routes (/api/status, /api/network, /api/feedback, /api/metrics, /api/health)
- ✅ Dashboard Web UI (Alpine.js + CSS responsive + feedback form)
- ✅ Prometheus Metrics (counters/gauges/histograms + lazy_static)
- ✅ Health Checks (pluggable checks + uptime + resource monitoring)
- ✅ CI/CD Pipeline (test, cross-compile, Docker, release, security audit)
- ✅ CLI Fase 4 (web, scale, rlhf, health)

### Fase 5 - Bootstrap, Gobernanza, Reputación & Ecosistema ✅
- ✅ Proposal System (Ed25519 signed proposals + GossipSub propagation)
- ✅ Voting System (72h time-lock + ≥30% quorum + ≥51% reputation-weighted approval)
- ✅ Reputation Ledger (redb immutable contribution registry + chain integrity)
- ✅ Reputation Scoring (credits by type + ZKP 1.5x multiplier + 50%/30d decay + anti-Sybil)
- ✅ Hugging Face/ModelScope Sync (rate limiting + cache + checksum verification)
- ✅ Model Registry (semantic versioning + SHA-256 checksums + rollback)
- ✅ Seed Registry (hardcoded + DNS discovery + health validation + weighted selection)
- ✅ Network Genesis (genesis mode + lease creation + SAE distribution + offline fallback)
- ✅ GOVERNANCE.md (principles, structure, proposal types, voting mechanics)
- ✅ CONTRIBUTING.md (prerequisites, setup, code structure, workflow, debugging)
- ✅ NETWORK_BOOTSTRAP.md (seed preparation, genesis, verification, troubleshooting)
- ✅ Release Packager (multi-platform build + checksums + Ed25519 signing)
- ✅ Changelog (semantic versioning v0.1.0 → v0.5.0)
- ✅ CLI Fase 5 (govern, reputation, sync, bootstrap, release)

### ✅ Fase 6 - Integración y Producción (Completada en v1.7/v1.8)
- ✅ Integración real con LLMs: Async Steering v1 + API Explorer v1 + Quantization v3 ([`src/protocol/async_steering.rs`](src/protocol/async_steering.rs), [`src/api/explorer_v1.rs`](src/api/explorer_v1.rs), [`src/bridge/quantization.rs`](src/bridge/quantization.rs))
- ✅ Tests de integración P2P: 6 integration test files + stress tests ([`tests/integration/`](tests/integration/))
- ✅ Benchmark de inferencia SAE: Criterion benchmarks + CI comparison ([`benchmarks/`](benchmarks/))
- ✅ Documentación API: OpenAPI spec + API Explorer v1 + Auth v2 ([`src/api/`](src/api/))
- 📄 Auditoría completa: [`docs/roadmap/phase6-audit-mapping.md`](docs/roadmap/phase6-audit-mapping.md)
- 📄 Roadmap v1.9: [`docs/roadmap/v1.9-roadmap-draft.md`](docs/roadmap/v1.9-roadmap-draft.md)

### ✅ Fase 7 - v1.9.0-stable: Production Ready (Completada)
- ✅ Security Audit & OSSF Compliance (8.5/10)
- ✅ Release Engineering v1.9.0-stable & Migration Guide
- ✅ Community Scaling & Final Grant Package
- ✅ Operational Prompt v9.0 & v2.0 Architectural Vision
- ✅ Final Sign-off & Operational Handover

### ✅ Fase 8 — v2.0 Sprint 1: GUI Tauri, ZKP v2 & K8s Base (Completada)
- ✅ Tauri GUI Scaffold con Neural Steering UI (31 tests)
- ✅ ZKP Multi-Curve Setup: BN254, BLS12-381, Pasta (20 tests)
- ✅ Proof Aggregation con batch verification (33 tests)
- ✅ Circuit Optimization con Pedersen precomputation (25 tests)

### ✅ Fase 9 — v2.0 Sprint 2: Core Integration & Optimization (Completada)
- ✅ Commitment Pool + Mobile Hardening (30 tests each)
- ✅ Federation ZKP Bridge + Scaling modules
- ✅ Dashboard v7 + WebSocket Federation Stream
- ✅ Security Audit v2.0 + Threat Model Update

### ✅ Fase 90 — Release Engineering v2.0.0-stable (Completada)
- ✅ **3025 tests passing** (99.7% pass rate)
- ✅ OSSF Score: **8.5/10** (PASSING)
- ✅ 80+ módulos implementados
- ✅ Constitución del Proyecto & Carta de Gobernanza
- ✅ Ciclo Operativo Autónomo & Monitoreo de Salud

### ✅ Fase 91-99 — Stewardship & RFC Process (Completada)
- ✅ Ciclo Operativo Autónomo (FASE 91)
- ✅ Constitución del Proyecto (FASE 92)
- ✅ Tracking de Hitos Comunitarios (FASE 93)
- ✅ Paquete Final de Handover (FASE 94)
- ✅ Estado del Proyecto & Anuncio Público (FASE 95)
- ✅ Ciclo de Revisión Trimestral (FASE 96)
- ✅ Proceso RFC Comunitario & RFC-001 (FASE 97)
- ✅ Roadmap de Evolución v2.1 → v3.0 (FASE 98)
- ✅ Handover de Estipulación & Prompt v13.0 (FASE 99)

### 🔧 v2.1 Sprint 1 — En Desarrollo (Post-RFC)
- 📋 Scaffold Estructural v2.1 (feature-gated, cero lógica)
- 📋 Plan de Remediación de Dependencias
- 📋 Observability Scaffold (Prometheus/Grafana metrics)
- 📋 Voting Dashboard Template + Tally Script
- 📋 Security Monitoring Pipeline (weekly cron)
- 📋 Testnet Infrastructure (Docker Compose scaffold)
- 📄 RFC-001: Feedback Aggregation — Discusión
- 📄 RFC-002: Observability Infrastructure — Draft
- 📄 RFC-003: Testnet/Infra v2.1 — Draft

## 🔒 Seguridad y Ética

- **Código auditable:** Todo el código es open source y revisable
- **Sin backdoors:** Verificado por la comunidad
- **Uso ético:** Diseñado exclusivamente para bienestar humano y IA responsable
- **Infraestructura voluntaria:** Participación opt-in global
- **OSSF Score:** 8.5/10 (PASSING) — [`security/audit_v2.0_sprint2.md`](security/audit_v2.0_sprint2.md)
- **Threat Model v2.0:** 17 amenazas identificadas y mitigadas — [`security/threat_model_v2.0.md`](security/threat_model_v2.0.md)
- **Monitoreo Semanal:** Security audit automatizado (Lunes 03:00 UTC) — [`.github/workflows/security-monitor.yml`](.github/workflows/security-monitor.yml)

## 📄 Licencia

Apache 2.0 + Cláusula de Uso Ético

```
Copyright 2024 ed2kIA Contributors

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

ADDITIONAL ETHICAL USE CLAUSE:
This software is designed exclusively for the benefit of humanity and
responsible AI development. It must be used transparently, remain
auditable, be free of backdoors, and be compatible with voluntary
global infrastructure for the progress of human knowledge and wellbeing.
```

## 🤝 Contribuir

1. Fork el repositorio
2. Crear rama de feature (`git checkout -b feature/amazing-feature`)
3. Commit cambios (`git commit -m 'Add amazing feature'`)
4. Push a la rama (`git push origin feature/amazing-feature`)
5. Abrir Pull Request

### Gobernanza Comunitaria

Este proyecto opera bajo la **[Constitución del Proyecto](docs/governance/project-constitution.md)** con los siguientes principios:

- **Propiedad Comunitaria:** Ningún individuo u organización tiene control exclusivo
- **Proceso RFC:** Cambios arquitectónicos requieren RFC + votación comunitaria
- **Votación Ponderada:** Tiers (Novice 0.5 → Guardian 3.0), quórum 30%, mayoría 60%
- **Ética Primero:** Cláusula de Uso Ético + Cero Lógica Financiera

Ver [`GOVERNANCE.md`](GOVERNANCE.md) y [`CONTRIBUTING.md`](CONTRIBUTING.md) para detalles completos.

## 💰 Apoyo & Financiamiento

ed2kIA es infraestructura pública de código abierto, análogo a Linux para la interpretabilidad de IA. Los incentivos son reputación técnica, impacto comunitario y gobernanza meritocrática. **No hay tokens, pools de liquidez ni mecanismos especulativos en el código.**

Financiamiento transparente vía:
- **Open Collective:** Gastos técnicos del proyecto (infraestructura, auditorías, CI/CD)
- **Gitcoin Grants:** Financiamiento comunitario vía matching pool
- **GitHub Sponsors:** Apoyo mensual de individuos y organizaciones

Ver [`docs/TRANSPARENCY_FRAMEWORK.md`](docs/TRANSPARENCY_FRAMEWORK.md) para detalles completos.

##  Contacto

- **Issue Tracker:** [GitHub Issues](https://github.com/ed2kia/ed2kIA/issues)
- **Discusión:** [GitHub Discussions](https://github.com/ed2kia/ed2kIA/discussions)

---

## 💬 Declaración del Creador

> "Como creador e ingeniero en jefe de ed2kIA declaro este proyecto como muestra mi de amor por la vida y toda la humanidad"
>
> **Roberto Estuardo Celis Hernández** — RECH

---

**ed2kIA** — Red descentralizada de interpretabilidad de IA para el beneficio humano. **v2.0.0-stable** | Modo: STEWARDSHIP | [Constitución](docs/governance/project-constitution.md) | [Source of Truth](docs/roadmap/source-of-truth.md)
