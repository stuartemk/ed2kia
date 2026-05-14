# Phase 7 Sprint 1 - Task Breakdown

**Sprint**: Phase 7 Sprint 1 (Weeks 1-4)
**Target**: v0.7.0-alpha
**Feature Flag**: `phase7-sprint1`
**Total Story Points**: 55

---

## Epic 1: Continuous Alignment Engine (32 SP)

### Task 1.1: Feedback Collector Module (5 SP)

**File**: `src/alignment/feedback_collector.rs`
**Assignee**: @alignment-lead
**Dependencies**: None

**Acceptance Criteria**:
- [ ] Define `FeedbackSource` enum (HumanCli, CommunityApi, AutoDetection, CrossNet)
- [ ] Define `AlignmentFeedback` struct with all fields
- [ ] Implement `FeedbackCollector::new()` with configurable buffer/flush params
- [ ] Implement `collect()` method with source validation
- [ ] Implement `flush_batch()` with min_batch_size enforcement
- [ ] Unit tests: buffer overflow, source routing, flush timing
- [ ] Clippy clean, no warnings

**Estimate**: 3 days

---

### Task 1.2: Preference Model (RLHF Core) (8 SP)

**File**: `src/alignment/preference_model.rs`
**Assignee**: @ml-team
**Dependencies**: Task 1.1

**Acceptance Criteria**:
- [ ] Define `PreferencePair` struct
- [ ] Implement `PreferenceModel::new()` with learning_rate and beta params
- [ ] Implement `add_pair()` with validation
- [ ] Implement `compute_reward_gradient()` using PPO-style loss
- [ ] Implement `apply_kl_penalty()` with configurable beta
- [ ] Unit tests: gradient magnitude bounds, KL penalty correctness
- [ ] Property test: gradient bounded by learning_rate (proptest)
- [ ] Integration test: Full preference → gradient pipeline
- [ ] Clippy clean, no warnings

**Estimate**: 5 days

---

### Task 1.3: Policy Updater with Safety (8 SP)

**File**: `src/alignment/policy_updater.rs`
**Assignee**: @alignment-lead
**Dependencies**: Task 1.2

**Acceptance Criteria**:
- [ ] Define `PolicyUpdate` struct with rollback_hash
- [ ] Implement `propose_update()` with delta validation
- [ ] Implement `apply_update()` with pre-checkpoint creation
- [ ] Implement `rollback()` restoring from checkpoint
- [ ] Implement `validate_safety()` checking max_update_rate
- [ ] Cooldown enforcement between updates on same layer
- [ ] Checkpoint file format: compressed Tensor bytes + metadata JSON
- [ ] Unit tests: safety bounds, checkpoint/rollback cycle, cooldown
- [ ] Integration test: Apply → Verify → Rollback → Verify restored
- [ ] Clippy clean, no warnings

**Estimate**: 5 days

---

### Task 1.4: Ethics Engine (5 SP)

**File**: `src/alignment/ethics_engine.rs`
**Assignee**: @security-lead
**Dependencies**: None

**Acceptance Criteria**:
- [ ] Define `EthicsViolation` enum (4 variants)
- [ ] Implement `EthicsEngine::new()` with reserved concepts
- [ ] Implement `check_update()` detecting all violation types
- [ ] Implement `add_reserved_concept()`
- [ ] Distributional shift detection (KL divergence threshold)
- [ ] Unit tests: Each violation type triggered and caught
- [ ] Integration test: Ethics engine blocks malicious update
- [ ] Clippy clean, no warnings

**Estimate**: 3 days

---

### Task 1.5: Alignment Module Integration (4 SP)

**File**: `src/alignment/mod.rs`
**Assignee**: @alignment-lead
**Dependencies**: Tasks 1.1-1.4

**Acceptance Criteria**:
- [ ] Create `src/alignment/` module structure
- [ ] Re-export public types in `mod.rs`
- [ ] Feature-gate behind `phase7-sprint1`
- [ ] Wire FeedbackCollector → PreferenceModel → PolicyUpdater pipeline
- [ ] Integration test: End-to-end alignment loop
- [ ] Metrics: `alignment_updates_total`, `alignment_rollbacks_total`, `ethics_violations_total`
- [ ] Clippy clean, no warnings

