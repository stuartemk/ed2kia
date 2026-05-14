# Phase 7 Sprint 2 – Architecture Document v2

**Version:** v0.7.0-alpha.2  
**Feature Flag:** `phase7-sprint2`  
**Date:** 2026-05-04  

---

## 1. System Overview

Phase 7 Sprint 2 extends the Continuous Alignment architecture with three critical modules that close the feedback loop, enable dynamic trust management, and provide versioned schema interoperability.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      ed2kIA Phase 7 Sprint 2                           │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐     │
│  │  Feedback Loop   │  │   Trust Scoring  │  │  Schema Registry │     │
│  │                  │  │                  │  │                  │     │
│  │  ingest()        │  │  update_score()  │  │  register()      │     │
│  │  compute_drift() │  │  detect_sybil()  │  │  validate()      │     │
│  │  apply_steering()│  │  propagate_      │  │  get_compatible()│     │
│  │  rollback_       │  │    _cross_net()  │  │  deprecate()     │     │
│  │    if_degraded() │  │  decay()         │  │                  │     │
│  └──────┬───────────┘  └──────┬───────────┘  └──────┬───────────┘     │
│         │                     │                      │                 │
│         │                     │                      │                 │
│         ▼                     ▼                      ▼                 │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐     │
│  │  AlignmentScorer │  │FederationBridge  │  │  TensorAdapter   │     │
│  │  (Sprint 1)      │  │  (Sprint 1)      │  │  (Phase 6)       │     │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘     │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Module Architecture

### 2.1 Alignment Feedback Loop

```
┌──────────────────────────────────────────────────────────────┐
│              AlignmentFeedbackLoop                           │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────┐    ┌──────────────┐    ┌───────────────┐   │
│  │  Feedback   │───>│   Drift      │───>│   Steering    │   │
│  │  Queue      │    │   Compute    │    │   Apply       │   │
│  │  (FIFO)     │    │              │    │               │   │
│  └─────────────┘    └──────────────┘    └───────┬───────┘   │
│         │                    │                        │      │
│         │                    │                    ┌─────┴────┐│
│         │                    │                    │Rollback? ││
│         │                    │                    └─────┬────┘│
│         │                    │                         │      │
│         ▼                    ▼                         ▼      │
│  ┌─────────────┐    ┌──────────────┐    ┌───────────────┐   │
│  │Rate Limiter │    │AlignmentScorer│    │  Audit Log    │   │
│  │(100/window) │    │  (Sprint 1)  │    │  (append-only)│   │
│  └─────────────┘    └──────────────┘    └───────────────┘   │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

**Data Flow:**
1. `AlignmentFeedback` enters FIFO queue via `ingest()`
2. Rate limiter checks: max 100 entries per 30s window
3. `compute_drift()` processes queue → feeds `AlignmentScorer`
4. `apply_steering()` generates delta, simulates application
5. If drift degrades > threshold → automatic `rollback_if_degraded()`
6. All actions logged to audit trail

**Key Config:**
```rust
FeedbackLoopConfig {
    alignment_config: AlignmentConfig,  // From Sprint 1
    feedback_window_ms: 30_000,         // 30s window
    max_queue_size: 1000,               // FIFO capacity
    rate_limit: 100,                    // Entries per window
    rollback_threshold: 0.1,            // 10% degradation trigger
    max_retries: 3,                     // Re-application attempts
}
```

---

### 2.2 Dynamic Trust Scorer

```
┌──────────────────────────────────────────────────────────────┐
│                  DynamicTrustScorer                          │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              Trust Formula                           │   │
│  │  trust = base × (1 - decay^days) × consensus × zkp  │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   Node       │  │    Sybil     │  │   Cross-     │     │
│  │   Records    │  │   Detection  │  │   Network    │     │
│  │              │  │              │  │   Propagate  │     │
│  │  - trust     │  │  - ASN group │  │              │     │
│  │  - status    │  │  - IP group  │  │  radius ≤ 5  │     │
│  │  - asn/ip    │  │  - crypto sig│  │              │     │
│  │  - syncs     │  │              │  │              │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              Node Status Transitions                 │   │
│  │                                                      │   │
│  │  Active (≥0.6) ←→ Degraded (0.3-0.6) → Banned (<0.3)│   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

**Trust Calculation:**
```
trust = base_score × (1 - decay_factor^days_since_activity) 
              × consensus_weight × zkp_multiplier
```

**Sybil Detection:**
1. Group nodes by ASN → if count > threshold (3) → suspicious
2. Group nodes by IP hash → if count > threshold (3) → suspicious
3. Cross-reference with cryptographic signatures
4. Generate `SybilCluster` with suspicion score

**Key Config:**
```rust
TrustConfig {
    decay_factor: 0.995,           // 0.5% per cycle
    decay_cycle_ms: 60_000,        // 60s cycle
    ban_threshold: 0.3,            // Ban below 0.3
    degraded_threshold: 0.6,       // Degrade below 0.6
    sybil_threshold: 3,            // >3 nodes = suspicious
    max_propagation_radius: 5,     // Max propagation depth
    consensus_weight: 1.0,         // Consensus multiplier
    zkp_multiplier: 1.0,           // ZKP multiplier
}
```

---

### 2.3 Schema Registry

