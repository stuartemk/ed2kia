# Stewardship Handover Guide — ed2kIA v2.0.0-stable

> **Version:** 1.0
> **Date:** 2026-05-16
> **Status:** ACTIVE — FASE 99
> **Scope:** Complete handover guide for stewardship mode operations

---

## 1. Overview

This document provides the complete handover guide for transitioning ed2kIA from active development to **Stewardship Mode**. Stewardship mode emphasizes maintenance, governance, community facilitation, and autonomous operations over new feature development.

### 1.1 What is Stewardship Mode?

Stewardship Mode is the operational state where:
- **Primary focus:** Maintenance, governance, community facilitation
- **Development:** Community-driven via RFC process
- **Operations:** Autonomous loops with quarterly reviews
- **Decision making:** Constitution-based, meritocratic, transparent
- **Financial logic:** Zero — no tokens, staking, or speculative value

### 1.2 Key Principles

| Principle | Description |
|-----------|-------------|
| **ESTIPULACIÓN > DESARROLLO** | Prioritize monitoring, governance, documentation, community |
| **GOBERNANZA & ÉTICA PRIMERO** | Zero financial logic, continuous alignment, transparency |
| **AUTO-PUSH PERMANENTE** | Validate → Commit → Push protocol active |
| **CERO SUPPOSICIONES** | Read real files, verify paths, dependencies |
| **VALIDACIÓN CONTINUA** | Every phase validated before merge |

---

## 2. Roles & Responsibilities

### 2.1 Stewardship Team

| Role | Responsibilities | Current Status |
|------|------------------|----------------|
| **Core Maintainers** | Code review, RFC approval, security patches | Active community |
| **Governance Lead** | Constitution enforcement, dispute resolution | Community-elected |
| **Operations Lead** | CI/CD health, monitoring, incident response | Autonomous + human oversight |
| **Community Lead** | Onboarding, ambassador program, events | Community-driven |
| **Security Lead** | Threat modeling, audits, CVE response | Rotating |

### 2.2 Role Requirements

- **Core Maintainer:** ≥6 months contribution, ≥10 merged PRs, security training
- **Governance Lead:** Elected by community, ≥1 year involvement
- **Operations Lead:** Technical expertise, on-call rotation
- **Community Lead:** Communication skills, inclusive practices
- **Security Lead:** Security expertise, incident response experience

### 2.3 Succession Plan

1. **Identify successors:** Ongoing mentorship program
2. **Shadow period:** 3 months minimum
3. **Transition:** Gradual responsibility transfer
4. **Documentation:** All processes documented
5. **Emergency contact:** Multiple maintainers per area

---

## 3. Operational Processes

### 3.1 Daily Operations

| Task | Frequency | Owner | Tool |
|------|-----------|-------|------|
| Health check | Daily 02:00 UTC | Autonomous | `scripts/autonomous_health_check.sh` |
| CI/CD monitoring | Continuous | GitHub Actions | `.github/workflows/` |
| Issue triage | Daily | Maintainers | GitHub Issues |
| PR review | <48h SLA | Maintainers | GitHub PRs |

### 3.2 Weekly Operations

| Task | Frequency | Owner | Output |
|------|-----------|-------|--------|
| Standup summary | Weekly | Operations | Standup report |
| Metrics review | Weekly | Operations | Metrics dashboard |
| Community check-in | Weekly | Community Lead | Community report |

### 3.3 Quarterly Operations

| Task | Frequency | Owner | Output |
|------|-----------|-------|--------|
| Quarterly review | Every 90 days | All leads | Review report |
| Roadmap update | Every 90 days | Governance | Updated roadmap |
| Security audit | Every 90 days | Security Lead | Audit report |
| Constitution review | Every 90 days | Governance | Review notes |

**Reference:** [`docs/operations/quarterly-review-template.md`](./quarterly-review-template.md)

---

## 4. Governance Processes

### 4.1 RFC Process

All significant changes require RFC approval:

1. **Draft:** Create RFC using template
2. **Discussion:** Community review (≥14 days)
3. **Approval:** ≥2 Core Team + 0 vetos
4. **Implementation:** Assigned to contributor
5. **Merge/Reject:** Final decision documented

**Reference:** [`docs/governance/rfc-process.md`](../governance/rfc-process.md)

### 4.2 Decision Making

| Decision Type | Process | Timeline |
|--------------|---------|----------|
| **Urgent security** | Security Lead + 1 maintainer | 24-72h |
| **Feature proposal** | RFC process | 30-60 days |
| **Governance change** | Community vote | 60-90 days |
| **Financial** | N/A — Zero financial logic | N/A |

### 4.3 Dispute Resolution

1. **Level 1:** Direct discussion between parties
2. **Level 2:** Mediation by Governance Lead
3. **Level 3:** Community vote
4. **Level 4:** Constitution amendment (if needed)

