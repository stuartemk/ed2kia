# Continuous Operations Cycle — ed2kIA v1.8.0-sprint1

**Version:** 1.0  
**Generated:** 2026-05-15  
**Sprint:** v1.8 "ChatGPT Moment"  
**Status:** Active  

---

## 1. Overview

This document defines the **automated continuous operations cycle** for ed2kIA, establishing the weekly rhythm, IA/Human role division, automated flows, and rollback criteria.

**Core Principle:** IA executes repetitive tasks and generates drafts; Human approves, escalates, and makes strategic decisions.

---

## 2. Weekly Cycle: Standup → Triage → PoC → Benchmark → Auto-Push

| Day | Phase | IA (Qweni) | Human (Orquestador) | Output |
|-----|-------|------------|---------------------|--------|
| **Mon** | Standup | Generate `weekly-standup-week[N].md`, run `scripts/update_weekly_metrics.sh`, collect metrics | Review standup, approve hitos, sign-off | Standup MD + metrics JSON |
| **Tue** | Triage | Label issues, assign teams, draft PR reviews, flag SEV-1/SEV-2 | Final PR approvals, SEV decisions, escalations | Triage report + labeled issues |
| **Wed** | PoC | Implement feature flags, write tests, draft docs, run `cargo check` | Architecture review, RFC alignment, feature approvals | PoC PR + test results |
| **Thu** | Benchmark | Run `cargo bench`, compare vs baseline, flag regressions >10% | Review benchmarks, approve performance trade-offs | Benchmark diff + regression report |
| **Fri** | Auto-Push | Validate → `git add -A` → `git commit` → `git push`, generate sign-off JSON | Final review, merge approvals, release decisions | Commits pushed + sign-off JSON |

### 2.1 Phase Details

#### Standup (Monday)
- **IA Actions:**
  1. Read `docs/operations/weekly-standup-week[N-1].md` (previous week)
  2. Execute `scripts/update_weekly_metrics.sh`
  3. Generate new `docs/operations/weekly-standup-week[N].md` with:
     - Hitos v1.8 Sprint (completed/in-progress/pending)
     - Issues/PRs Activos table
     - Métricas Día 1 (Nodos, Benchmarks, Funding, Comunidad)
     - Bloqueos (con owner + ETA)
     - Acciones de la Semana (con SLAs)
     - Risk Register (probability × impact)
     - Sign-off JSON
  4. Commit: `docs(ops): week [N] standup`
- **Human Actions:**
  1. Review standup for accuracy
  2. Approve/adjust hitos
  3. Sign-off: "Week [N] standup approved"

#### Triage (Tuesday)
- **IA Actions:**
  1. List unlabelled issues → apply labels per routing table
  2. Draft PR reviews (style, security, ethics, performance)
  3. Flag SEV-1/SEV-2 issues immediately
  4. Update `docs/operations/daily-metrics-dashboard.md`
- **Human Actions:**
  1. Final PR approvals/rejections
  2. SEV-1 war room decisions
  3. Escalation approvals (RFC-001, v1.8 roadmap)

#### PoC (Wednesday)
- **IA Actions:**
  1. Implement feature behind `#[cfg(feature = "v1.8-sprint1")]`
  2. Write tests (≥ 20 per module)
  3. Run `cargo check --features stable` + `cargo test --lib`
  4. Draft documentation updates
  5. Create PR with conventional commit
- **Human Actions:**
  1. Architecture review
  2. RFC-001 alignment check
  3. Feature flag approvals
  4. Merge decisions

#### Benchmark (Thursday)
- **IA Actions:**
  1. Run `cargo bench --package ed2kIA-benchmarks --features stable`
  2. Compare vs `benchmarks/results/baseline-v1.7.json`
  3. Flag regressions >10% (warning) or >15% (critical)
  4. Generate benchmark diff report
  5. If regression: create issue `perf(*)` + `regression` label
- **Human Actions:**
  1. Review benchmark diffs
  2. Approve performance trade-offs (if justified)
  3. Escalate critical regressions to RFC-001

