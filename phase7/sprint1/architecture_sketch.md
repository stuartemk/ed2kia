# Phase 7 Sprint 1 - Architecture Sketch

## Sprint Focus: Continuous Alignment Engine + Cross-Net Federation

**Duration**: 4 weeks (Weeks 1-4 of Phase 7)
**Target Release**: v0.7.0-alpha
**Feature Flag**: `phase7-sprint1`

---

## 1. Continuous Alignment Engine

### 1.1 Problem Statement

Currently, ed2kIA SAE models are trained offline and deployed as static weights. When the semantic landscape evolves (new concepts, shifting meanings), the model drifts from human intent. We need a continuous alignment loop that:

1. Collects human feedback in real-time
2. Computes gradient updates safely (RLHF-style)
3. Applies updates incrementally without disrupting live inference
4. Maintains an audit trail of all alignment changes

### 1.2 Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                   Continuous Alignment Engine               │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │  Feedback    │    │  Preference  │    │  Policy      │  │
│  │  Collector   │───▶│  Model       │───▶│  Updater     │  │
│  │  (real-time) │    │  (RLHF)      │    │  (safe apply)│  │
│  └──────────────┘    └──────────────┘    └──────────────┘  │
│        │                    │                    │          │
│        ▼                    ▼                    ▼          │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │  Audit       │    │  Reward      │    │  Rollback    │  │
│  │  Ledger      │    │  Shaping     │    │  Guardrails  │  │
│  │  (redb)      │    │  (ethics)    │    │  (safety)    │  │
│  └──────────────┘    └──────────────┘    └──────────────┘  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
         │                    │                    │
         ▼                    ▼                    ▼
   FeedbackStore        EthicsEngine         SAE Loader/Registry
   (existing)           (new)                (existing, extended)
```

### 1.3 Module Design

#### 1.3.1 `src/alignment/feedback_collector.rs`

**Purpose**: Aggregate feedback from multiple sources into a unified stream.

```rust
pub enum FeedbackSource {
    HumanCli,        // Existing feedback_cli.rs
    CommunityApi,    // New API endpoint /api/v2/alignment/feedback
    AutoDetection,   // Anomaly-based auto-flagging
    CrossNet,        // Feedback from federated peers
}

pub struct AlignmentFeedback {
    pub id: String,
    pub source: FeedbackSource,
    pub layer_id: u32,
    pub feature_index: u32,
    pub current_value: f32,
    pub desired_value: f32,  // Human-corrected target
    pub confidence: f32,     // Source confidence weight
    pub timestamp: u64,
    pub metadata: HashMap<String, String>,
}

pub struct FeedbackCollector {
    sources: Vec<FeedbackSource>,
    buffer: VecDeque<AlignmentFeedback>,
    flush_interval: Duration,
    min_batch_size: usize,
}
```

**Key Methods**:
- `collect(source, feedback) -> Result<()>` - Ingest feedback
- `flush_batch() -> Result<Vec<AlignmentFeedback>>` - Emit batch for training
- `get_pending_count() -> usize` - Monitor backlog

#### 1.3.2 `src/alignment/preference_model.rs`

**Purpose**: Learn human preferences from paired comparisons (chosen/rejected).

```rust
pub struct PreferencePair {
    pub context: String,           // Input context
    pub chosen_output: Vec<f32>,   // Human-preferred activations
    pub rejected_output: Vec<f32>, // Model's original output
    pub reward_signal: f32,        // Computed reward delta
}

pub struct PreferenceModel {
    pairs: Vec<PreferencePair>,
    learning_rate: f64,
    beta: f64,  // KL penalty coefficient (RLHF standard)
    max_pairs: usize,
}
```

**Key Methods**:
- `add_pair(pair: PreferencePair) -> Result<()>`
- `compute_reward_gradient() -> Result<Tensor>` - PPO-style gradient
- `apply_kl_penalty(current: &Tensor, reference: &Tensor) -> f64`

**Algorithm**: Implement simplified RLHF loss:
```
L(θ) = -E[log π_θ(chosen|context)] + β * KL(π_θ || π_ref)
```

#### 1.3.3 `src/alignment/policy_updater.rs`

**Purpose**: Apply alignment updates to SAE weights safely.

```rust
pub struct PolicyUpdate {
    pub update_id: String,
    pub layer_id: u32,
    pub weight_delta: Tensor,       // Gradient update
    pub expected_improvement: f64,  // Predicted reward gain
    pub rollback_hash: String,      // Pre-update checkpoint
    pub created_at: u64,
}

