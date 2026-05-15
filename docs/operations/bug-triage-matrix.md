# Bug Triage Matrix — ed2kIA v1.8.0-beta.1

**Version:** v1.8.0-beta.1
**Fecha:** 2026-05-15
**Estado:** ACTIVE
**FASE:** 61 — Performance Monitoring & Bug Triage Automation

---

## Severity Definitions

| Severity | Label | Response SLA | Resolution SLA | Description |
|----------|-------|--------------|----------------|-------------|
| **P0** | Critical | 2h | 24h | Data loss, security vulnerability, crash on main path, blocker for all testing |
| **P1** | High | 12h | 72h | Major feature broken, no workaround, blocks significant testing |
| **P2** | Medium | 48h | 1 week | Feature degraded, workaround exists, non-critical path |
| **P3** | Low | 7d | Next release | Cosmetic, documentation, minor UX, nice-to-have |

---

## Status Definitions

| Status | Meaning | Action Required |
|--------|---------|-----------------|
| **New** | Just reported, not reviewed | Triage within SLA |
| **Triaged** | Reviewed, severity assigned | Assign to owner |
| **In Progress** | Being worked on | Regular updates |
| **In Review** | PR open, awaiting review | Review within 24h |
| **Fixed** | Merged to main | Verify in next build |
| **Verified** | Confirmed fixed in beta | Close issue |
| **Won't Fix** | Intentional behavior / out of scope | Document rationale |
| **Duplicate** | Already tracked | Link to canonical issue |

---

## Module Categories

| Module | Description | Owner | Test Coverage |
|--------|-------------|-------|---------------|
| **p2p** | libp2p swarm, gossipsub, peer management | Core | `cargo test --features stable p2p` |
| **sae** | SAE loading, fine-tuning, forward pass | ML | `cargo test --features stable sae` |
| **zkp** | Proof generation, verification, batching | Crypto | `cargo test --features stable zkp` |
| **federation** | Cross-model scaling, shard routing | Core | `cargo test --features stable federation` |
| **bridge** | Quantization, async steering, federation bridge | Bridge | `cargo test --features stable bridge` |
| **api** | REST endpoints, auth, explorer | DX | `cargo test --features stable api` |
| **reputation** | Scoring, proofs, anti-sybil | Security | `cargo test --features stable reputation` |
| **governance** | Proposals, voting, reputation gating | Governance | `cargo test --features stable governance` |
| **web** | SSE, WebSocket, dashboard | DX | `cargo test --features stable web` |
| **wasm** | Mobile bridge, browser extension | DX | `cargo test --features v1.8-sprint2 wasm` |
| **routing** | Geographic routing, adaptive P2P | Core | `cargo test --features v1.8-sprint2 routing` |
| **dx** | Developer tools, CLI, configs | DX | `cargo test --features v1.8-sprint2` |

---

## Triage Workflow

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Bug Report │────▶│   Triage    │────▶│  Assign     │
│  (GitHub    │     │  (≤SLA)     │     │  Owner      │
│   Issue)    │     │             │     │             │
└─────────────┘     └─────────────┘     └─────────────┘
                                              │
                                              ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Verified   │◀────│  Fixed      │◀────│ In Progress │
