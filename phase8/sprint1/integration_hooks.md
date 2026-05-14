# Phase 8 Sprint 1 — Integration Hooks

**Version:** `0.8.0-alpha.1`
**Feature Flag:** `phase8-sprint1`

---

## Overview

This document describes the connection points between Phase 8 Sprint 1 modules and the existing v0.5.0 / v0.6.0 / v0.7.0 codebase. All Phase 8 code is isolated behind `#[cfg(feature = "phase8-sprint1")]` and does not modify existing modules.

---

## Marketplace ↔ Staking Registry

| Phase 8 Component | Existing Component | Integration Point |
|---|---|---|
| `ResourceMarketplace::settle_trade()` | `staking/registry.rs` — `ResourceCommitment`, `NodeStatus` | Verifies provider `NodeStatus::Active` before settlement |
| `NodeTrustInfo` | `staking/registry.rs` — `resource_score()` | Maps `resource_score()` → trust score for pricing |

**Hook:** `settle_trade()` queries `ResourceRegistry::get_active_nodes()` to confirm the provider node is registered and active. Rejected if node is `Slashed` or `Unregistered`.

---

## Marketplace ↔ Reputation Ledger

| Phase 8 Component | Existing Component | Integration Point |
|---|---|---|
| `ResourceMarketplace::settle_trade()` | `reputation/ledger.rs` — `Contribution`, `ContributionType` | Credits consumed from reputation balance |
| `NodeTrustInfo.credits` | `reputation/scoring.rs` — `NodeReputation.credits` | Credit threshold validation |

**Hook:** Before confirming a trade, `settle_trade()` checks that `NodeTrustInfo.credits >= min_credit_threshold`. Credits are derived from `ReputationScorer::process_contribution()` history.

---

## UI Backend ↔ Monitoring Metrics

| Phase 8 Component | Existing Component | Integration Point |
|---|---|---|
| `GET /api/v3/metrics/realtime` | `monitoring/metrics.rs` — `MetricsManager` | Reads Prometheus counters/gauges for response payload |
| `RealtimeMetrics` | `monitoring/health.rs` — health checks | Aggregates health status into metrics endpoint |

**Hook:** `get_metrics_realtime()` calls `MetricsManager::encode_metrics()` to populate `RealtimeMetrics.sae_latency_ms`, `consensus_latency_ms`, etc.

---

## UI Backend ↔ Federation Bridge

| Phase 8 Component | Existing Component | Integration Point |
|---|---|---|
| `GET /api/v3/federation/status` | `federation/bridge.rs` — `FederationBridge` | Reads `connected_peers`, `trusted_networks`, `sync_round` |
| `FederationStatus` | `federation/trust_scoring.rs` — `TrustStats` | Maps trust stats to federation status |

**Hook:** `get_federation_status()` queries `FederationBridge` for peer count and schema version, `DynamicTrustScorer::stats()` for trust distribution.

---

## SLO Engine ↔ Alignment Engine

| Phase 8 Component | Existing Component | Integration Point |
|---|---|---|
| `SLOConfig` (SAE Latency) | `alignment/engine.rs` — `AlignmentResult.latency_ms` | Tracks alignment latency as SLO metric |
| `SLOEngine::trigger_degradation()` | `alignment/feedback_loop.rs` — `rollback_if_degraded()` | Calls feedback loop rollback when SLO breached |

**Hook:** `track_metric("sae_latency", alignment_result.latency_ms)` feeds alignment latency into SLO evaluation. When SLO goes critical, `trigger_degradation()` invokes `AlignmentFeedbackLoop::rollback_if_degraded()`.

---

## SLO Engine ↔ Governance

| Phase 8 Component | Existing Component | Integration Point |
|---|---|---|
| `SLOResult.audit_log` | `governance/proposal.rs` — `Proposal` | SLO breaches can auto-generate governance proposals |
| `DegradationAction::Alert` | `governance/voting.rs` — emergency voting | Critical SLO breaches trigger emergency governance |

**Hook:** When `enforce_sla()` triggers `DegradationAction::FallbackCoreOnly`, an emergency governance proposal is created for community review.

---

## SLO Engine ↔ Ops Alerting

| Phase 8 Component | Existing Component | Integration Point |
|---|---|---|
| `DegradationAction::Alert` | `ops/alert_rules_v2.yml` | Routes alerts to configured channels (PagerDuty, Slack, email) |
| `SLOResult.breach_duration` | `ops/monitoring/grafana_dashboards.json` | SLO compliance visualized in Grafana |

**Hook:** Alert rules in `alert_rules_v2.yml` define thresholds that match `SLOConfig.warning_threshold`. Grafana dashboard panels display SLO compliance status.

---

## Feature Flag Isolation

```
phase8-sprint1 (v0.8.0-alpha.1)
├── src/marketplace/engine.rs    ←  #[cfg(feature = "phase8-sprint1")]
├── src/marketplace/tests.rs
├── src/ui/backend.rs
├── src/ui/tests.rs
├── src/slo/engine.rs
├── src/slo/tests.rs
└── src/phase8/mod.rs            ←  Re-exports (feature-gated)
```

**No modifications to:**
- `src/main.rs`
- `src/p2p/`
- `src/sae/`
- `src/consensus/`
- `src/phase6/`
- `src/phase7/`

---

## Activation

To enable Phase 8 Sprint 1 modules:

```bash
cargo check --features phase8-sprint1
cargo test --features phase8-sprint1
```

To build with all phases:

```bash
cargo build --features "phase6-sprint2,phase7-sprint2,phase8-sprint1"
```