**Estimate**: 2 days

---

### Task 1.6: SAE Loader Extension (2 SP)

**File**: `src/sae/loader.rs` (extension)
**Assignee**: @sae-lead
**Dependencies**: Task 1.3

**Acceptance Criteria**:
- [ ] Add `apply_delta(&self, delta: &Tensor) -> Result<Tensor>` method
- [ ] Validate delta shape matches layer dimensions
- [ ] Return updated weights (immutable pattern)
- [ ] Unit tests: Shape validation, delta application
- [ ] No breaking changes to existing API
- [ ] Clippy clean, no warnings

**Estimate**: 1 day

---

### Task 1.7: Alignment API Endpoints (4 SP)

**File**: `src/api/routes.rs` (extension)
**Assignee**: @api-lead
**Dependencies**: Task 1.5

**Acceptance Criteria**:
- [ ] `POST /api/v2/alignment/feedback` - Submit alignment feedback
- [ ] `GET /api/v2/alignment/status` - Current alignment state
- [ ] `GET /api/v2/alignment/history` - Update history (paginated)
- [ ] Request/response validation
- [ ] Error handling with proper HTTP status codes
- [ ] OpenAPI spec update
- [ ] Unit tests: Each endpoint with valid/invalid inputs
- [ ] Clippy clean, no warnings

**Estimate**: 2 days

---

## Epic 2: Cross-Net Federation (23 SP)

### Task 2.1: Network Identity & CrossNet Gateway (8 SP)

**File**: `src/federation/crossnet_gateway.rs`
**Assignee**: @fed-lead
**Dependencies**: None

**Acceptance Criteria**:
- [ ] Define `NetworkIdentity` struct
- [ ] Define `CrossNetUpdate` struct with signature field
- [ ] Implement `CrossNetGateway::new()` with local network identity
- [ ] Implement `add_trusted_network()` with key validation
- [ ] Implement Ed25519 signature verification for inbound updates
- [ ] Implement `submit_outbound()` with rate limiting
- [ ] Implement `receive_inbound()` with full validation chain
- [ ] Unit tests: Signature verify, rate limiting, queue management
- [ ] Integration test: Sign → Send → Verify → Accept flow
- [ ] Clippy clean, no warnings

**Estimate**: 5 days

---

### Task 2.2: Meta Aggregator (8 SP)

**File**: `src/federation/meta_aggregator.rs`
**Assignee**: @fed-lead
**Dependencies**: Task 2.1

**Acceptance Criteria**:
- [ ] Define `MetaAggregationResult` struct
- [ ] Implement `MetaAggregator::new()` with config params
- [ ] Implement `aggregate()` with weighted averaging
- [ ] Implement `compute_diversity_score()` using Shannon entropy
- [ ] Implement `update_network_weight()` for dynamic trust
- [ ] Diversity bonus injection (configurable α)
- [ ] Unit tests: Weighted avg correctness, diversity score ∈ [0,1]
- [ ] Property test: Diversity score bounded (proptest)
- [ ] Integration test: Multi-network aggregation with diversity
- [ ] Clippy clean, no warnings

**Estimate**: 5 days

---

### Task 2.3: Sync Protocol Extension (4 SP)

**File**: `src/federation/sync_protocol.rs` (extension)
**Assignee**: @p2p-team
**Dependencies**: Task 2.1

**Acceptance Criteria**:
- [ ] Add `CrossNetUpdate` variant to `SyncPayload` enum
- [ ] Add `handle_crossnet_update()` message handler
- [ ] Route cross-network updates to CrossNetGateway
- [ ] Backward compatible with existing protocol
- [ ] Unit tests: Message routing, backward compatibility
- [ ] No breaking changes to existing API
- [ ] Clippy clean, no warnings

**Estimate**: 2 days

---

### Task 2.4: Cross-Net API Endpoints (3 SP)

**File**: `src/api/routes.rs` (extension)
**Assignee**: @api-lead
**Dependencies**: Task 2.2

