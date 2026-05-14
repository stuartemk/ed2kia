# Phase 6 Sprint 1 - Task Breakdown

## Module: interoperability/

### T-601: Model Adapter Trait + Llama-3 Implementation

**File**: `src/interoperability/adapter.rs`
**Estimate**: 3 days
**Depends on**: None

```rust
// Required declarations
pub trait ModelAdapter {
    fn name(&self) -> &str;
    fn hidden_dim(&self) -> usize;
    fn extract_hidden_states(&self, input: &Tensor) -> Result<Tensor>;
    fn map_to_sae_input(&self, hidden: &Tensor) -> Result<Tensor>;
}

pub struct Llama3Adapter {
    model_path: PathBuf,
    hidden_dim: usize,
    num_layers: usize,
}

impl ModelAdapter for Llama3Adapter { /* ... */ }
```

**Acceptance Criteria**:
- [ ] `ModelAdapter` trait defined with 4 methods
- [ ] `Llama3Adapter` implements trait
- [ ] Hidden state extraction works for layer -2
- [ ] Output tensor compatible with SAE input format
- [ ] Tests: extract states from sample input

---

### T-602: ONNX Model Import

**File**: `src/interoperability/adapter.rs` (extended)
**Estimate**: 2 days
**Depends on**: T-601

```rust
pub struct OnnxAdapter {
    model_path: PathBuf,
    output_shape: Vec<usize>,
}

impl OnnxAdapter {
    pub fn from_onnx(path: &Path) -> Result<Self>;
    pub fn run_inference(&self, input: &Tensor) -> Result<Tensor>;
}
```

**Acceptance Criteria**:
- [ ] Load .onnx models via `ort` runtime
- [ ] Validate output shape matches expected
- [ ] Fallback to safetensors if ONNX fails
- [ ] Tests: load sample ONNX model

---

### T-603: Feature Schema Mapping

**File**: `src/interoperability/schema.rs`
**Estimate**: 2 days
**Depends on**: T-601

```rust
pub struct SchemaMapper {
    source_schema: FeatureSchema,
    target_schema: FeatureSchema,
    mapping: HashMap<u32, u32>,
}

impl SchemaMapper {
    pub fn new(source: FeatureSchema, target: FeatureSchema) -> Self;
    pub fn map_features(&self, features: &[SparseFeature]) -> Vec<SparseFeature>;
    pub fn auto_detect_mapping(&mut self) -> Result<usize>; // Returns num mappings found
}
```

**Acceptance Criteria**:
- [ ] Map features by semantic similarity (cosine > 0.85)
- [ ] Handle dimension mismatches (padding/truncation)
- [ ] Unmapped features flagged for human review
- [ ] Tests: map between two known schemas

---

## Module: federation/

### T-604: FedAvg with Krum Filtering

**File**: `src/federation/avg_aggregator.rs`
**Estimate**: 4 days
**Depends on**: None

```rust
pub struct FedAvgAggregator {
    config: FedAvgConfig,
    pending_updates: HashMap<u32, Vec<WeightUpdate>>,
}

pub struct FedAvgConfig {
    min_participants: usize,
    krum_select: usize,        // Number of updates to select
    max_byzantine: usize,      // Expected Byzantine nodes (f)
    learning_rate: f32,
}

impl FedAvgAggregator {
    pub fn receive_update(&mut self, update: WeightUpdate) -> Result<()>;
    pub fn aggregate_layer(&self, layer_id: u32) -> Result<AggregatedWeights>;
    fn krum_filter(&self, updates: &[WeightUpdate]) -> Vec<usize>; // Returns selected indices
    fn weighted_average(&self, selected: &[WeightUpdate]) -> Vec<f32>;
}
```

**Acceptance Criteria**:
- [ ] Receive weight updates from n nodes
- [ ] Krum filtering: select n-f-2 closest updates
- [ ] Weighted average of selected updates
- [ ] Reject if participants < min_participants
- [ ] Tests: correct aggregation with 0, 1, 2 Byzantine nodes

---

### T-605: Round Synchronization Protocol

**File**: `src/federation/sync_protocol.rs`
**Estimate**: 3 days
**Depends on**: T-604

```rust
pub struct SyncProtocol {
    current_round: u64,
    round_duration: Duration,
    participants: HashSet<PeerId>,
    coordinator: PeerId,
}

impl SyncProtocol {
    pub fn start_round(&mut self, round_id: u64) -> RoundSignal;
    pub fn register_participant(&mut self, peer_id: PeerId) -> Result<()>;
    pub fn check_round_complete(&self) -> bool;
    pub fn get_round_status(&self) -> RoundStatus;
}
```