pub struct PolicyUpdater {
    max_update_rate: f64,          // Max weight change per step
    cooldown_period: Duration,      // Min time between updates
    checkpoint_dir: PathBuf,
    pending_updates: Vec<PolicyUpdate>,
}
```

**Key Methods**:
- `propose_update(delta: Tensor, layer_id: u32) -> Result<PolicyUpdate>`
- `apply_update(update: &PolicyUpdate) -> Result<()>` - With pre-checkpoint
- `rollback(update: &PolicyUpdate) -> Result<()>` - Restore checkpoint
- `validate_safety(update: &PolicyUpdate) -> bool` - Check bounds

**Safety Constraints**:
- Max 5% weight change per update (configurable)
- Min 1 hour cooldown between updates on same layer
- Automatic rollback if reward decreases >10% post-update
- All updates logged to audit ledger

#### 1.3.4 `src/alignment/ethics_engine.rs`

**Purpose**: Ethical guardrails for alignment updates.

```rust
pub enum EthicsViolation {
    ConceptTampering(String),    // Attempting to modify reserved concept
    DistributionalShift(f64),    // Update causes >threshold shift
    AdversarialPattern,          // Update matches known attack pattern
    QuorumViolation,             // Insufficient human consensus
}

pub struct EthicsEngine {
    reserved_concepts: HashSet<String>,
    max_distributional_shift: f64,
    min_quorum_size: usize,
    violation_log: Vec<EthicsViolation>,
}
```

**Key Methods**:
- `check_update(update: &PolicyUpdate) -> Result<(), EthicsViolation>`
- `add_reserved_concept(concept: String)`
- `get_violations() -> Vec<EthicsViolation>`

### 1.4 Integration Points

| Existing Module | Integration | Changes Required |
|----------------|-------------|------------------|
| `src/human/feedback_cli.rs` | Feed into FeedbackCollector | Add `AlignmentFeedback` adapter |
| `src/human/concept_updater.rs` | Shared reserved concepts | EthicsEngine reads from ConceptUpdater |
| `src/rlhf/feedback_store.rs` | Storage backend | Extend schema for alignment data |
| `src/sae/loader.rs` | Weight updates | Add `apply_delta()` method |
| `src/api/routes.rs` | New endpoints | `/api/v2/alignment/*` routes |
| `src/monitoring/metrics.rs` | Telemetry | New alignment metrics |

---

## 2. Cross-Net Federation

### 2.1 Problem Statement

Current federation (Phase 6) operates within a single ed2kIA network. Cross-Net Federation enables multiple independent ed2kIA networks to exchange SAE updates, creating a meta-federation layer for:
- Cross-domain knowledge transfer
- Diversity-preserving aggregation
- Resilient multi-network consensus

### 2.2 Architecture Overview

```
┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│   Network A      │     │   Network B      │     │   Network C      │
│  (ed2kIA main)   │     │  (ed2kIA health) │     │  (ed2kIA finance)│
│                  │     │                  │     │                  │
│  ┌────────────┐  │     │  ┌────────────┐  │     │  ┌────────────┐  │
│  │ CrossNet   │  │     │  │ CrossNet   │  │     │  │ CrossNet   │  │
│  │ Gateway    │  │     │  │ Gateway    │  │     │  │ Gateway    │  │
│  │ (new)      │  │     │  │ (new)      │  │     │  │ (new)      │  │
│  └─────┬──────┘  │     │  └─────┬──────┘  │     │  └─────┬──────┘  │
│        │         │     │        │         │     │        │         │
│  ┌─────▼──────┐  │     │  ┌─────▼──────┐  │     │  ┌─────▼──────┐  │
│  │ FedAvg     │  │     │  │ FedAvg     │  │     │  │ FedAvg     │  │
│  │ (existing) │  │     │  │ (existing) │  │     │  │ (existing) │  │
│  └────────────┘  │     │  └────────────┘  │     │  └────────────┘  │
└──────────────────┘     └──────────────────┘     └──────────────────┘
         │                        │                        │
         └───────────┬────────────┴────────────────────────┘
                     │
              ┌──────▼──────┐
              │  Meta-Fed   │
              │  Aggregator │
              │  (new)      │
              └─────────────┘
```

### 2.3 Module Design

#### 2.3.1 `src/federation/crossnet_gateway.rs`

**Purpose**: Gateway for cross-network weight exchange.

```rust
pub struct NetworkIdentity {
    pub network_id: String,        // e.g., "ed2k-main", "ed2k-health"
    pub genesis_hash: String,      // Network genesis block hash
    pub public_key: String,        // Network-level Ed25519 key
    pub domain_tags: Vec<String>,  // Semantic domain tags
}

pub struct CrossNetUpdate {
    pub source_network: NetworkIdentity,
    pub layer_id: u32,
    pub weight_delta: Vec<f32>,
    pub local_round: u64,
    pub participant_count: usize,
    pub confidence: f32,
    pub signature: String,         // Network-level signature
}

pub struct CrossNetGateway {
    local_network: NetworkIdentity,
    trusted_networks: HashMap<String, NetworkIdentity>,
    inbound_queue: VecDeque<CrossNetUpdate>,
    outbound_queue: VecDeque<CrossNetUpdate>,
    exchange_rate: f64,            // Throttle: updates per hour
}
```

**Key Methods**:
- `add_trusted_network(identity: NetworkIdentity) -> Result<()>`
- `submit_outbound(update: CrossNetUpdate) -> Result<()>`
- `receive_inbound(update: CrossNetUpdate) -> Result<()>` - With signature verification
- `get_trusted_networks() -> Vec<NetworkIdentity>`

#### 2.3.2 `src/federation/meta_aggregator.rs`

**Purpose**: Aggregate updates from multiple networks with diversity preservation.

```rust
pub struct MetaAggregationResult {
    pub meta_round: u64,
    pub participating_networks: Vec<String>,
    pub aggregated_delta: Vec<f32>,
    pub diversity_score: f64,      // How diverse the inputs were
    pub consensus_level: f64,      // Cross-network agreement
    pub timestamp: u64,
}

pub struct MetaAggregator {
    min_networks: usize,           // Min networks for valid aggregation
    diversity_weight: f64,         // Weight for diversity in scoring
    network_weights: HashMap<String, f64>,  // Per-network trust weights
    history: Vec<MetaAggregationResult>,
}
```

**Key Methods**:
- `aggregate(round_updates: Vec<CrossNetUpdate>) -> Result<MetaAggregationResult>`
- `compute_diversity_score(updates: &[CrossNetUpdate]) -> f64`
- `update_network_weight(network_id: &str, new_weight: f64)`

**Diversity-Preserving Aggregation**:
```
weighted_avg = Σ(w_i * delta_i) / Σ(w_i)
diversity_bonus = α * entropy([w_1, w_2, ...])
final_update = weighted_avg + diversity_bonus * normalize(random_direction)
```

Where `α` controls diversity injection strength.

### 2.4 Integration Points

| Existing Module | Integration | Changes Required |
|----------------|-------------|------------------|
| `src/federation/avg_aggregator.rs` | Feed into CrossNetGateway | Export local aggregation results |
| `src/federation/sync_protocol.rs` | Cross-network messaging | Extend protocol for inter-network |
| `src/p2p/swarm.rs` | Multi-network discovery | Network identity in peer info |
| `src/security/memory_guard.rs` | Validate inbound updates | Same safety checks |
| `src/api/routes.rs` | New endpoints | `/api/v2/federation/crossnet/*` |

---

## 3. Data Flow Diagram

```
Human Feedback → FeedbackCollector → PreferenceModel → PolicyUpdater → SAE Weights
      │                    │                  │                │
      ▼                    ▼                  ▼                ▼
  Audit Ledger     Preference Pairs    Ethics Check     Checkpoint/Rollback
                     (redb)            (ethics_engine)   (file system)

Local FedAvg → CrossNetGateway → MetaAggregator → PolicyUpdater → SAE Weights
      │                │                  │                │
      ▼                ▼                  ▼                ▼
  Network A      Signature Verify    Diversity Score   Same as above
  Network B      (ed25519)           (entropy calc)
  Network C
```

---

## 4. Configuration

### 4.1 Alignment Engine Config

```toml
[alignment]
enabled = true
learning_rate = 0.001
kl_beta = 0.1
max_update_rate = 0.05          # 5% max weight change
cooldown_seconds = 3600         # 1 hour between updates
min_quorum_size = 3             # Min human votes per update
checkpoint_dir = "data/alignment_checkpoints/"
reserved_concepts = ["system_prompt", "safety_guard", "ethics_core"]
```

### 4.2 Cross-Net Federation Config

```toml
[crossnet]
enabled = true
network_id = "ed2k-main"
exchange_rate = 10              # Max 10 updates/hour per network
min_networks_for_aggregation = 2
diversity_weight = 0.1          # α parameter
trusted_networks = [
  { id = "ed2k-health", key = "ed25519_pub_...", domain = ["healthcare"] },
  { id = "ed2k-finance", key = "ed25519_pub_...", domain = ["finance"] },
]
```

---

## 5. Testing Strategy

### 5.1 Unit Tests
- FeedbackCollector: Buffer management, source routing
- PreferenceModel: Gradient computation, KL penalty
- PolicyUpdater: Safety bounds, checkpoint/rollback
- EthicsEngine: Violation detection, reserved concepts
- CrossNetGateway: Signature verification, queue management
- MetaAggregator: Diversity scoring, weighted aggregation

### 5.2 Integration Tests
- Full alignment loop: Feedback → Update → Apply → Verify
- Cross-network exchange: Sign → Send → Verify → Aggregate
- Safety: Ethics violations block updates
- Rollback: Failed update restores checkpoint

### 5.3 Property Tests (proptest)
- PreferenceModel: Gradient magnitude bounded by learning_rate
- MetaAggregator: Diversity score ∈ [0, 1]
- PolicyUpdater: Weight change ≤ max_update_rate

---

## 6. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Adversarial feedback poisoning | Model corruption | Quorum requirement + ethics engine |
| Cross-network Sybil attack | Fake networks pollute aggregation | Network-level key registration + governance approval |
| Alignment oscillation | Unstable weights | Cooldown period + reward trend monitoring |
| Diversity bonus instability | Noisy updates | Cap α parameter + smoothing window |
| Checkpoint storage growth | Disk exhaustion | LRU eviction policy + compression |

---

*Architecture Sketch v1.0 | Phase 7 Sprint 1 | 2026-05-04*
