# Phase 7 Sprint 1 - Sprint Kickoff

**Sprint**: Phase 7 Sprint 1
**Dates**: Week 1-4 (4 weeks)
**Target Release**: v0.7.0-alpha
**Feature Flag**: `phase7-sprint1`

---

## Sprint Goal

Build the foundation for **Continuous Alignment Engine** and **Cross-Net Federation**, enabling ed2kIA to:
1. Learn from human feedback in real-time with ethical guardrails
2. Exchange SAE updates across independent ed2kIA networks

**Success Criteria**:
- [ ] Continuous Alignment Engine processes feedback → update → apply loop end-to-end
- [ ] Ethics Engine blocks malicious updates (tested with adversarial cases)
- [ ] Cross-Net Gateway verifies and routes updates between 2+ simulated networks
- [ ] Meta Aggregator produces diversity-preserving aggregation
- [ ] All new code: 100% test coverage, clippy clean, feature-gated
- [ ] v0.7.0-alpha builds with `--features phase7-sprint1`

---

## Team & Roles

| Role | Person | Focus Area |
|------|--------|------------|
| Sprint Lead | @sprint-lead | Overall coordination, blocker removal |
| Alignment Lead | @alignment-lead | Tasks 1.1, 1.3, 1.5 |
| ML Team | @ml-team | Task 1.2 (Preference Model) |
| Security Lead | @security-lead | Task 1.4 (Ethics Engine) |
| SAE Lead | @sae-lead | Task 1.6 (SAE Loader extension) |
| Federation Lead | @fed-lead | Tasks 2.1, 2.2 |
| P2P Team | @p2p-team | Task 2.3 (Sync Protocol) |
| API Lead | @api-lead | Tasks 1.7, 2.4 |
| Dev Lead | @dev-lead | Tasks 3.1, 3.3 |
| DevOps Lead | @devops-lead | Task 3.2 (Monitoring) |

---

## Daily Cadence

### Standup (09:00 UTC, 15 min)
**Format** (async in `#sprint-standup`):
```
Yesterday: [what I completed]
Today: [what I'm working on]
Blockers: [anything blocking me]
```

### Code Review SLA
- PR submitted → Review within 4 hours (business hours)
- Review feedback → Response within 2 hours
- Max review cycle: 24 hours

### Daily Integration Build
- Triggered at 18:00 UTC on `main` + `feature/phase7-sprint1`
- Results posted to `#ci-results`
- Failures → On-call developer investigates within 1 hour

---

## Weekly Cadence

### Monday: Sprint Planning / Check-in
- **Time**: 14:00 UTC, 60 min
- **Focus**: Review week goals, assign tasks, identify dependencies
- **Output**: Updated task board, commitment list

### Wednesday: Mid-Sprint Review
- **Time**: 14:00 UTC, 30 min
- **Focus**: Progress check, demo completed work, adjust priorities
- **Output**: Burndown chart update, risk assessment

### Friday: Demo & Retro
- **Time**: 16:00 UTC, 60 min
- **Focus**: Demo working features, retro discussion, action items
- **Output**: Demo recording, retro action items

### Friday: Priority Triage
- **Time**: 10:00 UTC, 30 min
- **Focus**: Review community feedback, assign priorities (P0-P3)
- **Output**: Updated backlog, triage decisions logged

---

## Week-by-Week Plan

### Week 1: Foundation
**Theme**: "Build the pipes"

| Day | Focus | Deliverable |
|-----|-------|-------------|
| Day 1 | Kickoff + Feature gate | `phase7-sprint1` feature in Cargo.toml |
| Day 2 | Feedback Collector + Ethics Engine start | `src/alignment/feedback_collector.rs` skeleton |
| Day 3 | Feedback Collector complete | Tests passing, clippy clean |
| Day 4 | Ethics Engine + CrossNet Gateway start | `src/alignment/ethics_engine.rs` skeleton |
| Day 5 | Week 1 Demo | Feedback Collector + Ethics Engine demo |

**Week 1 Definition of Done**:
- [ ] `phase7-sprint1` feature compiles
- [ ] Feedback Collector passes all tests
- [ ] Ethics Engine detects all 4 violation types
- [ ] CrossNet Gateway started (NetworkIdentity struct)

---

### Week 2: Core Logic
**Theme**: "Make it think"

| Day | Focus | Deliverable |
|-----|-------|-------------|
| Day 6 | Preference Model start | `src/alignment/preference_model.rs` skeleton |
| Day 7 | Preference Model + CrossNet Gateway | Gradient computation working |
| Day 8 | Preference Model complete | PPO-style loss tested |
| Day 9 | Sync Protocol Extension | CrossNet messages routed |
| Day 10 | Week 2 Demo | Preference Model + CrossNet Gateway demo |

**Week 2 Definition of Done**:
- [ ] Preference Model computes valid gradients
- [ ] KL penalty bounded and tested
- [ ] CrossNet Gateway verifies Ed25519 signatures
- [ ] Sync Protocol routes cross-network messages

---

### Week 3: Safety & Aggregation
**Theme**: "Make it safe"

| Day | Focus | Deliverable |
|-----|-------|-------------|
| Day 11 | Policy Updater start | `src/alignment/policy_updater.rs` skeleton |
| Day 12 | Policy Updater + Meta Aggregator start | Checkpoint mechanism |
| Day 13 | Policy Updater complete | Rollback tested |
| Day 14 | Meta Aggregator | Diversity scoring working |
| Day 15 | Week 3 Demo | Policy Updater + Meta Aggregator demo |