**Acceptance Criteria**:
- [ ] Coordinator broadcasts round start
- [ ] Nodes register participation
- [ ] Round completes when all participants submit or timeout
- [ ] Late submissions rejected
- [ ] Tests: full round lifecycle

---

### T-606: Byzantine Tolerance Verification

**File**: `tests/federation_test.rs`
**Estimate**: 2 days
**Depends on**: T-604

**Test Cases**:
- [ ] Honest majority (n=10, f=0): correct aggregation
- [ ] Single Byzantine (n=10, f=1): Krum filters outlier
- [ ] Max Byzantine (n=10, f=3): still correct (f < n/3)
- [ ] Exceed Byzantine (n=10, f=4): detection and rejection
- [ ] Empty updates: handled gracefully
- [ ] Malformed updates: rejected with error

---

## Module: staking/

### T-607: Resource Commitment Registry

**File**: `src/staking/registry.rs`
**Estimate**: 2 days
**Depends on**: None

```rust
pub struct ResourceRegistry {
    commitments: HashMap<String, ResourceCommitment>,
    config: RegistryConfig,
}

pub struct ResourceCommitment {
    node_id: String,
    public_key: String,
    cpu_cores: usize,
    ram_gb: usize,
    gpu: Option<GpuSpec>,
    bandwidth_mbps: usize,
    signed_at: Instant,
    signature: Vec<u8>, // Ed25519
}

impl ResourceRegistry {
    pub fn register(&mut self, commitment: ResourceCommitment) -> Result<()>;
    pub fn verify_signature(&self, node_id: &str) -> Result<bool>;
    pub fn get_active_nodes(&self) -> Vec<&ResourceCommitment>;
    pub fn calculate_capacity(&self) -> TotalCapacity;
}
```

**Acceptance Criteria**:
- [ ] Register node with signed commitment
- [ ] Verify Ed25519 signature matches public key
- [ ] Reject duplicate registrations
- [ ] Calculate total network capacity
- [ ] Tests: register, verify, reject duplicates

---

### T-608: Resource Utilization Proof

**File**: `src/staking/proof.rs`
**Estimate**: 2 days
**Depends on**: T-607

```rust
pub struct ResourceProof {
    node_id: String,
    timestamp: Instant,
    cpu_usage: f64,
    memory_usage: f64,
    layers_processed: usize,
    signature: Vec<u8>,
}

impl ResourceProof {
    pub fn generate(node_id: &str, signing_key: &SigningKey) -> Result<Self>;
    pub fn verify(&self, public_key: &VerifyingKey) -> Result<bool>;
    pub fn is_fresh(&self, max_age: Duration) -> bool;
}
```

**Acceptance Criteria**:
- [ ] Generate proof with current resource metrics
- [ ] Sign proof with node's Ed25519 key
- [ ] Verify proof signature and freshness
- [ ] Reject expired proofs (> 5 min old)
- [ ] Tests: generate, verify, expire

---

## Module: api/

### T-609: OpenAPI 3.0 Specification

**File**: `src/api/openapi.rs`
**Estimate**: 2 days
**Depends on**: None

**Endpoints to Document**:
| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v2/status` | Network status and health |
| GET | `/api/v2/network` | Peer list, leases, consensus |
| POST | `/api/v2/feedback` | Submit human feedback |
| GET | `/api/v2/metrics` | Prometheus-format metrics |
| GET | `/api/v2/models` | Registered models (interoperability) |
| POST | `/api/v2/models/import` | Import external model |
| GET | `/api/v2/federation/rounds` | Federation round status |
| GET | `/api/v2/staking/nodes` | Staking registry |

**Acceptance Criteria**:
- [ ] Complete OpenAPI 3.0 JSON spec
- [ ] All endpoints documented with request/response schemas
- [ ] Example values for all fields
- [ ] Exportable as YAML and JSON
- [ ] Validated against OpenAPI schema

---

## Dependencies Graph

```
T-601 (adapter trait)
  ├── T-602 (ONNX import)
  └── T-603 (schema mapper)

T-604 (FedAvg + Krum)
  ├── T-605 (sync protocol)
  └── T-606 (Byzantine tests)

T-607 (staking registry)
  └── T-608 (resource proofs)

T-609 (OpenAPI spec)  -- independent
```

## Parallelization Strategy

| Track | Tasks | Can Start | Duration |
|-------|-------|-----------|----------|
| Interoperability | T-601 → T-602, T-603 | Day 1 | 7 days |
| Federation | T-604 → T-605, T-606 | Day 1 | 9 days |
| Staking | T-607 → T-608 | Day 1 | 4 days |
| API | T-609 | Day 1 | 2 days |

**Critical Path**: Federation track (9 days)