---

## 5. Technical Infrastructure

### 5.1 Repository Structure

```
ed2kIA/
├── src/                    # Source code (80+ modules)
├── tests/                  # Test suite (3025+ tests)
├── benchmarks/             # Performance benchmarks
├── docs/                   # Documentation
│   ├── governance/         # Governance docs
│   ├── operations/         # Operations docs
│   ├── roadmap/            # Roadmap docs
│   └── community/          # Community docs
├── .github/                # CI/CD workflows
│   └── workflows/          # Automated pipelines
├── deploy/                 # Deployment configs
├── scripts/                # Operational scripts
└── release/                # Release artifacts
```

### 5.2 CI/CD Pipeline

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `ci.yml` | Push/PR | Build, test, lint |
| `autonomous-maintenance.yml` | Daily + push | Health checks |
| `quarterly-review.yml` | Every 90 days | Quarterly review |

### 5.3 Monitoring

| Metric | Tool | Alert Threshold |
|--------|------|-----------------|
| Build health | GitHub Actions | Failed build |
| Test coverage | cargo-llvm-cov | <80% |
| Dependency audit | cargo-audit | Any CVE |
| Response time | Benchmarks | >20% regression |

---

## 6. Community Resources

### 6.1 Communication Channels

| Channel | Purpose | Frequency |
|---------|---------|-----------|
| GitHub Issues | Bug reports, features | Continuous |
| GitHub Discussions | Community chat | Continuous |
| RFC Process | Significant changes | As needed |
| Quarterly Review | Status updates | Every 90 days |

### 6.2 Contribution Guide

1. **Read CONTRIBUTING.md** — Project guidelines
2. **Choose an issue** — Good first issue, help wanted
3. **Create PR** — Follow PR template
4. **Review process** — Maintainer review (<48h)
5. **Merge** — After approval + CI pass

### 6.3 Ambassador Program

- **Role:** Community onboarding, event organization
- **Requirements:** Active contributor, communication skills
- **Benefits:** Recognition, early access, mentorship

---

## 7. Security & Compliance

### 7.1 Security Posture

| Metric | Value |
|--------|-------|
| **OSSF Score** | 8.5/10 (PASSING) |
| **Critical CVEs** | 0 |
| **High CVEs** | 0 |
| **Threat Model** | v2.0 (17 threats identified) |

### 7.2 Incident Response

1. **Detect:** Automated monitoring + community reports
2. **Assess:** Security Lead evaluates severity
3. **Contain:** Immediate mitigation if critical
4. **Fix:** Patch development + testing
5. **Communicate:** Public disclosure timeline
6. **Review:** Post-incident analysis

### 7.3 Compliance

- **OSSF:** 8.5/10 score maintained
- **License:** MIT/Apache-2.0 dual license
- **Transparency:** Public financial reports (if applicable)

---

## 8. Emergency Contacts & Escalation

### 8.1 Escalation Matrix

| Severity | Response Time | Escalation Path |
|----------|--------------|-----------------|
| **Critical** | 1 hour | Security Lead → Core Team → Public |
| **High** | 4 hours | Maintainer → Security Lead |
| **Medium** | 24 hours | Maintainer → Next standup |
| **Low** | 1 week | Backlog → Next sprint |

### 8.2 Emergency Procedures

1. **Security breach:** Activate incident response plan
2. **Data loss:** Restore from backup, investigate cause
3. **Maintainer unavailable:** Succession plan activation
4. **Community crisis:** Governance Lead mediation

---

## 9. Handover Checklist

### 9.1 Pre-Handover

- [ ] All documentation reviewed and up to date
- [ ] Access credentials transferred
- [ ] CI/CD pipelines verified
- [ ] Monitoring alerts configured
- [ ] Community communication plan ready

### 9.2 During Handover

- [ ] Shadow period completed (≥3 months)
- [ ] Knowledge transfer sessions completed
- [ ] Emergency procedures tested
- [ ] Succession plan documented

### 9.3 Post-Handover

- [ ] New stewards operational
- [ ] First quarterly review scheduled
- [ ] Community announcement made
- [ ] Handover retrospective completed

---

## 10. References

- [Project Constitution](../governance/project-constitution.md)
- [GOVERNANCE.md](../../GOVERNANCE.md)
- [Autonomous Loop](./autonomous-loop.md)
- [Quarterly Review Template](./quarterly-review-template.md)
- [RFC Process](../governance/rfc-process.md)
- [Evolution Roadmap](../governance/evolution-roadmap.md)
- [Long-Term Roadmap v2.1→v3.0](../roadmap/long-term-evolution-v2.1-to-v3.0.md)
- [State of ed2kIA v2.0](../announcements/state-of-ed2kIA-v2.0.md)

---

*This handover guide is a living document. Updates follow the RFC process.*
