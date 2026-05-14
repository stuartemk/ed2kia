# ed2kIA Community Feedback Triage Workflow

## Overview

This document defines the triage workflow for all community feedback submitted to ed2kIA v0.6.0-RC and beyond. The goal is to ensure every submission is acknowledged within 48 hours, prioritized within 72 hours, and routed to the correct team.

## Workflow Stages

```
[Submitted] → [Triage Queue] → [Acknowledged] → [Prioritized] → [Assigned] → [In Progress] → [Resolved/Rejected]
     │               │               │               │               │               │               │
   Auto-ID       24h SLA         Auto-reply      72h SLA        Sprint        Daily standup    Release notes
  (FB-XXXX)      Triage lead    (template)      Priority P0-P3  planning      updates          Credits
```

## Stage 1: Submission (Automated)

**Trigger**: Community member submits feedback via:
- GitHub Issues (template-driven)
- Discord `#feedback` channel (bot-collected)
- Email to `feedback@ed2kIA.org` (security disclosures)

**Automated Actions**:
1. Generate unique ID: `FB-XXXX-ABC123` (sequential + random suffix)
2. Validate against [`community_feedback_schema.json`](ops/feedback/community_feedback_schema.json)
3. Route to correct triage queue based on `type` and `severity`:
   - `security_disclosure` → Private security queue (encrypted)
   - `bug_report` + `critical/high` → Hot triage queue
   - All others → Standard triage queue
4. Post acknowledgment to submission channel with expected SLA

## Stage 2: Triage Queue (24h SLA)

**Owner**: Triage Lead (rotating weekly, see `#triage-lead` in Discord)

**Process**:
1. Review all items in queue daily at 10:00 UTC
2. For each item:
   - Verify reproducibility (if bug report)
   - Check for duplicates (search existing issues)
   - Assess impact scope (users affected, systems impacted)
   - Validate severity claim (escalate or downgrade)

**Decision Matrix**:

| Can Reproduce? | Impact Scope | Severity Claim | Triage Decision |
|----------------|--------------|----------------|-----------------|
| Yes | Network-wide | Critical | **Accept** → P0 |
| Yes | Multi-node | High | **Accept** → P1 |
| Yes | Single node | Medium | **Accept** → P1/P2 |
| Yes | Local only | Low | **Accept** → P2 |
| No | Any | Any | **Request Info** → 7-day wait |
| N/A | Any | Any (feature) | **Accept** → P2/P3 |
| Duplicate | Any | Any | **Mark Duplicate** → Link original |

**Output**: Status changed to `Acknowledged` with triage notes.

## Stage 3: Acknowledged (Auto-reply)

**Automated Actions**:
1. Send acknowledgment template to reporter:
   ```
   Thank you for your feedback (FB-XXXX-ABC123)!

   Status: Acknowledged
   Triage Lead: @triage-lead
   Expected priority assignment: within 48h
   Expected resolution/update: within current sprint

   We will keep you updated via this thread.
   ```
2. For security disclosures: Encrypt response, use PGP if key provided

## Stage 4: Prioritized (72h SLA)

**Owner**: Tech Lead + Triage Lead (joint decision)

**Process**:
1. Weekly prioritization meeting (Monday 14:00 UTC)
2. Review all `Acknowledged` items
3. Assign priority using **Impact × Effort** matrix:

| Impact \ Effort | Low (1-3 days) | Medium (1-2 sprints) | High (2+ sprints) |
|-----------------|----------------|----------------------|-------------------|
| **Critical** (network down) | P0 - Immediate | P0 - Start now | P1 - Phase 7 Sprint 1 |
| **High** (feature broken) | P1 - Current sprint | P1 - Current sprint | P2 - Next sprint |
| **Medium** (workaround exists) | P1 - Current sprint | P2 - Next sprint | P3 - Backlog |
| **Low** (cosmetic) | P2 - Next sprint | P3 - Backlog | P3 - Future |

4. Update `triage.priority` field in feedback record
5. Tag with target sprint: `phase7-sprint1`, `phase7-sprint2`, etc.

## Stage 5: Assigned (Sprint Planning)

**Owner**: Sprint Lead

**Process**:
1. During sprint planning, pull all P0/P1 items tagged for current sprint
2. Assign to developers based on:
   - Component expertise (see team matrix below)
   - Current workload
   - Availability
3. Update `triage.assigned_to` field
4. Create or link GitHub Issue with feedback ID in title

**Team Component Matrix**:

