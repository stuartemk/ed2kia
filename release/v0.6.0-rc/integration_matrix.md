# Integration Matrix: v0.5.0 ↔ Phase 6 (v0.6.0-RC)

> **Status**: Release Candidate Validation
> **Date**: 2026-05-04
> **Scope**: Cross-module dependency mapping, feature gates, fallback paths, and friction points.

---

## 1. Module Dependency Graph

```
                    ┌─────────────────┐
                    │   api/routes.rs  │ ← /api/v2/* endpoints
                    │   api/auth.rs    │ ← Ed25519 signature validation
                    │   api/openapi.rs │ ← OpenAPI 3.0 spec generation
                    └────────┬────────┘
                             │ depends on
              ┌──────────────┼──────────────┐
              ▼              ▼               ▼
    ┌───────────────┐ ┌──────────────┐ ┌──────────────┐
    │ staking/      │ │ federation/  │ │ interoperability/
    │ registry.rs   │ │ avg_agg.rs   │ │ onnx_adapter.rs
    │ proof.rs      │ │ sync_proto.rs│ │ adapter.rs
    └───────┬───────┘ └──────┬───────┘ └──────┬───────┘
            │                │                 │
            ▼                ▼                 ▼
    ┌───────────────┐ ┌──────────────┐ ┌──────────────┐
    │ reputation/   │ │ consensus/   │ │ sae/loader.rs
    │ ledger.rs     │ │ merkle.rs    │ │ schema.rs
    │ scoring.rs    │ │ validator.rs │ └──────────────┘
    └───────┬───────┘ └──────┬───────┘
            │                │
            ▼                ▼
    ┌───────────────┐ ┌──────────────┐
    │ governance/   │ │ zkp/         │
    │ proposal.rs   │ │ circuit.rs   │
    │ voting.rs     │ │ verifier.rs  │
    └───────────────┘ └──────────────┘
```

---

## 2. Cross-Module Integration Table

| Phase 6 Module | v0.5.0 Counterpart | Integration Type | Feature Gate | Fallback |
|---|---|---|---|---|
| `staking/registry.rs` | `reputation/ledger.rs` | Direct (stake → reputation) | `phase6-core` | Reputation-only mode |
| `staking/proof.rs` | `consensus/validator.rs` | Verification pipeline | `phase6-core` | Merkle-only validation |
| `federation/avg_aggregator.rs` | `bridge/consciousness.rs` | Weight aggregation | `phase6-core` | Local-only inference |
| `federation/sync_protocol.rs` | `p2p/swarm.rs` | P2P delta sync | `phase6-core` | Gossipsub v0.5 |
| `interoperability/onnx_adapter.rs` | `sae/loader.rs` | Model conversion | `phase6-sprint2` | Placeholder tensors |
| `interoperability/adapter.rs` | `interpret/feature_analyzer.rs` | Tensor normalization | `phase6-core` | Raw activations |
| `interoperability/schema.rs` | `sae/loader.rs` | Schema validation | `phase6-core` | Skip validation |
| `api/routes.rs` | `web/routes.rs` | API v2 endpoints | `phase6-sprint2` | API v1 only |
| `api/auth.rs` | N/A (new) | Ed25519 auth | `phase6-sprint2` | No auth (dev mode) |
| `api/openapi.rs` | N/A (new) | Spec generation | `phase6-sprint2` | Manual docs |
| `zkp/circuit.rs` | `consensus/merkle.rs` | ZKP commitments | `experimental` | Merkle proofs |
| `zkp/verifier.rs` | `consensus/validator.rs` | Proof verification | `experimental` | Structural check |

---

## 3. Feature Gate Matrix

| Feature Flag | Modules Enabled | Test Coverage | Production Ready |
|---|---|---|---|
| `core-only` | v0.5.0 modules only | 100% | ✅ STABLE |
| `phase6-core` | staking, federation, interoperability (base) | 170 tests, 0 warnings | ✅ RC |
| `phase6-sprint2` | API v2, auth, OpenAPI, ONNX adapter | 170 tests, 0 warnings | ✅ RC |
| `phase6-experimental` | ZKP circuits, advanced verification | Included in 170 | ⚠️ Canary only |
| `experimental` | All experimental features | Included in 170 | ❌ Research only |

### Activation Precedence
```
core-only (default)
  ↓ +phase6-core
phase6-core + phase6-sprint2 (recommended for RC)
  ↓ +phase6-experimental
full phase6 stack
  ↓ +experimental
research mode
```

