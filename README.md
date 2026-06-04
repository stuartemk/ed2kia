# 🌐 ed2kIA: Distributed Sparse Autoencoders for Edge LLM Interpretability

[![Version](https://img.shields.io/badge/v9.22.0-sprint86-blue.svg)](https://github.com/Stuartemk/ed2kIA/releases/tag/v9.22.0-sprint86)
[![Tests](https://img.shields.io/badge/Tests-6631%20PASS-green.svg)](https://github.com/Stuartemk/ed2kIA/actions)
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
- **Topological Coherence Metric (TCM):** 3D activation space for real-time misalignment detection
- **Automated Byzantine Eviction:** Staleness-aware weighting + BFT median aggregation
- **Compute Credits (CE):** Earn credits by running a node; spend credits to audit models
- **Post-Quantum Ready:** zk-STARKs, Ed25519, recursive SNARKs for proof aggregation

## 📦 Workspace Structure (v9.21.0)
```
ed2kIA/
├── crates/
│   ├── sae/            # Sparse Autoencoder module
│   ├── p2p/            # P2P networking layer (libp2p)
│   ├── consensus/      # Consensus (PoN, ZKP, MPC)
│   └── cli/            # CLI interface
├── src/                # Core library (feature-gated modules)
├── config/             # Bootstrap peers, node config
├── benchmarks/         # Reproducible evaluation scripts
└── tests/              # Integration + stress tests
```

## 📈 Comparative Analysis
| Feature | Petals | Anthropic SAE | ed2kIA |
|---------|--------|---------------|--------|
| Distributed Inference | ✅ | ❌ | ✅ |
| SAE Interpretability | ❌ | ✅ | ✅ |
| Edge/WASM Deployment | ❌ | ❌ | ✅ |
| Sybil Resistance | Low | N/A | High (BFT + TCM) |
| Open Source / Audit | ✅ | ❌ | ✅ |
| Real-time Activation Steering | ❌ | ❌ | ✅ |

## 🛠️ Development & Testing
```bash
# Build workspace
cargo build --workspace --features stable-core

# Run tests
cargo test --workspace --features stable-core

# Run benchmarks
bash benchmarks/run_advbench_eval.sh

# Deploy testnet
bash scripts/deploy_testnet.sh
```

## 🤝 Contributing
See [`CONTRIBUTING.md`](CONTRIBUTING.md) for workspace structure, coding standards, and PR workflow.

## 📜 Governance & Long-Term Vision
Technical specifications, ethical invariants, and architectural philosophy are documented in [`/philosophy/WHITE_PAPER.md`](philosophy/WHITE_PAPER.md).

---
*Built for interpretability, transparency, and symbiotic compute. Zero surveillance. Zero centralization.*