#### Auto-Push (Friday)
- **IA Actions:**
  1. Run full validation:
     - `cargo check --features stable` → 0 errors
     - `cargo clippy --features stable` → 0 warnings
     - `cargo test --features stable` → all green
  2. If PASS: `git add -A` → `git commit -m "type(scope): desc"` → `git push origin main`
  3. Generate sign-off JSON
  4. Archive weekly report → `release/reports/standup-YYYY-MM-DD.json`
- **Human Actions:**
  1. Final review of commits
  2. Approve push to main
  3. Weekly sign-off

---

## 3. IA/Human Role Division

### 3.1 IA (Qweni) — Execution Layer

**Responsibilities:**
- Automated triage (labeling, assignment, severity classification)
- Code implementation (with feature flags)
- Test generation (unit, integration, stress)
- Benchmark execution and comparison
- Documentation drafting
- Metrics collection and reporting
- Standup generation
- PR drafting with conventional commits
- Funding monitoring (channel status checks)

**Constraints:**
- NO merges to main without Human approval
- NO SEV-1 fixes without Human approval
- NO architectural changes without RFC alignment
- NO funding decisions or financial logic
- MUST read files before modifying (Zero Assumptions)
- MUST validate with CI before committing

### 3.2 Human (Orquestador) — Decision Layer

**Responsibilities:**
- Final PR approvals and merges
- Release decisions (tag, publish, deploy)
- SEV-1 incident commands (war room, rollback, comms)
- Architecture reviews and RFC approvals
- Funding strategy and grant applications
- Community engagement and communications
- Escalation resolutions
- Weekly sign-offs

**Constraints:**
- MUST review IA-generated drafts before approval
- MUST document decisions in incidents/decisions logs
- MUST enforce ethical clause (zero unsafe, zero telemetry, zero financial)

### 3.3 Handover Protocol

**End of Shift Handover (IA → Human):**
```json
{
  "shift": "IA Execution",
  "completed": ["task1", "task2"],
  "pending": ["task3"],
  "blockers": [],
  "metrics": { "tests_passed": 2891, "commits": 3 },
  "requires_approval": ["PR #123", "feature flag v1.8-sprint1"],
  "handover": "Ready for Human review"
}
```

**End of Week Handover (Human → IA):**
```json
{
  "week": "Week [N]",
  "approved_commits": ["hash1", "hash2"],
  "next_week_priorities": ["priority1", "priority2"],
  "active_incidents": [],
  "funding_status": "on_track",
  "handover": "Week [N+1] cycle initiated"
}
```

---

## 4. Automated Flow

### 4.1 Daily Flow

```
┌─────────────────────────────────────────────────────────┐
│  START: New Shift                                        │
├─────────────────────────────────────────────────────────┤
│  1. IA: Load DAY1_OPERATIONS_PROMPT.md v3.0             │
│  2. IA: Check active incidents (SEV-1/SEV-2)            │
│  3. IA: Run automated checks:                            │
│     - cargo check --features stable                      │
│     - git status (uncommitted changes)                   │
│     - GitHub API (stars, forks, open issues)             │
│     - Funding channel status                             │
│  4. IA: Update daily dashboard                           │
│  5. IA: Execute daily tasks (PR review, triage, etc.)   │
│  6. IA: Generate shift report JSON                       │
│  7. Human: Review + approve actions                      │
│  8. IA: Auto-push if validation PASS                     │
│  9. END: Handover JSON                                   │
└─────────────────────────────────────────────────────────┘
```

### 4.2 Weekly Flow

