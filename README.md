# 🌐 ed2kIA: Decentralized SAE Audit Network for Local LLMs

[![Version](https://img.shields.io/badge/v9.18.0-sprint82-blue.svg)](https://github.com/Stuartemk/ed2kIA/releases/tag/v9.18.0-sprint82)
[![Tests](https://img.shields.io/badge/Tests-190%2F190%20PASS-green.svg)](https://github.com/Stuartemk/ed2kIA/actions)
[![Audit](https://img.shields.io/badge/OSSF-8.5%2F10-yellow.svg)](https://github.com/Stuartemk/ed2kIA/security)
[![License](https://img.shields.io/badge/License-Apache%202.0%20%2B%20Ética-orange.svg)](LICENSE)

## 🚀 Quick Start

Audit local LLMs (Qwen 2B/4B, Llama, Mistral) in real-time using a distributed P2P network. No GPU required.

```bash
curl -sSf https://ed2kia.network/install.sh | sh
ed2k start --model qwen3.5:2b
```

## 📊 Architecture

- **Edge-Optimized WASM:** Async tensor routing, <500ms boot, <2GB RAM footprint
- **Distributed SAE Pipeline:** Sparse Autoencoder activations routed via libp2p GossipSub
- **Compute Credits (CE):** Earn credits by running a node; spend credits to audit models
- **Post-Quantum Ready:** zk-STARKs, Ed25519, recursive SNARKs for proof aggregation

## 🛠️ Development

```bash
cargo test --features v9.18-mvp-deployment
cargo run --bin ed2k -- audit --prompt "test input"
```

## 📜 Philosophy & Governance

The ethical invariants, topological alignment, and long-term vision are documented in [`/philosophy/WHITE_PAPER.md`](philosophy/WHITE_PAPER.md).

---

*Built for interpretability, transparency, and symbiotic compute. Zero surveillance. Zero centralization.*
