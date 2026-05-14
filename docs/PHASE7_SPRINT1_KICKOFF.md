# ed2kIA Phase 7 Sprint 1 - Official Kickoff Document

**Date**: 2026-05-04
**Version**: 1.0
**Status**: Ready for Execution
**Target Release**: v0.7.0-alpha

---

## Executive Summary

Phase 7 marks the transition from **infrastructure** (Phase 6: interoperability, federation, staking) to **intelligence** (continuous learning, cross-network collaboration, advanced UI, liquid governance).

**Sprint 1** focuses on two foundational capabilities:

1. **Continuous Alignment Engine**: Enable ed2kIA SAE models to learn from human feedback in real-time with ethical guardrails, safe weight updates, and full audit trails.

2. **Cross-Net Federation**: Enable multiple independent ed2kIA networks to exchange SAE updates through a meta-federation layer, preserving diversity and maintaining Byzantine fault tolerance.

These capabilities transform ed2kIA from a static inference network into a **continuously learning, multi-network intelligence fabric**.

---

## Strategic Context

### Why Phase 7?

Phase 6 delivered:
- ✅ Tensor adapter for cross-model compatibility (ONNX, LLaMA, Mistral)
- ✅ FedAvg + Krum federation aggregation
- ✅ Resource staking + proof system
- ✅ API v2 with Ed25519 signature auth
- ✅ WASM sandbox for isolated SAE execution
- ✅ ZKP circuits for batch verification

Phase 7 builds on this foundation to deliver:
- 🔄 **Continuous learning** (not just static inference)
- 🌐 **Multi-network federation** (not just single-network)
- 🖥️ **Advanced UI** (not just CLI)
- 🗳️ **Liquid governance** (not just basic voting)

### Why Sprint 1 Focuses on Alignment + Cross-Net?

**Continuous Alignment** is the highest-priority capability because:
- SAE models drift from human intent as concepts evolve
- Community needs real-time feedback incorporation (not offline retraining)
- Ethical guardrails are essential before scaling
- Foundation for Phase 7 Sprint 2 (Advanced UI needs live model updates)

**Cross-Net Federation** is the second priority because:
- Domain-specific networks (healthcare, finance, research) need knowledge sharing
- Diversity-preserving aggregation prevents model collapse
- Builds directly on Phase 6 federation infrastructure
- Enables Phase 7 Sprint 3 (Liquid governance across networks)

---

## Sprint 1 Deliverables

### Continuous Alignment Engine (32 story points)

| Component | File | Story Points | Status |
|-----------|------|-------------|--------|
| Feedback Collector | `src/alignment/feedback_collector.rs` | 5 | Planned |
| Preference Model (RLHF) | `src/alignment/preference_model.rs` | 8 | Planned |
| Policy Updater | `src/alignment/policy_updater.rs` | 8 | Planned |
| Ethics Engine | `src/alignment/ethics_engine.rs` | 5 | Planned |
| Module Integration | `src/alignment/mod.rs` | 4 | Planned |
| SAE Loader Extension | `src/sae/loader.rs` | 2 | Planned |
| Alignment API | `src/api/routes.rs` | 4 | Planned |

### Cross-Net Federation (23 story points)

| Component | File | Story Points | Status |
|-----------|------|-------------|--------|
| CrossNet Gateway | `src/federation/crossnet_gateway.rs` | 8 | Planned |
| Meta Aggregator | `src/federation/meta_aggregator.rs` | 8 | Planned |
| Sync Protocol Extension | `src/federation/sync_protocol.rs` | 4 | Planned |
| Cross-Net API | `src/api/routes.rs` | 3 | Planned |

### Infrastructure (4 story points)

| Component | File | Story Points | Status |
|-----------|------|-------------|--------|
| Feature Gate | `Cargo.toml`, `src/phase7/mod.rs` | 1 | Planned |
| Monitoring Metrics | `src/monitoring/metrics.rs` | 2 | Planned |
| Config Schema | `launch/genesis/config.toml` | 1 | Planned |

**Total**: 59 story points, target ≥45 completed (76%)

---

## Architecture Overview

### Continuous Alignment Data Flow

```
Human Feedback (CLI/API/Community)
         │
         ▼
  FeedbackCollector ──▶ Buffer + Source Routing
         │
         ▼
  PreferenceModel ──▶ RLHF Gradient Computation
         │
         ▼
  EthicsEngine ──▶ Safety Validation (4 violation types)
         │
         ▼
  PolicyUpdater ──▶ Safe Weight Application + Checkpoint
         │
         ▼
  SAE Weights Updated ──▶ Live Inference Improved
```

### Cross-Net Federation Data Flow

```
  Network A (ed2k-main)       Network B (ed2k-health)
         │                            │
  Local FedAvg Aggregation    Local FedAvg Aggregation
         │                            │
  CrossNet Gateway ───────┐  ┌─── CrossNet Gateway
  (sign + queue)          │  │   (sign + queue)
                          │  │
                          ▼  ▼
                    Meta Aggregator
                    (diversity-preserving
                     weighted avg)
                          │
                          ▼
                    Aggregated Delta
                    → PolicyUpdater → SAE Weights
```