| Component | Primary Owner | Secondary Owner |
|-----------|---------------|-----------------|
| SAE | @sae-lead | @ml-team |
| Federation | @fed-lead | @p2p-team |
| Staking | @staking-lead | @crypto-team |
| Governance | @gov-lead | @community-team |
| API | @api-lead | @backend-team |
| P2P | @p2p-lead | @network-team |
| ZKP | @crypto-lead | @security-team |
| WASM Sandbox | @security-lead | @backend-team |
| UI | @ui-lead | @frontend-team |
| Docs | @docs-lead | All |
| CI/CD | @devops-lead | @backend-team |
| Monitoring | @devops-lead | @sre-team |

## Stage 6: In Progress (Daily Updates)

**Owner**: Assigned Developer

**Process**:
1. Daily standup mention: "Working on FB-XXXX - [progress]"
2. Commit messages include feedback ID: `fix(FB-0001): description`
3. PR description links to feedback record
4. If blocked: Escalate to Sprint Lead within 4 hours

## Stage 7: Resolved/Rejected

**Owner**: Assigned Developer + Triage Lead (review)

**For Resolved**:
1. PR merged to `main` branch
2. Update `triage.resolution_notes` with fix description
3. Status → `Resolved`
4. Auto-notify reporter: "FB-XXXX has been resolved in PR #NNN"
5. Add to release notes draft (if user-facing)
6. Credit reporter in release notes

**For Rejected**:
1. Triage Lead provides written justification
2. Update `triage.rejection_reason`
3. Status → `Rejected`
4. Notify reporter with explanation and alternatives (if applicable)
5. Reporter can appeal within 7 days (escalate to Tech Lead)

## Escalation Paths

```
Level 0: Automated (bot acknowledgment, validation)
Level 1: Triage Lead (weekly rotation)
Level 2: Tech Lead (priority disputes, P0 decisions)
Level 3: Core Team (strategic decisions, resource allocation)
Level 4: Governance (community-veto items, ethical concerns)
```

**Escalation Triggers**:
- P0 item not acknowledged within 12h → Auto-escalate to Tech Lead
- Reporter appeal on rejection → Tech Lead review within 48h
- Security disclosure → Immediate Level 3 notification
- Ethical concern flagged → Governance council notification

## Metrics & Reporting

**Weekly Dashboard** (published every Monday):
- Total submissions this week
- Average time to acknowledgment (target: <24h)
- Average time to prioritization (target: <72h)
- Average time to resolution (target: <2 sprints for P1)
- Priority distribution (P0/P1/P2/P3)
- Component distribution
- Top contributors (by feedback quality)

**Monthly Report**:
- Trend analysis (volume, severity, resolution rate)
- Team performance (SLA compliance)
- Community satisfaction (optional survey)
- Process improvements proposed

## Templates

### Bug Report Acknowledgment
```
🔍 Bug Report Received: FB-XXXX-ABC123

Thank you for reporting this issue, @reporter!

Title: [Bug title]
Component: [component]
Severity: [claimed severity]

Next steps:
1. Triage team will attempt reproduction within 24h
2. Priority assignment within 72h
3. You'll receive updates here

If you have additional context, please add it to this thread.
```

### Feature Request Acknowledgment
```
💡 Feature Request Received: FB-XXXX-ABC123

Great suggestion, @reporter!

Title: [Feature title]
Component: [component]

Next steps:
1. Team will evaluate feasibility and impact
2. Priority assignment within 72h
3. If accepted, will be scheduled for a Phase 7 sprint

Your detailed description helps us prioritize. Thank you!
```

### Security Disclosure Acknowledgment (Private)
```
🔒 Security Disclosure Received: FB-XXXX-ABC123

Thank you for responsible disclosure.

This report is now in our private security queue.
Response time: 24h initial, 72h detailed assessment.

Please do NOT disclose publicly until we coordinate.
We follow CVE assignment process for valid findings.

PGP-encrypted communication enabled if key provided.
```

## Tools & Integration

- **GitHub Issues**: Primary tracking system
- **Discord Bot**: Auto-collects `#feedback` channel messages
- **Notion/Trello**: Optional visual board for triage team
- **Prometheus/Grafana**: Feedback metrics dashboard
- **Weekly Digest**: Automated email summary to core team

## Review & Improvement

This workflow is reviewed every sprint retro. Community members can propose improvements via `feedback_type=documentation` targeting the triage process itself.

---

*Last updated: 2026-05-04 | Version: 1.0 | Owner: Triage Team*
