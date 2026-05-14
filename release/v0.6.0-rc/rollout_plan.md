# Rollout Plan: ed2kIA v0.6.0-RC (Canary Strategy)

> **Version**: v0.6.0-RC
> **Date**: 2026-05-04
> **Strategy**: Progressive canary deployment with automated rollback
> **Target**: Production network (seed nodes → validated nodes → full network)

---

## 1. Executive Summary

This document defines the controlled rollout strategy for ed2kIA v0.6.0-RC, introducing Phase 6 features (staking, federation, API v2, ONNX interoperability) to the production network. The rollout follows a **three-phase canary approach** with automated monitoring and rollback triggers.

### Key Principles
- **Gradual**: 10% → 50% → 100% over 72 hours
- **Verifiable**: Each phase requires passing success thresholds
- **Reversible**: One-command rollback to v0.5.0 STABLE
- **Transparent**: All metrics publicly visible via `/api/v2/health`

---

## 2. Rollout Phases

### Phase T0: Seed Nodes (10%) — Day 0

| Parameter | Value |
|---|---|
| **Target** | 3-5 seed nodes (operators opted-in) |
| **Duration** | 24 hours observation |
| **Feature Flags** | `phase6-core`, `phase6-sprint2` |
| **Excluded** | `phase6-experimental`, `experimental` |

**Actions:**
1. Build v0.6.0-RC binaries for target architectures
2. Deploy to seed nodes via `ops/canary_deploy.sh`
3. Verify health endpoints respond on both API v1 and v2
4. Monitor consensus participation and federation sync

**Success Criteria:**
- [ ] All seed nodes report `status: "healthy"` via `/api/v2/health`
- [ ] Consensus participation ≥ 85% of seed nodes
- [ ] Federation round completion rate ≥ 90%
- [ ] API v2 error rate < 0.5%
- [ ] No panics or unrecovered errors in logs

**Rollback Trigger (ANY):**
- Consensus participation drops below 70%
- API error rate exceeds 2%
- Any node panic requiring manual restart
- Federation sync failures > 5 consecutive rounds

---

### Phase T+24h: Validated Nodes (50%) — Day 1

| Parameter | Value |
|---|---|
| **Target** | 50% of network (nodes with reputation ≥ 0.7) |
| **Duration** | 48 hours observation |
| **Feature Flags** | `phase6-core`, `phase6-sprint2` |
| **Eligibility** | Node reputation score ≥ 0.7, uptime ≥ 95% |

**Actions:**
1. Publish v0.6.0-RC to package registry
2. Notify eligible node operators via governance channel
3. Automated rollout via Docker image tag update
4. Monitor network-wide metrics

**Success Criteria:**
- [ ] Network consensus ≥ 85% across all nodes
- [ ] SAE inference latency ≤ 400ms (p95)
- [ ] Federation rounds complete within timeout
- [ ] Staking registry stable (no unexpected slashes)
- [ ] Cross-version compatibility (v0.5.0 ↔ v0.6.0-RC)

**Rollback Trigger (ANY):**
- Network consensus drops below 75%
- SAE latency p95 exceeds 800ms
- More than 3 node crashes in 1 hour
- Staking registry corruption detected
- Cross-version gossip failures

---

### Phase T+72h: Full Network (100%) — Day 3

| Parameter | Value |
|---|---|
| **Target** | All nodes (including low-reputation) |
| **Duration** | 7 days observation |
| **Feature Flags** | `phase6-core`, `phase6-sprint2` |
| **Optional** | `phase6-experimental` for opted-in research nodes |

**Actions:**
1. Update default Docker image tag to v0.6.0-RC
2. Update deployment documentation
3. Enable experimental features for research nodes
4. Begin v0.6.0 STABLE promotion evaluation

**Success Criteria:**
- [ ] All success criteria from T0 and T+24h maintained
- [ ] Network stability for 7 consecutive days
- [ ] No critical bugs reported
- [ ] Performance metrics within 10% of v0.5.0 baselines
- [ ] Community feedback positive (governance channel)

