# Phase 8 Sprint 2 - Architecture v2

## System Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                    ed2kIA v0.8.0-alpha.2                            │
│                    Phase 8 Sprint 2 Architecture                    │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐  │
│  │  CrossModelScaler│  │ContinuousAlign-  │  │   SLAEnforcer    │  │
│  │                  │  │    mentLoop      │  │                  │  │
│  │  • route_request │──│  • ingest_feedbk │──│  • evaluate_slos │  │
│  │  • balance_load  │──│  • compute_drift │──│  • trigger_degr  │  │
│  │  • fallback_to_  │──│  • human_review  │──│  • exec_rollback │  │
│  │    capacity      │  │  • apply_steering│  │  • notify_ops    │  │
│  │  • validate_     │  │                  │  │                  │  │
│  │    compatibility │  │                  │  │                  │  │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘  │
│           │                       │                       │        │
│           ▼                       ▼                       ▼        │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │              Phase 8 Sprint 1 Foundation                      │  │
│  ├──────────────┬──────────────────────┬───────────────────────┤  │
│  │  Resource    │    UI Backend        │    SLO Engine         │  │
│  │  Marketplace │    (Axum + SSE)      │    (Metric Tracking)  │  │
│  │              │                      │                       │  │
│  │  • list_res  │  • alignment_stream  │  • track_metric       │  │
│  │  • match_req │  • federation_status │  • evaluate_slo       │  │
│  │  • settle    │  • metrics_realtime  │  • enforce_sla        │  │
│  │  • anti_game │  • ws_events         │  • degradation        │  │
│  └──────────────┴──────────────────────┴───────────────────────┘  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## Module Dependencies

### CrossModelScaler Dependencies
```
cross_model.rs
    │
    ├─► NodeCapacity (struct) - Node metadata
    ├─► RoutingRequest (struct) - Request parameters
    ├─► ScaleResult (struct) - Routing result
    ├─► BTreeMap<String, NodeCapacity> - Node registry
    ├─► VecDeque<ScaleResult> - Routing history
    └─► Schema compatibility checker (semver)
```

### ContinuousAlignmentLoop Dependencies
```
continuous.rs
    │
    ├─► ContinuousFeedback (struct) - Feedback entry
    ├─► AlignmentLoopResult (struct) - Cycle result
    ├─► LoopConfig (struct) - Configuration
    ├─► HashMap<u32, VecDeque<ContinuousFeedback>> - Feedback buffer
    ├─► HashMap<u32, VecDeque<f32>> - Drift history
    ├─► VecDeque<String> - Audit trail
    └─► SHA-256 (sha2 crate) - Audit hashing
```

### SLAEnforcer Dependencies
```
enforcer.rs
    │
    ├─► SloStatusRecord (struct) - SLO tracking
    ├─► EnforcementResult (struct) - Action result
    ├─► EnforcerConfig (struct) - Configuration
    ├─► DegradationLevel (enum) - 4 levels
    ├─► OpsNotification (struct) - Notification payload
    ├─► HashMap<String, SloStatusRecord> - SLO registry
    └─► VecDeque<EnforcementResult> - History
```

## Data Flow Diagrams

### Request Routing Flow
```
Client Request
    │
    ▼
CrossModelScaler.route_request()
    │
    ├─► Validate schema compatibility
    │
    ├─► Filter active nodes
    │
    ├─► Check Sybil resistance (reputation >= 0.2)
    │
    ├─► Calculate load factors
    │
    ├─► Select best node (lowest latency × load_factor)
    │
    ├─► Check if load_factor > threshold
    │       │
    │       ├─ YES ──► fallback_to_capacity()
    │       │             │
    │       │             ├─► Try any active node
    │       │             │
    │       │             └─► Core-only mode if none
    │       │
    │       └─ NO ──► Route to selected node
    │
    ▼
ScaleResult { routed_to, load_factor, fallback_triggered, latency_ms }
```

### Alignment Loop Flow
```
Feedback Entry
    │
    ▼
ContinuousAlignmentLoop.ingest_feedback()
    │
    ├─► Validate feedback (NaN, Inf, confidence range)
    │
    ├─► Check rate limit
    │
    └─► Store in feedback buffer
            │
            ▼
    compute_drift(layer_id)
            │
            ├─► Calculate mean |current - desired|
            │
            └─► Store in drift history
                    │
                    ▼
    request_human_review()?
                    │
            ├─ drift > threshold AND confidence < 0.8
            │       │
            │       ├─ YES ──► Queue for human review
            │       │             │
            │       │             └─► applied = false
            │       │
            │       └─ NO ──► apply_steering()
            │                     │
            │                     ├─► Calculate adjustment
            │                     │
            │                     └─► Generate audit hash
            │
            ▼
    AlignmentLoopResult { applied, drift_score, human_review_required, audit_hash }
```