│  (Close)    │     │  (Merge PR) │     │ (Develop)   │
└─────────────┘     └─────────────┘     └─────────────┘
```

### Step-by-Step

1. **Report**: Tester creates issue using `beta-bug-report.md` template
2. **Auto-Label**: GitHub Actions adds `beta` + `needs-triage` labels
3. **Triage** (within SLA):
   - Verify reproduction steps
   - Assign severity (P0-P3)
   - Assign module category
   - Add `triaged` label, remove `needs-triage`
4. **Assign**: Link to owner based on module category
5. **Develop**: Owner creates fix PR with regression test
6. **Review**: Maintainer reviews within 24h
7. **Merge**: Fix lands in `main` branch
8. **Verify**: Tester confirms fix in next beta build
9. **Close**: Issue marked `verified` + closed

---

## Escalation Path

| Level | Role | Trigger | Action |
|-------|------|---------|--------|
| L1 | Triage Lead | New issue | Initial severity + module assignment |
| L2 | Module Owner | Triaged issue | Fix development + PR |
| L3 | Release Lead | P0/P1 unresolved after SLA | Emergency hotfix branch |
| L4 | Project Lead | Multiple P0s / release blocker | Beta pause / rollback decision |

### Escalation Contacts

| Role | Contact | Response Time |
|------|---------|---------------|
| Triage Lead | @Stuartemk (GitHub) | 2h |
| Module Owners | GitHub assignees | Per SLA |
| Release Lead | @Stuartemk (GitHub) | 1h for P0 |
| Project Lead | @Stuartemk (GitHub) | 30m for critical |

---

## Automated Triage Rules

### Auto-Label by Keyword

| Keyword Pattern | Auto-Label | Severity Hint |
|-----------------|------------|---------------|
| `panic\|crash\|segfault\|abort` | `crash` | P0-P1 |
| `memory\|leak\|oom\|overflow` | `memory` | P1 |
| `slow\|latency\|timeout\|hang` | `performance` | P1-P2 |
| `incorrect\|wrong\|mismatch\|fail` | `correctness` | P1-P2 |
| `ui\|css\|layout\|display` | `ui` | P3 |
| `doc\|typo\|comment\|readme` | `documentation` | P3 |
| `security\|exploit\|vulnerability` | `security` | P0 |

### Auto-Assign by Module Path

| File Path Pattern | Auto-Assign Module |
|-------------------|-------------------|
| `src/p2p/**` | p2p |
| `src/sae/**` | sae |
| `src/zkp/**` | zkp |
| `src/federation/**` | federation |
| `src/bridge/**` | bridge |
| `src/api/**` | api |
| `src/reputation/**` | reputation |
| `src/governance/**` | governance |
| `src/web/**` | web |
| `src/wasm/**` | wasm |
| `src/routing/**` | routing |

---

## Weekly Triage Cadence

| Day | Activity | Duration | Output |
|-----|----------|----------|--------|
| Monday | Review all new issues from weekend | 1h | Triage summary |
| Wednesday | Mid-week check + P0/P1 status | 30m | Status update |
| Friday | Weekly triage report + backlog grooming | 1h | Weekly report |

### Triage Report Template

```markdown
## Triage Report — YYYY-MM-DD

### Summary
- New issues: N
- Triaged: N
- Fixed: N
- Verified: N
- Open P0: N
- Open P1: N

### P0 Issues (Critical)
| Issue | Title | Module | Owner | ETA |
|-------|-------|--------|-------|-----|

### P1 Issues (High)
| Issue | Title | Module | Owner | ETA |
|-------|-------|--------|-------|-----|

### Blockers
- [ ] Any blockers to beta continuation

### Decisions Needed
- [ ] Any decisions requiring project lead input
```

---

## Metrics Dashboard Integration

The bug triage matrix feeds into the Dashboard v2 (see [`docs/operations/dashboard-v2-spec.md`](docs/operations/dashboard-v2-spec)):

| Metric | Source | Endpoint | Refresh |
|--------|--------|----------|---------|
| Open issues by severity | GitHub API | `/api/v2/metrics/issues` | 5min |
| Mean time to triage | Issue timestamps | `/api/v2/metrics/triage` | 1h |
| Mean time to fix | Issue → PR → Merge | `/api/v2/metrics/fix` | 1h |
| SLA compliance | SLA vs actual | `/api/v2/metrics/sla` | 1h |
| Module defect density | Issues / LOC | `/api/v2/metrics/defects` | Daily |

---

## Integration with Feedback Tracker

The bug triage matrix works alongside [`docs/beta/feedback-tracker.md`](../beta/feedback-tracker.md):

1. **Bug reported** → Created in GitHub Issues (beta-bug-report.md template)
2. **Triaged** → Added to feedback-tracker.md with severity + module
3. **Fixed** → Status updated in feedback-tracker.md
4. **Verified** → Issue closed + feedback-tracker.md marked resolved

---

## Emergency Procedures

### P0 Response (2h SLA)

1. **Detect**: Alert via GitHub notification / Discord #bug-reports
2. **Acknowledge**: Comment "Acknowledged, investigating" within 30m
3. **Reproduce**: Confirm bug in isolated environment within 1h
4. **Fix**: Create hotfix PR within 2h
5. **Deploy**: Emergency beta patch within 4h
6. **Communicate**: Update issue + Discord within 30m of fix

### Beta Pause Criteria

Trigger beta pause if:
- ≥2 P0 issues unresolved after 24h
- ≥1 data loss / security vulnerability
- Test pass rate drops below 80%
- Core P2P connectivity broken for >4h

### Rollback Procedure

1. Stop all beta testing activity
2. Notify testers via Discord + GitHub
3. Revert to previous stable tag: `git checkout v1.7.0-stable`
4. Document root cause in retrospective
5. Resume beta when fix verified

---

**Estado:** ACTIVE
**Última actualización:** 2026-05-15T21:07:00Z
**Autor:** Qweni (Auto-Push Protocol)
**FASE:** 61 — Performance Monitoring & Bug Triage Automation
