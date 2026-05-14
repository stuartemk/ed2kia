# Phase 7 Sprint 1 - Integration Hooks

**Sprint:** Phase 7 Sprint 1 - Continuous Alignment Engine + Cross-Net Federation Bridge  
**Version:** `0.7.0-alpha.1`  
**Feature Flag:** `phase7-sprint1`  
**Date:** 2026-05-04  
**Status:** ✅ Completed  
**Purpose:** Document connection points between Phase 7 modules and existing v0.5.0/v0.6.0 codebase

---

## 📋 Overview

This document describes all integration points between the new Phase 7 modules (`alignment/engine.rs`, `federation/bridge.rs`) and the existing ed2kIA codebase (v0.5.0/v0.6.0). Every hook is designed for **zero-breaking changes** to existing modules, using feature-gated conditionals and adapter patterns.

---

## 🔗 Integration Matrix

| Phase 7 Module | Existing Module | Integration Type | Direction | Data Flow |
|----------------|-----------------|------------------|-----------|-----------|
| `alignment/engine.rs` | `rlhf/feedback_store.rs` | Data Adapter | ← Read | `FeedbackEntry` → `AlignmentFeedback` |
| `alignment/engine.rs` | `interpret/feature_analyzer.rs` | Data Consumer | ← Read | `SparseFeature` → drift calculation |
| `alignment/engine.rs` | `human/concept_updater.rs` | Action Trigger | → Write | `SteeringDelta` → `ConceptUpdate` |
| `federation/bridge.rs` | `federation/sync_protocol.rs` | Protocol Extension | ← Read | `SyncMessage` → `DeltaUpdate` |
| `federation/bridge.rs` | `federation/avg_aggregator.rs` | Aggregation Consumer | → Write | `DeltaUpdate` → `WeightUpdate` |
| `federation/bridge.rs` | `consensus/validator.rs` | Trust Signal | ← Read | `ConsensusEvent` → trust record |
| `federation/bridge.rs` | `p2p/swarm.rs` | Network Transport | ↔ Bidirectional | P2P messages ↔ bridge handshakes |
| `phase7/mod.rs` | `phase6/mod.rs` | Feature Isolation | None | Compile-time separation |

---

## 🔌 Hook 1: RLHF Feedback Store → Alignment Engine

### Purpose
Convert human feedback entries from the RLHF feedback store into alignment feedback for drift calculation.

### Source Module
- **File:** `src/rlhf/feedback_store.rs`
- **Type:** `FeedbackEntry`
- **Fields:** `layer_id`, `feature_idx`, `feature_value`, `decision`, `correction`, `annotator_confidence`

### Target Module
- **File:** `src/alignment/engine.rs`
- **Type:** `AlignmentFeedback`
- **Fields:** `layer_id`, `feature_idx`, `current_activation`, `desired_value`, `annotator_confidence`, `concept`

### Adapter Function (Proposed)
```rust
#[cfg(feature = "phase7-sprint1")]
fn feedback_entry_to_alignment_feedback(entry: &FeedbackEntry) -> Option<AlignmentFeedback> {
    // Only convert entries with corrections (desired values)
    let desired_value = entry.correction?;
    
    Some(AlignmentFeedback {
        layer_id: entry.layer_id.clone(),
        feature_idx: entry.feature_idx,
        current_activation: entry.feature_value,
        desired_value: desired_value,
        annotator_confidence: entry.annotator_confidence,
        concept: entry.concept.clone(),
    })
}
```

### Integration Point
- **Location:** `src/alignment/engine.rs` (new adapter module)
- **Trigger:** Periodic feedback ingestion (e.g., every 100 new entries)
- **Error Handling:** Skip entries without corrections (no alignment signal)

### Validation
- **Test:** `test_feedback_entry_conversion` (verify field mapping)
- **Edge Case:** Entry with `correction = None` → returns `None`

---

## 🔌 Hook 2: Feature Analyzer → Alignment Engine

### Purpose
Provide SAE feature activations to the alignment engine for drift detection.

### Source Module
- **File:** `src/interpret/feature_analyzer.rs`
- **Type:** `SparseFeature`
- **Fields:** `index`, `activation`, `layer_id`

### Target Module
- **File:** `src/alignment/engine.rs`
- **Method:** `AlignmentScorer::calculate_drift()`
- **Input:** `activations: &[f32]` (extracted from `SparseFeature` array)