```
┌──────────────────────────────────────────────────────────────┐
│                    SchemaRegistry                            │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              Schema Definition                       │   │
│  │                                                      │   │
│  │  version: "1.0.0"                                    │   │
│  │  name: "sae_hidden_state"                            │   │
│  │  checksum: SHA-256                                   │   │
│  │  dimensions: [4096]                                  │   │
│  │  dtype: "f32"                                        │   │
│  │  backward_compat: Some("0.9.0")                      │   │
│  │  forward_compat: ["1.1.0", "1.2.0"]                  │   │
│  │  deprecated: false                                   │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   SemVer     │  │  Compat      │  │   Deprecation│     │
│  │   Validation │  │   Matrix     │  │   + Migration│     │
│  │              │  │              │  │              │     │
│  │  major.minor│  │  Backward    │  │  90-day      │     │
│  │  .patch     │  │  Forward     │  │  retention   │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

**Compatibility Rules:**
- **Backward:** New version accepts old data (dimension expansion OK)
- **Forward:** Old version can read new data structure
- **Breaking:** Dimension shrinking = breaking change (rejected unless `--allow-breaking`)

**Key Config:**
```rust
SchemaRegistryConfig {
    max_schemas: 100,
    require_backward_compat: true,
    deprecation_retention_days: 90,
}
```

---

## 3. Integration Architecture

### 3.1 E2E Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                        E2E Pipeline Flow                           │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  1. Feedback Ingestion                                              │
│     ┌──────────────┐    ┌──────────────────┐                       │
│     │ Human/System │───>│ AlignmentFeedback│                       │
│     │   Feedback   │    │     Loop         │                       │
│     └──────────────┘    └──────┬───────────┘                       │
│                                │                                    │
│  2. Drift Computation          ▼                                    │
│                    ┌──────────────────────┐                        │
│                    │  compute_drift()     │                        │
│                    │  → AlignmentScorer   │                        │
│                    └──────────┬───────────┘                        │
│                               │                                    │
│  3. Steering Application      ▼                                    │
│                    ┌──────────────────────┐                        │
│                    │  apply_steering()    │                        │
│                    │  → rollback if bad   │                        │
│                    └──────────┬───────────┘                        │
│                               │                                    │
│  4. Federation Sync           ▼                                    │
│                    ┌──────────────────────┐                        │
│                    │ FederationBridge     │                        │
│                    │  → process_delta()   │                        │
│                    └──────────┬───────────┘                        │
│                               │                                    │
│  5. Trust Scoring             ▼                                    │
│                    ┌──────────────────────┐                        │
│                    │ DynamicTrustScorer   │                        │
│                    │  → update_score()    │                        │
│                    │  → detect_sybil()    │                        │
│                    └──────────┬───────────┘                        │
│                               │                                    │
│  6. Schema Validation         ▼                                    │
│                    ┌──────────────────────┐                        │
│                    │  SchemaRegistry      │                        │
│                    │  → validate()        │                        │
│                    │  → get_compatible()  │                        │
│                    └──────────────────────┘                        │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 3.2 Feature Flag Isolation

```
phase7-sprint1 (v0.7.0-alpha.1)
├── alignment/engine.rs
├── alignment/tests.rs
├── federation/bridge.rs
└── federation/tests.rs

phase7-sprint2 (v0.7.0-alpha.2)
├── alignment/feedback_loop.rs
├── federation/trust_scoring.rs
├── interoperability/schema_registry.rs
└── tests/integration/phase7_e2e.rs
```

**Compile-Time Isolation:**
```rust
#[cfg(feature = "phase7-sprint1")]
pub mod alignment { /* Sprint 1 */ }

#[cfg(feature = "phase7-sprint2")]
pub mod sprint2 { /* Sprint 2 */ }
```

---

## 4. Error Handling Strategy

### 4.1 Error Types

| Module | Error Type | Key Variants |
|--------|-----------|--------------|
| Feedback Loop | `FeedbackLoopError` | ValidationFailed, RateLimitExceeded, RollbackTriggered |
| Trust Scoring | `TrustScoringError` | NodeNotFound, SybilDetected, NodeBanned |
| Schema Registry | `SchemaRegistryError` | SchemaNotFound, BackwardCompatibilityBroken, InvalidSemanticVersion |

### 4.2 Error Propagation

```
FeedbackLoopError
  └── AlignmentError (from Sprint 1)

TrustScoringError
  └── Independent (no upstream deps)

SchemaRegistryError
  └── Independent (no upstream deps)
```

---

## 5. Security Considerations

### 5.1 Sybil Resistance
- ASN-based clustering detection
- IP hash clustering detection
- Cryptographic signature verification
- Configurable thresholds

### 5.2 Schema Integrity
- SHA-256 checksums for all schemas
- Semantic versioning enforcement
- Breaking change protection
- Deprecation with migration paths

### 5.3 Audit Trail
- Append-only audit log in Feedback Loop
- Complete action tracking (ingest, drift, steering, rollback)
- Timestamped entries with action types and results

---

## 6. Performance Characteristics

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| Feedback ingest | O(1) | FIFO push with rate check |
| Drift compute | O(n) | n = pending feedback entries |
| Steering apply | O(1) | Simulated (production: O(weights)) |
| Trust update | O(1) | HashMap lookup |
| Sybil detect | O(n) | n = total nodes |
| Schema register | O(1) | HashMap insert |
| Schema validate | O(k) | k = compatible versions |

---

## 7. Testing Strategy

### 7.1 Unit Tests (52 total)
- `feedback_loop.rs`: 15 tests
- `trust_scoring.rs`: 18 tests
- `schema_registry.rs`: 19 tests

### 7.2 E2E Tests (15 total)
- Cross-module integration flows
- Complete pipeline validation
- Edge cases (rollback, Sybil, breaking changes)

### 7.3 Coverage Target
- 90%+ on Sprint 2 modules
- 100% on error paths
- 100% on security-critical paths (Sybil, schema validation)