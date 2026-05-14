# Changelog - ed2kIA v0.8.0-alpha

## v0.8.0-alpha.2 (2026-05-04) - Fase 8 Sprint 2

### Added
- **CrossModelScaler** (`src/scaling/cross_model.rs`)
  - Dynamic routing across distributed models based on node capacity, latency, reputation, schema compatibility
  - Safe fallback to core-only mode when nodes exceed load thresholds
  - Sybil resistance: nodes with reputation < 0.2 excluded from routing
  - Exponential Moving Average (EMA) latency tracking
  - 20 unit tests covering routing, load balancing, fallback, capacity limits, Sybil resistance

- **ContinuousAlignmentLoop** (`src/alignment/continuous.rs`)
  - Continuous loop: drift → feedback → steering → human validation → application
  - Human review trigger when `drift > threshold` AND `confidence < 0.8`
  - SHA-256 hashed audit trail with configurable capacity (256-512 entries)
  - Configurable feedback buffer per layer (64-256 entries)
  - 20 unit tests covering feedback ingestion, drift computation, human review, steering application

- **SLAEnforcer** (`src/slo/enforcer.rs`)
  - 4-level progressive degradation: Normal → L1 Warning → L2 Reduce Peers → L3 Core-Only → L4 Rollback
  - Configurable thresholds per SLO metric
  - Automatic rollback execution when breach windows exceeded
  - Operations notifications via structured notification queue
  - 21 unit tests covering SLO registration, evaluation, degradation, rollback, notifications

- **Integration Test** (`tests/integration/phase8_sprint2_e2e.rs`)
  - 12 end-to-end tests validating complete flow: scaling → alignment → SLO → marketplace → UI
  - Cross-module integration scenarios
  - Degradation cascade prevention validation

### Changed
- **Phase 8 Module** (`src/phase8/mod.rs`)
  - Updated version to `0.8.0-alpha.2`
  - Added Sprint 2 re-exports for `scaling::cross_model`, `alignment::continuous`, `slo::enforcer`
  - Feature-gated conditional compilation for `phase8-sprint2`

- **Cargo.toml**
  - Added `phase8-sprint2 = []` feature flag

### Technical Details
- **Feature Flag**: `#[cfg(feature = "phase8-sprint2")]`
- **Version**: `0.8.0-alpha.2`
- **Total Tests**: 61+ (20 scaling + 20 alignment + 21 enforcer + 12 integration)
- **Isolation**: No modifications to main, p2p/, sae/, consensus/, phase6/, phase7/

---

## v0.8.0-alpha.1 (2026-04-28) - Fase 8 Sprint 1

### Added
- **ResourceMarketplace** (`src/marketplace/engine.rs`)
  - Decentralized resource matching with dynamic pricing
  - Anti-gaming validation (price deviation, trust anomalies)
  - 12+ unit tests

- **UI Backend** (`src/ui/backend.rs`)
  - Axum endpoints for ed2kIA v3 API
  - SSE (Server-Sent Events) for real-time alignment streaming
  - LRU cache for response optimization
  - 10+ unit tests

- **SLO Engine** (`src/slo/engine.rs`)
  - Service Level Objective tracking, evaluation, enforcement
  - Metric window management with configurable sizes
  - Degradation action triggers
  - 10+ unit tests

- **Phase 8 Module** (`src/phase8/mod.rs`)
  - Feature-gated re-exports for Sprint 1 modules
  - Version `0.8.0-alpha.1`

### Technical Details
- **Feature Flag**: `#[cfg(feature = "phase8-sprint1")]`
- **Version**: `0.8.0-alpha.1`
- **Total Tests**: 32+ (12 marketplace + 10 UI + 10 SLO)

---

## v0.7.0-beta (2026-04-15) - Fase 7 Consolidación

### Summary
- Continuous Alignment Engine stabilization
- Cross-Net Federation Bridge hardening
- Feedback Loop closure with audit trail
- Trust Scoring with Sybil resistance
- Schema Registry with semver compatibility
- 120+ tests passing, 0 errors, 0 warnings

---

## Migration Guide: v0.7.0-beta → v0.8.0-alpha.2

### Breaking Changes
- None. Phase 8 modules are feature-gated and isolated.

### New Features
- Enable Sprint 1: `cargo build --features phase8-sprint1`
- Enable Sprint 2: `cargo build --features phase8-sprint2`
- Enable both: `cargo build --features "phase8-sprint1,phase8-sprint2"`

### API Additions
```rust
// CrossModelScaler
use ed2kia::phase8::scaling::cross_model::{CrossModelScaler, NodeCapacity, RoutingRequest};

// ContinuousAlignmentLoop
use ed2kia::phase8::alignment::continuous::{ContinuousAlignmentLoop, ContinuousFeedback};

// SLAEnforcer
use ed2kia::phase8::slo_enforcer::enforcer::{SLAEnforcer, EnforcerConfig};
```

### Configuration
- CrossModelScaler: Default load threshold 0.8, min reputation 0.2
- ContinuousAlignmentLoop: Default drift threshold 0.3, min confidence 0.8
- SLAEnforcer: Default warning 0.8, critical 0.95, rollback 0.99