### Integration Pattern
```rust
#[cfg(feature = "phase7-sprint1")]
fn extract_activations_for_layer(features: &[SparseFeature], layer_id: u32) -> Vec<f32> {
    features
        .iter()
        .filter(|f| f.layer_id == layer_id)
        .map(|f| f.activation)
        .collect()
}
```

### Integration Point
- **Location:** `src/alignment/engine.rs` (input preprocessing)
- **Trigger:** On-demand drift calculation (API call or periodic check)
- **Error Handling:** Return `AlignmentError::EmptyActivations` if no features for layer

### Validation
- **Test:** `test_extract_activations_empty_layer` (verify error handling)
- **Test:** `test_extract_activations_valid_layer` (verify filtering)

---

## 🔌 Hook 3: Alignment Engine → Concept Updater

### Purpose
Apply steering deltas as concept updates when drift exceeds critical threshold.

### Source Module
- **File:** `src/alignment/engine.rs`
- **Type:** `AlignmentResult`
- **Fields:** `steering_delta`, `flagged_concepts`, `drift_score`

### Target Module
- **File:** `src/human/concept_updater.rs`
- **Type:** `ConceptUpdate`
- **Fields:** `concept`, `layer_id`, `feature_idx`, `new_value`, `confidence`

### Integration Pattern
```rust
#[cfg(feature = "phase7-sprint1")]
fn steering_to_concept_update(result: &AlignmentResult) -> Vec<ConceptUpdate> {
    result.flagged_concepts.iter().map(|concept| {
        ConceptUpdate {
            concept: concept.clone(),
            layer_id: result.layer_id,  // Assuming layer_id in result
            feature_idx: 0,  // To be determined from delta index
            new_value: result.steering_delta[0],  // Delta application
            confidence: result.confidence,
        }
    }).collect()
}
```

### Integration Point
- **Location:** `src/human/concept_updater.rs` (new import + adapter)
- **Trigger:** When `drift_score > critical_threshold`
- **Error Handling:** Log warning if concept update fails (non-critical)

### Validation
- **Test:** `test_steering_to_concept_update` (verify mapping)
- **Edge Case:** Empty `flagged_concepts` → empty update list

---

## 🔌 Hook 4: Sync Protocol → Federation Bridge

### Purpose
Translate P2P sync messages into federation bridge delta updates.

### Source Module
- **File:** `src/federation/sync_protocol.rs`
- **Type:** `SyncMessage`
- **Fields:** `msg_type`, `sender_id`, `round`, `payload`

### Target Module
- **File:** `src/federation/bridge.rs`
- **Type:** `DeltaUpdate`
- **Fields:** `source_network`, `source_node`, `layer_id`, `weights`, `delta_hash`, `local_round`, `participant_count`, `confidence`, `source_schema`, `timestamp_ms`

### Integration Pattern
```rust
#[cfg(feature = "phase7-sprint1")]
fn sync_message_to_delta_update(message: &SyncMessage) -> Option<DeltaUpdate> {
    match &message.payload {
        SyncPayload::WeightUpdate(update) => {
            let delta_hash = compute_delta_hash(&message.sender_id, update.layer_id, &update.weights);
            Some(DeltaUpdate {
                source_network: update.source_network.clone(),
                source_node: message.sender_id.clone(),
                layer_id: update.layer_id,
                weights: update.weights.clone(),
                delta_hash,
                local_round: message.round,
                participant_count: update.participant_count,
                confidence: update.confidence,
                source_schema: update.source_schema.clone(),
                timestamp_ms: current_timestamp_ms(),
            })
        }
        _ => None,  // Only WeightUpdate payloads convert to DeltaUpdate
    }
}
```

### Integration Point
- **Location:** `src/federation/bridge.rs` (message translation layer)
- **Trigger:** On incoming `SyncMessage::WeightUpdate`
- **Error Handling:** Skip non-weight-update messages (log warning)

### Validation
- **Test:** `test_sync_message_to_delta_update` (verify field mapping)
- **Test:** `test_sync_message_non_weight_update` (verify None return)

---

## 🔌 Hook 5: Federation Bridge → FedAvg Aggregator

### Purpose
Submit merged delta updates to the FedAvg aggregator for local aggregation.

### Source Module
- **File:** `src/federation/bridge.rs`
- **Type:** `DeltaUpdate`
- **Fields:** `weights`, `layer_id`, `source_network`, `confidence`

### Target Module
- **File:** `src/federation/avg_aggregator.rs`
- **Type:** `WeightUpdate`
- **Fields:** `node_id`, `layer_id`, `weights`, `delta_hash`, `participant_count`, `confidence`