```
┌─────────────────────────────────────────────────────────┐
│  MON: Standup                                            │
│  ├─ IA: Generate standup MD + metrics                   │
│  └─ Human: Approve hitos + sign-off                     │
├─────────────────────────────────────────────────────────┤
│  TUE: Triage                                             │
│  ├─ IA: Label issues, draft PR reviews                  │
│  └─ Human: Final approvals, SEV decisions               │
├─────────────────────────────────────────────────────────┤
│  WED: PoC                                                │
│  ├─ IA: Implement + test + draft docs                   │
│  └─ Human: Architecture review, RFC alignment           │
├─────────────────────────────────────────────────────────┤
│  THU: Benchmark                                          │
│  ├─ IA: Run benchmarks, compare vs baseline             │
│  └─ Human: Review diffs, approve trade-offs             │
├─────────────────────────────────────────────────────────┤
│  FRI: Auto-Push                                          │
│  ├─ IA: Full validation → commit → push                 │
│  └─ Human: Final review, weekly sign-off                │
└─────────────────────────────────────────────────────────┘
```

### 4.3 Funding Monitoring Flow

```
┌─────────────────────────────────────────────────────────┐
│  FUNDING CHECK (Every shift)                             │
├─────────────────────────────────────────────────────────┤
│  1. IA: Run `scripts/verify_funding_channels.sh`        │
│  2. IA: Check GitHub Sponsors status                    │
│  3. IA: Check Open Collective balance                   │
│  4. IA: Check Gitcoin application status                │
│  5. IA: Check crypto wallet receivals                   │
│  6. IA: Update dashboard "Funding Recibido" section     │
│  7. If funding < 50% weekly target:                     │
│     └─ IA: Flag → Human: Escalate + Discord #funding    │
└─────────────────────────────────────────────────────────┘
```

---

## 5. Rollback Criteria

### 5.1 Automatic Rollback Triggers

| Trigger | Threshold | Action |
|---------|-----------|--------|
| Test failure rate | > 5% vs baseline | Auto-revert + issue |
| Benchmark regression | > 15% vs baseline | Auto-revert + SEV-2 |
| SEV-1 incident | Unresolved > 4h | Manual rollback |
| Feature flag panic | In production | Immediate revert |
| CI failure on main | Any push | Block + revert |

### 5.2 Rollback Procedure

**Step 1: Detect**
- IA monitors test results, benchmarks, and CI status
- Flag trigger condition

**Step 2: Isolate**
- Disable feature flag: Remove `#[cfg(feature = "v1.8-sprint1")]` or set to false
- Prevent further execution of problematic code

**Step 3: Revert**
```bash
git revert <commit_hash> --no-edit
# Commit message: revert: <original_commit_message>
# Reason: <trigger_condition>
```

**Step 4: Document**
- Create `incidents/rollback-[YYYY-MM-DD]-[short_description].md`:
  ```markdown
  # Rollback: [Description]
  - **Date:** [ISO-8601]
  - **Trigger:** [test_failure|benchmark_regression|sev1|panic]
  - **Reverted Commit:** [hash]
  - **Root Cause:** [analysis]
  - **Impact:** [users affected, duration]
  - **Resolution:** [steps taken]
  - **Prevention:** [future mitigation]
  ```

**Step 5: Notify**
- Discord #releases + @ed2kIA/core-team
- Update risk register in standup

**Step 6: Verify**
- Run full validation suite
- Confirm baseline restored
- Update dashboard

### 5.3 Rollback Decision Matrix

| Scenario | IA Action | Human Action | Timeline |
|----------|-----------|--------------|----------|
| Test failure > 5% | Auto-revert + document | Review + approve | < 30min |
| Benchmark > 15% | Flag + draft revert | Decision + execute | < 2h |
| SEV-1 incident | Isolate + notify | War room + command | < 15min |
| Feature panic | Immediate revert | Post-mortem | < 5min |
| CI failure | Block merge + alert | Root cause + fix | < 1h |

---

## 6. Escalation Paths

### 6.1 Escalation Matrix

| Issue Type | IA Action | Escalation Target | SLA |
|------------|-----------|-------------------|-----|
| SEV-1 Security | Isolate + notify | Human + Discord #security | 15min |
| SEV-2 Performance | Flag + benchmark | Human + Discord #performance | 2h |
| RFC-001 Latency | Document + issue | Human + RFC review | 24h |
| v1.8 Roadblock | Draft alternatives | Human + roadmap review | 48h |
| Funding < 50% | Alert + report | Human + Discord #funding | 4h |
| Community Incident | Monitor + draft response | Human + comms | 1h |