---

## 4. Data Flow Paths

### 4.1 Staking → Reputation Flow
```
Node registers (registry.rs::register())
  → ResourceCommitment created
  → Heartbeat processed (registry.rs::process_heartbeat())
  → Proof verified (registry.rs::verify_proof())
  → Reputation updated (reputation/scoring.rs)
  → Ledger entry created (reputation/ledger.rs)
```

### 4.2 Federation Sync Flow
```
Peer submits WeightUpdate (sync_protocol.rs)
  → Hash validated (avg_aggregator.rs::add_update())
  → Krum filter applied (avg_aggregator.rs::apply_krum_filter())
  → FedAvg aggregation (avg_aggregator.rs::aggregate())
  → AggregationResult broadcast via P2P
  → Local model updated (sae/loader.rs)
```

### 4.3 API v2 Request Flow
```
HTTP Request → /api/v2/*
  → auth_middleware() validates Ed25519 signature
  → Route handler processes request
  → Business logic (federation/staking/governance)
  → ApiResponse<T> serialized to JSON
  → OpenAPI spec available at /api/v2/openapi
```

### 4.4 ONNX Conversion Flow
```
.onnx file path provided
  → onnx_adapter.rs::load_model()
  → Cache check (avoids redundant loads)
  → candle_core::Tensor created
  → adapter.rs::normalize_dtype()
  → schema.rs::validate()
  → NormalizedHiddenState ready for SAE
```

---

## 5. Friction Points & Mitigations

### F1: Staking Registry ↔ Reputation Ledger
- **Risk**: Double-counting reputation if both modules update independently
- **Mitigation**: Registry is source of truth; reputation reads from registry stats
- **Validation**: `test_registry_stats()` confirms correct counts

### F2: Federation Sync ↔ P2P Swarm
- **Risk**: Message ordering issues during high churn
- **Mitigation**: Round-based ordering (FederationRound.round_id); stale rounds discarded
- **Validation**: `test_protocol_full_flow()` verifies round lifecycle

### F3: ONNX Adapter ↔ SAE Loader
- **Risk**: Tensor shape mismatches between models
- **Mitigation**: `adapter.rs::reshape_to_qwen()` handles dimension changes; `schema.rs` validates
- **Validation**: `test_reshape_to_qwen_expand()` and `test_validate_schema_ok()`

### F4: API Auth ↔ Key Management
- **Risk**: Key rotation without downtime
- **Mitigation**: In-memory cache with `add_authorized_key()` / `clear_cache()`; hot-reload ready
- **Validation**: `test_add_authorized_key()` and `test_clear_cache()`

### F5: ZKP Verification ↔ Consensus Validator
- **Risk**: ZKP failures blocking consensus
- **Mitigation**: 3-tier fallback: ZKP → Merkle → VRF; structural integrity always checked
- **Validation**: `test_zkp_verification()` and `test_vrf_verification()`

---

## 6. Configuration Compatibility

| Config Key | v0.5.0 | v0.6.0-RC | Migration |
|---|---|---|---|
| `features` | `core-only` (default) | `phase6-sprint2` recommended | Add to `Cargo.toml` |
| `max_heartbeat_age` | N/A | 86400s (24h) | New config |
| `slash_threshold` | N/A | 3 missed heartbeats | New config |
| `min_participants` | N/A | 3 for federation | New config |
| `require_signature` | N/A | false (default) | New config |
| `signature_timeout` | N/A | 300s | New config |

---

## 7. Rollback Boundaries

| Scenario | Rollback Action | Data Preserved |
|---|---|---|
| API v2 instability | Disable `phase6-sprint2` feature | Yes (API v1 unaffected) |
| Federation sync failures | Disable `phase6-core` | Yes (P2P gossipsub continues) |
| Staking registry corruption | Unregister affected nodes | Yes (reputation ledger intact) |
| ONNX conversion errors | Fall back to placeholder tensors | Yes (SAE loader unaffected) |
| ZKP verification failures | Fall back to Merkle-only | Yes (consensus continues) |

---

## 8. Validation Checklist

- [x] All 170 unit tests pass with `--features "phase6-sprint2"`
- [x] Zero clippy warnings with `-D warnings`
- [x] Feature gates properly isolate Phase 6 from v0.5.0
- [x] Fallback paths tested for each friction point
- [x] Configuration defaults documented
- [x] Rollback procedures defined per module

---

*Generated for ed2kIA v0.6.0-RC preparation. Last updated: 2026-05-04.*
