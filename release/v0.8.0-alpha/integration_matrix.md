# Integration Matrix - Phase 8 Sprint 1 ↔ Sprint 2

## Overview

This document maps the integration points between Phase 8 Sprint 1 (Marketplace, UI Backend, SLO Engine) and Sprint 2 (Cross-Model Scaling, Continuous Alignment, SLA Enforcer).

## Module Dependency Map

```
┌─────────────────────────────────────────────────────────────────┐
│                    Phase 8 Sprint 2 (v0.8.0-alpha.2)            │
├─────────────────────┬───────────────────────────────────────────┤
│  cross_model.rs     │  ContinuousAlignmentLoop                  │
│  (CrossModelScaler) │  (continuous.rs)                          │
├─────────────────────┼───────────────────────────────────────────┤
│  - NodeCapacity     │  - ingest_feedback()                      │
│  - route_request()  │  - compute_drift()                        │
│  - balance_load()   │  - request_human_review()                 │
│  - fallback_to_     │  - apply_steering()                       │
│    capacity()       │  - run_cycle()                            │
│  - validate_        │                                           │
│    compatibility()  │  ┌──────────────────────────────────┐    │
│                     │  │  Integrates with Sprint 1:       │    │
│                     │  │  - SLO Engine (slo/engine.rs)    │    │
│                     │  │  - UI Backend (ui/backend.rs)    │    │
│                     │  │  - Feedback Store (rlhf/)        │    │
│                     │  └──────────────────────────────────┘    │
├─────────────────────┴───────────────────────────────────────────┤
│  SLAEnforcer (enforcer.rs)                                      │
├─────────────────────────────────────────────────────────────────┤
│  - evaluate_slos()     ← SLO Engine metrics                     │
│  - trigger_degradation() → UI Backend notifications             │
│  - execute_rollback()  ← CrossModelScaler fallback              │
│  - notify_ops()        → Marketplace resource adjustments       │
└─────────────────────────────────────────────────────────────────┘
```

## Sprint 1 → Sprint 2 Integration Points

### 1. SLO Engine → SLA Enforcer
| Sprint 1 Component | Sprint 2 Component | Integration Type |
|-------------------|-------------------|------------------|
| `SLOEngine.evaluate_slo()` | `SLAEnforcer.evaluate_slos()` | Data flow: SLO metrics → enforcement decisions |
| `SLOEngine.track_metric()` | `SLAEnforcer.report_slo_value()` | Metric ingestion pipeline |
| `SLOEngine.trigger_degradation()` | `SLAEnforcer.trigger_degradation()` | Degradation action coordination |

**Data Flow**:
```
SLO Engine (track/evaluate) → SLA Enforcer (report/evaluate/degrade)
```

### 2. UI Backend → ContinuousAlignmentLoop
| Sprint 1 Component | Sprint 2 Component | Integration Type |
|-------------------|-------------------|------------------|
| `alignment_event_stream()` | `ContinuousAlignmentLoop.run_cycle()` | SSE streaming of alignment events |
| `RealtimeMetrics.alignment_drift` | `AlignmentLoopResult.drift_score` | Metric exposure |
| `RealtimeMetrics.slo_compliance` | `SLAEnforcer.evaluate_slos()` | SLO status in UI |

**Data Flow**:
```
AlignmentLoop (drift/steering) → UI Backend (SSE stream) → Frontend (real-time dashboard)
```

### 3. Marketplace → CrossModelScaler
| Sprint 1 Component | Sprint 2 Component | Integration Type |
|-------------------|-------------------|------------------|
| `ResourceMarketplace.list_resource()` | `CrossModelScaler.add_node()` | Node capacity as marketable resource |
| `ResourceMarketplace.match_request()` | `CrossModelScaler.route_request()` | Resource matching ↔ request routing |
| `NodeTrustInfo.trust_score` | `NodeCapacity.reputation` | Trust/reputation synchronization |

**Data Flow**:
```
Marketplace (listings/trust) ↔ CrossModelScaler (nodes/reputation)
```

## Sprint 2 Internal Integration

### CrossModelScaler ↔ ContinuousAlignmentLoop
- Scaling results (`ScaleResult.load_factor`) feed into alignment feedback
- Alignment drift scores influence routing priority
- Shared audit trail for operational visibility

### ContinuousAlignmentLoop ↔ SLAEnforcer
- Alignment drift metrics reported as SLO values
- SLO breaches can trigger alignment pauses
- Human review requests coordinated with degradation levels

### CrossModelScaler ↔ SLAEnforcer
- Fallback triggers report to enforcer
- Degradation level 3 (core-only) activates scaler fallback
- Rollback (level 4) resets scaler node registry

## Feature Flag Matrix

| Feature Flag | Modules Enabled | Version |
|-------------|----------------|---------|
| `phase8-sprint1` | marketplace, ui, slo::engine, phase8 | 0.8.0-alpha.1 |
| `phase8-sprint2` | scaling::cross_model, alignment::continuous, slo::enforcer, phase8 | 0.8.0-alpha.2 |
| Both | All Phase 8 modules | 0.8.0-alpha.2 |

## Test Coverage Matrix

| Module | Unit Tests | Integration Tests | Total |
|-------|-----------|------------------|-------|
| cross_model.rs | 20 | 4 (e2e) | 24 |
| continuous.rs | 20 | 2 (e2e) | 22 |
| enforcer.rs | 21 | 3 (e2e) | 24 |
| marketplace/engine.rs | 12 | 1 (e2e) | 13 |
| ui/backend.rs | 10 | 1 (e2e) | 11 |
| slo/engine.rs | 10 | 1 (e2e) | 11 |
| **Total** | **93** | **12** | **105** |

## Validation Checklist

- [x] CrossModelScaler routes to compatible schema nodes
- [x] ContinuousAlignmentLoop triggers human review when drift > threshold AND confidence < 0.8
- [x] SLAEnforcer executes progressive degradation (4 levels)
- [x] Fallback mechanism activates when nodes overloaded
- [x] Sybil resistance excludes low-reputation nodes
- [x] SLO metrics flow from Engine → Enforcer
- [x] Alignment events stream to UI via SSE
- [x] Marketplace trust scores sync with scaler reputation
- [x] Integration test validates full pipeline
- [x] 0 compilation errors, 0 clippy warnings