---

## Key Design Decisions

### 1. RLHF-Style Alignment (Not Direct Fine-Tuning)

**Decision**: Use PPO-style preference optimization with KL penalty.

**Rationale**:
- Preserves base model capabilities (KL penalty prevents catastrophic forgetting)
- Proven approach in LLM alignment (InstructGPT, ChatGPT)
- Configurable beta parameter for safety/performance tradeoff
- Audit trail of preference pairs for transparency

**Alternative Considered**: Direct gradient descent on human corrections.
**Rejected Because**: No safeguard against overfitting to individual feedback; no diversity preservation.

### 2. Ethics Engine as Hard Constraint (Not Soft Penalty)

**Decision**: Ethics violations block updates entirely (hard reject).

**Rationale**:
- Safety-critical: Concept tampering must be impossible
- Clear audit trail: Violations logged with full context
- Community trust: Transparent rejection reasons
- Regulatory compliance: Demonstrable safety controls

**Alternative Considered**: Soft penalty in loss function.
**Rejected Because**: Adversarial feedback could gradually push past soft constraints.

### 3. Network-Level Ed25519 Signatures (Not Per-Node)

**Decision**: Cross-network updates signed with network-level key, not individual node keys.

**Rationale**:
- Simplifies verification: One signature per update, not N signatures
- Network governance controls key: Key rotation through governance proposal
- Prevents Sybil: Can't fake network identity without key
- Builds on existing auth.rs infrastructure

**Alternative Considered**: Threshold signatures from multiple nodes.
**Rejected Because**: Overly complex for Sprint 1; can be added in Sprint 3.

### 4. Diversity Bonus in Meta Aggregation (Not Simple Weighted Avg)

**Decision**: Add diversity bonus proportional to input entropy.

**Rationale**:
- Prevents model collapse to majority network's distribution
- Preserves domain-specific knowledge (healthcare ≠ finance)
- Configurable α parameter for tuning
- Mathematically grounded (Shannon entropy)

**Alternative Considered**: Simple weighted average by network size.
**Rejected Because**: Large networks dominate; small specialized networks lose value.

---

## Risk Assessment

### Technical Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| RLHF gradient instability | Medium | High | Gradient clipping, learning rate caps, reward monitoring |
| Ethics engine false positives | Low | Medium | Configurable thresholds, appeal process |
| Cross-network signature verification failures | Low | Medium | Comprehensive test suite, fallback to manual review |
| Diversity bonus oscillation | Medium | Medium | α parameter caps, exponential smoothing |
| Checkpoint storage growth | Low | Low | LRU eviction, compression, size limits |

### Operational Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Scope creep from community feedback | High | Medium | Strict change control, backlog-only additions |
| Key team member unavailable | Low | High | Pair programming, documentation, code reviews |
| Integration conflicts between epics | Medium | High | Daily integration builds, feature flags |
| v0.6.0-RC issues require context switching | Medium | Medium | Dedicated hotfix branch, sprint buffer (14 SP) |

---

## Success Criteria

### Must Have (Launch Criteria for v0.7.0-alpha)

1. ✅ `cargo build --features phase7-sprint1` succeeds
2. ✅ `cargo test --features phase7-sprint1` passes (100%)
3. ✅ `cargo clippy --features phase7-sprint1` clean (0 warnings)
4. ✅ End-to-end alignment loop: Feedback → Gradient → Ethics Check → Apply → Verify
5. ✅ Cross-Net exchange: Sign → Send → Verify → Aggregate → Apply
6. ✅ Ethics engine blocks all 4 violation types (tested)
7. ✅ Diversity score ∈ [0, 1] (property tested)
8. ✅ Monitoring metrics visible in Grafana
9. ✅ API endpoints documented in OpenAPI spec
10. ✅ Security review: No critical findings

### Should Have

1. Community feedback API endpoint (`POST /api/v2/alignment/feedback`)
2. Grafana dashboard panels for alignment metrics
3. Config schema with sensible defaults
4. Release notes drafted

### Nice to Have

1. Property tests with proptest for all mathematical functions
2. Performance benchmarks for alignment loop latency
3. Example cross-network simulation script
4. Community onboarding guide for alignment feedback

---

## Dependencies on Phase 6

| Phase 6 Component | Used By Phase 7 Sprint 1 | Status |
|-------------------|--------------------------|--------|
| `src/human/feedback_cli.rs` | Feedback Collector | ✅ Stable |
| `src/human/concept_updater.rs` | Ethics Engine (reserved concepts) | ✅ Stable |
| `src/rlhf/feedback_store.rs` | Alignment storage backend | ✅ Stable |
| `src/sae/loader.rs` | Policy Updater (weight application) | ✅ Stable |
| `src/federation/avg_aggregator.rs` | CrossNet Gateway (local aggregation) | ✅ Stable |
| `src/federation/sync_protocol.rs` | Cross-Net messaging | ✅ Stable |
| `src/api/auth.rs` | Network-level signature verification | ✅ Stable |
| `src/api/routes.rs` | New API endpoints | ✅ Stable |
| `src/monitoring/metrics.rs` | New alignment/crossnet metrics | ✅ Stable |
| `src/p2p/swarm.rs` | Multi-network discovery | ✅ Stable |