**Promotion to STABLE:**
After 7 days of successful operation, v0.6.0-RC is promoted to v0.6.0 STABLE:
1. Tag `main` branch with `v0.6.0`
2. Update `Cargo.toml` version
3. Publish release notes
4. Archive RC documentation

---

## 3. Monitoring Dashboard

### Critical Metrics

| Metric | Tool | Threshold (Warning) | Threshold (Critical) |
|---|---|---|---|
| Consensus participation | `/api/v2/health` | < 85% | < 70% |
| SAE latency (p95) | `/api/v2/health` | > 400ms | > 800ms |
| API error rate | Prometheus | > 0.5% | > 2% |
| Federation round success | Metrics endpoint | < 90% | < 75% |
| Node crash rate | Systemd/Prometheus | > 1/hr | > 3/hr |
| Memory usage | Memory Guard | > 80% limit | > 95% limit |
| Disk I/O (redb) | System metrics | > 70% capacity | > 90% capacity |

### Health Check Endpoints

```bash
# API v1 (v0.5.0 compatible)
curl http://localhost:3030/api/v1/health

# API v2 (Phase 6)
curl http://localhost:3030/api/v2/health

# OpenAPI spec
curl http://localhost:3030/api/v2/openapi | jq '.info.version'
```

---

## 4. Rollback Procedure

### Automated Rollback
```bash
# One-command rollback to v0.5.0
./ops/rollback_v0.6.0.sh
```

### Manual Rollback Steps
1. **Stop v0.6.0-RC instances**: `systemctl stop ed2kia`
2. **Restore v0.5.0 binary**: `docker pull ed2kia:v0.5.0`
3. **Disable Phase 6 features**: Set `features = ["core-only"]` in config
4. **Clean Phase 6 data**: Remove `redb` files for staking/federation
5. **Restart with v0.5.0**: `systemctl start ed2kia`
6. **Verify**: Check `/api/v1/health` responds normally
7. **Notify**: Post rollback notice to governance channel

### Data Preservation
- **Reputation ledger**: Preserved (shared with v0.5.0)
- **Governance proposals**: Preserved (shared with v0.5.0)
- **Staking registry**: Archived, not deleted (for audit)
- **Federation state**: Cleared on rollback (safe to restart)
- **API v2 auth keys**: Preserved for re-deployment

---

## 5. Communication Plan

| Phase | Audience | Channel | Timing |
|---|---|---|---|
| Pre-T0 | Core team | Slack #dev | 24h before |
| T0 start | Seed operators | Direct message | At deployment |
| T0 results | All operators | Governance channel | T+24h |
| T+24h start | Eligible operators | Announcement | At deployment |
| T+24h results | All operators | Governance channel | T+72h |
| T+72h start | All operators | Announcement | At deployment |
| STABLE promotion | Public | GitHub Release | After 7 days |
| Rollback (if needed) | All operators | Emergency channel | Immediate |

---

## 6. Risk Assessment

| Risk | Probability | Impact | Mitigation |
|---|---|---|---|
| Consensus instability | Low | High | 3-tier ZKP fallback, Merkle-only mode |
| API v2 breaking changes | Low | Medium | API v1 remains active during rollout |
| Federation sync failures | Medium | Medium | Round-based ordering, timeout handling |
| Memory leaks in ONNX adapter | Low | High | Memory Guard limits, automatic cache clearing |
| Key management issues | Low | Medium | In-memory cache, hot-reload ready |
| Cross-version incompatibility | Medium | High | Feature gates isolate Phase 6 modules |
| Performance regression | Medium | Medium | Automated benchmarking, latency thresholds |

---

## 7. Approval Checklist

- [ ] Core team review of rollout plan
- [ ] Seed node operators confirm participation
- [ ] Monitoring dashboards configured
- [ ] Rollback script tested in staging
- [ ] Communication channels verified
- [ ] Emergency contact list updated
- [ ] v0.5.0 rollback binary available

---

*This rollout plan is part of the ed2kIA v0.6.0-RC preparation. Execute with caution, verify at each step, and prioritize network stability over speed.*
