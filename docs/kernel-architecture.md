# ed2kIA Kernel Architecture — Unified Blueprint (v2.1.0-sprint17)

> **Kernel Estuardiano**: Un organismo coherente, resiliente y protegido éticamente.
> Las 5 Leyes Estuardianas implementadas en código, validadas mediante E2E cross-validation.

---

## Table of Contents

1. [Philosophy: The 5 Stuartian Laws](#1-philosophy-the-5-stuartian-laws)
2. [System Architecture Overview](#2-system-architecture-overview)
3. [Module Map & Feature Gates](#3-module-map--feature-gates)
4. [E2E Data Flow: Kernel Pipeline](#4-e2e-data-flow-kernel-pipeline)
5. [Security Matrix](#5-security-matrix)
6. [Health Metrics & Observability](#6-health-metrics--observability)
7. [Operational Runbook](#7-operational-runbook)
8. [CRDT Convergence Guarantees](#8-crdt-convergence-guarantees)
9. [Activation Protocol](#9-activation-protocol)
10. [Glossary](#10-glossary)

---

## 1. Philosophy: The 5 Stuartian Laws

| Law | Name | Module | Feature Gate | Description |
|-----|------|--------|--------------|-------------|
| **Ley 1** | P2P Sovereignty | `async_gossip::mesh` | `v2.1-async-gossip` | GossipSub mesh con tolerancia a particiones. Cada nodo es soberano. |
| **Ley 2** | Transparent Audit | `alignment::sct_guard` + `federated::bft_aggregator` | `v2.1-sct-guard` + `v2.1-bft-aggregation` | SCT evalúa ética, BFT agrega con mediana coordenada. Cero censura. |
| **Ley 3** | Zero Waste | `qlora_gguf::{loader,adapter}` | `v2.1-qlora-gguf` | GGUF inmutable + QLoRA quantizado. Cero desperdicio computacional. |
| **Ley 4** | Edge Distribution | `orchestrator` + `sae::wasm_sharding` | `v2.1-orchestrator` + `v2.1-wasm-micro-sharding` | Micro-sharding ≤50MB para WASM/Edge. Computación distribuida. |
| **Ley 5** | Multiple Possibilities | `async_gossip::{cache,crdt}` | `v2.1-offline-cache` + `v2.1-crdt-state` | CRDTs sin bloqueos, convergencia eventual, tolerancia offline. |

### Law Integration Matrix

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    KERNEL ESTUARDIANO v2.1.0                            │
├─────────────┬─────────────┬─────────────┬─────────────┬─────────────────┤
│  Ley 1      │  Ley 2      │  Ley 3      │  Ley 4      │  Ley 5          │
│  P2P        │  Audit      │  Zero Waste │  Edge       │  Multiple       │
│  Sovereignty│  Transparent│             │  Dist.      │  Possibilities  │
├─────────────┼─────────────┼─────────────┼─────────────┼─────────────────┤
│ GossipSub   │ SCT Guard   │ GGUF Base   │ WASM Shard  │ CRDT Merge      │
│ Mesh Config │ BFT Median  │ QLoRA Diff  │ ≤50MB       │ No Locks        │
│ Peer Mgmt   │ KL Diverg.  │ FP8/INT4    │ Edge Node   │ Offline Cache   │
│ Msg Dedup   │ Slashing    │ Zero-Copy   │ Browser     │ Version Vector  │
└─────────────┴─────────────┴─────────────┴─────────────┴─────────────────┘
```

---

## 2. System Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         ed2kIA Mainnet Node                             │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐               │
│  │  GossipSub   │    │  SCT Guard   │    │  BFT         │               │
│  │  Mesh        │───>│  (Z < 0?)    │───>│  Aggregator  │               │
│  │  (Ley 1)     │    │  (Ley 2)     │    │  (Ley 2)     │               │
│  └──────┬───────┘    └──────────────┘    └──────┬───────┘               │
│         │                                       │                       │
│         │         ┌──────────────┐              │                       │
│         │         │  GGUF Base   │              │                       │
│         │         │  + QLoRA     │              │                       │
│         │         │  (Ley 3)     │              │                       │
│         │         └──────┬───────┘              │                       │
│         │                │                      │                       │
│         │         ┌──────▼───────┐    ┌────────▼────────┐              │
│         │         │  CRDT State  │    │  Offline Cache   │              │
│         │         │  (Ley 5)     │    │  (Ley 5)        │              │
│         │         └──────────────┘    └─────────────────┘              │
│         │                                                               │
│         │         ┌──────────────┐                                     │
│         └────────>│  WASM Edge   │                                     │
│                   │  Nodes       │                                     │
│                   │  (Ley 4)     │                                     │
│                   └──────────────┘                                     │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Key Structs |
|-----------|---------------|-------------|
| **GossipSub Mesh** | Peer management, message dedup, fanout | `GossipMesh`, `MeshConfig`, `MeshMessage` |
| **SCT Guard** | Ethical payload inspection, Z < 0 rejection | `SctGuard`, `GuardVerdict` |
| **BFT Aggregator** | Coordinate-wise median, Multi-Krum selection | `BftAggregator`, `BftConfig` |
| **GGUF Loader** | Model validation, SHA256 integrity | `GgufLoader`, `GgufModelInfo` |
| **QLoRA Adapter** | Quantized diff application, forward pass | `QloraAdapter`, `AdapterInfo` |
| **CRDT State** | Conflict-free reputation, GCounter/PNCounter/ORSet | `ReputationCrdt`, `VersionVector` |
| **Offline Cache** | Priority queue, exponential backoff sync | `GossipCache`, `CacheEntry` |
| **WASM Edge** | Micro-sharding, browser node bridge | `WasmSharding`, `BrowserNode` |

---

## 3. Module Map & Feature Gates

### Feature Gate Hierarchy

```
v2.1-kernel-integration (Sprint17)
├── v2.1-async-gossip          # GossipSub mesh
├── v2.1-offline-cache         # Offline storage
├── v2.1-crdt-state            # CRDT convergence
├── v2.1-bft-aggregation       # BFT median
├── v2.1-sct-guard             # SCT ethical shield
├── v2.1-sct-core              # SCT tensor core
├── v2.1-qlora-gguf            # QLoRA + GGUF
├── v2.1-proof-of-comprehension # PoC tasks
├── v2.1-stuartian-filter      # KL divergence + slashing
└── v2.1-chaos-engine          # Fault injection

v2.1-mainnet-activation (Sprint17)
└── v2.1-kernel-integration    # All above + activation protocol
```

### Module Dependencies

```
async_gossip::mesh
    ├── no external deps (self-contained)
    └── tests: 25+

async_gossip::cache
    ├── no external deps
    └── tests: 30+

async_gossip::crdt
    ├── bincode (serialization)
    └── tests: 40+

alignment::sct_guard
    ├── alignment::sct_core
    └── tests: 20+

federated::bft_aggregator
    ├── no external deps
    └── tests: 25+

qlora_gguf::loader
    ├── memmap2, safetensors
    └── tests: 15+

qlora_gguf::adapter
    ├── candle-core
    └── tests: 20+

qlora_gguf::payload
    ├── zstd (compression)
    └── tests: 15+
```

---

## 4. E2E Data Flow: Kernel Pipeline

### Complete Pipeline: GGUF → QLoRA → PoC → SCT → BFT → CRDT → Gossip → Cache

```
Step 1: GGUF Load (Ley 3)
┌─────────────────────────────────────────────┐
│ GgufLoader::validate(path)                  │
│   → SHA256 integrity check                  │
│   → Magic number validation                 │
│   → Architecture extraction                 │
│   → GgufBaseModel loaded                    │
└──────────────────┬──────────────────────────┘
                   │
                   ▼
Step 2: QLoRA Forward (Ley 3)
┌─────────────────────────────────────────────┐
│ QloraAdapter::apply(x)                      │
│   → W' = W_base + α·(B·A)·x               │
│   → FP8/INT4 quantized diff                 │
│   → Zero-copy memory mapping                │
└──────────────────┬──────────────────────────┘
                   │
                   ▼
Step 3: PoC Task Generation (Ley 2)
┌─────────────────────────────────────────────┐
│ PoCTask::new(task_id, dim, deadline)        │
│   → SAE activation batch                    │
│   → Cryptographic proof of useful work      │
│   → Gradient validation                     │
└──────────────────┬──────────────────────────┘
                   │
                   ▼
Step 4: SCT Evaluation (Ley 2)
┌─────────────────────────────────────────────┐
│ SctGuard::inspect_payload(node_id, payload) │
│   → StuartianTensor {x, y, z}              │
│   → Z < 0? → REJECT + slash_reputation     │
│   → Z >= 0? → APPROVE + forward to BFT     │
└──────────────────┬──────────────────────────┘
                   │
                   ▼
Step 5: BFT Aggregation (Ley 2)
┌─────────────────────────────────────────────┐
│ BftAggregator::aggregate(gradients)         │
│   → Coordinate-wise median                  │
│   → Multi-Krum selection (m=2)              │
│   → Outlier filtering (σ threshold)         │
│   → Aggregated gradient returned            │
└──────────────────┬──────────────────────────┘
                   │
                   ▼
Step 6: CRDT Merge (Ley 5)
┌─────────────────────────────────────────────┐
│ ReputationCrdt::merge(other)                │
│   → Commutative: merge(a,b) = merge(b,a)   │
│   → Associative: merge(a,merge(b,c)) = ...  │
│   → Idempotent: merge(a,a) = a             │
│   → No locks required                       │
└──────────────────┬──────────────────────────┘
                   │
                   ▼
Step 7: Gossip Publish (Ley 1)
┌─────────────────────────────────────────────┐
│ GossipMesh::publish(payload)                │
│   → QloraPayload::compress()                │
│   → to_gossipsub_bytes()                    │
│   → Fan-out to mesh peers                   │
│   → Message dedup via hash                  │
└──────────────────┬──────────────────────────┘
                   │
                   ▼
Step 8: Offline Cache (Ley 5)
┌─────────────────────────────────────────────┐
│ GossipCache::store(key, data, priority)     │
│   → Priority queue (timestamp-based)        │
│   → Exponential backoff on sync failure     │
│   → Sync batch on reconnection              │
│   → mark_synced() on success                │
└─────────────────────────────────────────────┘
```

### Pipeline Validation

Each step is validated in `tests/integration/kernel_e2e_test.rs`:

| Stage | Test Function | Stuartian Law |
|-------|--------------|---------------|
| GGUF Load | `stage1_gguf_loader_validates_integrity` | Ley 3 |
| QLoRA Forward | `stage2_qlora_adapter_forward_pass` | Ley 3 |
| Payload Compress | `stage3_qlora_payload_compress_for_gossipsub` | Ley 1 |
| PoC Task | `stage4_poc_task_lifecycle` | Ley 2 |
| SCT Guard (approve) | `stage5_sct_guard_approves_ethical_payload` | Ley 2 |
| SCT Guard (block) | `stage5_sct_guard_blocks_malicious_payload` | Ley 2 |
| BFT Aggregation | `stage6_bft_aggregation_rejects_byzantine` | Ley 2 |
| BFT Median | `stage6_bft_coordinate_wise_median` | Ley 2 |
| CRDT Reputation | `stage7_crdt_reputation_convergence` | Ley 5 |
| CRDT GCounter | `stage7_crdt_gcounter_idempotent_merge` | Ley 5 |
| CRDT ORSet | `stage7_crdt_orset_add_remove_convergence` | Ley 5 |
| Gossip Mesh | `stage8_gossip_mesh_publish_and_inject` | Ley 1 |
| Gossip Health | `stage8_gossip_mesh_health_check` | Ley 1 |
| Cache Store/Sync | `stage9_cache_store_and_sync` | Ley 5 |
| Cache Backoff | `stage9_cache_exponential_backoff` | Ley 5 |
| Version Vector | `stage10_version_vector_causal_ordering` | Ley 5 |
| Divergence Detect | `stage11_divergence_detection` | Ley 2 |
| Alignment Slashing | `stage12_slashing_penalty` | Ley 2 |
| Chaos Engine | `stage13_chaos_engine_lifecycle` | Ley 5 |
| PNCounter | `stage14_pncounter_bounded_reputation` | Ley 5 |
| **Full Pipeline** | `stage15_full_kernel_pipeline` | **All 5 Laws** |
| Error Handling | `stage16_error_handling_graceful` | All |

---

## 5. Security Matrix

### Threat Model & Mitigations

| Threat | Attack Vector | Mitigation | Module |
|--------|--------------|------------|--------|
| **Byzantine Node** | Malicious gradients | BFT coordinate-wise median + Multi-Krum | `bft_aggregator` |
| **Ethical Violation** | Z < 0 payloads | SCT Guard hard rejection + reputation slash | `sct_guard` |
| **Sybil Attack** | Fake node identities | Micro-PoW handshake + rate limiting | `sybil_micropow` |
| **Network Partition** | Split brain | CRDT eventual consistency + offline cache | `crdt` + `cache` |
| **Model Corruption** | Tampered GGUF | SHA256 integrity check on load | `loader` |
| **Gradient Poisoning** | Extreme outliers | Divergence detection + slashing | `divergence` + `slashing` |
| **DoS** | Message flood | Message dedup + TTL + rate limiting | `mesh` |
| **Censorship** | Block valid messages | BFT threshold = (N/2)+1, SCT transparency | `bft_aggregator` |

### Ethical Shield Layers

```
┌─────────────────────────────────────────────────────────┐
│  Layer 1: SCT Guard — Z < 0 hard rejection              │
│  Layer 2: BFT Median — Byzantine tolerance (f < N/3)    │
│  Layer 3: KL Divergence — Distributional drift detection │
│  Layer 4: Slashing — Deterministic reputation penalty   │
│  Layer 5: CRDT Max — Takes max reputation (no downgrade) │
└─────────────────────────────────────────────────────────┘
```

---

## 6. Health Metrics & Observability

### Key Metrics

| Metric | Type | Source | Description |
|--------|------|---------|-------------|
| `mesh_peer_count` | Gauge | `GossipMesh` | Active meshed peers |
| `mesh_message_rate` | Counter | `GossipMesh` | Messages published/sec |
| `sct_rejection_rate` | Gauge | `SctGuard` | Rejected / Total inspected |
| `bft_aggregation_latency` | Histogram | `BftAggregator` | Aggregation time (ms) |
| `crdt_merge_count` | Counter | `ReputationCrdt` | Total CRDT merges |
| `cache_sync_ratio` | Gauge | `GossipCache` | Synced / Total entries |
| `cache_backoff_ms` | Histogram | `GossipCache` | Backoff delays |
| `vv_vector_size` | Gauge | `VersionVector` | Active nodes in VV |
| `chaos_scenario_active` | Gauge | `ChaosEngine` | 1 if scenario running |

### Healthcheck Endpoints

| Endpoint | Method | Response |
|----------|--------|----------|
| `/api/health` | GET | `{"status": "healthy", "uptime": "..."}` |
| `/api/metrics` | GET | Prometheus text format |
| `/api/sct/status` | GET | `{"active": true, "violations": {...}}` |
| `/api/bft/status` | GET | `{"aggregations": N, "latency_p95": ...}` |

---

## 7. Operational Runbook

### Pre-Launch Checklist

```bash
# 1. Environment validation
bash scripts/activate-mainnet.sh --dry-run

# 2. Full validation pipeline
cargo check --features "v2.1-kernel-integration"
cargo test --test kernel_e2e_test --features "v2.1-kernel-integration"
cargo clippy --features "v2.1-kernel-integration" -- -D warnings

# 3. Syntax check activation script
bash -n scripts/activate-mainnet.sh

# 4. Dry run activation
bash scripts/activate-mainnet.sh --dry-run
```

### Launch Sequence

```bash
# 1. Activate mainnet (full pipeline)
bash scripts/activate-mainnet.sh --replicas 3 --log-level info

# 2. Verify health
curl http://localhost:9944/api/health

# 3. Check metrics
curl http://localhost:9944/api/metrics

# 4. Monitor Grafana
open http://localhost:3000
```

### Incident Response

| Incident | Detection | Response |
|----------|-----------|----------|
| Node crash | Healthcheck fails | Auto-restart via systemd/Docker |
| SCT violation spike | `sct_rejection_rate > 0.1` | Alert + investigate source nodes |
| BFT latency > 5s | `bft_aggregation_latency p95` | Scale replicas + check network |
| CRDT divergence | `crdt_merge_count` stalls | Force sync + check partition |
| Cache saturation | `cache_sync_ratio < 0.5` | Increase batch size + check connectivity |

### Rollback Procedure

```bash
# 1. Stop services
docker compose -f infra/docker-compose.testnet-v2.1.yml down

# 2. Rollback to previous tag
git checkout origin/main~1

# 3. Rebuild
cargo build --release --features "v2.1-kernel-integration"

# 4. Relaunch
bash scripts/activate-mainnet.sh --replicas 3
```

---

## 8. CRDT Convergence Guarantees

### Mathematical Properties

All CRDTs in `async_gossip::crdt` satisfy:

1. **Commutativity**: `merge(a, b) == merge(b, a)`
2. **Associativity**: `merge(a, merge(b, c)) == merge(merge(a, b), c)`
3. **Idempotency**: `merge(a, a) == a`

### CRDT Types

| Type | Operation | Merge Rule | Use Case |
|------|-----------|------------|----------|
| **GCounter** | Increment only | `max(a.x, b.x)` per node | Message counts |
| **PNCounter** | Inc + Dec (bounded) | `max()` inc, `max()` dec | Bounded reputation |
| **ORSet** | Add/Remove with tags | Union of tags, tombstones | Feature sets |
| **ReputationCrdt** | Max registry | `max(a.rep, b.rep)` per node | Node reputation |
| **VersionVector** | Causal ordering | `max()` per node | Dependency tracking |

### Convergence Proof (3-Node Example)

```
Initial: A={a:1}, B={b:1}, C={c:1}

Round 1: A→B, B→C
  B = merge(A, B) = {a:1, b:1}
  C = merge(B, C) = {a:1, b:1, c:1}

Round 2: C→A, A→B (full propagation)
  A = merge(C, A) = {a:1, b:1, c:1}
  B = merge(A, B) = {a:1, b:1, c:1}

Result: A == B == C ✓ (Full convergence in 2 rounds)
```

---

## 9. Activation Protocol

### `scripts/activate-mainnet.sh` — 6-Phase Activation

| Phase | Description | Validation |
|-------|-------------|------------|
| **Phase 1** | Environment Validation | Docker, Cargo, Git, required files |
| **Phase 2** | Pre-Launch Checks | cargo check, kernel_e2e_test, clippy |
| **Phase 3** | Docker Compose Launch | Build + up -d, container health |
| **Phase 4** | Healthchecks | `/api/health` HTTP 200, `/api/metrics` |
| **Phase 5** | SCTGuard + BFT Activation | POST `/api/sct/status`, GET `/api/bft/status` |
| **Phase 6** | Readiness Report | Final status with all endpoints |

### Activation Conditions

**Auto-push to `origin/main` ONLY if:**
- ✅ `cargo check` — zero errors
- ✅ `cargo test --test kernel_e2e_test` — 100% PASS
- ✅ `cargo clippy` — zero warnings
- ✅ `bash -n scripts/activate-mainnet.sh` — syntax valid
- ✅ All 5 Stuartian Laws validated in E2E test

---

## 10. Glossary

| Term | Definition |
|------|-----------|
| **Kernel Estuardiano** | Unified kernel implementing 5 Stuartian Laws as coherent organism |
| **SCT (Stuartian Context Tensor)** | 3D ethical alignment tensor {x, y, z} replacing 2D RLHF |
| **BFT (Byzantine Fault Tolerance)** | Coordinate-wise median + Multi-Krum for gradient aggregation |
| **CRDT** | Conflict-free Replicated Data Type — merge without locks |
| **QLoRA** | Quantized Low-Rank Adaptation — FP8/INT4 diffs over GGUF base |
| **GGUF** | Generic GGML Unified Format — immutable base model storage |
| **GossipSub** | Pub-sub mesh protocol for P2P message distribution |
| **Version Vector** | Causal clock tracking dependencies across partitions |
| **PoC (Proof of Comprehension)** | Cryptographic proof of useful SAE work |
| **KL Divergence** | Kullback-Leibler divergence — distributional drift detection |
| **Slashing** | Deterministic reputation penalty for misalignment |

---

## Appendix A: Git History & Versions

| Version | Sprint | Key Changes |
|---------|--------|-------------|
| v2.1.0-sprint16.4 | Async Gossip + CRDTs | GossipSub mesh, offline cache, CRDT convergence |
| v2.1.0-sprint16.3 | SCT Complete | SCT core, reward model, guard integration |
| v2.1.0-sprint16.2 | BFT + Committees | BFT aggregator, staleness awareness, committees |
| v2.1.0-sprint16.1 | QLoRA/GGUF | GGUF loader, QLoRA adapter, payload compression |
| v2.1.0-sprint17 | Kernel Integration | E2E validation, activation protocol, unified docs |

---

*Document generated: 2026-05-20 | Sprint17 | Kernel Integration Complete*
*ed2kIA v2.1.0 — Building on Stuartian Philosophy for Responsible AI*