### Integration Pattern
```rust
#[cfg(feature = "phase7-sprint1")]
fn delta_to_weight_update(delta: &DeltaUpdate) -> WeightUpdate {
    WeightUpdate {
        node_id: format!("{}:{}", delta.source_network, delta.source_node),
        layer_id: delta.layer_id,
        weights: delta.weights.clone(),
        delta_hash: delta.delta_hash.clone(),
        participant_count: delta.participant_count,
        confidence: delta.confidence,
    }
}
```

### Integration Point
- **Location:** `src/federation/avg_aggregator.rs` (new import + adapter)
- **Trigger:** After `FederationBridge::merge_updates()` completes
- **Error Handling:** Skip if aggregator rejects (log error, increment failed_syncs)

### Validation
- **Test:** `test_delta_to_weight_update` (verify field mapping)
- **Edge Case:** Empty weights → aggregator rejection

---

## 🔌 Hook 6: Consensus Validator → Federation Bridge Trust

### Purpose
Update trust records based on consensus validation results.

### Source Module
- **File:** `src/consensus/validator.rs`
- **Type:** `ConsensusEvent`
- **Fields:** `batch_id`, `signal_type`, `node_id`, `timestamp_ms`

### Target Module
- **File:** `src/federation/bridge.rs`
- **Type:** `TrustRecord`
- **Fields:** `network_id`, `trust_score`, `successful_syncs`, `failed_syncs`

### Integration Pattern
```rust
#[cfg(feature = "phase7-sprint1")]
fn consensus_event_to_trust_update(event: &ConsensusEvent) -> TrustUpdate {
    match event.signal_type {
        SignalType::Approve => TrustUpdate::Success(event.node_id.clone()),
        SignalType::Reject => TrustUpdate::Failure(event.node_id.clone()),
        SignalType::Pending => TrustUpdate::None,
    }
}
```

### Integration Point
- **Location:** `src/federation/bridge.rs` (trust record update)
- **Trigger:** On consensus event (approve/reject)
- **Error Handling:** Skip if node not in trusted networks (log info)

### Validation
- **Test:** `test_consensus_approve_increases_trust` (verify trust increase)
- **Test:** `test_consensus_reject_decreases_trust` (verify trust decrease)

---

## 🔌 Hook 7: P2P Swarm → Federation Bridge Transport

### Purpose
Use existing P2P swarm for federation bridge message transport.

### Source Module
- **File:** `src/p2p/swarm.rs`
- **Type:** `Swarm` (libp2p-based P2P network)
- **Methods:** `send_message()`, `receive_message()`

### Target Module
- **File:** `src/federation/bridge.rs`
- **Type:** `HandshakeMessage`, `DeltaUpdate`
- **Transport:** Serialized JSON/binary over P2P

### Integration Pattern
```rust
#[cfg(feature = "phase7-sprint1")]
async fn send_handshake_via_p2p(
    swarm: &mut Swarm,
    peer_id: PeerId,
    handshake: HandshakeMessage,
) -> Result<(), P2PError> {
    let payload = serde_json::to_vec(&handshake)?;
    swarm.send_message(peer_id, payload).await?;
    Ok(())
}
```

### Integration Point
- **Location:** `src/p2p/swarm.rs` (new feature-gated method)
- **Trigger:** `FederationBridge::init_handshake()` calls P2P send
- **Error Handling:** P2P errors → `BridgeError::HandshakeFailed`

### Validation
- **Test:** `test_send_handshake_via_p2p` (mock swarm, verify serialization)
- **Edge Case:** Peer not connected → P2P error

---

## 🔌 Hook 8: Phase 6 → Phase 7 Feature Isolation

### Purpose
Ensure compile-time isolation between Phase 6 and Phase 7 modules.

### Source Module
- **File:** `src/phase6/mod.rs`
- **Feature:** `phase6-experimental`

### Target Module
- **File:** `src/phase7/mod.rs`
- **Feature:** `phase7-sprint1`

### Integration Pattern
```rust
// src/phase6/mod.rs - NO changes required
#[cfg(feature = "phase6-experimental")]
pub mod federation {
    pub use crate::federation::avg_aggregator::{/* ... */};
}

// src/phase7/mod.rs - NEW module
#[cfg(feature = "phase7-sprint1")]
pub mod federation {
    pub use crate::federation::bridge::{/* ... */};
}
```

### Integration Point
- **Location:** `Cargo.toml` (feature definitions)
- **Trigger:** Compile-time feature selection
- **Error Handling:** Cannot enable both `phase6-experimental` and `phase7-sprint1` for same module path (namespace collision)