### 6.2 Escalation to RFC-001

**When:** Latency bottleneck > RFC-001 targets
1. IA measures current latency
2. IA compares vs RFC-001 targets
3. IA documents in `docs/rfc/rfc-001-latency-mitigation-v1.7.md`
4. IA creates issue `perf(*)` with benchmark data
5. Human reviews → assigns to v1.8 sprint or creates RFC follow-up

### 6.3 Escalation to v1.8 Roadmap

**When:** Feature blocked > 48h or requires architecture change
1. IA documents blocker + alternatives
2. IA drafts roadmap adjustment
3. Human reviews → updates `ISSUES_BATCH_V1.8.md`
4. Community notification via Discord #roadmap

---

## 7. Integration Points

### 7.1 Related Documents

| Document | Purpose | Update Frequency |
|----------|---------|------------------|
| `DAY1_OPERATIONS_PROMPT.md` | Daily operations prompt | Per shift |
| `docs/operations/weekly-standup-week[N].md` | Weekly standup | Weekly |
| `docs/operations/daily-metrics-dashboard.md` | Daily metrics | Per shift |
| `scripts/update_weekly_metrics.sh` | Metrics automation | Weekly |
| `benchmarks/results/baseline-v1.7.json` | Performance baseline | Per release |
| `ISSUES_BATCH_V1.8.md` | Sprint backlog | Per sprint |
| `CONTRIBUTING.md` | Auto-push protocol | Per update |

### 7.2 CI/CD Integration

- **GitHub Actions:** `.github/workflows/ci.yml`
  - Trigger: push/PR to main
  - Steps: cargo check → cargo clippy → cargo test → cargo bench
  - On failure: block merge + notify
- **Auto-Push:** Conventional commits trigger CI
- **Benchmark Archive:** Results stored in `benchmarks/results/`

---

## 8. Metrics & KPIs

### 8.1 Weekly KPIs

| Metric | Target | Measurement |
|--------|--------|-------------|
| Tests passed | ≥ 2891 | `cargo test --lib` |
| Benchmark regression | < 10% | vs baseline-v1.7.json |
| PR review time | < 24h | GitHub API |
| Issue triage time | < 4h | GitHub API |
| Standup accuracy | 100% | Human sign-off |
| Funding progress | ≥ 50% target | Channel checks |
| Community growth | +10% stars/forks | GitHub API |

### 8.2 Reporting

- **Daily:** `docs/operations/daily-metrics-dashboard.md`
- **Weekly:** `docs/operations/weekly-standup-week[N].md` + JSON
- **Sprint:** `release/reports/standup-YYYY-MM-DD.json` archive
- **Release:** `release/v1.8.0-stable/RELEASE_NOTES.md`

---

## 9. Sign-Off Template

```json
{
  "cycle": "weekly",
  "week": "Week [N]",
  "date": "[ISO-8601]",
  "version": "1.8.0-sprint1",
  "phases": {
    "standup": "PASS",
    "triage": "PASS",
    "poc": "PASS",
    "benchmark": "PASS",
    "auto_push": "PASS"
  },
  "metrics": {
    "tests_passed": 2891,
    "commits": 0,
    "prs_merged": 0,
    "issues_closed": 0,
    "funding_usd": 0
  },
  "rollbacks": [],
  "escalations": [],
  "signoff_ia": "Qweni Weekly Cycle Complete",
  "signoff_human": "Awaiting Orchestrator approval"
}
```

---

*Continuous Operations Cycle v1.0 — ed2kIA v1.8.0-sprint1*  
*Generated: 2026-05-15*  
*Weekly Cycle: Standup → Triage → PoC → Benchmark → Auto-Push*  
*Rollback Criteria: Defined | Escalation Paths: Active | Funding Monitoring: Continuous*
