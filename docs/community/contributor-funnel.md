# Contributor Funnel — ed2kIA v1.8

This document defines the contributor funnel for ed2kIA, mapping the journey from casual observer to core team member. Each tier unlocks increasing privileges, recognition, and governance weight.

## 1. Funnel Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    CONTRIBUTOR FUNNEL                           │
│                                                                 │
│  ┌───────────┐    ┌───────────┐    ┌───────────┐    ┌────────┐ │
│  │  Spectator │───▶ Contributor │───▶  Advocate  │───▶ Steward│ │
│  │   (Tier 0) │    │  (Tier 1)  │    │  (Tier 2)  │    │(Tier3)│ │
│  │           │    │           │    │           │    │      │ │
│  │  Observe  │    │  First    │    │  Regular  │    │ Lead │ │
│  │  Learn    │    │  PR       │    │  Contrib  │    │ Mentor│ │
│  │  Discuss  │    │  Issues   │    │  Review   │    │ Govern│ │
│  └───────────┘    └───────────┘    └───────────┘    └──────┘ │
│                                                                 │
│  Target Conversion:                                              │
│  Spectator → Contributor: 10%                                    │
│  Contributor → Advocate: 30%                                    │
│  Advocate → Steward: 20%                                        │
│  Steward → Guardian: 10%                                        │
└─────────────────────────────────────────────────────────────────┘
```

## 2. Tier Definitions

### Tier 0: Spectator

**Entry**: Automatic (anyone can participate)

| Aspect | Details |
|--------|---------|
| **Activities** | Read docs, browse code, join Discord, attend town halls |
| **Privileges** | Read access, discussion forums, issue comments |
| **Reputation** | 0 points |
| **Governance** | No voting weight |
| **Recognition** | None |

**Path to Next Tier**:
1. Read [`CONTRIBUTING.md`](../../CONTRIBUTING.md)
2. Pick a `good-first-issue` from GitHub Issues
3. Submit first Pull Request

### Tier 1: Contributor

**Entry**: First merged Pull Request OR first verified contribution

| Aspect | Details |
|--------|---------|
| **Activities** | Submit PRs, fix bugs, write docs, report issues |
| **Privileges** | Write access (via PR), issue creation, Discord `@contributor` role |
| **Reputation** | 100-500 points |
| **Governance** | 0.5x voting weight |
| **Recognition** | Contributor badge, name in `CONTRIBUTORS.md` |

**Requirements**:
- 1 merged PR (code, docs, or tests)
- OR 10 verified network contributions (compute, verification)
- Pass code of conduct agreement

**Path to Next Tier**:
1. Accumulate 500 reputation points
2. Review 5+ Pull Requests from other contributors
3. Participate in 1+ governance discussions

### Tier 2: Advocate

**Entry**: 500 reputation points + 5 PR reviews + governance participation

| Aspect | Details |
|--------|---------|
| **Activities** | Regular contributions, PR reviews, mentor newcomers |
| **Privileges** | Triage issues, label management, Discord `@advocate` role |
| **Reputation** | 500-2,000 points |
| **Governance** | 1x voting weight |
| **Recognition** | Advocate badge, featured in monthly report |

**Requirements**:
- 500+ reputation points
- 5+ PR reviews completed
- Active in governance discussions (1+ proposal comments)
- 30-day contribution streak (or equivalent activity)

**Path to Next Tier**:
1. Accumulate 2,000 reputation points
2. Mentor 3+ new Contributors to first PR
3. Lead 1+ feature implementation
4. Nomination by 2+ existing Stewards

### Tier 3: Steward

**Entry**: Nomination + election by existing Stewards

| Aspect | Details |
|--------|---------|
| **Activities** | Lead features, mentor Advocates, governance proposals |
| **Privileges** | PR approval, repo write access, proposal submission |
| **Reputation** | 2,000-15,000 points |
| **Governance** | 2x voting weight, proposal submission |
| **Recognition** | Steward badge, README contributors section |

**Requirements**:
- 2,000+ reputation points
- 3+ mentored Contributors reached Advocate tier
- 1+ feature implementation (merged)
- Nomination by 2+ existing Stewards
- Approval by Steward council (simple majority)

**Path to Next Tier**:
1. Accumulate 15,000 reputation points
2. Lead major feature (multi-sprint)
3. Demonstrate community leadership
4. Election by Guardian council

### Tier 4: Guardian (Core Team)

**Entry**: Election by Guardian council + community approval

| Aspect | Details |
|--------|---------|
| **Activities** | Strategic direction, core architecture, final approvals |
| **Privileges** | Repo admin, release management, steering committee |
| **Reputation** | 15,000+ points |
| **Governance** | 3x voting weight, constitutional amendments |
| **Recognition** | Guardian badge, core team listing, speaking opportunities |

**Requirements**:
- 15,000+ reputation points
- Major feature leadership (2+ merged)
- Community leadership (events, outreach, mentorship)
- Election by Guardian council (2/3 majority)
- Community approval period (7-day comment window)

## 3. Onboarding Experience

### 3.1 First-Time Visitor Journey

```
Visitor lands on GitHub
    │
    ▼
