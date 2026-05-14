# ed2kIA v1.3.0 — Architecture Documentation

**Date:** 2026-05-10
**Version:** 1.3.0-stable
**License:** Apache 2.0 + Ethical Use

---

## 1. System Overview

ed2kIA v1.3.0 is a decentralized AI verification infrastructure built in safe Rust. The system provides zero-knowledge proof generation/verification, adaptive federation sharding, distributed SAE fine-tuning, and cross-model gradient alignment — all operating behind the `--features stable` feature flag.

### Design Principles
1. **Zero Unsafe Rust:** All code compiles without `unsafe` blocks.
2. **Zero Financial Logic:** No tokens, staking rewards, or payment processing.
3. **Zero Telemetry:** No external data collection or analytics.
4. **Feature-Gated:** All v1.3 modules behind `cfg(feature = "stable")`.
5. **Deterministic Testing:** All behavior reproducible via configuration.

---

## 2. Module Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                        ed2kIA v1.3.0                              │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────┐  ┌──────────────────┐  ┌────────────────┐ │
│  │  Async ZKP v5   │  │ Federation       │  │ SAE Fine-      │ │
│  │                  │  │ Scaling v3       │  │ Tuning v3      │ │
│  │ • VRF Sampling  │  │ • Adaptive       │  │ • Cross-Model  │ │
│  │ • Parallel Ver. │  │ • Health Score   │  │ • Adaptive LR  │ │
│  │ • Incr. Merkle  │  │ • Consensus      │  │ • LZ4 CP       │ │
│  │ • Pre-compile   │  │ • Node Scoring   │  │ • Distributed  │ │
│  └────────┬────────┘  └────────┬─────────┘  └────────┬───────┘ │
│           │                    │                      │         │
│           │    ┌───────────────▼──────────────────────┤         │
│           │    │  Federation ZKP Bridge               │         │
│           │    │  • Cross-Shard Routing               │         │
│           │    │  • Merkle Root Sync                  │         │
│           │    │  • Resource-Aware Accept             │         │
│           │    └──────────────────────────────────────┘         │
│           │                                                     │
│  ┌────────▼────────┐                                            │
│  │ Cross-Model     │                                            │
│  │ Aligner         │                                            │
│  │ • Cosine Sim    │                                            │
│  │ • Dim Padding   │                                            │
│  │ • Score Track   │                                            │
│  └─────────────────┘                                            │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

---

## 3. Async ZKP v5

**File:** `src/zkp/async_zkp_v5.rs`

### Purpose
Next-generation proof engine with incremental Merkle accumulation, parallel verification, VRF-based sampling, and adaptive circuit selection.

### Key Types

| Type | Description |
|------|-------------|
| `AsyncZKPV5` | Main engine struct |
| `ZKPV5Config` | Configuration (batch size, workers, VRF rate, etc.) |
| `ZKPStatement` | Individual proof statement with pool, priority, complexity |
| `ZKPProof` | Generated proof with statement ID, circuit type, proof data |
| `ProofBatch` | Batch of statements ready for proof generation |
| `IncrementalAccumulator` | Sliding-window Merkle accumulator |
| `VerificationWorker` | Parallel verification worker (up to 32) |
| `PoolContext` | Resource pool with credits, reputation, latency |

### Configuration Defaults
```rust
ZKPV5Config {
    max_batch_size: 64,
    max_parallel_workers: 8,
    vrf_sampling_rate: 0.8,
    fallback_threshold: 2.0,
    max_pools: 16,
    proof_cache_size: 1024,
    accumulator_window: 128,
    precompile_enabled: true,
}
```

### VRF Sampling Logic
```rust
// In generate_batch_proofs():
let should_sample = batch.statements.len() > self.config.max_batch_size / 2;
let effective_rate = if should_sample { self.config.vrf_sampling_rate } else { 1.0 };
```
- When batch size exceeds `max_batch_size / 2`, VRF sampling activates.
- Otherwise, all statements are processed (rate = 1.0).

### Fallback Generation
```rust
fn should_use_fallback(&self, statement: &ZKPStatement) -> bool {
    let avg = self.avg_complexity(&statement.circuit_type);
    statement.complexity_score > avg * self.config.fallback_threshold
}
```
- High-complexity statements (> 2× average) use fallback proof generation.

### Parallel Verification
- Up to `max_parallel_workers` workers process proofs concurrently.
- Work distributed via priority queue (higher priority first, then FIFO).

---

## 4. Federation Scaling v3

**File:** `src/federation/scaling_v3.rs`

### Purpose
Adaptive sharding engine that monitors federation load and produces scaling decisions (`AddShard`, `RemoveShard`, `Rebalance`, `NoOp`).

### Key Types

| Type | Description |
|------|-------------|
| `FederationScalingV3` | Main scaling engine |
| `ScalingV3Config` | Thresholds and intervals |
| `NodeCapabilityV3` | Node with uptime, reputation, latency, resources |
| `ShardConfigV3` | Shard with node assignments and load tracking |
| `ScalingDecisionV3` | Decision with type, target, reason, confidence |
| `ScalingDecisionType` | Enum: AddShard, RemoveShard, Rebalance, NoOp |

