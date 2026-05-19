# ed2kIA - Red Descentralizada de Interpretabilidad

> **Imagina que los LLMs son sistemas complejos que requieren auditoría colaborativa. ed2kIA es una red de verificadores voluntarios distribuidos por todo el mundo que trabajan juntos para hacer estos sistemas más comprensibles, uno a uno, de forma transparente y verificable.**

> **Red descentralizada de código abierto para análisis interpretativo distribuido de LLMs usando Sparse Autoencoders (Qwen-Scope)**

[![License](https://img.shields.io/badge/License-Apache%202.0%20%2B%20Ethical-blue)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2021-orange)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/Version-2.1.0-sprint6-yellowgreen)](CHANGELOG.md)
[![Tests](https://img.shields.io/badge/Tests-2929_passing-success)](CHANGELOG.md)
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
| `v2.1-task-redundancy` | N-Node Dispatch — Configurable replication_factor for redundant task assignment | ✅ Implementado (5 tests) |
| `v2.1-consensus-engine` | Consensus Engine — O(N) index-hash grouping + epsilon-tolerant majority rule | ✅ Implementado (10 tests) |
| `v2.1-reputation-system` | Reputation Matrix — DashMap scores (+1/-50) + auto-ban on negative | ✅ Implementado (13 tests) |
| `v2.1-observability` | Metrics, Health Check, Health Endpoint | Draft (RFC-002) |
| `v2.1-security-hardening` | wasmtime ≥24.0.7, rustls-webpki ≥0.103.13 | Planificado Q2-Q3 2027 |
| `v2.1-gui` | GUI Bridge, Mobile, 3D Visualizer | Draft |
| `v2.1-zkp-v3` | ZKP v3, Recursive Prover, Cross-Chain | Draft |
| `v2.1-enterprise` | SSO, K8s Operator, Compliance | Draft |

> **Nota:** Los feature gates `v2.1-*` NO están incluidos en `default = ["stable"]`. Requieren activación explícita vía RFC comunitario.

## 🛡️ Sistema Inmunológico — Defensa contra Data Poisoning

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