### Validation
- **Test:** `cargo check --features "phase6-experimental"` (Phase 6 compiles)
- **Test:** `cargo check --features "phase7-sprint1"` (Phase 7 compiles)
- **Test:** `cargo check --features "phase6-experimental,phase7-sprint1"` (both compile, no collision)

---

## 🛡️ Safety Guarantees

### Zero Breaking Changes
- All Phase 7 modules use `#[cfg(feature = "phase7-sprint1")]` for compile-time isolation
- No modifications to existing v0.5.0/v0.6.0 modules
- New modules are additive (no deletions or renames)

### Backward Compatibility
- Phase 6 modules continue to function without Phase 7 enabled
- Feature flag allows gradual rollout (canary → staging → production)
- Data adapters handle missing fields gracefully (Option types)

### Error Isolation
- Phase 7 errors do not propagate to Phase 6 modules
- Adapter functions return `Option<T>` or `Result<T, AdapterError>`
- Failed adaptations log warnings but don't crash the system

---

## 🧪 Integration Testing Strategy

### Unit Tests (Per Module)
- **Alignment Engine:** 20 tests (existing)
- **Federation Bridge:** 22 tests (existing)
- **Phase 7 Module:** 5 tests (existing)

### Integration Tests (New)
- **Test 1:** Feedback Store → Alignment Engine data flow
- **Test 2:** Feature Analyzer → Drift Calculation pipeline
- **Test 3:** Steering Delta → Concept Update application
- **Test 4:** Sync Protocol → Federation Bridge message translation
- **Test 5:** Federation Bridge → FedAvg Aggregator submission
- **Test 6:** Consensus Validator → Trust Record updates
- **Test 7:** P2P Swarm → Federation Bridge transport

### End-to-End Tests (Sprint 2)
- **E2E-1:** Full feedback loop (human feedback → drift detection → steering adjustment)
- **E2E-2:** Cross-network federation (handshake → delta sync → merge → aggregate)
- **E2E-3:** Trust routing simulation (multiple networks, varying trust levels)

---

## 📊 Data Flow Diagrams

### Alignment Engine Data Flow
```
[Human Feedback] → FeedbackStore → AlignmentFeedback → AlignmentScorer
                                                          │
                                                          ▼
                                                    Drift Calculation
                                                          │
                                                          ▼
                                                Steering Delta Generation
                                                          │
                                                          ▼
                                              ConceptUpdater (semantic map)
```

### Federation Bridge Data Flow
```
[P2P Swarm] ←→ SyncProtocol ←→ SyncMessage ←→ DeltaUpdate
                                                        │
                                                        ▼
                                              FederationBridge Handshake
                                                        │
                                                        ▼
                                              Trust Score Calculation
                                                        │
                                                        ▼
                                              FedAvg Aggregator ←→ WeightUpdate
```

---

## 🚀 Deployment Considerations

### Feature Flag Rollout
1. **Canary:** Enable `phase7-sprint1` on 10% of nodes
2. **Staging:** Enable on 50% of nodes + monitor drift/trust metrics
3. **Production:** Enable on 100% of nodes after validation

### Monitoring Hooks
- **Drift Metrics:** Expose `drift_score` via `/api/v2/alignment/drift`
- **Trust Metrics:** Expose `trust_avg` via `/api/v2/federation/trust`
- **Error Rates:** Track `AlignmentError` and `BridgeError` occurrences

### Rollback Plan
- Disable `phase7-sprint1` feature flag
- Revert to Phase 6 modules (no data loss, adapters are stateless)
- Log all Phase 7 actions for audit/replay

---

## 📚 References

- **Phase 7 Sprint 1 Progress:** [`phase7/sprint1/progress.md`](./progress.md)
- **Phase 6 Architecture:** [`phase6/architecture_v2.md`](../../phase6/architecture_v2.md)
- **Phase 6 Integration Hooks:** [`phase6/sprint1/integration_hooks.md`](../../phase6/sprint1/integration_hooks.md)
- **RLHF Feedback Store:** [`src/rlhf/feedback_store.rs`](../../src/rlhf/feedback_store.rs)
- **Federation Sync Protocol:** [`src/federation/sync_protocol.rs`](../../src/federation/sync_protocol.rs)
- **Consensus Validator:** [`src/consensus/validator.rs`](../../src/consensus/validator.rs)

---

*Last Updated: 2026-05-04T10:40:00Z*  
*Author: Roo (Senior Rust Engineer)*  
*Review Status: Draft*