**Acceptance Criteria**:
- [ ] `GET /api/v2/federation/crossnet/networks` - List trusted networks
- [ ] `POST /api/v2/federation/crossnet/update` - Submit cross-network update
- [ ] `GET /api/v2/federation/crossnet/history` - Aggregation history
- [ ] Request validation and error handling
- [ ] OpenAPI spec update
- [ ] Unit tests: Each endpoint
- [ ] Clippy clean, no warnings

**Estimate**: 2 days

---

## Epic 3: Infrastructure & Tooling (0 SP - Enablers)

### Task 3.1: Feature Gate Configuration (1 SP)

**File**: `Cargo.toml`, `src/phase6/mod.rs`
**Assignee**: @dev-lead
**Dependencies**: None

**Acceptance Criteria**:
- [ ] Add `phase7-sprint1` feature to `Cargo.toml`
- [ ] Create `src/phase7/mod.rs` with feature-gated re-exports
- [ ] Update `enabled_features()` to detect Phase 7
- [ ] CI matrix includes `--features phase7-sprint1`
- [ ] Clippy clean with new feature flag

**Estimate**: 0.5 days

---

### Task 3.2: Monitoring Metrics (2 SP)

**File**: `src/monitoring/metrics.rs` (extension)
**Assignee**: @devops-lead
**Dependencies**: Tasks 1.5, 2.2

**Acceptance Criteria**:
- [ ] Alignment metrics: `alignment_updates_total`, `alignment_rollbacks_total`, `ethics_violations_total`, `alignment_learning_rate_gauge`
- [ ] Cross-Net metrics: `crossnet_updates_in_total`, `crossnet_updates_out_total`, `crossnet_diversity_score_gauge`, `crossnet_trusted_networks_gauge`
- [ ] Prometheus registration
- [ ] Grafana dashboard panel additions (update existing dashboard JSON)
- [ ] Unit tests: Metric increment/validation

**Estimate**: 1.5 days

---

### Task 3.3: Configuration Schema (1 SP)

**File**: `launch/genesis/config.toml` (extension)
**Assignee**: @dev-lead
**Dependencies**: Tasks 1.5, 2.2

**Acceptance Criteria**:
- [ ] Add `[alignment]` section to config schema
- [ ] Add `[crossnet]` section to config schema
- [ ] Default values documented
- [ ] Config loading in main.rs
- [ ] Validation on startup

**Estimate**: 0.5 days

---

## Sprint Timeline

```
Week 1 (Days 1-5):
├── Task 1.1: Feedback Collector (3 days)
├── Task 1.4: Ethics Engine (3 days)
├── Task 2.1: CrossNet Gateway (start, 5 days)
└── Task 3.1: Feature Gate (0.5 days)

Week 2 (Days 6-10):
├── Task 1.2: Preference Model (5 days)
├── Task 2.1: CrossNet Gateway (complete)
└── Task 2.3: Sync Protocol Extension (2 days)

Week 3 (Days 11-15):
├── Task 1.3: Policy Updater (5 days)
├── Task 2.2: Meta Aggregator (start, 5 days)
└── Task 1.6: SAE Loader Extension (1 day)

Week 4 (Days 16-20):
├── Task 1.5: Alignment Integration (2 days)
├── Task 1.7: Alignment API (2 days)
├── Task 2.2: Meta Aggregator (complete)
├── Task 2.4: Cross-Net API (2 days)
├── Task 3.2: Monitoring Metrics (1.5 days)
├── Task 3.3: Config Schema (0.5 days)
└── Integration testing + polish (2 days)
```

## Definition of Done

Per task:
- [ ] Code implemented per acceptance criteria
- [ ] Unit tests passing (100% coverage on new code)
- [ ] Integration tests passing (if applicable)
- [ ] Property tests passing (if applicable)
- [ ] Clippy clean, zero warnings
- [ ] Documentation comments on all public items
- [ ] Feature-gated correctly
- [ ] Code review approved by 1 core team member
- [ ] CI pipeline green

Per sprint:
- [ ] All tasks completed or formally deferred with reason
- [ ] End-to-end integration tests passing
- [ ] Feature flag `phase7-sprint1` builds and tests clean
- [ ] Monitoring metrics visible in Grafana
- [ ] Sprint retro completed
- [ ] v0.7.0-alpha release notes drafted

---

*Task Breakdown v1.0 | Phase 7 Sprint 1 | 2026-05-04*
