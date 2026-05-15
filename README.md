# ed2kIA - Red Descentralizada de Interpretabilidad

> **Imagina que los LLMs son como cajas negras que piensan en un idioma secreto. ed2kIA es como una red de traductores voluntarios distribuidos por todo el mundo que trabajan juntos para descifrar ese idioma, uno a uno, de forma transparente y verificable.**

> **Red descentralizada de código abierto para análisis interpretativo distribuido de LLMs usando Sparse Autoencoders (Qwen-Scope)**

[![License](https://img.shields.io/badge/License-Apache%202.0%20%2B%20Ethical-blue)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2021-orange)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/Version-1.8.0_BETA-brightgreen)](release/v1.8.0-beta.1/RELEASE_NOTES.md)
[![Tests](https://img.shields.io/badge/Tests-187_passed%20/%200_failed-success)](release/v1.6.0-stable/final_signoff.json)
[![Build](https://img.shields.io/badge/Build-0_errors%2C_0_warnings-success)](release/v1.6.0-stable/final_signoff.json)
[![CI](https://github.com/ed2kia/ed2kIA/actions/workflows/ci.yml/badge.svg)](https://github.com/ed2kia/ed2kIA/actions)

## 🌍 Mandato Ético

Este proyecto es de código abierto, transparente y diseñado exclusivamente para el progreso humano y el desarrollo responsable de la IA. Todo el código es auditable, libre de backdoors y compatible con infraestructura voluntaria global.

**Licencia:** Apache 2.0 + Cláusula de Uso Ético (bienestar humano/IA)

## 🌍 El Verdadero Poder de ed2kIA: Controlar a los Gigantes de la IA

Hoy en día, empresas multimillonarias como Google, OpenAI o Meta controlan las Inteligencias Artificiales más potentes del planeta. Esas IA son como "cajas negras": nadie sabe realmente cómo piensan, en qué se basan para tomar decisiones ni si nos están ocultando información.

Si solo las megacorporaciones pueden revisar el cerebro de los robots, ellas tendrán todo el poder del futuro. **ed2kIA cambia las reglas del juego.**

### 🤝 ¿Cómo planeamos vencer a los gigantes?
* **Un Supercomputador Humano:** En lugar de pagar millones de dólares en servidores, unimos las computadoras y teléfonos de miles de personas comunes en todo el mundo para crear una red de defensa digital.
* **Rayos X para Robots:** Nuestro software desarma las matemáticas complejas de la IA y las traduce a palabras que cualquier humano puede entender. Es un detector de mentiras universal para máquinas.
* **Poder para la Gente:** Al ser un proyecto 100% gratis y comunitario, le quitamos el monopolio a las grandes empresas. Cualquier estudiante o ciudadano podrá auditar a los robots para asegurarse de que no sean peligrosos, mentirosos o racistas.

No necesitas ser un científico para cambiar el futuro. Al prestar un poco de la potencia de tu PC o tu teléfono mientras duermes, te conviertes en un detective digital protegiendo a la humanidad.

## 📐 Arquitectura

### Decisiones Arquitectónicas Fijas

| Decisión | Implementación |
|----------|----------------|
| **Multiplataforma** | Windows/Linux/macOS desde Fase 1 |
| **Sharding** | Dinámico con Leases (5-10 min) gestionado por `LayerRouter` |
| **Comunicación** | Feedback Asincrónico + Steering Signals síncronos ligeros |
| **ZKP/WASM** | Placeholders seguros en Fase 1, implementación completa en Fase 3 |
| **Red P2P** | `libp2p` con KAD + mDNS para descubrimiento |
| **ML Engine** | `candle-core` + `candle-nn` + `safetensors` |
| **Serialización** | Prost (Protobuf) para metadatos, FlatBuffers para tensores |

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
│   │   └── router.rs       # LayerRouter (dynamic sharding + leases)
│   ├── bridge/
│   │   ├── tensor_flow.rs  # Node A → Node B tensor pipeline
│   │   └── consciousness.rs # Agregación + conflictos + steering (Fase 2)
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

## 🔒 Seguridad y Ética

- **Código auditable:** Todo el código es open source y revisable
- **Sin backdoors:** Verificado por la comunidad
- **Uso ético:** Diseñado exclusivamente para bienestar humano y IA responsable
- **Infraestructura voluntaria:** Participación opt-in global

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

**ed2kIA** - Descentralizando la interpretabilidad de IA para el beneficio humano.