### SLO Enforcement Flow
```
Metric Report
    │
    ▼
SLAEnforcer.report_slo_value()
    │
    ├─► Update SLO record
    │
    └─► Increment breach counter if value > target
            │
            ▼
    evaluate_slos()
            │
            ├─► Check breach windows
            │
            ├─► Determine degradation level
            │       │
            │       ├─ 0 breaches ──► Normal
            │       ├─ 1-2 breaches ──► Level 1 (Warning)
            │       ├─ 3-4 breaches ──► Level 2 (Reduce Peers)
            │       ├─ 5-6 breaches ──► Level 3 (Core-Only)
            │       └─ 7+ breaches ──► Level 4 (Rollback)
            │
            ▼
    trigger_degradation()?
            │
            ├─► Execute action based on level
            │
            ├─► notify_ops()
            │
            └─► Level 4? ──► execute_rollback()
                                │
                                └─► Reset SLO records
```

## Integration Points

### With Phase 7 Modules
| Phase 7 Module | Integration Point | Data Flow |
|---------------|------------------|-----------|
| `AlignmentScorer` | Drift computation | `compute_drift()` uses scorer results |
| `FeedbackStore` | Feedback persistence | `ingest_feedback()` stores to redb |
| `ConsciousnessBridge` | Steering signals | `apply_steering()` generates bridge signals |
| `DynamicTrustScorer` | Reputation sync | Node reputation ↔ trust score |

### With Phase 6 Modules
| Phase 6 Module | Integration Point | Data Flow |
|---------------|------------------|-----------|
| `PeerManager` | Node capacity | Peer metrics → NodeCapacity |
| `ResourceRegistry` | Staking validation | Active nodes → routing candidates |
| `SchemaRegistry` | Compatibility | Schema versions → compatibility check |

### With External Systems
| System | Integration | Protocol |
|-------|------------|----------|
| Prometheus | Metrics export | HTTP /metrics |
| Grafana | Dashboards | JSON API |
| Operations Team | Notifications | Structured queue |
| Human Reviewers | Alignment review | Pending queue |

## Configuration Reference

### CrossModelScaler Defaults
```rust
load_threshold: 0.8        // Fallback when load_factor > 0.8
min_reputation: 0.2        // Exclude nodes with reputation < 0.2
compatible_schemas: []     // Auto-detected from nodes
routing_history_size: 256  // Max history entries
```

### ContinuousAlignmentLoop Defaults
```rust
drift_threshold: 0.3       // Human review when drift > 0.3
min_confidence: 0.8        // AND confidence < 0.8
feedback_buffer_size: 64   // Per-layer buffer
audit_capacity: 256        // Max audit entries
max_rate_per_second: 100   // Rate limit
```

### SLAEnforcer Defaults
```rust
warning_threshold: 0.8     // Level 1 trigger
critical_threshold: 0.95   // Level 2-3 trigger
rollback_threshold: 0.99   // Level 4 trigger
max_degradation_level: 4   // Max level
breach_window_size: 5      // Windows before action
```

## Security Considerations

### Sybil Resistance
- Nodes with reputation < 0.2 are excluded from routing
- Reputation synchronized with Phase 7 DynamicTrustScorer
- ASN/IP-based cluster detection prevents fake node proliferation

### Audit Trail
- SHA-256 hashed audit entries for alignment actions
- Configurable capacity (256-512 entries)
- Immutable enforcement history for SLO actions

### Safe Fallback
- Progressive degradation prevents sudden failures
- Core-only mode as last resort
- Automatic rollback when thresholds exceeded

## Performance Targets

| Metric | Target | Measurement |
|-------|--------|-------------|
| Route request latency | < 1ms | `route_request()` benchmark |
| Drift computation | < 5ms | `compute_drift()` benchmark |
| SLO evaluation | < 10ms | `evaluate_slos()` benchmark |
| Memory footprint | < 10MB | Total Sprint 2 modules |
| Test execution | < 30s | Full test suite |

## v0.9.0 Roadmap Preview

### Phase 9 Sprint 1: Production Hardening
- Distributed consensus for scaling decisions
- Real-time alignment dashboard
- SLO prediction with ML models

### Phase 9 Sprint 2: Multi-Cluster Federation
- Cross-cluster scaling coordination
- Federated alignment aggregation
- Global SLO enforcement

### Phase 9 Sprint 3: Autonomous Operations
- Self-healing degradation recovery
- Adaptive threshold tuning
- Predictive scaling

### Phase 9 Sprint 4: v1.0.0 STABLE
- Full production readiness
- Security audit completion
- Performance benchmark certification
- Documentation finalization