README.md — "El Verdadero Poder de ed2kIA"
    │
    ▼
"Good First Issue" badge on Issues
    │
    ▼
Click issue → See detailed template
    │
    ▼
Template links to CONTRIBUTING.md
    │
    ▼
CONTRIBUTING.md shows quickstart
    │
    ▼
Fork → PR → Merge → Welcome!
```

### 3.2 Good First Issue Template

Every `good-first-issue` includes:

```markdown
## 🎯 What to Do
[Clear description of the task]

## 📚 Resources
- [Link to relevant code]
- [Link to documentation]
- [Link to similar examples]

## ✅ Acceptance Criteria
- [ ] Criterion 1
- [ ] Criterion 2
- [ ] Tests pass

## 🤝 Need Help?
- Ask in #contributing Discord channel
- Tag @advocates for guidance

## 🏆 Rewards
- +50 reputation points
- Contributor badge
- Welcome to the team!
```

### 3.3 Automated Welcome System

| Trigger | Action |
|---------|--------|
| First issue comment | Bot replies with onboarding guide |
| First PR opened | Bot assigns `good-first-issue` mentor |
| PR merged | Bot congratulates + assigns Contributor role |
| 500 rep reached | Bot notifies + suggests Advocate path |

## 4. Retention Strategies

### 4.1 Feedback Loops

| Activity | Feedback | Timeline |
|----------|----------|----------|
| PR Submitted | Acknowledgment + initial review | < 24 hours |
| PR Approved | Celebration + merge | < 48 hours |
| Issue Reported | Triage + label assignment | < 48 hours |
| Question Asked | Answer from Advocate+ | < 24 hours |

### 4.2 Recognition Program

| Milestone | Recognition |
|-----------|-------------|
| First PR | Welcome message + Contributor badge |
| 5th PR | Featured in monthly report |
| 50th PR | Advocate nomination consideration |
| 100th PR | Steward nomination consideration |
| 1 Year Active | Anniversary badge + special mention |
| 3 Year Active | Legend status + lifetime recognition |

### 4.3 Community Events

| Event | Frequency | Purpose |
|-------|-----------|---------|
| Office Hours | Weekly | Q&A with Stewards |
| Sprint Planning | Bi-weekly | Community input on priorities |
| Town Hall | Monthly | Transparency + roadmap updates |
| Hackathon | Quarterly | Intensive contribution events |
| Contributor Summit | Annually | In-person/virtual gathering |

## 5. Metrics & Tracking

### 5.1 Funnel Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Spectator → Contributor | 10% | First PR / unique visitors |
| Contributor → Advocate | 30% | Advocates / Contributors |
| Advocate → Steward | 20% | Stewards / Advocates |
| Average time to first PR | < 7 days | Issue open → PR merge |
| PR review time | < 48 hours | PR open → first review |
| Contributor retention (D30) | > 50% | Active after 30 days |

### 5.2 Dashboard

```
┌─────────────────────────────────────────────────────┐
│           CONTRIBUTOR FUNNEL DASHBOARD              │
├─────────────────────────────────────────────────────┤
│                                                     │
│  Spectators: 15,234                                 │
│  └──→ Contributors: 1,523 (10.0%) ✅                │
│      └──→ Advocates: 457 (30.0%) ✅                 │
│          └──→ Stewards: 91 (20.0%) ✅               │
│              └──→ Guardians: 9 (10.0%) ✅           │
│                                                     │
│  This Week:                                         │
│  ├── New Contributors: +23                          │
│  ├── PRs Merged: +47                                │
│  ├── Issues Resolved: +31                           │
│  └── Avg Review Time: 18h ✅                        │
│                                                     │
└─────────────────────────────────────────────────────┘
```

## 6. Integration with Governance

### 6.1 Voting Weight by Tier

| Tier | Voting Weight | Proposal Rights |
|------|--------------|-----------------|
| Spectator | 0x | None |
| Contributor | 0.5x | Comment only |
| Advocate | 1x | Comment + vote |
| Steward | 2x | Submit + vote |
| Guardian | 3x | Submit + vote + approve |

### 6.2 Proposal Submission Thresholds

| Proposal Type | Minimum Tier |
|--------------|--------------|
| Bug fix priority | Contributor |
| Feature request | Advocate |
| Funding request | Steward |
| Protocol change | Steward |
| Constitutional amendment | Guardian |

### 6.3 Steering Committee

- **Composition**: All Guardians + elected Stewards
- **Election**: Quarterly, by community vote
- **Term**: 3 months, renewable once
- **Responsibilities**: Strategic direction, conflict resolution, release approval

## 7. Anti-Patterns & Mitigations

### 7.1 Common Pitfalls

| Pitfall | Symptom | Mitigation |
|---------|---------|------------|
| PR abandonment | Stale PRs > 2 weeks | Auto-assign mentor + nudge |
| Review bottleneck | PRs waiting > 48h | Alert Stewards + rotate reviewers |
| Contributor burnout | Activity drop after spike | Check-in + workload management |
| Echo chamber | Same voices dominate | Actively seek diverse input |
| Scope creep | Issues too large for newcomers | Break down + add `good-first-issue` |

### 7.2 Burnout Prevention

| Signal | Action |
|--------|--------|
| PR frequency drops 50% | Personal check-in from Steward |
| No activity for 30 days | Re-engagement message |
| Negative sentiment in discussions | Private conversation + support |
| Overcommitment (too many PRs) | Encourage pacing + delegation |

## 8. Tools & Automation

### 8.1 GitHub Bots

| Bot | Purpose |
|-----|---------|
| `welcome-bot` | Greet first-time contributors |
| `triage-bot` | Auto-label + assign issues |
| `stale-bot` | Flag inactive PRs/issues |
| `rep-bot` | Track + display reputation |
| `mentor-bot` | Match newcomers with mentors |

### 8.2 Discord Integration

| Channel | Purpose |
|---------|---------|
| `#welcome` | First-time visitor introductions |
| `#contributing` | Help for first-time contributors |
| `#code-review` | PR discussion + feedback |
| `#governance` | Proposal discussion |
| `#showcase` | Share achievements + learnings |
| `#random` | Community building |

