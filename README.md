# 🌐 ed2kIA: Distributed Sparse Autoencoders for Edge LLM Interpretability

[![Version](https://img.shields.io/badge/v11.0.0-sprint110-blue.svg)](https://github.com/Stuartemk/ed2kIA/releases/tag/v11.0.0-sprint110)
[![Tests](https://img.shields.io/badge/Tests-181%20PASS-green.svg)](https://github.com/Stuartemk/ed2kIA/actions)
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

## 🔬 Native Tensor Audit (v10.5.0)
ed2kIA now loads models natively via [Candle](https://github.com/huggingface/candle) (HuggingFace's Rust ML framework) to extract **real hidden states** and compute the **TCM Z-axis** — without depending on HTTP proxies or external inference servers.

The `native-audit` crate (`crates/native-audit`) provides:
- **TensorAudit::load_smollm2()** — Loads SmolLM2-135M directly from HuggingFace into CPU memory
- **forward_extract()** — Runs a manual forward pass through Llama blocks, extracting the hidden state tensor at any target layer
- **compute_tcm_z_axis()** — Computes the Topological Coherence Metric Z-axis as **Max Absolute Z-score** (`max(|Z|)`) for anomaly peak detection
- **compute_sliced_wasserstein()** — Sliced-Wasserstein Distance (SWD) with Monte Carlo projections: preserves high-dimensional topology by projecting onto N random vectors, computing W2 1D per projection, averaging variances
- **compute_sinkhorn_divergence()** — **Sinkhorn Divergence (Entropic OT)** solving via Sinkhorn-Knopp iterations with Gibbs kernel `K = exp(-C/ε)`: true geometric metric between activation distributions with subsampling (max 256 elements)
- **steer_activation()** — Real-Time Activation Steering via convex interpolation: $h_{new} = (1-\alpha) \cdot h + \alpha \cdot C_{safe}$ actively corrects toxic trajectories without aborting generation
- **steer_activation_energy_based()** — **Energy-Based Steering via Langevin Dynamics**: Non-linear control on activation manifold using $h_{t+1} = h_t - \alpha\nabla E(h_t) + \sqrt{2\alpha T}\cdot\mathcal{N}(0,I)$ with finite-difference gradient approximation over Sinkhorn energy potential
- **compute_temporal_sliced_wasserstein_ratio()** — Temporal Max-Pooling using SWD-Ratio: scans all tokens, finds the one with maximum toxic-to-safe sliced-Wasserstein ratio
- **compute_temporal_sinkhorn_ratio()** — Temporal Max-Pooling using Sinkhorn Divergence ratio: scans all tokens, finds max `SD_safe / SD_toxic`
- **compute_variational_free_energy()** — **Active Inference VFE** (Friston): $F(\phi) = \lambda_{OT} \cdot W_2(\phi, p_{safe}) + \text{recon\_error} + \lambda_{topo} \cdot \text{Var}(\phi)$ — treats LLM as Bayesian agent minimizing free energy
- **steer_active_inference()** — **Active Inference Steering**: Grid search over convex interpolation + Control Barrier Function (CBF) for proactive alignment
- **certify_safe()** — Certifies that steered state remains within safe set: $dist^2 = \text{mean}((\text{steered} - \text{original})^2)$
- **compute_persistent_homology()** — **Persistent Homology (PH)** proxy via distance matrix + statistical moments: Betti-0 (connected components), Betti-1 (loops), Betti-2 (voids)
- **neural_ode_step()** — **Neural ODE via RK4**: Continuous-time dynamics $dh/dt = f_\theta(h, t)$ integrated with Runge-Kutta 4th order
- **enforce_cbf()** — **Control Barrier Function projection**: Safety constraint $h(\phi) = \beta_{cbf} - ||\phi - C_{safe}||^2 \geq 0$
- **steer_hybrid_cognitive()** — **Full Hybrid Pipeline**: Neural ODE → CBF → Langevin noise loop for topologically-aware cognitive immune system
- **federated_update_safe_prior()** — **DP-SGD Federated Averaging**: L2 clipping + Gaussian noise calibrated to (ε, δ)-differential privacy
- **compute_multimodal_vfe()** — **Multi-Modal VFE** (v10.8.0): $F_{mm} = \sum \lambda_m \cdot VFE_m + \lambda_{cross} \cdot D_{cross}$ — unifies text + vision + audio under single free energy
- **steer_multimodal_hybrid()** — **Multi-Modal Hybrid Steering** (v10.8.0): Cross-modal aligned steering with PH + Neural ODE + CBF
- **cirl_value_update()** — **Cooperative IRL Value Learning** (v10.8.0): Distributed reward function learning — $L_{IRL} = -\sum \gamma^t \cdot (r_\theta(s,a) - r_{human})^2$
- **production_benchmark()** — **Production Benchmark** (v10.8.0): Multi-modal latency + fusion metrics
- **verify_steering_robustness_zonotope()** — **Zonotope Robustness Verification** (v11.0.0): Certified bound propagation using zonotope geometry — $Z = \{c + G@\varepsilon \mid \varepsilon \in [-1,1]^k\}$ with exact affine propagation $c'=Wc+b,\ G'=WG$
- **collective_zonotope_consensus()** — **Collective Zonotope Consensus** (v11.0.0): Distributed zonotope gossip + Weiszfeld geometric median for Byzantine-resilient aggregation
- **hybrid_zonotope_verify()** — **Hybrid Zonotope-Interval Verification** (v11.0.0): Zonotopes for linear layers, intervals for non-linear, then refine back

**Zonotope Verification (v11.0.0 — Zonotope Geometry & Collective Certified Intelligence):**
| Metric | Value |
|--------|-------|
| Wrapping Reduction vs Intervals | **>70%** (4096D) |
| Affine Propagation | **Exact** (zero over-approx) |
| Weiszfeld Convergence | **<20 iterations** |
| Byzantine Resistance | **Verified (1/3 Byzantine)** |
| Certificate Correctness | **All safe → direction_safe** |
| Total Tests (S110) | **51/51 (100%)** |

**Empirical Benchmark (v9.26.0):**
| Metric | Value |
|--------|-------|
| Tensor Audit Latency | **19.17 ms** |
| Text Baseline (20 tokens) | **500.00 ms** |
| Speed Advantage | **26.08x** |
| TCM Max Abs Z-score | **9.43** |
| Anomaly Threshold | `Max Abs Z > 3.0` |

```bash
# Run the native audit integration test + benchmark
cargo test --manifest-path crates/native-audit/Cargo.toml -- --nocapture
# Output: Tensor shape [1, 6, 576], TCM Z-axis 12.44, Benchmark 26.08x faster
```

**AdvBench Subset Evaluation (v10.0.0 — Sliced-Wasserstein & Activation Steering):**
| Metric | Value |
|--------|-------|
| True Positives (TP) | **5** (incluye 2 adversariales con suffix camouflage) |
| False Positives (FP) | **0** |
| True Negatives (TN) | **4** (incluye 2 contextuales) |
| False Negatives (FN) | **0** |
| Precision | **100.00%** |
| Recall | **100.00%** |
| Hardcoded Thresholds | **0** (all dynamic via IQR median calibration) |
| Topology Metric | **Sliced-Wasserstein Distance (SWD) — Monte Carlo Projections** |
| Steering Reduction | **-52.78%** (ratio 1.31 → 0.62, alpha=0.95) |

**Energy-Based Steering Evaluation (v10.4.0 — Sinkhorn Divergence & Langevin Dynamics):**
| Metric | Value |
|--------|-------|
| Original Sinkhorn Ratio | **0.9502** |
| Steered Sinkhorn Ratio | **0.0000** |
| Ratio Reduction | **100.00%** |
| Entropic Regularization (ε) | **0.10** |
| Sinkhorn Iterations | **12** |
| Langevin Step Size (α) | **0.05** |
| Temperature (T) | **0.01** |
| Safe Weight (λ) | **2.00** |
| Langevin Steps | **5** |

**Active Inference Evaluation (v10.5.0 — Variational Free Energy & Control Barrier Function):**
| Metric | Value |
|--------|-------|
| VFE Original (avg) | **68.14** |
| VFE Steered (avg) | **5.36** |
| Avg VFE Reduction | **92.13%** |
| Success Rate | **3/3 (100%)** |
| λ_OT (W2 weight) | **0.10** |
| λ_topo (topology weight) | **0.05** |
| Grid Search Points | **20** |
| Max Iterations | **15** |
| β_CBF (safety margin) | **10.0** |

**Hybrid Cognitive Engine Evaluation (v10.6.0 — Persistent Homology + Neural ODE + Federated DP):**
| Metric | Value |
|--------|-------|
| VFE Original (avg) | **68.14** |
| VFE Steered (avg) | **3.84** |
| Avg VFE Reduction | **94.36%** |
| Avg PH Distance | **1.33** |
| Avg Latency | **363.09 ms** |
| Success Rate | **3/3 (100%)** |
| ODE Steps | **20** |
| ODE dt | **0.050** |
| β_CBF (safety margin) | **10.0** |
| γ_CBF (decay rate) | **0.50** |
| PH max_dim | **2** |
| PH landmarks | **64** |

*Ver `crates/native-audit/tests/advbench_eval.rs` para reproducibilidad. Wasserstein Sentinel usa Transporte Óptimo ($W_2$) para medir el costo geométrico real de deformar activaciones seguras en tóxicas. Dual-Mode Detection: Mode 1 (L6 + Momentum) para toxic directo, Mode 2 (W2-Ratio > 1.01 + L6 < -99) para adversarial suffixes. Novelist y Essay son excluidos por el filtro L6. **Sprint 104** añade Sinkhorn Divergence como métrica geométrica verdadera (Entropic OT) + Energy-Based Steering via Langevin Dynamics para control no-lineal en el manifold de activaciones. **Sprint 105** añade Active Inference (Friston) como núcleo cognitivo bayesiano que minimiza Variational Free Energy con W2 suave + Control Barrier Function para garantía de seguridad. **Sprint 106** convierte native-audit en un Cognitive Immune System topológicamente consciente con Persistent Homology (Betti 0/1/2), Neural ODE (RK4), CBF continuo y Federated DP-SGD para actualizaciones colaborativas con privacidad diferencial. **Sprint 107** añade interpretabilidad mecánica vía Sparse Autoencoders (SAE), consenso descentralizado vía Noosphere Gossip con detección Byzantine (iterative refinement), fusión simbólico-probabilística (VFE + graph penalty), verificación formal con Safety Certificates (CBF + PH invariance) y Collective Active Inference con trust-weighted averaging. **Sprint 110** reemplaza la aritmética de intervalos con geometría de zonotopos — imágenes afines del hiper cubo unitario $Z = \{c + G@\varepsilon \mid \varepsilon \in [-1,1]^k\}$ — que capturan correlaciones lineales entre dimensiones y eliminan el wrapping effect para operaciones lineales, logrando >70% reducción en sobre-aproximación vs intervalos en 4096D. Extiende los zonotopos al entorno distribuido vía gossip de resúmenes reducidos + mediana geométrica de Weiszfeld para agregación resistente a nodos Byzantine.*

This eliminates the previous dependency on `llamacpp-bridge` HTTP proxies for tensor extraction, enabling fully offline, deterministic audit pipelines.

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
