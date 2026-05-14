# Post-Launch Monitoring Protocol

## ed2kIA v1.6.0-stable

**Last Updated:** 2026-05-14
**Owner:** Core Team / Governance Council

---

## Table of Contents

1. [Monitoring Windows](#monitoring-windows)
2. [Key Metrics](#key-metrics)
3. [Alert Thresholds](#alert-thresholds)
4. [Incident Response](#incident-response)
5. [Community Feedback Loop](#community-feedback-loop)
6. [Rollback Procedure](#rollback-procedure)

---

## Monitoring Windows

| Window | Duration | Focus |
|--------|----------|-------|
| **Critical** | 0-48h | System stability, crash detection, security scanning |
| **Stabilization** | 48h-7d | Performance regression, user feedback triage |
| **Observation** | 7d-30d | Long-term trends, feature adoption, governance activity |

---

## Key Metrics

### System Health

| Metric | Target | Tool |
|--------|--------|------|
| Node uptime | ≥ 99.5% | Prometheus + Grafana |
| P2P sync latency | ≤ 500ms p95 | Network telemetry |
| ZKP verification time | ≤ 200ms p95 | Async ZKP v14 metrics |
| Federation shard load | ≤ 80% utilization | Scaling v7 metrics |
| Checkpoint integrity | 100% valid | SAE v7 integrity validation |

### Community Health

| Metric | Target | Tool |
|--------|--------|------|
| Open issues response time | ≤ 48h | GitHub Issues |
| PR review time | ≤ 72h | GitHub PRs |
| Active node operators | Growing | Network registry |
| Governance proposal activity | ≥ 1/week | Governance module |

---

## Alert Thresholds

### Critical (Page On-Call)

- Node crash rate > 5% in 1h window
- ZKP verification failures > 1%
- Checkpoint integrity validation failures
- Security vulnerability reported (CVE-level)

### Warning (Slack #monitoring)

- P2P sync latency > 1s p95
- Federation shard load > 85%
- Test failures in CI
- Unresolved critical issues > 24h

### Info (Daily Standup)

- New feature adoption metrics
- Governance proposal queue
- Community feedback trends
- Performance benchmark drift

---

## Incident Response

### Severity Levels

| Level | Response Time | Example |
|-------|--------------|---------|
| **SEV-1** | 15 min | Network partition, data loss |
| **SEV-2** | 1h | Major feature broken, performance degradation |
| **SEV-3** | 4h | Minor feature issue, documentation error |
| **SEV-4** | 24h | Enhancement request, cosmetic issue |

### Response Flow

1. **Detect:** Alert triggers → On-call acknowledges
2. **Triage:** Severity assigned → War room created (if SEV-1/2)
3. **Diagnose:** Root cause analysis → Fix proposed
4. **Fix:** Hotfix PR → Fast-track review → Deploy
5. **Verify:** Monitoring confirms resolution
6. **Postmortem:** Within 48h → Action items → Public update

---

## Community Feedback Loop

### Channels

| Channel | Purpose | Response SLA |
|---------|---------|--------------|
| GitHub Issues | Bug reports, feature requests | 48h |
| Discord #general | Community discussion | Best effort |
| Discord #support | Node operator help | 24h |
| Discord #security | Security reports | Immediate |
| Governance proposals | Protocol changes | Per voting schedule |

### Triage Process

1. **Auto-label:** PR Labeler + Issue templates auto-categorize
2. **Daily review:** Core team reviews new issues daily
3. **Weekly triage:** Governance council prioritizes backlog
4. **Sprint planning:** Selected items → Next sprint backlog

---

## Rollback Procedure

### When to Rollback

- SEV-1 incident with no fix in 2h
- Critical security vulnerability
- Data corruption detected
- Network partition > 30% of nodes

### Rollback Steps

1. **Announce:** Notify community via Discord + GitHub
2. **Tag:** Create rollback tag `v1.6.0-rollback-1`
3. **Deploy:** Operators switch to previous stable (`v1.5.0-stable`)
4. **Monitor:** Verify stability on rollback version
5. **Communicate:** Postmortem + timeline for fix

### Rollback Command

```bash
# Operators: Switch to previous stable
git fetch origin
git checkout v1.5.0-stable
cargo build --release --features "stable"
systemctl restart ed2kia
```

---

## Handover Checklist

- [ ] All monitoring dashboards active
- [ ] On-call rotation configured
- [ ] Alert channels tested
- [ ] Rollback procedure documented
- [ ] Community channels announced
- [ ] Governance council briefed
- [ ] Post-launch sign-off complete

---

*This document is living and should be updated after each major release.*