### Configuration Defaults
```rust
ScalingV3Config {
    scale_up_threshold: 0.7,
    scale_down_threshold: 0.3,
    min_nodes_per_shard: 2,
    max_nodes_per_shard: 50,
    rebalance_threshold: 0.5,
    evaluation_interval_ms: 5000,
}
```

### Evaluation Flow
```
evaluate()
  ├── Compute federation_load (avg of all node loads)
  ├── If load > scale_up_threshold → AddShard
  ├── If load < scale_down_threshold → RemoveShard
  ├── If shard imbalance > rebalance_threshold → Rebalance
  └── Otherwise → NoOp
```

### Node Scoring
```rust
fn scaling_score(&self) -> f64 {
    self.reputation * 0.4 + (1.0 - self.load_factor) * 0.3 + self.uptime * 0.3
}
```

---

## 5. Federation ZKP Bridge

**File:** `src/bridge/federation_zkp_bridge.rs`

### Purpose
Bridges Async ZKP v5 proofs with federation shards for cross-shard verification, Merkle root synchronization, and consensus tracking.

### Key Types

| Type | Description |
|------|-------------|
| `FederationZKPBridge` | Main bridge struct |
| `FederationZKPConfig` | Bridge configuration |
| `FederationProof` | Proof with source/target shards, votes, Merkle root |
| `ShardBridgeState` | Per-shard state (resources, reputation, load) |
| `FederationVerificationRecord` | Verification history entry |
| `MerkleSyncRecord` | Merkle root sync history |

### Configuration Defaults
```rust
FederationZKPConfig {
    max_shards: 16,
    consensus_threshold: 0.67,
    proof_ttl_ms: 300000,
    routing_strategy: 0,  // 0=capacity, 1=reputation, 2=round_robin
    cross_shard_aggregation: true,
    max_verification_hops: 3,
}
```

### Proof Acceptance
```rust
fn can_accept_proof(&self, cost: f64) -> bool {
    self.available_resources >= cost && self.active_proofs < self.max_active_proofs
}
```

### Cross-Shard Aggregation
```rust
fn needs_cross_shard_aggregation(&self, proof: &FederationProof) -> bool {
    self.config.cross_shard_aggregation && proof.target_shards.len() > 1
}
```
- Returns `true` only for multi-target proofs when aggregation is enabled.

### Routing Strategies
- **Capacity (0):** Select shard with highest `available_resources`.
- **Reputation (1):** Select shard with highest `reputation`.
- **Round-Robin (2):** Cycle through shards in order.

---

## 6. SAE Fine-Tuning v3

**File:** `src/sae/fine_tuning_v3.rs`

### Purpose
Distributed fine-tuning engine with cross-model gradient alignment, adaptive learning rates, and LZ4 checkpoint compression.

### Key Types

| Type | Description |
|------|-------------|
| `FineTuningV3` | Main training engine |
| `FineTuningV3Config` | Training configuration |
| `ModelProfile` | Model with ID, node ID, gradient dimensions |
| `NodeEntry` | Training node with uptime and reputation |
| `Checkpoint` | Compressed model state with LZ4 ratio |
| `TrainingStats` | Training metrics (rounds, loss, checkpoints) |

### Configuration Defaults
```rust
FineTuningV3Config {
    learning_rate: 0.001,
    max_models: 8,
    checkpoint_interval: 10,
    alignment_enabled: true,
    compression_enabled: true,
    min_node_uptime: 0.9,
}
```

### Training Step Flow
```
train_step(gradients)
  ├── select_best_node() — Pick node by uptime + reputation
  ├── compress_gradients() — If compression_enabled
  ├── align_gradients() — If alignment_enabled
  ├── Update model weights
  ├── If total_rounds % checkpoint_interval == 0 → Auto-checkpoint
  └── Update training stats
```

### Adaptive Learning Rate
```rust
fn compute_adaptive_lr(&self, current_norm: f64) -> f64 {
    // Reduce LR if gradient norm spikes, increase if stable
    let factor = self.compute_alignment_factor();
    let new_lr = self.config.learning_rate * (1.0 - factor * 0.1);
    new_lr.clamp(0.0001, 0.1)
}
```

### Checkpoint Compression
- Simulated LZ4 compression with ratio tracking.
- Typical compression ratio: 3.4x for 1024-dim gradients.

---

## 7. Cross-Model Aligner

**File:** `src/sae/cross_model_aligner.rs`

### Purpose
Aligns gradients across heterogeneous model architectures using cosine similarity normalization and dimension-agnostic padding.

### Key Types

| Type | Description |
|------|-------------|
| `CrossModelAligner` | Main aligner struct |
| `AlignerConfig` | Alignment configuration |
| `GradientProfile` | Per-model gradient profile with dimension |
| `AlignmentResult` | Result with aligned gradients and score |