### 8.3 Reputation Tracking

```
Reputation Sources:
├── Code Contributions
│   ├── Bug fix: +20 pts
│   ├── Feature: +50-200 pts (by complexity)
│   └── Tests: +10 pts
├── Documentation
│   ├── New doc: +30 pts
│   ├── Doc improvement: +10 pts
│   └── Translation: +20 pts
├── Community
│   ├── PR review: +15 pts
│   ├── Issue triage: +5 pts
│   ├── Mentorship: +25 pts
│   └── Event organization: +50 pts
└── Network
    ├── Compute contribution: +1 pt/hr
    ├── Verification: +2 pts/proof
    └── Data shard: +5 pts/shard
```

## 9. Success Stories Template

Document and share contributor journeys:

```markdown
# Contributor Spotlight: [Name]

## Journey
- **Started**: [Date]
- **First PR**: [Link]
- **Current Tier**: [Tier]
- **Total Contributions**: [Count]

## Impact
- Features implemented: [List]
- Bugs fixed: [Count]
- Contributors mentored: [Count]

## Quote
> "[Inspiring quote about their experience]"

## Advice for New Contributors
"[Practical advice from their experience]"
```

## 10. Continuous Improvement

### 10.1 Quarterly Review

Every quarter, review:
1. Funnel conversion rates
2. Contributor satisfaction survey
3. Time-to-first-PR metrics
4. Retention rates by cohort
5. Governance participation rates

### 10.2 Feedback Channels

| Channel | Frequency | Audience |
|---------|-----------|----------|
| Contributor Survey | Quarterly | All tiers |
| Steward Retrospective | Monthly | Stewards+ |
| Community Town Hall | Monthly | Open |
| 1-on-1 Check-ins | As needed | At-risk contributors |

---

*This document is a living guide. Update based on community feedback and data.*

**Document Version**: 1.0
**Last Updated**: 2026-05-14
**Owner**: ed2kIA Community Team
