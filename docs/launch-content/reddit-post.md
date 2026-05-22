# Reddit Post: r/MachineLearning, r/rust, r/LocalLLaMA

**Title:** ed2kIA v2.1.0-stable: A Decentralized Network for Distributed AI Interpretability Using Sparse Autoencoders (Rust, 3505 tests, zero tokens)

**Subreddits:** r/MachineLearning, r/rust, r/LocalLLaMA, r/artificial

---

## Post Body

**TL;DR:** We built a decentralized network where volunteers run Sparse Autoencoders on AI model shards, converge on shared feature dictionaries via CRDT gossip, and steer model behavior using a 3D ethical evaluation tensor. No tokens, no VC, no telemetry. Rust, 3,505 tests, OSSF 8.5/10. [GitHub](https://github.com/ed2kia/ed2kIA) | [Technical Report](https://github.com/ed2kia/ed2kIA/blob/main/docs/technical-report.md)

---

### The Problem

Large Language Models are the most complex systems ever built. But understanding their internal representations is concentrated in a few organizations with the resources to train and run Sparse Autoencoders (SAEs).

This creates three issues:

1. **Single point of failure** — If the central interpretability pipeline goes down, the community loses access to auditable feature decompositions.
2. **Accountability gap** — External researchers cannot independently verify SAE training data, hyperparameters, or feature selection.
3. **Access inequality** — Researchers without institutional GPU clusters cannot contribute to interpretability at scale.

### What We Built

ed2kIA is a decentralized, community-operated network for distributed interpretability analysis. Think of it as a federated learning system, but for SAE feature extraction instead of model training.

**Architecture at a glance:**

- **Network:** libp2p with GossipSub, mDNS/KAD discovery, WebRTC for browser nodes
- **SAE Engine:** Qwen-Scope architecture, Candle backend, Top-K activation (K=256)
- **Consensus:** Proof of Symbiosis (PoSymb) — Ed25519 signatures + reputation quorum
- **State Sync:** CRDTs (GCounter, PNCounter, ORSet) — converges without coordination
- **Ethical Steering:** Stuartian Context Tensor (SCT) — 3D evaluation replacing binary approve/reject

### The Stuartian Context Tensor (SCT)

This is our core contribution to interpretability methodology. Instead of binary "this feature is good/bad," we evaluate on three continuous axes:

```
x: Perceived benefit    [0.0, 1.0]  (via Sigmoid)
y: Cost/Friction        [0.0, 1.0]  (via Sigmoid)
z: Ethical focus        [-1.0, 1.0] (via Tanh)
```

The golden rule: **if z < 0 → REJECTED**. Deterministic, no exceptions.

A feature with high benefit (x ≈ 1.0) but negative ethical focus (z < 0) is rejected regardless of benefit. This prevents ethically misaligned features from entering the shared dictionary.

### Proof of Symbiosis (PoSymb)

We replaced Proof of Work/Stake with Proof of Symbiosis. Every interpretability contribution requires:

1. **Ed25519 signature** — Attributable to a specific node
2. **Existential Credit threshold** — Reputation-based, non-transferable
3. **Quorum validation** — Minimum f+1 verifiers must agree

Nodes with good ethical history have more influence. Nodes with misaligned contributions are isolated via "Network Apoptosis" (our immune system).

### Key Metrics

| Metric | Value |
|--------|-------|
| Tests | 3,505 passing (≥80% coverage) |
| P2P Sync | 256 nodes in <100ms |
| SAE Forward Pass | 8192 latent in <50ms |
| CRDT Merge | 1000 peers in <10ms |
| Security Threats | 15 assessed, 15 mitigated, 0 open |
| OSSF Score | 8.5/10 |
| Financial Logic | Zero (no tokens, no staking, no trading) |
| Telemetry | Zero (no phone home, no analytics) |

### Three Non-Negotiables

1. **Zero financial logic** — Existential Credit is non-transferable reputation, not currency. Cannot be bought, sold, or traded.
2. **Zero telemetry** — No phone home, no analytics, no usage tracking. All metrics are local or peer-to-peer.
3. **Human-resolved conflicts** — Automated systems detect conflicts; human stewards decide resolution.

### Tech Stack

Rust 2021 • libp2p • Candle (ML) • Ed25519 • CRDTs • GossipSub • WebRTC • WASM • Tauri • Prometheus/Grafana

### Try It

```bash
# Quickstart (60 seconds)
curl -sSL https://github.com/ed2kia/ed2kIA/raw/main/scripts/quickstart.sh | sh

# Local testnet (3 nodes)
./scripts/testnet-mode.sh --nodes 3

# Run benchmarks
cargo bench -p ed2kIA-benchmarks
```

### Links

- **Repository:** https://github.com/ed2kia/ed2kIA
- **Technical Report:** https://github.com/ed2kia/ed2kIA/blob/main/docs/technical-report.md
- **Security Audit:** https://github.com/ed2kia/ed2kIA/blob/main/docs/security/production-threat-model.md
- **Changelog:** https://github.com/ed2kia/ed2kIA/blob/main/CHANGELOG.md
- **License:** Apache 2.0 + Ethical Use Clause

### We're Looking For

- Researchers interested in mechanistic interpretability
- Rust developers who want to contribute to decentralized AI infrastructure
- Organizations that want to audit their models using distributed SAE analysis
- Anyone who believes interpretability should be a public good

---

**AMA.** We're the core contributors and will answer technical questions about architecture, SCT calibration, CRDT convergence, or anything else.

---

**Crosspost Notes:**
- r/MachineLearning: Emphasize SCT and SAE methodology
- r/rust: Emphasize CRDT implementation, libp2p integration, test coverage
- r/LocalLLaMA: Emphasize browser nodes, WASM, running locally
- r/artificial: Emphasize ethical governance, zero financial logic