### Configuration Defaults
```rust
AlignerConfig {
    max_models: 16,
    normalization_enabled: true,
    min_alignment_score: 0.5,
}
```

### Alignment Process
```
align(gradients, model_id)
  ├── Find reference profile (highest dimension)
  ├── Pad gradients to reference dimension
  ├── Compute cosine similarity with reference
  ├── Apply normalization factor if enabled
  └── Return AlignmentResult with score
```

### Cosine Similarity
```rust
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    let dot = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum::<f32>() as f64;
    let norm_a = a.iter().map(|x| x * x).sum::<f32>() as f64;
    let norm_b = b.iter().map(|x| x * x).sum::<f32>() as f64;
    dot / (norm_a * norm_b).sqrt()
}
```

---

## 8. Data Flow

### Proof Generation Flow
```
ZKPStatement
  → AsyncZKPV5.submit_statement()
  → AsyncZKPV5.start_batch()
  → AsyncZKPV5.add_to_batch()
  → AsyncZKPV5.precompile_batch()
  → AsyncZKPV5.generate_batch_proofs()
    ├── VRF Sampling (if batch > max_batch_size/2)
    ├── Fallback Check (if complexity > 2× avg)
    ├── Parallel Verification (up to 32 workers)
    └── Incremental Merkle Accumulation
  → ProofBatch with ZKPProof[]
```

### Cross-Shard Verification Flow
```
FederationProof
  → FederationZKPBridge.submit_proof()
    ├── Resource Check (can_accept_proof)
    ├── Register with source shard
    └── Track in pending proofs
  → FederationZKPBridge.submit_vote()
    ├── Record vote per shard
    └── Update proof votes
  → FederationZKPBridge.check_consensus()
    ├── Count yes/no votes
    └── Return true if yes_ratio >= consensus_threshold
  → FederationZKPBridge.complete_verification()
    ├── Move to verified
    ├── Update shard stats
    └── Record in history
```

### Training Flow
```
Gradients
  → FineTuningV3.train_step()
    ├── Node Selection (best uptime + reputation)
    ├── Gradient Compression (LZ4 simulation)
    ├── Gradient Alignment (CrossModelAligner)
    ├── Weight Update
    └── Auto-Checkpoint (if round % interval == 0)
  → TrainingResult
```

---

## 9. Error Handling

All modules use dedicated error enums implementing `std::fmt::Display`:

| Module | Error Type |
|--------|-----------|
| Async ZKP v5 | `ZKPV5Error` |
| Federation Scaling v3 | `ScalingV3Error` |
| Federation ZKP Bridge | `FederationZKPError` |
| SAE Fine-Tuning v3 | `FineTuningV3Error` |
| Cross-Model Aligner | `AlignerError` |

Common error patterns:
- **NotFound:** Resource (pool, shard, node, proof) not found.
- **InsufficientResources:** Not enough credits/resources.
- **CapacityExceeded:** Max capacity reached.
- **InvalidState:** Operation not allowed in current state.

---

## 10. Testing Strategy

### Unit Tests
- Each module has comprehensive inline tests (`#[cfg(test)] mod tests`).
- Total: 150+ unit tests across v1.3 modules.

### E2E Integration Tests
- **File:** `tests/integration/v1_3_sprint3_e2e.rs`
- **Tests:** 9 lifecycle tests covering full module interactions.
- **Coverage:** ZKP v5 lifecycle, fallback/VRF, federation scaling, bridge lifecycle, Merkle sync, cross-module pipeline.

### Stress Tests
- **File:** `tests/load/sprint3_stress_v1_3.rs`
- **Tests:** 13 stress tests with high iteration counts (100-500).
- **Coverage:** 200-round training, 100 checkpoints, 300 ZKP statements, 200 federation proofs, full pipeline.

---

## 11. Dependencies

### Runtime
- No external network dependencies.
- No database requirements.
- Pure Rust computation engine.

### Build
- Rust stable (2021 edition).
- `cargo build --release --features stable`.

### Development
- `cargo test --features stable` for full test suite.
- `cargo clippy --features stable -- -D warnings` for linting.
- `cargo audit` for security scanning.

---

## 12. Security Considerations

- **Zero `unsafe`:** All code is safe Rust.
- **No External I/O:** No network calls, file system access beyond config, or telemetry.
- **Bounded Resources:** All collections have max sizes (max_pools, max_shards, proof_cache_size).
- **Input Validation:** All public methods validate inputs before processing.
- **Time-Based Expiry:** Proofs expire based on `proof_ttl_ms`.

---

## 13. Future Extensions (v1.4.0+)

- **LP-94 Roadmap:** See `docs/v1.4.0_technical_roadmap.md` (pending).
- Potential areas:
  - Hardware acceleration for proof generation.
  - Additional routing strategies for Federation Bridge.
  - Multi-federation interoperability.
  - Enhanced checkpoint diffing for fine-tuning.

---

**Document maintained by ed2kIA Core Team.**
