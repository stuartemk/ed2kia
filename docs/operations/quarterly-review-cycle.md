# Quarterly Review Cycle — ed2kIA v2.0+

**Purpose:** Structured quarterly review process for strategic alignment, technical debt assessment, community health, and roadmap calibration.

**Frequency:** Every 3 months (Q1: Jan-Mar, Q2: Apr-Jun, Q3: Jul-Sep, Q4: Oct-Dec)

**Owner:** Orquestador + Core Team

---

## 1. Review Timeline

| Week | Activity | Owner | Deliverable |
|------|----------|-------|-------------|
| W1 | Data Collection | IA (Qweni) | `release/reports/quarterly-data-Q[N]YYYY.json` |
| W2 | Technical Review | Core Team | `release/reports/technical-review-Q[N]YYYY.md` |
| W3 | Community Review | Community Lead | `release/reports/community-review-Q[N]YYYY.md` |
| W4 | Strategic Calibration | Orquestador | `release/reports/quarterly-signoff-Q[N]YYYY.json` |

---

## 2. Data Collection Checklist

### 2.1 Technical Metrics
- [ ] Test coverage trend (quarterly delta)
- [ ] Benchmark regression analysis vs baseline
- [ ] Open issues by severity (P0-P3)
- [ ] PR merge rate (avg days to merge)
- [ ] CI/CD pipeline success rate
- [ ] Feature flag inventory (active vs deprecated)
- [ ] Dependency audit results (CVEs, outdated)
- [ ] Code churn by module (top 10)

### 2.2 Community Metrics
- [ ] Active contributors (quarterly)
- [ ] New contributors (quarterly)
- [ ] Ambassador program participation (Seed/Sprout/Tree)
- [ ] Discord activity (messages, unique users)
- [ ] GitHub stars/forks growth
- [ ] Documentation PRs merged
- [ ] First-time contributor PRs

### 2.3 Funding Metrics
- [ ] GitHub Sponsors revenue
- [ ] Open Collective donations
- [ ] Grant applications submitted
- [ ] Grant applications approved
- [ ] Total funding vs quarterly target
- [ ] Burn rate (if applicable)

### 2.4 Release Metrics
- [ ] Releases shipped (major/minor/patch)
- [ ] Hotfixes deployed
- [ ] Rollbacks executed
- [ ] Time to production (commit → release)
- [ ] Release notes completeness

---

## 3. Technical Review Template

```markdown
# Technical Review Q[N] YYYY

## Executive Summary
- Overall health: [GREEN/YELLOW/RED]
- Key achievements: [...]
- Critical risks: [...]

## Module Health
| Module | Tests | Coverage | Benchmarks | Status |
|--------|-------|----------|------------|--------|
| tauri_scaffold | 18 | 100% | N/A | GREEN |
| multi_curve_setup | 18 | 100% | N/A | GREEN |
| k8s_operator_base | 14 | 100% | N/A | GREEN |

## Technical Debt
- [ ] Deprecated feature flags to remove
- [ ] Pre-existing test failures to address
- [ ] Performance regressions to investigate
- [ ] Security vulnerabilities to patch

## Architecture Decisions
- [ ] New patterns to adopt
- [ ] Modules to refactor
- [ ] Dependencies to upgrade
- [ ] Infrastructure changes needed

## Next Quarter Technical Goals
1. [Goal 1]
2. [Goal 2]
3. [Goal 3]
```

---

## 4. Community Review Template

```markdown
# Community Review Q[N] YYYY

## Executive Summary
- Community health: [GREEN/YELLOW/RED]
- Growth rate: [+X% contributors, +Y% stars]
- Engagement: [High/Medium/Low]

## Ambassador Program
| Tier | Active | Completed | Retention |
|------|--------|-----------|-----------|
| Seed | 0 | 0 | N/A |
| Sprout | 0 | 0 | N/A |
| Tree | 0 | 0 | N/A |

## Contributor Experience
- Onboarding time (first PR): [X days]
- PR review time (avg): [X hours]
- Issue response time (P0): [X hours]
- Satisfaction (if surveyed): [X/5]

## Communication Channels
| Channel | Active Users | Posts/Week | Health |
|---------|-------------|------------|--------|
| Discord | 0 | 0 | N/A |
| GitHub Discussions | 0 | 0 | N/A |
| Issues | 0 | 0 | N/A |

## Next Quarter Community Goals
1. [Goal 1]
2. [Goal 2]
3. [Goal 3]
```

---

## 5. Strategic Calibration

### 5.1 Roadmap Review
- [ ] Current sprint alignment with quarterly goals
- [ ] Feature flag cleanup plan
- [ ] Version milestone assessment
- [ ] Release cadence evaluation

### 5.2 Risk Assessment
| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| [Risk 1] | High/Med/Low | High/Med/Low | [Action] |

### 5.3 Resource Planning
- [ ] Core team capacity (next quarter)
- [ ] Funding runway
- [ ] Infrastructure costs
- [ ] Tooling investments needed

### 5.4 Sign-off Criteria
- [ ] Technical review completed
- [ ] Community review completed
- [ ] Funding status assessed
- [ ] Next quarter goals defined
- [ ] Risks documented with mitigation
- [ ] Orquestador approval obtained

---

## 6. Automation Hooks

### 6.1 Data Collection Script
```bash
# Generate quarterly data report
bash scripts/update_weekly_metrics.sh --quarterly --quarter Q[N]YYYY
# Output: release/reports/quarterly-data-Q[N]YYYY.json
```

### 6.2 Automated Checks
- `cargo test --features stable` → Test count + pass rate
- `cargo bench -p ed2kIA-benchmarks` → Benchmark comparison
- `scripts/dependency_audit.sh` → Security audit
- `scripts/stable-maintenance.sh --full` → Full maintenance report

### 6.3 Report Generation
- IA (Qweni) auto-generates draft reports from collected data
- Human reviews and adds qualitative assessment
- Final sign-off requires Orquestador approval

---

## 7. Archive Structure

```
release/reports/
├── quarterly-data-Q12026.json
├── technical-review-Q12026.md
├── community-review-Q12026.md
├── quarterly-signoff-Q12026.json
├── quarterly-data-Q22026.json
└── ...
```

---

## 8. First Review Schedule

| Quarter | Dates | Focus |
|---------|-------|-------|
| Q2 2026 | Apr 1 - Jun 30 | v2.0 Sprint 1 stabilization, community scaling |
| Q3 2026 | Jul 1 - Sep 30 | v2.0 feature completion, grant execution |
| Q4 2026 | Oct 1 - Dec 31 | v2.0 release prep, annual review |

---

*Quarterly Review Cycle v1.0 — ed2kIA v2.0+*
*Created: 2026-05-16 (FASE 84)*
*Owner: Orquestador + Core Team*
