# Guía de Contribución a ed2kIA

> Bienvenido/a al proyecto ed2kIA. Esta guía te ayudará a contribuir de manera efectiva y segura.

## 🌍 Mandato Ético

Antes de contribuir, acepta implícitamente el mandato ético de ed2kIA:

- Este software es para el **bienestar humano y desarrollo responsable de IA**.
- Debe usarse de forma **transparente, auditable y libre de backdoors**.
- Es compatible con **infraestructura voluntaria global**.
- Licencia: **Apache 2.0 + Cláusula de Uso Ético**.

## 📋 Requisitos Previos

### Herramientas

- **Rust 1.75+** (edición 2021)
- **Cargo** (incluido con Rust)
- **Git**
- **Opcional:** CUDA toolkit (GPU), Docker, wasmtime

### Instalar Rust

```bash
# Windows
winget install Rustlang.Rust

# Linux/macOS
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.sh | sh
```

## 🚀 Setup del Proyecto

### 1. Clonar el repositorio

```bash
git clone https://github.com/ed2kia/ed2kIA.git
cd ed2kIA
```

### 2. Compilar

```bash
# Build básico (CPU)
cargo build

# Build con GPU CUDA
cargo build --features cuda

# Build con Metal (Apple Silicon)
cargo build --features metal

# Build de producción
cargo build --release
```

### 3. Verificar

```bash
# Syntax check
cargo check

# Tests unitarios
cargo test

# Lint
cargo clippy --all-targets --all-features -- -D warnings

# Formateo
cargo fmt --all
```

## 📐 Estructura del Código

```
src/
├── main.rs                 # Orquestador + CLI
├── p2p/                    # Red P2P (libp2p)
├── sae/                    # SAE Loader + Router
├── bridge/                 # Tensor Flow + Consciousness
├── interpret/              # Feature Analyzer + Semantic Map
├── consensus/              # Validator + Merkle
├── security/               # WASM Sandbox + Memory Guard
├── zkp/                    # Zero-Knowledge Proofs
├── human/                  # Human-in-the-loop
├── scaling/                # Peer Manager + Bootstrap (F4)
├── rlhf/                   # Feedback Store + Trainer Loop (F4)
├── web/                    # Web Server + Routes (F4)
├── monitoring/             # Metrics + Health (F4)
├── governance/             # Proposals + Voting (F5)
├── reputation/             # Ledger + Scoring (F5)
├── ecosystem/              # HF Sync + Model Registry (F5)
└── bootstrap/              # Seed Registry + Network Init (F5)
```

## 🛠️ Cómo Contribuir

### 1. Encontrar un Issue

- Revisa [GitHub Issues](https://github.com/ed2kia/ed2kIA/issues)
- Busca etiquetas: `good first issue`, `help wanted`, `phase-X`
- Comenta en el issue para claimarlo

### 2. Crear Rama

```bash
git checkout -b feature/phase5-my-feature
# o
git checkout -b fix/phase3-memory-leak
```

### 3. Desarrollar

- Sigue las convenciones de código existentes.
- Documenta cada función con `///`.
- Usa `tracing` para logs (`info!`, `warn!`, `error!`).
- Marca TODOs con `// TODO: Phase X - descripción`.

### 4. Tests

```bash
# Tests unitarios
cargo test

# Tests con coverage (requiere tarpaulin)
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

### 5. Commit y Push

```bash
git add .
git commit -m "feat(governance): add proposal signature verification"
git push origin feature/phase5-my-feature
```

### Convenciones de Commit

```
type(scope): description

Types:
- feat: Nueva funcionalidad
- fix: Corrección de bug
- docs: Documentación
- refactor: Refactorización
- test: Tests
- chore: Mantenimiento
```

### 6. Pull Request

- Crear PR contra `main`
- Incluir descripción clara del cambio
- Enlazar issue relacionado
- Esperar review de maintainers

## 🐛 Debugging

### Logs

```bash
# Verbose logs
RUST_LOG=debug cargo run -- join

# Solo warnings+
RUST_LOG=warn cargo run -- join

# Logs específicos de módulo
RUST_LOG=ed2kia::p2p=debug cargo run -- join
```

### Memory Profiling

```bash
# Valgrind (Linux)
valgrind --tool=massif target/release/ed2kia run

# Heaptrack (Linux)
heaptrack target/release/ed2kia run
```

### P2P Debugging

```bash
# Ver tráfico P2P
RUST_LOG=libp2p=debug cargo run -- join

# Ver GossipSub
RUST_LOG=gossipsub=debug cargo run -- join
```

## 📊 Métricas y Observabilidad

### Prometheus

```bash
# Acceder a métricas
curl http://localhost:3000/api/metrics
```

### Health Checks

```bash
# Verificar salud del nodo
curl http://localhost:3000/api/health
```

## 🔒 Seguridad

### Reglas

1. **Nunca** commitear claves privadas o secretos.
2. **Siempre** verificar dependencias: `cargo audit`
3. **Documentar** cualquier decisión de seguridad.
4. **Reportar** vulnerabilidades de forma privada.

### Auditoría

```bash
# Instalar cargo-audit
cargo install cargo-audit

# Ejecutar auditoría
cargo audit
```

## 📚 Recursos

- **Documentación Rust:** https://doc.rust-lang.org/
- **libp2p Docs:** https://docs.libp2p.io/
- **Candle Docs:** https://github.com/huggingface/candle
- **wasmtime Docs:** https://wasmtime.dev/
- **arkworks Docs:** https://arkworks.rs/

## 🤝 Código de Conducta

- Sé respetuoso/a con todos los contribuyentes.
- Asume buena fe.
- Critica el código, no a la persona.
- Sé inclusivo/a y acogedor/a.

## 💰 Incentivos de Contribución

ed2kIA es infraestructura pública. Los incentivos son puramente técnicos:

| Contribución | Incentivo |
|--------------|-----------|
| Código | Reputación técnica, peso en gobernanza |
| Documentación | Reconocimiento público, reputación |
| Auditoría de seguridad | Reconocimiento, reputación técnica |
| Operación de nodo | Créditos de cómputo, peso en consenso |
| Investigación | Publicación, reconocimiento académico |

**Ningún incentivo es financiero.** Los módulos `src/reputation/` y `src/staking/` son puramente técnicos: representan créditos de cómputo, peso en gobernanza y métricas anti-Sybil. No hay tokens ni mecanismos especulativos.

Financiamiento del proyecto vía Open Collective + Gitcoin Grants + GitHub Sponsors. Ver [`docs/TRANSPARENCY_FRAMEWORK.md`](TRANSPARENCY_FRAMEWORK.md).

## 📞 Contacto

- **Issues:** https://github.com/ed2kia/ed2kIA/issues
- **Discussions:** https://github.com/ed2kia/ed2kIA/discussions

---

**Gracias por contribuir a ed2kIA** - Descentralizando la interpretabilidad de IA para el beneficio humano.
