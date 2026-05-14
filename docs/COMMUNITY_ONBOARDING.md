# Community Onboarding - ed2kIA

## Welcome to ed2kIA

ed2kIA is a decentralized network for interpretable AI using Sparse Autoencoders (SAEs). This guide will help you get started as a community member, contributor, or node operator.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Understanding ed2kIA](#understanding-ed2kia)
3. [Contribution Flow](#contribution-flow)
4. [Communication Channels](#communication-channels)
5. [Code of Conduct](#code-of-conduct)
6. [Resources](#resources)

---

## Quick Start

### Prerequisites

- **Git**: https://git-scm.com/downloads
- **Rust 1.75+**: https://rustup.rs/
- **Basic Linux/terminal knowledge**

### 5-Minute Setup

```bash
# 1. Clone the repository
git clone https://github.com/ed2kIA/ed2kIA.git
cd ed2kIA

# 2. Build with core features
cargo build --features "core-only"

# 3. Run tests
cargo test --features "core-only"

# 4. Start a local node
cargo run --features "core-only" -- run --listen-port 3030
```

### First Contribution

1. **Find a good first issue**: Browse issues labeled `good-first-issue`
2. **Fork the repository**: Click "Fork" on GitHub
3. **Create a branch**: `git checkout -b feature/my-contribution`
4. **Make your changes**: Follow the coding standards
5. **Run tests**: `cargo test --features "core-only"`
6. **Submit a PR**: Use the pull request template

---

## Understanding ed2kIA

### Architecture Overview

ed2kIA consists of 6 phases of development:

| Phase | Name | Status | Description |
|-------|------|--------|-------------|
| **Fase 1** | SAE Core | ✅ Stable | Sparse Autoencoder loading and forward pass |
| **Fase 2** | P2P Network | ✅ Stable | libp2p-based peer-to-peer communication |
| **Fase 3** | Interpretability | ✅ Stable | Feature analysis, semantic mapping, feedback |
| **Fase 4** | Consensus & ZKP | ✅ Stable | Batch consensus, ZKP verification, Merkle trees |
| **Fase 5** | Human-in-the-Loop | ✅ Stable | RLHF feedback, concept updates, governance |
| **Fase 6** | Interoperability | 🚧 Dev | Federation, staking, API v2, cross-chain |

### Key Concepts

- **SAE (Sparse Autoencoder)**: Neural network that extracts sparse, interpretable features
- **Layer Lease**: Temporary ownership of an SAE layer for processing
- **Consensus Batch**: Group of feature activations verified by multiple nodes
- **ZKP (Zero-Knowledge Proof)**: Cryptographic proof of correct computation
- **Reputation Score**: Trust metric for node contributions (0.0 - 1.0)
- **Feedback Loop**: Human annotations that improve semantic mappings

### Feature Flags

| Flag | Description | Use Case |
|------|-------------|----------|
| `core-only` | Fases 1-3 only | Production nodes, minimal dependencies |
| `phase6-experimental` | Fases 4-6 | Development, testing new features |

---

## Contribution Flow

### Development Workflow

```bash
# 1. Sync main branch
git fetch origin
git checkout main
git pull

# 2. Create feature branch
git checkout -b feature/my-feature

# 3. Make changes and commit
git add .
git commit -m "feat: add my feature

Detailed description of changes.

Closes #123"

# 4. Push and create PR
git push origin feature/my-feature
# Create PR on GitHub
```

### Branch Naming Convention

| Type | Format | Example |
|------|--------|---------|
| Feature | `feature/<name>` | `feature/wasm-memory-limit` |
| Bug Fix | `fix/<name>` | `fix/consensus-timeout` |
| Documentation | `docs/<name>` | `docs/api-reference` |
| Phase 6 Work | `fase6/<name>` | `fase6/staking-registry` |

### Commit Message Format

```
type: short description

Longer description if needed.

- Bullet points for details
- Reference issues: Closes #123

type: feat | fix | docs | style | refactor | test | chore | perf
```

### Code Review Process

1. **Self-review**: Run `cargo clippy`, `cargo fmt`, `cargo test`
2. **Submit PR**: Fill out the PR template completely
3. **CI checks**: Wait for automated tests to pass
4. **Reviewer assignment**: Maintainers assign reviewers
5. **Address feedback**: Make requested changes
6. **Approval**: 2 approvals required for merge
7. **Merge**: Squash and merge to `main`

### Coding Standards

- **Rust**: Follow `rustfmt` defaults
- **Documentation**: All public functions need doc comments
- **Error handling**: Use `Result<T, E>`, avoid `unwrap()` in production code
- **Testing**: New features need tests, aim for >80% coverage
- **Clippy**: Zero warnings allowed

---

## Communication Channels

| Channel | Purpose | Link |
|---------|---------|------|
| **GitHub Issues** | Bug reports, feature requests | https://github.com/ed2kIA/ed2kIA/issues |
| **GitHub Discussions** | General discussion, Q&A | https://github.com/ed2kIA/ed2kIA/discussions |
| **Discord** | Real-time chat, community | [TBD] |
| **Matrix** | Decentralized chat | [TBD] |
| **Email** | Security reports | security@ed2kIA.org |

### Asking for Help

1. **Search first**: Check existing issues and documentation
2. **Be specific**: Include environment, logs, reproduction steps
3. **Use templates**: Fill out issue templates completely
4. **Be patient**: Maintainers are volunteers

---

## Code of Conduct

All participants must follow our [Code of Conduct](../.github/CODE_OF_CONDUCT.md). Key principles:

- **Respect**: Treat everyone with dignity
- **Inclusion**: Welcome diverse perspectives
- **Transparency**: Communicate openly
- **Collaboration**: Work together for the common good

---

## Resources

### Documentation

- [Operations Runbook](OPERATIONS_RUNBOOK.md) - Production operations guide
- [Node Operator Guide](NODE_OPERATOR_GUIDE.md) - Running a node
- [Release Notes](RELEASE_NOTES_v0.5.0.md) - Version history
- [Phase 6 Roadmap](PHASE6_ROADMAP.md) - Future development

### Technical References

- [SAE Research](https://www.lesswrong.com/sauY8pY73Nioy2Bky) - Interpretability background
- [libp2p Documentation](https://docs.libp2p.io/) - P2P network library
- [WASMtime Docs](https://docs.wasmtime.dev/) - WASM runtime
- [Arkworks Docs](https://docs.rs/ark-ec/) - Cryptographic primitives

### Development Tools

```bash
# Format code
cargo fmt

# Run clippy
cargo clippy --features "core-only"

# Run tests with coverage
cargo-tarpaulin --features "core-only" --out Html

# Generate documentation
cargo doc --no-deps --open

# Benchmark
cargo bench
```

### Getting Help

1. Check [FAQ](https://github.com/ed2kIA/ed2kIA/wiki/FAQ)
2. Search [GitHub Issues](https://github.com/ed2kIA/ed2kIA/issues)
3. Ask in [Discussions](https://github.com/ed2kIA/ed2kIA/discussions)
4. Join [Discord](https://discord.gg/ed2kIA)

---

## Next Steps

- [ ] Read the [Node Operator Guide](NODE_OPERATOR_GUIDE.md)
- [ ] Run a local node
- [ ] Find a `good-first-issue` to work on
- [ ] Join the community chat
- [ ] Introduce yourself in Discussions