**Critical Assumption**: All Phase 6 components are stable and tested with `--features phase6-sprint2`. No breaking changes expected.

---

## Testing Strategy

### Unit Tests (Per Module)
- 100% line coverage on new code
- Edge cases: Empty inputs, max values, error paths
- Property tests: Mathematical bounds (proptest)

### Integration Tests (Per Epic)
- Alignment: Full feedback → update → apply → verify loop
- Cross-Net: Sign → send → verify → aggregate → apply flow
- Safety: Ethics violations block updates
- Rollback: Failed update restores checkpoint

### End-to-End Tests (Sprint Level)
- Simulated 2-network federation with alignment updates
- Adversarial feedback injection (should be blocked)
- Load test: 1000 feedback items through alignment pipeline
- Chaos test: Network partition during cross-net exchange

### Performance Benchmarks
- Alignment loop latency: <100ms per update (target)
- Cross-net signature verification: <10ms per update (target)
- Meta aggregation: <500ms for 10 networks (target)

---

## Communication & Documentation

### During Sprint
- Daily standups in `#sprint-standup` (async)
- Blocker escalation in `#sprint-blockers` (1h SLA)
- Code reviews in `#code-review` (4h SLA)
- Weekly demos (Friday 16:00 UTC)

### Deliverables
- Architecture sketch: [`phase7/sprint1/architecture_sketch.md`](phase7/sprint1/architecture_sketch.md)
- Task breakdown: [`phase7/sprint1/task_breakdown.md`](phase7/sprint1/task_breakdown.md)
- Sprint kickoff: [`phase7/sprint1/sprint_kickoff.md`](phase7/sprint1/sprint_kickoff.md)
- This document: [`docs/PHASE7_SPRINT1_KICKOFF.md`](docs/PHASE7_SPRINT1_KICKOFF.md)

### Post-Sprint
- Sprint retro report
- v0.7.0-alpha release notes
- Updated roadmap with learnings
- Community announcement

---

## Next Steps

### Immediate (This Week)
1. ✅ Review and approve architecture sketch
2. ✅ Review and approve task breakdown
3. ✅ Confirm team availability and assignments
4. ⏳ Create `feature/phase7-sprint1` branch from `main`
5. ⏳ Configure CI for `phase7-sprint1` feature
6. ⏳ Sprint kickoff meeting

### Week 1
1. Implement feature gate (`phase7-sprint1`)
2. Build Feedback Collector module
3. Build Ethics Engine module
4. Start CrossNet Gateway

### Week 2
1. Build Preference Model (RLHF core)
2. Complete CrossNet Gateway
3. Extend Sync Protocol

### Week 3
1. Build Policy Updater with safety
2. Build Meta Aggregator
3. Extend SAE Loader

### Week 4
1. Integration testing
2. API endpoints
3. Monitoring + config
4. Sprint review + retro

---

## Appendix: Related Documents

| Document | Location | Purpose |
|----------|----------|---------|
| Phase 7 Roadmap | [`phase7/roadmap.md`](phase7/roadmap.md) | Full Phase 7 plan (4 sprints) |
| Phase 7 Backlog | [`phase7/backlog.md`](phase7/backlog.md) | Prioritized user stories |
| Phase 7 Research Notes | [`phase7/research_notes.md`](phase7/research_notes.md) | State of the art review |
| Architecture Sketch | [`phase7/sprint1/architecture_sketch.md`](phase7/sprint1/architecture_sketch.md) | Technical design |
| Task Breakdown | [`phase7/sprint1/task_breakdown.md`](phase7/sprint1/task_breakdown.md) | Tasks, estimates, assignments |
| Sprint Kickoff | [`phase7/sprint1/sprint_kickoff.md`](phase7/sprint1/sprint_kickoff.md) | Sprint ceremonies, cadence |
| v0.6.0-RC Rollout Plan | [`release/v0.6.0-rc/rollout_plan.md`](release/v0.6.0-rc/rollout_plan.md) | Canary deployment strategy |
| Community Feedback Schema | [`ops/feedback/community_feedback_schema.json`](ops/feedback/community_feedback_schema.json) | Feedback structure |
| Triage Workflow | [`ops/feedback/triage_workflow.md`](ops/feedback/triage_workflow.md) | Feedback processing |
| Alert Rules | [`ops/monitoring/alert_rules_v2.yml`](ops/monitoring/alert_rules_v2.yml) | Prometheus alerts |
| Grafana Dashboards | [`ops/monitoring/grafana_dashboards.json`](ops/monitoring/grafana_dashboards.json) | Monitoring dashboards |

---

**Approved By**: Core Team
**Date**: 2026-05-04
**Next Review**: Sprint 1 Weekly Check-in (Week 1, Day 5)

*Phase 7 Sprint 1 - From Static Inference to Continuous Intelligence*