**Week 3 Definition of Done**:
- [ ] Policy Updater applies deltas within safety bounds
- [ ] Checkpoint/rollback cycle verified
- [ ] Cooldown enforcement working
- [ ] Meta Aggregator produces valid diversity scores

---

### Week 4: Integration & Polish
**Theme**: "Make it work together"

| Day | Focus | Deliverable |
|-----|-------|-------------|
| Day 16 | Alignment Integration | End-to-end pipeline wired |
| Day 17 | API Endpoints | `/api/v2/alignment/*` + `/api/v2/federation/crossnet/*` |
| Day 18 | Monitoring + Config | Metrics visible, config schema complete |
| Day 19 | Integration Testing | Full pipeline tests, property tests |
| Day 20 | Sprint Review + Retro | v0.7.0-alpha ready |

**Week 4 Definition of Done**:
- [ ] End-to-end alignment loop tested
- [ ] All API endpoints documented and tested
- [ ] Monitoring metrics in Grafana
- [ ] Config schema validated
- [ ] v0.7.0-alpha builds clean with `--features phase7-sprint1`
- [ ] Sprint retro completed

---

## Risk Register

| Risk | Probability | Impact | Mitigation | Owner |
|------|------------|--------|------------|-------|
| RLHF gradient instability | Medium | High | Cap learning rate, add gradient clipping | @ml-team |
| Ed25519 key management complexity | Low | Medium | Use existing auth.rs patterns | @fed-lead |
| Diversity bonus causes oscillation | Medium | Medium | Cap α, add smoothing | @fed-lead |
| Checkpoint storage growth | Low | Low | LRU eviction, compression | @alignment-lead |
| Integration conflicts between epics | Medium | High | Daily integration builds | @sprint-lead |
| Scope creep from community feedback | High | Medium | Strict change control, backlog only | @sprint-lead |

---

## Communication Plan

### Channels
| Channel | Purpose | Response SLA |
|---------|---------|-------------|
| `#sprint-standup` | Daily async standups | Same day |
| `#sprint-blockers` | Blocker escalation | 1 hour |
| `#code-review` | PR discussions | 4 hours |
| `#ci-results` | Build/test notifications | 1 hour (failures) |
| `#alignment-dev` | Alignment Engine discussions | Same day |
| `#crossnet-dev` | Cross-Net Federation discussions | Same day |
| `#demo-prep` | Demo coordination | Same day |

### Escalation Path
1. **Peer**: Ask in relevant channel
2. **Sprint Lead**: If no response in 4 hours
3. **Tech Lead**: If sprint-lead unavailable or strategic decision
4. **Core Team**: If blocking entire sprint

---

## Pre-Requisites Checklist

Before sprint starts:
- [ ] `phase7/sprint1/architecture_sketch.md` reviewed and approved
- [ ] `phase7/sprint1/task_breakdown.md` reviewed and assigned
- [ ] Development environment ready (Rust toolchain, dependencies)
- [ ] CI pipeline configured for `phase7-sprint1` feature
- [ ] Feature branch `feature/phase7-sprint1` created from `main`
- [ ] Team availability confirmed (PTO, holidays)
- [ ] v0.6.0-RC release completed (no context switching)

---

## Sprint Ceremonies

### Kickoff Meeting (Day 1, 09:00 UTC)
**Agenda**:
1. Sprint goal review (10 min)
2. Architecture walkthrough (20 min)
3. Task assignment confirmation (15 min)
4. Risk discussion (10 min)
5. Q&A (5 min)

### Daily Standup (09:00 UTC, 15 min async)
Post in `#sprint-standup` using template.

### Weekly Demo (Friday, 16:00 UTC)
**Format**:
1. Live demo of working features (20 min)
2. Metrics review (10 min)
3. Burndown chart (5 min)
4. Next week preview (5 min)

### Sprint Retro (Day 20, 16:00 UTC)
**Format**: Start/Stop/Continue
1. What started this sprint that went well?
2. What should we stop doing?
3. What should we continue?
4. Action items for next sprint

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Story points completed | ≥45/55 (82%) | Task board |
| Test coverage (new code) | 100% | tarpaulin/cargo-coverage |
| Clippy warnings | 0 | cargo clippy |
| CI builds green | 100% | GitHub Actions |
| PR review time | <24h | GitHub metrics |
| Blocker resolution | <4h | `#sprint-blockers` |
| Community feedback triaged | 100% | Triage board |

---

## Launch Criteria for v0.7.0-alpha

All must be true:
1. `cargo build --features phase7-sprint1` succeeds
2. `cargo test --features phase7-sprint1` passes (all tests)
3. `cargo clippy --features phase7-sprint1` clean (0 warnings)
4. End-to-end alignment loop tested and passing
5. Cross-Net federation tested with 2 simulated networks
6. Ethics engine blocks adversarial updates (tested)
7. Monitoring metrics visible in Grafana dashboard
8. API endpoints documented in OpenAPI spec
9. Release notes drafted (`docs/RELEASE_NOTES_v0.7.0-alpha.md`)
10. Security review completed (no critical findings)

---

*Phase 7 Sprint 1 Kickoff | 2026-05-04*
